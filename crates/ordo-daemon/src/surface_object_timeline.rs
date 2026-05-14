use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

use crate::surface_work_items::SurfaceWorkItemViewer;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SurfaceObjectTimelineQuery {
    pub viewer: SurfaceWorkItemViewer,
    pub object_kind: Option<String>,
    pub object_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceObjectTimelineResponse {
    pub entries: Vec<SurfaceObjectTimelineEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceObjectTimelineEntry {
    pub id: String,
    pub object_kind: String,
    pub object_id: String,
    pub source_kind: String,
    pub source_id: String,
    pub event_type: String,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub visibility: String,
    pub occurred_at: String,
    pub sequence: i64,
    pub operational_context: Value,
    pub evidence_refs: Vec<String>,
    pub projected_at: String,
}

pub fn list_surface_object_timeline(
    db_path: &Path,
    query: SurfaceObjectTimelineQuery,
) -> Result<SurfaceObjectTimelineResponse> {
    let mut connection = Connection::open(db_path)?;
    rebuild_surface_object_timeline(&mut connection)?;
    load_surface_object_timeline(&connection, query)
}

pub fn rebuild_surface_object_timeline(connection: &mut Connection) -> Result<usize> {
    let transaction = connection.transaction()?;
    transaction.execute("DELETE FROM surface_object_timeline", [])?;
    let projected_at = Utc::now().to_rfc3339();
    let mut projected = 0;

    projected += project_job_events(&transaction, &projected_at)?;
    projected += project_artifacts(&transaction, &projected_at)?;
    projected += project_handoff_events(&transaction, &projected_at)?;
    projected += project_reward_events(&transaction, &projected_at)?;

    transaction.commit()?;
    Ok(projected)
}

pub fn load_surface_object_timeline(
    connection: &Connection,
    query: SurfaceObjectTimelineQuery,
) -> Result<SurfaceObjectTimelineResponse> {
    let mut statement = connection.prepare(
        "SELECT id, object_kind, object_id, source_kind, source_id, event_type,
                title, summary, status, visibility, occurred_at, sequence,
                operational_context_json, evidence_refs_json, projected_at
         FROM surface_object_timeline
         ORDER BY occurred_at DESC, sequence DESC, id ASC",
    )?;
    let entries = statement
        .query_map([], surface_object_timeline_entry_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let limit = query.limit.unwrap_or(100).min(500);
    Ok(SurfaceObjectTimelineResponse {
        entries: entries
            .into_iter()
            .filter(|entry| query_matches_entry(&query, entry))
            .take(limit)
            .map(|entry| entry_for_viewer(query.viewer, entry))
            .collect(),
    })
}

#[derive(Debug, Clone)]
struct TimelineInput {
    object_kind: &'static str,
    object_id: String,
    source_kind: &'static str,
    source_id: String,
    event_type: String,
    title: String,
    summary: String,
    status: String,
    visibility: String,
    occurred_at: String,
    sequence: i64,
    operational_context: Value,
    evidence_refs: Vec<String>,
}

fn project_job_events(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT event.id, event.job_id, event.task_key, event.sequence, event.event_type,
                event.payload_json, event.created_at, job.kind, job.status
         FROM job_events event
         JOIN jobs job ON job.id = event.job_id",
    )?;
    let mut projected = 0;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
        ))
    })?;
    for row in rows {
        let (
            event_id,
            job_id,
            task_key,
            sequence,
            event_type,
            payload_json,
            occurred_at,
            kind,
            status,
        ) = row?;
        let payload = parse_json_object(&payload_json);
        insert_entry(
            transaction,
            projected_at,
            TimelineInput {
                object_kind: "job",
                object_id: job_id.clone(),
                source_kind: "job_event",
                source_id: event_id.clone(),
                event_type: event_type.clone(),
                title: format!("Job {kind}"),
                summary: safe_job_event_summary(&event_type, &status, task_key.as_deref()),
                status: status.clone(),
                visibility: "staff".to_string(),
                occurred_at: occurred_at.clone(),
                sequence,
                operational_context: json!({
                    "jobId": job_id,
                    "taskKey": task_key,
                    "eventPayload": staff_safe_event_payload(payload),
                }),
                evidence_refs: vec![format!("job_event:{event_id}")],
            },
        )?;
        projected += 1;

        if let Some(task_key) = task_key {
            insert_entry(
                transaction,
                projected_at,
                TimelineInput {
                    object_kind: "job_task",
                    object_id: format!("{job_id}:{task_key}"),
                    source_kind: "job_event",
                    source_id: event_id.clone(),
                    event_type: event_type.clone(),
                    title: format!("Task {task_key}"),
                    summary: safe_job_event_summary(&event_type, &status, Some(&task_key)),
                    status: status.clone(),
                    visibility: "staff".to_string(),
                    occurred_at,
                    sequence,
                    operational_context: json!({
                        "jobId": job_id,
                        "taskKey": task_key,
                    }),
                    evidence_refs: vec![format!("job_event:{event_id}")],
                },
            )?;
            projected += 1;
        }
    }
    Ok(projected)
}

