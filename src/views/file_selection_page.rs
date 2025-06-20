use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum FileSelectionAction {
    Handled,
    NotHandled,
    FileSelected(String),
}

pub struct FileSelectionView {
    current_dir: PathBuf,
    entries: Vec<PathBuf>,
    state: ListState,
}

impl FileSelectionView {
    pub fn new() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        let mut view = Self {
            current_dir,
            entries: Vec::new(),
            state: ListState::default(),
        };
        view.load_directory()?;
        view.state.select(Some(0));
        Ok(view)
    }

    pub fn load_directory(&mut self) -> Result<()> {
        self.entries.clear();

        // Add parent directory option
        if let Some(parent) = self.current_dir.parent() {
            self.entries.push(parent.to_path_buf());
        }

        // Read directory entries
        let mut entries: Vec<PathBuf> = fs::read_dir(&self.current_dir)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| !name.starts_with('.'))
                    .unwrap_or(true)
            })
            .collect();

        // Sort directories first, then files
        entries.sort_by(|a, b| {
            let a_is_dir = a.is_dir();
            let b_is_dir = b.is_dir();
            if a_is_dir != b_is_dir {
                b_is_dir.cmp(&a_is_dir)
            } else {
                a.file_name().cmp(&b.file_name())
            }
        });

        self.entries.extend(entries);
        Ok(())
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<FileSelectionAction> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(selected) = self.state.selected() {
                    if selected > 0 {
                        self.state.select(Some(selected - 1));
                    }
                }
                Ok(FileSelectionAction::Handled)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(selected) = self.state.selected() {
                    if selected < self.entries.len().saturating_sub(1) {
                        self.state.select(Some(selected + 1));
                    }
                }
                Ok(FileSelectionAction::Handled)
            }
            KeyCode::Enter => {
                if let Some(selected) = self.state.selected() {
                    if selected < self.entries.len() {
                        let path = &self.entries[selected];
                        if path.is_dir() {
                            self.current_dir = path.clone();
                            self.load_directory()?;
                            self.state.select(Some(0));
                            Ok(FileSelectionAction::Handled)
                        } else if path.is_file() {
                            Ok(FileSelectionAction::FileSelected(
                                path.to_string_lossy().to_string(),
                            ))
                        } else {
                            Ok(FileSelectionAction::Handled)
                        }
                    } else {
                        Ok(FileSelectionAction::Handled)
                    }
                } else {
                    Ok(FileSelectionAction::Handled)
                }
            }
            _ => Ok(FileSelectionAction::NotHandled),
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(3),
                ]
                .as_ref(),
            )
            .split(frame.size());

        // Header
        let header = Paragraph::new(self.current_dir.display().to_string())
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Solution File"),
            );
        frame.render_widget(header, chunks[0]);

        // File list
        let items: Vec<ListItem> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let is_parent =
                    i == 0 && path.parent().is_some() && path.parent() != Some(&self.current_dir);
                let display_name = if is_parent {
                    "../".to_string()
                } else {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                    if path.is_dir() {
                        format!("{}/", name)
                    } else {
                        name.to_string()
                    }
                };

                let style = if path.is_dir() {
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD)
                } else if path.extension().and_then(|e| e.to_str()) == Some("py") {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(Span::styled(display_name, style)))
            })
            .collect();

        let files = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(files, chunks[1], &mut self.state);

        // Footer
        let footer_text = "↑/↓: Navigate | Enter: Select | q/Esc: Cancel";
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(footer, chunks[2]);
    }
}
