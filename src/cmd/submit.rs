use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use anyhow::{anyhow, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen};
use ratatui::prelude::*;
use ratatui::widgets::ListState;
use tokio::task::JoinHandle;

use crate::models::{
    AppState, GpuItem, GpuSelectionView, LeaderboardItem, LeaderboardSelectionView,
    SelectionAction, SelectionItem, SelectionView, SubmissionModeItem, SubmissionModeSelectionView,
};
use crate::service;
use crate::utils;
use crate::views::file_selection_page::{FileSelectionAction, FileSelectionView};
use crate::views::loading_page::{LoadingPage, LoadingPageState};
use crate::views::result_page::{ResultAction, ResultPageState, ResultView};
use crate::views::welcome_page::{WelcomeAction, WelcomeView};

#[derive(Default)]
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
    pub should_quit: bool,
    pub submission_task: Option<JoinHandle<Result<String, anyhow::Error>>>,
    pub leaderboards_task: Option<JoinHandle<Result<Vec<LeaderboardItem>, anyhow::Error>>>,
    pub gpus_task: Option<JoinHandle<Result<Vec<GpuItem>, anyhow::Error>>>,

    pub loading_page_state: LoadingPageState,
    pub result_page_state: ResultPageState,

    // View instances
    pub welcome_view: Option<WelcomeView>,
    pub file_selection_view: Option<FileSelectionView>,
    pub leaderboard_view: Option<LeaderboardSelectionView>,
    pub gpu_view: Option<GpuSelectionView>,
    pub submission_mode_view: Option<SubmissionModeSelectionView>,
    pub result_view: Option<ResultView>,
}

impl App {
    pub fn new(filepath: Option<String>, cli_id: String) -> Self {
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
            filepath: filepath.unwrap_or_default(),
            cli_id,
            submission_modes,
            selected_submission_mode: None,
            welcome_view: Some(WelcomeView::new()),
            file_selection_view: None,
            leaderboard_view: None,
            gpu_view: None,
            submission_mode_view: None,
            result_view: None,
            ..Default::default()
        };

        // Set initial state based on whether filepath is provided
        if app.filepath.is_empty() {
            app.app_state = AppState::Welcome;
        } else {
            app.app_state = AppState::LeaderboardSelection;
        }

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
        let has_leaderboard = !popcorn_directives.leaderboard_name.is_empty();
        let has_gpu = !popcorn_directives.gpus.is_empty();

        // Set the selected values from directives
        if has_leaderboard {
            self.selected_leaderboard = Some(popcorn_directives.leaderboard_name.clone());
        }
        if has_gpu {
            self.selected_gpu = Some(popcorn_directives.gpus[0].clone());
        }

