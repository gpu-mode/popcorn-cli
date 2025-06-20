use serde::{Deserialize, Serialize};
use ratatui::widgets::ListItem;
use ratatui::text::{Line, Span};
use ratatui::style::{Style, Color};
use crate::views::selection::SelectionItem;

#[derive(Clone, Debug)]
pub struct LeaderboardItem {
    pub title_text: String,
    pub task_description: String,
}

impl LeaderboardItem {
    pub fn new(title_text: String, task_description: String) -> Self {
        Self {
            title_text,
            task_description,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GpuItem {
    pub title_text: String,
}

impl GpuItem {
    pub fn new(title_text: String) -> Self {
        Self { title_text }
    }
}

#[derive(Clone, Debug)]
pub struct SubmissionModeItem {
    pub title_text: String,
    pub description_text: String,
    pub value: String,
}

impl SubmissionModeItem {
    pub fn new(title_text: String, description_text: String, value: String) -> Self {
        Self {
            title_text,
            description_text,
            value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum AppState {
    #[default]
    Welcome,
    FileSelection,
    LeaderboardSelection,
    GpuSelection,
    SubmissionModeSelection,
    WaitingForResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmissionResultMsg(pub String);

impl SelectionItem for LeaderboardItem {
    fn title(&self) -> &str {
        &self.title_text
    }
    
    fn description(&self) -> Option<&str> {
        Some(&self.task_description)
    }
    
    fn to_list_item(&self, _available_width: usize) -> ListItem {
        let title_line = Line::from(vec![
            Span::styled(self.title_text.clone(), Style::default().fg(Color::White)),
        ]);
        
        let description_line = Line::from(vec![
            Span::styled(self.task_description.clone(), Style::default().fg(Color::Gray)),
        ]);
        
        ListItem::new(vec![title_line, description_line])
    }
}

impl SelectionItem for GpuItem {
    fn title(&self) -> &str {
        &self.title_text
    }
    
    fn to_list_item(&self, _available_width: usize) -> ListItem {
        let title_line = Line::from(vec![
            Span::styled(self.title_text.clone(), Style::default().fg(Color::White)),
        ]);
        
        ListItem::new(vec![title_line])
    }
}

impl SelectionItem for SubmissionModeItem {
    fn title(&self) -> &str {
        &self.title_text
    }
    
    fn description(&self) -> Option<&str> {
        Some(&self.description_text)
    }
    
    fn to_list_item(&self, _available_width: usize) -> ListItem {
        let title_line = Line::from(vec![
            Span::styled(self.title_text.clone(), Style::default().fg(Color::White)),
        ]);
        
        let description_line = Line::from(vec![
            Span::styled(self.description_text.clone(), Style::default().fg(Color::Gray)),
        ]);
        
        ListItem::new(vec![title_line, description_line])
    }
}
