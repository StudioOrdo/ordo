use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

use crate::events::{append_realtime_event_tx, system_event, RealtimeEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionType {
    Client,
    Affiliate,
    Support,
    Service,
    WorkerOrdo,
}

impl ConnectionType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Client => "client",
            Self::Affiliate => "affiliate",
            Self::Support => "support",
            Self::Service => "service",
            Self::WorkerOrdo => "worker_ordo",
        }
    }
}

impl TryFrom<&str> for ConnectionType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "client" => Ok(Self::Client),
            "affiliate" => Ok(Self::Affiliate),
            "support" => Ok(Self::Support),
            "service" => Ok(Self::Service),
            "worker_ordo" => Ok(Self::WorkerOrdo),
            _ => bail!("Unsupported connection type: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Pending,
    Active,
    Suspended,
    Revoked,
    Archived,
}

impl ConnectionStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Revoked => "revoked",
            Self::Archived => "archived",
        }
    }
}

impl TryFrom<&str> for ConnectionStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            "revoked" => Ok(Self::Revoked),
            "archived" => Ok(Self::Archived),
            _ => bail!("Unsupported connection status: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionGrantStatus {
    Active,
    Revoked,
}

impl TryFrom<&str> for ConnectionGrantStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "revoked" => Ok(Self::Revoked),
            _ => bail!("Unsupported connection grant status: {value}"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionListResponse {
    pub connections: Vec<ConnectionView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionGrantListResponse {
    pub grants: Vec<ConnectionGrantView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionEventListResponse {
    pub events: Vec<ConnectionEventView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    pub id: String,
    pub connection_type: ConnectionType,
    pub display_name: String,
    pub status: ConnectionStatus,
    pub identity: Value,
    pub scope: Value,
    pub metadata: Value,
    pub created_by_actor_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub activated_at: Option<String>,
    pub suspended_at: Option<String>,
    pub revoked_at: Option<String>,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionGrantView {
    pub id: String,
    pub connection_id: String,
    pub resource_grant_id: String,
    pub resource_kind: String,
    pub resource_id: String,
    pub action: String,
    pub status: ConnectionGrantStatus,
    pub expires_at: Option<String>,
    pub grant_reason: Option<String>,
    pub created_by_actor_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub revoked_at: Option<String>,
    pub revoked_by_actor_id: Option<String>,
    pub revocation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionEventView {
    pub id: String,
    pub connection_id: String,
    pub event_type: String,
    pub payload: Value,
    pub receipt: Value,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionWriteRequest {
    pub connection_type: ConnectionType,
    pub display_name: String,
    pub status: Option<ConnectionStatus>,
    pub identity: Option<Value>,
    pub scope: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionGrantCreateRequest {
    pub resource_kind: String,
    pub resource_id: String,
    pub action: String,
    pub expires_at: Option<String>,
    pub grant_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionGrantRevokeRequest {
    pub revocation_reason: Option<String>,
}

#[derive(Debug, Clone)]
struct ConnectionRecord {
    id: String,
    connection_type: ConnectionType,
    display_name: String,
    status: ConnectionStatus,
    identity: Value,
    scope: Value,
    metadata: Value,
    created_by_actor_id: Option<String>,
    created_at: String,
    updated_at: String,
    activated_at: Option<String>,
    suspended_at: Option<String>,
    revoked_at: Option<String>,
    archived_at: Option<String>,
}

#[derive(Debug, Clone)]
struct ConnectionGrantRecord {
    id: String,
    connection_id: String,
    resource_grant_id: String,
    resource_kind: String,
    resource_id: String,
    action: String,
    status: ConnectionGrantStatus,
    expires_at: Option<String>,
    grant_reason: Option<String>,
    created_by_actor_id: Option<String>,
    created_at: String,
    updated_at: String,
    revoked_at: Option<String>,
    revoked_by_actor_id: Option<String>,
    revocation_reason: Option<String>,
}

pub fn list_connections(db_path: &Path) -> Result<ConnectionListResponse> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, connection_type, display_name, status, identity_json, scope_json,
                metadata_json, created_by_actor_id, created_at, updated_at, activated_at,
                suspended_at, revoked_at, archived_at
         FROM connections
         ORDER BY updated_at DESC, id DESC",
    )?;
    let connections = statement
        .query_map([], connection_from_row)?
        .map(|row| row.map(ConnectionRecord::into_view))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ConnectionListResponse { connections })
}

pub fn list_connection_grants(
    db_path: &Path,
    connection_id: &str,
) -> Result<ConnectionGrantListResponse> {
    let connection = Connection::open(db_path)?;
    let connection_id = require_identifier(connection_id, "Connection id")?;
    let mut statement = connection.prepare(
        "SELECT id, connection_id, resource_grant_id, resource_kind, resource_id, action,
                status, expires_at, grant_reason, created_by_actor_id, created_at, updated_at,
                revoked_at, revoked_by_actor_id, revocation_reason
         FROM connection_grants
         WHERE connection_id = ?1
         ORDER BY updated_at DESC, id DESC",
    )?;
    let grants = statement
        .query_map([connection_id], connection_grant_from_row)?
        .map(|row| row.map(ConnectionGrantRecord::into_view))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ConnectionGrantListResponse { grants })
}

pub fn list_connection_events(
    db_path: &Path,
    connection_id: &str,
) -> Result<ConnectionEventListResponse> {
    let connection = Connection::open(db_path)?;
    let connection_id = require_identifier(connection_id, "Connection id")?;
    let mut statement = connection.prepare(
        "SELECT event.id, event.connection_id, event.event_type, event.payload_json,
                COALESCE(receipt.payload_json, '{}'), event.occurred_at
         FROM connection_events event
         LEFT JOIN connection_receipts receipt ON receipt.event_id = event.id
         WHERE event.connection_id = ?1
         ORDER BY event.occurred_at DESC, event.id DESC",
    )?;
    let events = statement
        .query_map([connection_id], connection_event_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ConnectionEventListResponse { events })
}

pub fn create_connection(
    db_path: &Path,
    request: ConnectionWriteRequest,
    actor_id: Option<&str>,
) -> Result<(ConnectionView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let id = format!("connection_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();
    let display_name = require_text(&request.display_name, "Connection display name")?;
    let status = request.status.unwrap_or(ConnectionStatus::Pending);
    let (activated_at, suspended_at, revoked_at, archived_at) =
        status_timestamps(status, &now, None);
    transaction.execute(
        "INSERT INTO connections (
            id, connection_type, display_name, status, identity_json, scope_json, metadata_json,
            created_by_actor_id, created_at, updated_at, activated_at, suspended_at, revoked_at, archived_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?10, ?11, ?12, ?13)",
        params![
            id,
            request.connection_type.as_str(),
            display_name,
            status.as_str(),
            request.identity.unwrap_or_else(|| json!({})).to_string(),
            request.scope.unwrap_or_else(|| json!({})).to_string(),
            request.metadata.unwrap_or_else(|| json!({})).to_string(),
            actor_id,
            now,
            activated_at,
            suspended_at,
            revoked_at,
            archived_at,
        ],
    )?;
    append_connection_event_tx(
        &transaction,
        &id,
        "connection.created",
        json!({
            "connectionId": id,
            "connectionType": request.connection_type.as_str(),
            "status": status.as_str(),
        }),
        &now,
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "connection.created",
            json!({
                "connectionId": id,
                "connectionType": request.connection_type.as_str(),
                "status": status.as_str(),
            }),
        ),
    )?;
    transaction.commit()?;
    let record = find_connection_by_id(&connection, &id)?.expect("connection just inserted");
    Ok((record.into_view(), event))
}

pub fn update_connection(
    db_path: &Path,
    connection_id: &str,
    request: ConnectionWriteRequest,
    actor_id: Option<&str>,
) -> Result<(ConnectionView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let existing = find_connection_by_id(&transaction, connection_id)?
        .ok_or_else(|| anyhow::anyhow!("Connection was not found: {connection_id}"))?;
    let now = Utc::now().to_rfc3339();
    let display_name = require_text(&request.display_name, "Connection display name")?;
    let status = request.status.unwrap_or(existing.status);
    let (activated_at, suspended_at, revoked_at, archived_at) =
        status_timestamps(status, &now, Some(&existing));
    transaction.execute(
        "UPDATE connections
         SET connection_type = ?1,
             display_name = ?2,
             status = ?3,
             identity_json = ?4,
             scope_json = ?5,
             metadata_json = ?6,
             created_by_actor_id = COALESCE(created_by_actor_id, ?7),
             updated_at = ?8,
             activated_at = ?9,
             suspended_at = ?10,
             revoked_at = ?11,
             archived_at = ?12
         WHERE id = ?13",
        params![
            request.connection_type.as_str(),
            display_name,
            status.as_str(),
            request.identity.unwrap_or(existing.identity).to_string(),
            request.scope.unwrap_or(existing.scope).to_string(),
            request.metadata.unwrap_or(existing.metadata).to_string(),
            actor_id,
            now,
            activated_at,
            suspended_at,
            revoked_at,
            archived_at,
            connection_id,
        ],
    )?;
    if matches!(
        status,
        ConnectionStatus::Suspended | ConnectionStatus::Revoked | ConnectionStatus::Archived
    ) {
        revoke_active_connection_grants_tx(
            &transaction,
            connection_id,
            actor_id,
            status.as_str(),
            &now,
        )?;
    }
    append_connection_event_tx(
        &transaction,
        connection_id,
        "connection.updated",
        json!({
            "connectionId": connection_id,
            "connectionType": request.connection_type.as_str(),
            "status": status.as_str(),
        }),
        &now,
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "connection.updated",
            json!({
                "connectionId": connection_id,
                "connectionType": request.connection_type.as_str(),
                "status": status.as_str(),
            }),
        ),
    )?;
    transaction.commit()?;
    let record =
        find_connection_by_id(&connection, connection_id)?.expect("connection just updated");
    Ok((record.into_view(), event))
}

pub fn create_connection_grant(
    db_path: &Path,
    connection_id: &str,
    request: ConnectionGrantCreateRequest,
    actor_id: Option<&str>,
) -> Result<(ConnectionGrantView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let connection_record = find_connection_by_id(&transaction, connection_id)?
        .ok_or_else(|| anyhow::anyhow!("Connection was not found: {connection_id}"))?;
    if connection_record.status != ConnectionStatus::Active {
        bail!("Connection grants require an active connection.");
    }
    let now = Utc::now().to_rfc3339();
    let id = format!("connection_grant_{}", Uuid::new_v4());
    let resource_grant_id = format!("resource_grant_{}", Uuid::new_v4());
    let resource_kind = require_identifier(&request.resource_kind, "Resource kind")?;
    let resource_id = require_scoped_resource_id(&request.resource_id)?;
    let action = require_identifier(&request.action, "Grant action")?;
    transaction.execute(
        "INSERT INTO resource_grants (
            id, resource_kind, resource_id, action, subject_kind, subject_id, effect, created_at,
            expires_at, metadata_json
         ) VALUES (?1, ?2, ?3, ?4, 'connection', ?5, 'allow', ?6, ?7, ?8)",
        params![
            resource_grant_id,
            resource_kind,
            resource_id,
            action,
            connection_id,
            now,
            request.expires_at,
            json!({ "connectionGrantId": id, "grantReason": request.grant_reason }).to_string(),
        ],
    )?;
    transaction.execute(
        "INSERT INTO connection_grants (
            id, connection_id, resource_grant_id, resource_kind, resource_id, action, status,
            expires_at, grant_reason, created_by_actor_id, created_at, updated_at, revoked_at,
            revoked_by_actor_id, revocation_reason
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?8, ?9, ?10, ?10, NULL, NULL, NULL)",
        params![
            id,
            connection_id,
            resource_grant_id,
            resource_kind,
            resource_id,
            action,
            request.expires_at,
            normalize_optional_string(request.grant_reason),
            actor_id,
            now,
        ],
    )?;
    append_connection_event_tx(
        &transaction,
        connection_id,
        "connection.grant.created",
        json!({
            "connectionId": connection_id,
            "connectionGrantId": id,
            "resourceKind": resource_kind,
            "resourceId": resource_id,
            "action": action,
        }),
        &now,
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "connection.grant.created",
            json!({
                "connectionId": connection_id,
                "connectionGrantId": id,
                "resourceKind": resource_kind,
                "resourceId": resource_id,
                "action": action,
            }),
        ),
    )?;
    transaction.commit()?;
    let grant = find_connection_grant_by_id(&connection, &id)?.expect("grant just inserted");
    Ok((grant.into_view(), event))
}

