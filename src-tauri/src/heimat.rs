//! Heimat integration: a small client for the Heimat REST API (`/heimat-core/api/v1`) and the
//! comparison of a day's InTime work with the times already booked in Heimat.

use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{NaiveDate, NaiveDateTime};
use reqwest::header::ACCEPT;
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;

use crate::error::{Error, Result};
use crate::models::{HeimatConfig, HeimatMapping, HeimatTask, HeimatTime, Project, SyncRow, SyncStatus, Work};
use crate::repo;

pub struct HeimatClient {
    http: Client,
    base: String,
    token: String,
}

impl HeimatClient {
    pub fn new(config: &HeimatConfig) -> Result<Self> {
        if !config.is_configured() {
            return Err(Error::msg(
                "Heimat ist nicht eingerichtet. Trage URL und Zugangstoken unter Einstellungen → Heimat API ein.",
            ));
        }
        let http = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| Error::msg(format!("HTTP-Client konnte nicht erstellt werden: {e}")))?;
        let base = format!("{}/heimat-core/api/v1", config.url.trim().trim_end_matches('/'));
        let token = config.token.trim().trim_start_matches("Bearer ").to_string();
        Ok(HeimatClient { http, base, token })
    }

    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.http
            .request(method, format!("{}{path}", self.base))
            .bearer_auth(&self.token)
            .header("X-Client-Identifier", "InTime")
            .header(ACCEPT, "application/json")
    }

    async fn send(&self, request: RequestBuilder) -> Result<Response> {
        let response = request.send().await.map_err(|e| {
            Error::msg(format!("Heimat ist nicht erreichbar. Stimmt die URL und bist du im richtigen Netzwerk? ({e})"))
        })?;
        match response.status() {
            status if status.is_success() => Ok(response),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(Error::msg(
                "Heimat hat den Zugriff abgelehnt. Ist das Zugangstoken gültig und nicht abgelaufen?",
            )),
            status => {
                let body: String = response.text().await.unwrap_or_default().chars().take(300).collect();
                Err(Error::msg(format!("Heimat meldet einen Fehler ({status}). {body}")))
            }
        }
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str, date: Option<NaiveDate>) -> Result<T> {
        let mut request = self.request(Method::GET, path);
        if let Some(date) = date {
            request = request.query(&[("date", date.format("%Y-%m-%d").to_string())]);
        }
        self.send(request)
            .await?
            .json()
            .await
            .map_err(|_| Error::msg("Unerwartete Antwort von Heimat. Stimmt die URL?"))
    }

    /// `GET /my/tasks` – tasks the user can book on (optionally for one day).
    pub async fn tasks(&self, date: Option<NaiveDate>) -> Result<Vec<HeimatTask>> {
        self.get_json("/my/tasks", date).await
    }

    /// `GET /my/times` – times the user already booked (optionally for one day).
    pub async fn times(&self, date: Option<NaiveDate>) -> Result<Vec<HeimatTime>> {
        self.get_json("/my/times", date).await
    }

    /// `POST /my/times`
    pub async fn add_time(&self, time: &HeimatTime) -> Result<()> {
        self.send(self.request(Method::POST, "/my/times").json(time)).await?;
        Ok(())
    }

    /// `DELETE /my/times/{id}`
    pub async fn delete_time(&self, id: i64) -> Result<()> {
        self.send(self.request(Method::DELETE, &format!("/my/times/{id}"))).await?;
        Ok(())
    }
}

/// Tasks InTime can book on: each task once, without tasks that need start and end
/// times (InTime only books durations), sorted by project and task name.
pub fn bookable_tasks(mut tasks: Vec<HeimatTask>) -> Vec<HeimatTask> {
    let mut seen = std::collections::HashSet::new();
    tasks.retain(|t| !t.is_start_and_end_time_required && seen.insert(t.id));
    tasks.sort_by(|a, b| {
        (a.task_holder_name.to_lowercase(), a.name.to_lowercase())
            .cmp(&(b.task_holder_name.to_lowercase(), b.name.to_lowercase()))
    });
    tasks
}

fn join_notes<'a>(notes: impl IntoIterator<Item = &'a str>) -> String {
    let mut distinct: Vec<&str> = Vec::new();
    for note in notes.into_iter().map(str::trim).filter(|n| !n.is_empty()) {
        if !distinct.contains(&note) {
            distinct.push(note);
        }
    }
    distinct.join("; ")
}

