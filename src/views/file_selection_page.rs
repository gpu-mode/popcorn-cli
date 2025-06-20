use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io;
use std::path::PathBuf;
use std::fs;

pub struct FileSelectionScreen {
    current_dir: PathBuf,
    entries: Vec<PathBuf>,
    state: ListState,
    show_hidden: bool,
}

impl FileSelectionScreen {
    pub fn new() -> io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        let mut screen = Self {
            current_dir: current_dir.clone(),
            entries: Vec::new(),
            state: ListState::default(),
            show_hidden: false,
        };
        screen.load_directory()?;
        screen.state.select(Some(0));
        Ok(screen)
    }

    fn load_directory(&mut self) -> io::Result<()> {
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
                if self.show_hidden {
                    true
                } else {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| !name.starts_with('.'))
                        .unwrap_or(true)
                }
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

    pub async fn run(&mut self) -> io::Result<Option<String>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal).await;

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen
        )?;
        terminal.show_cursor()?;

        result
    }

    async fn run_app<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> io::Result<Option<String>> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => {
                        if let Some(selected) = self.state.selected() {
                            if selected < self.entries.len() {
                                let path = &self.entries[selected];
                                if path.is_dir() {
                                    self.current_dir = path.clone();
                                    self.load_directory()?;
                                    self.state.select(Some(0));
                                } else if path.is_file() {
                                    return Ok(Some(path.to_string_lossy().to_string()));
                                }
                            }
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if let Some(selected) = self.state.selected() {
                            if selected > 0 {
                                self.state.select(Some(selected - 1));
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if let Some(selected) = self.state.selected() {
                            if selected < self.entries.len() - 1 {
                                self.state.select(Some(selected + 1));
                            }
                        }
                    }
                    KeyCode::Char('h') => {
                        self.show_hidden = !self.show_hidden;
                        self.load_directory()?;
                        self.state.select(Some(0));
                    }
                    _ => {}
                }
            }
        }
    }

    fn ui(&self, f: &mut ratatui::Frame) {
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
            .split(f.size());

        // Header
        let header = Paragraph::new(self.current_dir.display().to_string())
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Select Solution File"));
        f.render_widget(header, chunks[0]);

        // File list
        let items: Vec<ListItem> = self.entries
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let is_parent = i == 0 && path.parent().is_some() && path.parent() != Some(&self.current_dir);
                let display_name = if is_parent {
                    "../".to_string()
                } else {
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?");
                    if path.is_dir() {
                        format!("{}/", name)
                    } else {
                        name.to_string()
                    }
                };

                let style = if path.is_dir() {
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
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
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol("> ");

        f.render_stateful_widget(files, chunks[1], &mut self.state.clone());

        // Footer
        let footer_text = if self.show_hidden {
            "↑/↓: Navigate | Enter: Select | h: Hide hidden files | q/Esc: Cancel"
        } else {
            "↑/↓: Navigate | Enter: Select | h: Show hidden files | q/Esc: Cancel"
        };
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(footer, chunks[2]);
    }
}