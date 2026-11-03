use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::AppState;
use crate::error::{Error, Result};
use crate::indicator::{TRAY_ID, circle_icon};
use crate::{paths, repo};

/// Opens (or focuses) a secondary window. All windows load the same frontend;
/// the view to render is passed via `window.__INTIME__`.
pub fn open(app: &AppHandle, view: &str, id: Option<i64>) -> Result<()> {
    let (label, title, width, height) = match view {
        "report" => ("report".to_string(), "InTime – Bericht", 1080.0, 740.0),
        "settings" => ("settings".to_string(), "InTime – Einstellungen", 760.0, 520.0),
        "heimat" => ("heimat".to_string(), "InTime – Heimat-Zuordnung", 820.0, 640.0),
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
    // Built hidden and shown once it sits in the middle of the target monitor.
    let mut builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(width, height)
        .min_inner_size(400.0, 400.0)
        .visible(false)
        .data_directory(paths::webview_dir())
        .initialization_script(&init);
    // Owned by the always-on-top main window, so Windows keeps it above the main window.
    if let Some(main) = app.get_webview_window("main") {
        builder = builder.parent(&main)?;
    }
    let window = builder.build()?;
    if let Err(e) = center_on_target_monitor(app, &window) {
        log::warn!("Fenster konnte nicht positioniert werden: {e}");
        window.center()?;
    }
    window.show()?;
    window.set_focus()?;
    Ok(())
}

/// Centers `window` in the work area of the monitor the main window is on, or of the
/// primary monitor if that is turned off in the settings.
fn center_on_target_monitor(app: &AppHandle, window: &WebviewWindow) -> Result<()> {
    let on_tracker_monitor = {
        let state = app.state::<AppState>();
        repo::settings(&state.db())?.windows_on_tracker_monitor
    };
    let main = app.get_webview_window("main");
    let monitor = match main {
        Some(main) if on_tracker_monitor => main.current_monitor()?,
        _ => app.primary_monitor()?,
    };
    let Some(monitor) = monitor else { return Err(Error::msg("Kein Bildschirm gefunden.")) };

    // The window was created with the scale of another monitor; size it for the target one.
    let ratio = monitor.scale_factor() / window.scale_factor()?;
    let size = window.outer_size()?;
    let area = monitor.work_area();
    let width = ((size.width as f64 * ratio) as i32).min(area.size.width as i32);
    let height = ((size.height as f64 * ratio) as i32).min(area.size.height as i32);
    let x = area.position.x + (area.size.width as i32 - width) / 2;
    let y = area.position.y + (area.size.height as i32 - height) / 2;
    window.set_position(PhysicalPosition::new(x, y))?;
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

/// Which part of the main window is visible. The window itself keeps its size and position;
/// hiding the project list only clips it away with a window region. Resizing and moving the
/// window instead made the WebView show its old content at the wrong place for a moment, so
/// the panel jumped back and forth whenever the list was shown or hidden.
#[derive(Debug, Clone, Copy, Default)]
pub struct MainLayout {
    pub collapsed: bool,
    /// The project list is left of the main panel (it is clipped from the left then).
    pub list_left: bool,
    /// Width of the main panel in logical pixels; the part that stays visible.
    pub main_width: f64,
}

/// The region last set, so window events that do not change it are ignored (setting a region
/// can itself cause window events, which must not lead to setting it again and again).
static APPLIED_REGION: std::sync::Mutex<Option<[i32; 4]>> = std::sync::Mutex::new(None);

pub fn set_main_layout(app: &AppHandle, layout: MainLayout) -> Result<()> {
    *crate::lock(&app.state::<AppState>().main_layout) = layout;
    apply_main_region(app)
}

/// Clips the main window to the main panel while the project list is hidden. A clipped part
/// is not part of the window at all: it shows and receives clicks of whatever is behind it.
/// A region is also kept while the list is shown, so the window looks the same in both states
/// (Windows draws no shadow or rounded corners for windows with a region).
pub fn apply_main_region(app: &AppHandle) -> Result<()> {
    use ::windows::Win32::Foundation::{POINT, RECT};
    use ::windows::Win32::Graphics::Gdi::{ClientToScreen, CreateRectRgn, DeleteObject, SetWindowRgn};
    use ::windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect};

    let Some(window) = app.get_webview_window("main") else { return Ok(()) };
    let layout = *crate::lock(&app.state::<AppState>().main_layout);
    let hwnd = window.hwnd()?;
    let main_width = (layout.main_width * window.scale_factor()?).round() as i32;
    let fail = |e: ::windows::core::Error| Error::msg(format!("Fensterbereich konnte nicht gesetzt werden: {e}"));
    unsafe {
        // Region coordinates are relative to the window rectangle, the panel to the client area.
        let (mut window_rect, mut client, mut origin) = (RECT::default(), RECT::default(), POINT::default());
        GetWindowRect(hwnd, &mut window_rect).map_err(fail)?;
        GetClientRect(hwnd, &mut client).map_err(fail)?;
        let _ = ClientToScreen(hwnd, &mut origin);
        let (dx, dy) = (origin.x - window_rect.left, origin.y - window_rect.top);
        let (width, height) = (client.right, client.bottom);
        // Minimized: keep the region, it still fits once the window is restored.
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        let (left, right) = match layout {
            MainLayout { collapsed: false, .. } => (0, width),
            MainLayout { list_left: true, .. } => ((width - main_width).max(0), width),
            MainLayout { list_left: false, .. } => (0, main_width.min(width)),
        };
        let rect = [dx + left, dy, dx + right, dy + height];
        let mut applied = crate::lock(&APPLIED_REGION);
        if *applied == Some(rect) {
            return Ok(());
        }
        let region = CreateRectRgn(rect[0], rect[1], rect[2], rect[3]);
        // On success the system owns the region; only a failed call leaves it to us.
        if SetWindowRgn(hwnd, Some(region), true) == 0 {
            let _ = DeleteObject(region.into());
            return Err(Error::msg("Fensterbereich konnte nicht gesetzt werden."));
        }
        *applied = Some(rect);
    }
    Ok(())
}

/// Gives InTime its own AppUserModelID; must run before the first window is created.
/// Without it Windows matches the exe to its Start menu entry and shows that entry's (cached)
/// icon in the taskbar instead of the window icon, so the project color would never appear.
pub fn set_app_id() {
    use ::windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
    use ::windows::core::w;

    if let Err(e) = unsafe { SetCurrentProcessExplicitAppUserModelID(w!("AdmirSchwab.InTime.Tracker")) } {
        log::warn!("App-ID konnte nicht gesetzt werden: {e}");
    }
}

/// Sets a window's title bar and taskbar icon from RGBA pixels. Tauri's `set_icon` only sets
/// the small icon; without a big one the taskbar falls back to the exe icon from Windows'
/// icon cache, which can still be an old one after an update.
pub fn set_icon(window: &WebviewWindow, rgba: Vec<u8>, width: u32, height: u32) -> Result<()> {
    use ::windows::Win32::Foundation::{LPARAM, WPARAM};
    use ::windows::Win32::UI::WindowsAndMessaging::{CreateIcon, DestroyIcon, HICON, ICON_BIG, SendMessageW, WM_SETICON};

    if rgba.len() != (width * height * 4) as usize {
        return Err(Error::msg("Ungültige Icon-Daten."));
    }
    window.set_icon(Image::new_owned(rgba.clone(), width, height))?;

    // 32-bit icons take BGRA pixels with alpha; the AND mask is then ignored (as in tao).
    let mut bgra = rgba;
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let and_mask = vec![0u8; bgra.len() / 4];
    unsafe {
        let icon = CreateIcon(None, width as i32, height as i32, 1, 32, and_mask.as_ptr(), bgra.as_ptr())
            .map_err(|e| Error::msg(format!("Icon konnte nicht erstellt werden: {e}")))?;
        let previous = SendMessageW(
            window.hwnd()?,
            WM_SETICON,
            Some(WPARAM(ICON_BIG as usize)),
            Some(LPARAM(icon.0 as isize)),
        );
        // Only this function sets big icons, so the replaced one is ours to free.
        if previous.0 != 0 {
            let _ = DestroyIcon(HICON(previous.0 as *mut _));
        }
    }
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
        let conn = state.db();
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
