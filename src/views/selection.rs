use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub trait SelectionItem {
    fn title(&self) -> &str;
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

        // Create the list widget
        let list = List::new(list_items)
            .block(Block::default().borders(Borders::ALL).title(title.clone()))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, layout_area, self.state_mut());
    }
}

#[derive(Debug, PartialEq)]
pub enum SelectionAction {
    Handled,
    NotHandled,
    Selected(usize),
}