fn project_artifacts(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, artifact_kind, title, status, visibility_ceiling, summary,
                source_kind, source_id, evidence_refs_json, health_status,
                created_by_job_id, created_at, updated_at
         FROM artifacts",
    )?;
    let mut projected = 0;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, Option<String>>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
        ))
    })?;
    for row in rows {
        let (
            id,
            artifact_kind,
            title,
            status,
            visibility_ceiling,
            summary,
            source_kind,
            source_id,
            evidence_refs_json,
            health_status,
            created_by_job_id,
            created_at,
            _updated_at,
        ) = row?;
        insert_entry(
            transaction,
            projected_at,
            TimelineInput {
                object_kind: "artifact",
                object_id: id.clone(),
                source_kind: "artifact",
                source_id: id.clone(),
                event_type: "artifact.current_state".to_string(),
                title,
                summary,
                status,
                visibility: visibility_from_ceiling(&visibility_ceiling).to_string(),
                occurred_at: created_at,
                sequence: 0,
                operational_context: json!({
                    "artifactKind": artifact_kind,
                    "healthStatus": health_status,
                    "createdByJobId": created_by_job_id,
                }),
                evidence_refs: append_source_ref(
                    parse_string_vec(&evidence_refs_json),
                    source_kind,
                    source_id,
                ),
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn project_handoff_events(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT event.id, event.handoff_item_id, event.event_type, event.payload_json,
                event.occurred_at, item.reason, item.delivery_state, item.visibility
         FROM handoff_events event
         JOIN handoff_inbox_items item ON item.id = event.handoff_item_id",
    )?;
    let mut projected = 0;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
        ))
    })?;
    for row in rows {
        let (
            event_id,
            handoff_item_id,
            event_type,
            payload_json,
            occurred_at,
            reason,
            state,
            visibility,
        ) = row?;
        insert_entry(
            transaction,
            projected_at,
            TimelineInput {
                object_kind: "handoff_inbox_item",
                object_id: handoff_item_id.clone(),
                source_kind: "handoff_event",
                source_id: event_id.clone(),
                event_type: event_type.clone(),
                title: reason,
                summary: format!("Support handoff event `{event_type}` recorded."),
                status: state,
                visibility,
                occurred_at,
                sequence: 0,
                operational_context: staff_safe_event_payload(parse_json_object(&payload_json)),
                evidence_refs: vec![
                    format!("handoff_event:{event_id}"),
                    format!("handoff_inbox_item:{handoff_item_id}"),
                ],
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn project_reward_events(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, program_id, rule_id, actor_id, connection_id, source_kind,
                source_id, state, evidence_refs_json, created_at, updated_at
         FROM reward_events",
    )?;
    let mut projected = 0;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
        ))
    })?;
    for row in rows {
        let (
            id,
            program_id,
            rule_id,
            actor_id,
            connection_id,
            source_kind,
            source_id,
            state,
            evidence_refs_json,
            _created_at,
            updated_at,
        ) = row?;
        insert_entry(
            transaction,
            projected_at,
            TimelineInput {
                object_kind: "reward_event",
                object_id: id.clone(),
                source_kind: "reward_event",
                source_id: id.clone(),
                event_type: format!("reward.{state}"),
                title: "Reward event".to_string(),
                summary: format!("Reward event is {state} for {source_kind}."),
                status: state,
                visibility: "staff".to_string(),
                occurred_at: updated_at,
                sequence: 0,
                operational_context: json!({
                    "programId": program_id,
                    "ruleId": rule_id,
                    "actorId": actor_id,
                    "connectionId": connection_id,
                    "sourceKind": source_kind,
                    "sourceId": source_id,
                }),
                evidence_refs: append_source_ref(
                    parse_string_vec(&evidence_refs_json),
                    Some(source_kind),
                    Some(source_id),
                ),
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn insert_entry(
    transaction: &Transaction<'_>,
    projected_at: &str,
    entry: TimelineInput,
) -> Result<()> {
    let id = format!(
        "surface_object_timeline:{}:{}:{}:{}",
        entry.object_kind, entry.object_id, entry.source_kind, entry.source_id
    );
    transaction.execute(
        "INSERT INTO surface_object_timeline (
            id, object_kind, object_id, source_kind, source_id, event_type,
            title, summary, status, visibility, occurred_at, sequence,
            operational_context_json, evidence_refs_json, projected_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15
         )",
        params![
            id,
            entry.object_kind,
            entry.object_id,
            entry.source_kind,
            entry.source_id,
            entry.event_type,
            entry.title,
            entry.summary,
            entry.status,
            entry.visibility,
            entry.occurred_at,
            entry.sequence,
            entry.operational_context.to_string(),
            json!(entry.evidence_refs).to_string(),
            projected_at,
        ],
    )?;
    Ok(())
}

fn surface_object_timeline_entry_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<SurfaceObjectTimelineEntry> {
    let operational_context_json: String = row.get(12)?;
    let evidence_refs_json: String = row.get(13)?;
    Ok(SurfaceObjectTimelineEntry {
        id: row.get(0)?,
        object_kind: row.get(1)?,
        object_id: row.get(2)?,
        source_kind: row.get(3)?,
        source_id: row.get(4)?,
        event_type: row.get(5)?,
        title: row.get(6)?,
        summary: row.get(7)?,
        status: row.get(8)?,
        visibility: row.get(9)?,
        occurred_at: row.get(10)?,
        sequence: row.get(11)?,
        operational_context: serde_json::from_str(&operational_context_json)
            .unwrap_or_else(|_| json!({})),
        evidence_refs: parse_string_vec(&evidence_refs_json),
        projected_at: row.get(14)?,
    })
}

fn query_matches_entry(
    query: &SurfaceObjectTimelineQuery,
    entry: &SurfaceObjectTimelineEntry,
) -> bool {
    viewer_can_read(query.viewer, &entry.visibility)
        && query
            .object_kind
            .as_deref()
            .is_none_or(|object_kind| object_kind == entry.object_kind)
        && query
            .object_id
            .as_deref()
            .is_none_or(|object_id| object_id == entry.object_id)
}

fn entry_for_viewer(
    viewer: SurfaceWorkItemViewer,
    mut entry: SurfaceObjectTimelineEntry,
) -> SurfaceObjectTimelineEntry {
    if matches!(
        viewer,
        SurfaceWorkItemViewer::Public | SurfaceWorkItemViewer::Member
    ) {
        entry.operational_context = json!({});
    }
    entry
}

fn viewer_can_read(viewer: SurfaceWorkItemViewer, visibility: &str) -> bool {
    match viewer {
        SurfaceWorkItemViewer::Public => visibility == "public",
        SurfaceWorkItemViewer::Member => matches!(visibility, "public" | "authenticated"),
        SurfaceWorkItemViewer::Staff => matches!(visibility, "public" | "authenticated" | "staff"),
        SurfaceWorkItemViewer::Owner | SurfaceWorkItemViewer::System => true,
    }
}

fn safe_job_event_summary(event_type: &str, status: &str, task_key: Option<&str>) -> String {
    match task_key {
        Some(task_key) => {
            format!("Task `{task_key}` recorded `{event_type}` while the job is {status}.")
        }
        None => format!("Job recorded `{event_type}` and is now {status}."),
    }
}

fn staff_safe_event_payload(payload: Value) -> Value {
    match payload {
        Value::Object(map) => {
            let mut safe = serde_json::Map::new();
            for (key, value) in map {
                if is_forbidden_context_key(&key) {
                    continue;
                }
                safe.insert(key, scrub_context_value(value));
            }
            Value::Object(safe)
        }
        _ => json!({}),
    }
}

fn scrub_context_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut safe = serde_json::Map::new();
            for (key, value) in map {
                if is_forbidden_context_key(&key) {
                    continue;
                }
                safe.insert(key, scrub_context_value(value));
            }
            Value::Object(safe)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(scrub_context_value).collect()),
        other => other,
    }
}

