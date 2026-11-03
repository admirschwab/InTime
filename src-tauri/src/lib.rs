mod commands;
mod db;
mod error;
mod heimat;
mod indicator;
mod models;
mod paths;
mod repo;
mod windows;

use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_log::{Target, TargetKind};

const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(15);

pub struct AppState {
    db: Mutex<rusqlite::Connection>,
    /// Last known physical position of the main window, persisted by the auto-save.
    main_position: Mutex<Option<(i32, i32)>>,
    /// Whether the project list of the main window is hidden (see `windows::apply_main_region`).
    pub main_layout: Mutex<windows::MainLayout>,
}

/// Locks a mutex even if a panic happened while it was held. A poisoned lock would otherwise
/// make every later command and the auto-save fail until InTime is restarted. The data
/// behind it is a database connection (transactions roll back on drop) or plain values.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl AppState {
    pub fn db(&self) -> MutexGuard<'_, rusqlite::Connection> {
        lock(&self.db)
    }
}

/// Writes the running work's end time and the window position. Because this runs every
/// few seconds, a crash or hard shutdown loses at most one interval.
fn save_state(app: &AppHandle) {
    let state = app.state::<AppState>();
    let position = *lock(&state.main_position);
    let changed = {
        let conn = state.db();
        if let Some(position) = position {
            let _ = repo::save_window_position(&conn, position);
        }
        repo::autosave(&conn, repo::now()).unwrap_or_else(|e| {
            log::error!("Auto-Save fehlgeschlagen: {e}");
            false
        })
    };
    if changed {
        commands::notify(app);
    }
}

/// Time keeps running while the PC is locked or asleep: the running work simply goes on.
/// A panic in one save must not end the thread, or nothing would be saved any more.
fn start_autosave(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(AUTOSAVE_INTERVAL);
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| save_state(&app))).is_err() {
                log::error!("Auto-Save abgestürzt, nächster Versuch in {} s.", AUTOSAVE_INTERVAL.as_secs());
            }
        }
    });
}

/// Panics end up in the log file instead of vanishing with the console-less release exe.
fn log_panics() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("Panic: {info}
{}", std::backtrace::Backtrace::force_capture());
        default(info);
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must come first: a second InTime would record times in parallel to the first one.
        // Instead it exits and the running instance comes to the front.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| windows::show_main(app)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .clear_targets()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::Folder { path: paths::logs_dir(), file_name: Some("intime".into()) }),
                ])
                .build(),
        )
        .setup(|app| {
            log_panics();
            log::info!("InTime {} gestartet.", app.package_info().version);
            windows::set_app_id();
            let data_dir = paths::data_dir();
            std::fs::create_dir_all(&data_dir)?;
            let conn = db::open(&data_dir.join("intime.db"))?;
            repo::recover_and_start(&conn)?;
            app.manage(AppState {
                db: Mutex::new(conn),
                main_position: Mutex::new(None),
                main_layout: Mutex::new(windows::MainLayout::default()),
            });

            let handle = app.handle();
            windows::create_main(handle)?;
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
                    *lock(&window.state::<AppState>().main_position) = Some((pos.x, pos.y));
                }
                // The region is in physical pixels and has to follow DPI changes.
                WindowEvent::ScaleFactorChanged { .. } | WindowEvent::Resized(_) => {
                    let _ = windows::apply_main_region(window.app_handle());
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
            commands::set_main_collapsed,
            commands::set_window_icon,
            commands::quit_app,
            commands::heimat_get_config,
            commands::heimat_save_config,
            commands::heimat_test,
            commands::heimat_tasks,
            commands::heimat_get_mappings,
            commands::heimat_save_mappings,
            commands::heimat_import_tasks,
            commands::heimat_day,
            commands::heimat_sync,
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
                let conn = state.db();
                if let Err(e) = repo::finish_active(&conn, repo::now(), None) {
                    log::error!("Beenden: laufender Eintrag konnte nicht gespeichert werden: {e}");
                }
            }
        });
}
