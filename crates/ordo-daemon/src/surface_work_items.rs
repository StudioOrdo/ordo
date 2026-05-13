use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceWorkItemViewer {
    Public,
    Member,
    Staff,
    Owner,
    System,
}

impl Default for SurfaceWorkItemViewer {
    fn default() -> Self {
        Self::Member
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SurfaceWorkItemQuery {
    pub viewer: SurfaceWorkItemViewer,
    pub surface_kind: Option<String>,
    pub room_kind: Option<String>,
    pub actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceWorkItemListResponse {
    pub items: Vec<SurfaceWorkItemView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceWorkItemView {
    pub id: String,
    pub surface_kind: String,
    pub room_kind: String,
    pub source_kind: String,
    pub source_id: String,
    pub object_kind: String,
    pub object_id: String,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub priority: i64,
    pub actor_context: Value,
    pub connection_context: Value,
    pub evidence_refs: Vec<String>,
    pub actions: Vec<String>,
    pub visibility: String,
    pub created_at: String,
    pub updated_at: String,
    pub projected_at: String,
}

pub fn list_surface_work_items(
    db_path: &Path,
    query: SurfaceWorkItemQuery,
) -> Result<SurfaceWorkItemListResponse> {
    let mut connection = Connection::open(db_path)?;
    rebuild_surface_work_items(&mut connection)?;
    load_surface_work_items(&connection, query)
}

pub fn rebuild_surface_work_items(connection: &mut Connection) -> Result<usize> {
    let transaction = connection.transaction()?;
    transaction.execute("DELETE FROM surface_work_items", [])?;
    let projected_at = Utc::now().to_rfc3339();
    let mut projected = 0;

    projected += project_offers(&transaction, &projected_at)?;
    projected += project_offer_acceptances(&transaction, &projected_at)?;
    projected += project_trials(&transaction, &projected_at)?;
    projected += project_tracked_entry_points(&transaction, &projected_at)?;
    projected += project_visitor_sessions(&transaction, &projected_at)?;
    projected += project_handoffs(&transaction, &projected_at)?;
    projected += project_feedback_requests(&transaction, &projected_at)?;
    projected += project_jobs(&transaction, &projected_at)?;
    projected += project_artifacts(&transaction, &projected_at)?;
    projected += project_issue_reports(&transaction, &projected_at)?;
    projected += project_resource_grants(&transaction, &projected_at)?;

    transaction.commit()?;
    Ok(projected)
}

pub fn load_surface_work_items(
    connection: &Connection,
    query: SurfaceWorkItemQuery,
) -> Result<SurfaceWorkItemListResponse> {
    let mut statement = connection.prepare(
        "SELECT id, surface_kind, room_kind, source_kind, source_id, object_kind,
                object_id, title, summary, status, priority, actor_context_json,
                connection_context_json, evidence_refs_json, actions_json, visibility,
                created_at, updated_at, projected_at
         FROM surface_work_items
         ORDER BY priority DESC, updated_at DESC, id ASC",
    )?;
    let items = statement
        .query_map([], surface_work_item_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let limit = query.limit.unwrap_or(100).min(500);
    Ok(SurfaceWorkItemListResponse {
        items: items
            .into_iter()
            .filter(|item| query_matches_item(&query, item))
            .take(limit)
            .collect(),
    })
}

#[derive(Debug, Clone)]
struct SurfaceWorkItemInput {
    surface_kind: &'static str,
    room_kind: &'static str,
    source_kind: &'static str,
    source_id: String,
    object_kind: &'static str,
    object_id: String,
    title: String,
    summary: String,
    status: String,
    priority: i64,
    actor_context: Value,
    connection_context: Value,
    evidence_refs: Vec<String>,
    actions: Vec<String>,
    visibility: String,
    created_at: String,
    updated_at: String,
}

fn project_offers(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, slug, title, summary, status, visibility, publication_state,
                created_at, updated_at
         FROM offers",
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
            slug,
            title,
            summary,
            status,
            visibility,
            publication_state,
            created_at,
            updated_at,
        ) = row?;
        insert_item(
            transaction,
            projected_at,
            SurfaceWorkItemInput {
                surface_kind: "growth",
                room_kind: "offers",
                source_kind: "offer",
                source_id: id.clone(),
                object_kind: "offer",
                object_id: id.clone(),
                title: title.clone(),
                summary: format!("Offer `{slug}` is {status} and {publication_state}."),
                status: status.clone(),
                priority: offer_priority(&status, &publication_state),
                actor_context: json!({}),
                connection_context: json!({}),
                evidence_refs: vec![format!("offer:{id}")],
                actions: vec!["inspect_offer".to_string(), "edit_offer".to_string()],
                visibility: visibility.clone(),
                created_at: created_at.clone(),
                updated_at: updated_at.clone(),
            },
        )?;
        projected += 1;

        if status == "available"
            && publication_state == "published"
            && matches!(visibility.as_str(), "public" | "authenticated")
        {
            insert_item(
                transaction,
                projected_at,
                SurfaceWorkItemInput {
                    surface_kind: "member",
                    room_kind: "offers",
                    source_kind: "offer",
                    source_id: id.clone(),
                    object_kind: "offer",
                    object_id: id.clone(),
                    title,
                    summary,
                    status,
                    priority: 35,
                    actor_context: json!({}),
                    connection_context: json!({}),
                    evidence_refs: vec![format!("offer:{id}")],
                    actions: vec!["view_offer".to_string(), "accept_offer".to_string()],
                    visibility,
                    created_at,
                    updated_at,
                },
            )?;
            projected += 1;
        }
    }
    Ok(projected)
}

fn project_offer_acceptances(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, offer_id, offer_slug, offer_title, status, accepted_at, created_at, updated_at
         FROM offer_acceptances",
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
        let (id, offer_id, offer_slug, offer_title, status, accepted_at, created_at, updated_at) =
            row?;
        insert_item(
            transaction,
            projected_at,
            SurfaceWorkItemInput {
                surface_kind: "growth",
                room_kind: "conversions",
                source_kind: "offer_acceptance",
                source_id: id.clone(),
                object_kind: "offer_acceptance",
                object_id: id.clone(),
                title: format!("Accepted: {offer_title}"),
                summary: format!("Offer `{offer_slug}` was accepted at {accepted_at}."),
                status,
                priority: 55,
                actor_context: json!({}),
                connection_context: json!({}),
                evidence_refs: vec![
                    format!("offer_acceptance:{id}"),
                    format!("offer:{offer_id}"),
                ],
                actions: vec!["review_acceptance".to_string()],
                visibility: "staff".to_string(),
                created_at,
                updated_at,
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn project_trials(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, acceptance_id, offer_id, offer_slug, status, trial_ends_at, created_at, updated_at
         FROM trials",
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
            id,
            acceptance_id,
            offer_id,
            offer_slug,
            status,
            trial_ends_at,
            created_at,
            updated_at,
        ) = row?;
        insert_item(
            transaction,
            projected_at,
            SurfaceWorkItemInput {
                surface_kind: "systems",
                room_kind: "trials",
                source_kind: "trial",
                source_id: id.clone(),
                object_kind: "trial",
                object_id: id.clone(),
                title: format!("Trial: {offer_slug}"),
                summary: format!("Trial is {status} and ends at {trial_ends_at}."),
                status: status.clone(),
                priority: trial_priority(&status),
                actor_context: json!({}),
                connection_context: json!({}),
                evidence_refs: vec![
                    format!("trial:{id}"),
                    format!("offer_acceptance:{acceptance_id}"),
                    format!("offer:{offer_id}"),
                ],
                actions: vec!["inspect_trial".to_string()],
                visibility: "staff".to_string(),
                created_at,
                updated_at,
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn project_tracked_entry_points(
    transaction: &Transaction<'_>,
    projected_at: &str,
) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, slug, label, status, source_kind, destination_surface,
                created_at, updated_at
         FROM tracked_entry_points",
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
        let (id, slug, label, status, source_kind, destination_surface, created_at, updated_at) =
            row?;
        insert_item(
            transaction,
            projected_at,
            SurfaceWorkItemInput {
                surface_kind: "growth",
                room_kind: "entry_points",
                source_kind: "tracked_entry_point",
                source_id: id.clone(),
                object_kind: "tracked_entry_point",
                object_id: id.clone(),
                title: label,
                summary: format!(
                    "Tracked entry `{slug}` from {source_kind} routes to {destination_surface}."
                ),
                status: status.clone(),
                priority: if status == "active" { 35 } else { 10 },
                actor_context: json!({}),
                connection_context: json!({}),
                evidence_refs: vec![format!("tracked_entry_point:{id}")],
                actions: vec!["inspect_entry_point".to_string()],
                visibility: "staff".to_string(),
                created_at,
                updated_at,
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn project_visitor_sessions(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, entry_point_id, entry_point_slug, status, destination_surface,
                created_at, updated_at, last_seen_at
         FROM visitor_sessions",
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
            id,
            entry_point_id,
            slug,
            status,
            destination_surface,
            created_at,
            updated_at,
            last_seen_at,
        ) = row?;
        insert_item(
            transaction,
            projected_at,
            SurfaceWorkItemInput {
                surface_kind: "growth",
                room_kind: "visitor_sessions",
                source_kind: "visitor_session",
                source_id: id.clone(),
                object_kind: "visitor_session",
                object_id: id.clone(),
                title: format!("Visitor from {slug}"),
                summary: format!(
                    "Visitor session is {status}; last seen at {last_seen_at}; destination {destination_surface}."
                ),
                status: status.clone(),
                priority: if status == "active" { 50 } else { 15 },
                actor_context: json!({}),
                connection_context: json!({}),
                evidence_refs: vec![
                    format!("visitor_session:{id}"),
                    format!("tracked_entry_point:{entry_point_id}"),
                ],
                actions: vec!["inspect_visitor_session".to_string()],
                visibility: "staff".to_string(),
                created_at,
                updated_at,
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn project_handoffs(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, source_kind, source_id, approval_requirement, delivery_state,
                created_by_actor_id, created_at, updated_at, reason, requested_action,
                urgency, assignee_actor_id, due_at, next_action_hint, evidence_refs_json,
                visibility
         FROM handoff_inbox_items",
    )?;
    let mut projected = 0;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, Option<String>>(11)?,
            row.get::<_, Option<String>>(12)?,
            row.get::<_, Option<String>>(13)?,
            row.get::<_, String>(14)?,
            row.get::<_, String>(15)?,
        ))
    })?;
    for row in rows {
        let (
            id,
            source_kind,
            source_id,
            approval_requirement,
            delivery_state,
            created_by_actor_id,
            created_at,
            updated_at,
            reason,
            requested_action,
            urgency,
            assignee_actor_id,
            due_at,
            next_action_hint,
            evidence_refs_json,
            visibility,
        ) = row?;
        let mut evidence_refs = vec![format!("handoff_inbox_item:{id}")];
        evidence_refs
            .extend(serde_json::from_str::<Vec<String>>(&evidence_refs_json).unwrap_or_default());
        insert_item(
            transaction,
            projected_at,
            SurfaceWorkItemInput {
                surface_kind: "support",
                room_kind: "handoffs",
                source_kind: "handoff_inbox_item",
                source_id: id.clone(),
                object_kind: "handoff_inbox_item",
                object_id: id.clone(),
                title: reason.clone(),
                summary: format!(
                    "A {urgency} priority {source_kind} handoff is {delivery_state}; requested action: {requested_action}."
                ),
                status: delivery_state.clone(),
                priority: handoff_priority(&delivery_state),
                actor_context: json!({
                    "createdByActorId": created_by_actor_id,
                    "assigneeActorId": assignee_actor_id,
                    "sourceKind": source_kind,
                    "sourceId": source_id,
                    "urgency": urgency,
                    "requestedAction": requested_action,
                    "dueAt": due_at,
                    "nextActionHint": next_action_hint,
                    "approvalRequirement": approval_requirement,
                }),
                connection_context: json!({}),
                evidence_refs,
                actions: handoff_actions(&delivery_state),
                visibility,
                created_at,
                updated_at,
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn project_feedback_requests(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut projected = 0;
    let mut statement = transaction.prepare(
        "SELECT id, target_kind, target_id, member_actor_id, connection_id,
                source_kind, source_id, prompt, member_context_summary, status,
                due_at, priority, evidence_refs_json, created_at, updated_at
         FROM feedback_requests",
    )?;
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
            row.get::<_, String>(9)?,
            row.get::<_, Option<String>>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
            row.get::<_, String>(13)?,
            row.get::<_, String>(14)?,
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
            prompt,
            member_context_summary,
            status,
            due_at,
            priority,
            evidence_refs_json,
            created_at,
            updated_at,
        ) = row?;
        let subject_context =
            feedback_member_subject_context(member_actor_id.as_deref(), connection_id.as_deref());
        if !subject_context.is_null() {
            insert_item(
                transaction,
                projected_at,
                SurfaceWorkItemInput {
                    surface_kind: "member",
                    room_kind: "requests",
                    source_kind: "feedback_request",
                    source_id: id.clone(),
                    object_kind: "feedback_request",
                    object_id: id.clone(),
                    title: "Feedback requested".to_string(),
                    summary: member_context_summary.clone(),
                    status: status.clone(),
                    priority: feedback_request_priority(&status, &priority),
                    actor_context: subject_context,
                    connection_context: connection_id
                        .as_deref()
                        .map(|connection_id| json!({ "connectionId": connection_id }))
                        .unwrap_or_else(|| json!({})),
                    evidence_refs: vec![format!("feedback_request:{id}")],
                    actions: feedback_member_actions(&status),
                    visibility: "authenticated".to_string(),
                    created_at: created_at.clone(),
                    updated_at: updated_at.clone(),
                },
            )?;
            projected += 1;
        }
        let evidence_refs = append_source_ref(
            append_source_ref(
                parse_string_vec(&evidence_refs_json),
                Some(target_kind.clone()),
                Some(target_id.clone()),
            ),
            Some(source_kind.clone()),
            source_id.clone(),
        );
        insert_item(
            transaction,
            projected_at,
            SurfaceWorkItemInput {
                surface_kind: "support",
                room_kind: "feedback",
                source_kind: "feedback_request",
                source_id: id.clone(),
                object_kind: "feedback_request",
                object_id: id.clone(),
                title: format!("Feedback request for {target_kind}"),
                summary: format!("Feedback request is {status}; member prompt: {prompt}"),
                status: status.clone(),
                priority: feedback_request_priority(&status, &priority),
                actor_context: json!({
                    "memberActorId": member_actor_id,
                    "connectionId": connection_id,
                    "targetKind": target_kind,
                    "targetId": target_id,
                    "sourceKind": source_kind,
                    "sourceId": source_id,
                    "dueAt": due_at,
                }),
                connection_context: json!({}),
                evidence_refs,
                actions: feedback_support_actions(&status),
                visibility: "staff".to_string(),
                created_at,
                updated_at,
            },
        )?;
        projected += 1;
    }

    let mut statement = transaction.prepare(
        "SELECT id, request_id, response_id, review_id, actor_id, state, reason,
                evidence_refs_json, created_at, updated_at
         FROM feedback_reward_eligibility",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
        ))
    })?;
    for row in rows {
        let (
            id,
            request_id,
            response_id,
            review_id,
            actor_id,
            state,
            reason,
            evidence_refs_json,
            created_at,
            updated_at,
        ) = row?;
        insert_item(
            transaction,
            projected_at,
            SurfaceWorkItemInput {
                surface_kind: "growth",
                room_kind: "rewards",
                source_kind: "feedback_reward_eligibility",
                source_id: id.clone(),
                object_kind: "feedback_reward_eligibility",
                object_id: id.clone(),
                title: "Feedback reward eligibility".to_string(),
                summary: format!(
                    "Feedback reward eligibility is {state}; ledger grant remains deferred."
                ),
                status: state.clone(),
                priority: feedback_reward_priority(&state),
                actor_context: json!({
                    "actorId": actor_id,
                    "requestId": request_id,
                    "responseId": response_id,
                    "reviewId": review_id,
                    "rewardLedgerDeferred": true,
                }),
                connection_context: json!({}),
                evidence_refs: append_source_ref(
                    parse_string_vec(&evidence_refs_json),
                    Some("feedback_request".to_string()),
                    Some(request_id),
                ),
                actions: vec!["inspect_reward_eligibility".to_string()],
                visibility: "staff".to_string(),
                created_at,
                updated_at,
            },
        )?;
        projected += 1;
        let _ = reason;
    }
    Ok(projected)
}

fn project_jobs(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, kind, status, origin, actor_id, created_at, updated_at
         FROM jobs",
    )?;
    let mut projected = 0;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
        ))
    })?;
    for row in rows {
        let (id, kind, status, origin, actor_id, created_at, updated_at) = row?;
        insert_item(
            transaction,
            projected_at,
            SurfaceWorkItemInput {
                surface_kind: "studio",
                room_kind: "runs",
                source_kind: "job",
                source_id: id.clone(),
                object_kind: "job",
                object_id: id.clone(),
                title: format!("Job: {kind}"),
                summary: format!("Job from {origin} is {status}."),
                status: status.clone(),
                priority: job_priority(&status),
                actor_context: json!({ "actorId": actor_id }),
                connection_context: json!({}),
                evidence_refs: vec![format!("job:{id}")],
                actions: vec!["inspect_job".to_string()],
                visibility: "staff".to_string(),
                created_at: created_at.clone(),
                updated_at: updated_at.clone(),
            },
        )?;
        projected += 1;
        insert_item(
            transaction,
            projected_at,
            SurfaceWorkItemInput {
                surface_kind: "systems",
                room_kind: "jobs",
                source_kind: "job",
                source_id: id.clone(),
                object_kind: "job",
                object_id: id.clone(),
                title: format!("Job: {kind}"),
                summary: format!("Operational job state is {status}."),
                status: status.clone(),
                priority: job_priority(&status),
                actor_context: json!({ "actorId": actor_id }),
                connection_context: json!({}),
                evidence_refs: vec![format!("job:{id}")],
                actions: vec!["inspect_job".to_string()],
                visibility: "owner".to_string(),
                created_at,
                updated_at,
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn project_artifacts(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, artifact_kind, title, status, visibility_ceiling, summary, source_kind,
                source_id, evidence_refs_json, health_status, created_by_job_id, created_at, updated_at
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
            evidence_json,
            health_status,
            job_id,
            created_at,
            updated_at,
        ) = row?;
        insert_item(
            transaction,
            projected_at,
            SurfaceWorkItemInput {
                surface_kind: "studio",
                room_kind: "artifacts",
                source_kind: "artifact",
                source_id: id.clone(),
                object_kind: "artifact",
                object_id: id.clone(),
                title,
                summary: format!(
                    "{summary} Health: {}.",
                    health_status.unwrap_or_else(|| "unknown".to_string())
                ),
                status: status.clone(),
                priority: artifact_priority(&status),
                actor_context: json!({ "createdByJobId": job_id }),
                connection_context: json!({}),
                evidence_refs: append_source_ref(
                    parse_string_vec(&evidence_json),
                    source_kind,
                    source_id,
                ),
                actions: vec!["review_artifact".to_string()],
                visibility: visibility_from_ceiling(&visibility_ceiling).to_string(),
                created_at,
                updated_at,
            },
        )?;
        projected += 1;
        let _ = artifact_kind;
    }
    Ok(projected)
}

fn project_issue_reports(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, status, severity, title, summary, created_at, updated_at
         FROM issue_report_artifacts",
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
        ))
    })?;
    for row in rows {
        let (id, status, severity, title, summary, created_at, updated_at) = row?;
        insert_item(
            transaction,
            projected_at,
            SurfaceWorkItemInput {
                surface_kind: "systems",
                room_kind: "reports",
                source_kind: "issue_report",
                source_id: id.clone(),
                object_kind: "issue_report",
                object_id: id.clone(),
                title,
                summary,
                status: status.clone(),
                priority: issue_report_priority(&severity, &status),
                actor_context: json!({}),
                connection_context: json!({}),
                evidence_refs: vec![format!("issue_report:{id}")],
                actions: vec!["review_report".to_string()],
                visibility: "staff".to_string(),
                created_at,
                updated_at,
            },
        )?;
        projected += 1;
    }
    Ok(projected)
}