pub fn revoke_connection_grant(
    db_path: &Path,
    grant_id: &str,
    request: ConnectionGrantRevokeRequest,
    actor_id: Option<&str>,
) -> Result<(ConnectionGrantView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let existing = find_connection_grant_by_id(&transaction, grant_id)?
        .ok_or_else(|| anyhow::anyhow!("Connection grant was not found: {grant_id}"))?;
    let now = Utc::now().to_rfc3339();
    let reason = normalize_optional_string(request.revocation_reason);
    transaction.execute(
        "UPDATE connection_grants
         SET status = 'revoked', updated_at = ?1, revoked_at = COALESCE(revoked_at, ?1),
             revoked_by_actor_id = COALESCE(revoked_by_actor_id, ?2), revocation_reason = ?3
         WHERE id = ?4",
        params![now, actor_id, reason, grant_id],
    )?;
    transaction.execute(
        "UPDATE resource_grants SET effect = 'revoked', expires_at = COALESCE(expires_at, ?1)
         WHERE id = ?2",
        params![now, existing.resource_grant_id],
    )?;
    append_connection_event_tx(
        &transaction,
        &existing.connection_id,
        "connection.grant.revoked",
        json!({
            "connectionId": existing.connection_id,
            "connectionGrantId": grant_id,
            "resourceKind": existing.resource_kind,
            "resourceId": existing.resource_id,
            "action": existing.action,
        }),
        &now,
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "connection.grant.revoked",
            json!({
                "connectionId": existing.connection_id,
                "connectionGrantId": grant_id,
                "resourceKind": existing.resource_kind,
                "resourceId": existing.resource_id,
                "action": existing.action,
            }),
        ),
    )?;
    transaction.commit()?;
    let grant = find_connection_grant_by_id(&connection, grant_id)?.expect("grant just revoked");
    Ok((grant.into_view(), event))
}

