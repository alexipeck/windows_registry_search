use tracing::{debug, info};
use winreg::RegKey;

use crate::{
    root::Root,
    static_selection::StaticSelection,
    worker_manager::{run, WorkerManager},
    KEY_COUNT, VALUE_COUNT,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

pub async fn worker_runtime(
    static_menu_selection: Arc<StaticSelection>,
    mut rx: tokio::sync::mpsc::Receiver<()>,
    stop: Arc<AtomicBool>,
) {
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        match rx.recv().await {
            Some(_) => {}
            None => break,
        }
        if stop.load(Ordering::SeqCst) {
            break;
        }
        KEY_COUNT.store(0, Ordering::SeqCst);
        VALUE_COUNT.store(0, Ordering::SeqCst);

        let roots = static_menu_selection.selected_roots.read().export_roots();
        let search_terms = static_menu_selection
            .search_term_tracker
            .read()
            .search_terms
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<String>>();
        let worker_manager = Arc::new(WorkerManager::new(
            search_terms,
            num_cpus::get(),
            static_menu_selection.results.to_owned(),
            static_menu_selection.stop.to_owned(),
            static_menu_selection.stop_notify.to_owned(),
        ));

        let mut work = Vec::new();
        for root in roots {
            for key_result in RegKey::predef(root).enum_keys() {
                KEY_COUNT.fetch_add(1, Ordering::SeqCst);
                match key_result {
                    Ok(key_name) => work.push((root, key_name)),
                    Err(err) => {
                        let root_name = match Root::from_isize(root) {
                            Some(root) => root.to_string(),
                            None => "InvalidRoot".into(),
                        };
                        let _ = worker_manager
                            .errors
                            .lock()
                            .insert(format!("{}, Subkey error: \"{}\"", root_name, err));
                    }
                }
            }
        }

        worker_manager.feed_queue(work);
        let start_time = Instant::now();
        run(worker_manager.to_owned()).await;

        /* eprintln!("Errors:");
        for error in worker_manager.errors.lock().iter() {
            eprintln!("{}", error);
        }

        println!("\nResults:");
        for result in worker_manager.results.lock().iter() {
            println!("{}", result);
        } */
        info!("Completed in {}ms.", start_time.elapsed().as_millis());

        static_menu_selection.stop.store(false, Ordering::SeqCst);
        let mut timer_lock = static_menu_selection.timer.write();
        if let Some((_, end_time)) = timer_lock.as_mut() {
            *end_time = Some(Instant::now());
        }
        drop(timer_lock);
        *static_menu_selection.running.lock() = false;
        static_menu_selection
            .run_control_temporarily_disabled
            .store(false, Ordering::SeqCst);
    }
    debug!("Worker thread closed.");
}
