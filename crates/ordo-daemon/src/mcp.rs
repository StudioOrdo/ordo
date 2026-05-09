use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::path::Path;

use crate::backups::{
    create_backup, list_backup_restore_jobs, run_restore_preflight, RestorePreflightRequest,
};
use crate::briefs::{generate_system_brief, latest_system_brief};
use crate::capabilities::{list_mcp_exported_capabilities, load_capabilities};
use crate::health::{build_health_report, build_readiness_report};
use crate::mcp_packs::{mcp_tool_is_enabled, validate_json_schema};
use crate::policy::{
    authorize_mcp_capability, record_policy_decision, ActorContext, PolicyDecisionCorrelation,
    PolicyOutcome, LOCAL_OWNER_ACTOR_ID,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpError {
    pub code: i64,
    pub message: String,
}

const JSONRPC_VERSION: &str = "2.0";
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

#[derive(Debug)]
struct McpDispatchError {
    code: i64,
    message: String,
}

impl McpDispatchError {
    fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: INVALID_REQUEST,
            message: message.into(),
        }
    }

    fn method_not_found(method: &str) -> Self {
        Self {
            code: METHOD_NOT_FOUND,
            message: format!("Method not found: {method}"),
        }
    }

    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: INVALID_PARAMS,
            message: message.into(),
        }
    }

    fn internal(error: anyhow::Error) -> Self {
        Self {
            code: INTERNAL_ERROR,
            message: error.to_string(),
        }
    }
}

type McpDispatchResult<T> = std::result::Result<T, McpDispatchError>;

pub fn handle_mcp_request(db_path: &Path, request: McpRequest) -> McpResponse {
    let mut request_object = Map::new();
    request_object.insert("jsonrpc".to_string(), Value::String(request.jsonrpc));
    if let Some(id) = request.id {
        request_object.insert("id".to_string(), id);
    }
    request_object.insert("method".to_string(), Value::String(request.method));
    if let Some(params) = request.params {
        request_object.insert("params".to_string(), params);
    }
    handle_mcp_value(db_path, Value::Object(request_object))
}

pub fn handle_mcp_json(db_path: &Path, request_body: &str) -> McpResponse {
    match serde_json::from_str::<Value>(request_body) {
        Ok(value) => handle_mcp_value(db_path, value),
        Err(_) => error_response(Value::Null, PARSE_ERROR, "Parse error"),
    }
}

pub fn handle_mcp_value(db_path: &Path, request: Value) -> McpResponse {
    let request_id = request_id_from_value(&request).unwrap_or(Value::Null);
    let request = match validate_mcp_request(request) {
        Ok(request) => request,
        Err(error) => return error_response(request_id, error.code, error.message),
    };

    let request_id = request.id.unwrap_or(Value::Null);
    match dispatch_mcp_request(db_path, &request.method, request.params) {
        Ok(result) => McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: request_id,
            result: Some(result),
            error: None,
        },
        Err(error) => McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: request_id,
            result: None,
            error: Some(McpError {
                code: error.code,
                message: error.message,
            }),
        },
    }
}

fn error_response(id: Value, code: i64, message: impl Into<String>) -> McpResponse {
    McpResponse {
        jsonrpc: JSONRPC_VERSION.to_string(),
        id,
        result: None,
        error: Some(McpError {
            code,
            message: message.into(),
        }),
    }
}

fn request_id_from_value(request: &Value) -> Option<Value> {
    let id = request.get("id")?;
    is_valid_request_id(id).then(|| id.clone())
}

