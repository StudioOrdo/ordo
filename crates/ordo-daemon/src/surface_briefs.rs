use anyhow::{ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::artifacts::{link_artifact, record_artifact, ArtifactInput, ArtifactLinkInput};
use crate::events::{append_realtime_event, system_event, RealtimeEvent};
use crate::kernel::{append_job_event, create_job_from_template};
use crate::templates::require_builtin_template;

pub const SURFACE_BRIEF_TEMPLATE_ID: &str = "surface.brief.generate";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceBriefView {
    pub id: String,
    pub surface_kind: String,
    pub subject_kind: Option<String>,
    pub subject_id: Option<String>,
    pub status: String,
    pub artifact_id: Option<String>,
    pub title: String,
    pub brief_markdown: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub created_by_job_id: Option<String>,
    pub generated_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub superseded_at: Option<String>,
    pub failure_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SurfaceBriefRequest {
    pub surface_kind: String,
    pub subject_kind: Option<String>,
    pub subject_id: Option<String>,
    pub origin: String,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeterministicSurfaceBriefInput {
    pub surface_kind: String,
    pub subject_kind: Option<String>,
    pub subject_id: Option<String>,
    pub title: String,
    pub brief_markdown: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub source_refs: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceBriefReadModel {
    pub latest_completed: Option<SurfaceBriefView>,
    pub refresh: Option<SurfaceBriefView>,
}

pub fn request_surface_brief_refresh(
    connection: &mut Connection,
    request: SurfaceBriefRequest,
) -> Result<SurfaceBriefView> {
    validate_surface_identity(
        &request.surface_kind,
        request.subject_kind.as_deref(),
        request.subject_id.as_deref(),
    )?;
    let template = require_builtin_template(SURFACE_BRIEF_TEMPLATE_ID)?;
    let job_id = create_job_from_template(
        connection,
        &template,
        &request.origin,
        request.actor_id.as_deref(),
        json!({
            "surfaceKind": request.surface_kind,
            "subjectKind": request.subject_kind,
            "subjectId": request.subject_id,
            "generator": "deterministic",
        }),
    )?;
    let now = Utc::now().to_rfc3339();
    let id = format!("surface_brief_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO surface_briefs (
            id, surface_kind, subject_kind, subject_id, status, title, brief_markdown,
            evidence_refs_json, limitations_json, created_by_job_id, generated_at,
            created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, 'queued', ?5, '', '[]', '[]', ?6, ?7, ?7, ?7)",
        params![
            id,
            request.surface_kind,
            request.subject_kind,
            request.subject_id,
            "Queued surface brief refresh",
            job_id,
            now,
        ],
    )?;
    let brief = load_surface_brief(connection, &id)?;
    append_realtime_event(
        connection,
        &system_event(
            "surface.brief.refresh.requested",
            json!({
                "surfaceBriefId": brief.id,
                "surfaceKind": brief.surface_kind,
                "subjectKind": brief.subject_kind,
                "subjectId": brief.subject_id,
                "jobId": job_id,
            }),
        ),
    )?;
    Ok(brief)
}

pub fn complete_deterministic_surface_brief(
    connection: &Connection,
    job_id: &str,
    input: DeterministicSurfaceBriefInput,
) -> Result<(SurfaceBriefView, RealtimeEvent)> {
    validate_brief_input(&input)?;
    ensure!(
        job_uses_surface_template(connection, job_id)?,
        "job is not a surface brief generation job"
    );

    mark_job_running(connection, job_id)?;
    run_task(
        connection,
        job_id,
        "scope.validate",
        json!({
            "surfaceKind": input.surface_kind,
            "subjectKind": input.subject_kind,
            "subjectId": input.subject_id,
            "valid": true,
        }),
    )?;
    run_task(
        connection,
        job_id,
        "evidence.collect",
        json!({
            "evidenceRefs": input.evidence_refs,
            "sourceRefs": input.source_refs,
        }),
    )?;
    run_task(
        connection,
        job_id,
        "draft.generate",
        json!({
            "mode": "deterministic",
            "title": input.title,
        }),
    )?;
    run_task(
        connection,
        job_id,
        "claims.validate",
        json!({
            "valid": true,
            "limitations": input.limitations,
        }),
    )?;

    let content_hash = content_hash(&input.title, &input.brief_markdown, &input.evidence_refs);
    let (artifact, _) = record_artifact(
        connection,
        ArtifactInput {
            artifact_kind: "surface.brief".to_string(),
            title: input.title.clone(),
            status: "published".to_string(),
            visibility_ceiling: "staff".to_string(),
            summary: first_brief_line(&input.brief_markdown),
            source_kind: Some("surface".to_string()),
            source_id: Some(surface_identity(
                &input.surface_kind,
                input.subject_kind.as_deref(),
                input.subject_id.as_deref(),
            )),
            evidence_refs: input.evidence_refs.clone(),
            provenance: json!({
                "generatedBy": SURFACE_BRIEF_TEMPLATE_ID,
                "jobId": job_id,
                "mode": "deterministic",
            }),
            content_hash,
            storage_uri: None,
            health_status: Some("available".to_string()),
            created_by_job_id: Some(job_id.to_string()),
        },
    )?;
    for (source_kind, source_id) in &input.source_refs {
        link_artifact(
            connection,
            &artifact.id,
            ArtifactLinkInput {
                link_kind: "evidence".to_string(),
                source_kind: source_kind.clone(),
                source_id: source_id.clone(),
                relation: "cited_by_surface_brief".to_string(),
                evidence_refs: input.evidence_refs.clone(),
                provenance: json!({
                    "generatedBy": SURFACE_BRIEF_TEMPLATE_ID,
                    "jobId": job_id,
                }),
            },
        )?;
    }

    mark_task_running(connection, job_id, "artifact.save")?;
    supersede_prior_completed(
        connection,
        &input.surface_kind,
        input.subject_kind.as_deref(),
        input.subject_id.as_deref(),
    )?;
    let now = Utc::now().to_rfc3339();
    let id = existing_refresh_for_job(connection, job_id)?
        .unwrap_or_else(|| format!("surface_brief_{}", Uuid::new_v4()));
    connection.execute(
        "INSERT INTO surface_briefs (
            id, surface_kind, subject_kind, subject_id, status, artifact_id, title,
            brief_markdown, evidence_refs_json, limitations_json, created_by_job_id,
            generated_at, created_at, updated_at, completed_at
         ) VALUES (?1, ?2, ?3, ?4, 'completed', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, ?11, ?11)
         ON CONFLICT(id) DO UPDATE SET
            status = 'completed',
            artifact_id = excluded.artifact_id,
            title = excluded.title,
            brief_markdown = excluded.brief_markdown,
            evidence_refs_json = excluded.evidence_refs_json,
            limitations_json = excluded.limitations_json,
            generated_at = excluded.generated_at,
            updated_at = excluded.updated_at,
            completed_at = excluded.completed_at,
            failure_message = NULL",
        params![
            id,
            input.surface_kind,
            input.subject_kind,
            input.subject_id,
            artifact.id,
            input.title,
            input.brief_markdown,
            json!(input.evidence_refs).to_string(),
            json!(input.limitations).to_string(),
            job_id,
            now,
        ],
    )?;
    mark_task_succeeded(
        connection,
        job_id,
        "artifact.save",
        json!({
            "surfaceBriefId": id,
            "artifactId": artifact.id,
        }),
    )?;
    mark_job_succeeded(connection, job_id, &id)?;
    let brief = load_surface_brief(connection, &id)?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "surface.brief.completed",
            json!({
                "surfaceBriefId": brief.id,
                "surfaceKind": brief.surface_kind,
                "subjectKind": brief.subject_kind,
                "subjectId": brief.subject_id,
                "artifactId": brief.artifact_id,
                "evidenceRefs": brief.evidence_refs,
                "limitations": brief.limitations,
            }),
        ),
    )?;
    Ok((brief, event))
}

