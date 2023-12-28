use ratatui::{text::{Line, Span}, style::{Style, Color}};

use crate::EditorMode;


#[derive(Debug, Clone)]
pub struct SearchEditor {
    mode: EditorMode,
    state: String,
}

impl SearchEditor {
    pub fn new_add() -> Self {
        Self {
            mode: EditorMode::Add,
            state: String::new(),
        }
    }
    pub fn new_edit(original: String) -> Self {
        Self {
            mode: EditorMode::Edit(original.to_owned()),
            state: original,
        }
    }
    pub fn add_char(&mut self, ch: char) {
        self.state.push(ch);
    }
    pub fn backspace(&mut self) {
        let _ = self.state.pop();
    }
    pub fn resolve(self) -> (EditorMode, String) {
        (self.mode, self.state)
    }

    pub fn render(&self) -> Line<'static> {
        Line::from(vec![Span::styled(
            format!("{}", self.state),
            Style::default().fg(Color::White),
        )])
    }
}