fn revoke_active_connection_grants_tx(
    transaction: &rusqlite::Transaction<'_>,
    connection_id: &str,
    actor_id: Option<&str>,
    reason: &str,
    now: &str,
) -> Result<()> {
    transaction.execute(
        "UPDATE resource_grants
         SET effect = 'revoked', expires_at = COALESCE(expires_at, ?1)
         WHERE id IN (
            SELECT resource_grant_id FROM connection_grants
            WHERE connection_id = ?2 AND status = 'active'
         )",
        params![now, connection_id],
    )?;
    transaction.execute(
        "UPDATE connection_grants
         SET status = 'revoked', updated_at = ?1, revoked_at = COALESCE(revoked_at, ?1),
             revoked_by_actor_id = COALESCE(revoked_by_actor_id, ?2), revocation_reason = ?3
         WHERE connection_id = ?4 AND status = 'active'",
        params![now, actor_id, reason, connection_id],
    )?;
    Ok(())
}

fn append_connection_event_tx(
    transaction: &rusqlite::Transaction<'_>,
    connection_id: &str,
    event_type: &str,
    payload: Value,
    occurred_at: &str,
) -> Result<String> {
    let event_id = format!("connection_event_{}", Uuid::new_v4());
    transaction.execute(
        "INSERT INTO connection_events (id, connection_id, event_type, payload_json, occurred_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            event_id,
            connection_id,
            event_type,
            payload.to_string(),
            occurred_at
        ],
    )?;
    transaction.execute(
        "INSERT INTO connection_receipts (id, connection_id, event_id, receipt_kind, payload_json, created_at)
         VALUES (?1, ?2, ?3, 'local_recorded', ?4, ?5)",
        params![
            format!("connection_receipt_{}", Uuid::new_v4()),
            connection_id,
            event_id,
            json!({ "eventType": event_type, "recorded": true }).to_string(),
            occurred_at,
        ],
    )?;
    Ok(event_id)
}