fn project_resource_grants(transaction: &Transaction<'_>, projected_at: &str) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, resource_kind, resource_id, action, subject_kind, subject_id, effect,
                created_at, COALESCE(expires_at, ''), metadata_json
         FROM resource_grants",
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
            row.get::<_, String>(9)?,
        ))
    })?;
    for row in rows {
        let (
            id,
            resource_kind,
            resource_id,
            action,
            subject_kind,
            subject_id,
            effect,
            created_at,
            expires_at,
            metadata_json,
        ) = row?;
        if effect != "allow" || subject_kind == "role" {
            continue;
        }
        let expires_summary = if expires_at.is_empty() {
            "without an expiration".to_string()
        } else {
            format!("until {expires_at}")
        };
        insert_item(
            transaction,
            projected_at,
            SurfaceWorkItemInput {
                surface_kind: "member",
                room_kind: "access",
                source_kind: "resource_grant",
                source_id: id.clone(),
                object_kind: "resource_grant",
                object_id: id.clone(),
                title: format!("Access: {resource_kind}"),
                summary: format!(
                    "Access grant allows `{action}` on `{resource_kind}` {expires_summary}."
                ),
                status: "active".to_string(),
                priority: 45,
                actor_context: json!({
                    "subjectKind": subject_kind,
                    "subjectId": subject_id,
                }),
                connection_context: connection_context_for_subject(&subject_kind, &subject_id),
                evidence_refs: vec![
                    format!("resource_grant:{id}"),
                    format!("{resource_kind}:{resource_id}"),
                ],
                actions: vec!["inspect_access".to_string()],
                visibility: "authenticated".to_string(),
                created_at: created_at.clone(),
                updated_at: created_at,
            },
        )?;
        projected += 1;
        let _ = metadata_json;
    }
    Ok(projected)
}