/// Compares a day's InTime work with the times booked in Heimat, one row per Heimat task.
/// Only projects counted as work time are considered. The running work counts up to `now`.
pub fn day_overview(
    works: &[Work],
    projects: &[Project],
    mappings: &[HeimatMapping],
    tasks: &[HeimatTask],
    times: &[HeimatTime],
    now: NaiveDateTime,
) -> Result<Vec<SyncRow>> {
    let project_of = |id: i64| projects.iter().find(|p| p.id == id);

    // Seconds and notes per work project, in project order.
    let mut per_project: BTreeMap<(i64, i64), (i64, Vec<&str>)> = BTreeMap::new();
    for work in works {
        let Some(project) = project_of(work.project_id).filter(|p| p.is_work) else { continue };
        let start = repo::parse(&work.start_time)?;
        let end = if work.active { now.max(start) } else { repo::parse(&work.end_time)? };
        let entry = per_project.entry((project.sort_index, project.id)).or_default();
        entry.0 += (end - start).num_seconds();
        entry.1.push(&work.notes);
    }

    let row = |task_id: Option<i64>, status: SyncStatus| SyncRow {
        task_id,
        task_name: String::new(),
        task_holder_name: String::new(),
        booking_hint: String::new(),
        is_note_optional: true,
        status,
        project_ids: Vec::new(),
        intime_seconds: 0,
        intime_notes: String::new(),
        heimat_seconds: 0,
        heimat_notes: String::new(),
        existing_time_ids: Vec::new(),
        suggest_sync: false,
    };

    let mut rows: Vec<SyncRow> = Vec::new();
    let mut notes_by_row: Vec<Vec<&str>> = Vec::new();
    for (&(_, project_id), (seconds, notes)) in &per_project {
        let mapping = mappings.iter().find(|m| m.project_id == project_id);
        let existing = mapping.and_then(|m| rows.iter().position(|r| r.task_id == Some(m.task_id)));
        let index = match (mapping, existing) {
            (Some(_), Some(i)) => i,
            (Some(m), None) => {
                let mut r = row(Some(m.task_id), SyncStatus::Ready);
                r.task_name = m.task_name.clone();
                r.task_holder_name = m.task_holder_name.clone();
                rows.push(r);
                notes_by_row.push(Vec::new());
                rows.len() - 1
            }
            (None, _) => {
                rows.push(row(None, SyncStatus::Unmapped));
                notes_by_row.push(Vec::new());
                rows.len() - 1
            }
        };
        rows[index].project_ids.push(project_id);
        rows[index].intime_seconds += seconds;
        notes_by_row[index].extend(notes);
    }
    for (r, notes) in rows.iter_mut().zip(notes_by_row) {
        r.intime_notes = join_notes(notes);
    }

    // Times booked in Heimat on tasks nobody worked on in InTime today.
    for time in times {
        if !rows.iter().any(|r| r.task_id == Some(time.task_id)) {
            let mut r = row(Some(time.task_id), SyncStatus::HeimatOnly);
            r.project_ids = mappings.iter().filter(|m| m.task_id == time.task_id).map(|m| m.project_id).collect();
            if let Some(m) = mappings.iter().find(|m| m.task_id == time.task_id) {
                r.task_name = m.task_name.clone();
                r.task_holder_name = m.task_holder_name.clone();
            }
            rows.push(r);
        }
    }

    for r in &mut rows {
        let Some(task_id) = r.task_id else { continue };
        let booked: Vec<&HeimatTime> = times.iter().filter(|t| t.task_id == task_id).collect();
        r.heimat_seconds = booked.iter().map(|t| t.duration_in_minutes * 60).sum();
        r.heimat_notes = join_notes(booked.iter().filter_map(|t| t.note.as_deref()));
        r.existing_time_ids = booked.iter().map(|t| t.id).collect();
        match tasks.iter().find(|t| t.id == task_id) {
            Some(task) => {
                r.task_name = task.name.clone();
                r.task_holder_name = task.task_holder_name.clone();
                r.booking_hint = task.booking_hint.clone().unwrap_or_default();
                r.is_note_optional = task.is_note_optional;
            }
            None if r.status == SyncStatus::Ready => r.status = SyncStatus::Unavailable,
            None => {}
        }
        if r.task_name.is_empty() {
            r.task_name = format!("Unbekannte Aufgabe #{task_id}");
        }
        r.suggest_sync = r.status == SyncStatus::Ready
            && r.intime_seconds > 0
            && (r.heimat_seconds == 0 || (r.intime_seconds - r.heimat_seconds).abs() >= 15 * 60);
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(id: i64, is_work: bool) -> Project {
        Project {
            id,
            name: format!("P{id}"),
            description: String::new(),
            color: "#000".into(),
            is_work,
            is_default: false,
            is_enabled: true,
            sort_index: id,
        }
    }

    fn work(project_id: i64, start: &str, end: &str, notes: &str) -> Work {
        Work {
            id: 0,
            project_id,
            start_time: format!("2026-10-27T{start}:00"),
            end_time: format!("2026-10-27T{end}:00"),
            notes: notes.into(),
            active: false,
        }
    }

    fn mapping(project_id: i64, task_id: i64) -> HeimatMapping {
        HeimatMapping { project_id, task_id, task_name: format!("T{task_id}"), task_holder_name: "H".into() }
    }

    fn task(id: i64) -> HeimatTask {
        HeimatTask { id, name: format!("Task {id}"), task_holder_name: "Holder".into(), ..Default::default() }
    }

    fn time(id: i64, task_id: i64, minutes: i64, note: &str) -> HeimatTime {
        HeimatTime {
            id,
            task_id,
            date: "2026-10-27".into(),
            duration_in_minutes: minutes,
            note: Some(note.into()),
            ..Default::default()
        }
    }

    fn now() -> NaiveDateTime {
        repo::parse("2026-10-27T18:00:00").unwrap()
    }

    #[test]
    fn projects_on_the_same_task_are_combined_and_non_work_is_ignored() {
        let projects = [project(1, true), project(2, true), project(3, false)];
        let works = [
            work(1, "08:00", "09:00", "a"),
            work(2, "09:00", "09:30", "b"),
            work(1, "10:00", "10:30", "a"),
            work(3, "12:00", "13:00", "Pause"),
        ];
        let rows =
            day_overview(&works, &projects, &[mapping(1, 10), mapping(2, 10)], &[task(10)], &[], now()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, SyncStatus::Ready);
        assert_eq!(rows[0].project_ids, vec![1, 2]);
        assert_eq!(rows[0].intime_seconds, 2 * 3600);
        assert_eq!(rows[0].intime_notes, "a; b");
        assert_eq!(rows[0].task_name, "Task 10");
        assert!(rows[0].suggest_sync);
    }

    #[test]
    fn statuses_for_unmapped_unavailable_and_heimat_only() {
        let projects = [project(1, true), project(2, true)];
        let works = [work(1, "08:00", "09:00", ""), work(2, "09:00", "10:00", "")];
        let rows = day_overview(
            &works,
            &projects,
            &[mapping(2, 20)],
            &[task(30)],
            &[time(7, 30, 60, "schon gebucht")],
            now(),
        )
        .unwrap();
        let status: Vec<_> = rows.iter().map(|r| (r.task_id, r.status.clone())).collect();
        assert_eq!(
            status,
            vec![
                (None, SyncStatus::Unmapped),
                (Some(20), SyncStatus::Unavailable),
                (Some(30), SyncStatus::HeimatOnly)
            ]
        );
        assert!(rows.iter().all(|r| !r.suggest_sync));
        assert_eq!(rows[2].heimat_seconds, 3600);
        assert_eq!(rows[2].existing_time_ids, vec![7]);
    }

    #[test]
    fn already_booked_times_are_only_suggested_when_off_by_15_minutes() {
        let projects = [project(1, true)];
        let works = [work(1, "08:00", "09:10", "")];
        let close = day_overview(&works, &projects, &[mapping(1, 10)], &[task(10)], &[time(1, 10, 60, "")], now());
        assert!(!close.unwrap()[0].suggest_sync);
        let off = day_overview(&works, &projects, &[mapping(1, 10)], &[task(10)], &[time(1, 10, 45, "")], now());
        assert!(off.unwrap()[0].suggest_sync);
    }

    #[test]
    fn running_work_counts_until_now() {
        let mut running = work(1, "17:00", "17:00", "");
        running.active = true;
        let rows = day_overview(&[running], &[project(1, true)], &[], &[], &[], now()).unwrap();
        assert_eq!(rows[0].intime_seconds, 3600);
    }

    #[test]
    fn bookable_tasks_are_unique_sorted_and_duration_only() {
        let mut needs_times = task(3);
        needs_times.is_start_and_end_time_required = true;
        let mut b = task(2);
        b.task_holder_name = "Alpha".into();
        let tasks = bookable_tasks(vec![task(1), b, task(1), needs_times]);
        assert_eq!(tasks.iter().map(|t| t.id).collect::<Vec<_>>(), vec![2, 1]);
    }
}
