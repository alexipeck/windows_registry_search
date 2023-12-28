use std::{time::Instant, collections::BTreeSet};

use ratatui::{text::{Span, Line}, style::{Style, Color}};
use tracing::{error, debug};

use crate::{EditorMode, DEBOUNCE, SELECTION_COLOUR};


pub struct SearchTermTracker {
    pub search_term_selected: usize,
    pub search_term_last_changed: Instant,
    pub search_terms: BTreeSet<String>,
}

impl Default for SearchTermTracker {
    fn default() -> Self {
        Self {
            search_term_selected: 0,
            search_term_last_changed: Instant::now(),
            search_terms: BTreeSet::new(),
        }
    }
}

impl SearchTermTracker {
    fn get_value_from_index(&self, index: usize) -> Option<String> {
        if self.search_terms.is_empty() {
            return None;
        }
        self.search_terms.iter().nth(index).cloned()
    }

    pub fn get_value_at_current_index(&self) -> Option<String> {
        self.get_value_from_index(self.search_term_selected)
    }

    pub fn update(&mut self, editor_mode: EditorMode, state: String) {
        let mut current_index_value = self.get_value_at_current_index();
        if current_index_value.is_none() && self.search_terms.len() > 0 {
            error!("Error retrieving value from search terms by index when map is not empty. Add/Edit action discarded.");
            return;
        }
        match editor_mode {
            EditorMode::Add => {
                let _ = self.search_terms.insert(state);
            }
            EditorMode::Edit(original) => {
                if current_index_value.as_ref().unwrap() == &original {
                    current_index_value = Some(state.to_owned());
                }
                self.search_terms.remove(&original);
                let _ = self.search_terms.insert(state);
            }
        }
        if let Some(current_index_value) = &current_index_value {
            for (index, search_term) in self.search_terms.iter().enumerate() {
                if search_term == current_index_value {
                    if self.search_term_selected != index {
                        self.search_term_selected = index;
                        return;
                    }
                    error!("Current value was not found in ordered map, this is a logic error.");
                }
            }
        } else {
            debug!("No value present to guarantee same entry is selected after modification, map is assumed to have been empty prior.");
        }
    }

    pub fn remove(&mut self, term: String) {}

    pub fn up(&mut self) {
        if self.search_term_last_changed.elapsed() < DEBOUNCE {
            return;
        }
        let search_terms_len = self.search_terms.len();
        if search_terms_len == 0 {
            return;
        }
        let max_index: usize = if search_terms_len > 1 {
            search_terms_len - 1
        } else {
            search_terms_len
        };
        let current = self.search_term_selected;
        self.search_term_selected = if current == 0 { max_index } else { current - 1 };
        self.search_term_last_changed = Instant::now();
    }

    pub fn down(&mut self) {
        if self.search_term_last_changed.elapsed() < DEBOUNCE {
            return;
        }
        let search_terms_len = self.search_terms.len();
        if search_terms_len == 0 {
            return;
        }
        let max_index: usize = if search_terms_len > 1 {
            search_terms_len - 1
        } else {
            search_terms_len
        };
        let current = self.search_term_selected;
        self.search_term_selected = if current + 1 <= max_index {
            current + 1
        } else {
            0
        };
        self.search_term_last_changed = Instant::now();
    }

    pub fn render(&self, pane_selected: bool) -> Vec<Line<'static>> {
        self.search_terms
            .iter()
            .enumerate()
            .map(|(index, term)| {
                Line::from(vec![Span::styled(
                    term.to_string(),
                    Style::default().fg(if pane_selected && index == self.search_term_selected {
                        SELECTION_COLOUR
                    } else {
                        Color::White
                    }),
                )])
            })
            .collect::<Vec<Line>>()
    }
}