fn validate_mcp_request(request: Value) -> McpDispatchResult<McpRequest> {
    let object = request
        .as_object()
        .ok_or_else(|| McpDispatchError::invalid_request("JSON-RPC request must be an object"))?;
    let jsonrpc = object
        .get("jsonrpc")
        .and_then(Value::as_str)
        .ok_or_else(|| McpDispatchError::invalid_request("JSON-RPC request requires jsonrpc"))?;
    if jsonrpc != JSONRPC_VERSION {
        return Err(McpDispatchError::invalid_request(
            "JSON-RPC request jsonrpc must be \"2.0\"",
        ));
    }

    if let Some(id) = object.get("id") {
        if !is_valid_request_id(id) {
            return Err(McpDispatchError::invalid_request(
                "JSON-RPC request id must be a string, number, or null",
            ));
        }
    }

    let method = object
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| McpDispatchError::invalid_request("JSON-RPC request requires method"))?;
    if method.trim().is_empty() {
        return Err(McpDispatchError::invalid_request(
            "JSON-RPC request method must not be empty",
        ));
    }

    let params = object.get("params").cloned();
    if let Some(params) = &params {
        if !params.is_object() {
            return Err(McpDispatchError::invalid_request(
                "JSON-RPC request params must be an object when present",
            ));
        }
    }

    Ok(McpRequest {
        jsonrpc: jsonrpc.to_string(),
        id: object.get("id").cloned(),
        method: method.to_string(),
        params,
    })
}

fn is_valid_request_id(id: &Value) -> bool {
    id.is_null() || id.is_string() || id.is_number()
}

fn dispatch_mcp_request(
    db_path: &Path,
    method: &str,
    params: Option<Value>,
) -> McpDispatchResult<Value> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "ordo-daemon", "version": env!("CARGO_PKG_VERSION") }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => list_tools(db_path),
        "tools/call" => call_tool(db_path, params),
        other => Err(McpDispatchError::method_not_found(other)),
    }
}

fn list_tools(db_path: &Path) -> McpDispatchResult<Value> {
    let connection =
        Connection::open(db_path).map_err(|error| McpDispatchError::internal(error.into()))?;
    let tools: Vec<Value> = list_mcp_exported_capabilities(&connection)
        .map_err(McpDispatchError::internal)?
        .into_iter()
        .filter_map(
            |capability| match mcp_tool_is_enabled(&connection, &capability.id) {
                Ok(true) => Some(Ok(capability)),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            },
        )
        .collect::<Result<Vec<_>, _>>()
        .map_err(McpDispatchError::internal)?
        .into_iter()
        .map(|capability| {
            json!({
                "name": capability.id,
                "title": capability.label,
                "description": capability.description,
                "inputSchema": capability.input_schema,
                "metadata": {
                    "mcpExportPolicy": capability.mcp_export_policy,
                    "sideEffects": capability.side_effects,
                    "approvalRequirement": capability.approval_requirement,
                },
            })
        })
        .collect();
    Ok(json!({ "tools": tools }))
}

fn call_tool(db_path: &Path, params: Option<Value>) -> McpDispatchResult<Value> {
    let params =
        params.ok_or_else(|| McpDispatchError::invalid_params("tools/call requires params"))?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| McpDispatchError::invalid_params("tools/call requires params.name"))?;
    if name.trim().is_empty() {
        return Err(McpDispatchError::invalid_params(
            "tools/call params.name must not be empty",
        ));
    }
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !arguments.is_object() {
        return Err(McpDispatchError::invalid_params(
            "tools/call params.arguments must be an object when present",
        ));
    }

    let connection =
        Connection::open(db_path).map_err(|error| McpDispatchError::internal(error.into()))?;
    let exported =
        list_mcp_exported_capabilities(&connection).map_err(McpDispatchError::internal)?;
    let capability = exported
        .iter()
        .find(|capability| capability.id == name)
        .ok_or_else(|| {
            McpDispatchError::invalid_params(format!("Capability is not exported to MCP: {name}"))
        })?;
    if !mcp_tool_is_enabled(&connection, &capability.id).map_err(McpDispatchError::internal)? {
        return Err(McpDispatchError::invalid_params(format!(
            "MCP tool is disabled or blocked by pack metadata: {name}"
        )));
    }
    validate_json_schema(&capability.input_schema, "inputSchema")
        .map_err(|error| McpDispatchError::invalid_params(error.to_string()))?;
    let policy_decision =
        authorize_mcp_capability(&connection, ActorContext::mcp_client(), capability);
    record_policy_decision(
        &connection,
        &policy_decision,
        PolicyDecisionCorrelation::default(),
    )
    .map_err(|error| McpDispatchError::internal(error.into()))?;
    if policy_decision.outcome == PolicyOutcome::Denied {
        return Err(McpDispatchError::invalid_params(policy_decision.reason));
    }
    validate_tool_arguments_against_schema(name, &arguments, &capability.input_schema)?;
    drop(connection);

    let structured_content = match name {
        "capability.catalog.list" => {
            let connection = Connection::open(db_path)
                .map_err(|error| McpDispatchError::internal(error.into()))?;
            json!({ "capabilities": load_capabilities(&connection).map_err(McpDispatchError::internal)? })
        }
        "system.status.read" | "appliance.runtime.status" => json!({
            "health": build_health_report(),
            "readiness": build_readiness_report(db_path),
        }),
        "brief.system.latest" => {
            json!({ "brief": latest_system_brief(db_path).map_err(McpDispatchError::internal)? })
        }
        "brief.system.generate" => {
            json!({ "brief": generate_system_brief(db_path, "mcp", Some(LOCAL_OWNER_ACTOR_ID)).map_err(McpDispatchError::internal)? })
        }
        "backup.restore_jobs.list" => {
            json!(list_backup_restore_jobs(db_path).map_err(McpDispatchError::internal)?)
        }
        "backup.create" => {
            json!({ "job": create_backup(db_path, "mcp", Some(LOCAL_OWNER_ACTOR_ID)).map_err(McpDispatchError::internal)? })
        }
        "restore.preflight.validate" => {
            let request: RestorePreflightRequest = serde_json::from_value(arguments)
                .map_err(|error| McpDispatchError::invalid_params(error.to_string()))?;
            json!({ "job": run_restore_preflight(db_path, request, "mcp", Some(LOCAL_OWNER_ACTOR_ID)).map_err(McpDispatchError::internal)? })
        }
        other => {
            return Err(McpDispatchError::invalid_params(format!(
                "MCP tool has no projection handler: {other}"
            )))
        }
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&structured_content)
                .map_err(|error| McpDispatchError::internal(error.into()))?,
        }],
        "structuredContent": structured_content,
        "ordoPolicy": policy_decision.metadata(),
        "isError": false,
    }))
}

