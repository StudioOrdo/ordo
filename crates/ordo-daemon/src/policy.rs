use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::capabilities::{
    CapabilityDefinition, MCP_EXPORT_POLICY_DANGEROUS_NONE, MCP_EXPORT_POLICY_OPERATOR_CONFIRMED,
};

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

    pub fn mcp_client() -> Self {
        Self::new(ActorKind::McpClient, "mcp", None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    System,
    DaemonRoute,
    Capability,
    ProcessTemplate,
    Job,
    JobArtifact,
    BriefArtifact,
    IssueReport,
    DiagnosticLog,
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::DaemonRoute => "daemon_route",
            Self::Capability => "capability",
            Self::ProcessTemplate => "process_template",
            Self::Job => "job",
            Self::JobArtifact => "job_artifact",
            Self::BriefArtifact => "brief_artifact",
            Self::IssueReport => "issue_report",
            Self::DiagnosticLog => "diagnostic_log",
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
    Validate,
    Prepare,
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
            Self::Validate => "validate",
            Self::Prepare => "prepare",
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

pub fn authorize_mcp_capability(
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
        MCP_EXPORT_POLICY_OPERATOR_CONFIRMED => PolicyDecision {
            outcome: PolicyOutcome::ReviewRequired,
            actor,
            action: PolicyAction::CallTool,
            resource,
            capability_id: Some(capability.id.clone()),
            reason: "Capability requires operator confirmation before execution.".to_string(),
        },
        _ => PolicyDecision {
            outcome: PolicyOutcome::Allowed,
            actor,
            action: PolicyAction::CallTool,
            resource,
            capability_id: Some(capability.id.clone()),
            reason: "Capability is exported through the governed MCP projection.".to_string(),
        },
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

    fn capability(id: &str, policy: &str) -> CapabilityDefinition {
        CapabilityDefinition {
            id: id.to_string(),
            label: id.to_string(),
            description: String::new(),
            family: "system".to_string(),
            input_schema: json!({}),
            output_contract: json!({}),
            roles_allowed: vec!["owner".to_string()],
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
        assert_eq!(
            authorize_mcp_capability(
                ActorContext::mcp_client(),
                &capability("system.status.read", MCP_EXPORT_POLICY_READ_ONLY),
            )
            .outcome,
            PolicyOutcome::Allowed
        );
        assert_eq!(
            authorize_mcp_capability(
                ActorContext::mcp_client(),
                &capability("backup.create", MCP_EXPORT_POLICY_LOCAL_MUTATION),
            )
            .outcome,
            PolicyOutcome::Allowed
        );
        assert_eq!(
            authorize_mcp_capability(
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
}
