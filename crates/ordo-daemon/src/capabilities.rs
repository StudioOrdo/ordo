use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;

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
            "safe_system_tool",
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
            "safe_system_tool",
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
            "safe_system_tool",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "safe_system_tool",
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
            "safe_system_tool",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "safe_system_tool",
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
            "safe_system_tool",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "safe_system_tool",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
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
            "none",
            false,
            &["restore.record"],
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
                created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)
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
                artifact_kinds_json, scheduler_eligible, prompt_exposure, mcp_export_policy
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
                artifact_kinds_json, scheduler_eligible, prompt_exposure, mcp_export_policy
         FROM capabilities
         WHERE mcp_export_policy = 'safe_system_tool'
         ORDER BY family ASC, id ASC",
    )?;
    let rows = statement.query_map([], capability_from_row)?;
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
                    artifact_kinds_json, scheduler_eligible, prompt_exposure, mcp_export_policy
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
        prompt_exposure: if mcp_exported { "system" } else { "internal" }.to_string(),
        mcp_export_policy: mcp_export_policy.to_string(),
    }
}

fn capability_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CapabilityDefinition> {
    let input_schema_json: String = row.get(4)?;
    let output_contract_json: String = row.get(5)?;
    let roles_allowed_json: String = row.get(6)?;
    let retry_policy_json: String = row.get(9)?;
    let artifact_kinds_json: String = row.get(10)?;

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
    }

    #[test]
    fn lists_only_safe_mcp_exports() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();

        let exported = list_mcp_exported_capabilities(&connection).unwrap();
        assert!(exported
            .iter()
            .any(|capability| capability.id == "system.status.read"));
        assert!(exported
            .iter()
            .all(|capability| capability.mcp_export_policy == "safe_system_tool"));
    }
}
