use crate::{
    alt_reg_value_to_string, root::Root, KEY_COUNT, REGEDIT_OUTPUT_FOR_BLANK_NAMES, VALUE_COUNT,
};
use parking_lot::Mutex;
use std::{
    collections::{BTreeSet, HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::Notify;
use winreg::{enums::*, RegKey};

pub async fn run_thread(worker_manager: Arc<WorkerManager>) {
    loop {
        let key_pair = match worker_manager.get_work().await {
            Some(key_pair) => key_pair,
            None => break,
        };
        worker_manager.feed_queue_and_process_values(key_pair);
    }
}

pub struct WorkerManager {
    threads: usize,
    search_terms: Vec<String>,
    key_queue: Arc<Mutex<VecDeque<(isize, String)>>>,
    work_ready_for_processing: Arc<Notify>,
    threads_waiting_for_work: Arc<AtomicUsize>,
    no_work_left: Arc<Notify>,
    pub results: Arc<Mutex<BTreeSet<String>>>,
    pub errors: Arc<Mutex<HashSet<String>>>,
    stop: Arc<AtomicBool>,
    stop_notify: Arc<Notify>,
}

impl WorkerManager {
    pub fn new(
        search_terms: Vec<String>,
        threads_to_use: usize,
        results: Arc<Mutex<BTreeSet<String>>>,
        stop: Arc<AtomicBool>,
        stop_notify: Arc<Notify>,
    ) -> Self {
        Self {
            threads: threads_to_use,
            search_terms: search_terms
                .into_iter()
                .map(|term| term.to_lowercase())
                .collect(),
            key_queue: Arc::new(Mutex::new(VecDeque::new())),
            work_ready_for_processing: Arc::new(Notify::new()),
            threads_waiting_for_work: Arc::new(AtomicUsize::new(0)),

            no_work_left: Arc::new(Notify::new()),

            results,
            errors: Arc::new(Mutex::new(HashSet::new())),

            stop,
            stop_notify,
        }
    }

    fn feed_queue_and_process_values(&self, (reg_key, key_path): (isize, String)) {
        if self.string_matches(&key_path) {
            let root_name = match Root::from_isize(reg_key) {
                Some(root) => root.to_string(),
                None => "InvalidRoot".into(),
            };
            self.results
                .lock()
                .insert(format!("{}\\{}", root_name, &key_path));
        }
        let registry_key =
            match RegKey::predef(reg_key).open_subkey_with_flags(key_path.to_owned(), KEY_READ) {
                Ok(registry_key) => registry_key,
                Err(err) => {
                    let root_name = match Root::from_isize(reg_key) {
                        Some(root) => root.to_string(),
                        None => "InvalidRoot".into(),
                    };
                    self.errors.lock().insert(format!(
                        "{}: {}, Key error: \"{}\"",
                        &key_path, root_name, err
                    ));
                    return;
                }
            };
        {
            let mut key_paths = Vec::new();
            for key_result in registry_key.enum_keys() {
                KEY_COUNT.fetch_add(1, Ordering::SeqCst);
                match key_result {
                    Ok(key_name) => {
                        key_paths.push((reg_key, format!("{}\\{}", &key_path, key_name)));
                    }
                    Err(err) => {
                        self.errors
                            .lock()
                            .insert(format!("{}, Subkey error: \"{}\"", &key_path, err));
                    }
                }
            }
            self.feed_queue(key_paths);
            self.work_ready_for_processing.notify_waiters();
        }

        for value_result in registry_key.enum_values() {
            VALUE_COUNT.fetch_add(1, Ordering::SeqCst);
            match value_result {
                Ok((value_name, reg_value)) => {
                    let vtype = reg_value.vtype.to_owned();
                    let data = alt_reg_value_to_string(reg_value);
                    if self.any_string_matches(&value_name, &data) {
                        let value_name = if value_name.is_empty() {
                            if REGEDIT_OUTPUT_FOR_BLANK_NAMES {
                                "(Default)".to_string()
                            } else {
                                value_name
                            }
                        } else {
                            value_name
                        };
                        let root_name = match Root::from_isize(reg_key) {
                            Some(root) => root.to_string(),
                            None => "InvalidRoot".into(),
                        };
                        self.results.lock().insert(format!(
                            "{}\\{}\\{} = \"{}\" ({:?})",
                            root_name, &key_path, value_name, data, vtype,
                        ));
                    }
                }
                Err(err) => {
                    self.errors
                        .lock()
                        .insert(format!("{}, Value error: \"{}\"", &key_path, err));
                }
            }
        }
    }

    pub async fn get_work(&self) -> Option<(isize, String)> {
        loop {
            let work = self.key_queue.lock().pop_front();
            if let Some(key) = work {
                return Some(key);
            } else {
                self.threads_waiting_for_work.fetch_add(1, Ordering::SeqCst);
                tokio::select! {
                    _ = self.work_ready_for_processing.notified() => {},
                    _ = self.no_work_left.notified() => return None,
                }
                self.threads_waiting_for_work.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }

    pub fn feed_queue(&self, keys: Vec<(isize, String)>) {
        let mut lock = self.key_queue.lock();
        lock.extend(keys);
    }

    pub fn any_string_matches(&self, string: &str, string2: &str) -> bool {
        let string_lowercase = string.to_lowercase();
        let string2_lowercase = string2.to_lowercase();
        for term in self.search_terms.iter() {
            if string_lowercase.contains(term) || string2_lowercase.contains(term) {
                return true;
            }
        }
        false
    }

    pub fn string_matches(&self, string: &str) -> bool {
        let string_lowercase = string.to_lowercase();
        for term in self.search_terms.iter() {
            if string_lowercase.contains(term) {
                return true;
            }
        }
        false
    }
}

pub async fn run(worker_manager: Arc<WorkerManager>) {
    for _ in 0..worker_manager.threads {
        let worker_manager = worker_manager.to_owned();
        tokio::spawn(run_thread(worker_manager));
    }
    worker_manager.work_ready_for_processing.notify_waiters();
    loop {
        if worker_manager
            .threads_waiting_for_work
            .load(Ordering::SeqCst)
            == worker_manager.threads
        {
            if worker_manager.key_queue.lock().len() == 0 {
                worker_manager.no_work_left.notify_waiters();
                break;
            } else {
                worker_manager.work_ready_for_processing.notify_waiters();
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