fn find_connection_by_id(
    connection: &Connection,
    connection_id: &str,
) -> rusqlite::Result<Option<ConnectionRecord>> {
    connection
        .query_row(
            "SELECT id, connection_type, display_name, status, identity_json, scope_json,
                    metadata_json, created_by_actor_id, created_at, updated_at, activated_at,
                    suspended_at, revoked_at, archived_at
             FROM connections WHERE id = ?1",
            [connection_id],
            connection_from_row,
        )
        .optional()
}

fn find_connection_grant_by_id(
    connection: &Connection,
    grant_id: &str,
) -> rusqlite::Result<Option<ConnectionGrantRecord>> {
    connection
        .query_row(
            "SELECT id, connection_id, resource_grant_id, resource_kind, resource_id, action,
                    status, expires_at, grant_reason, created_by_actor_id, created_at, updated_at,
                    revoked_at, revoked_by_actor_id, revocation_reason
             FROM connection_grants WHERE id = ?1",
            [grant_id],
            connection_grant_from_row,
        )
        .optional()
}

fn connection_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConnectionRecord> {
    let connection_type: String = row.get(1)?;
    let status: String = row.get(3)?;
    let identity_json: String = row.get(4)?;
    let scope_json: String = row.get(5)?;
    let metadata_json: String = row.get(6)?;
    Ok(ConnectionRecord {
        id: row.get(0)?,
        connection_type: ConnectionType::try_from(connection_type.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, error.into())
        })?,
        display_name: row.get(2)?,
        status: ConnectionStatus::try_from(status.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, error.into())
        })?,
        identity: serde_json::from_str(&identity_json).unwrap_or_else(|_| json!({})),
        scope: serde_json::from_str(&scope_json).unwrap_or_else(|_| json!({})),
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        created_by_actor_id: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        activated_at: row.get(10)?,
        suspended_at: row.get(11)?,
        revoked_at: row.get(12)?,
        archived_at: row.get(13)?,
    })
}

