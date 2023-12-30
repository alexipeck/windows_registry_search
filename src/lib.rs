use std::{
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use parking_lot::RwLock;
use ratatui::style::Color;
use search_editor::SearchEditor;
use winreg::{enums::RegType, RegValue};

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

pub static KEY_COUNT: AtomicUsize = AtomicUsize::new(0);
pub static VALUE_COUNT: AtomicUsize = AtomicUsize::new(0);

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

pub fn alt_reg_value_to_string(reg_value: RegValue) -> String {
    match reg_value.vtype {
        RegType::REG_SZ | RegType::REG_EXPAND_SZ => {
            String::from_utf8_lossy(&reg_value.bytes).to_string()
        }
        RegType::REG_BINARY => {
            format!("BIN_LENGTH: {}", reg_value.bytes.len())
        }
        RegType::REG_DWORD => {
            let u32 = match reg_value.bytes.try_into() {
                Ok(t) => t,
                Err(_err) => return "Invalid REG_DWORD".into(),
            };
            u32::from_le_bytes(u32).to_string()
        }
        RegType::REG_DWORD_BIG_ENDIAN => {
            let u32 = match reg_value.bytes.try_into() {
                Ok(t) => t,
                Err(_err) => return "Invalid REG_DWORD_BIG_ENDIAN".into(),
            };
            u32::from_be_bytes(u32).to_string()
        }
        RegType::REG_QWORD => {
            let u64 = match reg_value.bytes.try_into() {
                Ok(t) => t,
                Err(_err) => return "Invalid REG_DWORD".into(),
            };
            u64::from_le_bytes(u64).to_string()
        }
        RegType::REG_MULTI_SZ | RegType::REG_RESOURCE_LIST => {
            // Split at null bytes and join
            reg_value
                .bytes
                .split(|&b| b == 0)
                .filter_map(|s| std::str::from_utf8(s).ok())
                .collect::<Vec<&str>>()
                .join(", ")
        }
        RegType::REG_LINK
        | RegType::REG_FULL_RESOURCE_DESCRIPTOR
        | RegType::REG_RESOURCE_REQUIREMENTS_LIST => reg_value.to_string(),
        RegType::REG_NONE => "None".into(),
    }
}
