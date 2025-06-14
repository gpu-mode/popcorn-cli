use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmissionResultMsg(pub String);
