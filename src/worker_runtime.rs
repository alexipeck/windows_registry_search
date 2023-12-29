use tracing::{debug, info};

use crate::{
    static_selection::StaticSelection,
    worker_manager::{run, WorkerManager},
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
        if static_menu_selection.running.load(Ordering::SeqCst) {
            static_menu_selection
                .run_control_temporarily_disabled
                .store(true, Ordering::SeqCst);
            static_menu_selection.stop.store(true, Ordering::SeqCst);
        } else {
            let roots = static_menu_selection.selected_roots.read().export_roots();
            let search_terms = static_menu_selection
                .search_term_tracker
                .read()
                .search_terms
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<String>>();
            static_menu_selection
                .run_control_temporarily_disabled
                .store(true, Ordering::SeqCst);
            let stop = static_menu_selection.stop.to_owned();
            let stop_notify = static_menu_selection.stop_notify.to_owned();
            let run_control_temporarily_disabled = static_menu_selection
                .run_control_temporarily_disabled
                .to_owned();
            let running = static_menu_selection.running.to_owned();
            let results = static_menu_selection.results.to_owned();
            running.store(true, Ordering::SeqCst);
            run_control_temporarily_disabled.store(false, Ordering::SeqCst);

            let worker_manager = Arc::new(WorkerManager::new(
                search_terms,
                num_cpus::get(),
                results,
                stop.to_owned(),
                stop_notify,
            ));

            worker_manager.feed_queue(vec!["Software".to_string()]);
            let start_time = Instant::now();
            run(worker_manager.to_owned()).await;

            /* eprintln!("Errors:");
            for error in worker_manager.errors.lock().iter() {
                eprintln!("{}", error);
            }

            println!("\nResults:");
            for result in worker_manager.results.lock().iter() {
                println!("{}", result);
            }
            println!(
                "Key count: {}, Value count: {}",
                KEY_COUNT.load(Ordering::SeqCst),
                VALUE_COUNT.load(Ordering::SeqCst)
            ); */
            info!("Completed in {}ms.", start_time.elapsed().as_millis());

            stop.store(false, Ordering::SeqCst);
            running.store(false, Ordering::SeqCst);
            run_control_temporarily_disabled.store(false, Ordering::SeqCst);
        }
    }
    debug!("Worker thread closed.");
}
