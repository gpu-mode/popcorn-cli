use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};
use crate::views::ascii_art::{AsciiArt, create_background_pattern};

// Color constants
pub const COLOR_TITLE: Color = Color::Rgb(218, 119, 86);      // #da7756 - Orange
pub const COLOR_BACKGROUND: Color = Color::Rgb(139, 69, 19);  // Dark orange/brown
pub const COLOR_SELECTED: Color = Color::Yellow;
pub const COLOR_UNSELECTED: Color = Color::Rgb(169, 169, 169); // Light gray

// Layout constants
pub const TITLE_HEIGHT: u16 = 10;
pub const TITLE_SPACING: u16 = 3;
pub const MENU_ITEM_HEIGHT: u16 = 3;
pub const MENU_ITEM_SPACING: u16 = 2;
pub const HORIZONTAL_MARGIN: u16 = 20; // Percentage for centering

#[derive(Debug, PartialEq)]
pub enum WelcomeAction {
    Handled,
    NotHandled,
    Submit,
    ViewHistory,
}

pub struct WelcomeView {
    selected_index: usize,
}

impl WelcomeView {
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> WelcomeAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                WelcomeAction::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_index < 1 { // We have 2 menu items (0 and 1)
                    self.selected_index += 1;
                }
                WelcomeAction::Handled
            }
            KeyCode::Enter => {
                match self.selected_index {
                    0 => WelcomeAction::Submit,
                    1 => WelcomeAction::ViewHistory,
                    _ => WelcomeAction::Handled,
                }
            }
            _ => WelcomeAction::NotHandled
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        // Create a retro background pattern
        let bg_text = create_background_pattern(frame.size().width, frame.size().height);
        let background = Paragraph::new(bg_text)
            .style(Style::default().fg(COLOR_BACKGROUND));
        frame.render_widget(background, frame.size());

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(TITLE_HEIGHT),
                    Constraint::Length(TITLE_SPACING),
                    Constraint::Min(10),
                ]
                .as_ref(),
            )
            .split(frame.size());

        // ASCII art title
        let title_text = AsciiArt::kernelbot_title();
        let title_lines: Vec<Line> = title_text
            .iter()
            .map(|&line| {
                Line::from(vec![Span::styled(
                    line,
                    Style::default()
                        .fg(COLOR_TITLE)
                        .add_modifier(Modifier::BOLD),
                )])
            })
            .collect();

        let title = Paragraph::new(title_lines)
            .alignment(Alignment::Center)
            .block(Block::default());

        frame.render_widget(title, chunks[0]);

        // Menu
        let menu_items = vec!["Submit", "View History"];
        let menu_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(MENU_ITEM_HEIGHT),
                    Constraint::Length(MENU_ITEM_SPACING),
                    Constraint::Length(MENU_ITEM_HEIGHT),
                ]
                .as_ref(),
            )
            .split(chunks[2]);

        // Center the menu horizontally
        let centered_menu_area: Vec<_> = menu_area
            .iter()
            .enumerate()
            .filter_map(|(i, &area)| {
                if i % 2 == 0 {
                    Some(Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(
                            [
                                Constraint::Percentage(HORIZONTAL_MARGIN),
                                Constraint::Percentage(100 - 2 * HORIZONTAL_MARGIN),
                                Constraint::Percentage(HORIZONTAL_MARGIN),
                            ]
                            .as_ref(),
                        )
                        .split(area)[1])
                } else {
                    None
                }
            })
            .collect();

        // Render menu items
        for (i, (item, area)) in menu_items.iter().zip(centered_menu_area.iter()).enumerate() {
            let is_selected = i == self.selected_index;
            
            let menu_lines = match *item {
                "Submit" => AsciiArt::submit_menu_item(is_selected),
                "View History" => AsciiArt::history_menu_item(is_selected),
                _ => continue,
            };
            
            let style = if is_selected {
                Style::default()
                    .fg(COLOR_SELECTED)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(COLOR_UNSELECTED)
            };

            let menu_text = menu_lines.join("\n");
            let menu_item = Paragraph::new(menu_text)
                .style(style)
                .alignment(Alignment::Center);

            frame.render_widget(menu_item, *area);
        }
    }
}