fn connection_grant_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConnectionGrantRecord> {
    let status: String = row.get(6)?;
    Ok(ConnectionGrantRecord {
        id: row.get(0)?,
        connection_id: row.get(1)?,
        resource_grant_id: row.get(2)?,
        resource_kind: row.get(3)?,
        resource_id: row.get(4)?,
        action: row.get(5)?,
        status: ConnectionGrantStatus::try_from(status.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, error.into())
        })?,
        expires_at: row.get(7)?,
        grant_reason: row.get(8)?,
        created_by_actor_id: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
        revoked_at: row.get(12)?,
        revoked_by_actor_id: row.get(13)?,
        revocation_reason: row.get(14)?,
    })
}

fn connection_event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConnectionEventView> {
    let payload_json: String = row.get(3)?;
    let receipt_json: String = row.get(4)?;
    Ok(ConnectionEventView {
        id: row.get(0)?,
        connection_id: row.get(1)?,
        event_type: row.get(2)?,
        payload: serde_json::from_str(&payload_json).unwrap_or_else(|_| json!({})),
        receipt: serde_json::from_str(&receipt_json).unwrap_or_else(|_| json!({})),
        occurred_at: row.get(5)?,
    })
}

impl ConnectionRecord {
    fn into_view(self) -> ConnectionView {
        ConnectionView {
            id: self.id,
            connection_type: self.connection_type,
            display_name: self.display_name,
            status: self.status,
            identity: self.identity,
            scope: self.scope,
            metadata: self.metadata,
            created_by_actor_id: self.created_by_actor_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            activated_at: self.activated_at,
            suspended_at: self.suspended_at,
            revoked_at: self.revoked_at,
            archived_at: self.archived_at,
        }
    }
}

