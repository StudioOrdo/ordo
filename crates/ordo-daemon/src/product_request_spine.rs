use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

use crate::surface_work_items::SurfaceWorkItemViewer;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProductRequestSpineQuery {
    pub viewer: SurfaceWorkItemViewer,
    pub request_kind: Option<String>,
    pub source_kind: Option<String>,
    pub actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductRequestSpineResponse {
    pub requests: Vec<ProductRequestSpineItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductRequestSpineItem {
    pub id: String,
    pub request_kind: String,
    pub source_kind: String,
    pub source_id: String,
    pub object_kind: String,
    pub object_id: String,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub priority: i64,
    pub actor_kind: Option<String>,
    pub actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub visibility: String,
    pub due_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub safe_context: Value,
    pub evidence_refs: Vec<String>,
    pub actions: Vec<String>,
    pub projected_at: String,
}

pub fn list_product_request_spine(
    db_path: &Path,
    query: ProductRequestSpineQuery,
) -> Result<ProductRequestSpineResponse> {
    let mut connection = Connection::open(db_path)?;
    rebuild_product_request_spine(&mut connection)?;
    load_product_request_spine(&connection, query)
}

pub fn rebuild_product_request_spine(connection: &mut Connection) -> Result<usize> {
    let transaction = connection.transaction()?;
    transaction.execute("DELETE FROM product_request_spine", [])?;
    let projected_at = Utc::now().to_rfc3339();
    let mut projected = 0;

    projected += project_feedback_requests(&transaction, &projected_at)?;
    projected += project_handoff_requests(&transaction, &projected_at)?;
    projected += project_artifact_patch_requests(&transaction, &projected_at)?;
    projected += project_job_human_gate_requests(&transaction, &projected_at)?;

    transaction.commit()?;
    Ok(projected)
}

pub fn load_product_request_spine(
    connection: &Connection,
    query: ProductRequestSpineQuery,
) -> Result<ProductRequestSpineResponse> {
    let mut statement = connection.prepare(
        "SELECT id, request_kind, source_kind, source_id, object_kind, object_id,
                title, summary, status, priority, actor_kind, actor_id, connection_id,
                visibility, due_at, created_at, updated_at, safe_context_json,
                evidence_refs_json, actions_json, projected_at
         FROM product_request_spine
         ORDER BY priority DESC, COALESCE(due_at, updated_at) ASC, updated_at DESC, id ASC",
    )?;
    let items = statement
        .query_map([], product_request_spine_item_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let limit = query.limit.unwrap_or(100).min(500);
    Ok(ProductRequestSpineResponse {
        requests: items
            .into_iter()
            .filter(|item| query_matches_item(&query, item))
            .take(limit)
            .map(|item| item_for_viewer(query.viewer, item))
            .collect(),
    })
}

#[derive(Debug, Clone)]
struct ProductRequestInput {
    request_kind: &'static str,
    source_kind: &'static str,
    source_id: String,
    object_kind: &'static str,
    object_id: String,
    title: String,
    summary: String,
    status: String,
    priority: i64,
    actor_kind: Option<&'static str>,
    actor_id: Option<String>,
    connection_id: Option<String>,
    visibility: String,
    due_at: Option<String>,
    created_at: String,
    updated_at: String,
    safe_context: Value,
    evidence_refs: Vec<String>,
    actions: Vec<String>,
}

fn project_feedback_requests(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, target_kind, target_id, member_actor_id, connection_id, source_kind,
                source_id, member_context_summary, status, due_at, priority,
                evidence_refs_json, created_at, updated_at
         FROM feedback_requests",
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
            row.get::<_, Option<String>>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, Option<String>>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
            row.get::<_, String>(13)?,
        ))
    })?;
    for row in rows {
        let (
            id,
            target_kind,
            target_id,
            member_actor_id,
            connection_id,
            source_kind,
            source_id,
            member_context_summary,
            status,
            due_at,
            priority,
            evidence_refs_json,
            created_at,
            updated_at,
        ) = row?;
        insert_request(
            transaction,
            projected_at,
            ProductRequestInput {
                request_kind: "feedback",
                source_kind: "feedback_request",
                source_id: id.clone(),
                object_kind: "feedback_request",
                object_id: id.clone(),
                title: "Feedback request".to_string(),
                summary: member_context_summary,
                status: status.clone(),
                priority: feedback_priority(&status, &priority),
                actor_kind: member_actor_id.as_ref().map(|_| "actor"),
                actor_id: member_actor_id,
                connection_id,
                visibility: "authenticated".to_string(),
                due_at,
                created_at,
                updated_at,
                safe_context: json!({
                    "targetKind": target_kind,
                    "targetId": target_id,
                    "sourceKind": source_kind,
                    "sourceId": source_id,
                }),
                evidence_refs: append_source_ref(
                    append_source_ref(
                        parse_string_vec(&evidence_refs_json),
                        Some("feedback_request".to_string()),
                        Some(id),
                    ),
                    Some(target_kind),
                    Some(target_id),
                ),
                actions: feedback_actions(&status),
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn project_handoff_requests(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, source_kind, source_id, delivery_state, created_by_actor_id,
                created_at, updated_at, reason, requested_action, urgency, due_at,
                evidence_refs_json, visibility
         FROM handoff_inbox_items",
    )?;
    let mut projected = 0;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
        ))
    })?;
    for row in rows {
        let (
            id,
            source_kind,
            source_id,
            delivery_state,
            created_by_actor_id,
            created_at,
            updated_at,
            reason,
            requested_action,
            urgency,
            due_at,
            evidence_refs_json,
            visibility,
        ) = row?;
        insert_request(
            transaction,
            projected_at,
            ProductRequestInput {
                request_kind: "support_handoff",
                source_kind: "handoff_inbox_item",
                source_id: id.clone(),
                object_kind: "handoff_inbox_item",
                object_id: id.clone(),
                title: reason,
                summary: format!(
                    "Support handoff is {delivery_state}; requested action is {requested_action}."
                ),
                status: delivery_state.clone(),
                priority: handoff_priority(&delivery_state, &urgency),
                actor_kind: created_by_actor_id.as_ref().map(|_| "actor"),
                actor_id: created_by_actor_id,
                connection_id: None,
                visibility,
                due_at,
                created_at,
                updated_at,
                safe_context: json!({
                    "sourceKind": source_kind,
                    "sourceId": source_id,
                    "urgency": urgency,
                }),
                evidence_refs: append_source_ref(
                    append_source_ref(
                        parse_string_vec(&evidence_refs_json),
                        Some("handoff_inbox_item".to_string()),
                        Some(id),
                    ),
                    Some(source_kind),
                    source_id,
                ),
                actions: handoff_actions(&delivery_state),
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn project_artifact_patch_requests(
    transaction: &Transaction<'_>,
    projected_at: &str,
) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT p.id, p.source_artifact_id, p.review_state, p.proposed_by_actor_id,
                p.evidence_refs_json, p.created_at, p.updated_at, a.title, a.visibility_ceiling
         FROM artifact_patch_proposals p
         JOIN artifacts a ON a.id = p.source_artifact_id",
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
            row.get::<_, String>(8)?,
        ))
    })?;
    for row in rows {
        let (
            id,
            artifact_id,
            review_state,
            proposed_by_actor_id,
            evidence_refs_json,
            created_at,
            updated_at,
            artifact_title,
            visibility_ceiling,
        ) = row?;
        insert_request(
            transaction,
            projected_at,
            ProductRequestInput {
                request_kind: "artifact_review",
                source_kind: "artifact_patch_proposal",
                source_id: id.clone(),
                object_kind: "artifact",
                object_id: artifact_id.clone(),
                title: format!("Review artifact patch for {artifact_title}"),
                summary: format!("Artifact patch proposal is {review_state}."),
                status: review_state.clone(),
                priority: artifact_review_priority(&review_state),
                actor_kind: Some("actor"),
                actor_id: Some(proposed_by_actor_id),
                connection_id: None,
                visibility: visibility_from_ceiling(&visibility_ceiling).to_string(),
                due_at: None,
                created_at,
                updated_at,
                safe_context: json!({ "artifactId": artifact_id }),
                evidence_refs: append_source_ref(
                    parse_string_vec(&evidence_refs_json),
                    Some("artifact_patch_proposal".to_string()),
                    Some(id),
                ),
                actions: artifact_review_actions(&review_state),
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn project_job_human_gate_requests(
    transaction: &Transaction<'_>,
    projected_at: &str,
) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT task.id, task.job_id, task.task_key, task.label, task.status,
                task.created_at, task.updated_at, job.actor_id, job.kind
         FROM job_tasks task
         JOIN jobs job ON job.id = task.job_id
         WHERE task.status = 'waiting_for_input'",
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
            row.get::<_, Option<String>>(7)?,
            row.get::<_, String>(8)?,
        ))
    })?;
    for row in rows {
        let (id, job_id, task_key, label, status, created_at, updated_at, actor_id, job_kind) =
            row?;
        insert_request(
            transaction,
            projected_at,
            ProductRequestInput {
                request_kind: "human_gate",
                source_kind: "job_task",
                source_id: id,
                object_kind: "job_task",
                object_id: format!("{job_id}:{task_key}"),
                title: label,
                summary: format!("Job task `{task_key}` is waiting for human input."),
                status,
                priority: 70,
                actor_kind: actor_id.as_ref().map(|_| "actor"),
                actor_id,
                connection_id: None,
                visibility: "staff".to_string(),
                due_at: None,
                created_at,
                updated_at,
                safe_context: json!({
                    "jobId": job_id,
                    "taskKey": task_key,
                    "jobKind": job_kind,
                }),
                evidence_refs: vec![format!("job:{job_id}"), format!("job_task:{task_key}")],
                actions: vec![
                    "inspect_job_task".to_string(),
                    "resolve_human_gate".to_string(),
                ],
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn insert_request(
    transaction: &Transaction<'_>,
    projected_at: &str,
    request: ProductRequestInput,
) -> Result<()> {
    let id = format!(
        "product_request_spine:{}:{}:{}",
        request.request_kind, request.source_kind, request.source_id
    );
    transaction.execute(
        "INSERT INTO product_request_spine (
            id, request_kind, source_kind, source_id, object_kind, object_id,
            title, summary, status, priority, actor_kind, actor_id, connection_id,
            visibility, due_at, created_at, updated_at, safe_context_json,
            evidence_refs_json, actions_json, projected_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21
         )",
        params![
            id,
            request.request_kind,
            request.source_kind,
            request.source_id,
            request.object_kind,
            request.object_id,
            request.title,
            request.summary,
            request.status,
            request.priority,
            request.actor_kind,
            request.actor_id,
            request.connection_id,
            request.visibility,
            request.due_at,
            request.created_at,
            request.updated_at,
            request.safe_context.to_string(),
            json!(request.evidence_refs).to_string(),
            json!(request.actions).to_string(),
            projected_at,
        ],
    )?;
    Ok(())
}

fn product_request_spine_item_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProductRequestSpineItem> {
    let safe_context_json: String = row.get(17)?;
    let evidence_refs_json: String = row.get(18)?;
    let actions_json: String = row.get(19)?;
    Ok(ProductRequestSpineItem {
        id: row.get(0)?,
        request_kind: row.get(1)?,
        source_kind: row.get(2)?,
        source_id: row.get(3)?,
        object_kind: row.get(4)?,
        object_id: row.get(5)?,
        title: row.get(6)?,
        summary: row.get(7)?,
        status: row.get(8)?,
        priority: row.get(9)?,
        actor_kind: row.get(10)?,
        actor_id: row.get(11)?,
        connection_id: row.get(12)?,
        visibility: row.get(13)?,
        due_at: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
        safe_context: serde_json::from_str(&safe_context_json).unwrap_or_else(|_| json!({})),
        evidence_refs: parse_string_vec(&evidence_refs_json),
        actions: parse_string_vec(&actions_json),
        projected_at: row.get(20)?,
    })
}

fn query_matches_item(query: &ProductRequestSpineQuery, item: &ProductRequestSpineItem) -> bool {
    viewer_can_read(query.viewer, &item.visibility)
        && member_subject_scope_matches(query, item)
        && query
            .request_kind
            .as_deref()
            .is_none_or(|request_kind| request_kind == item.request_kind)
        && query
            .source_kind
            .as_deref()
            .is_none_or(|source_kind| source_kind == item.source_kind)
        && query
            .actor_id
            .as_deref()
            .is_none_or(|actor_id| item.actor_id.as_deref() == Some(actor_id))
        && query
            .connection_id
            .as_deref()
            .is_none_or(|connection_id| item.connection_id.as_deref() == Some(connection_id))
}

fn member_subject_scope_matches(
    query: &ProductRequestSpineQuery,
    item: &ProductRequestSpineItem,
) -> bool {
    if query.viewer != SurfaceWorkItemViewer::Member {
        return true;
    }

    if item.visibility != "authenticated" {
        return true;
    }

    query.actor_id.as_deref().is_some_and(|actor_id| {
        item.actor_kind.as_deref() == Some("actor") && item.actor_id.as_deref() == Some(actor_id)
    }) || query
        .connection_id
        .as_deref()
        .is_some_and(|connection_id| item.connection_id.as_deref() == Some(connection_id))
}

fn item_for_viewer(
    viewer: SurfaceWorkItemViewer,
    mut item: ProductRequestSpineItem,
) -> ProductRequestSpineItem {
    if matches!(
        viewer,
        SurfaceWorkItemViewer::Public | SurfaceWorkItemViewer::Member
    ) {
        item.safe_context = json!({});
    }
    item
}

fn viewer_can_read(viewer: SurfaceWorkItemViewer, visibility: &str) -> bool {
    match viewer {
        SurfaceWorkItemViewer::Public => visibility == "public",
        SurfaceWorkItemViewer::Member => matches!(visibility, "public" | "authenticated"),
        SurfaceWorkItemViewer::Staff => matches!(visibility, "public" | "authenticated" | "staff"),
        SurfaceWorkItemViewer::Owner | SurfaceWorkItemViewer::System => true,
    }
}

fn feedback_priority(status: &str, priority: &str) -> i64 {
    let base = match status {
        "responded" => 78,
        "follow_up_requested" => 62,
        "open" => 58,
        "accepted" | "rejected" => 20,
        "expired" => 45,
        _ => 35,
    };
    base + match priority {
        "urgent" | "high" => 15,
        "low" => -10,
        _ => 0,
    }
}

fn handoff_priority(status: &str, urgency: &str) -> i64 {
    let base = match status {
        "pending_owner_approval" => 80,
        "assigned" => 78,
        "queued" => 75,
        "continue_screening" => 50,
        "declined" => 20,
        _ => 35,
    };
    base + match urgency {
        "critical" => 20,
        "high" => 10,
        "low" => -10,
        _ => 0,
    }
}

fn artifact_review_priority(status: &str) -> i64 {
    match status {
        "proposed" => 68,
        "no_op" => 30,
        "accepted" => 20,
        _ => 45,
    }
}

fn feedback_actions(status: &str) -> Vec<String> {
    match status {
        "open" | "follow_up_requested" => vec!["answer_feedback_request".to_string()],
        "responded" => vec!["review_feedback_request".to_string()],
        _ => vec!["inspect_feedback_request".to_string()],
    }
}

fn handoff_actions(status: &str) -> Vec<String> {
    match status {
        "pending_owner_approval" | "queued" | "assigned" => vec!["review_handoff".to_string()],
        "continue_screening" => vec!["return_to_ordo".to_string()],
        _ => vec!["inspect_handoff".to_string()],
    }
}

fn artifact_review_actions(status: &str) -> Vec<String> {
    match status {
        "proposed" => vec!["review_artifact_patch".to_string()],
        _ => vec!["inspect_artifact_patch".to_string()],
    }
}

fn visibility_from_ceiling(visibility: &str) -> &'static str {
    match visibility {
        "public" => "public",
        "authenticated" | "member" | "client" => "authenticated",
        "owner" | "owner_system" => "owner",
        _ => "staff",
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;

    const NOW: &str = "2026-05-14T13:00:00Z";

    #[test]
    fn rebuild_projects_existing_human_input_sources_in_priority_order() {
        let mut connection = setup_connection();
        insert_request_fixture(&connection);

        let projected = rebuild_product_request_spine(&mut connection).unwrap();
        assert_eq!(projected, 4);

        let requests = load_product_request_spine(
            &connection,
            ProductRequestSpineQuery {
                viewer: SurfaceWorkItemViewer::Staff,
                ..ProductRequestSpineQuery::default()
            },
        )
        .unwrap()
        .requests;

        for expected in [
            "feedback_request",
            "handoff_inbox_item",
            "artifact_patch_proposal",
            "job_task",
        ] {
            assert!(
                requests
                    .iter()
                    .any(|request| request.source_kind == expected),
                "missing request source kind {expected}: {requests:#?}"
            );
        }
        assert_eq!(requests[0].request_kind, "support_handoff");
        assert_eq!(requests[0].status, "pending_owner_approval");
        assert!(requests[0]
            .evidence_refs
            .contains(&"handoff_inbox_item:handoff_1".to_string()));
    }

    #[test]
    fn member_view_filters_staff_sources_and_strips_request_context() {
        let mut connection = setup_connection();
        insert_request_fixture(&connection);
        rebuild_product_request_spine(&mut connection).unwrap();

        let unscoped_member_requests = load_product_request_spine(
            &connection,
            ProductRequestSpineQuery {
                viewer: SurfaceWorkItemViewer::Member,
                ..ProductRequestSpineQuery::default()
            },
        )
        .unwrap()
        .requests;
        assert!(unscoped_member_requests.is_empty());

        let member_requests = load_product_request_spine(
            &connection,
            ProductRequestSpineQuery {
                viewer: SurfaceWorkItemViewer::Member,
                actor_id: Some("actor_member_1".to_string()),
                ..ProductRequestSpineQuery::default()
            },
        )
        .unwrap()
        .requests;

        assert_eq!(member_requests.len(), 1);
        assert_eq!(member_requests[0].source_kind, "feedback_request");
        assert_eq!(member_requests[0].visibility, "authenticated");
        assert_eq!(member_requests[0].safe_context, json!({}));

        let serialized = serde_json::to_string(&member_requests).unwrap();
        assert!(!serialized.contains("staffRouting"));
        assert!(!serialized.contains("providerSecret"));
        assert!(!serialized.contains("rawPrompt"));
        assert!(!serialized.contains("ownerOnly"));
        assert!(!serialized.contains("private artifact text"));
    }

    #[test]
    fn rebuild_is_idempotent_and_removes_stale_requests() {
        let mut connection = setup_connection();
        insert_request_fixture(&connection);

        rebuild_product_request_spine(&mut connection).unwrap();
        let first = staff_keys(&connection);

        rebuild_product_request_spine(&mut connection).unwrap();
        assert_eq!(first, staff_keys(&connection));

        connection
            .execute(
                "DELETE FROM artifact_patch_proposals WHERE id = 'patch_1'",
                [],
            )
            .unwrap();
        rebuild_product_request_spine(&mut connection).unwrap();
        let after_delete = staff_keys(&connection);
        assert!(!after_delete
            .iter()
            .any(|key| key == "artifact_patch_proposal:patch_1"));
    }

    fn setup_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        connection
    }

    fn staff_keys(connection: &Connection) -> Vec<String> {
        load_product_request_spine(
            connection,
            ProductRequestSpineQuery {
                viewer: SurfaceWorkItemViewer::Staff,
                ..ProductRequestSpineQuery::default()
            },
        )
        .unwrap()
        .requests
        .iter()
        .map(|request| format!("{}:{}", request.source_kind, request.source_id))
        .collect()
    }

    fn insert_request_fixture(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO feedback_requests (
                    id, target_kind, target_id, member_actor_id, connection_id,
                    conversation_id, source_kind, source_id, prompt, member_context_summary,
                    status, due_at, priority, created_by_actor_id, evidence_refs_json,
                    provenance_json, staff_context_json, created_at, updated_at
                 ) VALUES (
                    'feedback_request_1', 'trial', 'trial_1', 'actor_member_1', 'connection_1',
                    NULL, 'support', 'handoff_1',
                    'What would make Ordo more useful?',
                    'Feedback requested for the NYC pilot trial.',
                    'open', '2026-05-15T10:00:00Z', 'high', 'actor_staff',
                    '[\"trial:trial_1\"]', '{\"rawPrompt\":\"do not leak\"}',
                    '{\"staffRouting\":\"keith\",\"providerSecret\":\"do not leak\"}', ?1, ?1
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO handoff_inbox_items (
                    id, source_kind, source_id, destination_kind, destination_id, request_json,
                    evidence_json, approval_requirement, delivery_state, created_by_actor_id,
                    created_at, updated_at, reason, requested_action, urgency, assignee_actor_id,
                    due_at, next_action_hint, evidence_refs_json, visibility
                 ) VALUES (
                    'handoff_1', 'conversation', 'conversation_1', 'staff', 'keith',
                    '{\"message\":\"strategy help\",\"rawPrompt\":\"do not leak\"}',
                    '{\"staffRouting\":\"keith\",\"providerSecret\":\"do not leak\"}',
                    'owner_approval_required', 'pending_owner_approval', 'actor_local_owner',
                    ?1, ?1, 'Trial user asked for strategy help', 'Schedule Keith review',
                    'high', 'actor_keith', '2026-05-14T15:00:00Z',
                    'Review conversation brief first', '[\"conversation:conversation_1\"]', 'staff'
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO process_templates (
                    id, capability_id, kind, name, version, description, tasks_json,
                    created_at, updated_at
                 ) VALUES (
                    'studio.request', 'studio.request', 'studio.request', 'Request job',
                    1, 'request test', '[]', ?1, ?1
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
                    'job_1', 'studio.request', 1, 'studio.request', 'studio.request',
                    'running', 'test', 'actor_member_1',
                    '{\"compiledPlanPrivateInputs\":\"do not leak\"}', ?1, ?1
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO job_tasks (
                    id, job_id, task_key, capability_id, task_kind, label, required, status,
                    input_json, retry_policy_json, output_json, attempt_count, created_at, updated_at
                 ) VALUES (
                    'task_waiting_1', 'job_1', 'human_review', 'studio.request',
                    'studio.human_review', 'Human review needed', 1, 'waiting_for_input',
                    '{\"taskResultPrivatePayload\":\"do not leak\"}', '{}', NULL, 0, ?1, ?1
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO artifacts (
                    id, artifact_kind, title, status, visibility_ceiling, summary, source_kind,
                    source_id, evidence_refs_json, provenance_json, content_hash, health_status,
                    created_by_job_id, created_at, updated_at
                 ) VALUES (
                    'artifact_1', 'studio.copy', 'Landing Copy', 'ready', 'staff',
                    'Copy ready for review.', 'job', 'job_1', '[\"job:job_1\"]',
                    '{\"privateArtifactText\":\"private artifact text\"}', 'sha256:artifact',
                    'available', 'job_1', ?1, ?1
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO artifact_versions (
                    id, artifact_id, version, content_hash, storage_uri, metadata_json, created_at
                 ) VALUES (
                    'artifact_version_1', 'artifact_1', 1, 'sha256:version',
                    'ordo://artifacts/artifact_1/v1', '{\"ownerOnly\":\"do not leak\"}', ?1
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO artifact_patch_proposals (
                    id, source_artifact_id, source_version_id, base_hash, proposed_hash,
                    patch_text, preview_json, evidence_refs_json, provenance_json,
                    review_state, proposed_by_actor_id, created_at, updated_at
                 ) VALUES (
                    'patch_1', 'artifact_1', 'artifact_version_1', 'sha256:base',
                    'sha256:proposed', '--- a\\n+++ b\\n@@\\n-a\\n+b\\n',
                    '{\"changed\":true}', '[\"artifact:artifact_1\"]',
                    '{\"rawPolicyInternals\":\"do not leak\"}', 'proposed',
                    'actor_staff', ?1, ?1
                 )",
                [NOW],
            )
            .unwrap();
    }
}