fn insert_item(
    transaction: &Transaction<'_>,
    projected_at: &str,
    item: SurfaceWorkItemInput,
) -> Result<()> {
    let id = format!(
        "surface_work_item:{}:{}:{}",
        item.surface_kind, item.source_kind, item.source_id
    );
    transaction.execute(
        "INSERT INTO surface_work_items (
            id, surface_kind, room_kind, source_kind, source_id, object_kind, object_id,
            title, summary, status, priority, actor_context_json, connection_context_json,
            evidence_refs_json, actions_json, visibility, created_at, updated_at, projected_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
            ?16, ?17, ?18, ?19
         )",
        params![
            id,
            item.surface_kind,
            item.room_kind,
            item.source_kind,
            item.source_id,
            item.object_kind,
            item.object_id,
            item.title,
            item.summary,
            item.status,
            item.priority,
            item.actor_context.to_string(),
            item.connection_context.to_string(),
            json!(item.evidence_refs).to_string(),
            json!(item.actions).to_string(),
            item.visibility,
            item.created_at,
            item.updated_at,
            projected_at,
        ],
    )?;
    Ok(())
}

fn surface_work_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SurfaceWorkItemView> {
    let actor_context_json: String = row.get(11)?;
    let connection_context_json: String = row.get(12)?;
    let evidence_refs_json: String = row.get(13)?;
    let actions_json: String = row.get(14)?;
    Ok(SurfaceWorkItemView {
        id: row.get(0)?,
        surface_kind: row.get(1)?,
        room_kind: row.get(2)?,
        source_kind: row.get(3)?,
        source_id: row.get(4)?,
        object_kind: row.get(5)?,
        object_id: row.get(6)?,
        title: row.get(7)?,
        summary: row.get(8)?,
        status: row.get(9)?,
        priority: row.get(10)?,
        actor_context: serde_json::from_str(&actor_context_json).unwrap_or_else(|_| json!({})),
        connection_context: serde_json::from_str(&connection_context_json)
            .unwrap_or_else(|_| json!({})),
        evidence_refs: parse_string_vec(&evidence_refs_json),
        actions: parse_string_vec(&actions_json),
        visibility: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
        projected_at: row.get(18)?,
    })
}

