mod commands;
mod db;
mod error;
mod indicator;
mod models;
mod repo;
mod windows;

use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};

const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(15);

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    /// Last known physical position of the main window, persisted by the auto-save.
    pub main_position: Mutex<Option<(i32, i32)>>,
}

/// Writes the running work's end time and the window position. Because this runs every
/// few seconds, a crash or hard shutdown loses at most one interval.
fn save_state(app: &AppHandle) {
    let state = app.state::<AppState>();
    let position = *state.main_position.lock().unwrap();
    let conn = state.db.lock().unwrap();
    if let Err(e) = repo::autosave(&conn) {
        log::error!("Auto-Save fehlgeschlagen: {e}");
    }
    if let Some(position) = position {
        let _ = repo::save_window_position(&conn, position);
    }
}

fn start_autosave(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(AUTOSAVE_INTERVAL);
        save_state(&app);
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("intime.db"))?;
            repo::recover_and_start(&conn)?;
            app.manage(AppState { db: Mutex::new(conn), main_position: Mutex::new(None) });

            let handle = app.handle();
            windows::create_tray(handle)?;
            windows::restore_main_position(handle)?;
            indicator::refresh(handle);
            start_autosave(handle.clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            match event {
                // The frontend decides (confirmation dialog) and then calls `quit_app`.
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.emit_to("main", "request-close", ());
                }
                WindowEvent::Moved(pos) => {
                    *window.state::<AppState>().main_position.lock().unwrap() = Some((pos.x, pos.y));
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_projects,
            commands::create_project,
            commands::update_project,
            commands::delete_project,
            commands::reorder_projects,
            commands::get_current,
            commands::switch_project,
            commands::update_current_notes,
            commands::get_works_for_day,
            commands::get_days_with_work,
            commands::create_work,
            commands::update_work,
            commands::delete_work,
            commands::get_settings,
            commands::save_settings,
            commands::export_data,
            commands::import_data,
            commands::open_window,
            commands::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                if app.try_state::<AppState>().is_none() {
                    return;
                }
                save_state(app);
                let state = app.state::<AppState>();
                let conn = state.db.lock().unwrap();
                if let Err(e) = repo::finish_active(&conn, repo::now(), None) {
                    log::error!("Beenden: laufender Eintrag konnte nicht gespeichert werden: {e}");
                }
            }
        });
}