        // Determine initial state based on what's available
        match (has_leaderboard, has_gpu) {
            (true, true) => {
                // Both leaderboard and GPU specified - go directly to submission mode selection
                self.app_state = AppState::SubmissionModeSelection;
                self.submission_mode_view = Some(SubmissionModeSelectionView::new(
                    self.submission_modes.clone(),
                    popcorn_directives.leaderboard_name,
                    popcorn_directives.gpus[0].clone(),
                ));
            }
            (true, false) => {
                // Only leaderboard specified - need to select GPU
                self.app_state = AppState::GpuSelection;
            }
            (false, true) => {
                // Only GPU specified - need to select leaderboard
                self.app_state = AppState::LeaderboardSelection;
            }
            (false, false) => {
                // Neither specified - start from leaderboard selection
                self.app_state = AppState::LeaderboardSelection;
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<bool> {
        // Global key handling (esc, ctrl+c)
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return Ok(true);
        }

        if key.code == KeyCode::Esc {
            self.should_quit = true;
            return Ok(true);
        }

        // Delegate to views based on current state
        match self.app_state {
            AppState::Welcome => {
                if let Some(view) = &mut self.welcome_view {
                    match view.handle_key_event(key) {
                        WelcomeAction::Submit => {
                            self.app_state = AppState::FileSelection;
                            self.file_selection_view = Some(FileSelectionView::new()?);
                            return Ok(true);
                        }
                        WelcomeAction::ViewHistory => {
                            self.show_error(
                                "View History feature is not yet implemented".to_string(),
                            );
                            return Ok(true);
                        }
                        WelcomeAction::Handled => return Ok(true),
                        WelcomeAction::NotHandled => return Ok(false),
                    }
                }
            }
            AppState::FileSelection => {
                if let Some(view) = &mut self.file_selection_view {
                    match view.handle_key_event(key)? {
                        FileSelectionAction::FileSelected(filepath) => {
                            self.filepath = filepath.clone();

                            // Parse directives from the selected file
                            match utils::get_popcorn_directives(&filepath) {
                                Ok((directives, has_multiple_gpus)) => {
                                    if has_multiple_gpus {
                                        self.show_error("Multiple GPUs are not supported yet. Please specify only one GPU.".to_string());
                                        return Ok(true);
                                    }

                                    // Apply directives to determine next state
                                    self.initialize_with_directives(directives);

                                    // Spawn appropriate task based on the new state
                                    // TODO: make spawn_x tasks also show error if they fail, a lot of duplicate code
                                    match self.app_state {
                                        AppState::LeaderboardSelection => {
                                            if let Err(e) = self.spawn_load_leaderboards() {
                                                self.show_error(format!(
                                                    "Error starting leaderboard fetch: {}",
                                                    e
                                                ));
                                            }
                                        }
                                        AppState::GpuSelection => {
                                            if let Err(e) = self.spawn_load_gpus() {
                                                self.show_error(format!(
                                                    "Error starting GPU fetch: {}",
                                                    e
                                                ));
                                            }
                                        }
                                        AppState::SubmissionModeSelection => {
                                            // View already created in initialize_with_directives
                                        }
                                        _ => {}
                                    }
                                }
                                Err(e) => {
                                    self.show_error(format!(
                                        "Error parsing file directives: {}",
                                        e
                                    ));
                                    return Ok(true);
                                }
                            }
                            return Ok(true);
                        }
                        FileSelectionAction::Handled => return Ok(true),
                        FileSelectionAction::NotHandled => return Ok(false),
                    }
                }
            }
            AppState::LeaderboardSelection => {
                if let Some(view) = &mut self.leaderboard_view {
                    match view.handle_key_event(key) {
                        SelectionAction::Selected(idx) => {
                            self.selected_leaderboard = Some(view.items()[idx].title().to_string());

                            if self.selected_gpu.is_none() {
                                self.app_state = AppState::GpuSelection;
                                if let Err(e) = self.spawn_load_gpus() {
                                    self.show_error(format!("Error starting GPU fetch: {}", e));
                                }
                            } else {
                                self.app_state = AppState::SubmissionModeSelection;
                                self.submission_mode_view = Some(SubmissionModeSelectionView::new(
                                    self.submission_modes.clone(),
                                    self.selected_leaderboard.as_ref().unwrap().clone(),
                                    self.selected_gpu.as_ref().unwrap().clone(),
                                ));
                            }
                            return Ok(true);
                        }
                        SelectionAction::Handled => return Ok(true),
                        SelectionAction::NotHandled => return Ok(false),
                    }
                }
            }
            AppState::GpuSelection => {
                if let Some(view) = &mut self.gpu_view {
                    match view.handle_key_event(key) {
                        SelectionAction::Selected(idx) => {
                            self.selected_gpu = Some(view.items()[idx].title().to_string());
                            self.app_state = AppState::SubmissionModeSelection;
                            self.submission_mode_view = Some(SubmissionModeSelectionView::new(
                                self.submission_modes.clone(),
                                self.selected_leaderboard.as_ref().unwrap().clone(),
                                self.selected_gpu.as_ref().unwrap().clone(),
                            ));
                            return Ok(true);
                        }
                        SelectionAction::Handled => return Ok(true),
                        SelectionAction::NotHandled => return Ok(false),
                    }
                }
            }
            AppState::SubmissionModeSelection => {
                if let Some(view) = &mut self.submission_mode_view {
                    match view.handle_key_event(key) {
                        SelectionAction::Selected(idx) => {
                            self.selected_submission_mode = Some(view.items()[idx].value.clone());
                            self.app_state = AppState::WaitingForResult;
                            if let Err(e) = self.spawn_submit_solution() {
                                self.show_error(format!("Error starting submission: {}", e));
                            }
                            return Ok(true);
                        }
                        SelectionAction::Handled => return Ok(true),
                        SelectionAction::NotHandled => return Ok(false),
                    }
                }
            }
            AppState::ShowingResult => {
                if let Some(view) = &mut self.result_view {
                    match view.handle_key_event(key) {
                        ResultAction::Quit => {
                            self.should_quit = true;
                            return Ok(true);
                        }
                        ResultAction::Handled => {
                            // Update scroll state based on key
                            view.update_scroll(key, &mut self.result_page_state);
                            return Ok(true);
                        }
                        ResultAction::NotHandled => return Ok(false),
                    }
                }
            }
            _ => {}
        }

        Ok(false)
    }

    fn show_error(&mut self, error_message: String) {
        self.result_view = Some(ResultView::new(error_message));
        self.app_state = AppState::ShowingResult;
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
                        self.leaderboards = leaderboards.clone();
                        self.leaderboard_view = Some(LeaderboardSelectionView::new(leaderboards));

                        if let Some(selected_name) = &self.selected_leaderboard {
                            if let Some(index) = self
                                .leaderboards
                                .iter()
                                .position(|lb| lb.title() == selected_name)
                            {
                                if let Some(view) = &mut self.leaderboard_view {
                                    view.state_mut().select(Some(index));
                                }
                                if self.selected_gpu.is_some() {
                                    self.app_state = AppState::SubmissionModeSelection;
                                    self.submission_mode_view =
                                        Some(SubmissionModeSelectionView::new(
                                            self.submission_modes.clone(),
                                            self.selected_leaderboard.as_ref().unwrap().clone(),
                                            self.selected_gpu.as_ref().unwrap().clone(),
                                        ));
                                } else {
                                    self.app_state = AppState::GpuSelection;
                                    if let Err(e) = self.spawn_load_gpus() {
                                        self.show_error(format!("Error starting GPU fetch: {}", e));
                                        return;
                                    }
                                }
                            } else {
                                self.selected_leaderboard = None;
                                if let Some(view) = &mut self.leaderboard_view {
                                    view.state_mut().select(Some(0));
                                }
                                self.app_state = AppState::LeaderboardSelection;
                            }
                        } else if let Some(view) = &mut self.leaderboard_view {
                            view.state_mut().select(Some(0));
                        }
                    }
                    Ok(Err(e)) => self.show_error(format!("Error fetching leaderboards: {}", e)),
                    Err(e) => self.show_error(format!("Task join error: {}", e)),
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
                        self.gpus = gpus.clone();
                        self.gpu_view = Some(GpuSelectionView::new(
                            gpus,
                            self.selected_leaderboard
                                .as_ref()
                                .unwrap_or(&"N/A".to_string())
                                .clone(),
                        ));

                        if let Some(selected_name) = &self.selected_gpu {
                            if let Some(index) = self
                                .gpus
                                .iter()
                                .position(|gpu| gpu.title() == selected_name)
                            {
                                if let Some(view) = &mut self.gpu_view {
                                    view.state_mut().select(Some(index));
                                }
                                self.app_state = AppState::SubmissionModeSelection;
                                self.submission_mode_view = Some(SubmissionModeSelectionView::new(
                                    self.submission_modes.clone(),
                                    self.selected_leaderboard.as_ref().unwrap().clone(),
                                    self.selected_gpu.as_ref().unwrap().clone(),
                                ));
                            } else {
                                self.selected_gpu = None;
                                if let Some(view) = &mut self.gpu_view {
                                    view.state_mut().select(Some(0));
                                }
                                self.app_state = AppState::GpuSelection;
                            }
                        } else if let Some(view) = &mut self.gpu_view {
                            view.state_mut().select(Some(0));
                        }
                    }
                    Ok(Err(e)) => self.show_error(format!("Error fetching GPUs: {}", e)),
                    Err(e) => self.show_error(format!("Task join error: {}", e)),
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
                        // Process the status text
                        let trimmed = status.trim();
                        let content = if trimmed.starts_with('[')
                            && trimmed.ends_with(']')
                            && trimmed.len() >= 2
                        {
                            &trimmed[1..trimmed.len() - 1]
                        } else {
                            trimmed
                        };
                        let content = content.replace("\\n", "\n");

