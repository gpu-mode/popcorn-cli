use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{anyhow, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen};
use ratatui::prelude::*;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use tokio::task::JoinHandle;

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
    pub loading_message: Option<String>,
    pub should_quit: bool,
    pub submission_task: Option<JoinHandle<Result<String, anyhow::Error>>>,
    pub leaderboards_task: Option<JoinHandle<Result<Vec<LeaderboardItem>, anyhow::Error>>>,
    pub gpus_task: Option<JoinHandle<Result<Vec<GpuItem>, anyhow::Error>>>,
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
            loading_message: None,
            should_quit: false,
            submission_task: None,
            leaderboards_task: None,
            gpus_task: None,
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
        } else if !popcorn_directives.gpus.is_empty() {
            self.selected_gpu = Some(popcorn_directives.gpus[0].clone());
            if !popcorn_directives.leaderboard_name.is_empty() {
                self.selected_leaderboard = Some(popcorn_directives.leaderboard_name);
                self.modal_state = ModelState::SubmissionModeSelection;
            } else {
                self.modal_state = ModelState::LeaderboardSelection;
            }
        } else {
            self.modal_state = ModelState::LeaderboardSelection;
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        // Allow quitting anytime, even while loading
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return Ok(true);
        }

        // Ignore other keys while loading
        if self.loading_message.is_some() {
            return Ok(false);
        }

        match key.code {
            KeyCode::Char('q') => {
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
                                // Spawn GPU loading task
                                if let Err(e) = self.spawn_load_gpus() {
                                    self.set_error_and_quit(format!(
                                        "Error starting GPU fetch: {}",
                                        e
                                    ));
                                }
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
                            self.modal_state = ModelState::WaitingForResult; // State for logic, UI uses loading msg
                                                                             // Spawn the submission task
                            if let Err(e) = self.spawn_submit_solution() {
                                self.set_error_and_quit(format!(
                                    "Error starting submission: {}",
                                    e
                                ));
                            }
                            return Ok(true);
                        }
                    }
                }
                _ => {} // WaitingForResult state doesn't handle Enter
            },
            KeyCode::Up => {
                self.move_selection_up();
                return Ok(true);
            }
            KeyCode::Down => {
                self.move_selection_down();
                return Ok(true);
            }
            _ => {} // Ignore other keys
        }

        Ok(false)
    }

    // Helper to reduce repetition
    fn set_error_and_quit(&mut self, error_message: String) {
        self.final_status = Some(error_message);
        self.should_quit = true;
        self.loading_message = None; // Clear loading on error
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
                    if idx < self.leaderboards.len().saturating_sub(1) {
                        self.leaderboards_state.select(Some(idx + 1));
                    }
                }
            }
            ModelState::GpuSelection => {
                if let Some(idx) = self.gpus_state.selected() {
                    if idx < self.gpus.len().saturating_sub(1) {
                        self.gpus_state.select(Some(idx + 1));
                    }
                }
            }
            ModelState::SubmissionModeSelection => {
                if let Some(idx) = self.submission_modes_state.selected() {
                    if idx < self.submission_modes.len().saturating_sub(1) {
                        self.submission_modes_state.select(Some(idx + 1));
                    }
                }
            }
            _ => {}
        }
    }

    pub fn spawn_load_leaderboards(&mut self) -> Result<()> {
        if self.leaderboards_task.is_some() {
            return Ok(());
        }
        self.loading_message = Some("Fetching leaderboards...".to_string());
        let handle = tokio::spawn(async { service::fetch_leaderboards().await });
        self.leaderboards_task = Some(handle);
        Ok(())
    }

    pub fn spawn_load_gpus(&mut self) -> Result<()> {
        if self.gpus_task.is_some() {
            return Ok(());
        }
        let leaderboard = self
            .selected_leaderboard
            .clone()
            .ok_or_else(|| anyhow!("Cannot load GPUs without a selected leaderboard."))?;

        self.loading_message = Some("Fetching GPUs...".to_string());

        let handle = tokio::spawn(async move { service::fetch_available_gpus(&leaderboard).await });
        self.gpus_task = Some(handle);
        Ok(())
    }

    pub fn spawn_submit_solution(&mut self) -> Result<()> {
        if self.submission_task.is_some() {
            return Ok(());
        }
        let leaderboard = self
            .selected_leaderboard
            .clone()
            .ok_or_else(|| anyhow!("Internal Error: No leaderboard selected"))?;

        let gpu = self
            .selected_gpu
            .clone()
            .ok_or_else(|| anyhow!("Internal Error: No GPU selected"))?;

        let submission_mode = self
            .selected_submission_mode
            .clone()
            .ok_or_else(|| anyhow!("Internal Error: No submission mode selected"))?;

        let filepath = self.filepath.clone();

        self.loading_message = Some("Submitting solution...".to_string());

        let handle = tokio::spawn(async move {
            match fs::read(&filepath) {
                Ok(file_content) => {
                    service::submit_solution(
                        &leaderboard,
                        &gpu,
                        &submission_mode,
                        &filepath,
                        &file_content,
                    )
                    .await
                }
                Err(e) => Err(anyhow!("Failed to read file {}: {}", filepath, e)),
            }
        });
        self.submission_task = Some(handle);
        Ok(())
    }

    pub async fn check_leaderboard_task(&mut self) {
        let mut result_to_process: Option<_> = None;
        if let Some(handle) = self.leaderboards_task.as_mut() {
            if handle.is_finished() {
                // Task is finished, take it and await the result.
                if let Some(h) = self.leaderboards_task.take() {
                    result_to_process = Some(h.await);
                }
            }
        }

        if let Some(join_result) = result_to_process {
            match join_result {
                Ok(Ok(leaderboards)) => {
                    self.leaderboards = leaderboards;
                    if !self.leaderboards.is_empty() {
                        self.leaderboards_state.select(Some(0));
                    } else {
                        self.leaderboards_state.select(None); // Ensure selection is cleared if empty
                    }
                    self.loading_message = None; // Clear loading on success
                }
                Ok(Err(e)) => {
                    self.set_error_and_quit(format!("Error fetching leaderboards: {}", e));
                }
                Err(e) => {
                    // This usually means the task panicked.
                    self.set_error_and_quit(format!("Leaderboard fetch task failed: {}", e));
                }
            }
        }
    }

    pub async fn check_gpu_task(&mut self) {
        let mut result_to_process: Option<_> = None;
        if let Some(handle) = self.gpus_task.as_mut() {
            if handle.is_finished() {
                if let Some(h) = self.gpus_task.take() {
                    result_to_process = Some(h.await);
                }
            }
        }

        if let Some(join_result) = result_to_process {
            match join_result {
                Ok(Ok(gpus)) => {
                    self.gpus = gpus;
                    if self.gpus.is_empty() {
                        self.set_error_and_quit(
                            "No GPUs available for the selected leaderboard.".to_string(),
                        );
                        self.gpus_state.select(None); // Clear selection if empty
                    } else {
                        self.gpus_state.select(Some(0));
                    }
                    self.loading_message = None; // Clear loading on success
                }
                Ok(Err(e)) => {
                    self.set_error_and_quit(format!("Error fetching GPUs: {}", e));
                }
                Err(e) => {
                    self.set_error_and_quit(format!("GPU fetch task failed: {}", e));
                }
            }
        }
    }

    pub async fn check_submission_task(&mut self) {
        let mut result_to_process: Option<_> = None;
        if let Some(handle) = self.submission_task.as_mut() {
            if handle.is_finished() {
                if let Some(h) = self.submission_task.take() {
                    result_to_process = Some(h.await);
                }
            }
        }

        if let Some(join_result) = result_to_process {
            match join_result {
                Ok(Ok(result)) => {
                    self.final_status = Some(result);
                    self.should_quit = true;
                    self.loading_message = None;
                }
                Ok(Err(e)) => {
                    self.set_error_and_quit(format!("Submission failed: {}", e));
                }
                Err(e) => {
                    self.set_error_and_quit(format!("Submission task failed: {}", e));
                }
            }
        }
    }
}

