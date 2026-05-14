use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use cron::Schedule;
use rusqlite::{params, Connection};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

use crate::briefs::SYSTEM_BRIEF_TEMPLATE_ID;
use crate::kernel::create_job_from_template;
use crate::templates::require_builtin_template_version;

pub const SYSTEM_BRIEF_SCHEDULE_ID: &str = "schedule_system_brief_generate";

#[derive(Debug, Clone)]
pub struct ScheduleRecord {
    pub id: String,
    pub template_id: String,
    pub template_version: i64,
    pub name: String,
    pub schedule_kind: String,
    pub cron_expression: Option<String>,
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
    pub cron_expression: Option<String>,
    pub interval_seconds: Option<i64>,
    pub run_at: Option<String>,
    pub next_due_at: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerOperationsResponse {
    pub generated_at: String,
    pub schedules: Vec<SchedulerOperationsSchedule>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerOperationsSchedule {
    pub id: String,
    pub name: String,
    pub template_id: String,
    pub template_version: i64,
    pub schedule_kind: String,
    pub enabled: bool,
    pub timezone: String,
    pub cron_expression: Option<String>,
    pub interval_seconds: Option<i64>,
    pub run_at: Option<String>,
    pub last_due_at: Option<String>,
    pub next_due_at: String,
    pub last_run: Option<SchedulerOperationsRun>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerOperationsRun {
    pub id: String,
    pub job_id: Option<String>,
    pub due_at: String,
    pub claimed_at: Option<String>,
    pub completed_at: Option<String>,
    pub status: String,
    pub has_error: bool,
}

pub fn create_schedule(connection: &Connection, input: CreateScheduleInput) -> Result<()> {
    validate_schedule_input(&input)?;
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO schedules (
            id, template_id, template_version, name, schedule_kind, cron_expression, interval_seconds,
            run_at, enabled, next_due_at, payload_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10, ?11, ?11)",
        params![
            input.id,
            input.template_id,
            input.template_version,
            input.name,
            input.schedule_kind,
            input.cron_expression,
            input.interval_seconds,
            input.run_at,
            input.next_due_at,
            input.payload.to_string(),
            now,
        ],
    )?;
    Ok(())
}

pub fn read_scheduler_operations(db_path: &Path) -> Result<SchedulerOperationsResponse> {
    let connection = Connection::open(db_path)?;
    read_scheduler_operations_from_connection(&connection, Utc::now())
}

pub fn read_scheduler_operations_from_connection(
    connection: &Connection,
    now: DateTime<Utc>,
) -> Result<SchedulerOperationsResponse> {
    let mut statement = connection.prepare(
        "SELECT id, template_id, template_version, name, schedule_kind, cron_expression,
                interval_seconds, run_at, timezone, enabled, last_due_at, next_due_at
         FROM schedules
         ORDER BY enabled DESC, next_due_at ASC, id ASC",
    )?;
    let schedule_rows = statement.query_map([], |row| {
        Ok((
            ScheduleRecord {
                id: row.get(0)?,
                template_id: row.get(1)?,
                template_version: row.get(2)?,
                name: row.get(3)?,
                schedule_kind: row.get(4)?,
                cron_expression: row.get(5)?,
                interval_seconds: row.get(6)?,
                run_at: row.get(7)?,
                enabled: row.get::<_, i64>(9)? == 1,
                next_due_at: row.get(11)?,
                payload_json: "{}".to_string(),
            },
            row.get::<_, String>(8)?,
            row.get::<_, Option<String>>(10)?,
        ))
    })?;

    let mut schedules = Vec::new();
    for schedule_result in schedule_rows {
        let (schedule, timezone, last_due_at) = schedule_result?;
        let last_run = latest_schedule_run(connection, &schedule.id)?;
        let limitations = schedule_limitations(&schedule, last_run.as_ref(), now);
        schedules.push(SchedulerOperationsSchedule {
            id: schedule.id,
            name: schedule.name,
            template_id: schedule.template_id,
            template_version: schedule.template_version,
            schedule_kind: schedule.schedule_kind,
            enabled: schedule.enabled,
            timezone,
            cron_expression: schedule.cron_expression,
            interval_seconds: schedule.interval_seconds,
            run_at: schedule.run_at,
            last_due_at,
            next_due_at: schedule.next_due_at,
            last_run,
            limitations,
        });
    }

    Ok(SchedulerOperationsResponse {
        generated_at: now.to_rfc3339(),
        schedules,
    })
}

fn latest_schedule_run(
    connection: &Connection,
    schedule_id: &str,
) -> Result<Option<SchedulerOperationsRun>> {
    let mut statement = connection.prepare(
        "SELECT id, job_id, due_at, claimed_at, completed_at, status, error_message
         FROM scheduled_job_runs
         WHERE schedule_id = ?1
         ORDER BY claimed_at DESC, due_at DESC, id DESC
         LIMIT 1",
    )?;
    let mut rows = statement.query([schedule_id])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };

    Ok(Some(SchedulerOperationsRun {
        id: row.get(0)?,
        job_id: row.get(1)?,
        due_at: row.get(2)?,
        claimed_at: row.get(3)?,
        completed_at: row.get(4)?,
        status: row.get(5)?,
        has_error: row.get::<_, Option<String>>(6)?.is_some(),
    }))
}

fn schedule_limitations(
    schedule: &ScheduleRecord,
    last_run: Option<&SchedulerOperationsRun>,
    now: DateTime<Utc>,
) -> Vec<String> {
    let mut limitations = Vec::new();
    if !schedule.enabled {
        limitations.push("disabled schedules are shown for inspection only".to_string());
    }
    if DateTime::parse_from_rfc3339(&schedule.next_due_at)
        .map(|due_at| due_at.with_timezone(&Utc) <= now)
        .unwrap_or(false)
    {
        limitations
            .push("schedule is due or overdue; this read path does not claim work".to_string());
    }
    if last_run
        .map(|run| run.status == "failed" || run.has_error)
        .unwrap_or(false)
    {
        limitations.push(
            "last run recorded a failure; inspect protected daemon logs for details".to_string(),
        );
    }
    limitations
}

pub fn ensure_default_system_brief_schedule(connection: &Connection) -> Result<()> {
    let existing_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM schedules WHERE id = ?1",
        [SYSTEM_BRIEF_SCHEDULE_ID],
        |row| row.get(0),
    )?;
    if existing_count > 0 {
        return Ok(());
    }

