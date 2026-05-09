use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::capabilities::{
    CapabilityDefinition, MCP_EXPORT_POLICY_DANGEROUS_NONE, MCP_EXPORT_POLICY_OPERATOR_CONFIRMED,
};

pub const SYSTEM_ACTOR_ID: &str = "actor_system";
pub const LOCAL_OWNER_ACTOR_ID: &str = "actor_local_owner";
pub const SYSTEM_ROLE_ID: &str = "role_system";
pub const OWNER_ROLE_ID: &str = "role_owner";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    System,
    LocalOwner,
    BrowserOperator,
    Scheduler,
    McpClient,
}

impl ActorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::LocalOwner => "local_owner",
            Self::BrowserOperator => "browser_operator",
            Self::Scheduler => "scheduler",
            Self::McpClient => "mcp_client",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorContext {
    pub kind: ActorKind,
    pub id: Option<String>,
    pub origin: String,
}

impl ActorContext {
    pub fn new(kind: ActorKind, origin: impl Into<String>, id: Option<String>) -> Self {
        Self {
            kind,
            id,
            origin: origin.into(),
        }
    }

    pub fn browser_operator() -> Self {
        Self::new(ActorKind::BrowserOperator, "http", None)
    }

    pub fn local_owner(origin: impl Into<String>) -> Self {
        Self::new(
            ActorKind::LocalOwner,
            origin,
            Some(LOCAL_OWNER_ACTOR_ID.to_string()),
        )
    }

    pub fn mcp_client() -> Self {
        Self::new(
            ActorKind::McpClient,
            "mcp",
            Some(LOCAL_OWNER_ACTOR_ID.to_string()),
        )
    }

