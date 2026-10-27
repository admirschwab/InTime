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
    /// Added in v0.2.0; older exports have none.
    #[serde(default)]
    pub heimat_mappings: Vec<HeimatMapping>,
}

// ---------------------------------------------------------------- Heimat

/// Connection to a Heimat instance. Kept separate from `Settings` so the
/// access token never ends up in an export file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HeimatConfig {
    pub url: String,
    pub token: String,
}

impl HeimatConfig {
    pub fn is_configured(&self) -> bool {
        !self.url.trim().is_empty() && !self.token.trim().is_empty()
    }
}

/// A bookable task as returned by `GET /my/tasks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HeimatTask {
    pub id: i64,
    pub name: String,
    pub task_holder_name: String,
    pub task_holder_type: String,
    pub is_favorite: bool,
    pub booking_hint: Option<String>,
    pub is_start_and_end_time_required: bool,
    pub is_note_optional: bool,
}

impl Default for HeimatTask {
    fn default() -> Self {
        HeimatTask {
            id: 0,
            name: String::new(),
            task_holder_name: String::new(),
            task_holder_type: String::new(),
            is_favorite: false,
            booking_hint: None,
            is_start_and_end_time_required: false,
            is_note_optional: true,
        }
    }
}

/// A booked time as used by `GET/POST /my/times`. InTime books durations only,
/// so `start` and `end` stay empty.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HeimatTime {
    pub id: i64,
    pub task_id: i64,
    pub date: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub duration_in_minutes: i64,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeimatMapping {
    pub project_id: i64,
    pub task_id: i64,
    pub task_name: String,
    pub task_holder_name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SyncStatus {
    /// Mapped to an available task – can be transferred.
    Ready,
    /// Work on a project that has no Heimat task assigned.
    Unmapped,
    /// Mapped task is no longer offered by Heimat for this day.
    Unavailable,
    /// Booked in Heimat, but nothing tracked in InTime.
    HeimatOnly,
}

/// One line of the day comparison: everything booked on one Heimat task.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRow {
    pub task_id: Option<i64>,
    pub task_name: String,
    pub task_holder_name: String,
    pub booking_hint: String,
    pub is_note_optional: bool,
    pub status: SyncStatus,
    pub project_ids: Vec<i64>,
    pub intime_seconds: i64,
    pub intime_notes: String,
    pub heimat_seconds: i64,
    pub heimat_notes: String,
    pub existing_time_ids: Vec<i64>,
    /// Pre-selected for transfer: ready and missing or off by at least 15 minutes.
    pub suggest_sync: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncItem {
    pub task_id: i64,
    pub minutes: i64,
    pub note: String,
    /// Times already booked on this task for the day; replaced by the new booking.
    pub existing_time_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub task_id: i64,
    pub error: Option<String>,
}