    create_schedule(
        connection,
        CreateScheduleInput {
            id: SYSTEM_BRIEF_SCHEDULE_ID.to_string(),
            template_id: SYSTEM_BRIEF_TEMPLATE_ID.to_string(),
            template_version: 1,
            name: "Generate System Brief".to_string(),
            schedule_kind: "interval".to_string(),
            cron_expression: None,
            interval_seconds: Some(3600),
            run_at: None,
            next_due_at: Utc::now().to_rfc3339(),
            payload: serde_json::json!({ "sectionKey": "system" }),
        },
    )
}

pub fn list_due_schedules(
    connection: &Connection,
    now: DateTime<Utc>,
) -> Result<Vec<ScheduleRecord>> {
    let mut statement = connection.prepare(
        "SELECT id, template_id, template_version, name, schedule_kind, cron_expression, interval_seconds,
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
            cron_expression: row.get(5)?,
            interval_seconds: row.get(6)?,
            run_at: row.get(7)?,
            enabled: row.get::<_, i64>(8)? == 1,
            next_due_at: row.get(9)?,
            payload_json: row.get(10)?,
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

    let template =
        require_builtin_template_version(&schedule.template_id, schedule.template_version)?;
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
        "SELECT id, template_id, template_version, name, schedule_kind, cron_expression, interval_seconds,
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
                cron_expression: row.get(5)?,
                interval_seconds: row.get(6)?,
                run_at: row.get(7)?,
                enabled: row.get::<_, i64>(8)? == 1,
                next_due_at: row.get(9)?,
                payload_json: row.get(10)?,
            })
        },
    )?;
    Ok(schedule)
}

