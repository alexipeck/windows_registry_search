use crate::{
    root::{Root, SelectedRoots},
    search_term_tracker::SearchTermTracker,
    worker_manager::{run, WorkerManager},
    DEBOUNCE, SELECTION_COLOUR,
};
use parking_lot::{Mutex, RwLock};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
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

pub struct StaticSelection {
    pub pane_selected: Arc<AtomicU8>,       //horizontal
    pane_last_changed: Arc<Mutex<Instant>>, //horizontal

    pub search_term_tracker: Arc<RwLock<SearchTermTracker>>,

    root_selected: Arc<AtomicU8>,
    root_selection_last_changed: Arc<Mutex<Instant>>,

    pub selected_roots: Arc<RwLock<SelectedRoots>>,

    pub running: Arc<AtomicBool>,
    pub run_control_temporarily_disabled: Arc<AtomicBool>, //running thread resets this once closed
    pub stop: Arc<AtomicBool>,                             //running thread resets this once closed
    pub stop_notify: Arc<Notify>,

    pub results: Arc<Mutex<HashSet<String>>>,
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
        self.results
            .lock()
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
