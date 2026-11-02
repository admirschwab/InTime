//! Everything InTime stores lives under `%LOCALAPPDATA%\InTime`, next to the installed exe:
//!
//! ```text
//! %LOCALAPPDATA%\InTime\
//!   InTime.exe, uninstall.ps1, install.json   (written by the installer)
//!   data\intime.db                            (projects, times, settings)
//!   logs\                                     (log files)
//!   webview\                                  (WebView2 cache)
//! ```

use std::path::PathBuf;

pub fn base_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("InTime")
}

pub fn data_dir() -> PathBuf {
    base_dir().join("data")
}

pub fn logs_dir() -> PathBuf {
    base_dir().join("logs")
}

pub fn webview_dir() -> PathBuf {
    base_dir().join("webview")
}
