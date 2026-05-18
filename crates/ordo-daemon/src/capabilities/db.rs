use super::registry::*;
use super::types::*;
use crate::schema::db::ConnectionExt;
use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::json;
use std::collections::BTreeSet;
use std::path::Path;

/// Seeds the database with all built-in capabilities on application startup.
/// Preserves any previously synced configurations while adding new ones.
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

/// Lists all capabilities from the catalog, ordered by their ID.
pub fn list_capabilities(db_path: &Path) -> Result<CapabilityCatalogResponse> {
    let connection = Connection::open(db_path)?;
    Ok(CapabilityCatalogResponse {
        capabilities: load_capabilities(&connection)?,
    })
}

/// Loads the raw list of all capabilities from the active database connection.
pub fn load_capabilities(connection: &Connection) -> Result<Vec<CapabilityDefinition>> {
    connection.query_many(
        "SELECT id, label, description, family, input_schema_json, output_contract_json,
                roles_allowed_json, execution_target, timeout_seconds, retry_policy_json,
                artifact_kinds_json, scheduler_eligible, prompt_exposure, mcp_export_policy,
                side_effects_json, approval_requirement
         FROM capabilities
         ORDER BY family ASC, id ASC",
        [],
        capability_from_row,
    )
}

/// Filters and returns only capabilities that are configured for export
/// over the Model Context Protocol (MCP) using a specific export boundary.
pub fn list_mcp_exported_capabilities(
    connection: &Connection,
) -> Result<Vec<CapabilityDefinition>> {
    connection.query_many(
        "SELECT id, label, description, family, input_schema_json, output_contract_json,
                roles_allowed_json, execution_target, timeout_seconds, retry_policy_json,
                artifact_kinds_json, scheduler_eligible, prompt_exposure, mcp_export_policy,
                side_effects_json, approval_requirement
         FROM capabilities
         WHERE mcp_export_policy IN (?1, ?2, ?3)
         ORDER BY family ASC, id ASC",
        params![
            MCP_EXPORT_POLICY_READ_ONLY,
            MCP_EXPORT_POLICY_LOCAL_MUTATION,
            MCP_EXPORT_POLICY_OPERATOR_CONFIRMED
        ],
        capability_from_row,
    )
}

/// Fetches a single capability by its canonical string ID.
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

/// Asserts that all listed capability IDs are formally registered in the local schema.
/// Returns an error listing the unknown capability IDs if any are absent.
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
            "conversation.read",
            "conversation.handoff.manage",
            "conversation.agent.delegate",
            "conversation.episode.manage",
            "conversation.message.create",
            "conversation.message.edit",
            "conversation.message.delete",
            "conversation.reaction.write",
            "conversation.receipt.write",
            "conversation.presence.write",
            "llm.invoke",
            "llm.cancel",
            "llm.tool.request",
            "llm.tool.approve",
            "llm.tool.reject",
            "llm.tool.execute",
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
        use crate::route_contracts::protected_route_capability_ids;

        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        let capability_ids = protected_route_capability_ids()
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