fn validate_schedule_input(input: &CreateScheduleInput) -> Result<()> {
    match input.schedule_kind.as_str() {
        "interval" => {
            let interval_seconds = input
                .interval_seconds
                .ok_or_else(|| anyhow::anyhow!("Interval schedule requires interval_seconds"))?;
            if interval_seconds <= 0 {
                bail!("Interval schedule requires a positive interval_seconds");
            }
            if input.cron_expression.is_some() {
                bail!("Interval schedule cannot include cron_expression");
            }
        }
        "one_shot" => {
            if input.run_at.is_none() {
                bail!("One-shot schedule requires run_at");
            }
            if input.cron_expression.is_some() {
                bail!("One-shot schedule cannot include cron_expression");
            }
        }
        "cron" => {
            let expression = input
                .cron_expression
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Cron schedule requires cron_expression"))?;
            parse_cron_schedule(expression)?;
            if input.interval_seconds.is_some() {
                bail!("Cron schedule cannot include interval_seconds");
            }
        }
        other => bail!("Unsupported schedule kind: {other}"),
    }

    DateTime::parse_from_rfc3339(&input.next_due_at)
        .with_context(|| format!("Invalid next_due_at for schedule {}", input.id))?;
    if let Some(run_at) = &input.run_at {
        DateTime::parse_from_rfc3339(run_at)
            .with_context(|| format!("Invalid run_at for schedule {}", input.id))?;
    }

    Ok(())
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
        "cron" => {
            let expression = schedule
                .cron_expression
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Cron schedule requires cron_expression"))?;
            let cron_schedule = parse_cron_schedule(expression)?;
            match cron_schedule.after(&now).next() {
                Some(next_due_at) => Ok((next_due_at.to_rfc3339(), true)),
                None => Ok((schedule.next_due_at.clone(), false)),
            }
        }
        other => bail!("Unsupported schedule kind: {other}"),
    }
}