                        // Create result view and transition to showing result
                        self.result_view = Some(ResultView::new(content));
                        self.app_state = AppState::ShowingResult;
                    }
                    Ok(Err(e)) => {
                        // Show error in result view
                        self.result_view =
                            Some(ResultView::new(format!("Submission error: {}", e)));
                        self.app_state = AppState::ShowingResult;
                    }
                    Err(e) => {
                        // Show task join error in result view
                        self.result_view = Some(ResultView::new(format!("Task join error: {}", e)));
                        self.app_state = AppState::ShowingResult;
                    }
                }
            }
        }
    }
}

pub fn ui(app: &mut App, frame: &mut Frame) {
    match app.app_state {
        AppState::Welcome => {
            if let Some(view) = &mut app.welcome_view {
                view.render(frame);
            }
        }
        AppState::FileSelection => {
            if let Some(view) = &mut app.file_selection_view {
                view.render(frame);
            }
        }
        AppState::LeaderboardSelection => {
            if let Some(view) = &mut app.leaderboard_view {
                view.render(frame);
            }
        }
        AppState::GpuSelection => {
            if let Some(view) = &mut app.gpu_view {
                view.render(frame);
            }
        }
        AppState::SubmissionModeSelection => {
            if let Some(view) = &mut app.submission_mode_view {
                view.render(frame);
            }
        }
        AppState::WaitingForResult => {
            let loading_page = LoadingPage::default();
            frame.render_stateful_widget(
                &loading_page,
                frame.size(),
                &mut app.loading_page_state.clone(),
            )
        }
        AppState::ShowingResult => {
            if let Some(view) = &mut app.result_view {
                view.render(frame, &mut app.result_page_state);
            }
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
        Some(fp) => {
            if !Path::new(&fp).exists() {
                return Err(anyhow!("File not found: {}", fp));
            }
            Some(fp)
        }
        None => None,
    };

    let mut app = App::new(file_to_submit.clone(), cli_id);

    // If we have a filepath, process directives and setup initial state
    if let Some(ref file_path) = file_to_submit {
        let (directives, has_multiple_gpus) = utils::get_popcorn_directives(file_path)?;

        if has_multiple_gpus {
            return Err(anyhow!(
                "Multiple GPUs are not supported yet. Please specify only one GPU."
            ));
        }

        // First apply directives as defaults
        app.initialize_with_directives(directives);

        // Then override with CLI flags if provided
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

        // Spawn the initial task based on the starting state BEFORE setting up the TUI
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
    }

    // Now, set up the TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    while !app.should_quit {
        terminal.draw(|f| ui(&mut app, f))?;

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

    // Restore terminal
    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    Ok(())
}