fn validate_tool_arguments_against_schema(
    tool_name: &str,
    arguments: &Value,
    schema: &Value,
) -> McpDispatchResult<()> {
    let schema_object = schema.as_object().ok_or_else(|| {
        McpDispatchError::invalid_params(format!("{tool_name} input schema must be an object"))
    })?;
    if schema_object.get("type").and_then(Value::as_str) != Some("object") {
        return Ok(());
    }

    let arguments_object = arguments.as_object().ok_or_else(|| {
        McpDispatchError::invalid_params(format!("{tool_name} arguments must be an object"))
    })?;

    validate_required_properties(tool_name, arguments_object, schema_object)?;
    validate_known_properties(tool_name, arguments_object, schema_object)?;
    validate_property_types(tool_name, arguments_object, schema_object)?;
    Ok(())
}

fn validate_required_properties(
    tool_name: &str,
    arguments: &Map<String, Value>,
    schema: &Map<String, Value>,
) -> McpDispatchResult<()> {
    let Some(required) = schema.get("required").and_then(Value::as_array) else {
        return Ok(());
    };
    for property in required {
        let property = property.as_str().ok_or_else(|| {
            McpDispatchError::invalid_params(format!(
                "{tool_name} input schema required entries must be strings"
            ))
        })?;
        if !arguments.contains_key(property) {
            return Err(McpDispatchError::invalid_params(format!(
                "{tool_name} arguments missing required property: {property}"
            )));
        }
    }
    Ok(())
}

fn validate_known_properties(
    tool_name: &str,
    arguments: &Map<String, Value>,
    schema: &Map<String, Value>,
) -> McpDispatchResult<()> {
    if schema.get("additionalProperties") != Some(&Value::Bool(false)) {
        return Ok(());
    }
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for property in arguments.keys() {
        if !properties.contains_key(property) {
            return Err(McpDispatchError::invalid_params(format!(
                "{tool_name} arguments contain unsupported property: {property}"
            )));
        }
    }
    Ok(())
}

fn validate_property_types(
    tool_name: &str,
    arguments: &Map<String, Value>,
    schema: &Map<String, Value>,
) -> McpDispatchResult<()> {
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return Ok(());
    };
    for (property, property_schema) in properties {
        let Some(value) = arguments.get(property) else {
            continue;
        };
        let Some(expected_type) = property_schema.get("type").and_then(Value::as_str) else {
            continue;
        };
        if !json_value_matches_type(value, expected_type) {
            return Err(McpDispatchError::invalid_params(format!(
                "{tool_name} arguments property {property} must be {expected_type}"
            )));
        }
    }
    Ok(())
}

