use crate::schema::db::ConnectionExt;
use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

use crate::business::BusinessFactViewer;
use crate::events::{append_realtime_event_tx, system_event, RealtimeEvent};
use crate::policy::{
    authorize_resource_access, ActorContext, ActorKind, PolicyAction, ResourceKind, ResourceRef,
    LOCAL_OWNER_ACTOR_ID,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusStatus {
    Draft,
    Approved,
    Archived,
    Revoked,
}

impl CorpusStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Approved => "approved",
            Self::Archived => "archived",
            Self::Revoked => "revoked",
        }
    }
}

impl TryFrom<&str> for CorpusStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "draft" => Ok(Self::Draft),
            "approved" => Ok(Self::Approved),
            "archived" => Ok(Self::Archived),
            "revoked" => Ok(Self::Revoked),
            _ => bail!("Unsupported corpus status: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusVisibility {
    Public,
    Authenticated,
    Staff,
    Owner,
}

impl CorpusVisibility {
    fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Authenticated => "authenticated",
            Self::Staff => "staff",
            Self::Owner => "owner",
        }
    }
}

impl TryFrom<&str> for CorpusVisibility {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "public" => Ok(Self::Public),
            "authenticated" | "signed_in" => Ok(Self::Authenticated),
            "staff" | "staff_admin" => Ok(Self::Staff),
            "owner" | "owner_system" => Ok(Self::Owner),
            _ => bail!("Unsupported corpus visibility: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusViewer {
    Public,
    Authenticated,
    Staff,
    Owner,
}

impl From<CorpusViewer> for BusinessFactViewer {
    fn from(value: CorpusViewer) -> Self {
        match value {
            CorpusViewer::Public => BusinessFactViewer::Public,
            CorpusViewer::Authenticated => BusinessFactViewer::Authenticated,
            CorpusViewer::Staff => BusinessFactViewer::Staff,
            CorpusViewer::Owner => BusinessFactViewer::Owner,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusSourceWriteRequest {
    pub source_kind: Option<String>,
    pub label: String,
    pub uri: Option<String>,
    pub resource_kind: Option<String>,
    pub resource_id: Option<String>,
    pub status: Option<CorpusStatus>,
    pub visibility: Option<CorpusVisibility>,
    pub provenance: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusItemWriteRequest {
    pub source_id: String,
    pub item_kind: Option<String>,
    pub ordinal: Option<i64>,
    pub title: String,
    pub body_text: String,
    pub resource_kind: Option<String>,
    pub resource_id: Option<String>,
    pub status: Option<CorpusStatus>,
    pub visibility: Option<CorpusVisibility>,
    pub provenance: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusRetrievalQuery {
    pub query: String,
    pub viewer: Option<CorpusViewer>,
    pub actor_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusSourceListResponse {
    pub sources: Vec<CorpusSourceView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusItemListResponse {
    pub items: Vec<CorpusItemView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusSourceView {
    pub id: String,
    pub source_kind: String,
    pub label: String,
    pub uri: Option<String>,
    pub resource_kind: String,
    pub resource_id: String,
    pub status: CorpusStatus,
    pub classification: Value,
    pub provenance: Value,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusItemView {
    pub id: String,
    pub source_id: String,
    pub item_kind: String,
    pub ordinal: i64,
    pub title: String,
    pub body_text: String,
    pub content_hash: String,
    pub resource_kind: String,
    pub resource_id: String,
    pub status: CorpusStatus,
    pub classification: Value,
    pub provenance: Value,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusRetrievalResponse {
    pub query: String,
    pub viewer: CorpusViewer,
    pub evidence_state: String,
    pub results: Vec<CorpusRetrievalResult>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusRetrievalResult {
    pub item: CorpusItemView,
    pub source: CorpusSourceView,
    pub rank: f64,
    pub snippet: String,
    pub evidence: Value,
}

struct CorpusSourceRecord {
    id: String,
    source_kind: String,
    label: String,
    uri: Option<String>,
    resource_kind: String,
    resource_id: String,
    status: CorpusStatus,
    classification: Value,
    provenance: Value,
    metadata: Value,
    created_at: String,
    updated_at: String,
}

struct CorpusItemRecord {
    id: String,
    source_id: String,
    item_kind: String,
    ordinal: i64,
    title: String,
    body_text: String,
    content_hash: String,
    resource_kind: String,
    resource_id: String,
    status: CorpusStatus,
    classification: Value,
    provenance: Value,
    metadata: Value,
    created_at: String,
    updated_at: String,
}

pub fn list_corpus_sources(
    db_path: &Path,
    viewer: Option<CorpusViewer>,
) -> Result<CorpusSourceListResponse> {
    let connection = Connection::open(db_path)?;
    let viewer = viewer.unwrap_or(CorpusViewer::Owner);
    let actor = actor_for_viewer(viewer, None);
    let records = load_corpus_sources(&connection)?;
    Ok(CorpusSourceListResponse {
        sources: records
            .into_iter()
            .filter(|record| {
                can_view_corpus_record(
                    &connection,
                    viewer,
                    &actor,
                    record.status,
                    &record.classification,
                    &record.resource_kind,
                    &record.resource_id,
                )
            })
            .map(CorpusSourceRecord::into_view)
            .collect(),
    })
}

pub fn read_corpus_source(
    db_path: &Path,
    source_id: &str,
    viewer: Option<CorpusViewer>,
) -> Result<CorpusSourceView> {
    let connection = Connection::open(db_path)?;
    let viewer = viewer.unwrap_or(CorpusViewer::Owner);
    let actor = actor_for_viewer(viewer, None);
    let record = require_corpus_source(&connection, source_id)?;
    if !can_view_corpus_record(
        &connection,
        viewer,
        &actor,
        record.status,
        &record.classification,
        &record.resource_kind,
        &record.resource_id,
    ) {
        bail!("Corpus source is not visible to this viewer");
    }
    Ok(record.into_view())
}

pub fn create_corpus_source(
    db_path: &Path,
    request: CorpusSourceWriteRequest,
    _actor_id: Option<&str>,
) -> Result<(CorpusSourceView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let id = format!("corpus_source_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();
    let visibility = request.visibility.unwrap_or(CorpusVisibility::Owner);
    let status = request.status.unwrap_or(CorpusStatus::Draft);
    transaction.execute(
        "INSERT INTO corpus_sources (
            id, source_kind, label, uri, resource_kind, resource_id, status,
            classification_json, provenance_json, metadata_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            id,
            normalize_optional_string(request.source_kind)
                .unwrap_or_else(|| "operator_text".to_string()),
            require_non_empty(&request.label, "Source label")?,
            normalize_optional_string(request.uri),
            normalize_optional_string(request.resource_kind)
                .unwrap_or_else(|| resource_kind_for_visibility(visibility).to_string()),
            normalize_optional_string(request.resource_id)
                .unwrap_or_else(|| resource_id_for_visibility(visibility, &id)),
            status.as_str(),
            classification_json(visibility, status).to_string(),
            request.provenance.unwrap_or_else(|| json!({})).to_string(),
            request.metadata.unwrap_or_else(|| json!({})).to_string(),
            now,
        ],
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "corpus.source.created",
            json!({ "sourceId": id, "status": status.as_str(), "visibility": visibility.as_str() }),
        ),
    )?;
    transaction.commit()?;
    Ok((require_corpus_source(&connection, &id)?.into_view(), event))
}

pub fn update_corpus_source(
    db_path: &Path,
    source_id: &str,
    request: CorpusSourceWriteRequest,
    _actor_id: Option<&str>,
) -> Result<(CorpusSourceView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let existing = require_corpus_source(&transaction, source_id)?;
    let status = request.status.unwrap_or(existing.status);
    let visibility = request
        .visibility
        .unwrap_or_else(|| visibility_from_classification(&existing.classification));
    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "UPDATE corpus_sources
         SET source_kind = ?1, label = ?2, uri = ?3, resource_kind = ?4, resource_id = ?5,
             status = ?6, classification_json = ?7, provenance_json = ?8, metadata_json = ?9,
             updated_at = ?10
         WHERE id = ?11",
        params![
            normalize_optional_string(request.source_kind).unwrap_or(existing.source_kind),
            require_non_empty(&request.label, "Source label")?,
            normalize_optional_string(request.uri).or(existing.uri),
            normalize_optional_string(request.resource_kind).unwrap_or(existing.resource_kind),
            normalize_optional_string(request.resource_id).unwrap_or(existing.resource_id),
            status.as_str(),
            classification_json(visibility, status).to_string(),
            request
                .provenance
                .unwrap_or(existing.provenance)
                .to_string(),
            request.metadata.unwrap_or(existing.metadata).to_string(),
            now,
            source_id,
        ],
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "corpus.source.updated",
            json!({ "sourceId": source_id, "status": status.as_str(), "visibility": visibility.as_str() }),
        ),
    )?;
    transaction.commit()?;
    Ok((
        require_corpus_source(&connection, source_id)?.into_view(),
        event,
    ))
}

pub fn list_corpus_items(
    db_path: &Path,
    source_id: Option<&str>,
    viewer: Option<CorpusViewer>,
) -> Result<CorpusItemListResponse> {
    let connection = Connection::open(db_path)?;
    let viewer = viewer.unwrap_or(CorpusViewer::Owner);
    let actor = actor_for_viewer(viewer, None);
    let records = load_corpus_items(&connection, source_id)?;
    Ok(CorpusItemListResponse {
        items: records
            .into_iter()
            .filter(|record| {
                can_view_corpus_record(
                    &connection,
                    viewer,
                    &actor,
                    record.status,
                    &record.classification,
                    &record.resource_kind,
                    &record.resource_id,
                )
            })
            .map(CorpusItemRecord::into_view)
            .collect(),
    })
}

pub fn read_corpus_item(
    db_path: &Path,
    item_id: &str,
    viewer: Option<CorpusViewer>,
) -> Result<CorpusItemView> {
    let connection = Connection::open(db_path)?;
    let viewer = viewer.unwrap_or(CorpusViewer::Owner);
    let actor = actor_for_viewer(viewer, None);
    let record = require_corpus_item(&connection, item_id)?;
    if !can_view_corpus_record(
        &connection,
        viewer,
        &actor,
        record.status,
        &record.classification,
        &record.resource_kind,
        &record.resource_id,
    ) {
        bail!("Corpus item is not visible to this viewer");
    }
    Ok(record.into_view())
}

pub fn create_corpus_item(
    db_path: &Path,
    request: CorpusItemWriteRequest,
    _actor_id: Option<&str>,
) -> Result<(CorpusItemView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    require_corpus_source(&transaction, &request.source_id)?;
    let id = format!("corpus_item_{}", Uuid::new_v4());
    let title = require_non_empty(&request.title, "Item title")?;
    let body_text = require_non_empty(&request.body_text, "Item body")?;
    let visibility = request.visibility.unwrap_or(CorpusVisibility::Owner);
    let status = request.status.unwrap_or(CorpusStatus::Draft);
    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "INSERT INTO corpus_items (
            id, source_id, item_kind, ordinal, title, body_text, content_hash, resource_kind,
            resource_id, status, classification_json, provenance_json, metadata_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
        params![
            id,
            request.source_id,
            normalize_optional_string(request.item_kind).unwrap_or_else(|| "chunk".to_string()),
            request.ordinal.unwrap_or(0),
            title,
            body_text,
            content_hash(&body_text),
            normalize_optional_string(request.resource_kind).unwrap_or_else(|| resource_kind_for_visibility(visibility).to_string()),
            normalize_optional_string(request.resource_id)
                .unwrap_or_else(|| resource_id_for_visibility(visibility, &id)),
            status.as_str(),
            classification_json(visibility, status).to_string(),
            request.provenance.unwrap_or_else(|| json!({})).to_string(),
            request.metadata.unwrap_or_else(|| json!({})).to_string(),
            now,
        ],
    )?;
    upsert_fts_item(&transaction, &id, &title, &body_text)?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "corpus.item.created",
            json!({ "itemId": id, "sourceId": request.source_id, "status": status.as_str(), "visibility": visibility.as_str() }),
        ),
    )?;
    transaction.commit()?;
    Ok((require_corpus_item(&connection, &id)?.into_view(), event))
}

pub fn update_corpus_item(
    db_path: &Path,
    item_id: &str,
    request: CorpusItemWriteRequest,
    _actor_id: Option<&str>,
) -> Result<(CorpusItemView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    require_corpus_source(&transaction, &request.source_id)?;
    let existing = require_corpus_item(&transaction, item_id)?;
    let title = require_non_empty(&request.title, "Item title")?;
    let body_text = require_non_empty(&request.body_text, "Item body")?;
    let visibility = request
        .visibility
        .unwrap_or_else(|| visibility_from_classification(&existing.classification));
    let status = request.status.unwrap_or(existing.status);
    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "UPDATE corpus_items
         SET source_id = ?1, item_kind = ?2, ordinal = ?3, title = ?4, body_text = ?5,
             content_hash = ?6, resource_kind = ?7, resource_id = ?8, status = ?9,
             classification_json = ?10, provenance_json = ?11, metadata_json = ?12,
             updated_at = ?13
         WHERE id = ?14",
        params![
            request.source_id,
            normalize_optional_string(request.item_kind).unwrap_or(existing.item_kind),
            request.ordinal.unwrap_or(existing.ordinal),
            title,
            body_text,
            content_hash(&body_text),
            normalize_optional_string(request.resource_kind).unwrap_or(existing.resource_kind),
            normalize_optional_string(request.resource_id).unwrap_or(existing.resource_id),
            status.as_str(),
            classification_json(visibility, status).to_string(),
            request
                .provenance
                .unwrap_or(existing.provenance)
                .to_string(),
            request.metadata.unwrap_or(existing.metadata).to_string(),
            now,
            item_id,
        ],
    )?;
    upsert_fts_item(&transaction, item_id, &title, &body_text)?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "corpus.item.updated",
            json!({ "itemId": item_id, "sourceId": request.source_id, "status": status.as_str(), "visibility": visibility.as_str() }),
        ),
    )?;
    transaction.commit()?;
    Ok((
        require_corpus_item(&connection, item_id)?.into_view(),
        event,
    ))
}

pub fn retrieve_corpus(
    db_path: &Path,
    request: CorpusRetrievalQuery,
) -> Result<CorpusRetrievalResponse> {
    let connection = Connection::open(db_path)?;
    let query = require_non_empty(&request.query, "Retrieval query")?;
    let viewer = request.viewer.unwrap_or(CorpusViewer::Public);
    let actor = actor_for_viewer(viewer, request.actor_id.as_deref());
    let limit = request.limit.unwrap_or(10).clamp(1, 25);
    let candidates = retrieve_candidates(&connection, &query, limit * 4)?;
    let mut results = Vec::new();
    for (item_id, rank, snippet) in candidates {
        let item = require_corpus_item(&connection, &item_id)?;
        let source = require_corpus_source(&connection, &item.source_id)?;
        if source.status != CorpusStatus::Approved || item.status != CorpusStatus::Approved {
            continue;
        }
        if !can_view_corpus_record(
            &connection,
            viewer,
            &actor,
            item.status,
            &item.classification,
            &item.resource_kind,
            &item.resource_id,
        ) {
            continue;
        }
        results.push(CorpusRetrievalResult {
            evidence: retrieval_evidence(&item, &source),
            item: item.into_view(),
            source: source.into_view(),
            rank,
            snippet,
        });
        if results.len() >= limit {
            break;
        }
    }
    let evidence_state = if results.is_empty() {
        "missing_evidence"
    } else {
        "evidence_found"
    }
    .to_string();
    Ok(CorpusRetrievalResponse {
        query,
        viewer,
        evidence_state,
        results,
        limitations: vec![
            "Retrieval uses local SQLite FTS only; no embeddings or provider calls are used.".to_string(),
            "Candidates are filtered by approval status, visibility, and local resource access before being returned.".to_string(),
        ],
    })
}

fn retrieve_candidates(
    connection: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<(String, f64, String)>> {
    let fts_query = sanitize_fts_query(query)?;
    connection.query_many(
        "SELECT item_id, bm25(corpus_items_fts) AS rank,
                snippet(corpus_items_fts, 2, '[', ']', ' ... ', 16) AS snippet
         FROM corpus_items_fts
         WHERE corpus_items_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2",
        params![fts_query, limit as i64],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
}

fn load_corpus_sources(connection: &Connection) -> Result<Vec<CorpusSourceRecord>> {
    connection.query_many(
        "SELECT id, source_kind, label, uri, resource_kind, resource_id, status,
                classification_json, provenance_json, metadata_json, created_at, updated_at
         FROM corpus_sources ORDER BY updated_at DESC, id DESC",
        [],
        corpus_source_from_row,
    )
}

fn require_corpus_source(connection: &Connection, source_id: &str) -> Result<CorpusSourceRecord> {
    connection
        .query_row(
            "SELECT id, source_kind, label, uri, resource_kind, resource_id, status,
                    classification_json, provenance_json, metadata_json, created_at, updated_at
             FROM corpus_sources WHERE id = ?1",
            [source_id],
            corpus_source_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("Corpus source was not found: {source_id}"))
}

fn load_corpus_items(
    connection: &Connection,
    source_id: Option<&str>,
) -> Result<Vec<CorpusItemRecord>> {
    let (sql, params): (&str, Vec<&str>) = if let Some(source_id) = source_id {
        (
            "SELECT id, source_id, item_kind, ordinal, title, body_text, content_hash,
                    resource_kind, resource_id, status, classification_json, provenance_json,
                    metadata_json, created_at, updated_at
             FROM corpus_items WHERE source_id = ?1 ORDER BY source_id, ordinal, updated_at DESC",
            vec![source_id],
        )
    } else {
        (
            "SELECT id, source_id, item_kind, ordinal, title, body_text, content_hash,
                    resource_kind, resource_id, status, classification_json, provenance_json,
                    metadata_json, created_at, updated_at
             FROM corpus_items ORDER BY source_id, ordinal, updated_at DESC",
            Vec::new(),
        )
    };
    connection.query_many(
        sql,
        rusqlite::params_from_iter(params),
        corpus_item_from_row,
    )
}

fn require_corpus_item(connection: &Connection, item_id: &str) -> Result<CorpusItemRecord> {
    connection
        .query_row(
            "SELECT id, source_id, item_kind, ordinal, title, body_text, content_hash,
                    resource_kind, resource_id, status, classification_json, provenance_json,
                    metadata_json, created_at, updated_at
             FROM corpus_items WHERE id = ?1",
            [item_id],
            corpus_item_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("Corpus item was not found: {item_id}"))
}

fn corpus_source_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CorpusSourceRecord> {
    let status: String = row.get(6)?;
    let classification_json: String = row.get(7)?;
    let provenance_json: String = row.get(8)?;
    let metadata_json: String = row.get(9)?;
    Ok(CorpusSourceRecord {
        id: row.get(0)?,
        source_kind: row.get(1)?,
        label: row.get(2)?,
        uri: row.get(3)?,
        resource_kind: row.get(4)?,
        resource_id: row.get(5)?,
        status: CorpusStatus::try_from(status.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, error.into())
        })?,
        classification: serde_json::from_str(&classification_json).unwrap_or_else(|_| json!({})),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn corpus_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CorpusItemRecord> {
    let status: String = row.get(9)?;
    let classification_json: String = row.get(10)?;
    let provenance_json: String = row.get(11)?;
    let metadata_json: String = row.get(12)?;
    Ok(CorpusItemRecord {
        id: row.get(0)?,
        source_id: row.get(1)?,
        item_kind: row.get(2)?,
        ordinal: row.get(3)?,
        title: row.get(4)?,
        body_text: row.get(5)?,
        content_hash: row.get(6)?,
        resource_kind: row.get(7)?,
        resource_id: row.get(8)?,
        status: CorpusStatus::try_from(status.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, error.into())
        })?,
        classification: serde_json::from_str(&classification_json).unwrap_or_else(|_| json!({})),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

impl CorpusSourceRecord {
    fn into_view(self) -> CorpusSourceView {
        CorpusSourceView {
            id: self.id,
            source_kind: self.source_kind,
            label: self.label,
            uri: self.uri,
            resource_kind: self.resource_kind,
            resource_id: self.resource_id,
            status: self.status,
            classification: self.classification,
            provenance: self.provenance,
            metadata: self.metadata,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl CorpusItemRecord {
    fn into_view(self) -> CorpusItemView {
        CorpusItemView {
            id: self.id,
            source_id: self.source_id,
            item_kind: self.item_kind,
            ordinal: self.ordinal,
            title: self.title,
            body_text: self.body_text,
            content_hash: self.content_hash,
            resource_kind: self.resource_kind,
            resource_id: self.resource_id,
            status: self.status,
            classification: self.classification,
            provenance: self.provenance,
            metadata: self.metadata,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn upsert_fts_item(
    connection: &Connection,
    item_id: &str,
    title: &str,
    body_text: &str,
) -> Result<()> {
    connection.execute("DELETE FROM corpus_items_fts WHERE item_id = ?1", [item_id])?;
    connection.execute(
        "INSERT INTO corpus_items_fts (item_id, title, body_text) VALUES (?1, ?2, ?3)",
        params![item_id, title, body_text],
    )?;
    Ok(())
}

fn can_view_corpus_record(
    connection: &Connection,
    viewer: CorpusViewer,
    actor: &ActorContext,
    status: CorpusStatus,
    classification: &Value,
    resource_kind: &str,
    resource_id: &str,
) -> bool {
    if viewer == CorpusViewer::Owner {
        return true;
    }
    if status != CorpusStatus::Approved {
        return false;
    }
    let visibility = visibility_from_classification(classification);
    let visibility_allowed = match viewer {
        CorpusViewer::Public => visibility == CorpusVisibility::Public,
        CorpusViewer::Authenticated => matches!(
            visibility,
            CorpusVisibility::Public | CorpusVisibility::Authenticated
        ),
        CorpusViewer::Staff => matches!(
            visibility,
            CorpusVisibility::Public | CorpusVisibility::Authenticated | CorpusVisibility::Staff
        ),
        CorpusViewer::Owner => true,
    };
    visibility_allowed && durable_resource_allows(connection, actor, resource_kind, resource_id)
}

fn durable_resource_allows(
    connection: &Connection,
    actor: &ActorContext,
    resource_kind: &str,
    resource_id: &str,
) -> bool {
    let Some(kind) = policy_resource_kind(resource_kind) else {
        return resource_kind == "public";
    };
    authorize_resource_access(
        connection,
        actor.clone(),
        PolicyAction::Read,
        ResourceRef::new(kind, resource_id),
        Some("corpus.retrieve"),
    )
    .allowed()
}

fn policy_resource_kind(value: &str) -> Option<ResourceKind> {
    match value {
        "system" => Some(ResourceKind::System),
        "owner_system" => Some(ResourceKind::OwnerSystem),
        "private_actor" => Some(ResourceKind::PrivateActor),
        "corpus_source" => Some(ResourceKind::CorpusSource),
        "corpus_item" => Some(ResourceKind::CorpusItem),
        _ => None,
    }
}

fn actor_for_viewer(viewer: CorpusViewer, actor_id: Option<&str>) -> ActorContext {
    match viewer {
        CorpusViewer::Owner => ActorContext::new(
            ActorKind::LocalOwner,
            "http",
            Some(actor_id.unwrap_or(LOCAL_OWNER_ACTOR_ID).to_string()),
        ),
        CorpusViewer::Public => ActorContext::new(ActorKind::BrowserOperator, "public", None),
        CorpusViewer::Authenticated | CorpusViewer::Staff => ActorContext::new(
            ActorKind::BrowserOperator,
            "http",
            actor_id.map(ToString::to_string),
        ),
    }
}

fn visibility_from_classification(classification: &Value) -> CorpusVisibility {
    classification
        .get("visibility")
        .and_then(Value::as_str)
        .and_then(|value| CorpusVisibility::try_from(value).ok())
        .unwrap_or(CorpusVisibility::Owner)
}

fn classification_json(visibility: CorpusVisibility, status: CorpusStatus) -> Value {
    json!({
        "visibility": visibility.as_str(),
        "publicationState": if status == CorpusStatus::Approved { "published" } else { status.as_str() },
        "retrieval": "sqlite_fts",
        "generatedAnswer": false,
    })
}

fn resource_kind_for_visibility(visibility: CorpusVisibility) -> &'static str {
    match visibility {
        CorpusVisibility::Public => "system",
        CorpusVisibility::Authenticated | CorpusVisibility::Staff | CorpusVisibility::Owner => {
            "owner_system"
        }
    }
}

fn resource_id_for_visibility(visibility: CorpusVisibility, fallback_id: &str) -> String {
    match visibility {
        CorpusVisibility::Public => "public".to_string(),
        CorpusVisibility::Authenticated | CorpusVisibility::Staff | CorpusVisibility::Owner => {
            fallback_id.to_string()
        }
    }
}

fn retrieval_evidence(item: &CorpusItemRecord, source: &CorpusSourceRecord) -> Value {
    json!({
        "schemaVersion": 1,
        "sourceId": source.id,
        "sourceKind": source.source_kind,
        "itemId": item.id,
        "contentHash": item.content_hash,
        "sourceProvenance": source.provenance,
        "itemProvenance": item.provenance,
        "classification": item.classification,
        "generatedAnswer": false,
    })
}

fn sanitize_fts_query(query: &str) -> Result<String> {
    let terms = query
        .split_whitespace()
        .map(|term| term.trim_matches(|character: char| !character.is_ascii_alphanumeric()))
        .filter(|term| !term.is_empty())
        .take(12)
        .map(|term| format!("\"{}\"", term.replace('"', "")))
        .collect::<Vec<_>>();
    if terms.is_empty() {
        bail!("Retrieval query must contain at least one searchable term");
    }
    Ok(terms.join(" OR "))
}

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn require_non_empty(value: &str, label: &str) -> Result<String> {
    normalize_optional_string(Some(value.to_string()))
        .ok_or_else(|| anyhow::anyhow!("{label} is required"))
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
    use crate::policy::LOCAL_OWNER_ACTOR_ID;
    use crate::schema::init_database;
    use tempfile::TempDir;

    fn setup_db() -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        (temp_dir, db_path)
    }

    fn create_source_and_item(
        db_path: &Path,
        visibility: CorpusVisibility,
        status: CorpusStatus,
        body: &str,
    ) -> CorpusItemView {
        let (source, _) = create_corpus_source(
            db_path,
            CorpusSourceWriteRequest {
                source_kind: Some("operator_text".to_string()),
                label: format!("{visibility:?} source"),
                uri: None,
                resource_kind: None,
                resource_id: None,
                status: Some(status),
                visibility: Some(visibility),
                provenance: Some(json!({ "seed": "test" })),
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        let (item, _) = create_corpus_item(
            db_path,
            CorpusItemWriteRequest {
                source_id: source.id,
                item_kind: Some("chunk".to_string()),
                ordinal: Some(1),
                title: format!("{visibility:?} item"),
                body_text: body.to_string(),
                resource_kind: None,
                resource_id: None,
                status: Some(status),
                visibility: Some(visibility),
                provenance: Some(json!({ "lineage": "test" })),
                metadata: Some(json!({ "embedding": "not_present" })),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        item
    }

    #[test]
    fn corpus_ingestion_persists_provenance_hash_and_fts_retrieval() {
        let (_temp_dir, db_path) = setup_db();
        let item = create_source_and_item(
            &db_path,
            CorpusVisibility::Public,
            CorpusStatus::Approved,
            "Ordo retrieves approved public bakery knowledge through local SQLite.",
        );

        assert!(item.content_hash.starts_with("sha256:"));
        assert_eq!(item.provenance["lineage"], "test");

        let retrieved = retrieve_corpus(
            &db_path,
            CorpusRetrievalQuery {
                query: "bakery knowledge".to_string(),
                viewer: Some(CorpusViewer::Public),
                actor_id: None,
                limit: Some(5),
            },
        )
        .unwrap();
        assert_eq!(retrieved.evidence_state, "evidence_found");
        assert_eq!(retrieved.results.len(), 1);
        assert_eq!(retrieved.results[0].item.id, item.id);
        assert_eq!(retrieved.results[0].evidence["generatedAnswer"], false);
    }

    #[test]
    fn public_retrieval_blocks_draft_private_and_owner_material() {
        let (_temp_dir, db_path) = setup_db();
        create_source_and_item(
            &db_path,
            CorpusVisibility::Public,
            CorpusStatus::Draft,
            "secret draft roadmap retrieval sentinel",
        );
        create_source_and_item(
            &db_path,
            CorpusVisibility::Owner,
            CorpusStatus::Approved,
            "owner only payroll retrieval sentinel",
        );
        create_source_and_item(
            &db_path,
            CorpusVisibility::Public,
            CorpusStatus::Approved,
            "public menu retrieval sentinel",
        );

        let public = retrieve_corpus(
            &db_path,
            CorpusRetrievalQuery {
                query: "retrieval sentinel".to_string(),
                viewer: Some(CorpusViewer::Public),
                actor_id: None,
                limit: Some(10),
            },
        )
        .unwrap();
        assert_eq!(public.results.len(), 1);
        assert!(public.results[0].item.body_text.contains("public menu"));

        let owner = retrieve_corpus(
            &db_path,
            CorpusRetrievalQuery {
                query: "retrieval sentinel".to_string(),
                viewer: Some(CorpusViewer::Owner),
                actor_id: Some(LOCAL_OWNER_ACTOR_ID.to_string()),
                limit: Some(10),
            },
        )
        .unwrap();
        assert_eq!(owner.results.len(), 2);
        assert!(owner
            .results
            .iter()
            .any(|result| result.item.body_text.contains("owner only payroll")));
        assert!(!owner
            .results
            .iter()
            .any(|result| result.item.body_text.contains("secret draft roadmap")));
    }

    #[test]
    fn missing_evidence_is_explicit() {
        let (_temp_dir, db_path) = setup_db();
        let response = retrieve_corpus(
            &db_path,
            CorpusRetrievalQuery {
                query: "nonexistent evidence".to_string(),
                viewer: Some(CorpusViewer::Public),
                actor_id: None,
                limit: Some(3),
            },
        )
        .unwrap();

        assert_eq!(response.evidence_state, "missing_evidence");
        assert!(response.results.is_empty());
        assert!(response
            .limitations
            .iter()
            .any(|limitation| limitation.contains("no embeddings")));
    }
}
