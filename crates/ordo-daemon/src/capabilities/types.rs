use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Governed export policy for an MCP-compatible capability.
pub const MCP_EXPORT_POLICY_READ_ONLY: &str = "read_only";
/// Export policy permitting local mutation actions.
pub const MCP_EXPORT_POLICY_LOCAL_MUTATION: &str = "local_mutation";
/// Export policy requiring active operator confirmation for invocation.
pub const MCP_EXPORT_POLICY_OPERATOR_CONFIRMED: &str = "operator_confirmed";
/// Indicates a capability is safe and lacks dangerous side-effects.
pub const MCP_EXPORT_POLICY_DANGEROUS_NONE: &str = "dangerous_none";

/// Comprehensive schema definition of a system capability, including
/// its API signature, side effects, and export policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityDefinition {
    pub id: String,
    pub label: String,
    pub description: String,
    pub family: String,
    pub input_schema: Value,
    pub output_contract: Value,
    pub roles_allowed: Vec<String>,
    pub execution_target: String,
    pub timeout_seconds: i64,
    pub retry_policy: Value,
    pub artifact_kinds: Vec<String>,
    pub scheduler_eligible: bool,
    pub prompt_exposure: String,
    pub mcp_export_policy: String,
    pub side_effects: Vec<String>,
    pub approval_requirement: String,
}

/// Defines the response structure for listing the capability catalog.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityCatalogResponse {
    /// Ordered list of available system capabilities.
    pub capabilities: Vec<CapabilityDefinition>,
}