    pub fn system() -> Self {
        Self::new(
            ActorKind::System,
            "system",
            Some(SYSTEM_ACTOR_ID.to_string()),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    System,
    OwnerSystem,
    PrivateActor,
    DaemonRoute,
    Capability,
    ProcessTemplate,
    Job,
    JobArtifact,
    BriefArtifact,
    IssueReport,
    DiagnosticLog,
    CorpusSource,
    CorpusItem,
    Connection,
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::OwnerSystem => "owner_system",
            Self::PrivateActor => "private_actor",
            Self::DaemonRoute => "daemon_route",
            Self::Capability => "capability",
            Self::ProcessTemplate => "process_template",
            Self::Job => "job",
            Self::JobArtifact => "job_artifact",
            Self::BriefArtifact => "brief_artifact",
            Self::IssueReport => "issue_report",
            Self::DiagnosticLog => "diagnostic_log",
            Self::CorpusSource => "corpus_source",
            Self::CorpusItem => "corpus_item",
            Self::Connection => "connection",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRef {
    pub kind: ResourceKind,
    pub id: String,
}

impl ResourceRef {
    pub fn new(kind: ResourceKind, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    Read,
    Inspect,
    Execute,
    Generate,
    Create,
    Update,
    Validate,
    Prepare,
    Export,
    Approve,
    CallTool,
}

impl PolicyAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Inspect => "inspect",
            Self::Execute => "execute",
            Self::Generate => "generate",
            Self::Create => "create",
            Self::Update => "update",
            Self::Validate => "validate",
            Self::Prepare => "prepare",
            Self::Export => "export",
            Self::Approve => "approve",
            Self::CallTool => "call_tool",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyOutcome {
    Allowed,
    Denied,
    ReviewRequired,
}

impl PolicyOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Denied => "denied",
            Self::ReviewRequired => "review_required",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityTier {
    Public,
    SignedIn,
    ClientPrivate,
    StaffAdmin,
    OwnerSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PurposeTier {
    Marketing,
    Tutoring,
    Operations,
    LegalTax,
    Medical,
    Finance,
    InternalStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTier {
    SafeLocal,
    ExternalModelAllowed,
    ExternalToolAllowed,
    ApprovalRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataHandlingTier {
    Normal,
    Sensitive,
    Regulated,
    LocalOnly,
    RedactedPlaceholder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Draft,
    NeedsEvidence,
    NeedsReview,
    ReadyForReview,
    Approved,
    Exported,
    Superseded,
    Withdrawn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceClassification {
    pub visibility: VisibilityTier,
    pub purpose: PurposeTier,
    pub execution: ExecutionTier,
    pub data_handling: DataHandlingTier,
    pub approval_state: ApprovalState,
}

impl ResourceClassification {
    pub fn local_operations_ready_for_review() -> Self {
        Self {
            visibility: VisibilityTier::OwnerSystem,
            purpose: PurposeTier::Operations,
            execution: ExecutionTier::SafeLocal,
            data_handling: DataHandlingTier::LocalOnly,
            approval_state: ApprovalState::ReadyForReview,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedAccessEvidence {
    pub loopback: bool,
    pub token: bool,
}

impl ProtectedAccessEvidence {
    pub fn allowed(&self) -> bool {
        self.loopback || self.token
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyDecision {
    pub outcome: PolicyOutcome,
    pub actor: ActorContext,
    pub action: PolicyAction,
    pub resource: ResourceRef,
    pub capability_id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PolicyDecisionCorrelation {
    pub request_id: Option<String>,
    pub job_id: Option<String>,
    pub task_key: Option<String>,
    pub artifact_id: Option<String>,
}

impl PolicyDecision {
    pub fn allowed(&self) -> bool {
        matches!(
            self.outcome,
            PolicyOutcome::Allowed | PolicyOutcome::ReviewRequired
        )
    }

    pub fn metadata(&self) -> Value {
        json!({
            "outcome": self.outcome.as_str(),
            "actor": {
                "kind": self.actor.kind.as_str(),
                "id": self.actor.id,
                "origin": self.actor.origin,
            },
            "action": self.action.as_str(),
            "resource": {
                "kind": self.resource.kind.as_str(),
                "id": self.resource.id,
            },
            "capabilityId": self.capability_id,
            "reason": self.reason,
        })
    }
}

pub fn record_policy_decision(
    connection: &Connection,
    decision: &PolicyDecision,
    correlation: PolicyDecisionCorrelation,
) -> rusqlite::Result<String> {
    let id = format!("policy_decision_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO policy_decisions (
            id, decided_at, actor_kind, actor_id, actor_origin, action, resource_kind,
            resource_id, capability_id, outcome, reason, request_id, job_id, task_key,
            artifact_id, metadata_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            id,
            Utc::now().to_rfc3339(),
            decision.actor.kind.as_str(),
            decision.actor.id.as_deref(),
            decision.actor.origin.as_str(),
            decision.action.as_str(),
            decision.resource.kind.as_str(),
            decision.resource.id.as_str(),
            decision.capability_id.as_deref(),
            decision.outcome.as_str(),
            decision.reason.as_str(),
            correlation.request_id.as_deref(),
            correlation.job_id.as_deref(),
            correlation.task_key.as_deref(),
            correlation.artifact_id.as_deref(),
            decision.metadata().to_string(),
        ],
    )?;
    Ok(id)
}

pub fn authorize_protected_daemon_action(
    actor: ActorContext,
    action: PolicyAction,
    resource: ResourceRef,
    capability_id: Option<&str>,
    access: ProtectedAccessEvidence,
) -> PolicyDecision {
    if access.allowed() {
        PolicyDecision {
            outcome: PolicyOutcome::Allowed,
            actor,
            action,
            resource,
            capability_id: capability_id.map(ToString::to_string),
            reason: "Protected daemon access satisfied by loopback or daemon access token."
                .to_string(),
        }
    } else {
        PolicyDecision {
            outcome: PolicyOutcome::Denied,
            actor,
            action,
            resource,
            capability_id: capability_id.map(ToString::to_string),
            reason:
                "Protected daemon route requires loopback access or a valid daemon access token."
                    .to_string(),
        }
    }
}

pub fn authorize_resource_access(
    connection: &Connection,
    actor: ActorContext,
    action: PolicyAction,
    resource: ResourceRef,
    capability_id: Option<&str>,
) -> PolicyDecision {
    match resource_access_allowed(connection, &actor, action, &resource) {
        Ok(true) => PolicyDecision {
            outcome: PolicyOutcome::Allowed,
            actor,
            action,
            resource,
            capability_id: capability_id.map(ToString::to_string),
            reason: "Durable resource grant allows this actor action.".to_string(),
        },
        Ok(false) => PolicyDecision {
            outcome: PolicyOutcome::Denied,
            actor,
            action,
            resource,
            capability_id: capability_id.map(ToString::to_string),
            reason: "No durable resource grant allows this actor action.".to_string(),
        },
        Err(error) => PolicyDecision {
            outcome: PolicyOutcome::Denied,
            actor,
            action,
            resource,
            capability_id: capability_id.map(ToString::to_string),
            reason: format!("Durable resource grant check failed: {error}"),
        },
    }
}

pub fn authorize_connection_resource_access(
    connection: &Connection,
    connection_id: &str,
    action: PolicyAction,
    resource: ResourceRef,
    capability_id: Option<&str>,
) -> PolicyDecision {
    match connection_resource_access_allowed(connection, connection_id, action, &resource) {
        Ok(true) => PolicyDecision {
            outcome: PolicyOutcome::Allowed,
            actor: ActorContext::new(
                ActorKind::BrowserOperator,
                "connection",
                Some(connection_id.to_string()),
            ),
            action,
            resource,
            capability_id: capability_id.map(ToString::to_string),
            reason: "Durable connection grant allows this connection action.".to_string(),
        },
        Ok(false) => PolicyDecision {
            outcome: PolicyOutcome::Denied,
            actor: ActorContext::new(
                ActorKind::BrowserOperator,
                "connection",
                Some(connection_id.to_string()),
            ),
            action,
            resource,
            capability_id: capability_id.map(ToString::to_string),
            reason: "No active durable connection grant allows this connection action.".to_string(),
        },
        Err(error) => PolicyDecision {
            outcome: PolicyOutcome::Denied,
            actor: ActorContext::new(
                ActorKind::BrowserOperator,
                "connection",
                Some(connection_id.to_string()),
            ),
            action,
            resource,
            capability_id: capability_id.map(ToString::to_string),
            reason: format!("Durable connection grant check failed: {error}"),
        },
    }
}

pub fn authorize_capability_access(
    connection: &Connection,
    actor: ActorContext,
    action: PolicyAction,
    capability: &CapabilityDefinition,
) -> PolicyDecision {
    let resource = ResourceRef::new(ResourceKind::Capability, capability.id.clone());
    match capability_access_allowed(connection, &actor, capability) {
        Ok(true) => PolicyDecision {
            outcome: PolicyOutcome::Allowed,
            actor,
            action,
            resource,
            capability_id: Some(capability.id.clone()),
            reason: "Durable role membership allows this capability.".to_string(),
        },
        Ok(false) => PolicyDecision {
            outcome: PolicyOutcome::Denied,
            actor,
            action,
            resource,
            capability_id: Some(capability.id.clone()),
            reason: "No durable role membership allows this capability.".to_string(),
        },
        Err(error) => PolicyDecision {
            outcome: PolicyOutcome::Denied,
            actor,
            action,
            resource,
            capability_id: Some(capability.id.clone()),
            reason: format!("Durable capability role check failed: {error}"),
        },
    }
}

fn capability_access_allowed(
    connection: &Connection,
    actor: &ActorContext,
    capability: &CapabilityDefinition,
) -> rusqlite::Result<bool> {
    let Some(actor_id) = actor.id.as_deref() else {
        return Ok(false);
    };
    let owner_allowed = capability
        .roles_allowed
        .iter()
        .any(|role| durable_role_id_for_catalog_role(role) == Some(OWNER_ROLE_ID));
    let system_allowed = capability
        .roles_allowed
        .iter()
        .any(|role| durable_role_id_for_catalog_role(role) == Some(SYSTEM_ROLE_ID));
    if !owner_allowed && !system_allowed {
        return Ok(false);
    }

    let allowed: i64 = connection.query_row(
        "SELECT COUNT(*)
         FROM actor_role_memberships
         WHERE actor_id = ?1
           AND ((?2 AND role_id = ?3) OR (?4 AND role_id = ?5))",
        params![
            actor_id,
            owner_allowed,
            OWNER_ROLE_ID,
            system_allowed,
            SYSTEM_ROLE_ID,
        ],
        |row| row.get(0),
    )?;

    Ok(allowed > 0)
}

fn durable_role_id_for_catalog_role(role: &str) -> Option<&'static str> {
    match role {
        "owner" => Some(OWNER_ROLE_ID),
        "system" => Some(SYSTEM_ROLE_ID),
        _ => None,
    }
}

fn resource_access_allowed(
    connection: &Connection,
    actor: &ActorContext,
    action: PolicyAction,
    resource: &ResourceRef,
) -> rusqlite::Result<bool> {
    if resource.kind == ResourceKind::System && resource.id == "public" {
        return Ok(matches!(action, PolicyAction::Read | PolicyAction::Inspect));
    }

    let Some(actor_id) = actor.id.as_deref() else {
        return Ok(false);
    };

    let allowed: i64 = connection.query_row(
        "SELECT COUNT(*)
         FROM resource_grants grant_row
         WHERE grant_row.effect = 'allow'
                     AND (grant_row.expires_at IS NULL OR grant_row.expires_at > ?5)
           AND grant_row.resource_kind = ?1
           AND grant_row.resource_id IN (?2, '*')
           AND grant_row.action IN (?3, '*')
           AND (
                (grant_row.subject_kind = 'actor' AND grant_row.subject_id = ?4)
                OR (
                    grant_row.subject_kind = 'role'
                    AND grant_row.subject_id IN (
                        SELECT role_id FROM actor_role_memberships WHERE actor_id = ?4
                    )
                )
           )",
        params![
            resource.kind.as_str(),
            resource.id,
            action.as_str(),
            actor_id,
            Utc::now().to_rfc3339(),
        ],
        |row| row.get(0),
    )?;

    Ok(allowed > 0)
}

fn connection_resource_access_allowed(
    connection: &Connection,
    connection_id: &str,
    action: PolicyAction,
    resource: &ResourceRef,
) -> rusqlite::Result<bool> {
    let allowed: i64 = connection.query_row(
        "SELECT COUNT(*)
         FROM resource_grants grant_row
         INNER JOIN connection_grants connection_grant
            ON connection_grant.resource_grant_id = grant_row.id
         INNER JOIN connections connection_row
            ON connection_row.id = connection_grant.connection_id
         WHERE connection_row.id = ?4
           AND connection_row.status = 'active'
           AND connection_grant.status = 'active'
           AND grant_row.effect = 'allow'
           AND grant_row.subject_kind = 'connection'
           AND grant_row.subject_id = ?4
           AND grant_row.resource_kind = ?1
           AND grant_row.resource_id IN (?2, '*')
           AND grant_row.action IN (?3, '*')
           AND (grant_row.expires_at IS NULL OR grant_row.expires_at > ?5)
           AND (connection_grant.expires_at IS NULL OR connection_grant.expires_at > ?5)",
        params![
            resource.kind.as_str(),
            resource.id,
            action.as_str(),
            connection_id,
            Utc::now().to_rfc3339(),
        ],
        |row| row.get(0),
    )?;

    Ok(allowed > 0)
}

pub fn authorize_mcp_capability(
    connection: &Connection,
    actor: ActorContext,
    capability: &CapabilityDefinition,
) -> PolicyDecision {
    let resource = ResourceRef::new(ResourceKind::Capability, capability.id.clone());
    match capability.mcp_export_policy.as_str() {
        MCP_EXPORT_POLICY_DANGEROUS_NONE => PolicyDecision {
            outcome: PolicyOutcome::Denied,
            actor,
            action: PolicyAction::CallTool,
            resource,
            capability_id: Some(capability.id.clone()),
            reason: "Capability is not exported through the governed MCP projection.".to_string(),
        },
        MCP_EXPORT_POLICY_OPERATOR_CONFIRMED => {
            let role_decision =
                authorize_capability_access(connection, actor, PolicyAction::CallTool, capability);
            if role_decision.outcome == PolicyOutcome::Denied {
                role_decision
            } else {
                PolicyDecision {
                    outcome: PolicyOutcome::ReviewRequired,
                    actor: role_decision.actor,
                    action: PolicyAction::CallTool,
                    resource,
                    capability_id: Some(capability.id.clone()),
                    reason: "Capability requires operator confirmation before execution."
                        .to_string(),
                }
            }
        }
        _ => authorize_capability_access(connection, actor, PolicyAction::CallTool, capability),
    }
}

pub fn provenance_metadata(
    actor: ActorContext,
    action: PolicyAction,
    resource: ResourceRef,
    capability_id: Option<&str>,
    classification: ResourceClassification,
) -> Value {
    json!({
        "schemaVersion": 1,
        "actor": {
            "kind": actor.kind.as_str(),
            "id": actor.id,
            "origin": actor.origin,
        },
        "action": action.as_str(),
        "resource": {
            "kind": resource.kind.as_str(),
            "id": resource.id,
        },
        "capabilityId": capability_id,
        "classification": classification,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::{MCP_EXPORT_POLICY_LOCAL_MUTATION, MCP_EXPORT_POLICY_READ_ONLY};
    use crate::schema::init_schema;

    fn capability(id: &str, policy: &str) -> CapabilityDefinition {
        capability_with_roles(id, policy, &["owner"])
    }

    fn capability_with_roles(id: &str, policy: &str, roles: &[&str]) -> CapabilityDefinition {
        CapabilityDefinition {
            id: id.to_string(),
            label: id.to_string(),
            description: String::new(),
            family: "system".to_string(),
            input_schema: json!({}),
            output_contract: json!({}),
            roles_allowed: roles.iter().map(|role| role.to_string()).collect(),
            execution_target: "rust".to_string(),
            timeout_seconds: 30,
            retry_policy: json!({}),
            artifact_kinds: vec![],
            scheduler_eligible: false,
            prompt_exposure: "hidden".to_string(),
            mcp_export_policy: policy.to_string(),
            side_effects: vec![],
            approval_requirement: "none".to_string(),
        }
    }

    #[test]
    fn protected_daemon_action_requires_loopback_or_token() {
        let denied = authorize_protected_daemon_action(
            ActorContext::browser_operator(),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/backups/create"),
            Some("backup.create"),
            ProtectedAccessEvidence {
                loopback: false,
                token: false,
            },
        );
        assert_eq!(denied.outcome, PolicyOutcome::Denied);

        let allowed = authorize_protected_daemon_action(
            ActorContext::browser_operator(),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/backups/create"),
            Some("backup.create"),
            ProtectedAccessEvidence {
                loopback: true,
                token: false,
            },
        );
        assert_eq!(allowed.outcome, PolicyOutcome::Allowed);
    }

    #[test]
    fn mcp_policy_distinguishes_export_review_and_denial() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert_eq!(
            authorize_mcp_capability(
                &connection,
                ActorContext::mcp_client(),
                &capability("system.status.read", MCP_EXPORT_POLICY_READ_ONLY),
            )
            .outcome,
            PolicyOutcome::Allowed
        );
        assert_eq!(
            authorize_mcp_capability(
                &connection,
                ActorContext::mcp_client(),
                &capability("backup.create", MCP_EXPORT_POLICY_LOCAL_MUTATION),
            )
            .outcome,
            PolicyOutcome::Allowed
        );
        assert_eq!(
            authorize_mcp_capability(
                &connection,
                ActorContext::mcp_client(),
                &capability(
                    "restore.preflight.validate",
                    MCP_EXPORT_POLICY_OPERATOR_CONFIRMED
                ),
            )
            .outcome,
            PolicyOutcome::ReviewRequired
        );
        assert_eq!(
            authorize_mcp_capability(
                &connection,
                ActorContext::mcp_client(),
                &capability("restore.execute", MCP_EXPORT_POLICY_DANGEROUS_NONE),
            )
            .outcome,
            PolicyOutcome::Denied
        );
    }

    #[test]
    fn provenance_metadata_names_actor_action_resource_and_classification() {
        let metadata = provenance_metadata(
            ActorContext::browser_operator(),
            PolicyAction::Prepare,
            ResourceRef::new(ResourceKind::IssueReport, "report_123"),
            Some("issue.report.prepare"),
            ResourceClassification::local_operations_ready_for_review(),
        );

        assert_eq!(metadata["actor"]["kind"], "browser_operator");
        assert_eq!(metadata["action"], "prepare");
        assert_eq!(metadata["resource"]["kind"], "issue_report");
        assert_eq!(metadata["classification"]["visibility"], "owner_system");
    }

    #[test]
    fn durable_grants_allow_owner_system_and_deny_unknown_actor() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        let owner_decision = authorize_resource_access(
            &connection,
            ActorContext::local_owner("test"),
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::OwnerSystem, "issue_report_artifacts"),
            None,
        );
        assert_eq!(owner_decision.outcome, PolicyOutcome::Allowed);

        let unknown_decision = authorize_resource_access(
            &connection,
            ActorContext::new(
                ActorKind::BrowserOperator,
                "test",
                Some("actor_unknown".to_string()),
            ),
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::OwnerSystem, "issue_report_artifacts"),
            None,
        );
        assert_eq!(unknown_decision.outcome, PolicyOutcome::Denied);
    }

    #[test]
    fn durable_private_actor_grants_do_not_cross_actors() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
                 VALUES ('actor_student_a', 'external_user', 'Student A', 'active', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
                 VALUES ('actor_student_b', 'external_user', 'Student B', 'active', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO resource_grants (
                    id, resource_kind, resource_id, action, subject_kind, subject_id, effect, created_at, metadata_json
                 ) VALUES (
                    'grant_student_a_report', 'private_actor', 'report_private', 'read', 'actor', 'actor_student_a', 'allow', 'now', '{}'
                 )",
                [],
            )
            .unwrap();

        let student_a = authorize_resource_access(
            &connection,
            ActorContext::new(
                ActorKind::BrowserOperator,
                "test",
                Some("actor_student_a".to_string()),
            ),
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::PrivateActor, "report_private"),
            None,
        );
        let student_b = authorize_resource_access(
            &connection,
            ActorContext::new(
                ActorKind::BrowserOperator,
                "test",
                Some("actor_student_b".to_string()),
            ),
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::PrivateActor, "report_private"),
            None,
        );

        assert_eq!(student_a.outcome, PolicyOutcome::Allowed);
        assert_eq!(student_b.outcome, PolicyOutcome::Denied);
    }

    #[test]
    fn durable_grants_protect_corpus_resources_before_retrieval_exists() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
                 VALUES ('actor_private_reader', 'external_user', 'Private Reader', 'active', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
                 VALUES ('actor_other_reader', 'external_user', 'Other Reader', 'active', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO corpus_sources (
                    id, source_kind, label, uri, resource_kind, resource_id, status,
                    classification_json, provenance_json, metadata_json, created_at, updated_at
                 ) VALUES (
                    'corpus_source_private', 'markdown', 'Private Notes', NULL,
                    'private_actor', 'knowledge_private_reader', 'approved', '{}', '{}', '{}', 'now', 'now'
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO resource_grants (
                    id, resource_kind, resource_id, action, subject_kind, subject_id, effect, created_at, metadata_json
                 ) VALUES (
                    'grant_private_reader_corpus', 'corpus_source', 'corpus_source_private', 'read', 'actor', 'actor_private_reader', 'allow', 'now', '{}'
                 )",
                [],
            )
            .unwrap();

        let owner_system = authorize_resource_access(
            &connection,
            ActorContext::local_owner("test"),
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::OwnerSystem, "knowledge_owner_manual"),
            None,
        );
        let private_reader = authorize_resource_access(
            &connection,
            ActorContext::new(
                ActorKind::BrowserOperator,
                "test",
                Some("actor_private_reader".to_string()),
            ),
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::CorpusSource, "corpus_source_private"),
            None,
        );
        let other_reader = authorize_resource_access(
            &connection,
            ActorContext::new(
                ActorKind::BrowserOperator,
                "test",
                Some("actor_other_reader".to_string()),
            ),
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::CorpusSource, "corpus_source_private"),
            None,
        );

        assert_eq!(owner_system.outcome, PolicyOutcome::Allowed);
        assert_eq!(private_reader.outcome, PolicyOutcome::Allowed);
        assert_eq!(other_reader.outcome, PolicyOutcome::Denied);
    }

    #[test]
    fn capability_roles_bind_to_durable_role_memberships() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        let owner_capability = capability_with_roles(
            "brief.system.generate",
            MCP_EXPORT_POLICY_LOCAL_MUTATION,
            &["owner", "system"],
        );
        let owner_decision = authorize_capability_access(
            &connection,
            ActorContext::local_owner("test"),
            PolicyAction::Execute,
            &owner_capability,
        );
        let system_decision = authorize_capability_access(
            &connection,
            ActorContext::system(),
            PolicyAction::Execute,
            &owner_capability,
        );
        let unknown_decision = authorize_capability_access(
            &connection,
            ActorContext::new(
                ActorKind::BrowserOperator,
                "test",
                Some("actor_unknown".to_string()),
            ),
            PolicyAction::Execute,
            &owner_capability,
        );

        assert_eq!(owner_decision.outcome, PolicyOutcome::Allowed);
        assert_eq!(system_decision.outcome, PolicyOutcome::Allowed);
        assert_eq!(unknown_decision.outcome, PolicyOutcome::Denied);
    }

    #[test]
    fn capability_roles_do_not_bypass_resource_grants() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let capability = capability_with_roles(
            "issue.report.prepare",
            MCP_EXPORT_POLICY_LOCAL_MUTATION,
            &["owner"],
        );

        let capability_decision = authorize_capability_access(
            &connection,
            ActorContext::local_owner("test"),
            PolicyAction::Execute,
            &capability,
        );
        let resource_decision = authorize_resource_access(
            &connection,
            ActorContext::local_owner("test"),
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::PrivateActor, "other_actor_private_resource"),
            Some("issue.report.prepare"),
        );

        assert_eq!(capability_decision.outcome, PolicyOutcome::Allowed);
        assert_eq!(resource_decision.outcome, PolicyOutcome::Denied);
    }

    #[test]
    fn records_policy_decision_audit_evidence() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let decision = authorize_resource_access(
            &connection,
            ActorContext::local_owner("test"),
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::OwnerSystem, "issue_report_artifacts"),
            Some("issue.report.list"),
        );

        let id = record_policy_decision(
            &connection,
            &decision,
            PolicyDecisionCorrelation {
                request_id: Some("request_123".to_string()),
                ..PolicyDecisionCorrelation::default()
            },
        )
        .unwrap();

        let row: (String, String, String, String, String) = connection
            .query_row(
                "SELECT actor_kind, action, resource_kind, outcome, request_id
                 FROM policy_decisions WHERE id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(row.0, "local_owner");
        assert_eq!(row.1, "read");
        assert_eq!(row.2, "owner_system");
        assert_eq!(row.3, "allowed");
        assert_eq!(row.4, "request_123");
    }

    #[test]
    fn records_denied_policy_decision_audit_evidence() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let decision = authorize_resource_access(
            &connection,
            ActorContext::new(
                ActorKind::BrowserOperator,
                "test",
                Some("actor_unknown".to_string()),
            ),
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::OwnerSystem, "issue_report_artifacts"),
            None,
        );

        let id =
            record_policy_decision(&connection, &decision, PolicyDecisionCorrelation::default())
                .unwrap();

        let outcome: String = connection
            .query_row(
                "SELECT outcome FROM policy_decisions WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(outcome, "denied");
    }
}
