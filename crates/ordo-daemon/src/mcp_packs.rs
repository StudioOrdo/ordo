use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::path::Path;

use crate::capabilities::{load_capability, MCP_EXPORT_POLICY_DANGEROUS_NONE};
use crate::schema::db::ConnectionExt;
use crate::policy::{
    provenance_metadata, ActorContext, ActorKind, PolicyAction, ResourceClassification,
    ResourceKind, ResourceRef,
};

const PACK_STATUS_ENABLED: &str = "enabled";
const PACK_STATUS_DISABLED: &str = "disabled";
const TOOL_EXPORT_EXPORTED: &str = "exported";
const TOOL_EXPORT_BLOCKED: &str = "blocked";
const TOOL_EXPORT_DISABLED: &str = "disabled";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPackInstallRequest {
    pub manifest: McpPackManifest,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPackManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    #[serde(default)]
    pub tools: Vec<McpPackToolManifest>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPackToolManifest {
    pub name: String,
    pub capability_id: String,
    pub input_schema: Value,
    pub output_contract: Value,
    #[serde(default)]
    pub side_effects: Vec<String>,
    pub approval_requirement: String,
    #[serde(default)]
    pub artifact_kinds: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPackListResponse {
    pub packs: Vec<McpPackView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPackResponse {
    pub pack: McpPackView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPackView {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: String,
    pub manifest: Value,
    pub provenance: Value,
    pub tools: Vec<McpPackToolView>,
    pub created_by_actor_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPackToolView {
    pub id: String,
    pub pack_id: String,
    pub tool_name: String,
    pub capability_id: String,
    pub input_schema: Value,
    pub output_contract: Value,
    pub side_effects: Vec<String>,
    pub approval_requirement: String,
    pub artifact_kinds: Vec<String>,
    pub mcp_export_policy: String,
    pub export_status: String,
    pub disabled_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

struct McpPackRecord {
    id: String,
    name: String,
    version: String,
    status: String,
    manifest: Value,
    provenance: Value,
    created_by_actor_id: Option<String>,
    created_at: String,
    updated_at: String,
}

pub fn list_mcp_packs(db_path: &Path) -> Result<McpPackListResponse> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, name, version, status, manifest_json, provenance_json,
                created_by_actor_id, created_at, updated_at
         FROM mcp_packs ORDER BY updated_at DESC, id ASC",
    )?;
    let rows = statement.query_map([], mcp_pack_from_row)?;
    let mut packs = Vec::new();
    for row in rows {
        let record = row?;
        let tools = load_pack_tools(&connection, &record.id)?;
        packs.push(record.into_view(tools));
    }
    Ok(McpPackListResponse { packs })
}

pub fn read_mcp_pack(db_path: &Path, pack_id: &str) -> Result<McpPackResponse> {
    let connection = Connection::open(db_path)?;
    let record = require_mcp_pack(&connection, pack_id)?;
    let tools = load_pack_tools(&connection, pack_id)?;
    Ok(McpPackResponse {
        pack: record.into_view(tools),
    })
}

pub fn install_mcp_pack(
    db_path: &Path,
    request: McpPackInstallRequest,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<McpPackResponse> {
    let mut connection = Connection::open(db_path)?;
    validate_pack_manifest(&connection, &request.manifest)?;
    let transaction = connection.transaction()?;
    let now = Utc::now().to_rfc3339();
    let provenance = pack_provenance(&request.manifest.id, origin, actor_id);
    transaction.execute(
        "INSERT INTO mcp_packs (
            id, name, version, status, manifest_json, provenance_json,
            created_by_actor_id, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            version = excluded.version,
            status = excluded.status,
            manifest_json = excluded.manifest_json,
            provenance_json = excluded.provenance_json,
            created_by_actor_id = excluded.created_by_actor_id,
            updated_at = excluded.updated_at",
        params![
            request.manifest.id,
            request.manifest.name,
            request.manifest.version,
            PACK_STATUS_ENABLED,
            serde_json::to_string(&request.manifest)?,
            provenance.to_string(),
            actor_id,
            now,
        ],
    )?;
    transaction.execute(
        "DELETE FROM mcp_pack_tools WHERE pack_id = ?1",
        [request.manifest.id.as_str()],
    )?;
    for tool in &request.manifest.tools {
        let capability = load_capability(&transaction, &tool.capability_id)?
            .ok_or_else(|| anyhow::anyhow!("Unknown capability: {}", tool.capability_id))?;
        let export_status = if capability.mcp_export_policy == MCP_EXPORT_POLICY_DANGEROUS_NONE {
            TOOL_EXPORT_BLOCKED
        } else {
            TOOL_EXPORT_EXPORTED
        };
        transaction.execute(
            "INSERT INTO mcp_pack_tools (
                id, pack_id, tool_name, capability_id, input_schema_json, output_contract_json,
                side_effects_json, approval_requirement, artifact_kinds_json, mcp_export_policy,
                export_status, disabled_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, ?12, ?12)",
            params![
                format!("{}_{}", request.manifest.id, tool.name),
                request.manifest.id,
                tool.name,
                tool.capability_id,
                tool.input_schema.to_string(),
                tool.output_contract.to_string(),
                serde_json::to_string(&tool.side_effects)?,
                tool.approval_requirement,
                serde_json::to_string(&tool.artifact_kinds)?,
                capability.mcp_export_policy,
                export_status,
                now,
            ],
        )?;
    }
    transaction.commit()?;
    read_mcp_pack(db_path, &request.manifest.id)
}

pub fn disable_mcp_pack(
    db_path: &Path,
    pack_id: &str,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<McpPackResponse> {
    let connection = Connection::open(db_path)?;
    require_mcp_pack(&connection, pack_id)?;
    let now = Utc::now().to_rfc3339();
    let provenance = pack_provenance(pack_id, origin, actor_id);
    connection.execute(
        "UPDATE mcp_packs
         SET status = ?2, provenance_json = ?3, updated_at = ?4
         WHERE id = ?1",
        params![pack_id, PACK_STATUS_DISABLED, provenance.to_string(), now],
    )?;
    connection.execute(
        "UPDATE mcp_pack_tools
         SET export_status = ?2, disabled_at = ?3, updated_at = ?3
         WHERE pack_id = ?1",
        params![pack_id, TOOL_EXPORT_DISABLED, now],
    )?;
    read_mcp_pack(db_path, pack_id)
}

pub fn mcp_tool_is_enabled(connection: &Connection, capability_id: &str) -> Result<bool> {
    let mut statement = connection.prepare(
        "SELECT p.status, t.export_status
         FROM mcp_pack_tools t
         JOIN mcp_packs p ON p.id = t.pack_id
         WHERE t.capability_id = ?1",
    )?;
    let rows = statement.query_map([capability_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let states = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    if states.is_empty() {
        return Ok(true);
    }
    Ok(states.iter().any(|(pack_status, export_status)| {
        pack_status == PACK_STATUS_ENABLED && export_status == TOOL_EXPORT_EXPORTED
    }))
}

fn validate_pack_manifest(connection: &Connection, manifest: &McpPackManifest) -> Result<()> {
    require_identifier(&manifest.id, "Pack id")?;
    require_non_empty(&manifest.name, "Pack name")?;
    require_non_empty(&manifest.version, "Pack version")?;
    if manifest.tools.is_empty() {
        bail!("MCP pack manifest requires at least one tool");
    }
    let mut tool_names = BTreeSet::new();
    for tool in &manifest.tools {
        require_identifier(&tool.name, "Tool name")?;
        if !tool_names.insert(tool.name.clone()) {
            bail!("Duplicate MCP pack tool name: {}", tool.name);
        }
        require_identifier(&tool.capability_id, "Tool capability id")?;
        validate_json_schema(&tool.input_schema, "inputSchema")?;
        validate_json_schema(&tool.output_contract, "outputContract")?;
        validate_side_effects(&tool.side_effects)?;
        require_non_empty(&tool.approval_requirement, "Tool approval requirement")?;
        let capability = load_capability(connection, &tool.capability_id)?
            .ok_or_else(|| anyhow::anyhow!("Unknown capability: {}", tool.capability_id))?;
        if tool.input_schema != capability.input_schema {
            bail!(
                "Tool {} input schema must match registered capability {}",
                tool.name,
                tool.capability_id
            );
        }
        if tool.output_contract != capability.output_contract {
            bail!(
                "Tool {} output contract must match registered capability {}",
                tool.name,
                tool.capability_id
            );
        }
        if tool.approval_requirement != capability.approval_requirement {
            bail!(
                "Tool {} approval requirement must match registered capability {}",
                tool.name,
                tool.capability_id
            );
        }
        if tool.side_effects != capability.side_effects {
            bail!(
                "Tool {} side effects must match registered capability {}",
                tool.name,
                tool.capability_id
            );
        }
    }
    Ok(())
}

pub fn validate_json_schema(schema: &Value, label: &str) -> Result<()> {
    let object = schema
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("{label} must be a JSON object"))?;
    let schema_type = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("{label} requires a type"))?;
    if !matches!(
        schema_type,
        "object" | "array" | "boolean" | "integer" | "number" | "string"
    ) {
        bail!("{label} has unsupported type: {schema_type}");
    }
    if let Some(required) = object.get("required") {
        let required = required
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("{label} required must be an array"))?;
        for entry in required {
            if entry.as_str().is_none() {
                bail!("{label} required entries must be strings");
            }
        }
    }
    if let Some(properties) = object.get("properties") {
        let properties = properties
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("{label} properties must be an object"))?;
        validate_property_schemas(properties, label)?;
    }
    if let Some(additional) = object.get("additionalProperties") {
        if !(additional.is_boolean() || additional.is_object()) {
            bail!("{label} additionalProperties must be a boolean or object");
        }
    }
    Ok(())
}

fn validate_property_schemas(properties: &Map<String, Value>, label: &str) -> Result<()> {
    for (name, property_schema) in properties {
        let object = property_schema
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("{label} property {name} schema must be an object"))?;
        let Some(schema_type) = object.get("type").and_then(Value::as_str) else {
            continue;
        };
        if !matches!(
            schema_type,
            "object" | "array" | "boolean" | "integer" | "number" | "string"
        ) {
            bail!("{label} property {name} has unsupported type: {schema_type}");
        }
    }
    Ok(())
}

fn validate_side_effects(side_effects: &[String]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for effect in side_effects {
        require_identifier(effect, "Side effect")?;
        if !seen.insert(effect) {
            bail!("Duplicate side effect: {effect}");
        }
    }
    Ok(())
}

fn require_identifier(value: &str, label: &str) -> Result<()> {
    let normalized = require_non_empty(value, label)?;
    if normalized
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        Ok(())
    } else {
        bail!("{label} may only contain ASCII letters, numbers, dots, underscores, or hyphens")
    }
}

fn require_non_empty(value: &str, label: &str) -> Result<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        bail!("{label} is required");
    }
    Ok(normalized)
}

