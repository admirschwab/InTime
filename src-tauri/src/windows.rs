use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

use crate::AppState;
use crate::error::{Error, Result};
use crate::indicator::{TRAY_ID, circle_icon};
use crate::{paths, repo};

/// Opens (or focuses) a secondary window. All windows load the same frontend;
/// the view to render is passed via `window.__INTIME__`.
pub fn open(app: &AppHandle, view: &str, id: Option<i64>) -> Result<()> {
    let (label, title, width, height) = match view {
        "report" => ("report".to_string(), "InTime – Bericht", 980.0, 700.0),
        "settings" => ("settings".to_string(), "InTime – Einstellungen", 760.0, 520.0),
        "project" => {
            let label = format!("project-{}", id.map_or("new".to_string(), |id| id.to_string()));
            let title = if id.is_some() { "InTime – Projekt bearbeiten" } else { "InTime – Neues Projekt" };
            (label, title, 440.0, 520.0)
        }
        _ => return Err(Error::msg(format!("Unbekannte Ansicht: {view}"))),
    };

    if let Some(window) = app.get_webview_window(&label) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    let init = format!("window.__INTIME__ = {};", serde_json::json!({ "view": view, "id": id }));
    let mut builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(width, height)
        .min_inner_size(400.0, 400.0)
        .center()
        .data_directory(paths::webview_dir())
        .initialization_script(&init);
    // Owned by the always-on-top main window, so Windows keeps it above the main window.
    if let Some(main) = app.get_webview_window("main") {
        builder = builder.parent(&main)?;
    }
    builder.build()?;
    Ok(())
}

/// The small always-on-top tracker window. Created in code (not tauri.conf.json)
/// so its WebView data goes to `%LOCALAPPDATA%\InTime\webview`.
pub fn create_main(app: &AppHandle) -> Result<()> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("InTime")
        .inner_size(540.0, 320.0)
        .resizable(false)
        .maximizable(false)
        .decorations(false)
        .always_on_top(true)
        .center()
        .data_directory(paths::webview_dir())
        .build()?;
    Ok(())
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn create_tray(app: &AppHandle) -> Result<()> {
    let show = MenuItemBuilder::with_id("show", "Anzeigen").build(app)?;
    let report = MenuItemBuilder::with_id("report", "Bericht").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Einstellungen").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Beenden").build(app)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = MenuBuilder::new(app).items(&[&show, &report, &settings, &separator, &quit]).build()?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(circle_icon("#8a8f98", 32))
        .tooltip("InTime")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main(app),
            "report" => {
                let _ = open(app, "report", None);
            }
            "settings" => {
                let _ = open(app, "settings", None);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Moves the main window to its last saved position if that is still on a connected monitor.
pub fn restore_main_position(app: &AppHandle) -> Result<()> {
    let position = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        if !repo::settings(&conn)?.save_window_position {
            return Ok(());
        }
        repo::window_position(&conn)?
    };
    let (Some((x, y)), Some(window)) = (position, app.get_webview_window("main")) else { return Ok(()) };
    let on_screen = window.available_monitors()?.iter().any(|m| {
        let (pos, size) = (m.position(), m.size());
        x >= pos.x && y >= pos.y && x < pos.x + size.width as i32 - 40 && y < pos.y + size.height as i32 - 40
    });
    if on_screen {
        window.set_position(PhysicalPosition::new(x, y))?;
    }
    Ok(())
}
