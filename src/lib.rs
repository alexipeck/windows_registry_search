use std::{time::Duration, sync::{atomic::{AtomicUsize, AtomicBool, Ordering}, Arc}};

use parking_lot::{RwLock, Mutex};
use ratatui::style::Color;
use search_editor::SearchEditor;
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};


pub mod controls;
pub mod renderer;
pub mod static_selection;
pub mod search_term_tracker;
pub mod root;
pub mod search_editor;
pub mod worker_manager;

pub const DEBOUNCE: Duration = Duration::from_millis(100);
pub const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(200);
pub const SELECTION_COLOUR: Color = Color::Cyan;

static SYSTEM_TO_SOLVE_A_DUMB_PROBLEM_TRIPPED: Mutex<bool> = Mutex::new(false);
static SYSTEM_TO_SOLVE_A_DUMB_PROBLEM_ALL_CLEAR: AtomicBool = AtomicBool::new(false);

pub fn function_to_solve_a_dumb_problem() -> bool {
    if !SYSTEM_TO_SOLVE_A_DUMB_PROBLEM_ALL_CLEAR.load(Ordering::SeqCst) {
        let mut lock = SYSTEM_TO_SOLVE_A_DUMB_PROBLEM_TRIPPED.lock();
        let tripped = *lock;
        if !tripped {
            *lock = true;
            SYSTEM_TO_SOLVE_A_DUMB_PROBLEM_ALL_CLEAR.store(true, Ordering::SeqCst);
            return false;
        }
        return true;
    }
    true
}

static KEY_COUNT: AtomicUsize = AtomicUsize::new(0);
static VALUE_COUNT: AtomicUsize = AtomicUsize::new(0);
static HKLM: RegKey = RegKey::predef(HKEY_LOCAL_MACHINE);

const REGEDIT_OUTPUT_FOR_BLANK_NAMES: bool = true;

#[derive(Debug, Clone)]
pub enum EditorMode {
    Add,
    Edit(String),
}

#[derive(Debug, Clone)]
pub enum Focus {
    Main,
    SearchMod(Arc<RwLock<Option<SearchEditor>>>),
    Help,
    ConfirmClose,
}