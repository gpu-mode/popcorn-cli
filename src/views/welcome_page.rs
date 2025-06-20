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

// Color constants
const COLOR_TITLE: Color = Color::Rgb(218, 119, 86);      // #da7756 - Orange
const COLOR_BACKGROUND: Color = Color::Rgb(139, 69, 19);  // Dark orange/brown
const COLOR_SELECTED: Color = Color::Yellow;
const COLOR_UNSELECTED: Color = Color::Rgb(169, 169, 169); // Light gray

// Layout constants
const TITLE_HEIGHT: u16 = 10;
const TITLE_SPACING: u16 = 3;
const MENU_ITEM_HEIGHT: u16 = 3;
const MENU_ITEM_SPACING: u16 = 2;
const HORIZONTAL_MARGIN: u16 = 20; // Percentage for centering

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

    fn create_centered_menu_areas(&self, menu_chunk: ratatui::layout::Rect) -> Vec<ratatui::layout::Rect> {
        // Create menu areas with spacing
        let menu_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                self.menu_items
                    .iter()
                    .enumerate()
                    .flat_map(|(i, _)| {
                        if i == 0 {
                            vec![Constraint::Length(MENU_ITEM_HEIGHT)]
                        } else {
                            vec![Constraint::Length(MENU_ITEM_SPACING), Constraint::Length(MENU_ITEM_HEIGHT)]
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .split(menu_chunk);

        // Center each menu area horizontally
        menu_areas
            .iter()
            .enumerate()
            .filter_map(|(i, &area)| {
                // Only return the actual menu item areas, not the spacing
                if i % 2 == 0 {
                    Some(self.center_horizontally(area))
                } else {
                    None
                }
            })
            .collect()
    }

    fn center_horizontally(&self, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(HORIZONTAL_MARGIN),
                    Constraint::Percentage(100 - 2 * HORIZONTAL_MARGIN),
                    Constraint::Percentage(HORIZONTAL_MARGIN),
                ]
                .as_ref(),
            )
            .split(area)[1]
    }

    fn render_background(&self, f: &mut ratatui::Frame) {
        let bg_text = create_background_pattern(f.size().width, f.size().height);
        let background = Paragraph::new(bg_text)
            .style(Style::default().fg(COLOR_BACKGROUND));
        f.render_widget(background, f.size());
    }

    fn render_title(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
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

        f.render_widget(title, area);
    }

    fn render_menu_item(&self, f: &mut ratatui::Frame, item: &str, area: ratatui::layout::Rect, is_selected: bool) {
        let menu_lines = match item {
            "Submit" => AsciiArt::submit_menu_item(is_selected),
            "View History" => AsciiArt::history_menu_item(is_selected),
            _ => return,
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

        f.render_widget(menu_item, area);
    }

    fn ui(&self, f: &mut ratatui::Frame) {
        // Render background
        self.render_background(f);

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(TITLE_HEIGHT),
                    Constraint::Length(TITLE_SPACING),
                    Constraint::Min(5), // Menu
                ]
                .as_ref(),
            )
            .split(f.size());

        // Render title
        self.render_title(f, chunks[0]);

        // Create centered menu areas
        let menu_areas = self.create_centered_menu_areas(chunks[2]);

        // Render menu items
        for (i, (item, area)) in self.menu_items.iter().zip(menu_areas.iter()).enumerate() {
            self.render_menu_item(f, item, *area, i == self.selected_index);
        }
    }
}