impl ConnectionGrantRecord {
    fn into_view(self) -> ConnectionGrantView {
        ConnectionGrantView {
            id: self.id,
            connection_id: self.connection_id,
            resource_grant_id: self.resource_grant_id,
            resource_kind: self.resource_kind,
            resource_id: self.resource_id,
            action: self.action,
            status: self.status,
            expires_at: self.expires_at,
            grant_reason: self.grant_reason,
            created_by_actor_id: self.created_by_actor_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            revoked_at: self.revoked_at,
            revoked_by_actor_id: self.revoked_by_actor_id,
            revocation_reason: self.revocation_reason,
        }
    }
}

fn status_timestamps(
    status: ConnectionStatus,
    now: &str,
    existing: Option<&ConnectionRecord>,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let existing_active = existing.and_then(|record| record.activated_at.clone());
    let existing_suspended = existing.and_then(|record| record.suspended_at.clone());
    let existing_revoked = existing.and_then(|record| record.revoked_at.clone());
    let existing_archived = existing.and_then(|record| record.archived_at.clone());
    (
        if status == ConnectionStatus::Active {
            existing_active.or_else(|| Some(now.to_string()))
        } else {
            existing_active
        },
        if status == ConnectionStatus::Suspended {
            existing_suspended.or_else(|| Some(now.to_string()))
        } else {
            existing_suspended
        },
        if status == ConnectionStatus::Revoked {
            existing_revoked.or_else(|| Some(now.to_string()))
        } else {
            existing_revoked
        },
        if status == ConnectionStatus::Archived {
            existing_archived.or_else(|| Some(now.to_string()))
        } else {
            existing_archived
        },
    )
}

fn require_scoped_resource_id(value: &str) -> Result<String> {
    if value.trim() == "*" {
        bail!("Connection grants must name an explicit resource id.");
    }
    let normalized = require_identifier(value, "Resource id")?;
    Ok(normalized)
}

fn require_identifier(value: &str, label: &str) -> Result<String> {
    let normalized = normalize_optional_string(Some(value.to_string()))
        .ok_or_else(|| anyhow::anyhow!("{label} is required."))?;
    if normalized.len() > 160
        || !normalized.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':' | '/')
        })
    {
        bail!("{label} must be a stable identifier.");
    }
    Ok(normalized)
}

