use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;

pub const MCP_EXPORT_POLICY_READ_ONLY: &str = "read_only";
pub const MCP_EXPORT_POLICY_LOCAL_MUTATION: &str = "local_mutation";
pub const MCP_EXPORT_POLICY_OPERATOR_CONFIRMED: &str = "operator_confirmed";
pub const MCP_EXPORT_POLICY_DANGEROUS_NONE: &str = "dangerous_none";

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityCatalogResponse {
    pub capabilities: Vec<CapabilityDefinition>,
}

pub fn built_in_capabilities() -> Vec<CapabilityDefinition> {
    vec![
        capability(
            "capability.catalog.list",
            "List Capability Catalog",
            "Read the governed capabilities registered in this appliance.",
            "system",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object", "properties": { "capabilities": { "type": "array" } } }),
            "rust",
            true,
            MCP_EXPORT_POLICY_READ_ONLY,
            false,
            &[],
        ),
        capability(
            "system.status.read",
            "Read System Status",
            "Read daemon health, readiness, and catalog status.",
            "system",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object", "properties": { "health": { "type": "object" }, "readiness": { "type": "object" } } }),
            "rust",
            true,
            MCP_EXPORT_POLICY_READ_ONLY,
            false,
            &[],
        ),
        capability(
            "appliance.runtime.status",
            "Read Appliance Runtime Status",
            "Read the local appliance runtime status exposed by the daemon.",
            "system",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            true,
            MCP_EXPORT_POLICY_READ_ONLY,
            false,
            &[],
        ),
        capability(
            "install.state.read",
            "Read Local Install State",
            "Read local appliance install state, owner profile, business profile, and provider readiness summary.",
            "install",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "install.complete",
            "Complete Local Install",
            "Persist local owner and business install identity for the appliance.",
            "install",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "providers.list",
            "List Provider Configuration",
            "Read redacted local provider configuration and secret source state.",
            "providers",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "providers.update",
            "Update Provider Configuration",
            "Update local provider configuration and write-only provider secret references.",
            "providers",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "business.facts.list",
            "List Business Facts",
            "Read local business facts through visibility and publication filters.",
            "business",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "business.facts.write",
            "Write Business Facts",
            "Create or update local business facts with provenance, visibility, and publication state.",
            "business",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "entry_points.list",
            "List Tracked Entry Points",
            "Read tracked entry points and their public-safe destination payloads.",
            "entry_points",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "entry_points.write",
            "Write Tracked Entry Points",
            "Create or update tracked links and QR payload records for public-safe destinations.",
            "entry_points",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "visitor_sessions.list",
            "List Visitor Sessions",
            "Read visitor sessions with tracked entry point attribution context.",
            "entry_points",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "offers.list",
            "List Offers",
            "Read durable offer records and their publication state.",
            "offers",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "offers.write",
            "Write Offers",
            "Create or update durable offer records and public availability state.",
            "offers",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "offer_acceptances.list",
            "List Offer Acceptances",
            "Read offer acceptances with visitor session and attribution context.",
            "offers",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "trials.list",
            "List Trials",
            "Read trial lifecycle state and recorded decision evidence.",
            "offers",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "trials.transition",
            "Transition Trial State",
            "Record conversion, void, expiration, or follow-up-needed trial decisions.",
            "offers",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "connections.list",
            "List Connections",
            "Read durable connection records and status state.",
            "connections",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "connections.write",
            "Write Connections",
            "Create and update durable connection records and lifecycle state.",
            "connections",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "connection_grants.write",
            "Write Connection Grants",
            "Create and revoke explicit scoped grants attached to connections.",
            "connections",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "connection_grants.list",
            "List Connection Grants",
            "Read explicit scoped grants attached to connections.",
            "connections",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "connection_events.list",
            "List Connection Events",
            "Read durable connection events and local receipt evidence.",
            "connections",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "availability.read",
            "Read Availability State",
            "Read local availability schedule and operator presence state.",
            "availability",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "availability.write",
            "Write Availability State",
            "Update local availability schedule and operator presence state.",
            "availability",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "handoff.eligibility.evaluate",
            "Evaluate Handoff Eligibility",
            "Record a local handoff eligibility decision with evidence.",
            "handoff",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "handoff.inbox.list",
            "List Handoff Inbox",
            "Read owner attention inbox items and local handoff state.",
            "handoff",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "handoff.inbox.write",
            "Write Handoff Inbox",
            "Create and resolve owner attention inbox items without external delivery.",
            "handoff",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "handoff.receipts.list",
            "List Handoff Receipts",
            "Read local receipt evidence for handoff inbox items.",
            "handoff",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "system.health.check",
            "System Health Check",
            "Create a governed system health check job.",
            "system",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            true,
            &[],
        ),
        capability(
            "system.health.probe",
            "Probe Appliance Health",
            "Probe daemon health as a task in the operation kernel.",
            "system",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "system.health.record",
            "Record Health Evidence",
            "Record health evidence from a system health check task.",
            "system",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &["system.health"],
        ),
        capability(
            "scheduler.system.brief.run",
            "Run Due System Brief Schedules",
            "Claim due System Brief schedules and create governed jobs.",
            "system",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            true,
            &[],
        ),
        capability(
            "brief.system.latest",
            "Read Latest System Brief",
            "Read the latest durable System Brief artifact.",
            "brief",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            true,
            MCP_EXPORT_POLICY_READ_ONLY,
            false,
            &["brief.system"],
        ),
        capability(
            "brief.system.generate",
            "Generate System Brief",
            "Create a governed System Brief generation job.",
            "brief",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            true,
            MCP_EXPORT_POLICY_LOCAL_MUTATION,
            true,
            &["brief.system"],
        ),
        capability(
            "brief.scope.validate",
            "Validate Brief Scope",
            "Validate System Brief scope before evidence collection.",
            "brief",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "brief.evidence.collect",
            "Collect Brief Evidence",
            "Collect health, readiness, and schedule evidence for the System Brief.",
            "brief",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "brief.evidence.manifest",
            "Build Evidence Manifest",
            "Build a manifest for brief evidence sources.",
            "brief",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "brief.draft.generate",
            "Generate Brief Draft",
            "Generate deterministic System Brief draft content.",
            "brief",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "brief.claims.validate",
            "Validate Brief Claims",
            "Validate System Brief claims against collected evidence.",
            "brief",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "brief.artifact.save",
            "Save Brief Artifact",
            "Save durable System Brief artifact.",
            "brief",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &["brief.system"],
        ),
        capability(
            "backup.restore_jobs.list",
            "List Backup And Restore Jobs",
            "Read backup and restore job state from SQLite.",
            "backup",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            true,
            MCP_EXPORT_POLICY_READ_ONLY,
            false,
            &["backup.archive", "restore.safety_record"],
        ),
        capability(
            "backup.create",
            "Create Backup",
            "Create a governed backup job and durable backup archive.",
            "backup",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object" }),
            "rust",
            true,
            MCP_EXPORT_POLICY_LOCAL_MUTATION,
            true,
            &["backup.archive"],
        ),
        capability(
            "backup.boundary.check",
            "Check Backup Boundary",
            "Check the appliance data boundary before backup.",
            "backup",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "backup.lock.acquire",
            "Acquire Backup Lock",
            "Acquire the single-appliance backup lock.",
            "backup",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "backup.sqlite.snapshot",
            "Snapshot SQLite",
            "Create a SQLite snapshot for backup.",
            "backup",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &["backup.archive"],
        ),
        capability(
            "backup.files.scan",
            "Scan Data Files",
            "Scan local appliance files inside the data boundary.",
            "backup",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "backup.archive.write",
            "Write Backup Archive",
            "Write a durable backup archive.",
            "backup",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &["backup.archive"],
        ),
        capability(
            "backup.manifest.write",
            "Write Backup Manifest",
            "Write a backup manifest with evidence and checksums.",
            "backup",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &["backup.manifest"],
        ),
        capability(
            "backup.integrity.verify",
            "Verify Backup Integrity",
            "Verify manifest and archive checksums.",
            "backup",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "backup.record",
            "Record Backup",
            "Record backup artifact evidence in SQLite.",
            "backup",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &["backup.archive"],
        ),
        capability(
            "restore.execute",
            "Execute Restore",
            "Create a governed restore execution job.",
            "restore",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &["restore.safety_record"],
        ),
        capability(
            "restore.preflight.validate",
            "Validate Restore Preflight",
            "Verify a backup archive and create a non-destructive restore safety record.",
            "restore",
            json!({
                "type": "object",
                "required": ["backupId", "confirmation"],
                "properties": {
                    "backupId": { "type": "string" },
                    "confirmation": { "type": "string" }
                },
                "additionalProperties": false
            }),
            json!({ "type": "object" }),
            "rust",
            true,
            MCP_EXPORT_POLICY_OPERATOR_CONFIRMED,
            false,
            &["restore.safety_record"],
        ),
        capability(
            "restore.request.validate",
            "Validate Restore Request",
            "Validate restore request input.",
            "restore",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "restore.archive.verify",
            "Verify Restore Archive",
            "Verify a backup archive before restore.",
            "restore",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "restore.confirmation.require",
            "Require Restore Confirmation",
            "Require explicit restore confirmation text.",
            "restore",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "restore.safety.backup",
            "Create Restore Safety Backup",
            "Create restore safety evidence before destructive operations.",
            "restore",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &["restore.safety_record"],
        ),
        capability(
            "restore.lock.acquire",
            "Acquire Restore Lock",
            "Acquire restore lock before live data replacement.",
            "restore",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "restore.sqlite.restore",
            "Restore SQLite",
            "Restore SQLite from a verified backup archive.",
            "restore",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "restore.files.restore",
            "Restore Files",
            "Restore local files from a verified backup archive.",
            "restore",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "restore.state.verify",
            "Verify Restored State",
            "Verify appliance state after restore.",
            "restore",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "system.next.restart",
            "Restart Next.js",
            "Restart the supervised Next.js management process when safe.",
            "system",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "restore.record",
            "Record Restore",
            "Record restore artifact evidence in SQLite.",
            "restore",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &["restore.record"],
        ),
        capability(
            "diagnostic.logs.list",
            "List Diagnostic Logs",
            "Read recent structured diagnostic logs from local SQLite.",
            "diagnostics",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object", "properties": { "logs": { "type": "array" } } }),
            "rust",
            true,
            MCP_EXPORT_POLICY_READ_ONLY,
            false,
            &[],
        ),
        capability(
            "policy.decisions.list",
            "List Policy Decisions",
            "Read bounded recent policy decision audit records for protected local operations.",
            "policy",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "issue.report.list",
            "List Issue Reports",
            "Read locally prepared issue report artifacts.",
            "issue",
            json!({ "type": "object", "additionalProperties": false }),
            json!({ "type": "object", "properties": { "reports": { "type": "array" } } }),
            "rust",
            true,
            MCP_EXPORT_POLICY_READ_ONLY,
            false,
            &["issue.report"],
        ),
        capability(
            "issue.report.prepare",
            "Prepare Issue Report",
            "Create a governed local issue report with diagnostics, evidence, and redaction notes.",
            "issue",
            json!({ "type": "object", "required": ["description"], "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            true,
            MCP_EXPORT_POLICY_LOCAL_MUTATION,
            false,
            &["issue.report"],
        ),
        capability(
            "issue.scope.validate",
            "Validate Issue Scope",
            "Validate issue report request scope.",
            "issue",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "issue.narrative.capture",
            "Capture Issue Narrative",
            "Normalize issue report narrative fields.",
            "issue",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "issue.diagnostics.collect",
            "Collect Issue Diagnostics",
            "Collect health, readiness, log, and source diagnostics.",
            "issue",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "issue.events.collect",
            "Collect Issue Events",
            "Collect recent persisted events for issue reports.",
            "issue",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "issue.jobs.collect",
            "Collect Issue Jobs",
            "Collect recent job evidence for issue reports.",
            "issue",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "issue.redactions.apply",
            "Apply Issue Redactions",
            "Apply report redaction policy and record limitations.",
            "issue",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "issue.draft.generate",
            "Generate Issue Draft",
            "Render the local issue report draft.",
            "issue",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &[],
        ),
        capability(
            "issue.artifact.save",
            "Save Issue Artifact",
            "Save durable issue report artifacts.",
            "issue",
            json!({ "type": "object", "additionalProperties": true }),
            json!({ "type": "object" }),
            "rust",
            false,
            MCP_EXPORT_POLICY_DANGEROUS_NONE,
            false,
            &["issue.report"],
        ),
    ]
}

pub fn seed_builtin_capabilities(connection: &Connection) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    for capability in built_in_capabilities() {
        connection.execute(
            "INSERT INTO capabilities (
                id, label, description, family, input_schema_json, output_contract_json,
                roles_allowed_json, execution_target, timeout_seconds, retry_policy_json,
                artifact_kinds_json, scheduler_eligible, prompt_exposure, mcp_export_policy,
                     side_effects_json, approval_requirement, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?17)
             ON CONFLICT(id) DO UPDATE SET
                label = excluded.label,
                description = excluded.description,
                family = excluded.family,
                input_schema_json = excluded.input_schema_json,
                output_contract_json = excluded.output_contract_json,
                roles_allowed_json = excluded.roles_allowed_json,
                execution_target = excluded.execution_target,
                timeout_seconds = excluded.timeout_seconds,
                retry_policy_json = excluded.retry_policy_json,
                artifact_kinds_json = excluded.artifact_kinds_json,
                scheduler_eligible = excluded.scheduler_eligible,
                prompt_exposure = excluded.prompt_exposure,
                mcp_export_policy = excluded.mcp_export_policy,
                side_effects_json = excluded.side_effects_json,
                approval_requirement = excluded.approval_requirement,
                updated_at = excluded.updated_at",
            params![
                capability.id,
                capability.label,
                capability.description,
                capability.family,
                capability.input_schema.to_string(),
                capability.output_contract.to_string(),
                serde_json::to_string(&capability.roles_allowed)?,
                capability.execution_target,
                capability.timeout_seconds,
                capability.retry_policy.to_string(),
                serde_json::to_string(&capability.artifact_kinds)?,
                if capability.scheduler_eligible { 1 } else { 0 },
                capability.prompt_exposure,
                capability.mcp_export_policy,
                serde_json::to_string(&capability.side_effects)?,
                capability.approval_requirement,
                now,
            ],
        )?;
    }

    Ok(())
}

