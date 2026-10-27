//! Keeps the tray icon, tray tooltip and taskbar icon in the color of the running project.

use tauri::image::Image;
use tauri::{AppHandle, Manager};

use crate::AppState;
use crate::repo;

pub const TRAY_ID: &str = "main";

fn parse_hex(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    let channel = |i: usize| hex.get(i..i + 2).and_then(|c| u8::from_str_radix(c, 16).ok()).unwrap_or(128);
    (channel(0), channel(2), channel(4))
}

/// An anti-aliased filled circle with a slightly darker rim, so light colors stay visible.
pub fn circle_icon(hex: &str, size: u32) -> Image<'static> {
    let (r, g, b) = parse_hex(hex);
    let center = (size as f32 - 1.0) / 2.0;
    let radius = size as f32 / 2.0 - 0.5;
    let rim = (size as f32 / 12.0).max(1.0);
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let d = ((x as f32 - center).powi(2) + (y as f32 - center).powi(2)).sqrt();
            let alpha = (radius - d + 0.5).clamp(0.0, 1.0);
            let shade = if d > radius - rim { 0.7 } else { 1.0 };
            let i = ((y * size + x) * 4) as usize;
            rgba[i] = (r as f32 * shade) as u8;
            rgba[i + 1] = (g as f32 * shade) as u8;
            rgba[i + 2] = (b as f32 * shade) as u8;
            rgba[i + 3] = (alpha * 255.0) as u8;
        }
    }
    Image::new_owned(rgba, size, size)
}

pub fn refresh(app: &AppHandle) {
    let project = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().unwrap();
        repo::active_work(&conn).ok().flatten().and_then(|w| repo::project(&conn, w.project_id).ok())
    };
    let Some(project) = project else { return };

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(circle_icon(&project.color, 32)));
        let _ = tray.set_tooltip(Some(format!("InTime – {}", project.name)));
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_icon(circle_icon(&project.color, 64));
    }
}