fn require_mcp_pack(connection: &Connection, pack_id: &str) -> Result<McpPackRecord> {
    connection
        .query_row(
            "SELECT id, name, version, status, manifest_json, provenance_json,
                    created_by_actor_id, created_at, updated_at
             FROM mcp_packs WHERE id = ?1",
            [pack_id],
            mcp_pack_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("MCP pack was not found: {pack_id}"))
}

fn mcp_pack_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<McpPackRecord> {
    let manifest_json: String = row.get(4)?;
    let provenance_json: String = row.get(5)?;
    Ok(McpPackRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        version: row.get(2)?,
        status: row.get(3)?,
        manifest: serde_json::from_str(&manifest_json).unwrap_or_else(|_| json!({})),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        created_by_actor_id: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn load_pack_tools(connection: &Connection, pack_id: &str) -> Result<Vec<McpPackToolView>> {
    connection.query_many("SELECT id, pack_id, tool_name, capability_id, input_schema_json, output_contract_json,
                side_effects_json, approval_requirement, artifact_kinds_json, mcp_export_policy,
                export_status, disabled_at, created_at, updated_at
         FROM mcp_pack_tools WHERE pack_id = ?1 ORDER BY tool_name ASC", [pack_id], mcp_pack_tool_from_row)
}

fn mcp_pack_tool_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<McpPackToolView> {
    let input_schema_json: String = row.get(4)?;
    let output_contract_json: String = row.get(5)?;
    let side_effects_json: String = row.get(6)?;
    let artifact_kinds_json: String = row.get(8)?;
    Ok(McpPackToolView {
        id: row.get(0)?,
        pack_id: row.get(1)?,
        tool_name: row.get(2)?,
        capability_id: row.get(3)?,
        input_schema: serde_json::from_str(&input_schema_json).unwrap_or_else(|_| json!({})),
        output_contract: serde_json::from_str(&output_contract_json).unwrap_or_else(|_| json!({})),
        side_effects: serde_json::from_str(&side_effects_json).unwrap_or_default(),
        approval_requirement: row.get(7)?,
        artifact_kinds: serde_json::from_str(&artifact_kinds_json).unwrap_or_default(),
        mcp_export_policy: row.get(9)?,
        export_status: row.get(10)?,
        disabled_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

impl McpPackRecord {
    fn into_view(self, tools: Vec<McpPackToolView>) -> McpPackView {
        McpPackView {
            id: self.id,
            name: self.name,
            version: self.version,
            status: self.status,
            manifest: self.manifest,
            provenance: self.provenance,
            tools,
            created_by_actor_id: self.created_by_actor_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn pack_provenance(pack_id: &str, origin: &str, actor_id: Option<&str>) -> Value {
    provenance_metadata(
        actor_context_for_origin(origin, actor_id),
        PolicyAction::Validate,
        ResourceRef::new(ResourceKind::McpPack, pack_id),
        Some("mcp.packs.write"),
        ResourceClassification::local_operations_ready_for_review(),
    )
}

fn actor_context_for_origin(origin: &str, actor_id: Option<&str>) -> ActorContext {
    let kind = match origin {
        "mcp" => ActorKind::McpClient,
        "scheduler" => ActorKind::Scheduler,
        "system" => ActorKind::System,
        _ => ActorKind::BrowserOperator,
    };
    ActorContext::new(kind, origin, actor_id.map(ToString::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::{
        load_capability, seed_builtin_capabilities, MCP_EXPORT_POLICY_LOCAL_MUTATION,
    };
    use crate::policy::LOCAL_OWNER_ACTOR_ID;
    use crate::schema::init_schema;

    fn setup_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        connection
    }

    fn status_tool_manifest(connection: &Connection) -> McpPackManifest {
        let capability = load_capability(connection, "system.status.read")
            .unwrap()
            .unwrap();
        McpPackManifest {
            id: "pack.local.status".to_string(),
            name: "Local Status Pack".to_string(),
            version: "1.0.0".to_string(),
            description: Some("test pack".to_string()),
            tools: vec![McpPackToolManifest {
                name: "system.status.read".to_string(),
                capability_id: capability.id,
                input_schema: capability.input_schema,
                output_contract: capability.output_contract,
                side_effects: capability.side_effects,
                approval_requirement: capability.approval_requirement,
                artifact_kinds: capability.artifact_kinds,
            }],
        }
    }

    #[test]
    fn validates_and_installs_pack_manifest_metadata() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        crate::schema::init_database(&db_path).unwrap();
        let connection = Connection::open(&db_path).unwrap();
        let manifest = status_tool_manifest(&connection);
        drop(connection);

        let response = install_mcp_pack(
            &db_path,
            McpPackInstallRequest { manifest },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(response.pack.status, PACK_STATUS_ENABLED);
        assert_eq!(response.pack.tools.len(), 1);
        assert_eq!(response.pack.tools[0].export_status, TOOL_EXPORT_EXPORTED);
        assert_eq!(response.pack.provenance["resource"]["kind"], "mcp_pack");
    }

    #[test]
    fn rejects_manifest_schema_that_differs_from_capability_contract() {
        let connection = setup_connection();
        let mut manifest = status_tool_manifest(&connection);
        manifest.tools[0].input_schema = json!({ "type": "object", "required": [42] });

        let error = validate_pack_manifest(&connection, &manifest).unwrap_err();
        assert!(error.to_string().contains("required entries"));
    }

    #[test]
    fn blocks_dangerous_non_exported_capability_tools() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        crate::schema::init_database(&db_path).unwrap();
        let connection = Connection::open(&db_path).unwrap();
        let capability = load_capability(&connection, "restore.execute")
            .unwrap()
            .unwrap();
        let manifest = McpPackManifest {
            id: "pack.local.restore".to_string(),
            name: "Restore Pack".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            tools: vec![McpPackToolManifest {
                name: "restore.execute".to_string(),
                capability_id: capability.id,
                input_schema: capability.input_schema,
                output_contract: capability.output_contract,
                side_effects: capability.side_effects,
                approval_requirement: capability.approval_requirement,
                artifact_kinds: capability.artifact_kinds,
            }],
        };
        drop(connection);

        let response = install_mcp_pack(
            &db_path,
            McpPackInstallRequest { manifest },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(response.pack.tools[0].export_status, TOOL_EXPORT_BLOCKED);
        assert!(
            !mcp_tool_is_enabled(&Connection::open(&db_path).unwrap(), "restore.execute").unwrap()
        );
    }

    #[test]
    fn disabling_pack_hides_exported_tools() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        crate::schema::init_database(&db_path).unwrap();
        let connection = Connection::open(&db_path).unwrap();
        let manifest = status_tool_manifest(&connection);
        drop(connection);
        install_mcp_pack(
            &db_path,
            McpPackInstallRequest { manifest },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let response = disable_mcp_pack(&db_path, "pack.local.status", "test", None).unwrap();

        assert_eq!(response.pack.status, PACK_STATUS_DISABLED);
        assert_eq!(response.pack.tools[0].export_status, TOOL_EXPORT_DISABLED);
        assert!(
            !mcp_tool_is_enabled(&Connection::open(&db_path).unwrap(), "system.status.read")
                .unwrap()
        );
    }

    #[test]
    fn validates_side_effect_metadata() {
        let connection = setup_connection();
        let mut manifest = status_tool_manifest(&connection);
        manifest.tools[0].side_effects = vec!["bad effect".to_string()];

        let error = validate_pack_manifest(&connection, &manifest).unwrap_err();
        assert!(error.to_string().contains("Side effect"));
    }

    #[test]
    fn rejects_unknown_arbitrary_execution_capabilities() {
        let connection = setup_connection();
        let mut manifest = status_tool_manifest(&connection);
        manifest.tools[0].name = "shell.run".to_string();
        manifest.tools[0].capability_id = "shell.run".to_string();

        let error = validate_pack_manifest(&connection, &manifest).unwrap_err();
        assert!(error.to_string().contains("Unknown capability"));
    }

    #[test]
    fn records_local_mutation_side_effect_and_approval_metadata() {
        let connection = setup_connection();
        let capability = load_capability(&connection, "backup.create")
            .unwrap()
            .unwrap();
        assert_eq!(
            capability.mcp_export_policy,
            MCP_EXPORT_POLICY_LOCAL_MUTATION
        );
        let manifest = McpPackManifest {
            id: "pack.local.backup".to_string(),
            name: "Backup Pack".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            tools: vec![McpPackToolManifest {
                name: "backup.create".to_string(),
                capability_id: capability.id,
                input_schema: capability.input_schema,
                output_contract: capability.output_contract,
                side_effects: capability.side_effects,
                approval_requirement: capability.approval_requirement,
                artifact_kinds: capability.artifact_kinds,
            }],
        };
        validate_pack_manifest(&connection, &manifest).unwrap();
    }
}