fn json_value_matches_type(value: &Value, expected_type: &str) -> bool {
    match expected_type {
        "array" => value.is_array(),
        "boolean" => value.is_boolean(),
        "integer" => value.as_i64().is_some(),
        "number" => value.is_number(),
        "object" => value.is_object(),
        "string" => value.is_string(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::load_capability;
    use crate::mcp_packs::{
        disable_mcp_pack, install_mcp_pack, McpPackInstallRequest, McpPackManifest,
        McpPackToolManifest,
    };
    use crate::schema::init_database;
    use tempfile::TempDir;

    fn test_db_path() -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        (temp_dir, db_path)
    }

    fn request(method: &str, params: Option<Value>) -> McpRequest {
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: method.to_string(),
            params,
        }
    }

    fn assert_error(response: McpResponse, code: i64, message_contains: &str) {
        let error = response.error.unwrap();
        assert_eq!(error.code, code);
        assert!(
            error.message.contains(message_contains),
            "expected message to contain {message_contains:?}, got {:?}",
            error.message
        );
    }

    #[test]
    fn mcp_lists_tools_with_policy_metadata() {
        let (_temp_dir, db_path) = test_db_path();

        let response = handle_mcp_request(&db_path, request("tools/list", None));

        assert!(response.error.is_none());
        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert!(tools
            .iter()
            .any(|tool| tool["name"] == "system.status.read"));
        let backup_create = tools
            .iter()
            .find(|tool| tool["name"] == "backup.create")
            .unwrap();
        assert_eq!(
            backup_create["metadata"]["mcpExportPolicy"],
            "local_mutation"
        );
        assert_eq!(
            backup_create["metadata"]["approvalRequirement"],
            "local_access_required"
        );
        assert!(backup_create["metadata"]["sideEffects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|effect| effect == "writes_backup_archive"));
        assert!(!tools.iter().any(|tool| tool["name"] == "restore.execute"));
    }

    #[test]
    fn mcp_system_status_reads_daemon_state() {
        let (_temp_dir, db_path) = test_db_path();

        let response = handle_mcp_request(
            &db_path,
            request(
                "tools/call",
                Some(json!({ "name": "system.status.read", "arguments": {} })),
            ),
        );

        assert!(response.error.is_none());
        let result = response.result.unwrap();
        assert_eq!(result["ordoPolicy"]["outcome"], "allowed");
        assert_eq!(result["ordoPolicy"]["actor"]["kind"], "mcp_client");
        assert_eq!(result["structuredContent"]["health"]["status"], "ok");
        assert_eq!(result["structuredContent"]["readiness"]["status"], "ready");

        let connection = Connection::open(&db_path).unwrap();
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id = 'system.status.read' AND outcome = 'allowed'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn mcp_review_required_policy_decisions_are_persisted_before_argument_validation() {
        let (_temp_dir, db_path) = test_db_path();

        let response = handle_mcp_request(
            &db_path,
            request(
                "tools/call",
                Some(
                    json!({ "name": "restore.preflight.validate", "arguments": { "backupId": "backup_1" } }),
                ),
            ),
        );

        assert_error(response, INVALID_PARAMS, "missing required property");
        let connection = Connection::open(&db_path).unwrap();
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id = 'restore.preflight.validate' AND outcome = 'review_required'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn malformed_json_returns_parse_error() {
        let (_temp_dir, db_path) = test_db_path();

        let response = handle_mcp_json(&db_path, "{");

        assert_error(response, PARSE_ERROR, "Parse error");
    }

    #[test]
    fn invalid_request_requires_jsonrpc_version() {
        let (_temp_dir, db_path) = test_db_path();

        let response = handle_mcp_value(&db_path, json!({ "id": 1, "method": "ping" }));

        assert_error(response, INVALID_REQUEST, "requires jsonrpc");
    }

    #[test]
    fn invalid_request_rejects_bad_id_shape() {
        let (_temp_dir, db_path) = test_db_path();

        let response = handle_mcp_value(
            &db_path,
            json!({ "jsonrpc": "2.0", "id": { "bad": true }, "method": "ping" }),
        );

        assert_error(response, INVALID_REQUEST, "id must be");
    }

    #[test]
    fn unknown_method_returns_method_not_found() {
        let (_temp_dir, db_path) = test_db_path();

        let response = handle_mcp_request(&db_path, request("missing/method", None));

        assert_error(response, METHOD_NOT_FOUND, "Method not found");
    }

    #[test]
    fn tools_call_requires_tool_name() {
        let (_temp_dir, db_path) = test_db_path();

        let response = handle_mcp_request(
            &db_path,
            request("tools/call", Some(json!({ "arguments": {} }))),
        );

        assert_error(response, INVALID_PARAMS, "params.name");
    }

    #[test]
    fn tools_call_rejects_unexported_tool_before_dispatch() {
        let (_temp_dir, db_path) = test_db_path();

        let response = handle_mcp_request(
            &db_path,
            request(
                "tools/call",
                Some(json!({ "name": "restore.execute", "arguments": {} })),
            ),
        );

        assert_error(response, INVALID_PARAMS, "not exported");
    }

    #[test]
    fn tools_call_rejects_unsupported_arguments_before_dispatch() {
        let (_temp_dir, db_path) = test_db_path();

        let response = handle_mcp_request(
            &db_path,
            request(
                "tools/call",
                Some(json!({ "name": "backup.create", "arguments": { "extra": true } })),
            ),
        );

        assert_error(response, INVALID_PARAMS, "unsupported property");
    }

    #[test]
    fn tools_call_rejects_missing_required_argument_before_dispatch() {
        let (_temp_dir, db_path) = test_db_path();

        let response = handle_mcp_request(
            &db_path,
            request(
                "tools/call",
                Some(
                    json!({ "name": "restore.preflight.validate", "arguments": { "backupId": "backup_1" } }),
                ),
            ),
        );

        assert_error(response, INVALID_PARAMS, "missing required property");
    }

    #[test]
    fn tools_call_rejects_wrong_argument_type_before_dispatch() {
        let (_temp_dir, db_path) = test_db_path();

        let response = handle_mcp_request(
            &db_path,
            request(
                "tools/call",
                Some(json!({
                    "name": "restore.preflight.validate",
                    "arguments": { "backupId": 42, "confirmation": "RESTORE backup_1" }
                })),
            ),
        );

        assert_error(response, INVALID_PARAMS, "backupId must be string");
    }

    #[test]
    fn disabled_pack_hides_and_blocks_mcp_tool_projection() {
        let (_temp_dir, db_path) = test_db_path();
        let connection = Connection::open(&db_path).unwrap();
        let capability = load_capability(&connection, "system.status.read")
            .unwrap()
            .unwrap();
        drop(connection);
        install_mcp_pack(
            &db_path,
            McpPackInstallRequest {
                manifest: McpPackManifest {
                    id: "pack.local.status".to_string(),
                    name: "Local Status Pack".to_string(),
                    version: "1.0.0".to_string(),
                    description: None,
                    tools: vec![McpPackToolManifest {
                        name: "system.status.read".to_string(),
                        capability_id: capability.id,
                        input_schema: capability.input_schema,
                        output_contract: capability.output_contract,
                        side_effects: capability.side_effects,
                        approval_requirement: capability.approval_requirement,
                        artifact_kinds: capability.artifact_kinds,
                    }],
                },
            },
            "test",
            None,
        )
        .unwrap();
        disable_mcp_pack(&db_path, "pack.local.status", "test", None).unwrap();

        let list_response = handle_mcp_request(&db_path, request("tools/list", None));
        assert!(list_response.error.is_none());
        let tools = list_response.result.unwrap()["tools"]
            .as_array()
            .unwrap()
            .clone();
        assert!(!tools
            .iter()
            .any(|tool| tool["name"] == "system.status.read"));

        let call_response = handle_mcp_request(
            &db_path,
            request(
                "tools/call",
                Some(json!({ "name": "system.status.read", "arguments": {} })),
            ),
        );

        assert_error(call_response, INVALID_PARAMS, "disabled or blocked");
    }
}
