use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{anyhow, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen};
use ratatui::prelude::*;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::models::{GpuItem, LeaderboardItem, ModelState, SubmissionModeItem};
use crate::service;
use crate::utils;

pub struct App {
    pub filepath: String,
    pub leaderboards: Vec<LeaderboardItem>,
    pub leaderboards_state: ListState,
    pub selected_leaderboard: Option<String>,
    pub gpus: Vec<GpuItem>,
    pub gpus_state: ListState,
    pub selected_gpu: Option<String>,
    pub submission_modes: Vec<SubmissionModeItem>,
    pub submission_modes_state: ListState,
    pub selected_submission_mode: Option<String>,
    pub modal_state: ModelState,
    pub final_status: Option<String>,
    pub is_loading: bool,
    pub should_quit: bool,
}

impl App {
    pub fn new<P: AsRef<Path>>(filepath: P) -> Self {
        let submission_modes = vec![
            SubmissionModeItem::new(
                "Test".to_string(),
                "Test the solution and give detailed results about passed/failed tests.".to_string(),
                "test".to_string(),
            ),
            SubmissionModeItem::new(
                "Benchmark".to_string(),
                "Benchmark the solution, this also runs the tests and afterwards runs the benchmark, returning detailed timing results".to_string(),
                "benchmark".to_string(),
            ),
            SubmissionModeItem::new(
                "Leaderboard".to_string(),
                "Submit to the leaderboard, this first runs public tests and then private tests. If both pass, the submission is evaluated and submit to the leaderboard.".to_string(),
                "leaderboard".to_string(),
            ),
            SubmissionModeItem::new(
                "Private".to_string(),
                "TODO".to_string(),
                "private".to_string(),
            ),
            SubmissionModeItem::new(
                "Script".to_string(),
                "TODO".to_string(),
                "script".to_string(),
            ),
            SubmissionModeItem::new(
                "Profile".to_string(),
                "TODO".to_string(),
                "profile".to_string(),
            ),
        ];

        let mut app = Self {
            filepath: filepath.as_ref().to_string_lossy().to_string(),
            leaderboards: Vec::new(),
            leaderboards_state: ListState::default(),
            selected_leaderboard: None,
            gpus: Vec::new(),
            gpus_state: ListState::default(),
            selected_gpu: None,
            submission_modes,
            submission_modes_state: ListState::default(),
            selected_submission_mode: None,
            modal_state: ModelState::LeaderboardSelection,
            final_status: None,
            is_loading: false,
            should_quit: false,
        };

        // Initialize list states
        app.leaderboards_state.select(Some(0));
        app.gpus_state.select(Some(0));
        app.submission_modes_state.select(Some(0));

        app
    }

    pub fn initialize_with_directives(&mut self, popcorn_directives: utils::PopcornDirectives) {
        if !popcorn_directives.leaderboard_name.is_empty() {
            self.selected_leaderboard = Some(popcorn_directives.leaderboard_name);

            if !popcorn_directives.gpus.is_empty() {
                self.selected_gpu = Some(popcorn_directives.gpus[0].clone());
                self.modal_state = ModelState::SubmissionModeSelection;
            } else {
                self.modal_state = ModelState::GpuSelection;
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('c')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.should_quit = true;
                return Ok(true);
            }
            KeyCode::Enter => match self.modal_state {
                ModelState::LeaderboardSelection => {
                    if let Some(idx) = self.leaderboards_state.selected() {
                        if idx < self.leaderboards.len() {
                            self.selected_leaderboard =
                                Some(self.leaderboards[idx].title_text.clone());

                            if self.selected_gpu.is_none() {
                                self.modal_state = ModelState::GpuSelection;
                            } else {
                                self.modal_state = ModelState::SubmissionModeSelection;
                            }
                            return Ok(true);
                        }
                    }
                }
                ModelState::GpuSelection => {
                    if let Some(idx) = self.gpus_state.selected() {
                        if idx < self.gpus.len() {
                            self.selected_gpu = Some(self.gpus[idx].title_text.clone());
                            self.modal_state = ModelState::SubmissionModeSelection;
                            return Ok(true);
                        }
                    }
                }
                ModelState::SubmissionModeSelection => {
                    if let Some(idx) = self.submission_modes_state.selected() {
                        if idx < self.submission_modes.len() {
                            self.selected_submission_mode =
                                Some(self.submission_modes[idx].value.clone());
                            self.modal_state = ModelState::WaitingForResult;
                            self.is_loading = true;
                            return Ok(true);
                        }
                    }
                }
                _ => {}
            },
            KeyCode::Up => {
                self.move_selection_up();
                return Ok(true);
            }
            KeyCode::Down => {
                self.move_selection_down();
                return Ok(true);
            }
            _ => {}
        }

        Ok(false)
    }