pub fn ui(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .margin(1)
        .constraints([Constraint::Min(0)].as_ref())
        .split(frame.size());

    if let Some(message) = &app.loading_message {
        let text = format!("{} (Press Ctrl+C to quit)", message);
        let paragraph = Paragraph::new(text)
            .block(Block::default().title("Status").borders(Borders::ALL))
            .alignment(Alignment::Center);
        frame.render_widget(paragraph, chunks[0]);
        return;
    }

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
            let paragraph =
                Paragraph::new("").block(Block::default().title("Status").borders(Borders::ALL));
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
        println!(
            "Warning: multiple GPUs specified, only the first one ({}) will be used.",
            popcorn_directives.gpus[0]
        );
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

    // Perform initial data loading by spawning tasks
    match app.modal_state {
        ModelState::LeaderboardSelection => {
            // Spawn the task, handle immediate spawn error
            if let Err(e) = app.spawn_load_leaderboards() {
                // Error during spawning itself (rare)
                app.final_status = Some(format!("Error starting leaderboard fetch: {}", e));
                app.should_quit = true;
            }
        }
        ModelState::GpuSelection => {
            // Spawn the task, handle immediate spawn error
            if let Err(e) = app.spawn_load_gpus() {
                // Error during spawning itself (e.g., no leaderboard selected)
                app.final_status = Some(format!("Error starting GPU fetch: {}", e));
                app.should_quit = true;
            }
        }
        _ => { /* No initial loading needed for other states */ }
    }

    // Main event loop
    while !app.should_quit {
        // Draw UI (shows loading screen if loading_message is Some)
        terminal.draw(|frame| ui(&app, frame))?;

        // Handle events first (to ensure Ctrl+C works during checks below)
        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key_event(key)?;
                    // If event handling caused quit, break early
                    if app.should_quit {
                        break;
                    }
                }
            }
        }

        app.check_leaderboard_task().await;

        app.check_gpu_task().await;

        app.check_submission_task().await;
    }

    // Cleanup terminal
    terminal.clear()?;
    disable_raw_mode()?;
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;

    // Brief pause allows the terminal to restore properly before printing final output
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Clear screen again and move cursor to top-left for final output
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
