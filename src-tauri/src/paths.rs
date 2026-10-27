//! Everything InTime stores lives under `%LOCALAPPDATA%\InTime`, next to the installed exe:
//!
//! ```text
//! %LOCALAPPDATA%\InTime\
//!   InTime.exe, uninstall.ps1, install.json   (written by the installer)
//!   data\intime.db                            (projects, times, settings)
//!   logs\                                     (log files)
//!   webview\                                  (WebView2 cache)
//! ```

use std::path::{Path, PathBuf};

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

/// Versions up to 0.1.0 kept the database in `%APPDATA%\app.intime.desktop`.
/// Copies it over once if the new location has no database yet.
pub fn migrate_legacy_db(legacy_dir: &Path, db_path: &Path) -> std::io::Result<()> {
    let legacy_db = legacy_dir.join("intime.db");
    if db_path.exists() || !legacy_db.exists() {
        return Ok(());
    }
    for suffix in ["", "-wal", "-shm"] {
        let from = legacy_dir.join(format!("intime.db{suffix}"));
        if from.exists() {
            std::fs::copy(&from, db_path.with_file_name(format!("intime.db{suffix}")))?;
        }
    }
    log::info!("Datenbank von {} übernommen", legacy_dir.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_db_is_copied_once() {
        let root = std::env::temp_dir().join(format!("intime-test-{}", std::process::id()));
        let (legacy, new) = (root.join("legacy"), root.join("new"));
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(legacy.join("intime.db"), b"old").unwrap();
        let db = new.join("intime.db");

        migrate_legacy_db(&legacy, &db).unwrap();
        assert_eq!(std::fs::read(&db).unwrap(), b"old");

        // An existing database is never overwritten.
        std::fs::write(&db, b"current").unwrap();
        migrate_legacy_db(&legacy, &db).unwrap();
        assert_eq!(std::fs::read(&db).unwrap(), b"current");

        std::fs::remove_dir_all(root).unwrap();
    }
}