fn require_text(value: &str, label: &str) -> Result<String> {
    normalize_optional_string(Some(value.to_string()))
        .ok_or_else(|| anyhow::anyhow!("{label} is required."))
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().replace(char::is_whitespace, " "))
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{
        authorize_connection_resource_access, PolicyAction, PolicyOutcome, ResourceKind,
        ResourceRef, LOCAL_OWNER_ACTOR_ID,
    };
    use crate::schema::init_database;
    use tempfile::TempDir;

    #[test]
    fn connection_grant_is_scoped_and_policy_consultable() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (connection, _) = create_connection(
            &db_path,
            active_connection_request("Support"),
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        let (grant, event) = create_connection_grant(
            &db_path,
            &connection.id,
            ConnectionGrantCreateRequest {
                resource_kind: "issue_report".to_string(),
                resource_id: "report_1".to_string(),
                action: "read".to_string(),
                expires_at: None,
                grant_reason: Some("support review".to_string()),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(grant.status, ConnectionGrantStatus::Active);
        assert_eq!(event.event_type, "connection.grant.created");
        let sqlite = Connection::open(&db_path).unwrap();
        let allowed = authorize_connection_resource_access(
            &sqlite,
            &connection.id,
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::IssueReport, "report_1"),
            None,
        );
        let denied = authorize_connection_resource_access(
            &sqlite,
            &connection.id,
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::IssueReport, "report_2"),
            None,
        );
        assert_eq!(allowed.outcome, PolicyOutcome::Allowed);
        assert_eq!(denied.outcome, PolicyOutcome::Denied);
    }

    #[test]
    fn revoked_grant_no_longer_authorizes_connection() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (connection, _) = create_connection(
            &db_path,
            active_connection_request("Affiliate"),
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        let (grant, _) = create_connection_grant(
            &db_path,
            &connection.id,
            ConnectionGrantCreateRequest {
                resource_kind: "brief_artifact".to_string(),
                resource_id: "brief_1".to_string(),
                action: "inspect".to_string(),
                expires_at: None,
                grant_reason: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        revoke_connection_grant(
            &db_path,
            &grant.id,
            ConnectionGrantRevokeRequest {
                revocation_reason: Some("done".to_string()),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let sqlite = Connection::open(&db_path).unwrap();
        let denied = authorize_connection_resource_access(
            &sqlite,
            &connection.id,
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::BriefArtifact, "brief_1"),
            None,
        );
        assert_eq!(denied.outcome, PolicyOutcome::Denied);
    }

    #[test]
    fn suspended_connection_revokes_active_grants_and_records_event_receipts() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (connection, _) = create_connection(
            &db_path,
            active_connection_request("Worker"),
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        create_connection_grant(
            &db_path,
            &connection.id,
            ConnectionGrantCreateRequest {
                resource_kind: "corpus_item".to_string(),
                resource_id: "item_1".to_string(),
                action: "read".to_string(),
                expires_at: None,
                grant_reason: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let (updated, event) = update_connection(
            &db_path,
            &connection.id,
            ConnectionWriteRequest {
                connection_type: ConnectionType::WorkerOrdo,
                display_name: "Worker".to_string(),
                status: Some(ConnectionStatus::Suspended),
                identity: None,
                scope: None,
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(updated.status, ConnectionStatus::Suspended);
        assert_eq!(event.event_type, "connection.updated");
        let grants = list_connection_grants(&db_path, &connection.id).unwrap();
        assert_eq!(grants.grants[0].status, ConnectionGrantStatus::Revoked);
        let events = list_connection_events(&db_path, &connection.id).unwrap();
        assert!(events
            .events
            .iter()
            .any(|event| event.receipt["recorded"] == true));
    }

    #[test]
    fn wildcard_resource_ids_are_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (connection, _) = create_connection(
            &db_path,
            active_connection_request("Client"),
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let result = create_connection_grant(
            &db_path,
            &connection.id,
            ConnectionGrantCreateRequest {
                resource_kind: "owner_system".to_string(),
                resource_id: "*".to_string(),
                action: "read".to_string(),
                expires_at: None,
                grant_reason: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        );

        assert!(result
            .unwrap_err()
            .to_string()
            .contains("explicit resource id"));
    }

    #[test]
    fn expired_connection_grant_does_not_authorize_access() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (connection, _) = create_connection(
            &db_path,
            active_connection_request("Expired"),
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        create_connection_grant(
            &db_path,
            &connection.id,
            ConnectionGrantCreateRequest {
                resource_kind: "issue_report".to_string(),
                resource_id: "report_1".to_string(),
                action: "read".to_string(),
                expires_at: Some((Utc::now() - chrono::Duration::minutes(1)).to_rfc3339()),
                grant_reason: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let sqlite = Connection::open(&db_path).unwrap();
        let denied = authorize_connection_resource_access(
            &sqlite,
            &connection.id,
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::IssueReport, "report_1"),
            None,
        );
        assert_eq!(denied.outcome, PolicyOutcome::Denied);
    }

    fn active_connection_request(display_name: &str) -> ConnectionWriteRequest {
        ConnectionWriteRequest {
            connection_type: ConnectionType::Support,
            display_name: display_name.to_string(),
            status: Some(ConnectionStatus::Active),
            identity: Some(json!({ "label": display_name })),
            scope: Some(json!({ "purpose": "test" })),
            metadata: None,
        }
    }
}
