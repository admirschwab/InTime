use chrono::NaiveDate;
use tauri::{AppHandle, Emitter, State};

use crate::AppState;
use crate::error::{Error, Result};
use crate::heimat::{self, HeimatClient};
use crate::models::{
    Current, ExportData, HeimatConfig, HeimatMapping, HeimatTask, HeimatTime, Project, ProjectInput, Settings,
    SyncItem, SyncResult, SyncRow, Work, WorkInput,
};
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

// ---------------------------------------------------------------- Heimat

/// Colors for projects imported from Heimat, used in turn.
const IMPORT_COLORS: &[&str] = &[
    "#4c8bf5", "#57ab5a", "#f0883e", "#8f6ff5", "#2ea0a0", "#d062b8", "#d4a72c", "#e5534b",
];

fn heimat_client(state: &State<'_, AppState>) -> Result<HeimatClient> {
    HeimatClient::new(&repo::heimat_config(&state.db.lock().unwrap())?)
}

#[tauri::command]
pub fn heimat_get_config(state: State<AppState>) -> Result<HeimatConfig> {
    repo::heimat_config(&state.db.lock().unwrap())
}

#[tauri::command]
pub fn heimat_save_config(app: AppHandle, state: State<AppState>, config: HeimatConfig) -> Result<()> {
    repo::save_heimat_config(&state.db.lock().unwrap(), &config)?;
    let _ = app.emit("heimat-changed", ());
    Ok(())
}

/// Checks URL and token without saving them; returns the number of bookable tasks.
#[tauri::command]
pub async fn heimat_test(config: HeimatConfig) -> Result<usize> {
    let tasks = HeimatClient::new(&config)?.tasks(None).await?;
    Ok(heimat::bookable_tasks(tasks).len())
}

#[tauri::command]
pub async fn heimat_tasks(state: State<'_, AppState>) -> Result<Vec<HeimatTask>> {
    let client = heimat_client(&state)?;
    Ok(heimat::bookable_tasks(client.tasks(None).await?))
}

#[tauri::command]
pub fn heimat_get_mappings(state: State<AppState>) -> Result<Vec<HeimatMapping>> {
    repo::heimat_mappings(&state.db.lock().unwrap())
}

#[tauri::command]
pub fn heimat_save_mappings(app: AppHandle, state: State<AppState>, mappings: Vec<HeimatMapping>) -> Result<()> {
    repo::replace_heimat_mappings(&state.db.lock().unwrap(), &mappings)?;
    notify(&app);
    Ok(())
}

/// Creates one work project per Heimat task, already mapped to that task.
#[tauri::command]
pub fn heimat_import_tasks(app: AppHandle, state: State<AppState>, tasks: Vec<HeimatTask>) -> Result<()> {
    {
        let conn = state.db.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        for (i, task) in tasks.iter().enumerate() {
            let description = match task.booking_hint.as_deref().map(str::trim) {
                Some(hint) if !hint.is_empty() => format!("{}\n{hint}", task.task_holder_name),
                _ => task.task_holder_name.clone(),
            };
            let input = ProjectInput {
                name: task.name.clone(),
                description,
                color: IMPORT_COLORS[i % IMPORT_COLORS.len()].into(),
                is_work: true,
                is_default: false,
            };
            let project = repo::create_project(&tx, &input)?;
            repo::insert_heimat_mappings(
                &tx,
                &[HeimatMapping {
                    project_id: project.id,
                    task_id: task.id,
                    task_name: task.name.clone(),
                    task_holder_name: task.task_holder_name.clone(),
                }],
            )?;
        }
        tx.commit()?;
    }
    notify(&app);
    Ok(())
}

/// Compares the day's InTime work with what is already booked in Heimat.
#[tauri::command]
pub async fn heimat_day(state: State<'_, AppState>, date: String) -> Result<Vec<SyncRow>> {
    let date = parse_date(&date)?;
    let (client, works, projects, mappings) = {
        let conn = state.db.lock().unwrap();
        (
            HeimatClient::new(&repo::heimat_config(&conn)?)?,
            repo::works_for_day(&conn, date)?,
            repo::projects(&conn, true)?,
            repo::heimat_mappings(&conn)?,
        )
    };
    let tasks = heimat::bookable_tasks(client.tasks(Some(date)).await?);
    let times = client.times(Some(date)).await?;
    heimat::day_overview(&works, &projects, &mappings, &tasks, &times, repo::now())
}

/// Books the given durations in Heimat. Times already booked on the same task for the
/// day are replaced. Each item succeeds or fails on its own.
#[tauri::command]
pub async fn heimat_sync(state: State<'_, AppState>, date: String, items: Vec<SyncItem>) -> Result<Vec<SyncResult>> {
    let date = parse_date(&date)?;
    let client = heimat_client(&state)?;
    let mut results = Vec::new();
    for item in items {
        let outcome = async {
            if item.minutes <= 0 || item.minutes % 15 != 0 {
                return Err(Error::msg("Heimat erlaubt nur Zeiten in 15-Minuten-Schritten (mindestens 0:15)."));
            }
            for id in &item.existing_time_ids {
                client.delete_time(*id).await?;
            }
            let time = HeimatTime {
                id: 0,
                task_id: item.task_id,
                date: date.format("%Y-%m-%d").to_string(),
                start: None,
                end: None,
                duration_in_minutes: item.minutes,
                note: Some(item.note.trim().to_string()),
            };
            client.add_time(&time).await
        }
        .await;
        if let Err(e) = &outcome {
            log::error!("Heimat-Übertragung für Aufgabe {} fehlgeschlagen: {e}", item.task_id);
        }
        results.push(SyncResult { task_id: item.task_id, error: outcome.err().map(|e| e.to_string()) });
    }
    Ok(results)
}
