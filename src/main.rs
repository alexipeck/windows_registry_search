use parking_lot::Mutex;
use std::{
    collections::VecDeque,
    error::Error,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use winreg::{enums::*, RegKey};

const MAX_THREADS: usize = 64;

static KEY_COUNT: AtomicUsize = AtomicUsize::new(0);
static VALUE_COUNT: AtomicUsize = AtomicUsize::new(0);
static HKLM: RegKey = RegKey::predef(HKEY_LOCAL_MACHINE);

fn feed_queue_and_process_values(
    key_path: &str,
    worker_manager: &Arc<WorkerManager>,
) -> Result<(), Box<dyn Error>> {
    if worker_manager.string_matches(key_path) {
        println!("{}", key_path);
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
                Err(_err) => {
                    //println!("{}", err)
                },
            }
        }
        worker_manager.feed_queue(key_paths);
    }

    for value_result in registry_key.enum_values() {
        VALUE_COUNT.fetch_add(1, Ordering::SeqCst);
        match value_result {
            Ok((value_name, reg_value)) => {
                let data = reg_value.to_string();
                if worker_manager.any_string_matches(&value_name, &data) {
                    format!(
                        "Name: {}, Type: {:?}, Data: \"{}\"",
                        value_name,
                        reg_value.vtype,
                        data
                    );
                }
            }
            Err(_err) => {
                //println!("{}", err)
            },
        }
    }
    Ok(())
}

pub struct WorkerManager {
    max_threads: usize,
    current_threads: AtomicUsize,
    timeout: Duration,
    search_terms: Vec<String>,
    key_queue: Arc<Mutex<VecDeque<String>>>,
}

impl WorkerManager {
    pub fn new(max_threads: usize, timeout: u64, search_terms: Vec<String>) -> Self {
        Self {
            max_threads,
            current_threads: AtomicUsize::new(0),
            timeout: Duration::from_millis(timeout),
            search_terms: search_terms.into_iter().map(|term| term.to_lowercase()).collect(),
            key_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    fn wait(&self) {
        while self.current_threads.load(Ordering::SeqCst) > self.max_threads {
            thread::sleep(self.timeout)
        }
    }
    pub fn acquire(&self) -> String {
        loop {
            self.wait();
            if let Some(key) = self.key_queue.lock().pop_front() {
                self.current_threads.fetch_add(1, Ordering::SeqCst);
                return key;
            }
        }
    }
    pub fn release(&self) {
        self.current_threads.fetch_sub(1, Ordering::SeqCst);
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

    pub fn output_progress_until_finish(&self) {
        let start_time = Instant::now();
        let min_runtime = Duration::from_secs(1);
        loop {
            if self.key_queue.lock().len() == 0 && start_time.elapsed() > min_runtime {
                break;
            }
            println!("Key count: {}, Value count: {}", KEY_COUNT.load(Ordering::SeqCst), VALUE_COUNT.load(Ordering::SeqCst));
            thread::sleep(Duration::from_secs(1));
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let worker_manager = Arc::new(WorkerManager::new(MAX_THREADS, 500, vec!["Google Chrome".to_string(), "7-Zip".to_string()]));

    worker_manager.feed_queue(vec!["Software".to_string()]);
    let start_time = Instant::now();
    for _ in 0..MAX_THREADS {
        let worker_manager = worker_manager.to_owned();

        thread::spawn(move || loop {
            let worker_manager = worker_manager.to_owned();
            let key_path = worker_manager.acquire();
            if let Err(_err) = feed_queue_and_process_values(&key_path, &worker_manager) {
                //println!("{}", err);
            }
            worker_manager.release();
        });
    }
    worker_manager.output_progress_until_finish();
    println!("Completed in {}ms.", start_time.elapsed().as_millis());
    Ok(())
}