    fn move_selection_up(&mut self) {
        match self.modal_state {
            ModelState::LeaderboardSelection => {
                if let Some(idx) = self.leaderboards_state.selected() {
                    if idx > 0 {
                        self.leaderboards_state.select(Some(idx - 1));
                    }
                }
            }
            ModelState::GpuSelection => {
                if let Some(idx) = self.gpus_state.selected() {
                    if idx > 0 {
                        self.gpus_state.select(Some(idx - 1));
                    }
                }
            }
            ModelState::SubmissionModeSelection => {
                if let Some(idx) = self.submission_modes_state.selected() {
                    if idx > 0 {
                        self.submission_modes_state.select(Some(idx - 1));
                    }
                }
            }
            _ => {}
        }
    }

    fn move_selection_down(&mut self) {
        match self.modal_state {
            ModelState::LeaderboardSelection => {
                if let Some(idx) = self.leaderboards_state.selected() {
                    if idx < self.leaderboards.len() - 1 {
                        self.leaderboards_state.select(Some(idx + 1));
                    }
                }
            }
            ModelState::GpuSelection => {
                if let Some(idx) = self.gpus_state.selected() {
                    if idx < self.gpus.len() - 1 {
                        self.gpus_state.select(Some(idx + 1));
                    }
                }
            }
            ModelState::SubmissionModeSelection => {
                if let Some(idx) = self.submission_modes_state.selected() {
                    if idx < self.submission_modes.len() - 1 {
                        self.submission_modes_state.select(Some(idx + 1));
                    }
                }
            }
            _ => {}
        }
    }