fn parse_cron_schedule(expression: &str) -> Result<Schedule> {
    Schedule::from_str(expression).with_context(|| format!("Invalid cron expression: {expression}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::schema::init_schema;
    use crate::templates::seed_builtin_templates;
    use rusqlite::Connection;
    use serde_json::json;

    #[test]
    fn finds_due_schedules() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        seed_builtin_templates(&connection).unwrap();
        create_schedule(
            &connection,
            CreateScheduleInput {
                id: "schedule_1".to_string(),
                template_id: "system.health.check".to_string(),
                template_version: 1,
                name: "Health check".to_string(),
                schedule_kind: "interval".to_string(),
                cron_expression: None,
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
        seed_builtin_capabilities(&connection).unwrap();
        seed_builtin_templates(&connection).unwrap();
        create_schedule(
            &connection,
            CreateScheduleInput {
                id: "schedule_2".to_string(),
                template_id: "system.health.check".to_string(),
                template_version: 1,
                name: "Health check".to_string(),
                schedule_kind: "interval".to_string(),
                cron_expression: None,
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

    #[test]
    fn rejects_invalid_cron_expression_at_create_time() {
        let connection = test_connection();
        let error = create_schedule(
            &connection,
            CreateScheduleInput {
                id: "schedule_bad_cron".to_string(),
                template_id: "system.health.check".to_string(),
                template_version: 1,
                name: "Bad cron".to_string(),
                schedule_kind: "cron".to_string(),
                cron_expression: Some("not a cron expression".to_string()),
                interval_seconds: None,
                run_at: None,
                next_due_at: "2026-05-07T10:00:00Z".to_string(),
                payload: json!({}),
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("Invalid cron expression"));
    }

    #[test]
    fn rejects_unknown_template_before_schedule_persistence() {
        let connection = test_connection();
        let error = create_schedule(
            &connection,
            CreateScheduleInput {
                id: "schedule_unknown_template".to_string(),
                template_id: "missing.template".to_string(),
                template_version: 1,
                name: "Unknown template".to_string(),
                schedule_kind: "cron".to_string(),
                cron_expression: Some("0 0 * * * * *".to_string()),
                interval_seconds: None,
                run_at: None,
                next_due_at: "2026-05-07T10:00:00Z".to_string(),
                payload: json!({}),
            },
        )
        .unwrap_err();
        let schedule_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schedules WHERE id = 'schedule_unknown_template'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(error.to_string().contains("FOREIGN KEY constraint failed"));
        assert_eq!(schedule_count, 0);
    }

    #[test]
    fn malformed_payload_rejects_before_job_or_run_creation() {
        let mut connection = test_connection();
        create_cron_schedule(
            &connection,
            "schedule_malformed_payload",
            "0 0 * * * * *",
            "2026-05-07T10:00:00Z",
        );
        connection
            .execute(
                "UPDATE schedules SET payload_json = '{not json' WHERE id = 'schedule_malformed_payload'",
                [],
            )
            .unwrap();

        let error = create_job_for_due_schedule(
            &mut connection,
            "schedule_malformed_payload",
            utc("2026-05-07T10:00:00Z"),
        )
        .unwrap_err();
        let job_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM jobs", [], |row| row.get(0))
            .unwrap();
        let run_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM scheduled_job_runs", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert!(error.to_string().contains("key must be a string"));
        assert_eq!(job_count, 0);
        assert_eq!(run_count, 0);
    }

    #[test]
    fn cron_due_schedule_creates_one_job_and_advances_to_next_fire() {
        let mut connection = test_connection();
        create_cron_schedule(
            &connection,
            "schedule_hourly",
            "0 0 * * * * *",
            "2026-05-07T10:00:00Z",
        );

        let now = utc("2026-05-07T10:00:00Z");
        let job_id = create_job_for_due_schedule(&mut connection, "schedule_hourly", now).unwrap();
        let duplicate = create_job_for_due_schedule(&mut connection, "schedule_hourly", now)
            .unwrap_err()
            .to_string();
        let run_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM scheduled_job_runs WHERE schedule_id = 'schedule_hourly'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let next_due_at: String = connection
            .query_row(
                "SELECT next_due_at FROM schedules WHERE id = 'schedule_hourly'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let job_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM jobs WHERE id = ?1",
                [&job_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(job_count, 1);
        assert_eq!(run_count, 1);
        assert_eq!(next_due_at, "2026-05-07T11:00:00+00:00");
        assert!(duplicate.contains("not due yet"));
    }

    #[test]
    fn cron_next_due_supports_daily_weekly_month_boundary_and_missed_windows() {
        let daily = schedule_record("0 30 2 * * * *", "2026-05-07T02:30:00Z");
        let weekly = schedule_record("0 15 9 * * Mon *", "2026-05-04T09:15:00Z");
        let month_boundary = schedule_record("0 0 0 1 * * *", "2026-02-01T00:00:00Z");

        assert_eq!(
            next_due_after_run(&daily, utc("2026-05-07T03:00:00Z"))
                .unwrap()
                .0,
            "2026-05-08T02:30:00+00:00"
        );
        assert_eq!(
            next_due_after_run(&weekly, utc("2026-05-04T09:15:00Z"))
                .unwrap()
                .0,
            "2026-05-11T09:15:00+00:00"
        );
        assert_eq!(
            next_due_after_run(&month_boundary, utc("2026-02-01T00:00:00Z"))
                .unwrap()
                .0,
            "2026-03-01T00:00:00+00:00"
        );
    }

    #[test]
    fn cron_no_future_run_disables_schedule_without_creating_external_work() {
        let mut connection = test_connection();
        create_cron_schedule(
            &connection,
            "schedule_expired",
            "0 0 0 1 1 * 2026",
            "2026-01-01T00:00:00Z",
        );

        let job_id = create_job_for_due_schedule(
            &mut connection,
            "schedule_expired",
            utc("2026-01-01T00:00:00Z"),
        )
        .unwrap();
        let (enabled, next_due_at): (i64, String) = connection
            .query_row(
                "SELECT enabled, next_due_at FROM schedules WHERE id = 'schedule_expired'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert!(job_id.starts_with("job_"));
        assert_eq!(enabled, 0);
        assert_eq!(next_due_at, "2026-01-01T00:00:00Z");
    }

    #[test]
    fn one_shot_schedule_is_disabled_after_due_run() {
        let mut connection = test_connection();
        create_schedule(
            &connection,
            CreateScheduleInput {
                id: "schedule_once".to_string(),
                template_id: "system.health.check".to_string(),
                template_version: 1,
                name: "Once".to_string(),
                schedule_kind: "one_shot".to_string(),
                cron_expression: None,
                interval_seconds: None,
                run_at: Some("2026-05-07T10:00:00Z".to_string()),
                next_due_at: "2026-05-07T10:00:00Z".to_string(),
                payload: json!({}),
            },
        )
        .unwrap();

        create_job_for_due_schedule(
            &mut connection,
            "schedule_once",
            utc("2026-05-07T10:00:00Z"),
        )
        .unwrap();
        let enabled: i64 = connection
            .query_row(
                "SELECT enabled FROM schedules WHERE id = 'schedule_once'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(enabled, 0);
    }

    #[test]
    fn scheduler_operations_shape_omits_payloads_and_raw_failure_details() {
        let mut connection = test_connection();
        create_schedule(
            &connection,
            CreateScheduleInput {
                id: "schedule_private_payload".to_string(),
                template_id: "system.health.check".to_string(),
                template_version: 1,
                name: "Private payload schedule".to_string(),
                schedule_kind: "cron".to_string(),
                cron_expression: Some("0 0 * * * * *".to_string()),
                interval_seconds: None,
                run_at: None,
                next_due_at: "2026-05-07T10:00:00Z".to_string(),
                payload: json!({ "privateToken": "sk-private", "ownerNote": "do not expose" }),
            },
        )
        .unwrap();
        let job_id = create_job_for_due_schedule(
            &mut connection,
            "schedule_private_payload",
            utc("2026-05-07T10:00:00Z"),
        )
        .unwrap();
        connection
            .execute(
                "UPDATE scheduled_job_runs
                 SET completed_at = '2026-05-07T10:00:01Z',
                     status = 'failed',
                     error_message = 'provider secret sk-private leaked'
                 WHERE job_id = ?1",
                [&job_id],
            )
            .unwrap();

        let operations =
            read_scheduler_operations_from_connection(&connection, utc("2026-05-07T10:30:00Z"))
                .unwrap();
        let schedule = operations
            .schedules
            .iter()
            .find(|schedule| schedule.id == "schedule_private_payload")
            .unwrap();
        let serialized = serde_json::to_string(&operations).unwrap();

        assert_eq!(schedule.schedule_kind, "cron");
        assert_eq!(schedule.cron_expression.as_deref(), Some("0 0 * * * * *"));
        assert_eq!(
            schedule.last_run.as_ref().unwrap().job_id.as_deref(),
            Some(job_id.as_str())
        );
        assert!(schedule.last_run.as_ref().unwrap().has_error);
        assert!(schedule.limitations.contains(
            &"last run recorded a failure; inspect protected daemon logs for details".to_string()
        ));
        assert!(!serialized.contains("sk-private"));
        assert!(!serialized.contains("ownerNote"));
        assert!(!serialized.contains("provider secret"));
    }

    #[test]
    fn scheduler_operations_handles_empty_and_disabled_schedules() {
        let connection = test_connection();
        create_schedule(
            &connection,
            CreateScheduleInput {
                id: "schedule_disabled".to_string(),
                template_id: "system.health.check".to_string(),
                template_version: 1,
                name: "Disabled schedule".to_string(),
                schedule_kind: "one_shot".to_string(),
                cron_expression: None,
                interval_seconds: None,
                run_at: Some("2026-05-07T10:00:00Z".to_string()),
                next_due_at: "2026-05-07T10:00:00Z".to_string(),
                payload: json!({}),
            },
        )
        .unwrap();
        connection
            .execute(
                "UPDATE schedules SET enabled = 0 WHERE id = 'schedule_disabled'",
                [],
            )
            .unwrap();

        let operations =
            read_scheduler_operations_from_connection(&connection, utc("2026-05-07T09:00:00Z"))
                .unwrap();
        let schedule = operations
            .schedules
            .iter()
            .find(|schedule| schedule.id == "schedule_disabled")
            .unwrap();

        assert!(!schedule.enabled);
        assert_eq!(schedule.schedule_kind, "one_shot");
        assert!(schedule.last_run.is_none());
        assert!(schedule
            .limitations
            .contains(&"disabled schedules are shown for inspection only".to_string()));

        connection.execute("DELETE FROM schedules", []).unwrap();
        let empty =
            read_scheduler_operations_from_connection(&connection, utc("2026-05-07T09:00:00Z"))
                .unwrap();
        assert!(empty.schedules.is_empty());
    }

    fn test_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        seed_builtin_templates(&connection).unwrap();
        connection
    }

    fn create_cron_schedule(
        connection: &Connection,
        id: &str,
        expression: &str,
        next_due_at: &str,
    ) {
        create_schedule(
            connection,
            CreateScheduleInput {
                id: id.to_string(),
                template_id: "system.health.check".to_string(),
                template_version: 1,
                name: id.to_string(),
                schedule_kind: "cron".to_string(),
                cron_expression: Some(expression.to_string()),
                interval_seconds: None,
                run_at: None,
                next_due_at: next_due_at.to_string(),
                payload: json!({}),
            },
        )
        .unwrap();
    }

    fn schedule_record(expression: &str, next_due_at: &str) -> ScheduleRecord {
        ScheduleRecord {
            id: "schedule_test".to_string(),
            template_id: "system.health.check".to_string(),
            template_version: 1,
            name: "Test schedule".to_string(),
            schedule_kind: "cron".to_string(),
            cron_expression: Some(expression.to_string()),
            interval_seconds: None,
            run_at: None,
            enabled: true,
            next_due_at: next_due_at.to_string(),
            payload_json: "{}".to_string(),
        }
    }

    fn utc(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }
}
