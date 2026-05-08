use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use serde_json::Value;
use uuid::Uuid;

use crate::kernel::create_job_from_template;
use crate::templates::require_builtin_template;

#[derive(Debug, Clone)]
pub struct ScheduleRecord {
    pub id: String,
    pub template_id: String,
    pub template_version: i64,
    pub name: String,
    pub schedule_kind: String,
    pub interval_seconds: Option<i64>,
    pub run_at: Option<String>,
    pub enabled: bool,
    pub next_due_at: String,
    pub payload_json: String,
}

pub struct CreateScheduleInput {
    pub id: String,
    pub template_id: String,
    pub template_version: i64,
    pub name: String,
    pub schedule_kind: String,
    pub interval_seconds: Option<i64>,
    pub run_at: Option<String>,
    pub next_due_at: String,
    pub payload: Value,
}

pub fn create_schedule(connection: &Connection, input: CreateScheduleInput) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO schedules (
            id, template_id, template_version, name, schedule_kind, interval_seconds,
            run_at, enabled, next_due_at, payload_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, ?10, ?10)",
        params![
            input.id,
            input.template_id,
            input.template_version,
            input.name,
            input.schedule_kind,
            input.interval_seconds,
            input.run_at,
            input.next_due_at,
            input.payload.to_string(),
            now,
        ],
    )?;
    Ok(())
}

pub fn list_due_schedules(
    connection: &Connection,
    now: DateTime<Utc>,
) -> Result<Vec<ScheduleRecord>> {
    let mut statement = connection.prepare(
        "SELECT id, template_id, template_version, name, schedule_kind, interval_seconds,
                run_at, enabled, next_due_at, payload_json
         FROM schedules
         WHERE enabled = 1",
    )?;
    let schedule_rows = statement.query_map([], |row| {
        Ok(ScheduleRecord {
            id: row.get(0)?,
            template_id: row.get(1)?,
            template_version: row.get(2)?,
            name: row.get(3)?,
            schedule_kind: row.get(4)?,
            interval_seconds: row.get(5)?,
            run_at: row.get(6)?,
            enabled: row.get::<_, i64>(7)? == 1,
            next_due_at: row.get(8)?,
            payload_json: row.get(9)?,
        })
    })?;

    let mut due_schedules = Vec::new();
    for schedule_result in schedule_rows {
        let schedule = schedule_result?;
        let next_due_at = DateTime::parse_from_rfc3339(&schedule.next_due_at)?.with_timezone(&Utc);
        if next_due_at <= now {
            due_schedules.push(schedule);
        }
    }

    Ok(due_schedules)
}

pub fn create_job_for_due_schedule(
    connection: &mut Connection,
    schedule_id: &str,
    now: DateTime<Utc>,
) -> Result<String> {
    let schedule = load_schedule(connection, schedule_id)?;
    if !schedule.enabled {
        bail!("Schedule {schedule_id} is disabled");
    }

    let due_at = DateTime::parse_from_rfc3339(&schedule.next_due_at)?.with_timezone(&Utc);
    if due_at > now {
        bail!("Schedule {schedule_id} is not due yet");
    }

    let template = require_builtin_template(&schedule.template_id)?;
    let payload: Value = serde_json::from_str(&schedule.payload_json)?;
    let job_id = create_job_from_template(
        connection,
        &template,
        "scheduler",
        Some(&schedule.id),
        payload,
    )?;
    let run_id = format!("schedule_run_{}", Uuid::new_v4());
    let now_iso = now.to_rfc3339();

    connection.execute(
        "INSERT INTO scheduled_job_runs (id, schedule_id, job_id, due_at, claimed_at, completed_at, status)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, 'created')",
        params![run_id, schedule.id, job_id, schedule.next_due_at, now_iso],
    )?;

    let (next_due_at, enabled) = next_due_after_run(&schedule, now)?;
    connection.execute(
        "UPDATE schedules
         SET last_due_at = ?1, next_due_at = ?2, enabled = ?3, updated_at = ?4
         WHERE id = ?5",
        params![
            schedule.next_due_at,
            next_due_at,
            if enabled { 1 } else { 0 },
            now_iso,
            schedule.id
        ],
    )?;

    Ok(job_id)
}

fn load_schedule(connection: &Connection, schedule_id: &str) -> Result<ScheduleRecord> {
    let schedule = connection.query_row(
        "SELECT id, template_id, template_version, name, schedule_kind, interval_seconds,
                run_at, enabled, next_due_at, payload_json
         FROM schedules
         WHERE id = ?1",
        [schedule_id],
        |row| {
            Ok(ScheduleRecord {
                id: row.get(0)?,
                template_id: row.get(1)?,
                template_version: row.get(2)?,
                name: row.get(3)?,
                schedule_kind: row.get(4)?,
                interval_seconds: row.get(5)?,
                run_at: row.get(6)?,
                enabled: row.get::<_, i64>(7)? == 1,
                next_due_at: row.get(8)?,
                payload_json: row.get(9)?,
            })
        },
    )?;
    Ok(schedule)
}

fn next_due_after_run(schedule: &ScheduleRecord, now: DateTime<Utc>) -> Result<(String, bool)> {
    match schedule.schedule_kind.as_str() {
        "interval" => {
            let interval_seconds = schedule
                .interval_seconds
                .ok_or_else(|| anyhow::anyhow!("Interval schedule requires interval_seconds"))?;
            Ok((
                (now + Duration::seconds(interval_seconds)).to_rfc3339(),
                true,
            ))
        }
        "one_shot" => Ok((schedule.next_due_at.clone(), false)),
        other => bail!("Unsupported schedule kind: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;
    use crate::templates::seed_builtin_templates;
    use rusqlite::Connection;
    use serde_json::json;

    #[test]
    fn finds_due_schedules() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_templates(&connection).unwrap();
        create_schedule(
            &connection,
            CreateScheduleInput {
                id: "schedule_1".to_string(),
                template_id: "system.health.check".to_string(),
                template_version: 1,
                name: "Health check".to_string(),
                schedule_kind: "interval".to_string(),
                interval_seconds: Some(30),
                run_at: None,
                next_due_at: "2026-05-07T10:00:00Z".to_string(),
                payload: json!({}),
            },
        )
        .unwrap();

        let now = DateTime::parse_from_rfc3339("2026-05-07T10:00:01Z")
            .unwrap()
            .with_timezone(&Utc);
        let due = list_due_schedules(&connection, now).unwrap();

        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, "schedule_1");
    }

    #[test]
    fn due_schedule_creates_job_and_advances_interval() {
        let mut connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_templates(&connection).unwrap();
        create_schedule(
            &connection,
            CreateScheduleInput {
                id: "schedule_2".to_string(),
                template_id: "system.health.check".to_string(),
                template_version: 1,
                name: "Health check".to_string(),
                schedule_kind: "interval".to_string(),
                interval_seconds: Some(60),
                run_at: None,
                next_due_at: "2026-05-07T10:00:00Z".to_string(),
                payload: json!({ "reason": "test" }),
            },
        )
        .unwrap();

        let now = DateTime::parse_from_rfc3339("2026-05-07T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let job_id = create_job_for_due_schedule(&mut connection, "schedule_2", now).unwrap();

        let run_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM scheduled_job_runs WHERE job_id = ?1",
                [&job_id],
                |row| row.get(0),
            )
            .unwrap();
        let next_due_at: String = connection
            .query_row(
                "SELECT next_due_at FROM schedules WHERE id = 'schedule_2'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(run_count, 1);
        assert_eq!(next_due_at, "2026-05-07T10:01:00+00:00");
    }
}
