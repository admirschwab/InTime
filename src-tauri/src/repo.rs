use chrono::{Duration, Local, NaiveDate, NaiveDateTime, Timelike};
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::error::{Error, Result};
use crate::models::{
    Current, ExportData, HeimatConfig, HeimatMapping, Project, ProjectInput, ProjectTime, Settings, Work, WorkInput,
};

pub const DT_FMT: &str = "%Y-%m-%dT%H:%M:%S";

pub fn now() -> NaiveDateTime {
    Local::now().naive_local().with_nanosecond(0).unwrap()
}

pub fn fmt(dt: NaiveDateTime) -> String {
    dt.format(DT_FMT).to_string()
}

pub fn parse(s: &str) -> Result<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, DT_FMT)
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M"))
        .map_err(|_| Error::msg(format!("Ungültiger Zeitpunkt: {s}")))
}

fn day_start(date: NaiveDate) -> String {
    fmt(date.and_hms_opt(0, 0, 0).unwrap())
}

// ---------------------------------------------------------------- projects

const PROJECT_COLS: &str = "id, name, description, color, is_work, is_default, is_enabled, sort_index";

fn project_from_row(r: &Row) -> rusqlite::Result<Project> {
    Ok(Project {
        id: r.get(0)?,
        name: r.get(1)?,
        description: r.get(2)?,
        color: r.get(3)?,
        is_work: r.get(4)?,
        is_default: r.get(5)?,
        is_enabled: r.get(6)?,
        sort_index: r.get(7)?,
    })
}

