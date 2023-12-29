use std::fmt;

use strum::EnumIter;
use winreg::{
    enums::{
        HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS,
    },
    RegKey,
};

#[derive(EnumIter, Copy, Clone)]
pub enum Root {
    HkeyClassesRoot = 0,
    HkeyCurrentUser = 1,
    HkeyLocalMachine = 2,
    HkeyUsers = 3,
    HkeyCurrentConfig = 4,
    HkeyPerformanceData = 5,
    HkeyPerformanceText = 6,
    HkeyPerformanceNLSText = 7,
    HkeyDynData = 8,
    HkeyCurrentUserLocalSettings = 9,
}

impl fmt::Display for Root {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::HkeyClassesRoot => "HKEY_CLASSES_ROOT",
                Self::HkeyCurrentUser => "HKEY_CURRENT_USER",
                Self::HkeyLocalMachine => "HKEY_LOCAL_MACHINE",
                Self::HkeyUsers => "HKEY_USERS",
                Self::HkeyCurrentConfig => "HKEY_CURRENT_CONFIG",
                Self::HkeyPerformanceData => "HKEY_PERFORMANCE_DATA",
                Self::HkeyPerformanceText => "HKEY_PERFORMANCE_TEXT",
                Self::HkeyPerformanceNLSText => "HKEY_PERFORMANCE_NLSTEXT",
                Self::HkeyDynData => "HKEY_DYN_DATA",
                Self::HkeyCurrentUserLocalSettings => "HKEY_CURRENT_USER_LOCAL_SETTINGS",
            }
        )
    }
}

impl Root {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::HkeyClassesRoot),
            1 => Some(Self::HkeyCurrentUser),
            2 => Some(Self::HkeyLocalMachine),
            3 => Some(Self::HkeyUsers),
            4 => Some(Self::HkeyCurrentConfig),
            5 => Some(Self::HkeyPerformanceData),
            6 => Some(Self::HkeyPerformanceText),
            7 => Some(Self::HkeyPerformanceNLSText),
            8 => Some(Self::HkeyDynData),
            9 => Some(Self::HkeyCurrentUserLocalSettings),
            _ => None,
        }
    }
}

pub struct SelectedRoots {
    classes_root: bool,
    current_user: bool,
    local_machine: bool,
    users: bool,
    current_config: bool,
    performance_data: bool,
    performance_text: bool,
    performance_nls_text: bool,
    dyn_data: bool,
    current_user_local_settings: bool,
}

impl Default for SelectedRoots {
    fn default() -> Self {
        Self {
            classes_root: false,
            current_user: false,
            local_machine: true,
            users: true,
            current_config: false,
            performance_data: false,
            performance_text: false,
            performance_nls_text: false,
            dyn_data: false,
            current_user_local_settings: false,
        }
    }
}

impl SelectedRoots {
    pub fn export_roots(&self) -> Vec<RegKey> {
        let mut selected_roots = Vec::new();

        if self.classes_root {
            selected_roots.push(RegKey::predef(HKEY_CLASSES_ROOT));
        }
        if self.current_user {
            selected_roots.push(RegKey::predef(HKEY_CURRENT_USER));
        }
        if self.local_machine {
            selected_roots.push(RegKey::predef(HKEY_LOCAL_MACHINE));
        }
        if self.users {
            selected_roots.push(RegKey::predef(HKEY_USERS));
        }
        if self.current_config {
            selected_roots.push(RegKey::predef(HKEY_CURRENT_CONFIG));
        }

        selected_roots
    }

    pub fn is_enabled(&self, root: &Root) -> bool {
        match root {
            Root::HkeyClassesRoot => self.classes_root,
            Root::HkeyCurrentUser => self.current_user,
            Root::HkeyLocalMachine => self.local_machine,
            Root::HkeyUsers => self.users,
            Root::HkeyCurrentConfig => self.current_config,
            Root::HkeyPerformanceData => self.performance_data,
            Root::HkeyPerformanceText => self.performance_text,
            Root::HkeyPerformanceNLSText => self.performance_nls_text,
            Root::HkeyDynData => self.dyn_data,
            Root::HkeyCurrentUserLocalSettings => self.current_user_local_settings,
        }
    }

    pub fn toggle(&mut self, root: &Root) {
        match root {
            Root::HkeyClassesRoot => self.classes_root = !self.classes_root,
            Root::HkeyCurrentUser => self.current_user = !self.current_user,
            Root::HkeyLocalMachine => self.local_machine = !self.local_machine,
            Root::HkeyUsers => self.users = !self.users,
            Root::HkeyCurrentConfig => self.current_config = !self.current_config,
            Root::HkeyPerformanceData => self.performance_data = !self.performance_data,
            Root::HkeyPerformanceText => self.performance_text = !self.performance_text,
            Root::HkeyPerformanceNLSText => self.performance_nls_text = !self.performance_nls_text,
            Root::HkeyDynData => self.dyn_data = !self.dyn_data,
            Root::HkeyCurrentUserLocalSettings => {
                self.current_user_local_settings = !self.current_user_local_settings
            }
        }
    }
}