fn query_matches_item(query: &SurfaceWorkItemQuery, item: &SurfaceWorkItemView) -> bool {
    viewer_can_read(query.viewer, &item.visibility)
        && viewer_surface_scope_matches(query.viewer, item)
        && member_subject_scope_matches(query, item)
        && query
            .surface_kind
            .as_deref()
            .is_none_or(|surface| surface == item.surface_kind)
        && query
            .room_kind
            .as_deref()
            .is_none_or(|room| room == item.room_kind)
}

fn viewer_can_read(viewer: SurfaceWorkItemViewer, visibility: &str) -> bool {
    match viewer {
        SurfaceWorkItemViewer::Public => visibility == "public",
        SurfaceWorkItemViewer::Member => matches!(visibility, "public" | "authenticated"),
        SurfaceWorkItemViewer::Staff => matches!(visibility, "public" | "authenticated" | "staff"),
        SurfaceWorkItemViewer::Owner | SurfaceWorkItemViewer::System => true,
    }
}

fn viewer_surface_scope_matches(viewer: SurfaceWorkItemViewer, item: &SurfaceWorkItemView) -> bool {
    match viewer {
        SurfaceWorkItemViewer::Public | SurfaceWorkItemViewer::Member => {
            item.surface_kind == "member"
        }
        SurfaceWorkItemViewer::Staff
        | SurfaceWorkItemViewer::Owner
        | SurfaceWorkItemViewer::System => true,
    }
}

