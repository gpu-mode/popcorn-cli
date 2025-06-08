use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use anyhow::{anyhow, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen};
use ratatui::prelude::*;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use tokio::task::JoinHandle;

use crate::models::{AppState, GpuItem, LeaderboardItem, SubmissionModeItem};
use crate::service;
use crate::utils;
use crate::views::loading_page::{LoadingPage, LoadingPageState};
use crate::views::result_page::ResultPage;

#[derive(Default, Debug)]
pub struct App {
    pub filepath: String,
    pub cli_id: String,

    pub leaderboards: Vec<LeaderboardItem>,
    pub leaderboards_state: ListState,
    pub selected_leaderboard: Option<String>,

    pub gpus: Vec<GpuItem>,
    pub gpus_state: ListState,
    pub selected_gpu: Option<String>,

    pub submission_modes: Vec<SubmissionModeItem>,
    pub submission_modes_state: ListState,
    pub selected_submission_mode: Option<String>,

    pub app_state: AppState,
    pub final_status: Option<String>,

    pub should_quit: bool,
    pub submission_task: Option<JoinHandle<Result<String, anyhow::Error>>>,
    pub leaderboards_task: Option<JoinHandle<Result<Vec<LeaderboardItem>, anyhow::Error>>>,
    pub gpus_task: Option<JoinHandle<Result<Vec<GpuItem>, anyhow::Error>>>,

    pub loading_page_state: LoadingPageState,
}

