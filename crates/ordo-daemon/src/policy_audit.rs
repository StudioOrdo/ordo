use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

const DEFAULT_POLICY_DECISION_LIMIT: usize = 50;
const MAX_POLICY_DECISION_LIMIT: usize = 250;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyDecisionAuditEntry {
    pub id: String,
    pub decided_at: String,
    pub actor: PolicyDecisionAuditActor,
    pub action: String,
    pub resource: PolicyDecisionAuditResource,
    pub capability_id: Option<String>,
    pub outcome: String,
    pub reason: String,
    pub request_id: Option<String>,
    pub job_id: Option<String>,
    pub task_key: Option<String>,
    pub artifact_id: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyDecisionAuditActor {
    pub kind: String,
    pub id: Option<String>,
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyDecisionAuditResource {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyDecisionAuditQuery {
    pub outcome: Option<String>,
    pub actor_kind: Option<String>,
    pub capability_id: Option<String>,
    pub resource_kind: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyDecisionAuditResponse {
    pub decisions: Vec<PolicyDecisionAuditEntry>,
}

pub fn list_policy_decisions(
    db_path: &Path,
    query: PolicyDecisionAuditQuery,
) -> Result<PolicyDecisionAuditResponse> {
    let connection = Connection::open(db_path)?;
    Ok(PolicyDecisionAuditResponse {
        decisions: query_policy_decisions(&connection, &query)?,
    })
}

pub fn query_policy_decisions(
    connection: &Connection,
    query: &PolicyDecisionAuditQuery,
) -> Result<Vec<PolicyDecisionAuditEntry>> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_POLICY_DECISION_LIMIT)
        .min(MAX_POLICY_DECISION_LIMIT);
    let mut statement = connection.prepare(
        "SELECT id, decided_at, actor_kind, actor_id, actor_origin, action,
                resource_kind, resource_id, capability_id, outcome, reason, request_id,
                job_id, task_key, artifact_id, metadata_json
         FROM policy_decisions
         WHERE (?1 IS NULL OR outcome = ?1)
           AND (?2 IS NULL OR actor_kind = ?2)
           AND (?3 IS NULL OR capability_id = ?3)
           AND (?4 IS NULL OR resource_kind = ?4)
         ORDER BY decided_at DESC, id DESC
         LIMIT ?5",
    )?;
    let rows = statement.query_map(
        params![
            query.outcome.as_deref(),
            query.actor_kind.as_deref(),
            query.capability_id.as_deref(),
            query.resource_kind.as_deref(),
            limit as i64,
        ],
        policy_decision_from_row,
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn policy_decision_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PolicyDecisionAuditEntry> {
    let metadata_json: String = row.get(15)?;
    Ok(PolicyDecisionAuditEntry {
        id: row.get(0)?,
        decided_at: row.get(1)?,
        actor: PolicyDecisionAuditActor {
            kind: row.get(2)?,
            id: row.get(3)?,
            origin: row.get(4)?,
        },
        action: row.get(5)?,
        resource: PolicyDecisionAuditResource {
            kind: row.get(6)?,
            id: row.get(7)?,
        },
        capability_id: row.get(8)?,
        outcome: row.get(9)?,
        reason: row.get(10)?,
        request_id: row.get(11)?,
        job_id: row.get(12)?,
        task_key: row.get(13)?,
        artifact_id: row.get(14)?,
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{
        record_policy_decision, ActorContext, PolicyAction, PolicyDecision,
        PolicyDecisionCorrelation, PolicyOutcome, ResourceKind, ResourceRef,
    };
    use crate::schema::init_schema;

    fn decision(outcome: PolicyOutcome, capability_id: &str) -> PolicyDecision {
        PolicyDecision {
            outcome,
            actor: ActorContext::mcp_client(),
            action: PolicyAction::CallTool,
            resource: ResourceRef::new(ResourceKind::Capability, capability_id),
            capability_id: Some(capability_id.to_string()),
            reason: format!("{capability_id} decision"),
        }
    }

    #[test]
    fn queries_allowed_denied_and_review_required_decisions_newest_first() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        record_policy_decision(
            &connection,
            &decision(PolicyOutcome::Allowed, "system.status.read"),
            PolicyDecisionCorrelation::default(),
        )
        .unwrap();
        record_policy_decision(
            &connection,
            &decision(PolicyOutcome::Denied, "backup.create"),
            PolicyDecisionCorrelation::default(),
        )
        .unwrap();
        record_policy_decision(
            &connection,
            &decision(PolicyOutcome::ReviewRequired, "restore.preflight.validate"),
            PolicyDecisionCorrelation::default(),
        )
        .unwrap();

        let decisions = query_policy_decisions(
            &connection,
            &PolicyDecisionAuditQuery {
                outcome: None,
                actor_kind: None,
                capability_id: None,
                resource_kind: None,
                limit: Some(10),
            },
        )
        .unwrap();

        assert_eq!(decisions.len(), 3);
        assert!(decisions.iter().any(|entry| entry.outcome == "allowed"));
        assert!(decisions.iter().any(|entry| entry.outcome == "denied"));
        assert!(decisions
            .iter()
            .any(|entry| entry.outcome == "review_required"));
    }

    #[test]
    fn filters_policy_decisions_by_supported_fields() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        record_policy_decision(
            &connection,
            &decision(PolicyOutcome::Allowed, "system.status.read"),
            PolicyDecisionCorrelation::default(),
        )
        .unwrap();
        record_policy_decision(
            &connection,
            &decision(PolicyOutcome::ReviewRequired, "restore.preflight.validate"),
            PolicyDecisionCorrelation::default(),
        )
        .unwrap();

        let decisions = query_policy_decisions(
            &connection,
            &PolicyDecisionAuditQuery {
                outcome: Some("review_required".to_string()),
                actor_kind: Some("mcp_client".to_string()),
                capability_id: Some("restore.preflight.validate".to_string()),
                resource_kind: Some("capability".to_string()),
                limit: Some(10),
            },
        )
        .unwrap();

        assert_eq!(decisions.len(), 1);
        assert_eq!(
            decisions[0].capability_id.as_deref(),
            Some("restore.preflight.validate")
        );
        assert_eq!(decisions[0].actor.kind, "mcp_client");
        assert_eq!(decisions[0].resource.kind, "capability");
    }

    #[test]
    fn bounds_policy_decision_query_limit() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        for index in 0..300 {
            record_policy_decision(
                &connection,
                &decision(PolicyOutcome::Allowed, &format!("capability.{index}")),
                PolicyDecisionCorrelation::default(),
            )
            .unwrap();
        }

        let decisions = query_policy_decisions(
            &connection,
            &PolicyDecisionAuditQuery {
                outcome: None,
                actor_kind: None,
                capability_id: None,
                resource_kind: None,
                limit: Some(1_000),
            },
        )
        .unwrap();

        assert_eq!(decisions.len(), MAX_POLICY_DECISION_LIMIT);
    }
}