pub fn fail_surface_brief_refresh(
    connection: &Connection,
    job_id: &str,
    message: &str,
) -> Result<(SurfaceBriefView, RealtimeEvent)> {
    let now = Utc::now().to_rfc3339();
    let refresh_id = existing_refresh_for_job(connection, job_id)?
        .ok_or_else(|| anyhow::anyhow!("surface brief refresh not found for job {job_id}"))?;
    connection.execute(
        "UPDATE surface_briefs
         SET status = 'failed', failure_message = ?1, updated_at = ?2
         WHERE id = ?3",
        params![message, now, refresh_id],
    )?;
    mark_job_failed(connection, job_id, message)?;
    let brief = load_surface_brief(connection, &refresh_id)?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "surface.brief.failed",
            json!({
                "surfaceBriefId": brief.id,
                "surfaceKind": brief.surface_kind,
                "message": message,
            }),
        ),
    )?;
    Ok((brief, event))
}

pub fn load_surface_brief_read_model(
    connection: &Connection,
    surface_kind: &str,
    subject_kind: Option<&str>,
    subject_id: Option<&str>,
) -> Result<SurfaceBriefReadModel> {
    Ok(SurfaceBriefReadModel {
        latest_completed: load_latest_completed_surface_brief(
            connection,
            surface_kind,
            subject_kind,
            subject_id,
        )?,
        refresh: load_latest_refresh_surface_brief(
            connection,
            surface_kind,
            subject_kind,
            subject_id,
        )?,
    })
}

