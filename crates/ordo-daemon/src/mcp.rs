use anyhow::{bail, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

use crate::backups::{
    create_backup, list_backup_restore_jobs, run_restore_preflight, RestorePreflightRequest,
};
use crate::briefs::{generate_system_brief, latest_system_brief};
use crate::capabilities::{list_mcp_exported_capabilities, load_capabilities};
use crate::health::{build_health_report, build_readiness_report};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRequest {
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

pub fn handle_mcp_request(db_path: &Path, request: McpRequest) -> McpResponse {
    let request_id = request.id.unwrap_or(Value::Null);
    match dispatch_mcp_request(
        db_path,
        &request.method,
        request.params.unwrap_or_else(|| json!({})),
    ) {
        Ok(result) => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            result: Some(result),
            error: None,
        },
        Err(error) => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            result: None,
            error: Some(McpError {
                code: -32000,
                message: error.to_string(),
            }),
        },
    }
}

fn dispatch_mcp_request(db_path: &Path, method: &str, params: Value) -> Result<Value> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "ordo-daemon", "version": env!("CARGO_PKG_VERSION") }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => list_tools(db_path),
        "tools/call" => call_tool(db_path, params),
        other => bail!("Unsupported MCP method: {other}"),
    }
}

fn list_tools(db_path: &Path) -> Result<Value> {
    let connection = Connection::open(db_path)?;
    let tools: Vec<Value> = list_mcp_exported_capabilities(&connection)?
        .into_iter()
        .map(|capability| {
            json!({
                "name": capability.id,
                "title": capability.label,
                "description": capability.description,
                "inputSchema": capability.input_schema,
            })
        })
        .collect();
    Ok(json!({ "tools": tools }))
}

fn call_tool(db_path: &Path, params: Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("tools/call requires params.name"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let connection = Connection::open(db_path)?;
    let exported = list_mcp_exported_capabilities(&connection)?;
    if !exported.iter().any(|capability| capability.id == name) {
        bail!("Capability is not exported to MCP: {name}");
    }
    drop(connection);

    let structured_content = match name {
        "capability.catalog.list" => {
            json!({ "capabilities": load_capabilities(&Connection::open(db_path)?)? })
        }
        "system.status.read" | "appliance.runtime.status" => json!({
            "health": build_health_report(),
            "readiness": build_readiness_report(db_path),
        }),
        "brief.system.latest" => json!({ "brief": latest_system_brief(db_path)? }),
        "brief.system.generate" => json!({ "brief": generate_system_brief(db_path, "mcp", None)? }),
        "backup.restore_jobs.list" => json!(list_backup_restore_jobs(db_path)?),
        "backup.create" => json!({ "job": create_backup(db_path, "mcp", None)? }),
        "restore.preflight.validate" => {
            let request: RestorePreflightRequest = serde_json::from_value(arguments)?;
            json!({ "job": run_restore_preflight(db_path, request, "mcp", None)? })
        }
        other => bail!("MCP tool has no projection handler: {other}"),
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&structured_content)?,
        }],
        "structuredContent": structured_content,
        "isError": false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_database;
    use tempfile::TempDir;

    #[test]
    fn mcp_lists_safe_system_tools() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        let response = handle_mcp_request(
            &db_path,
            McpRequest {
                id: Some(json!(1)),
                method: "tools/list".to_string(),
                params: None,
            },
        );

        assert!(response.error.is_none());
        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert!(tools
            .iter()
            .any(|tool| tool["name"] == "system.status.read"));
    }

    #[test]
    fn mcp_system_status_reads_daemon_state() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        let response = handle_mcp_request(
            &db_path,
            McpRequest {
                id: Some(json!(2)),
                method: "tools/call".to_string(),
                params: Some(json!({ "name": "system.status.read", "arguments": {} })),
            },
        );

        assert!(response.error.is_none());
        let result = response.result.unwrap();
        assert_eq!(result["structuredContent"]["health"]["status"], "ok");
        assert_eq!(result["structuredContent"]["readiness"]["status"], "ready");
    }
}