fn is_forbidden_context_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    [
        "private",
        "secret",
        "prompt",
        "provider",
        "policy",
        "staffrouting",
        "owneronly",
        "privateartifacttext",
        "compiledplanprivateinputs",
        "taskresultprivatepayload",
        "raw",
        "unsupportedclaim",
    ]
    .iter()
    .any(|forbidden| normalized.contains(forbidden))
}

fn parse_json_object(value: &str) -> Value {
    serde_json::from_str(value).unwrap_or_else(|_| json!({}))
}

fn parse_string_vec(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

fn append_source_ref(
    mut evidence_refs: Vec<String>,
    source_kind: Option<String>,
    source_id: Option<String>,
) -> Vec<String> {
    if let (Some(kind), Some(id)) = (source_kind, source_id) {
        evidence_refs.push(format!("{kind}:{id}"));
    }
    evidence_refs
}

fn visibility_from_ceiling(visibility: &str) -> &str {
    match visibility {
        "public" => "public",
        "authenticated" | "member" => "authenticated",
        "staff" => "staff",
        "owner" | "system" => "owner",
        _ => "staff",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;

    const NOW: &str = "2026-05-14T12:00:00Z";

    #[test]
    fn rebuild_projects_mixed_canonical_and_event_history_in_stable_order() {
        let mut connection = setup_connection();
        insert_timeline_fixture(&connection);

        let projected = rebuild_surface_object_timeline(&mut connection).unwrap();
        assert!(projected >= 6);

        let job_entries = load_surface_object_timeline(
            &connection,
            SurfaceObjectTimelineQuery {
                viewer: SurfaceWorkItemViewer::Staff,
                object_kind: Some("job".to_string()),
                object_id: Some("job_1".to_string()),
                ..SurfaceObjectTimelineQuery::default()
            },
        )
        .unwrap()
        .entries;
        assert_eq!(
            entry_keys(&job_entries),
            vec![
                "job_event:event_job_task_succeeded".to_string(),
                "job_event:event_job_created".to_string(),
            ]
        );
        assert_eq!(
            job_entries[0].summary,
            "Task `task_a` recorded `task.succeeded` while the job is running."
        );
        assert!(job_entries[0]
            .evidence_refs
            .contains(&"job_event:event_job_task_succeeded".to_string()));

        let all_entries = load_surface_object_timeline(
            &connection,
            SurfaceObjectTimelineQuery {
                viewer: SurfaceWorkItemViewer::Staff,
                ..SurfaceObjectTimelineQuery::default()
            },
        )
        .unwrap()
        .entries;
        for expected in ["job_event", "artifact", "handoff_event", "reward_event"] {
            assert!(
                all_entries
                    .iter()
                    .any(|entry| entry.source_kind == expected),
                "missing timeline source kind {expected}: {all_entries:#?}"
            );
        }
    }

    #[test]
    fn member_view_filters_staff_history_and_scrubs_authenticated_artifact_context() {
        let mut connection = setup_connection();
        insert_timeline_fixture(&connection);
        rebuild_surface_object_timeline(&mut connection).unwrap();

        let member_entries = load_surface_object_timeline(
            &connection,
            SurfaceObjectTimelineQuery {
                viewer: SurfaceWorkItemViewer::Member,
                ..SurfaceObjectTimelineQuery::default()
            },
        )
        .unwrap()
        .entries;

        assert!(member_entries
            .iter()
            .any(|entry| entry.source_kind == "artifact"
                && entry.object_id == "artifact_member_safe"));
        assert!(!member_entries
            .iter()
            .any(|entry| entry.source_kind == "job_event"));
        assert!(!member_entries
            .iter()
            .any(|entry| entry.source_kind == "handoff_event"));
        assert!(!member_entries
            .iter()
            .any(|entry| entry.source_kind == "reward_event"));

        let serialized = serde_json::to_string(&member_entries).unwrap();
        assert!(!serialized.contains("actor_staff"));
        assert!(!serialized.contains("providerSecret"));
        assert!(!serialized.contains("rawPrompt"));
        assert!(!serialized.contains("private artifact text"));
        assert!(member_entries
            .iter()
            .all(|entry| entry.operational_context == json!({})));
    }

    #[test]
    fn rebuild_is_idempotent_and_removes_stale_timeline_rows() {
        let mut connection = setup_connection();
        insert_timeline_fixture(&connection);

        rebuild_surface_object_timeline(&mut connection).unwrap();
        let first = load_surface_object_timeline(
            &connection,
            SurfaceObjectTimelineQuery {
                viewer: SurfaceWorkItemViewer::Owner,
                ..SurfaceObjectTimelineQuery::default()
            },
        )
        .unwrap()
        .entries;

        rebuild_surface_object_timeline(&mut connection).unwrap();
        let second = load_surface_object_timeline(
            &connection,
            SurfaceObjectTimelineQuery {
                viewer: SurfaceWorkItemViewer::Owner,
                ..SurfaceObjectTimelineQuery::default()
            },
        )
        .unwrap()
        .entries;
        assert_eq!(entry_keys(&first), entry_keys(&second));

        connection
            .execute("DELETE FROM reward_events WHERE id = 'reward_event_1'", [])
            .unwrap();
        rebuild_surface_object_timeline(&mut connection).unwrap();
        let after_delete = load_surface_object_timeline(
            &connection,
            SurfaceObjectTimelineQuery {
                viewer: SurfaceWorkItemViewer::Owner,
                ..SurfaceObjectTimelineQuery::default()
            },
        )
        .unwrap()
        .entries;
        assert!(!after_delete.iter().any(
            |entry| entry.source_kind == "reward_event" && entry.source_id == "reward_event_1"
        ));
    }

    fn setup_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        connection
    }

    fn entry_keys(entries: &[SurfaceObjectTimelineEntry]) -> Vec<String> {
        entries
            .iter()
            .map(|entry| format!("{}:{}", entry.source_kind, entry.source_id))
            .collect()
    }

    fn insert_timeline_fixture(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO process_templates (
                    id, capability_id, kind, name, version, description, tasks_json,
                    created_at, updated_at
                 ) VALUES (
                    'studio.timeline', 'studio.timeline', 'studio.timeline', 'Timeline',
                    1, 'timeline test', '[]', ?1, ?1
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO jobs (
                    id, template_id, template_version, capability_id, kind, status, origin,
                    actor_id, input_json, created_at, updated_at
                 ) VALUES (
                    'job_1', 'studio.timeline', 1, 'studio.timeline', 'studio.timeline',
                    'running', 'test', 'actor_member_1', '{\"rawPrompt\":\"do not leak\"}',
                    '2026-05-14T12:00:00Z', '2026-05-14T12:02:00Z'
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO job_tasks (
                    id, job_id, task_key, capability_id, task_kind, label, required, status,
                    input_json, retry_policy_json, output_json, attempt_count, started_at,
                    completed_at, created_at, updated_at, error_message
                 ) VALUES (
                    'task_1', 'job_1', 'task_a', 'studio.timeline', 'studio.timeline',
                    'Task A', 1, 'succeeded', '{}', '{}', NULL, 1,
                    '2026-05-14T12:01:00Z', '2026-05-14T12:02:00Z',
                    '2026-05-14T12:00:00Z', '2026-05-14T12:02:00Z', NULL
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO job_events (
                    id, job_id, task_key, sequence, event_type, payload_json, created_at
                 ) VALUES (
                    'event_job_created', 'job_1', NULL, 1, 'job.created',
                    '{\"rawPrompt\":\"do not leak\",\"providerSecret\":\"sk-live\"}',
                    '2026-05-14T12:00:00Z'
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO job_events (
                    id, job_id, task_key, sequence, event_type, payload_json, created_at
                 ) VALUES (
                    'event_job_task_succeeded', 'job_1', 'task_a', 2, 'task.succeeded',
                    '{\"taskKey\":\"task_a\",\"safe\":\"ok\"}',
                    '2026-05-14T12:02:00Z'
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO artifacts (
                    id, artifact_kind, title, status, visibility_ceiling, summary, source_kind,
                    source_id, evidence_refs_json, provenance_json, content_hash, health_status,
                    created_by_job_id, created_at, updated_at
                 ) VALUES (
                    'artifact_member_safe', 'studio.storyboard', 'Storyboard', 'ready',
                    'authenticated', 'Member-safe storyboard summary.', 'job', 'job_1',
                    '[\"job:job_1\"]',
                    '{\"privateArtifactText\":\"private artifact text\",\"rawPrompt\":\"do not leak\"}',
                    'sha256:storyboard', 'available', 'job_1',
                    '2026-05-14T12:03:00Z', '2026-05-14T12:03:00Z'
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO handoff_inbox_items (
                    id, source_kind, source_id, destination_kind, destination_id, request_json,
                    evidence_json, approval_requirement, delivery_state, created_at, updated_at,
                    reason, requested_action, urgency, assignee_actor_id, due_at,
                    next_action_hint, evidence_refs_json, visibility
                 ) VALUES (
                    'handoff_1', 'conversation', 'conversation_1', 'staff', 'keith',
                    '{\"message\":\"help\",\"rawPrompt\":\"do not leak\"}',
                    '{\"staffRouting\":\"keith\",\"providerSecret\":\"do not leak\"}',
                    'owner_approval_required', 'pending_owner_approval',
                    '2026-05-14T12:04:00Z', '2026-05-14T12:04:00Z',
                    'Trial user asked for strategy help', 'Review request', 'high',
                    'actor_staff', NULL, 'Review brief first',
                    '[\"conversation:conversation_1\"]', 'staff'
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO handoff_events (
                    id, handoff_item_id, event_type, payload_json, occurred_at
                 ) VALUES (
                    'handoff_event_1', 'handoff_1', 'handoff.inbox.created',
                    '{\"actorId\":\"actor_staff\",\"staffRouting\":\"keith\"}',
                    '2026-05-14T12:04:00Z'
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO reward_programs (
                    id, slug, name, status, visibility, terms_json, policy_json, created_at, updated_at
                 ) VALUES (
                    'reward_program_1', 'pilot', 'Pilot Rewards', 'active', 'staff', '{}', '{}',
                    '2026-05-14T12:05:00Z', '2026-05-14T12:05:00Z'
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO reward_rules (
                    id, program_id, trigger_kind, status, benefit_kind, benefit_quantity,
                    benefit_unit, max_quantity_per_actor, qualification_policy_json,
                    created_at, updated_at
                 ) VALUES (
                    'reward_rule_1', 'reward_program_1', 'feedback_accepted', 'active',
                    'hosted_days', 7, 'day', 30, '{}',
                    '2026-05-14T12:05:00Z', '2026-05-14T12:05:00Z'
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO reward_events (
                    id, program_id, rule_id, actor_id, connection_id, source_kind,
                    source_id, state, idempotency_key, reason, evidence_refs_json,
                    provenance_json, created_at, updated_at
                 ) VALUES (
                    'reward_event_1', 'reward_program_1', 'reward_rule_1', 'actor_member_1',
                    'connection_1', 'feedback_request', 'feedback_request_1', 'qualified',
                    'reward-key-1', 'Accepted feedback', '[\"feedback_request:feedback_request_1\"]',
                    '{}', '2026-05-14T12:05:00Z', '2026-05-14T12:05:00Z'
                 )",
                [],
            )
            .unwrap();
    }
}