impl App {
    pub fn new<P: AsRef<Path>>(filepath: P, cli_id: String) -> Self {
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
                "Profile".to_string(),
                "Work in progress...".to_string(),
                "profile".to_string(),
            ),
        ];

        let mut app = Self {
            filepath: filepath.as_ref().to_string_lossy().to_string(),
            cli_id,
            submission_modes,
            selected_submission_mode: None,
            ..Default::default()
        };

        app.leaderboards_state.select(Some(0));
        app.gpus_state.select(Some(0));
        app.submission_modes_state.select(Some(0));
        app
    }

    pub fn update_loading_page_state(&mut self, terminal_width: u16) {
        if self.app_state != AppState::WaitingForResult {
            return;
        }

        let st = &mut self.loading_page_state;
        st.progress_column = {
            if st.progress_column < terminal_width {
                st.progress_column + 1
            } else {
                st.loop_count += 1;
                0
            }
        };
        st.progress_bar = f64::from(st.progress_column) * 100.0 / f64::from(terminal_width);
    }

    pub fn initialize_with_directives(&mut self, popcorn_directives: utils::PopcornDirectives) {
        if !popcorn_directives.leaderboard_name.is_empty() {
            self.selected_leaderboard = Some(popcorn_directives.leaderboard_name);

            if !popcorn_directives.gpus.is_empty() {
                self.selected_gpu = Some(popcorn_directives.gpus[0].clone());
                self.app_state = AppState::SubmissionModeSelection;
            } else {
                self.app_state = AppState::GpuSelection;
            }
        } else if !popcorn_directives.gpus.is_empty() {
            self.selected_gpu = Some(popcorn_directives.gpus[0].clone());
            if !popcorn_directives.leaderboard_name.is_empty() {
                self.selected_leaderboard = Some(popcorn_directives.leaderboard_name);
                self.app_state = AppState::SubmissionModeSelection;
            } else {
                self.app_state = AppState::LeaderboardSelection;
            }
        } else {
            self.app_state = AppState::LeaderboardSelection;
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        // Allow quitting anytime, even while loading
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return Ok(true);
        }

        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                return Ok(true);
            }
            KeyCode::Enter => match self.app_state {
                AppState::LeaderboardSelection => {
                    if let Some(idx) = self.leaderboards_state.selected() {
                        if idx < self.leaderboards.len() {
                            self.selected_leaderboard =
                                Some(self.leaderboards[idx].title_text.clone());

                            if self.selected_gpu.is_none() {
                                self.app_state = AppState::GpuSelection;
                                if let Err(e) = self.spawn_load_gpus() {
                                    self.set_error_and_quit(format!(
                                        "Error starting GPU fetch: {}",
                                        e
                                    ));
                                }
                            } else {
                                self.app_state = AppState::SubmissionModeSelection;
                            }
                            return Ok(true);
                        }
                    }
                }
                AppState::GpuSelection => {
                    if let Some(idx) = self.gpus_state.selected() {
                        if idx < self.gpus.len() {
                            self.selected_gpu = Some(self.gpus[idx].title_text.clone());
                            self.app_state = AppState::SubmissionModeSelection;
                            return Ok(true);
                        }
                    }
                }
                AppState::SubmissionModeSelection => {
                    if let Some(idx) = self.submission_modes_state.selected() {
                        if idx < self.submission_modes.len() {
                            self.selected_submission_mode =
                                Some(self.submission_modes[idx].value.clone());
                            self.app_state = AppState::WaitingForResult;
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

    fn set_error_and_quit(&mut self, error_message: String) {
        self.final_status = Some(error_message);
        self.should_quit = true;
    }

    fn move_selection_up(&mut self) {
        match self.app_state {
            AppState::LeaderboardSelection => {
                if let Some(idx) = self.leaderboards_state.selected() {
                    if idx > 0 {
                        self.leaderboards_state.select(Some(idx - 1));
                    }
                }
            }
            AppState::GpuSelection => {
                if let Some(idx) = self.gpus_state.selected() {
                    if idx > 0 {
                        self.gpus_state.select(Some(idx - 1));
                    }
                }
            }
            AppState::SubmissionModeSelection => {
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
        match self.app_state {
            AppState::LeaderboardSelection => {
                if let Some(idx) = self.leaderboards_state.selected() {
                    if idx < self.leaderboards.len().saturating_sub(1) {
                        self.leaderboards_state.select(Some(idx + 1));
                    }
                }
            }
            AppState::GpuSelection => {
                if let Some(idx) = self.gpus_state.selected() {
                    if idx < self.gpus.len().saturating_sub(1) {
                        self.gpus_state.select(Some(idx + 1));
                    }
                }
            }
            AppState::SubmissionModeSelection => {
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
        let client = service::create_client(Some(self.cli_id.clone()))?;
        self.leaderboards_task = Some(tokio::spawn(async move {
            service::fetch_leaderboards(&client).await
        }));
        Ok(())
    }

    pub fn spawn_load_gpus(&mut self) -> Result<()> {
        let client = service::create_client(Some(self.cli_id.clone()))?;
        let leaderboard_name = self
            .selected_leaderboard
            .clone()
            .ok_or_else(|| anyhow!("Leaderboard not selected"))?;
        self.gpus_task = Some(tokio::spawn(async move {
            service::fetch_gpus(&client, &leaderboard_name).await
        }));
        Ok(())
    }

    pub fn spawn_submit_solution(&mut self) -> Result<()> {
        let client = service::create_client(Some(self.cli_id.clone()))?;
        let filepath = self.filepath.clone();
        let leaderboard = self
            .selected_leaderboard
            .clone()
            .ok_or_else(|| anyhow!("Leaderboard not selected"))?;
        let gpu = self
            .selected_gpu
            .clone()
            .ok_or_else(|| anyhow!("GPU not selected"))?;
        let mode = self
            .selected_submission_mode
            .clone()
            .ok_or_else(|| anyhow!("Submission mode not selected"))?;

        // Read file content
        let mut file = File::open(&filepath)?;
        let mut file_content = String::new();
        file.read_to_string(&mut file_content)?;

        self.submission_task = Some(tokio::spawn(async move {
            service::submit_solution(&client, &filepath, &file_content, &leaderboard, &gpu, &mode)
                .await
        }));
        Ok(())
    }

    pub async fn check_leaderboard_task(&mut self) {
        if let Some(handle) = &mut self.leaderboards_task {
            if handle.is_finished() {
                let task = self.leaderboards_task.take().unwrap();
                match task.await {
                    Ok(Ok(leaderboards)) => {
                        self.leaderboards = leaderboards;
                        if let Some(selected_name) = &self.selected_leaderboard {
                            if let Some(index) = self
                                .leaderboards
                                .iter()
                                .position(|lb| &lb.title_text == selected_name)
                            {
                                self.leaderboards_state.select(Some(index));
                                if self.selected_gpu.is_some() {
                                    self.app_state = AppState::SubmissionModeSelection;
                                } else {
                                    self.app_state = AppState::GpuSelection;
                                    if let Err(e) = self.spawn_load_gpus() {
                                        self.set_error_and_quit(format!(
                                            "Error starting GPU fetch: {}",
                                            e
                                        ));
                                        return;
                                    }
                                }
                            } else {
                                self.selected_leaderboard = None;
                                self.leaderboards_state.select(Some(0));
                                self.app_state = AppState::LeaderboardSelection;
                            }
                        } else {
                            self.leaderboards_state.select(Some(0));
                        }
                    }
                    Ok(Err(e)) => {
                        self.set_error_and_quit(format!("Error fetching leaderboards: {}", e))
                    }
                    Err(e) => self.set_error_and_quit(format!("Task join error: {}", e)),
                }
            }
        }
    }

    pub async fn check_gpu_task(&mut self) {
        if let Some(handle) = &mut self.gpus_task {
            if handle.is_finished() {
                let task = self.gpus_task.take().unwrap();
                match task.await {
                    Ok(Ok(gpus)) => {
                        self.gpus = gpus;
                        if let Some(selected_name) = &self.selected_gpu {
                            if let Some(index) = self
                                .gpus
                                .iter()
                                .position(|gpu| &gpu.title_text == selected_name)
                            {
                                self.gpus_state.select(Some(index));
                                self.app_state = AppState::SubmissionModeSelection;
                            } else {
                                self.selected_gpu = None;
                                self.gpus_state.select(Some(0));
                                self.app_state = AppState::GpuSelection;
                            }
                        } else {
                            self.gpus_state.select(Some(0));
                        }
                    }
                    Ok(Err(e)) => self.set_error_and_quit(format!("Error fetching GPUs: {}", e)),
                    Err(e) => self.set_error_and_quit(format!("Task join error: {}", e)),
                }
            }
        }
    }

    pub async fn check_submission_task(&mut self) {
        if let Some(handle) = &mut self.submission_task {
            if handle.is_finished() {
                let task = self.submission_task.take().unwrap();
                match task.await {
                    Ok(Ok(status)) => {
                        self.final_status = Some(status);
                        self.should_quit = true; // Quit after showing final status
                    }
                    Ok(Err(e)) => self.set_error_and_quit(format!("Submission error: {}", e)),
                    Err(e) => self.set_error_and_quit(format!("Task join error: {}", e)),
                }
            }
        }
    }
}

pub fn ui(app: &App, frame: &mut Frame) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0)].as_ref())
        .split(frame.size());

    let list_area = main_layout[0];
    let available_width = list_area.width.saturating_sub(4) as usize;

    let list_block = Block::default().borders(Borders::ALL);
    let list_style = Style::default().fg(Color::White);

    match app.app_state {
        AppState::LeaderboardSelection => {
            let items: Vec<ListItem> = app
                .leaderboards
                .iter()
                .map(|lb| {
                    let title_line = Line::from(Span::styled(
                        lb.title_text.clone(),
                        Style::default().fg(Color::White).bold(),
                    ));
                    let mut lines = vec![title_line];
                    for desc_part in lb.task_description.split('\n') {
                        lines.push(Line::from(Span::styled(
                            desc_part.to_string(),
                            Style::default().fg(Color::Gray).dim(),
                        )));
                    }
                    ListItem::new(lines)
                })
                .collect();
            let list = List::new(items)
                .block(list_block.title("Select Leaderboard"))
                .style(list_style)
                .highlight_style(Style::default().bg(Color::DarkGray))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, main_layout[0], &mut app.leaderboards_state.clone());
        }
        AppState::GpuSelection => {
            let items: Vec<ListItem> = app
                .gpus
                .iter()
                .map(|gpu| {
                    let line = Line::from(vec![Span::styled(
                        gpu.title_text.clone(),
                        Style::default().fg(Color::White).bold(),
                    )]);
                    ListItem::new(line)
                })
                .collect();
            let list = List::new(items)
                .block(list_block.title(format!(
                    "Select GPU for '{}'",
                    app.selected_leaderboard.as_deref().unwrap_or("N/A")
                )))
                .style(list_style)
                .highlight_style(Style::default().bg(Color::DarkGray))
                .highlight_symbol("> ");
            frame.render_stateful_widget(list, main_layout[0], &mut app.gpus_state.clone());
        }
        AppState::SubmissionModeSelection => {
            let items: Vec<ListItem> = app
                .submission_modes
                .iter()
                .map(|mode| {
                    let strings = utils::custom_wrap(
                        mode.title_text.clone(),
                        mode.description_text.clone(),
                        available_width,
                    );

                    let lines: Vec<Line> = strings
                        .into_iter()
                        .enumerate()
                        .map(|(i, line)| {
                            if i == 0 {
                                Line::from(Span::styled(
                                    line,
                                    Style::default().fg(Color::White).bold(),
                                ))
                            } else {
                                Line::from(Span::styled(
                                    line.clone(),
                                    Style::default().fg(Color::Gray).dim(),
                                ))
                            }
                        })
                        .collect::<Vec<Line>>();
                    ListItem::new(lines)
                })
                .collect::<Vec<ListItem>>();
            let list = List::new(items)
                .block(list_block.title(format!(
                    "Select Submission Mode for '{}' on '{}'",
                    app.selected_leaderboard.as_deref().unwrap_or("N/A"),
                    app.selected_gpu.as_deref().unwrap_or("N/A")
                )))
                .style(list_style)
                .highlight_style(Style::default().bg(Color::DarkGray))
                .highlight_symbol("> ");
            frame.render_stateful_widget(
                list,
                main_layout[0],
                &mut app.submission_modes_state.clone(),
            );
        }
        AppState::WaitingForResult => {
            let loading_page = LoadingPage::default();
            frame.render_stateful_widget(
                &loading_page,
                main_layout[0],
                &mut app.loading_page_state.clone(),
            )
        }
    }
}

pub async fn run_submit_tui(
    filepath: Option<String>,
    gpu: Option<String>,
    leaderboard: Option<String>,
    mode: Option<String>,
    cli_id: String,
) -> Result<()> {
    let file_to_submit = match filepath {
        Some(fp) => fp,
        None => {
            // Prompt user for filepath if not provided
            println!("Please enter the path to your solution file:");
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            input.trim().to_string()
        }
    };

    if !Path::new(&file_to_submit).exists() {
        return Err(anyhow!("File not found: {}", file_to_submit));
    }

    let (directives, has_multiple_gpus) = utils::get_popcorn_directives(&file_to_submit)?;

    if has_multiple_gpus {
        return Err(anyhow!(
            "Multiple GPUs are not supported yet. Please specify only one GPU."
        ));
    }

    // Perform direct submission if all required parameters are provided via CLI
    if let (Some(gpu_flag), Some(leaderboard_flag), Some(mode_flag)) = (&gpu, &leaderboard, &mode) {
        // Read file content
        let mut file = File::open(&file_to_submit)?;
        let mut file_content = String::new();
        file.read_to_string(&mut file_content)?;

        // Create client and submit directly
        let client = service::create_client(Some(cli_id))?;
        println!("Submitting solution directly with:");
        println!("  File: {}", file_to_submit);
        println!("  Leaderboard: {}", leaderboard_flag);
        println!("  GPU: {}", gpu_flag);
        println!("  Mode: {}", mode_flag);

        // Make the submission
        let result = service::submit_solution(
            &client,
            &file_to_submit,
            &file_content,
            leaderboard_flag,
            gpu_flag,
            mode_flag,
        )
        .await?;

        println!("Submission result: {}", result);

        utils::display_ascii_art();
        return Ok(());
    }

    let mut app = App::new(&file_to_submit, cli_id);

    // Override directives with CLI flags if provided
    if let Some(gpu_flag) = gpu {
        app.selected_gpu = Some(gpu_flag);
    }
    if let Some(leaderboard_flag) = leaderboard {
        app.selected_leaderboard = Some(leaderboard_flag);
    }
    if let Some(mode_flag) = mode {
        app.selected_submission_mode = Some(mode_flag);
        // Skip to submission if we have all required fields
        if app.selected_gpu.is_some() && app.selected_leaderboard.is_some() {
            app.app_state = AppState::WaitingForResult;
        }
    }

    // If no CLI flags, use directives
    if app.selected_gpu.is_none() && app.selected_leaderboard.is_none() {
        app.initialize_with_directives(directives);
    }

    // Spawn the initial task based on the starting state BEFORE setting up the TUI
    // If spawning fails here, we just return the error directly without TUI cleanup.
    match app.app_state {
        AppState::LeaderboardSelection => {
            if let Err(e) = app.spawn_load_leaderboards() {
                return Err(anyhow!("Error starting leaderboard fetch: {}", e));
            }
        }
        AppState::GpuSelection => {
            if let Err(e) = app.spawn_load_gpus() {
                return Err(anyhow!("Error starting GPU fetch: {}", e));
            }
        }
        AppState::WaitingForResult => {
            if let Err(e) = app.spawn_submit_solution() {
                return Err(anyhow!("Error starting submission: {}", e));
            }
        }
        _ => {}
    }

    // Now, set up the TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    while !app.should_quit {
        terminal.draw(|f| ui(&app, f))?;

        app.check_leaderboard_task().await;
        app.check_gpu_task().await;
        app.check_submission_task().await;

        app.update_loading_page_state(terminal.size()?.width);

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key_event(key)?;
                }
            }
        }
    }

    let mut result_text = "Submission cancelled.".to_string();

    if let Some(status) = app.final_status {
        let trimmed = status.trim();
        let content = if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 2 {
            &trimmed[1..trimmed.len() - 1]
        } else {
            trimmed
        };

        // Replace all literal "\n" with actual newlines
        let content = content.replace("\\n", "\n");

        result_text = content.to_string();
    }

    let mut result_page = ResultPage::new(result_text.clone());
    while !result_page.ack {
        terminal
            .draw(|frame: &mut Frame| {
                frame.render_widget(&result_page, frame.size());
            })
            .unwrap();

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    result_page.handle_key_event(key);
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    // utils::display_ascii_art();

    Ok(())
}