    pub async fn load_leaderboards(&mut self) -> Result<()> {
        match service::fetch_leaderboards().await {
            Ok(leaderboards) => {
                self.leaderboards = leaderboards;
            }
            Err(e) => {
                return Err(e);
            }
        }
        Ok(())
    }
    pub async fn load_gpus(&mut self) -> Result<()> {
        if let Some(leaderboard) = &self.selected_leaderboard {
            match service::fetch_available_gpus(leaderboard).await {
                Ok(gpus) => {
                    self.gpus = gpus;
                    if self.gpus.is_empty() {
                        return Err(anyhow!("No GPUs available for this leaderboard."));
                    }
                }
                Err(e) => {
                    if e.to_string().contains("Invalid leaderboard name") {
                        return Err(anyhow!("Invalid leaderboard name: '{}'. Please check if the leaderboard exists.", leaderboard));
                    }
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    pub async fn submit_solution(&mut self) -> Result<()> {
        let leaderboard = self
            .selected_leaderboard
            .as_ref()
            .ok_or_else(|| anyhow!("No leaderboard selected"))?;

        let gpu = self
            .selected_gpu
            .as_ref()
            .ok_or_else(|| anyhow!("No GPU selected"))?;

        let submission_mode = self
            .selected_submission_mode
            .as_ref()
            .ok_or_else(|| anyhow!("No submission mode selected"))?;

        let file_content = fs::read(&self.filepath)?;

        let result = service::submit_solution(
            leaderboard,
            gpu,
            submission_mode,
            &self.filepath,
            &file_content,
        )
        .await;

        match result {
            Ok(result) => {
                self.final_status = Some(result);
                self.should_quit = true;
            }
            Err(e) => {
                return Err(e);
            }
        }

        Ok(())
    }
}

pub fn ui(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .margin(1)
        .constraints([Constraint::Min(0)].as_ref())
        .split(frame.size());

    match app.modal_state {
        ModelState::LeaderboardSelection => {
            let items: Vec<ListItem> = app
                .leaderboards
                .iter()
                .map(|item| ListItem::new(format!("{}\n{}", item.title(), item.description())))
                .collect();

            let list = List::new(items)
                .block(Block::default().title("Leaderboards").borders(Borders::ALL))
                .highlight_style(Style::default().bg(Color::White).fg(Color::Black));

            frame.render_stateful_widget(list, chunks[0], &mut app.leaderboards_state.clone());
        }
        ModelState::GpuSelection => {
            let items: Vec<ListItem> = app
                .gpus
                .iter()
                .map(|item| ListItem::new(item.title()))
                .collect();

            let list = List::new(items)
                .block(Block::default().title("GPUs").borders(Borders::ALL))
                .highlight_style(Style::default().bg(Color::White).fg(Color::Black));

            frame.render_stateful_widget(list, chunks[0], &mut app.gpus_state.clone());
        }
        ModelState::SubmissionModeSelection => {
            let items: Vec<ListItem> = app
                .submission_modes
                .iter()
                .map(|item| ListItem::new(format!("{}\n{}", item.title(), item.description())))
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .title("Submission Mode")
                        .borders(Borders::ALL),
                )
                .highlight_style(Style::default().bg(Color::White).fg(Color::Black));

            frame.render_stateful_widget(list, chunks[0], &mut app.submission_modes_state.clone());
        }
        ModelState::WaitingForResult => {
            let text = "Submitting solution... press Ctrl+C to quit";

            let paragraph = Paragraph::new(text)
                .block(Block::default().title("Status").borders(Borders::ALL))
                .alignment(Alignment::Center);

            frame.render_widget(paragraph, chunks[0]);
        }
    }
}

pub async fn execute() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage: popcorn <filepath>");
        return Ok(());
    }

    let filepath = &args[1];
    let path = Path::new(filepath);

    if !path.exists() {
        println!("File does not exist: {}", filepath);
        return Ok(());
    }

    let (popcorn_directives, has_multiple_gpus) = utils::get_popcorn_directives(filepath)?;

    if has_multiple_gpus {
        println!("Error: multiple GPUs are not yet supported, continue with the first gpu? ({}) [y/N]", popcorn_directives.gpus[0]);
        print!("Continue? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() != "y" {
            return Ok(());
        }
    }

    // Initialize app
    let mut app = App::new(filepath);
    app.initialize_with_directives(popcorn_directives);

    // Initialize terminal
    enable_raw_mode()?;
    crossterm::execute!(io::stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Load initial data
    if app.modal_state == ModelState::LeaderboardSelection {
        match app.load_leaderboards().await {
            Ok(_) => {}
            Err(e) => {
                app.final_status = Some(format!("Error: {}", e));
                app.should_quit = true;
            }
        }
    }

    if app.modal_state == ModelState::GpuSelection {
        match app.load_gpus().await {
            Ok(_) => {}
            Err(e) => {
                app.final_status = Some(format!("Error: {}", e));
                app.should_quit = true;
            }
        }
    }

    // Main event loop
    loop {
        terminal.draw(|frame| ui(&app, frame))?;

        if app.is_loading && app.modal_state == ModelState::WaitingForResult {
            if let Err(e) = app.submit_solution().await {
                app.final_status = Some(format!("Error: {}", e));
                app.should_quit = true;
            }
            app.is_loading = false;
        }

        if app.should_quit {
            break;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                app.handle_key_event(key)?;
            }
        }
    }

    terminal.clear()?;
    disable_raw_mode()?;

    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;

    std::thread::sleep(std::time::Duration::from_millis(100));

    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0)
    )?;

    utils::display_ascii_art();

    if let Some(status) = app.final_status {
        println!("\nResult:\n\n{}\n", status);
    }

    Ok(())
}