pub fn projects(conn: &Connection, include_disabled: bool) -> Result<Vec<Project>> {
    let filter = if include_disabled { "" } else { "WHERE is_enabled = 1" };
    let mut stmt = conn.prepare(&format!("SELECT {PROJECT_COLS} FROM project {filter} ORDER BY sort_index, id"))?;
    let rows = stmt.query_map([], project_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn project(conn: &Connection, id: i64) -> Result<Project> {
    conn.query_row(&format!("SELECT {PROJECT_COLS} FROM project WHERE id = ?1"), [id], project_from_row)
        .optional()?
        .ok_or_else(|| Error::msg("Projekt nicht gefunden."))
}

/// Returns the default project, creating one if none exists (e.g. on first start).
pub fn default_project_id(conn: &Connection) -> Result<i64> {
    let id: Option<i64> = conn
        .query_row(
            "SELECT id FROM project WHERE is_default = 1 AND is_enabled = 1 ORDER BY id LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    match id {
        Some(id) => Ok(id),
        None => {
            let input = ProjectInput {
                name: "Leerlauf".into(),
                description: "Wird beim Start automatisch ausgewählt.".into(),
                color: "#8a8f98".into(),
                is_work: false,
                is_default: true,
            };
            Ok(create_project(conn, &input)?.id)
        }
    }
}

fn validate_project(input: &ProjectInput) -> Result<()> {
    if input.name.trim().is_empty() {
        return Err(Error::msg("Der Projektname darf nicht leer sein."));
    }
    Ok(())
}

pub fn create_project(conn: &Connection, input: &ProjectInput) -> Result<Project> {
    validate_project(input)?;
    let next: i64 = conn.query_row("SELECT COALESCE(MAX(sort_index), -1) + 1 FROM project", [], |r| r.get(0))?;
    if input.is_default {
        conn.execute("UPDATE project SET is_default = 0", [])?;
    }
    conn.execute(
        "INSERT INTO project (name, description, color, is_work, is_default, is_enabled, sort_index)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
        params![input.name.trim(), input.description, input.color, input.is_work, input.is_default, next],
    )?;
    project(conn, conn.last_insert_rowid())
}

pub fn update_project(conn: &Connection, id: i64, input: &ProjectInput) -> Result<Project> {
    validate_project(input)?;
    let existing = project(conn, id)?;
    if existing.is_default && !input.is_default {
        return Err(Error::msg(
            "Es muss immer ein Standardprojekt geben. Markiere stattdessen ein anderes Projekt als Standard.",
        ));
    }
    if input.is_default {
        conn.execute("UPDATE project SET is_default = 0 WHERE id != ?1", [id])?;
    }
    conn.execute(
        "UPDATE project SET name = ?2, description = ?3, color = ?4, is_work = ?5, is_default = ?6 WHERE id = ?1",
        params![id, input.name.trim(), input.description, input.color, input.is_work, input.is_default],
    )?;
    project(conn, id)
}

/// Projects are only disabled, so past work keeps its project in reports.
pub fn delete_project(conn: &Connection, id: i64) -> Result<()> {
    if project(conn, id)?.is_default {
        return Err(Error::msg("Das Standardprojekt kann nicht gelöscht werden."));
    }
    conn.execute("UPDATE project SET is_enabled = 0, is_default = 0 WHERE id = ?1", [id])?;
    conn.execute("DELETE FROM heimat_mapping WHERE project_id = ?1", [id])?;
    if active_work(conn)?.is_some_and(|w| w.project_id == id) {
        let default_id = default_project_id(conn)?;
        switch_project(conn, default_id, None, 0)?;
    }
    Ok(())
}

pub fn reorder_projects(conn: &Connection, ids: &[i64]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    for (index, id) in ids.iter().enumerate() {
        tx.execute("UPDATE project SET sort_index = ?2 WHERE id = ?1", params![id, index as i64])?;
    }
    tx.commit()?;
    Ok(())
}

// ---------------------------------------------------------------- works

const WORK_COLS: &str = "id, project_id, start_time, end_time, notes, active";

fn work_from_row(r: &Row) -> rusqlite::Result<Work> {
    Ok(Work {
        id: r.get(0)?,
        project_id: r.get(1)?,
        start_time: r.get(2)?,
        end_time: r.get(3)?,
        notes: r.get(4)?,
        active: r.get(5)?,
    })
}

pub fn active_work(conn: &Connection) -> Result<Option<Work>> {
    Ok(conn
        .query_row(&format!("SELECT {WORK_COLS} FROM work WHERE active = 1 ORDER BY id DESC LIMIT 1"), [], work_from_row)
        .optional()?)
}

fn start_active(conn: &Connection, project_id: i64, start: NaiveDateTime) -> Result<()> {
    let start = fmt(start);
    conn.execute(
        "INSERT INTO work (project_id, start_time, end_time, notes, active) VALUES (?1, ?2, ?2, '', 1)",
        params![project_id, start],
    )?;
    Ok(())
}

/// Ends the running work at `end`. Keeps the stored notes unless `notes` is given.
/// A running work without duration and notes is dropped instead of saved.
pub fn finish_active(conn: &Connection, end: NaiveDateTime, notes: Option<&str>) -> Result<()> {
    let Some(work) = active_work(conn)? else { return Ok(()) };
    let start = parse(&work.start_time)?;
    let end = end.max(start);
    let notes = notes.map(str::to_owned).unwrap_or(work.notes);
    if end == start && notes.trim().is_empty() {
        conn.execute("DELETE FROM work WHERE id = ?1", [work.id])?;
    } else {
        conn.execute(
            "UPDATE work SET end_time = ?2, notes = ?3, active = 0 WHERE id = ?1",
            params![work.id, fmt(end), notes],
        )?;
    }
    Ok(())
}

/// Switches to another project. With `transfer_minutes` the last n minutes of the
/// running work are booked on the new project instead.
pub fn switch_project(conn: &Connection, project_id: i64, notes: Option<&str>, transfer_minutes: i64) -> Result<()> {
    let target = project(conn, project_id)?;
    if !target.is_enabled {
        return Err(Error::msg("Dieses Projekt wurde gelöscht."));
    }
    let mut at = now() - Duration::minutes(transfer_minutes.max(0));
    if let Some(work) = active_work(conn)? {
        at = at.max(parse(&work.start_time)?);
    }
    let tx = conn.unchecked_transaction()?;
    finish_active(&tx, at, notes)?;
    start_active(&tx, project_id, at)?;
    tx.commit()?;
    Ok(())
}

/// Called on startup: closes work left running by a crash (its end time is the last
/// auto-save) and starts the default project.
pub fn recover_and_start(conn: &Connection) -> Result<()> {
    conn.execute(
        "DELETE FROM work WHERE active = 1 AND start_time = end_time AND trim(notes) = ''",
        [],
    )?;
    conn.execute("UPDATE work SET active = 0 WHERE active = 1", [])?;
    let default_id = default_project_id(conn)?;
    start_active(conn, default_id, now())
}

pub fn autosave(conn: &Connection) -> Result<()> {
    conn.execute("UPDATE work SET end_time = ?1 WHERE active = 1", [fmt(now())])?;
    Ok(())
}

pub fn update_current_notes(conn: &Connection, notes: &str) -> Result<()> {
    conn.execute("UPDATE work SET notes = ?1 WHERE active = 1", [notes])?;
    Ok(())
}

pub fn current(conn: &Connection) -> Result<Current> {
    let work = match active_work(conn)? {
        Some(work) => work,
        None => {
            start_active(conn, default_project_id(conn)?, now())?;
            active_work(conn)?.expect("active work was just created")
        }
    };
    let today = day_start(Local::now().date_naive());
    let mut stmt = conn.prepare(
        "SELECT project_id, SUM(strftime('%s', end_time) - strftime('%s', start_time))
         FROM work WHERE active = 0 AND start_time >= ?1 GROUP BY project_id",
    )?;
    let today = stmt
        .query_map([today], |r| Ok(ProjectTime { project_id: r.get(0)?, seconds: r.get(1)? }))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Current { work, today })
}

pub fn works_for_day(conn: &Connection, date: NaiveDate) -> Result<Vec<Work>> {
    let next = date.succ_opt().ok_or_else(|| Error::msg("Ungültiges Datum."))?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {WORK_COLS} FROM work WHERE start_time >= ?1 AND start_time < ?2 ORDER BY start_time, id"
    ))?;
    let rows = stmt
        .query_map([day_start(date), day_start(next)], work_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn days_with_work(conn: &Connection, year: i32, month: u32) -> Result<Vec<u32>> {
    let prefix = format!("{year:04}-{month:02}-%");
    let mut stmt =
        conn.prepare("SELECT DISTINCT CAST(substr(start_time, 9, 2) AS INTEGER) FROM work WHERE start_time LIKE ?1")?;
    let rows = stmt.query_map([prefix], |r| r.get(0))?.collect::<rusqlite::Result<Vec<u32>>>()?;
    Ok(rows)
}

fn validate_work(conn: &Connection, input: &WorkInput) -> Result<(String, String)> {
    let start = parse(&input.start_time)?;
    let end = parse(&input.end_time)?;
    if end <= start {
        return Err(Error::msg("Das Ende muss nach dem Beginn liegen."));
    }
    if end > now() {
        return Err(Error::msg("Der Eintrag darf nicht in der Zukunft enden."));
    }
    project(conn, input.project_id)?;
    Ok((fmt(start), fmt(end)))
}

fn ensure_not_active(conn: &Connection, id: i64) -> Result<()> {
    let active: Option<bool> =
        conn.query_row("SELECT active FROM work WHERE id = ?1", [id], |r| r.get(0)).optional()?;
    match active {
        None => Err(Error::msg("Eintrag nicht gefunden.")),
        Some(true) => Err(Error::msg("Der laufende Eintrag kann nicht bearbeitet werden.")),
        Some(false) => Ok(()),
    }
}

pub fn create_work(conn: &Connection, input: &WorkInput) -> Result<()> {
    let (start, end) = validate_work(conn, input)?;
    conn.execute(
        "INSERT INTO work (project_id, start_time, end_time, notes, active) VALUES (?1, ?2, ?3, ?4, 0)",
        params![input.project_id, start, end, input.notes],
    )?;
    Ok(())
}

pub fn update_work(conn: &Connection, id: i64, input: &WorkInput) -> Result<()> {
    ensure_not_active(conn, id)?;
    let (start, end) = validate_work(conn, input)?;
    conn.execute(
        "UPDATE work SET project_id = ?2, start_time = ?3, end_time = ?4, notes = ?5 WHERE id = ?1",
        params![id, input.project_id, start, end, input.notes],
    )?;
    Ok(())
}

pub fn delete_work(conn: &Connection, id: i64) -> Result<()> {
    ensure_not_active(conn, id)?;
    conn.execute("DELETE FROM work WHERE id = ?1", [id])?;
    Ok(())
}

// ---------------------------------------------------------------- settings

pub fn settings(conn: &Connection) -> Result<Settings> {
    let value: Option<String> =
        conn.query_row("SELECT value FROM setting WHERE key = 'settings'", [], |r| r.get(0)).optional()?;
    Ok(value.and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default())
}

pub fn save_settings(conn: &Connection, settings: &Settings) -> Result<()> {
    set_value(conn, "settings", &serde_json::to_string(settings)?)
}

pub fn window_position(conn: &Connection) -> Result<Option<(i32, i32)>> {
    let value: Option<String> =
        conn.query_row("SELECT value FROM setting WHERE key = 'window_position'", [], |r| r.get(0)).optional()?;
    Ok(value.and_then(|v| serde_json::from_str(&v).ok()))
}

pub fn save_window_position(conn: &Connection, pos: (i32, i32)) -> Result<()> {
    set_value(conn, "window_position", &serde_json::to_string(&pos)?)
}

fn set_value(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO setting (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

// ---------------------------------------------------------------- export / import

pub fn export(conn: &Connection) -> Result<ExportData> {
    let mut stmt = conn.prepare(&format!("SELECT {WORK_COLS} FROM work ORDER BY start_time, id"))?;
    let now = fmt(now());
    let works = stmt
        .query_map([], work_from_row)?
        .map(|w| {
            w.map(|mut w| {
                if w.active {
                    w.end_time = now.clone();
                    w.active = false;
                }
                w
            })
        })
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ExportData {
        app: "InTime".into(),
        version: 1,
        exported_at: now,
        projects: projects(conn, true)?,
        works,
        settings: settings(conn)?,
        heimat_mappings: heimat_mappings(conn)?,
    })
}

/// Replaces all projects, works and settings with the imported data.
pub fn import(conn: &Connection, data: &ExportData) -> Result<()> {
    if data.app != "InTime" {
        return Err(Error::msg("Die Datei ist kein InTime-Export."));
    }
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM work", [])?;
    tx.execute("DELETE FROM heimat_mapping", [])?;
    tx.execute("DELETE FROM project", [])?;
    for p in &data.projects {
        tx.execute(
            &format!("INSERT INTO project ({PROJECT_COLS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"),
            params![p.id, p.name, p.description, p.color, p.is_work, p.is_default, p.is_enabled, p.sort_index],
        )?;
    }
    for w in &data.works {
        tx.execute(
            "INSERT INTO work (id, project_id, start_time, end_time, notes, active) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![w.id, w.project_id, fmt(parse(&w.start_time)?), fmt(parse(&w.end_time)?), w.notes],
        )?;
    }
    save_settings(&tx, &data.settings)?;
    insert_heimat_mappings(&tx, &data.heimat_mappings)?;
    tx.commit()?;
    Ok(())
}

// ---------------------------------------------------------------- Heimat

pub fn heimat_config(conn: &Connection) -> Result<HeimatConfig> {
    let value: Option<String> =
        conn.query_row("SELECT value FROM setting WHERE key = 'heimat'", [], |r| r.get(0)).optional()?;
    Ok(value.and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default())
}

pub fn save_heimat_config(conn: &Connection, config: &HeimatConfig) -> Result<()> {
    let config = HeimatConfig { url: config.url.trim().trim_end_matches('/').to_string(), token: config.token.trim().to_string() };
    set_value(conn, "heimat", &serde_json::to_string(&config)?)
}

pub fn heimat_mappings(conn: &Connection) -> Result<Vec<HeimatMapping>> {
    let mut stmt = conn.prepare(
        "SELECT m.project_id, m.task_id, m.task_name, m.task_holder_name
         FROM heimat_mapping m JOIN project p ON p.id = m.project_id ORDER BY p.sort_index, p.id",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(HeimatMapping { project_id: r.get(0)?, task_id: r.get(1)?, task_name: r.get(2)?, task_holder_name: r.get(3)? })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Replaces all mappings; projects missing from `mappings` are no longer mapped.
pub fn replace_heimat_mappings(conn: &Connection, mappings: &[HeimatMapping]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM heimat_mapping", [])?;
    insert_heimat_mappings(&tx, mappings)?;
    tx.commit()?;
    Ok(())
}

pub fn insert_heimat_mappings(conn: &Connection, mappings: &[HeimatMapping]) -> Result<()> {
    for m in mappings {
        conn.execute(
            "INSERT OR REPLACE INTO heimat_mapping (project_id, task_id, task_name, task_holder_name) VALUES (?1, ?2, ?3, ?4)",
            params![m.project_id, m.task_id, m.task_name, m.task_holder_name],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    fn input(name: &str) -> ProjectInput {
        ProjectInput { name: name.into(), description: String::new(), color: "#ff0000".into(), is_work: true, is_default: false }
    }

    #[test]
    fn startup_creates_default_project_and_running_work() {
        let conn = open_in_memory();
        recover_and_start(&conn).unwrap();
        let projects = projects(&conn, false).unwrap();
        assert_eq!(projects.len(), 1);
        assert!(projects[0].is_default);
        assert_eq!(active_work(&conn).unwrap().unwrap().project_id, projects[0].id);
    }

    #[test]
    fn switch_saves_notes_and_starts_new_work() {
        let conn = open_in_memory();
        recover_and_start(&conn).unwrap();
        let first = active_work(&conn).unwrap().unwrap();
        conn.execute("UPDATE work SET start_time = ?1 WHERE id = ?2", params![fmt(now() - Duration::minutes(30)), first.id])
            .unwrap();
        let p = create_project(&conn, &input("A")).unwrap();

        switch_project(&conn, p.id, Some("erledigt"), 10).unwrap();

        let old: Work = conn.query_row(&format!("SELECT {WORK_COLS} FROM work WHERE id = ?1"), [first.id], work_from_row).unwrap();
        assert!(!old.active);
        assert_eq!(old.notes, "erledigt");
        let running = active_work(&conn).unwrap().unwrap();
        assert_eq!(running.project_id, p.id);
        assert_eq!(running.start_time, old.end_time);
        let transferred = now() - parse(&running.start_time).unwrap();
        assert!(transferred >= Duration::minutes(10) && transferred < Duration::minutes(11));
    }

    #[test]
    fn transfer_cannot_go_before_running_work_start() {
        let conn = open_in_memory();
        recover_and_start(&conn).unwrap();
        let first = active_work(&conn).unwrap().unwrap();
        let p = create_project(&conn, &input("A")).unwrap();
        switch_project(&conn, p.id, Some("x"), 600).unwrap();
        assert_eq!(active_work(&conn).unwrap().unwrap().start_time, first.start_time);
    }

    #[test]
    fn default_project_cannot_be_deleted_and_deleting_active_switches_to_default() {
        let conn = open_in_memory();
        recover_and_start(&conn).unwrap();
        let default_id = default_project_id(&conn).unwrap();
        assert!(delete_project(&conn, default_id).is_err());

        let p = create_project(&conn, &input("A")).unwrap();
        switch_project(&conn, p.id, None, 0).unwrap();
        delete_project(&conn, p.id).unwrap();
        assert_eq!(active_work(&conn).unwrap().unwrap().project_id, default_id);
        assert_eq!(projects(&conn, false).unwrap().len(), 1);
        assert_eq!(projects(&conn, true).unwrap().len(), 2);
    }

    #[test]
    fn running_work_cannot_be_edited() {
        let conn = open_in_memory();
        recover_and_start(&conn).unwrap();
        let running = active_work(&conn).unwrap().unwrap();
        assert!(delete_work(&conn, running.id).is_err());
    }

    #[test]
    fn export_import_roundtrip() {
        let conn = open_in_memory();
        recover_and_start(&conn).unwrap();
        let p = create_project(&conn, &input("A")).unwrap();
        let end = now() - Duration::hours(1);
        create_work(
            &conn,
            &WorkInput { project_id: p.id, start_time: fmt(end - Duration::hours(2)), end_time: fmt(end), notes: "n".into() },
        )
        .unwrap();
        let data = export(&conn).unwrap();

        let other = open_in_memory();
        import(&other, &data).unwrap();
        assert_eq!(projects(&other, true).unwrap().len(), 2);
        let works: i64 = other.query_row("SELECT COUNT(*) FROM work WHERE active = 0", [], |r| r.get(0)).unwrap();
        assert_eq!(works, data.works.len() as i64);
    }
}
