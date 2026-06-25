#[derive(Clone, Debug)]
pub struct LeaderboardItem {
    pub title_text: String,
    pub task_description: String,
}

impl LeaderboardItem {
    pub fn new(title_text: String, task_description: String) -> Self {
        Self {
            title_text,
            task_description,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GpuItem {
    pub title_text: String,
}

impl GpuItem {
    pub fn new(title_text: String) -> Self {
        Self { title_text }
    }
}

#[derive(Clone, Debug)]
pub struct SubmissionModeItem {
    pub title_text: String,
    pub description_text: String,
    pub value: String,
}

impl SubmissionModeItem {
    pub fn new(title_text: String, description_text: String, value: String) -> Self {
        Self {
            title_text,
            description_text,
            value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum AppState {
    #[default]
    LeaderboardSelection,
    GpuSelection,
    SubmissionModeSelection,
    WaitingForResult,
}

/// Summary of a user submission for list view
#[derive(Clone, Debug)]
pub struct UserSubmission {
    pub id: i64,
    pub leaderboard_name: String,
    pub file_name: String,
    pub submission_time: String,
    pub done: bool,
    pub runs: Vec<UserSubmissionRun>,
}

/// A run summary for list view (gpu_type and score only)
#[derive(Clone, Debug)]
pub struct UserSubmissionRun {
    pub gpu_type: String,
    pub score: Option<f64>,
}

/// Full submission details including code and runs
#[derive(Clone, Debug)]
pub struct SubmissionDetails {
    pub id: i64,
    pub leaderboard_id: i64,
    pub leaderboard_name: String,
    pub file_name: String,
    pub user_id: String,
    pub submission_time: String,
    pub done: bool,
    pub code: String,
    pub runs: Vec<SubmissionRun>,
    pub job: Option<SubmissionJobStatus>,
    pub queue: Option<SubmissionQueueStatus>,
}

#[derive(Clone, Debug)]
pub struct SubmissionJobStatus {
    pub status: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SubmissionQueueStatus {
    pub stage: Option<String>,
    pub message: Option<String>,
    pub position: Option<i64>,
    pub jobs_ahead: Option<i64>,
}

/// A single run within a submission
#[derive(Clone, Debug)]
pub struct SubmissionRun {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub mode: String,
    pub secret: bool,
    pub runner: String,
    pub score: Option<f64>,
    pub passed: bool,
}
