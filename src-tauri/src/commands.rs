use chrono::NaiveDate;
use tauri::{AppHandle, Emitter, State};

use crate::AppState;
use crate::error::{Error, Result};
use crate::models::{Current, ExportData, Project, ProjectInput, Settings, Work, WorkInput};
use crate::{indicator, repo, windows};

/// Tells all windows to reload their data and updates the tray/taskbar color.
fn notify(app: &AppHandle) {
    let _ = app.emit("data-changed", ());
    indicator::refresh(app);
}

#[tauri::command]
pub fn get_projects(state: State<AppState>, include_disabled: bool) -> Result<Vec<Project>> {
    repo::projects(&state.db.lock().unwrap(), include_disabled)
}

#[tauri::command]
pub fn create_project(app: AppHandle, state: State<AppState>, input: ProjectInput) -> Result<Project> {
    let project = repo::create_project(&state.db.lock().unwrap(), &input)?;
    notify(&app);
    Ok(project)
}

#[tauri::command]
pub fn update_project(app: AppHandle, state: State<AppState>, id: i64, input: ProjectInput) -> Result<Project> {
    let project = repo::update_project(&state.db.lock().unwrap(), id, &input)?;
    notify(&app);
    Ok(project)
}

#[tauri::command]
pub fn delete_project(app: AppHandle, state: State<AppState>, id: i64) -> Result<()> {
    repo::delete_project(&state.db.lock().unwrap(), id)?;
    notify(&app);
    Ok(())
}

#[tauri::command]
pub fn reorder_projects(app: AppHandle, state: State<AppState>, ids: Vec<i64>) -> Result<()> {
    repo::reorder_projects(&state.db.lock().unwrap(), &ids)?;
    notify(&app);
    Ok(())
}

#[tauri::command]
pub fn get_current(state: State<AppState>) -> Result<Current> {
    repo::current(&state.db.lock().unwrap())
}

#[tauri::command]
pub fn switch_project(
    app: AppHandle,
    state: State<AppState>,
    project_id: i64,
    notes: Option<String>,
    transfer_minutes: Option<i64>,
) -> Result<()> {
    repo::switch_project(&state.db.lock().unwrap(), project_id, notes.as_deref(), transfer_minutes.unwrap_or(0))?;
    notify(&app);
    Ok(())
}

#[tauri::command]
pub fn update_current_notes(state: State<AppState>, notes: String) -> Result<()> {
    repo::update_current_notes(&state.db.lock().unwrap(), &notes)
}

fn parse_date(date: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| Error::msg(format!("Ungültiges Datum: {date}")))
}

#[tauri::command]
pub fn get_works_for_day(state: State<AppState>, date: String) -> Result<Vec<Work>> {
    repo::works_for_day(&state.db.lock().unwrap(), parse_date(&date)?)
}

#[tauri::command]
pub fn get_days_with_work(state: State<AppState>, year: i32, month: u32) -> Result<Vec<u32>> {
    repo::days_with_work(&state.db.lock().unwrap(), year, month)
}

#[tauri::command]
pub fn create_work(app: AppHandle, state: State<AppState>, input: WorkInput) -> Result<()> {
    repo::create_work(&state.db.lock().unwrap(), &input)?;
    notify(&app);
    Ok(())
}

#[tauri::command]
pub fn update_work(app: AppHandle, state: State<AppState>, id: i64, input: WorkInput) -> Result<()> {
    repo::update_work(&state.db.lock().unwrap(), id, &input)?;
    notify(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_work(app: AppHandle, state: State<AppState>, id: i64) -> Result<()> {
    repo::delete_work(&state.db.lock().unwrap(), id)?;
    notify(&app);
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings> {
    repo::settings(&state.db.lock().unwrap())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> Result<()> {
    repo::save_settings(&state.db.lock().unwrap(), &settings)?;
    let _ = app.emit("settings-changed", &settings);
    Ok(())
}

#[tauri::command]
pub fn export_data(state: State<AppState>, path: String) -> Result<()> {
    let data = repo::export(&state.db.lock().unwrap())?;
    std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
    Ok(())
}

/// Replaces all data with the file's content and restarts the app.
#[tauri::command]
pub fn import_data(app: AppHandle, state: State<AppState>, path: String) -> Result<()> {
    let data: ExportData = serde_json::from_str(&std::fs::read_to_string(path)?)
        .map_err(|_| Error::msg("Die Datei konnte nicht gelesen werden. Ist es ein InTime-Export?"))?;
    {
        let conn = state.db.lock().unwrap();
        repo::finish_active(&conn, repo::now(), None)?;
        repo::import(&conn, &data)?;
    }
    app.restart();
}

// Window creation must not run on the main thread on Windows, hence async.
#[tauri::command]
pub async fn open_window(app: AppHandle, view: String, id: Option<i64>) -> Result<()> {
    windows::open(&app, &view, id)
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
