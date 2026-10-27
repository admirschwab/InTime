use std::path::Path;

use rusqlite::Connection;

use crate::error::Result;

/// Schema migrations, applied in order. `PRAGMA user_version` stores how many have run.
/// Never edit an existing entry – append a new one instead.
const MIGRATIONS: &[&str] = &[r#"
    CREATE TABLE project (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        name        TEXT    NOT NULL,
        description TEXT    NOT NULL DEFAULT '',
        color       TEXT    NOT NULL,
        is_work     INTEGER NOT NULL DEFAULT 1,
        is_default  INTEGER NOT NULL DEFAULT 0,
        is_enabled  INTEGER NOT NULL DEFAULT 1,
        sort_index  INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE work (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        project_id INTEGER NOT NULL REFERENCES project(id),
        start_time TEXT    NOT NULL,
        end_time   TEXT    NOT NULL,
        notes      TEXT    NOT NULL DEFAULT '',
        active     INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX idx_work_start ON work(start_time);
    CREATE TABLE setting (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
"#, r#"
    -- Which Heimat task an InTime project is booked on (v0.2.0).
    CREATE TABLE heimat_mapping (
        project_id       INTEGER PRIMARY KEY REFERENCES project(id) ON DELETE CASCADE,
        task_id          INTEGER NOT NULL,
        task_name        TEXT    NOT NULL,
        task_holder_name TEXT    NOT NULL
    );
"#];

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    let version: usize = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i + 1)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
pub fn open_in_memory() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrate(&conn).unwrap();
    conn
}