pub fn list_capabilities(db_path: &Path) -> Result<CapabilityCatalogResponse> {
    let connection = Connection::open(db_path)?;
    Ok(CapabilityCatalogResponse {
        capabilities: load_capabilities(&connection)?,
    })
}

pub fn load_capabilities(connection: &Connection) -> Result<Vec<CapabilityDefinition>> {
    let mut statement = connection.prepare(
        "SELECT id, label, description, family, input_schema_json, output_contract_json,
                roles_allowed_json, execution_target, timeout_seconds, retry_policy_json,
                artifact_kinds_json, scheduler_eligible, prompt_exposure, mcp_export_policy,
                side_effects_json, approval_requirement
         FROM capabilities
         ORDER BY family ASC, id ASC",
    )?;
    let rows = statement.query_map([], capability_from_row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn list_mcp_exported_capabilities(
    connection: &Connection,
) -> Result<Vec<CapabilityDefinition>> {
    let mut statement = connection.prepare(
        "SELECT id, label, description, family, input_schema_json, output_contract_json,
                roles_allowed_json, execution_target, timeout_seconds, retry_policy_json,
                artifact_kinds_json, scheduler_eligible, prompt_exposure, mcp_export_policy,
                side_effects_json, approval_requirement
         FROM capabilities
         WHERE mcp_export_policy IN (?1, ?2, ?3)
         ORDER BY family ASC, id ASC",
    )?;
    let rows = statement.query_map(
        params![
            MCP_EXPORT_POLICY_READ_ONLY,
            MCP_EXPORT_POLICY_LOCAL_MUTATION,
            MCP_EXPORT_POLICY_OPERATOR_CONFIRMED
        ],
        capability_from_row,
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn load_capability(
    connection: &Connection,
    capability_id: &str,
) -> Result<Option<CapabilityDefinition>> {
    connection
        .query_row(
            "SELECT id, label, description, family, input_schema_json, output_contract_json,
                    roles_allowed_json, execution_target, timeout_seconds, retry_policy_json,
                    artifact_kinds_json, scheduler_eligible, prompt_exposure, mcp_export_policy,
                    side_effects_json, approval_requirement
             FROM capabilities
             WHERE id = ?1",
            [capability_id],
            capability_from_row,
        )
        .optional()
        .map_err(Into::into)
}

pub fn assert_capability_ids_registered(
    connection: &Connection,
    capability_ids: &[String],
) -> Result<()> {
    let requested: BTreeSet<String> = capability_ids.iter().cloned().collect();
    for capability_id in requested {
        if load_capability(connection, &capability_id)?.is_none() {
            bail!("Unknown capability: {capability_id}");
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn capability(
    id: &str,
    label: &str,
    description: &str,
    family: &str,
    input_schema: Value,
    output_contract: Value,
    execution_target: &str,
    mcp_exported: bool,
    mcp_export_policy: &str,
    scheduler_eligible: bool,
    artifact_kinds: &[&str],
) -> CapabilityDefinition {
    let policy_exported = is_mcp_export_policy_exported(mcp_export_policy);
    debug_assert_eq!(mcp_exported, policy_exported);
    let side_effects = side_effects_for_capability(id, mcp_export_policy);
    CapabilityDefinition {
        id: id.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        family: family.to_string(),
        input_schema,
        output_contract,
        roles_allowed: vec!["owner".to_string(), "system".to_string()],
        execution_target: execution_target.to_string(),
        timeout_seconds: 30,
        retry_policy: json!({ "maxAttempts": 1 }),
        artifact_kinds: artifact_kinds
            .iter()
            .map(|artifact_kind| artifact_kind.to_string())
            .collect(),
        scheduler_eligible,
        prompt_exposure: if policy_exported {
            "system"
        } else {
            "internal"
        }
        .to_string(),
        mcp_export_policy: mcp_export_policy.to_string(),
        side_effects,
        approval_requirement: approval_requirement_for_policy(mcp_export_policy).to_string(),
    }
}

fn is_mcp_export_policy_exported(policy: &str) -> bool {
    matches!(
        policy,
        MCP_EXPORT_POLICY_READ_ONLY
            | MCP_EXPORT_POLICY_LOCAL_MUTATION
            | MCP_EXPORT_POLICY_OPERATOR_CONFIRMED
    )
}

fn side_effects_for_capability(id: &str, mcp_export_policy: &str) -> Vec<String> {
    let side_effects = match id {
        "brief.system.generate" => &["creates_job", "writes_sqlite", "writes_brief_artifact"][..],
        "backup.create" => &["creates_job", "writes_sqlite", "writes_backup_archive"][..],
        "restore.preflight.validate" => &[
            "creates_job",
            "writes_sqlite",
            "writes_restore_safety_record",
        ][..],
        "restore.execute" => &["not_mcp_exported", "may_replace_live_state"][..],
        "issue.report.prepare" => &[
            "creates_job",
            "writes_sqlite",
            "writes_issue_report_artifact",
        ][..],
        _ if mcp_export_policy == MCP_EXPORT_POLICY_READ_ONLY => &[][..],
        _ => &["not_mcp_exported", "internal_kernel_effects"][..],
    };
    side_effects
        .iter()
        .map(|effect| effect.to_string())
        .collect()
}

fn approval_requirement_for_policy(policy: &str) -> &str {
    match policy {
        MCP_EXPORT_POLICY_READ_ONLY => "none",
        MCP_EXPORT_POLICY_LOCAL_MUTATION => "local_access_required",
        MCP_EXPORT_POLICY_OPERATOR_CONFIRMED => "operator_confirmation_required",
        MCP_EXPORT_POLICY_DANGEROUS_NONE => "not_exported",
        _ => "not_exported",
    }
}

fn capability_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CapabilityDefinition> {
    let input_schema_json: String = row.get(4)?;
    let output_contract_json: String = row.get(5)?;
    let roles_allowed_json: String = row.get(6)?;
    let retry_policy_json: String = row.get(9)?;
    let artifact_kinds_json: String = row.get(10)?;
    let side_effects_json: String = row.get(14)?;

    Ok(CapabilityDefinition {
        id: row.get(0)?,
        label: row.get(1)?,
        description: row.get(2)?,
        family: row.get(3)?,
        input_schema: serde_json::from_str(&input_schema_json).unwrap_or_else(|_| json!({})),
        output_contract: serde_json::from_str(&output_contract_json).unwrap_or_else(|_| json!({})),
        roles_allowed: serde_json::from_str(&roles_allowed_json).unwrap_or_default(),
        execution_target: row.get(7)?,
        timeout_seconds: row.get(8)?,
        retry_policy: serde_json::from_str(&retry_policy_json).unwrap_or_else(|_| json!({})),
        artifact_kinds: serde_json::from_str(&artifact_kinds_json).unwrap_or_default(),
        scheduler_eligible: row.get::<_, i64>(11)? == 1,
        prompt_exposure: row.get(12)?,
        mcp_export_policy: row.get(13)?,
        side_effects: serde_json::from_str(&side_effects_json).unwrap_or_default(),
        approval_requirement: row.get(15)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;

    #[test]
    fn seeds_and_loads_builtin_capabilities() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();

        let capabilities = load_capabilities(&connection).unwrap();
        assert!(capabilities
            .iter()
            .any(|capability| capability.id == "system.status.read"));
        assert!(capabilities
            .iter()
            .any(|capability| capability.id == "backup.create"));
        assert!(capabilities
            .iter()
            .any(|capability| capability.id == "restore.preflight.validate"));
        for capability_id in [
            "install.state.read",
            "install.complete",
            "providers.list",
            "providers.update",
            "business.facts.list",
            "business.facts.write",
            "policy.decisions.list",
        ] {
            assert!(capabilities
                .iter()
                .any(|capability| capability.id == capability_id));
        }
    }

    #[test]
    fn lists_exported_mcp_policy_tiers() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();

        let exported = list_mcp_exported_capabilities(&connection).unwrap();
        assert!(exported
            .iter()
            .any(|capability| capability.id == "system.status.read"));
        assert!(exported
            .iter()
            .any(|capability| capability.id == "backup.create"
                && capability.mcp_export_policy == MCP_EXPORT_POLICY_LOCAL_MUTATION));
        assert!(exported
            .iter()
            .any(|capability| capability.id == "restore.preflight.validate"
                && capability.mcp_export_policy == MCP_EXPORT_POLICY_OPERATOR_CONFIRMED));
        assert!(exported
            .iter()
            .all(|capability| is_mcp_export_policy_exported(&capability.mcp_export_policy)));
        assert!(!exported
            .iter()
            .any(|capability| capability.id == "restore.execute"));
        assert!(!exported.iter().any(|capability| matches!(
            capability.id.as_str(),
            "install.state.read"
                | "install.complete"
                | "providers.list"
                | "providers.update"
                | "business.facts.list"
                | "business.facts.write"
                | "policy.decisions.list"
        )));
    }

    #[test]
    fn protected_route_capability_ids_are_registered() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        let capability_ids = [
            "install.state.read",
            "install.complete",
            "providers.list",
            "providers.update",
            "business.facts.list",
            "business.facts.write",
            "policy.decisions.list",
        ]
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

        assert_capability_ids_registered(&connection, &capability_ids).unwrap();
    }

    #[test]
    fn capability_metadata_distinguishes_side_effects_and_approval() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();

        let backup_create = load_capability(&connection, "backup.create")
            .unwrap()
            .unwrap();
        assert_eq!(
            backup_create.mcp_export_policy,
            MCP_EXPORT_POLICY_LOCAL_MUTATION
        );
        assert_eq!(backup_create.approval_requirement, "local_access_required");
        assert!(backup_create
            .side_effects
            .iter()
            .any(|effect| effect == "writes_backup_archive"));

        let status_read = load_capability(&connection, "system.status.read")
            .unwrap()
            .unwrap();
        assert_eq!(status_read.mcp_export_policy, MCP_EXPORT_POLICY_READ_ONLY);
        assert_eq!(status_read.approval_requirement, "none");
        assert!(status_read.side_effects.is_empty());

        let restore_execute = load_capability(&connection, "restore.execute")
            .unwrap()
            .unwrap();
        assert_eq!(
            restore_execute.mcp_export_policy,
            MCP_EXPORT_POLICY_DANGEROUS_NONE
        );
        assert_eq!(restore_execute.approval_requirement, "not_exported");
    }
}
