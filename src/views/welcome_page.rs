use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Terminal,
};
use std::io;
use crate::views::ascii_art::{AsciiArt, create_background_pattern};

pub struct WelcomeScreen {
    selected_index: usize,
    menu_items: Vec<String>,
}

impl WelcomeScreen {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            menu_items: vec!["Submit".to_string(), "View History".to_string()],
        }
    }

    pub async fn run(&mut self) -> io::Result<Option<String>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal).await;

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
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
                        return Ok(Some(self.menu_items[self.selected_index].clone()));
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if self.selected_index < self.menu_items.len() - 1 {
                            self.selected_index += 1;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn ui(&self, f: &mut ratatui::Frame) {
        // Create a retro background pattern
        let bg_text = create_background_pattern(f.size().width, f.size().height);
        let background = Paragraph::new(bg_text)
            .style(Style::default().fg(Color::Rgb(139, 69, 19))); // Dark orange/brown
        f.render_widget(background, f.size());

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(10), // ASCII art title (increased for filled version)
                    Constraint::Length(3),  // Spacing
                    Constraint::Min(5),     // Menu
                ]
                .as_ref(),
            )
            .split(f.size());

        // ASCII art title - filled version
        let title_text = AsciiArt::kernelbot_title();

        let title_lines: Vec<Line> = title_text
            .iter()
            .map(|&line| {
                Line::from(vec![Span::styled(
                    line,
                    Style::default()
                        .fg(Color::Rgb(218, 119, 86))  // #da7756
                        .add_modifier(Modifier::BOLD),
                )])
            })
            .collect();

        let title = Paragraph::new(title_lines)
            .alignment(Alignment::Center)
            .block(Block::default());

        f.render_widget(title, chunks[0]);

        // Menu - arcade style with medium text
        let menu_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),  // First menu item (medium)
                    Constraint::Length(2),  // Spacing
                    Constraint::Length(3),  // Second menu item (medium)
                ]
                .as_ref(),
            )
            .split(chunks[2]);

        // Center the menu horizontally
        let centered_menu_area: Vec<_> = menu_area
            .iter()
            .map(|&area| {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Percentage(20),
                            Constraint::Percentage(60),
                            Constraint::Percentage(20),
                        ]
                        .as_ref(),
                    )
                    .split(area)[1]
            })
            .collect();

        // Render each menu item separately for arcade-style appearance
        for (i, item) in self.menu_items.iter().enumerate() {
            let area_index = i * 2; // Skip spacing constraints
            if area_index < centered_menu_area.len() {
                let is_selected = i == self.selected_index;
                
                // Get ASCII art for menu items
                let menu_lines = match item.as_str() {
                    "Submit" => AsciiArt::submit_menu_item(is_selected),
                    "View History" => AsciiArt::history_menu_item(is_selected),
                    _ => continue,
                };
                
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Rgb(169, 169, 169)) // Light gray
                };

                let menu_text = menu_lines.join("\n");
                let menu_item = Paragraph::new(menu_text)
                    .style(style)
                    .alignment(Alignment::Center);

                f.render_widget(menu_item, centered_menu_area[area_index]);
            }
        }

    }
}
