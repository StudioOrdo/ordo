use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

use crate::health::{build_health_report, build_readiness_report};
use crate::kernel::{append_job_event, create_job_from_template};
use crate::scheduler::{create_job_for_due_schedule, list_due_schedules};
use crate::templates::{require_builtin_template, require_builtin_template_version};

pub const SYSTEM_BRIEF_TEMPLATE_ID: &str = "brief.system.generate";
const SYSTEM_SECTION_KEY: &str = "system";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefEvidence {
    pub label: String,
    pub value: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefProcessProvenance {
    pub job_id: String,
    pub template_id: String,
    pub template_version: i64,
    pub origin: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefArtifact {
    pub id: String,
    pub section_key: String,
    pub job_id: Option<String>,
    pub process: Option<BriefProcessProvenance>,
    pub version: i64,
    pub title: String,
    pub summary: Vec<String>,
    pub body_markdown: String,
    pub evidence: Vec<BriefEvidence>,
    pub limitations: Vec<String>,
    pub visibility: String,
    pub created_at: String,
    pub valid_until: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestBriefResponse {
    pub brief: Option<BriefArtifact>,
}

pub fn latest_system_brief(db_path: &Path) -> Result<Option<BriefArtifact>> {
    let connection = Connection::open(db_path)?;
    load_latest_system_brief(&connection)
}

pub fn generate_system_brief(
    db_path: &Path,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<BriefArtifact> {
    let mut connection = Connection::open(db_path)?;
    generate_system_brief_with_connection(&mut connection, db_path, origin, actor_id)
}

pub fn run_due_system_brief_schedules(db_path: &Path) -> Result<Vec<BriefArtifact>> {
    let mut connection = Connection::open(db_path)?;
    let now = Utc::now();
    let due_schedules = list_due_schedules(&connection, now)?;
    let mut generated_briefs = Vec::new();

    for schedule in due_schedules
        .into_iter()
        .filter(|schedule| schedule.template_id == SYSTEM_BRIEF_TEMPLATE_ID)
    {
        let job_id = create_job_for_due_schedule(&mut connection, &schedule.id, now)?;
        match complete_system_brief_job(&connection, db_path, &job_id) {
            Ok(brief) => {
                mark_schedule_run_completed(&connection, &job_id)?;
                generated_briefs.push(brief);
            }
            Err(error) => {
                mark_job_failed(&connection, &job_id, &error.to_string())?;
                return Err(error);
            }
        }
    }

    Ok(generated_briefs)
}

fn generate_system_brief_with_connection(
    connection: &mut Connection,
    db_path: &Path,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<BriefArtifact> {
    let template = require_builtin_template(SYSTEM_BRIEF_TEMPLATE_ID)?;
    let job_id = create_job_from_template(
        connection,
        &template,
        origin,
        actor_id,
        json!({ "sectionKey": SYSTEM_SECTION_KEY, "generator": "deterministic" }),
    )?;

    match complete_system_brief_job(connection, db_path, &job_id) {
        Ok(brief) => Ok(brief),
        Err(error) => {
            mark_job_failed(connection, &job_id, &error.to_string())?;
            Err(error)
        }
    }
}

fn complete_system_brief_job(
    connection: &Connection,
    db_path: &Path,
    job_id: &str,
) -> Result<BriefArtifact> {
    let job = load_job_for_brief(connection, job_id)?;
    if job.template_id != SYSTEM_BRIEF_TEMPLATE_ID {
        bail!("Job {job_id} is not a System Brief generation job");
    }

    let template = require_builtin_template_version(&job.template_id, job.template_version)?;
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE jobs SET status = 'running', started_at = COALESCE(started_at, ?1), updated_at = ?1 WHERE id = ?2",
        params![now, job_id],
    )?;

    let draft = build_deterministic_system_brief(connection, db_path, job_id, &job)?;

    run_task(
        connection,
        job_id,
        template
            .task("scope.validate")
            .map(|task| task.key.as_str())
            .unwrap_or("scope.validate"),
        json!({ "sectionKey": SYSTEM_SECTION_KEY, "valid": true }),
    )?;
    run_task(
        connection,
        job_id,
        "evidence.collect",
        json!({ "evidence": draft.evidence }),
    )?;
    run_task(
        connection,
        job_id,
        "evidence.manifest",
        json!({
            "evidenceCount": draft.evidence.len(),
            "sources": draft.evidence.iter().map(|evidence| evidence.source.clone()).collect::<Vec<_>>()
        }),
    )?;
    run_task(
        connection,
        job_id,
        "draft.generate",
        json!({ "mode": "deterministic", "summary": draft.summary }),
    )?;
    run_task(
        connection,
        job_id,
        "claims.validate",
        json!({ "valid": true, "limitations": draft.limitations }),
    )?;

    mark_task_running(connection, job_id, "artifact.save")?;
    let brief = insert_brief_artifact(connection, draft)?;
    append_job_event(
        connection,
        job_id,
        Some("artifact.save"),
        "brief.artifact.created",
        json!({ "briefId": brief.id, "sectionKey": brief.section_key, "version": brief.version }),
    )?;
    mark_task_succeeded(
        connection,
        job_id,
        "artifact.save",
        json!({ "briefId": brief.id, "version": brief.version }),
    )?;

    let completed_at = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE jobs
         SET status = 'succeeded', current_task_key = NULL, completed_at = ?1, updated_at = ?1,
             completed_required_task_count = required_task_count
         WHERE id = ?2",
        params![completed_at, job_id],
    )?;
    append_job_event(
        connection,
        job_id,
        None,
        "job.succeeded",
        json!({ "briefId": brief.id, "sectionKey": brief.section_key }),
    )?;

    Ok(brief)
}

#[derive(Debug, Clone)]
struct BriefJob {
    template_id: String,
    template_version: i64,
    origin: String,
}

#[derive(Debug, Clone)]
struct BriefDraft {
    id: String,
    section_key: String,
    job_id: String,
    process: BriefProcessProvenance,
    version: i64,
    title: String,
    summary: Vec<String>,
    body_markdown: String,
    evidence: Vec<BriefEvidence>,
    limitations: Vec<String>,
    visibility: String,
    created_at: String,
    valid_until: Option<String>,
}

fn build_deterministic_system_brief(
    connection: &Connection,
    db_path: &Path,
    job_id: &str,
    job: &BriefJob,
) -> Result<BriefDraft> {
    let health = build_health_report();
    let readiness = build_readiness_report(db_path);
    let created_at = Utc::now().to_rfc3339();
    let version = next_brief_version(connection, SYSTEM_SECTION_KEY)?;
    let health_status = health.status.clone();
    let readiness_status = readiness.status.clone();
    let sqlite_detail = readiness
        .checks
        .iter()
        .find(|check| check.name == "sqlite")
        .map(|check| check.detail.clone())
        .unwrap_or_else(|| "SQLite readiness evidence is not available.".to_string());
    let evidence = vec![
        BriefEvidence {
            label: "Daemon health".to_string(),
            value: health_status.clone(),
            source: "/health".to_string(),
        },
        BriefEvidence {
            label: "SQLite readiness".to_string(),
            value: readiness_status.clone(),
            source: "/ready".to_string(),
        },
        BriefEvidence {
            label: "Brief generator".to_string(),
            value: "deterministic fallback".to_string(),
            source: SYSTEM_BRIEF_TEMPLATE_ID.to_string(),
        },
    ];
    let limitations = vec![
        "No LLM adapter is configured; this brief uses deterministic local evidence.".to_string(),
        "Job history and persisted event replay are still limited to the Phase 1 daemon schema."
            .to_string(),
    ];
    let summary = vec![
        format!("Daemon health is {health_status}."),
        format!("SQLite readiness is {readiness_status}: {sqlite_detail}"),
        "The System Brief is now a durable artifact written through the brief.system.generate process.".to_string(),
    ];
    let body_markdown = format!(
        "System Brief\n\n- Health: {health_status}\n- Readiness: {readiness_status}\n- Generator: deterministic fallback\n\nRecommended next action: keep the appliance running and inspect Events if health or readiness changes."
    );

    Ok(BriefDraft {
        id: format!("brief_{}", Uuid::new_v4()),
        section_key: SYSTEM_SECTION_KEY.to_string(),
        job_id: job_id.to_string(),
        process: BriefProcessProvenance {
            job_id: job_id.to_string(),
            template_id: job.template_id.clone(),
            template_version: job.template_version,
            origin: job.origin.clone(),
            status: "succeeded".to_string(),
        },
        version,
        title: "System Brief".to_string(),
        summary,
        body_markdown,
        evidence,
        limitations,
        visibility: "local_operator".to_string(),
        created_at,
        valid_until: None,
    })
}

fn insert_brief_artifact(connection: &Connection, draft: BriefDraft) -> Result<BriefArtifact> {
    let summary_json = serde_json::to_string(&draft.summary)?;
    let evidence_json = serde_json::to_string(&draft.evidence)?;
    let limitations_json = serde_json::to_string(&draft.limitations)?;
    connection.execute(
        "INSERT INTO brief_artifacts (
            id, section_key, job_id, version, title, summary_json, body_markdown,
            evidence_json, limitations_json, visibility, created_at, valid_until
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            draft.id,
            draft.section_key,
            draft.job_id,
            draft.version,
            draft.title,
            summary_json,
            draft.body_markdown,
            evidence_json,
            limitations_json,
            draft.visibility,
            draft.created_at,
            draft.valid_until,
        ],
    )?;
    connection.execute(
        "INSERT INTO job_artifacts (id, job_id, task_key, artifact_kind, uri, label, metadata_json, created_at)
         VALUES (?1, ?2, 'artifact.save', 'brief.system', ?3, 'System Brief', ?4, ?5)",
        params![
            format!("artifact_{}", Uuid::new_v4()),
            draft.job_id,
            format!("ordo://briefs/system/{}", draft.id),
            json!({ "briefId": draft.id, "sectionKey": draft.section_key, "version": draft.version }).to_string(),
            draft.created_at,
        ],
    )?;

    Ok(BriefArtifact {
        id: draft.id,
        section_key: draft.section_key,
        job_id: Some(draft.job_id),
        process: Some(draft.process),
        version: draft.version,
        title: draft.title,
        summary: draft.summary,
        body_markdown: draft.body_markdown,
        evidence: draft.evidence,
        limitations: draft.limitations,
        visibility: draft.visibility,
        created_at: draft.created_at,
        valid_until: draft.valid_until,
    })
}

fn load_latest_system_brief(connection: &Connection) -> Result<Option<BriefArtifact>> {
    connection
        .query_row(
            "SELECT b.id, b.section_key, b.job_id, b.version, b.title, b.summary_json,
                    b.body_markdown, b.evidence_json, b.limitations_json, b.visibility,
                    b.created_at, b.valid_until,
                    j.template_id, j.template_version, j.origin, j.status
             FROM brief_artifacts b
             LEFT JOIN jobs j ON j.id = b.job_id
             WHERE b.section_key = ?1
             ORDER BY b.created_at DESC, b.version DESC
             LIMIT 1",
            [SYSTEM_SECTION_KEY],
            |row| {
                let job_id: Option<String> = row.get(2)?;
                let template_id: Option<String> = row.get(12)?;
                let template_version: Option<i64> = row.get(13)?;
                let origin: Option<String> = row.get(14)?;
                let status: Option<String> = row.get(15)?;
                let process = match (
                    job_id.clone(),
                    template_id,
                    template_version,
                    origin,
                    status,
                ) {
                    (
                        Some(job_id),
                        Some(template_id),
                        Some(template_version),
                        Some(origin),
                        Some(status),
                    ) => Some(BriefProcessProvenance {
                        job_id,
                        template_id,
                        template_version,
                        origin,
                        status,
                    }),
                    _ => None,
                };
                let summary_json: String = row.get(5)?;
                let evidence_json: String = row.get(7)?;
                let limitations_json: String = row.get(8)?;

                Ok(BriefArtifact {
                    id: row.get(0)?,
                    section_key: row.get(1)?,
                    job_id,
                    process,
                    version: row.get(3)?,
                    title: row.get(4)?,
                    summary: serde_json::from_str(&summary_json).unwrap_or_default(),
                    body_markdown: row.get(6)?,
                    evidence: serde_json::from_str(&evidence_json).unwrap_or_default(),
                    limitations: serde_json::from_str(&limitations_json).unwrap_or_default(),
                    visibility: row.get(9)?,
                    created_at: row.get(10)?,
                    valid_until: row.get(11)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn load_job_for_brief(connection: &Connection, job_id: &str) -> Result<BriefJob> {
    let job = connection.query_row(
        "SELECT template_id, template_version, origin FROM jobs WHERE id = ?1",
        [job_id],
        |row| {
            Ok(BriefJob {
                template_id: row.get(0)?,
                template_version: row.get(1)?,
                origin: row.get(2)?,
            })
        },
    )?;
    Ok(job)
}

fn next_brief_version(connection: &Connection, section_key: &str) -> Result<i64> {
    let version = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) + 1 FROM brief_artifacts WHERE section_key = ?1",
        [section_key],
        |row| row.get(0),
    )?;
    Ok(version)
}

fn run_task(connection: &Connection, job_id: &str, task_key: &str, output: Value) -> Result<()> {
    mark_task_running(connection, job_id, task_key)?;
    mark_task_succeeded(connection, job_id, task_key, output)?;
    Ok(())
}

fn mark_task_running(connection: &Connection, job_id: &str, task_key: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let status: String = connection.query_row(
        "SELECT status FROM job_tasks WHERE job_id = ?1 AND task_key = ?2",
        params![job_id, task_key],
        |row| row.get(0),
    )?;
    if status == "pending" {
        connection.execute(
            "UPDATE job_tasks SET status = 'ready', updated_at = ?1 WHERE job_id = ?2 AND task_key = ?3",
            params![now, job_id, task_key],
        )?;
        append_job_event(
            connection,
            job_id,
            Some(task_key),
            "task.ready",
            json!({ "taskKey": task_key }),
        )?;
    }

    connection.execute(
        "UPDATE job_tasks
         SET status = 'running', attempt_count = attempt_count + 1,
             started_at = COALESCE(started_at, ?1), updated_at = ?1
         WHERE job_id = ?2 AND task_key = ?3",
        params![now, job_id, task_key],
    )?;
    connection.execute(
        "UPDATE jobs SET status = 'running', current_task_key = ?1, updated_at = ?2 WHERE id = ?3",
        params![task_key, now, job_id],
    )?;
    append_job_event(
        connection,
        job_id,
        Some(task_key),
        "task.started",
        json!({ "taskKey": task_key }),
    )?;
    Ok(())
}

fn mark_task_succeeded(
    connection: &Connection,
    job_id: &str,
    task_key: &str,
    output: Value,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE job_tasks
         SET status = 'succeeded', output_json = ?1, completed_at = ?2, updated_at = ?2
         WHERE job_id = ?3 AND task_key = ?4",
        params![output.to_string(), now, job_id, task_key],
    )?;
    let completed_required_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM job_tasks
         WHERE job_id = ?1 AND required = 1 AND status IN ('succeeded', 'skipped')",
        [job_id],
        |row| row.get(0),
    )?;
    connection.execute(
        "UPDATE jobs SET completed_required_task_count = ?1, updated_at = ?2 WHERE id = ?3",
        params![completed_required_count, now, job_id],
    )?;
    append_job_event(
        connection,
        job_id,
        Some(task_key),
        "task.succeeded",
        json!({ "taskKey": task_key, "output": output }),
    )?;
    Ok(())
}

fn mark_job_failed(connection: &Connection, job_id: &str, message: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE jobs
         SET status = 'failed', current_task_key = NULL, completed_at = ?1, updated_at = ?1, failure_message = ?2
         WHERE id = ?3",
        params![now, message, job_id],
    )?;
    append_job_event(
        connection,
        job_id,
        None,
        "job.failed",
        json!({ "message": message }),
    )?;
    Ok(())
}

fn mark_schedule_run_completed(connection: &Connection, job_id: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE scheduled_job_runs SET status = 'completed', completed_at = ?1 WHERE job_id = ?2",
        params![now, job_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::ensure_default_system_brief_schedule;
    use crate::schema::init_database;
    use tempfile::NamedTempFile;

    #[test]
    fn generated_system_brief_is_durable_and_provenanced() {
        let db_file = NamedTempFile::new().unwrap();
        init_database(db_file.path()).unwrap();

        let brief = generate_system_brief(db_file.path(), "test", Some("tester")).unwrap();
        let latest = latest_system_brief(db_file.path()).unwrap().unwrap();

        assert_eq!(latest.id, brief.id);
        assert_eq!(latest.section_key, "system");
        assert_eq!(latest.summary.len(), 3);
        assert!(!latest.evidence.is_empty());
        assert_eq!(
            latest.process.as_ref().unwrap().template_id,
            SYSTEM_BRIEF_TEMPLATE_ID
        );
    }

    #[test]
    fn due_system_brief_schedule_generates_artifact_and_advances() {
        let db_file = NamedTempFile::new().unwrap();
        init_database(db_file.path()).unwrap();

        let generated = run_due_system_brief_schedules(db_file.path()).unwrap();
        let latest = latest_system_brief(db_file.path()).unwrap();

        assert_eq!(generated.len(), 1);
        assert!(latest.is_some());

        let connection = Connection::open(db_file.path()).unwrap();
        ensure_default_system_brief_schedule(&connection).unwrap();
        let due_count = list_due_schedules(&connection, Utc::now()).unwrap().len();
        assert_eq!(due_count, 0);
    }
}
