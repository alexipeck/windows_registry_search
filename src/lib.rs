use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use parking_lot::{Mutex, RwLock};
use ratatui::style::Color;
use search_editor::SearchEditor;
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

pub mod controls;
pub mod renderer;
pub mod root;
pub mod search_editor;
pub mod search_term_tracker;
pub mod static_selection;
pub mod worker_manager;
pub mod worker_runtime;

pub const DEBOUNCE: Duration = Duration::from_millis(100);
pub const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(200);
pub const SELECTION_COLOUR: Color = Color::Cyan;

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
