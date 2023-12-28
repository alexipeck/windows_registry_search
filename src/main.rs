use directories::BaseDirs;
use parking_lot::RwLock;
use registry_playground::{controls::controls, static_selection::StaticSelection, Focus, renderer::renderer_wrappers_wrapper};
use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{task::JoinHandle, try_join};
use tracing::Level;
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, registry::Registry, Layer};


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let base_directories = BaseDirs::new().expect("Base directories not found");
    let log_path = base_directories
        .config_dir()
        .join("windows_registry_search/logs/");
    let file = tracing_appender::rolling::daily(log_path, format!("log"));
    let (file_writer, _guard) = tracing_appender::non_blocking(file);
    let level_filter = LevelFilter::from_level(Level::DEBUG);
    let logfile_layer = tracing_subscriber::fmt::layer()
        .with_line_number(true)
        .with_writer(file_writer)
    .    with_filter(level_filter);
    let subscriber = Registry::default().with(logfile_layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let focus: Arc<RwLock<Focus>> = Arc::new(RwLock::new(Focus::Main));
    let static_menu_selection: Arc<StaticSelection> = Arc::new(StaticSelection::default());
    let stop: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let controls_thread: JoinHandle<()> = tokio::spawn(controls(static_menu_selection.to_owned(), focus.to_owned(), stop.to_owned()));

    let renderer_thread: JoinHandle<Result<(), ()>> = tokio::spawn(renderer_wrappers_wrapper(static_menu_selection.to_owned(), focus.to_owned(), stop.to_owned()));

    let _ = try_join!(controls_thread, renderer_thread);
    let stopping = stop.load(Ordering::SeqCst);
    if !stopping {
        stop.store(true, Ordering::SeqCst);
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(())
}
