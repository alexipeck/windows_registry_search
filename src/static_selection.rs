use parking_lot::{Mutex, RwLock};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
    time::Instant,
};
use strum::IntoEnumIterator;
use tokio::sync::Notify;
use tracing::{debug, info};
use ratatui::{
    style::{Color, Style},
    text::{Span, Line},
};
use crate::{search_term_tracker::SearchTermTracker, root::{SelectedRoots, Root}, SELECTION_COLOUR, DEBOUNCE, worker_manager::{WorkerManager, run}};

pub struct StaticSelection {
    pub pane_selected: Arc<AtomicU8>,           //horizontal
    pane_last_changed: Arc<Mutex<Instant>>, //horizontal

    pub search_term_tracker: Arc<RwLock<SearchTermTracker>>,

    root_selected: Arc<AtomicU8>,
    root_selection_last_changed: Arc<Mutex<Instant>>,

    selected_roots: Arc<RwLock<SelectedRoots>>,

    pub running: Arc<AtomicBool>,
    pub run_control_temporarily_disabled: Arc<AtomicBool>, //running thread resets this once closed
    stop: Arc<AtomicBool>,                             //running thread resets this once closed
    stop_notify: Arc<Notify>,

    results: Arc<Mutex<HashSet<String>>>,
}

impl Default for StaticSelection {
    fn default() -> Self {
        Self {
            pane_selected: Arc::new(AtomicU8::new(0)),
            pane_last_changed: Arc::new(Mutex::new(Instant::now())),
            root_selected: Arc::new(AtomicU8::new(0)),
            root_selection_last_changed: Arc::new(Mutex::new(Instant::now())),
            search_term_tracker: Arc::new(RwLock::new(SearchTermTracker::default())),
            selected_roots: Arc::new(RwLock::new(SelectedRoots::default())),
            running: Arc::new(AtomicBool::new(false)),
            run_control_temporarily_disabled: Arc::new(AtomicBool::new(false)),
            stop: Arc::new(AtomicBool::new(false)),
            stop_notify: Arc::new(Notify::new()),
            results: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}

impl StaticSelection {
    pub fn generate_root_list(&self) -> Vec<Line<'static>> {
        let root_selected = self.root_selected.load(Ordering::SeqCst);
        let pane_selected = self.pane_selected.load(Ordering::SeqCst) == 0;
        Root::iter()
            .map(|root| {
                let root_enabled = self.selected_roots.read().is_enabled(&root);
                Line::from(vec![
                    Span::styled(
                        format!("{:38}", root.to_string(),),
                        Style::default().fg(if pane_selected && root as u8 == root_selected {
                            SELECTION_COLOUR
                        } else {
                            Color::White
                        }),
                    ),
                    Span::styled(
                        if root_enabled { "Enabled" } else { "Disabled" },
                        Style::default().fg(if root_enabled {
                            Color::Green
                        } else {
                            Color::White
                        }),
                    ),
                ])
            })
            .collect::<Vec<Line>>()
    }

    pub fn generate_results(&self) -> Vec<Line<'static>> {
        self.results.lock()
            .iter()
            .map(|result| {
                Line::from(vec![Span::styled(
                    result.to_string(),
                    Style::default().fg(Color::White),
                )])
            })
            .collect::<Vec<Line>>()
    }

    pub fn pane_left(&self) {
        if self.pane_last_changed.lock().elapsed() < DEBOUNCE {
            return;
        }
        let new_value = match self.pane_selected.load(Ordering::SeqCst) {
            0 => 2,
            1 => 0,
            2 => 1,
            _ => return,
        };
        self.pane_selected.store(new_value, Ordering::SeqCst);
        *self.pane_last_changed.lock() = Instant::now();
    }

    pub fn pane_right(&self) {
        if self.pane_last_changed.lock().elapsed() < DEBOUNCE {
            return;
        }
        let new_value = match self.pane_selected.load(Ordering::SeqCst) {
            0 => 1,
            1 => 2,
            2 => 0,
            _ => return,
        };
        self.pane_selected.store(new_value, Ordering::SeqCst);
        *self.pane_last_changed.lock() = Instant::now();
    }

    pub fn root_up(&self) {
        if self.root_selection_last_changed.lock().elapsed() < DEBOUNCE {
            return;
        }
        let new_value = match self.root_selected.load(Ordering::SeqCst) {
            0 => 4,
            1 => 0,
            2 => 1,
            3 => 2,
            4 => 3,
            _ => return,
        };
        self.root_selected.store(new_value, Ordering::SeqCst);
        *self.root_selection_last_changed.lock() = Instant::now();
    }

    pub fn root_down(&self) {
        if self.root_selection_last_changed.lock().elapsed() < DEBOUNCE {
            return;
        }
        let new_value = match self.root_selected.load(Ordering::SeqCst) {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 4,
            4 => 0,
            _ => return,
        };
        self.root_selected.store(new_value, Ordering::SeqCst);
        *self.root_selection_last_changed.lock() = Instant::now();
    }

    pub fn root_toggle(&self) {
        let selected = self.root_selected.load(Ordering::SeqCst);
        if let Some(root) = Root::from_u8(selected) {
            self.selected_roots.write().toggle(&root);
        }
    }
}

pub async fn toggle_running(static_menu_selection: Arc<StaticSelection>) {
    debug!("A");
    if static_menu_selection.running.load(Ordering::SeqCst) {
        debug!("B");
        static_menu_selection.run_control_temporarily_disabled
            .store(true, Ordering::SeqCst);
        static_menu_selection.stop.store(true, Ordering::SeqCst);
    } else {
        debug!("C");
        let roots = static_menu_selection.selected_roots.read().export_roots();
        let search_terms = static_menu_selection
            .search_term_tracker
            .read()
            .search_terms
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<String>>();
        static_menu_selection.run_control_temporarily_disabled
            .store(true, Ordering::SeqCst);
        let stop = static_menu_selection.stop.to_owned();
        let stop_notify = static_menu_selection.stop_notify.to_owned();
        let run_control_temporarily_disabled = static_menu_selection.run_control_temporarily_disabled.to_owned();
        let running = static_menu_selection.running.to_owned();
        let results = static_menu_selection.results.to_owned();
        debug!("D");
        let _ = tokio::spawn(async move {
            debug!("1");
            running.store(true, Ordering::SeqCst);
            debug!("2");
            run_control_temporarily_disabled.store(false, Ordering::SeqCst);
            debug!("3");

            let worker_manager = Arc::new(WorkerManager::new(search_terms, num_cpus::get(), results, stop.to_owned(), stop_notify));

            debug!("4");
            worker_manager.feed_queue(vec!["Software".to_string()]);
            let start_time = Instant::now();
            debug!("E");
            run(worker_manager.to_owned()).await;
            debug!("F");

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
        });
    }
}