fn member_subject_scope_matches(query: &SurfaceWorkItemQuery, item: &SurfaceWorkItemView) -> bool {
    if query.viewer != SurfaceWorkItemViewer::Member {
        return true;
    }

    match item.source_kind.as_str() {
        "feedback_request" | "resource_grant" => subject_matches_query(query, &item.actor_context),
        _ => true,
    }
}

fn subject_matches_query(query: &SurfaceWorkItemQuery, actor_context: &Value) -> bool {
    let subject_kind = actor_context.get("subjectKind").and_then(Value::as_str);
    let subject_id = actor_context.get("subjectId").and_then(Value::as_str);

    match subject_kind {
        Some("actor") => query
            .actor_id
            .as_deref()
            .is_some_and(|actor_id| subject_id.is_some_and(|subject_id| subject_id == actor_id)),
        Some("connection") => query.connection_id.as_deref().is_some_and(|connection_id| {
            subject_id.is_some_and(|subject_id| subject_id == connection_id)
        }),
        _ => false,
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

fn connection_context_for_subject(subject_kind: &str, subject_id: &str) -> Value {
    if subject_kind == "connection" {
        json!({ "connectionId": subject_id })
    } else {
        json!({})
    }
}

fn visibility_from_ceiling(visibility_ceiling: &str) -> &'static str {
    match visibility_ceiling {
        "public" => "public",
        "authenticated" | "member" | "client" => "authenticated",
        "owner" | "owner_system" => "owner",
        _ => "staff",
    }
}

fn feedback_member_subject_context(
    member_actor_id: Option<&str>,
    connection_id: Option<&str>,
) -> Value {
    if let Some(member_actor_id) = member_actor_id.filter(|value| !value.trim().is_empty()) {
        json!({
            "subjectKind": "actor",
            "subjectId": member_actor_id,
        })
    } else if let Some(connection_id) = connection_id.filter(|value| !value.trim().is_empty()) {
        json!({
            "subjectKind": "connection",
            "subjectId": connection_id,
        })
    } else {
        Value::Null
    }
}

fn feedback_member_actions(status: &str) -> Vec<String> {
    match status {
        "open" | "follow_up_requested" => vec!["answer_feedback_request".to_string()],
        "responded" => vec!["view_feedback_status".to_string()],
        "accepted" => vec!["view_feedback_result".to_string()],
        "rejected" => vec!["view_feedback_result".to_string()],
        _ => vec!["inspect_feedback_request".to_string()],
    }
}

fn feedback_support_actions(status: &str) -> Vec<String> {
    match status {
        "open" => vec![
            "inspect_feedback_request".to_string(),
            "wait_for_member_response".to_string(),
        ],
        "responded" => vec![
            "review_feedback_request".to_string(),
            "accept_feedback".to_string(),
            "reject_feedback".to_string(),
            "request_follow_up".to_string(),
        ],
        "follow_up_requested" => vec!["inspect_follow_up".to_string()],
        _ => vec!["inspect_feedback_request".to_string()],
    }
}

fn feedback_request_priority(status: &str, priority: &str) -> i64 {
    let base = match status {
        "responded" => 78,
        "open" => 58,
        "follow_up_requested" => 62,
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

fn feedback_reward_priority(state: &str) -> i64 {
    match state {
        "pending_qualification" => 72,
        "needs_follow_up" => 55,
        "not_qualified" => 15,
        _ => 35,
    }
}

fn offer_priority(status: &str, publication_state: &str) -> i64 {
    match (status, publication_state) {
        ("available", "published") => 45,
        ("available", _) => 35,
        ("paused", _) => 25,
        ("draft", _) => 20,
        _ => 5,
    }
}

fn trial_priority(status: &str) -> i64 {
    match status {
        "follow_up_needed" => 70,
        "expired" => 60,
        "started" => 40,
        _ => 10,
    }
}

fn handoff_priority(status: &str) -> i64 {
    match status {
        "pending_owner_approval" => 80,
        "queued" => 75,
        "assigned" => 78,
        "continue_screening" => 50,
        "declined" => 20,
        _ => 35,
    }
}

fn handoff_actions(status: &str) -> Vec<String> {
    match status {
        "pending_owner_approval" | "queued" => vec![
            "review_handoff".to_string(),
            "assign_handoff".to_string(),
            "resolve_handoff".to_string(),
            "decline_handoff".to_string(),
        ],
        "assigned" => vec![
            "review_handoff".to_string(),
            "return_to_ordo".to_string(),
            "resolve_handoff".to_string(),
            "decline_handoff".to_string(),
        ],
        "continue_screening" => vec!["return_to_ordo".to_string(), "resolve_handoff".to_string()],
        _ => vec!["inspect_handoff".to_string()],
    }
}

fn job_priority(status: &str) -> i64 {
    match status {
        "failed" => 80,
        "waiting_for_input" => 70,
        "running" => 55,
        "ready" | "pending" => 40,
        "succeeded" => 15,
        _ => 30,
    }
}

fn artifact_priority(status: &str) -> i64 {
    match status {
        "ready" | "ready_for_review" => 55,
        "failed" => 75,
        "published" => 20,
        _ => 35,
    }
}

fn issue_report_priority(severity: &str, status: &str) -> i64 {
    let severity_priority = match severity {
        "critical" => 95,
        "high" => 85,
        "medium" => 65,
        "low" => 40,
        _ => 50,
    };
    if matches!(status, "closed" | "archived") {
        severity_priority.min(20)
    } else {
        severity_priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;
    use rusqlite::params;

    const NOW: &str = "2026-05-13T10:00:00Z";

    #[test]
    fn rebuild_projects_current_canonical_objects_without_future_claims() {
        let mut connection = setup_connection();
        insert_full_pilot_fixture(&connection);

        let projected = rebuild_surface_work_items(&mut connection).unwrap();
        assert!(projected >= 12);

        let items = load_surface_work_items(
            &connection,
            SurfaceWorkItemQuery {
                viewer: SurfaceWorkItemViewer::Staff,
                ..SurfaceWorkItemQuery::default()
            },
        )
        .unwrap()
        .items;

        for expected in [
            "offer",
            "offer_acceptance",
            "trial",
            "tracked_entry_point",
            "visitor_session",
            "handoff_inbox_item",
            "job",
            "artifact",
            "issue_report",
            "resource_grant",
            "feedback_request",
            "feedback_reward_eligibility",
        ] {
            assert!(
                items.iter().any(|item| item.source_kind == expected),
                "missing projected source kind {expected}: {items:#?}"
            );
        }
        assert!(!items.iter().any(|item| item.source_kind == "reward_event"));

        let handoff = items
            .iter()
            .find(|item| item.source_kind == "handoff_inbox_item")
            .unwrap();
        let handoff_json = serde_json::to_string(handoff).unwrap();
        assert!(!handoff_json.contains("providerSecret"));
        assert!(!handoff_json.contains("rawPrompt"));
        assert_eq!(handoff.surface_kind, "support");
        assert_eq!(handoff.visibility, "staff");
        assert_eq!(handoff.title, "Trial user asked for strategy help");
        assert!(handoff.summary.contains("high priority conversation"));
        assert!(handoff
            .actions
            .iter()
            .any(|action| action == "assign_handoff"));
        assert!(handoff
            .evidence_refs
            .iter()
            .any(|reference| reference == "conversation:conversation_1"));
        assert_eq!(handoff.actor_context["assigneeActorId"], "actor_keith");
    }

    #[test]
    fn member_view_omits_staff_only_handoffs_and_owner_role_grants() {
        let mut connection = setup_connection();
        insert_full_pilot_fixture(&connection);
        rebuild_surface_work_items(&mut connection).unwrap();

        let member_items = load_surface_work_items(
            &connection,
            SurfaceWorkItemQuery {
                viewer: SurfaceWorkItemViewer::Member,
                surface_kind: Some("member".to_string()),
                actor_id: Some("actor_member_1".to_string()),
                ..SurfaceWorkItemQuery::default()
            },
        )
        .unwrap()
        .items;

        assert!(member_items
            .iter()
            .any(|item| item.source_kind == "resource_grant"
                && item.source_id == "grant_member_trial"));
        assert!(!member_items
            .iter()
            .any(|item| item.source_kind == "resource_grant"
                && item.source_id == "grant_member_other"));
        assert!(!member_items
            .iter()
            .any(|item| item.source_kind == "handoff_inbox_item"));
        assert!(member_items
            .iter()
            .any(|item| item.source_kind == "feedback_request"
                && item.source_id == "feedback_request_1"
                && item
                    .actions
                    .iter()
                    .any(|action| action == "answer_feedback_request")));
        assert!(!member_items
            .iter()
            .any(|item| item.source_kind == "resource_grant"
                && item.source_id.starts_with("grant_role_owner")));
        assert!(member_items
            .iter()
            .all(|item| matches!(item.visibility.as_str(), "public" | "authenticated")));
    }

    #[test]
    fn member_and_public_reads_are_least_privilege_when_scope_is_missing() {
        let mut connection = setup_connection();
        insert_full_pilot_fixture(&connection);
        rebuild_surface_work_items(&mut connection).unwrap();

        let default_items = load_surface_work_items(&connection, SurfaceWorkItemQuery::default())
            .unwrap()
            .items;
        assert!(default_items
            .iter()
            .all(|item| item.surface_kind == "member"));
        assert!(!default_items
            .iter()
            .any(|item| item.source_kind == "resource_grant"));
        assert!(!default_items
            .iter()
            .any(|item| item.actions.iter().any(|action| action == "edit_offer")));

        let public_items = load_surface_work_items(
            &connection,
            SurfaceWorkItemQuery {
                viewer: SurfaceWorkItemViewer::Public,
                ..SurfaceWorkItemQuery::default()
            },
        )
        .unwrap()
        .items;
        assert!(public_items
            .iter()
            .all(|item| item.surface_kind == "member"));
        assert!(public_items.iter().all(|item| item.visibility == "public"));
        assert!(!public_items
            .iter()
            .any(|item| item.source_kind == "resource_grant"));
    }

    #[test]
    fn rebuild_is_idempotent_and_removes_stale_projection_rows() {
        let mut connection = setup_connection();
        insert_offer_fixture(&connection, "offer_stale", "stale-offer", "available");

        rebuild_surface_work_items(&mut connection).unwrap();
        let first = load_surface_work_items(
            &connection,
            SurfaceWorkItemQuery {
                viewer: SurfaceWorkItemViewer::Owner,
                ..SurfaceWorkItemQuery::default()
            },
        )
        .unwrap()
        .items;

        rebuild_surface_work_items(&mut connection).unwrap();
        let second = load_surface_work_items(
            &connection,
            SurfaceWorkItemQuery {
                viewer: SurfaceWorkItemViewer::Owner,
                ..SurfaceWorkItemQuery::default()
            },
        )
        .unwrap()
        .items;
        assert_eq!(item_keys(&first), item_keys(&second));

        connection
            .execute(
                "UPDATE offers SET status = 'paused', updated_at = ?1 WHERE id = 'offer_stale'",
                [NOW],
            )
            .unwrap();
        rebuild_surface_work_items(&mut connection).unwrap();
        let paused = load_surface_work_items(
            &connection,
            SurfaceWorkItemQuery {
                viewer: SurfaceWorkItemViewer::Owner,
                ..SurfaceWorkItemQuery::default()
            },
        )
        .unwrap()
        .items;
        assert!(paused.iter().any(|item| item.source_kind == "offer"
            && item.source_id == "offer_stale"
            && item.status == "paused"));

        connection
            .execute("DELETE FROM offers WHERE id = 'offer_stale'", [])
            .unwrap();
        rebuild_surface_work_items(&mut connection).unwrap();
        let after_delete = load_surface_work_items(
            &connection,
            SurfaceWorkItemQuery {
                viewer: SurfaceWorkItemViewer::Owner,
                ..SurfaceWorkItemQuery::default()
            },
        )
        .unwrap()
        .items;
        assert!(!after_delete
            .iter()
            .any(|item| item.source_id == "offer_stale"));
    }

    fn setup_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        connection
    }

    fn item_keys(items: &[SurfaceWorkItemView]) -> Vec<(String, String, String)> {
        items
            .iter()
            .map(|item| {
                (
                    item.surface_kind.clone(),
                    item.source_kind.clone(),
                    item.source_id.clone(),
                )
            })
            .collect()
    }

    fn insert_full_pilot_fixture(connection: &Connection) {
        insert_offer_fixture(connection, "offer_pilot", "nyc-pilot", "available");
        connection
            .execute(
                "INSERT INTO tracked_entry_points (
                    id, slug, label, status, source_kind, source_label, destination_surface,
                    destination_id, public_path, qr_payload_json, attribution_json, metadata_json,
                    created_at, updated_at
                 ) VALUES (
                    'entry_nyc', 'nyc', 'NYC meetup QR', 'active', 'campaign', 'NYC Meetup',
                    'offers', 'offer_pilot', '/e/nyc', '{}', '{\"campaign\":\"nyc\"}', '{}',
                    ?1, ?1
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO visitor_sessions (
                    id, entry_point_id, entry_point_slug, status, destination_surface,
                    destination_id, attribution_json, created_at, updated_at, last_seen_at
                 ) VALUES (
                    'visitor_session_1', 'entry_nyc', 'nyc', 'active', 'offers',
                    'offer_pilot', '{\"campaign\":\"nyc\"}', ?1, ?1, ?1
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO offer_acceptances (
                    id, offer_id, offer_slug, offer_title, visitor_session_id, entry_point_id,
                    entry_point_slug, attribution_json, acceptance_context_json, status,
                    accepted_at, created_at, updated_at
                 ) VALUES (
                    'offer_acceptance_1', 'offer_pilot', 'nyc-pilot', 'NYC Pilot',
                    'visitor_session_1', 'entry_nyc', 'nyc', '{\"campaign\":\"nyc\"}',
                    '{\"source\":\"test\"}', 'accepted', ?1, ?1, ?1
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO trials (
                    id, acceptance_id, offer_id, offer_slug, visitor_session_id, status,
                    started_at, trial_ends_at, decision_evidence_json, created_at, updated_at
                 ) VALUES (
                    'trial_1', 'offer_acceptance_1', 'offer_pilot', 'nyc-pilot',
                    'visitor_session_1', 'started', ?1, '2026-06-12T10:00:00Z', '{}', ?1, ?1
                 )",
                [NOW],
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
                    '{\"message\":\"strategy help\",\"rawPrompt\":\"do not leak\"}',
                    '{\"providerSecret\":\"do not leak\"}', 'owner_approval_required',
                    'pending_owner_approval', ?1, ?1, 'Trial user asked for strategy help',
                    'Schedule Keith review', 'high', 'actor_keith', '2026-05-14T15:00:00Z',
                    'Review conversation brief first', '[\"conversation:conversation_1\"]', 'staff'
                 )",
                [NOW],
            )
            .unwrap();
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
                    'What would make Ordo more useful this week?',
                    'Keith asked for feedback on the active NYC pilot trial.',
                    'open', '2026-05-15T10:00:00Z', 'high', 'actor_keith',
                    '[\"handoff:handoff_1\"]', '{\"source\":\"test\"}',
                    '{\"staffNotes\":\"do not leak\"}', ?1, ?1
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO feedback_reward_eligibility (
                    id, request_id, response_id, review_id, actor_id, state, reason,
                    evidence_refs_json, created_at, updated_at
                 ) VALUES (
                    'feedback_reward_eligibility_1', 'feedback_request_1', NULL,
                    NULL, 'actor_member_1', 'pending_qualification',
                    'accepted but reward ledger deferred',
                    '[\"feedback_request_review:feedback_request_review_1\"]', ?1, ?1
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
                    'studio.video.make', 'studio.video.make', 'studio.video.make',
                    'Studio Video', 1, 'test', '[]', ?1, ?1
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
                    'job_1', 'studio.video.make', 1, 'studio.video.make', 'studio.video.make',
                    'running', 'test', 'actor_member_1', '{\"rawPrompt\":\"do not leak\"}', ?1, ?1
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
                    'artifact_1', 'promo.video', 'Promo Video', 'ready', 'staff',
                    'Candidate 30 second promo video.', 'job', 'job_1', '[\"job_1\"]',
                    '{\"rawPrompt\":\"do not leak\"}', 'sha256:test', 'available',
                    'job_1', ?1, ?1
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO issue_report_artifacts (
                    id, status, severity, title, summary, description, markdown_body,
                    diagnostics_json, evidence_json, redactions_json, created_at, updated_at
                 ) VALUES (
                    'report_1', 'ready_for_review', 'medium', 'Pilot issue report',
                    'A report needs review.', 'internal detail', '# Report', '{}',
                    '[\"artifact_1\"]', '[]', ?1, ?1
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO resource_grants (
                    id, resource_kind, resource_id, action, subject_kind, subject_id, effect,
                    created_at, metadata_json
                 ) VALUES (
                    'grant_member_trial', 'offer', 'offer_pilot', 'use', 'actor',
                    'actor_member_1', 'allow', ?1, '{\"reason\":\"accepted_offer\"}'
                 )",
                [NOW],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO resource_grants (
                    id, resource_kind, resource_id, action, subject_kind, subject_id, effect,
                    created_at, metadata_json
                 ) VALUES (
                    'grant_member_other', 'offer', 'offer_pilot', 'use', 'actor',
                    'actor_member_2', 'allow', ?1, '{\"reason\":\"accepted_offer\"}'
                 )",
                [NOW],
            )
            .unwrap();
    }

    fn insert_offer_fixture(connection: &Connection, offer_id: &str, slug: &str, status: &str) {
        connection
            .execute(
                "INSERT INTO offers (
                    id, slug, title, summary, status, visibility, publication_state, trial_days,
                    source_kind, terms_json, metadata_json, created_at, updated_at
                 ) VALUES (
                    ?1, ?2, 'NYC Pilot', 'Thirty day hosted Ordo trial.', ?3, 'public',
                    'published', 30, 'test', '{}', '{}', ?4, ?4
                 )",
                params![offer_id, slug, status, NOW],
            )
            .unwrap();
    }
}