pub fn load_latest_completed_surface_brief(
    connection: &Connection,
    surface_kind: &str,
    subject_kind: Option<&str>,
    subject_id: Option<&str>,
) -> Result<Option<SurfaceBriefView>> {
    load_surface_brief_by_statuses(
        connection,
        surface_kind,
        subject_kind,
        subject_id,
        &["completed"],
    )
}

fn load_latest_refresh_surface_brief(
    connection: &Connection,
    surface_kind: &str,
    subject_kind: Option<&str>,
    subject_id: Option<&str>,
) -> Result<Option<SurfaceBriefView>> {
    load_surface_brief_by_statuses(
        connection,
        surface_kind,
        subject_kind,
        subject_id,
        &["queued", "running", "failed"],
    )
}

fn validate_surface_identity(
    surface_kind: &str,
    subject_kind: Option<&str>,
    subject_id: Option<&str>,
) -> Result<()> {
    ensure!(!surface_kind.trim().is_empty(), "surface kind is required");
    ensure!(
        subject_kind.is_some() == subject_id.is_some(),
        "subject kind and subject id must be provided together"
    );
    Ok(())
}

fn validate_brief_input(input: &DeterministicSurfaceBriefInput) -> Result<()> {
    validate_surface_identity(
        &input.surface_kind,
        input.subject_kind.as_deref(),
        input.subject_id.as_deref(),
    )?;
    ensure!(!input.title.trim().is_empty(), "brief title is required");
    ensure!(
        !input.brief_markdown.trim().is_empty(),
        "brief markdown is required"
    );
    ensure!(
        !input.evidence_refs.is_empty(),
        "surface brief evidence refs are required"
    );
    ensure!(
        !input.limitations.is_empty(),
        "surface brief limitations are required"
    );
    Ok(())
}

fn job_uses_surface_template(connection: &Connection, job_id: &str) -> Result<bool> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM jobs WHERE id = ?1 AND template_id = ?2",
        params![job_id, SURFACE_BRIEF_TEMPLATE_ID],
        |row| row.get(0),
    )?;
    Ok(count == 1)
}

