use parking_lot::Mutex;
use tokio::sync::Notify;
use std::{
    collections::{VecDeque, HashSet},
    error::Error,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use winreg::{enums::*, RegKey};

static KEY_COUNT: AtomicUsize = AtomicUsize::new(0);
static VALUE_COUNT: AtomicUsize = AtomicUsize::new(0);
static HKLM: RegKey = RegKey::predef(HKEY_LOCAL_MACHINE);

const REGEDIT_OUTPUT_FOR_BLANK_NAMES: bool = true;

pub struct WorkerManager {
    threads: usize,
    search_terms: Vec<String>,
    key_queue: Arc<Mutex<VecDeque<String>>>,
    work_ready_for_processing: Arc<Notify>,
    threads_waiting_for_work: Arc<AtomicUsize>,
    no_work_left: Arc<Notify>,
    pub results: Arc<Mutex<HashSet<String>>>,
    pub errors: Arc<Mutex<HashSet<String>>>,
}

impl WorkerManager {
    pub fn new(search_terms: Vec<String>, threads_to_use: usize) -> Self {
        Self {
            threads: threads_to_use,
            search_terms: search_terms.into_iter().map(|term| term.to_lowercase()).collect(),
            key_queue: Arc::new(Mutex::new(VecDeque::new())),
            work_ready_for_processing: Arc::new(Notify::new()),
            threads_waiting_for_work: Arc::new(AtomicUsize::new(0)),

            no_work_left: Arc::new(Notify::new()),

            results: Arc::new(Mutex::new(HashSet::new())),
            errors: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    fn feed_queue_and_process_values(
        &self,
        key_path: &str,
    ) -> Result<(), Box<dyn Error>> {
        if self.string_matches(key_path) {
            self.results.lock().insert(format!("HKEY_LOCAL_MACHINE\\{}", key_path));
        }
        let registry_key = HKLM.open_subkey_with_flags(key_path, KEY_READ)?;
        {
            let mut key_paths = Vec::new();
            for key_result in registry_key.enum_keys() {
                KEY_COUNT.fetch_add(1, Ordering::SeqCst);
                match key_result {
                    Ok(key_name) => {
                        let key_path = format!("{}\\{}", key_path, key_name);
                        key_paths.push(key_path);
                    }
                    Err(err) => {
                        self.errors.lock().insert(format!("{}, Subkey error: \"{}\"", key_path, err));
                    },
                }
            }
            self.feed_queue(key_paths);
            self.work_ready_for_processing.notify_waiters();
        }
    
        for value_result in registry_key.enum_values() {
            VALUE_COUNT.fetch_add(1, Ordering::SeqCst);
            match value_result {
                Ok((value_name, reg_value)) => {
                    let data = reg_value.to_string();
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
                        self.results.lock().insert(format!(
                            "HKEY_LOCAL_MACHINE\\{}\\{} = \"{}\" ({:?})",
                            key_path,
                            value_name,
                            data,
                            reg_value.vtype,
                        ));
                    }
                }
                Err(err) => {
                    self.errors.lock().insert(format!("{}, Value error: \"{}\"", key_path, err));
                },
            }
        }
        Ok(())
    }

    pub async fn get_work(&self) -> Option<String> {
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

    pub fn feed_queue(&self, keys: Vec<String>) {
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

    pub async fn run(&self, worker_manager: Arc<WorkerManager>) {
        for _ in 0..worker_manager.threads {
            let worker_manager = worker_manager.to_owned();
            tokio::spawn(run_thread(worker_manager));
        }
        self.work_ready_for_processing.notify_waiters();
        loop {
            if worker_manager.threads_waiting_for_work.load(Ordering::SeqCst) == worker_manager.threads {
                if self.key_queue.lock().len() == 0 {
                    self.no_work_left.notify_waiters();
                    break;
                } else {
                    self.work_ready_for_processing.notify_waiters();
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

async fn run_thread(worker_manager: Arc<WorkerManager>) {
    loop {
        let key_path = match worker_manager.get_work().await {
            Some(key_path) => key_path,
            None => break,
        };
        if let Err(err) = worker_manager.feed_queue_and_process_values(&key_path) {
            worker_manager.errors.lock().insert(format!("{}, Key error: \"{}\"", key_path, err));
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let worker_manager = Arc::new(WorkerManager::new(vec!["Google Chrome".to_string(), "7-Zip".to_string()], num_cpus::get()));

    worker_manager.feed_queue(vec!["Software".to_string()]);
    let start_time = Instant::now();
    worker_manager.run(worker_manager.to_owned()).await;

    eprintln!("Errors:");
    for error in worker_manager.errors.lock().iter() {
        eprintln!("{}", error);
    }

    println!("\nResults:");
    for result in worker_manager.results.lock().iter() {
        println!("{}", result);
    }
    println!("Key count: {}, Value count: {}", KEY_COUNT.load(Ordering::SeqCst), VALUE_COUNT.load(Ordering::SeqCst));
    println!("Completed in {}ms.", start_time.elapsed().as_millis());
    Ok(())
}
