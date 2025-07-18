use serde::{Deserialize, Serialize};
use ratatui::widgets::{ListItem, Block, Borders, List, ListState};
use ratatui::text::{Line, Span};
use ratatui::style::{Style, Color};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};
use crossterm::event::{KeyCode, KeyEvent};
use crate::utils;

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
    ShowingResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmissionResultMsg(pub String);

pub trait SelectionItem {
    fn title(&self) -> &str;

    #[allow(dead_code)]
    fn description(&self) -> Option<&str> {
        None
    }
    fn to_list_item(&self, available_width: usize) -> ListItem;
}

pub trait SelectionView<T: SelectionItem + std::clone::Clone> {
    fn title(&self) -> String;
    fn items(&self) -> &[T];
    fn state(&self) -> &ListState;
    fn state_mut(&mut self) -> &mut ListState;

    fn handle_key_event(&mut self, key: KeyEvent) -> SelectionAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(selected) = self.state().selected() {
                    if selected > 0 {
                        self.state_mut().select(Some(selected - 1));
                    }
                }
                SelectionAction::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(selected) = self.state().selected() {
                    if selected < self.items().len().saturating_sub(1) {
                        self.state_mut().select(Some(selected + 1));
                    }
                }
                SelectionAction::Handled
            }
            KeyCode::Enter => {
                if let Some(selected) = self.state().selected() {
                    if selected < self.items().len() {
                        SelectionAction::Selected(selected)
                    } else {
                        SelectionAction::Handled
                    }
                } else {
                    SelectionAction::Handled
                }
            }
            _ => SelectionAction::NotHandled,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)].as_ref())
            .split(frame.size());

        let list_area = main_layout[0];
        let available_width = list_area.width.saturating_sub(4) as usize;

        // Get all the data we need first to avoid borrowing conflicts
        let title = self.title().to_string();
        let layout_area = main_layout[0];

        let items = self.items().to_vec();

        let list_items: Vec<ListItem> = items
            .iter()
            .map(|item| item.to_list_item(available_width))
            .collect();

        // Create the list widget with orange theme colors
        let list = List::new(list_items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(title.clone())
                .border_style(Style::default().fg(Color::Rgb(218, 119, 86))))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default()
                .bg(Color::Rgb(218, 119, 86))
                .fg(Color::White))
            .highlight_symbol("► ");

        frame.render_stateful_widget(list, layout_area, self.state_mut());
    }
}

#[derive(Debug, PartialEq)]
pub enum SelectionAction {
    Handled,
    NotHandled,
    Selected(usize),
}

impl SelectionItem for LeaderboardItem {
    fn title(&self) -> &str {
        &self.title_text
    }
    
    fn description(&self) -> Option<&str> {
        Some(&self.task_description)
    }
    
    fn to_list_item(&self, available_width: usize) -> ListItem {
        let title_line = Line::from(vec![
            Span::styled(self.title_text.clone(), Style::default().fg(Color::White)),
        ]);
        
        // Wrap long descriptions to fit available width
        let max_desc_width = available_width.saturating_sub(2); // Leave some padding
        let wrapped_lines = utils::wrap_text(&self.task_description, max_desc_width);
        
        let mut lines = vec![title_line];
        for wrapped_line in wrapped_lines {
            lines.push(Line::from(vec![
                Span::styled(wrapped_line, Style::default().fg(Color::Gray)),
            ]));
        }
        
        ListItem::new(lines)
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
    
    fn to_list_item(&self, available_width: usize) -> ListItem {
        let title_line = Line::from(vec![
            Span::styled(self.title_text.clone(), Style::default().fg(Color::White)),
        ]);
        
        // Wrap long descriptions to fit available width
        let max_desc_width = available_width.saturating_sub(2); // Leave some padding
        let wrapped_lines = utils::wrap_text(&self.description_text, max_desc_width);
        
        let mut lines = vec![title_line];
        for wrapped_line in wrapped_lines {
            lines.push(Line::from(vec![
                Span::styled(wrapped_line, Style::default().fg(Color::Gray)),
            ]));
        }
        
        ListItem::new(lines)
    }
}

// Selection view implementations for different types
pub struct LeaderboardSelectionView {
    leaderboards: Vec<LeaderboardItem>,
    leaderboards_state: ListState,
}

impl LeaderboardSelectionView {
    pub fn new(leaderboards: Vec<LeaderboardItem>) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            leaderboards,
            leaderboards_state: state,
        }
    }
}

impl SelectionView<LeaderboardItem> for LeaderboardSelectionView {
    fn title(&self) -> String {
        "Select Leaderboard".to_string()
    }
    
    fn items(&self) -> &[LeaderboardItem] {
        &self.leaderboards
    }
    
    fn state(&self) -> &ListState {
        &self.leaderboards_state
    }
    
    fn state_mut(&mut self) -> &mut ListState {
        &mut self.leaderboards_state
    }
}

pub struct GpuSelectionView {
    gpus: Vec<GpuItem>,
    gpus_state: ListState,
    leaderboard_name: String,
}

impl GpuSelectionView {
    pub fn new(gpus: Vec<GpuItem>, leaderboard_name: String) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            gpus,
            gpus_state: state,
            leaderboard_name,
        }
    }
}

impl SelectionView<GpuItem> for GpuSelectionView {
    fn title(&self) -> String {
        format!("Select GPU for '{}'", self.leaderboard_name)
    }
    
    fn items(&self) -> &[GpuItem] {
        &self.gpus
    }
    
    fn state(&self) -> &ListState {
        &self.gpus_state
    }
    
    fn state_mut(&mut self) -> &mut ListState {
        &mut self.gpus_state
    }
}

pub struct SubmissionModeSelectionView {
    submission_modes: Vec<SubmissionModeItem>,
    submission_modes_state: ListState,
    leaderboard_name: String,
    gpu_name: String,
}

impl SubmissionModeSelectionView {
    pub fn new(submission_modes: Vec<SubmissionModeItem>, leaderboard_name: String, gpu_name: String) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            submission_modes,
            submission_modes_state: state,
            leaderboard_name,
            gpu_name,
        }
    }
}

impl SelectionView<SubmissionModeItem> for SubmissionModeSelectionView {
    fn title(&self) -> String {
        format!("Select Submission Mode for '{}' on '{}'", self.leaderboard_name, self.gpu_name)
    }
    
    fn items(&self) -> &[SubmissionModeItem] {
        &self.submission_modes
    }
    
    fn state(&self) -> &ListState {
        &self.submission_modes_state
    }
    
    fn state_mut(&mut self) -> &mut ListState {
        &mut self.submission_modes_state
    }
}