fn existing_refresh_for_job(connection: &Connection, job_id: &str) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT id FROM surface_briefs WHERE created_by_job_id = ?1 ORDER BY created_at DESC LIMIT 1",
            [job_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

fn supersede_prior_completed(
    connection: &Connection,
    surface_kind: &str,
    subject_kind: Option<&str>,
    subject_id: Option<&str>,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let prior =
        load_latest_completed_surface_brief(connection, surface_kind, subject_kind, subject_id)?;
    if let Some(prior) = prior {
        connection.execute(
            "UPDATE surface_briefs
             SET status = 'superseded', superseded_at = ?1, updated_at = ?1
             WHERE id = ?2",
            params![now, prior.id],
        )?;
        append_realtime_event(
            connection,
            &system_event(
                "surface.brief.superseded",
                json!({
                    "surfaceBriefId": prior.id,
                    "surfaceKind": prior.surface_kind,
                    "subjectKind": prior.subject_kind,
                    "subjectId": prior.subject_id,
                }),
            ),
        )?;
    }
    Ok(())
}

fn load_surface_brief_by_statuses(
    connection: &Connection,
    surface_kind: &str,
    subject_kind: Option<&str>,
    subject_id: Option<&str>,
    statuses: &[&str],
) -> Result<Option<SurfaceBriefView>> {
    let status_values = statuses
        .iter()
        .map(|status| format!("'{status}'"))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT id, surface_kind, subject_kind, subject_id, status, artifact_id, title,
                brief_markdown, evidence_refs_json, limitations_json, created_by_job_id,
                generated_at, created_at, updated_at, completed_at, superseded_at,
                failure_message
         FROM surface_briefs
         WHERE surface_kind = ?1
           AND COALESCE(subject_kind, '') = COALESCE(?2, '')
           AND COALESCE(subject_id, '') = COALESCE(?3, '')
           AND status IN ({status_values})
         ORDER BY generated_at DESC, created_at DESC
         LIMIT 1"
    );
    connection
        .query_row(
            &sql,
            params![surface_kind, subject_kind, subject_id],
            surface_brief_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn load_surface_brief(connection: &Connection, brief_id: &str) -> Result<SurfaceBriefView> {
    connection
        .query_row(
            "SELECT id, surface_kind, subject_kind, subject_id, status, artifact_id, title,
                    brief_markdown, evidence_refs_json, limitations_json, created_by_job_id,
                    generated_at, created_at, updated_at, completed_at, superseded_at,
                    failure_message
             FROM surface_briefs WHERE id = ?1",
            [brief_id],
            surface_brief_from_row,
        )
        .map_err(Into::into)
}

fn surface_brief_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SurfaceBriefView> {
    let evidence_json: String = row.get(8)?;
    let limitations_json: String = row.get(9)?;
    Ok(SurfaceBriefView {
        id: row.get(0)?,
        surface_kind: row.get(1)?,
        subject_kind: row.get(2)?,
        subject_id: row.get(3)?,
        status: row.get(4)?,
        artifact_id: row.get(5)?,
        title: row.get(6)?,
        brief_markdown: row.get(7)?,
        evidence_refs: serde_json::from_str(&evidence_json).unwrap_or_default(),
        limitations: serde_json::from_str(&limitations_json).unwrap_or_default(),
        created_by_job_id: row.get(10)?,
        generated_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        completed_at: row.get(14)?,
        superseded_at: row.get(15)?,
        failure_message: row.get(16)?,
    })
}

fn mark_job_running(connection: &Connection, job_id: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE jobs SET status = 'running', started_at = COALESCE(started_at, ?1), updated_at = ?1 WHERE id = ?2",
        params![now, job_id],
    )?;
    if let Some(refresh_id) = existing_refresh_for_job(connection, job_id)? {
        connection.execute(
            "UPDATE surface_briefs SET status = 'running', updated_at = ?1 WHERE id = ?2",
            params![now, refresh_id],
        )?;
    }
    Ok(())
}

fn run_task(connection: &Connection, job_id: &str, task_key: &str, output: Value) -> Result<()> {
    mark_task_running(connection, job_id, task_key)?;
    mark_task_succeeded(connection, job_id, task_key, output)?;
    Ok(())
}

fn mark_task_running(connection: &Connection, job_id: &str, task_key: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE job_tasks
         SET status = CASE WHEN status = 'pending' THEN 'ready' ELSE status END,
             updated_at = ?1
         WHERE job_id = ?2 AND task_key = ?3",
        params![now, job_id, task_key],
    )?;
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

fn mark_job_succeeded(connection: &Connection, job_id: &str, brief_id: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE jobs
         SET status = 'succeeded', current_task_key = NULL, completed_at = ?1, updated_at = ?1,
             completed_required_task_count = required_task_count
         WHERE id = ?2",
        params![now, job_id],
    )?;
    append_job_event(
        connection,
        job_id,
        None,
        "job.succeeded",
        json!({ "surfaceBriefId": brief_id }),
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

fn content_hash(title: &str, body: &str, evidence_refs: &[String]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    hasher.update(b"\n");
    hasher.update(body.as_bytes());
    for evidence_ref in evidence_refs {
        hasher.update(b"\n");
        hasher.update(evidence_ref.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn first_brief_line(markdown: &str) -> String {
    markdown
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Surface brief")
        .trim_start_matches('#')
        .trim()
        .to_string()
}

fn surface_identity(
    surface_kind: &str,
    subject_kind: Option<&str>,
    subject_id: Option<&str>,
) -> String {
    match (subject_kind, subject_id) {
        (Some(kind), Some(id)) => format!("{surface_kind}:{kind}:{id}"),
        _ => surface_kind.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::schema::init_schema;
    use crate::templates::seed_builtin_templates;

    #[test]
    fn surface_brief_records_require_evidence_and_limitations() {
        let mut connection = test_connection();
        let queued = request_surface_brief_refresh(&mut connection, request("offers")).unwrap();

        let missing = complete_deterministic_surface_brief(
            &connection,
            queued.created_by_job_id.as_ref().unwrap(),
            DeterministicSurfaceBriefInput {
                surface_kind: "offers".to_string(),
                subject_kind: None,
                subject_id: None,
                title: "Offers brief".to_string(),
                brief_markdown: "Offers have evidence.".to_string(),
                evidence_refs: vec![],
                limitations: vec!["No payment evidence.".to_string()],
                source_refs: vec![],
            },
        );

        assert!(missing.is_err());
    }

    #[test]
    fn deterministic_generation_creates_completed_brief_and_artifact_link() {
        let mut connection = test_connection();
        let queued = request_surface_brief_refresh(&mut connection, request("offers")).unwrap();
        let job_id = queued.created_by_job_id.clone().unwrap();

        let (brief, event) = complete_deterministic_surface_brief(
            &connection,
            &job_id,
            offers_brief("Offer brief v1"),
        )
        .unwrap();

        assert_eq!(brief.status, "completed");
        assert_eq!(brief.surface_kind, "offers");
        assert!(brief.artifact_id.is_some());
        assert_eq!(brief.evidence_refs, vec!["offer_starter"]);
        assert_eq!(event.event_type, "surface.brief.completed");
        let artifact_links: i64 = connection
            .query_row("SELECT COUNT(*) FROM artifact_links", [], |row| row.get(0))
            .unwrap();
        assert_eq!(artifact_links, 1);
    }

    #[test]
    fn latest_completed_loads_while_refresh_is_queued_or_running() {
        let mut connection = test_connection();
        let first = request_surface_brief_refresh(&mut connection, request("asks")).unwrap();
        complete_deterministic_surface_brief(
            &connection,
            first.created_by_job_id.as_ref().unwrap(),
            asks_brief("Ask brief v1"),
        )
        .unwrap();
        let refresh = request_surface_brief_refresh(&mut connection, request("asks")).unwrap();

        let read_model = load_surface_brief_read_model(&connection, "asks", None, None).unwrap();
        assert_eq!(
            read_model.latest_completed.as_ref().unwrap().title,
            "Ask brief v1"
        );
        assert_eq!(read_model.refresh.as_ref().unwrap().id, refresh.id);
        assert_eq!(read_model.refresh.as_ref().unwrap().status, "queued");
    }

    #[test]
    fn newer_completed_brief_supersedes_older_completed_brief() {
        let mut connection = test_connection();
        let first = request_surface_brief_refresh(&mut connection, request("artifacts")).unwrap();
        let first = complete_deterministic_surface_brief(
            &connection,
            first.created_by_job_id.as_ref().unwrap(),
            artifact_surface_brief("Artifact brief v1"),
        )
        .unwrap()
        .0;
        let second = request_surface_brief_refresh(&mut connection, request("artifacts")).unwrap();
        let second = complete_deterministic_surface_brief(
            &connection,
            second.created_by_job_id.as_ref().unwrap(),
            artifact_surface_brief("Artifact brief v2"),
        )
        .unwrap()
        .0;

        let old = load_surface_brief(&connection, &first.id).unwrap();
        let latest = load_latest_completed_surface_brief(&connection, "artifacts", None, None)
            .unwrap()
            .unwrap();
        assert_eq!(old.status, "superseded");
        assert!(old.superseded_at.is_some());
        assert_eq!(latest.id, second.id);
    }

    #[test]
    fn failed_refresh_preserves_previous_completed_brief() {
        let mut connection = test_connection();
        let first =
            request_surface_brief_refresh(&mut connection, request("conversations")).unwrap();
        complete_deterministic_surface_brief(
            &connection,
            first.created_by_job_id.as_ref().unwrap(),
            conversation_brief("Conversation brief v1"),
        )
        .unwrap();
        let refresh =
            request_surface_brief_refresh(&mut connection, request("conversations")).unwrap();
        fail_surface_brief_refresh(
            &connection,
            refresh.created_by_job_id.as_ref().unwrap(),
            "deterministic evidence unavailable",
        )
        .unwrap();

        let read_model =
            load_surface_brief_read_model(&connection, "conversations", None, None).unwrap();
        assert_eq!(
            read_model.latest_completed.as_ref().unwrap().title,
            "Conversation brief v1"
        );
        assert_eq!(read_model.refresh.as_ref().unwrap().status, "failed");
    }

    fn test_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        seed_builtin_templates(&connection).unwrap();
        connection
    }

    fn request(surface_kind: &str) -> SurfaceBriefRequest {
        SurfaceBriefRequest {
            surface_kind: surface_kind.to_string(),
            subject_kind: None,
            subject_id: None,
            origin: "test".to_string(),
            actor_id: Some("tester".to_string()),
        }
    }

    fn offers_brief(title: &str) -> DeterministicSurfaceBriefInput {
        DeterministicSurfaceBriefInput {
            surface_kind: "offers".to_string(),
            subject_kind: None,
            subject_id: None,
            title: title.to_string(),
            brief_markdown: "Offer evidence says the Starter offer exists. Next action: inspect current outcomes.".to_string(),
            evidence_refs: vec!["offer_starter".to_string()],
            limitations: vec!["No payment or external CRM evidence.".to_string()],
            source_refs: vec![("offer".to_string(), "offer_starter".to_string())],
        }
    }

    fn asks_brief(title: &str) -> DeterministicSurfaceBriefInput {
        DeterministicSurfaceBriefInput {
            surface_kind: "asks".to_string(),
            subject_kind: None,
            subject_id: None,
            title: title.to_string(),
            brief_markdown:
                "Ask evidence is present. Next action: request consent before attribution."
                    .to_string(),
            evidence_refs: vec!["ask_beta".to_string()],
            limitations: vec!["Referral contact is not confirmed.".to_string()],
            source_refs: vec![("ask".to_string(), "ask_beta".to_string())],
        }
    }

    fn artifact_surface_brief(title: &str) -> DeterministicSurfaceBriefInput {
        DeterministicSurfaceBriefInput {
            surface_kind: "artifacts".to_string(),
            subject_kind: None,
            subject_id: None,
            title: title.to_string(),
            brief_markdown:
                "Artifact evidence is available. Next action: decide client deliverable status."
                    .to_string(),
            evidence_refs: vec!["artifact_qr_card_1".to_string()],
            limitations: vec!["No external performance data.".to_string()],
            source_refs: vec![("artifact".to_string(), "artifact_qr_card_1".to_string())],
        }
    }

    fn conversation_brief(title: &str) -> DeterministicSurfaceBriefInput {
        DeterministicSurfaceBriefInput {
            surface_kind: "conversations".to_string(),
            subject_kind: None,
            subject_id: None,
            title: title.to_string(),
            brief_markdown:
                "Conversation evidence is available. Next action: answer the pricing question."
                    .to_string(),
            evidence_refs: vec!["message_ava_14".to_string()],
            limitations: vec!["No hosted identity evidence.".to_string()],
            source_refs: vec![("message".to_string(), "message_ava_14".to_string())],
        }
    }
}
