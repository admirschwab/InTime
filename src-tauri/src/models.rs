use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub color: String,
    pub is_work: bool,
    pub is_default: bool,
    pub is_enabled: bool,
    pub sort_index: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInput {
    pub name: String,
    pub description: String,
    pub color: String,
    pub is_work: bool,
    pub is_default: bool,
}

/// A tracked time slot. Times are local wall-clock times formatted as `YYYY-MM-DDTHH:MM:SS`.
/// The `active` work is the one currently running; its end time is refreshed by the auto-save.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Work {
    pub id: i64,
    pub project_id: i64,
    pub start_time: String,
    pub end_time: String,
    pub notes: String,
    pub active: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkInput {
    pub project_id: i64,
    pub start_time: String,
    pub end_time: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTime {
    pub project_id: i64,
    pub seconds: i64,
}

/// The running work plus today's finished time per project (excluding the running work).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Current {
    pub work: Work,
    pub today: Vec<ProjectTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub background_color: String,
    pub text_color: String,
    pub hover_background_color: String,
    pub hover_text_color: String,
    pub display_projects_right: bool,
    pub hide_projects_on_mouse_exit: bool,
    pub save_window_position: bool,
    pub remind_notes: bool,
    pub remind_notes_only_work: bool,
    pub confirm_close: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            background_color: "#1f2329".into(),
            text_color: "#e8eaed".into(),
            hover_background_color: "#2d333b".into(),
            hover_text_color: "#ffffff".into(),
            display_projects_right: false,
            hide_projects_on_mouse_exit: false,
            save_window_position: true,
            remind_notes: false,
            remind_notes_only_work: false,
            confirm_close: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportData {
    pub app: String,
    pub version: u32,
    pub exported_at: String,
    pub projects: Vec<Project>,
    pub works: Vec<Work>,
    pub settings: Settings,
}
