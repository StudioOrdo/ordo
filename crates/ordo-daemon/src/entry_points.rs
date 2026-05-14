use anyhow::{bail, ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

use crate::artifacts::{
    add_artifact_version, link_artifact, load_artifact, record_artifact, ArtifactInput,
    ArtifactLinkInput, ArtifactVersionView, ArtifactView,
};
use crate::events::{append_realtime_event_tx, system_event, RealtimeEvent};
use crate::offers::list_public_available_offers;
use crate::public_surfaces::public_surfaces;

pub const QR_ASSET_ARTIFACT_KIND: &str = "tracked_entry.qr_asset";
const QR_ASSET_CONTRACT_SCHEMA_VERSION: &str = "ordo.qr_asset_contract.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryPointStatus {
    Active,
    Disabled,
    Archived,
}

impl EntryPointStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::Archived => "archived",
        }
    }
}

impl TryFrom<&str> for EntryPointStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "disabled" => Ok(Self::Disabled),
            "archived" => Ok(Self::Archived),
            _ => bail!("Unsupported entry point status: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicDestinationSurface {
    About,
    Offers,
    Asks,
    Feed,
}

impl PublicDestinationSurface {
    fn as_str(self) -> &'static str {
        match self {
            Self::About => "about",
            Self::Offers => "offers",
            Self::Asks => "asks",
            Self::Feed => "feed",
        }
    }
}

impl TryFrom<&str> for PublicDestinationSurface {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "about" => Ok(Self::About),
            "offers" => Ok(Self::Offers),
            "asks" => Ok(Self::Asks),
            "feed" => Ok(Self::Feed),
            _ => bail!("Unsupported public destination surface: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisitorSessionStatus {
    Active,
    Ended,
}

impl VisitorSessionStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Ended => "ended",
        }
    }
}

impl TryFrom<&str> for VisitorSessionStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "ended" => Ok(Self::Ended),
            _ => bail!("Unsupported visitor session status: {value}"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryPointListResponse {
    pub entry_points: Vec<TrackedEntryPointView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisitorSessionListResponse {
    pub sessions: Vec<VisitorSessionView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedEntryPointView {
    pub id: String,
    pub slug: String,
    pub label: String,
    pub status: EntryPointStatus,
    pub source_kind: String,
    pub source_label: Option<String>,
    pub destination_surface: PublicDestinationSurface,
    pub destination_id: Option<String>,
    pub public_path: String,
    pub qr_payload: Value,
    pub attribution: Value,
    pub metadata: Value,
    pub created_by_actor_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicEntryPointView {
    pub slug: String,
    pub label: String,
    pub destination_surface: PublicDestinationSurface,
    pub destination_id: Option<String>,
    pub public_path: String,
    pub qr_payload: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisitorSessionView {
    pub id: String,
    pub entry_point_id: String,
    pub entry_point_slug: String,
    pub status: VisitorSessionStatus,
    pub destination_surface: PublicDestinationSurface,
    pub destination_id: Option<String>,
    pub attribution: Value,
    pub created_at: String,
    pub updated_at: String,
    pub last_seen_at: String,
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicVisitorSessionView {
    pub id: String,
    pub entry_point_slug: String,
    pub status: VisitorSessionStatus,
    pub destination_surface: PublicDestinationSurface,
    pub destination_id: Option<String>,
    pub attribution: Value,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrAssetView {
    pub artifact: ArtifactView,
    pub version: Option<ArtifactVersionView>,
    pub entry_point_id: String,
    pub entry_point_slug: String,
    pub destination_surface: PublicDestinationSurface,
    pub destination_id: Option<String>,
    pub public_path: String,
    pub payload: String,
    pub payload_hash: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryPointWriteRequest {
    pub slug: String,
    pub label: String,
    pub status: Option<EntryPointStatus>,
    pub source_kind: String,
    pub source_label: Option<String>,
    pub destination_surface: PublicDestinationSurface,
    pub destination_id: Option<String>,
    pub attribution: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisitorSessionCreateRequest {
    pub entry_point_slug: String,
    pub session_id: Option<String>,
    pub user_agent: Option<String>,
    pub attribution: Option<Value>,
}

#[derive(Debug, Clone)]
struct TrackedEntryPointRecord {
    id: String,
    slug: String,
    label: String,
    status: EntryPointStatus,
    source_kind: String,
    source_label: Option<String>,
    destination_surface: PublicDestinationSurface,
    destination_id: Option<String>,
    public_path: String,
    qr_payload: Value,
    attribution: Value,
    metadata: Value,
    created_by_actor_id: Option<String>,
    created_at: String,
    updated_at: String,
    archived_at: Option<String>,
}

#[derive(Debug, Clone)]
struct VisitorSessionRecord {
    id: String,
    entry_point_id: String,
    entry_point_slug: String,
    status: VisitorSessionStatus,
    destination_surface: PublicDestinationSurface,
    destination_id: Option<String>,
    attribution: Value,
    created_at: String,
    updated_at: String,
    last_seen_at: String,
    ended_at: Option<String>,
}

pub fn list_entry_points(db_path: &Path) -> Result<EntryPointListResponse> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, slug, label, status, source_kind, source_label, destination_surface,
                destination_id, public_path, qr_payload_json, attribution_json, metadata_json,
                created_by_actor_id, created_at, updated_at, archived_at
         FROM tracked_entry_points
         ORDER BY updated_at DESC, id DESC",
    )?;
    let entry_points = statement
        .query_map([], tracked_entry_point_from_row)?
        .map(|row| row.map(TrackedEntryPointRecord::into_view))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(EntryPointListResponse { entry_points })
}

pub fn list_visitor_sessions(db_path: &Path) -> Result<VisitorSessionListResponse> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, entry_point_id, entry_point_slug, status, destination_surface, destination_id,
                attribution_json, created_at, updated_at, last_seen_at, ended_at
         FROM visitor_sessions
         ORDER BY created_at DESC, id DESC",
    )?;
    let sessions = statement
        .query_map([], visitor_session_from_row)?
        .map(|row| row.map(VisitorSessionRecord::into_view))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(VisitorSessionListResponse { sessions })
}

pub fn create_entry_point(
    db_path: &Path,
    request: EntryPointWriteRequest,
    actor_id: Option<&str>,
) -> Result<(TrackedEntryPointView, RealtimeEvent)> {
    ensure_public_destination(
        db_path,
        request.destination_surface,
        request.destination_id.as_deref(),
    )?;
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let now = Utc::now().to_rfc3339();
    let id = format!("entry_point_{}", Uuid::new_v4());
    let slug = require_identifier(&request.slug, "Entry point slug")?;
    let label = require_text(&request.label, "Entry point label")?;
    let source_kind = require_identifier(&request.source_kind, "Entry point source kind")?;
    let destination_id = normalize_identifier(request.destination_id, "Destination id")?;
    let status = request.status.unwrap_or(EntryPointStatus::Active);
    let public_path = public_path(&slug);
    let qr_payload = qr_payload(
        &slug,
        &public_path,
        request.destination_surface,
        destination_id.as_deref(),
    );
    let archived_at = (status == EntryPointStatus::Archived).then(|| now.clone());

    transaction.execute(
        "INSERT INTO tracked_entry_points (
            id, slug, label, status, source_kind, source_label, destination_surface,
            destination_id, public_path, qr_payload_json, attribution_json, metadata_json,
            created_by_actor_id, created_at, updated_at, archived_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14, ?15)",
        params![
            id,
            slug,
            label,
            status.as_str(),
            source_kind,
            normalize_optional_string(request.source_label),
            request.destination_surface.as_str(),
            destination_id,
            public_path,
            qr_payload.to_string(),
            request.attribution.unwrap_or_else(|| json!({})).to_string(),
            request.metadata.unwrap_or_else(|| json!({})).to_string(),
            actor_id,
            now,
            archived_at,
        ],
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "entry_point.created",
            json!({
                "entryPointId": id,
                "slug": slug,
                "status": status.as_str(),
                "destinationSurface": request.destination_surface.as_str(),
            }),
        ),
    )?;
    transaction.commit()?;
    let record = find_entry_point_by_id(&connection, &id)?.expect("entry point just inserted");
    Ok((record.into_view(), event))
}

pub fn update_entry_point(
    db_path: &Path,
    entry_point_id: &str,
    request: EntryPointWriteRequest,
    actor_id: Option<&str>,
) -> Result<(TrackedEntryPointView, RealtimeEvent)> {
    ensure_public_destination(
        db_path,
        request.destination_surface,
        request.destination_id.as_deref(),
    )?;
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let existing = find_entry_point_by_id(&transaction, entry_point_id)?
        .ok_or_else(|| anyhow::anyhow!("Tracked entry point was not found: {entry_point_id}"))?;
    let now = Utc::now().to_rfc3339();
    let slug = require_identifier(&request.slug, "Entry point slug")?;
    let label = require_text(&request.label, "Entry point label")?;
    let source_kind = require_identifier(&request.source_kind, "Entry point source kind")?;
    let destination_id = normalize_identifier(request.destination_id, "Destination id")?;
    let status = request.status.unwrap_or(existing.status);
    let public_path = public_path(&slug);
    let qr_payload = qr_payload(
        &slug,
        &public_path,
        request.destination_surface,
        destination_id.as_deref(),
    );
    let archived_at =
        if status == EntryPointStatus::Archived && existing.status != EntryPointStatus::Archived {
            Some(now.clone())
        } else if status == EntryPointStatus::Archived {
            existing.archived_at
        } else {
            None
        };

    transaction.execute(
        "UPDATE tracked_entry_points
         SET slug = ?1,
             label = ?2,
             status = ?3,
             source_kind = ?4,
             source_label = ?5,
             destination_surface = ?6,
             destination_id = ?7,
             public_path = ?8,
             qr_payload_json = ?9,
             attribution_json = ?10,
             metadata_json = ?11,
             created_by_actor_id = COALESCE(created_by_actor_id, ?12),
             updated_at = ?13,
             archived_at = ?14
         WHERE id = ?15",
        params![
            slug,
            label,
            status.as_str(),
            source_kind,
            normalize_optional_string(request.source_label),
            request.destination_surface.as_str(),
            destination_id,
            public_path,
            qr_payload.to_string(),
            request
                .attribution
                .unwrap_or(existing.attribution)
                .to_string(),
            request.metadata.unwrap_or(existing.metadata).to_string(),
            actor_id,
            now,
            archived_at,
            entry_point_id,
        ],
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "entry_point.updated",
            json!({
                "entryPointId": entry_point_id,
                "slug": slug,
                "status": status.as_str(),
                "destinationSurface": request.destination_surface.as_str(),
            }),
        ),
    )?;
    transaction.commit()?;
    let record =
        find_entry_point_by_id(&connection, entry_point_id)?.expect("entry point just updated");
    Ok((record.into_view(), event))
}

pub fn resolve_entry_point(db_path: &Path, slug: &str) -> Result<PublicEntryPointView> {
    let connection = Connection::open(db_path)?;
    let slug = require_identifier(slug, "Entry point slug")?;
    let record = find_entry_point_by_slug(&connection, &slug)?
        .ok_or_else(|| anyhow::anyhow!("Tracked entry point was not found: {slug}"))?;
    if record.status != EntryPointStatus::Active {
        bail!("Tracked entry point is not active.");
    }
    ensure_public_destination(
        db_path,
        record.destination_surface,
        record.destination_id.as_deref(),
    )?;
    Ok(record.into_public_view())
}

pub fn create_visitor_session(
    db_path: &Path,
    request: VisitorSessionCreateRequest,
) -> Result<(VisitorSessionView, RealtimeEvent)> {
    let entry_point = resolve_entry_point(db_path, &request.entry_point_slug)?;
    let mut connection = Connection::open(db_path)?;
    let record = find_entry_point_by_slug(&connection, &entry_point.slug)?
        .expect("entry point resolved before session creation");
    let transaction = connection.transaction()?;
    let now = Utc::now().to_rfc3339();
    let user_agent_hash = request.user_agent.as_deref().and_then(hash_optional_text);
    let additional_attribution = sanitize_public_attribution(request.attribution);

    if let Some(session_id) = normalize_identifier(request.session_id, "Visitor session id")? {
        let existing = find_visitor_session_by_id(&transaction, &session_id)?
            .ok_or_else(|| anyhow::anyhow!("Visitor session was not found: {session_id}"))?;
        if existing.status != VisitorSessionStatus::Active || existing.ended_at.is_some() {
            bail!("Visitor session is not active.");
        }
        if existing.entry_point_id != record.id || existing.entry_point_slug != record.slug {
            bail!("Visitor session does not belong to this entry point.");
        }
        let attribution = merge_attribution(existing.attribution, additional_attribution);
        transaction.execute(
            "UPDATE visitor_sessions
             SET attribution_json = ?1,
                 user_agent_hash = COALESCE(user_agent_hash, ?2),
                 updated_at = ?3,
                 last_seen_at = ?3
             WHERE id = ?4",
            params![attribution.to_string(), user_agent_hash, now, session_id],
        )?;
        append_visitor_session_event_tx(
            &transaction,
            &format!("visitor_session_event_{}", Uuid::new_v4()),
            &session_id,
            &record.id,
            "visitor_session.resumed",
            json!({
                "sessionId": session_id,
                "entryPointId": record.id,
                "entryPointSlug": record.slug,
                "destinationSurface": record.destination_surface.as_str(),
                "destinationId": record.destination_id,
            }),
            &now,
        )?;
        let event = append_realtime_event_tx(
            &transaction,
            &system_event(
                "visitor_session.resumed",
                json!({
                    "sessionId": session_id,
                    "entryPointId": record.id,
                    "entryPointSlug": record.slug,
                    "destinationSurface": record.destination_surface.as_str(),
                    "destinationId": record.destination_id,
                }),
            ),
        )?;
        transaction.commit()?;
        let session =
            find_visitor_session_by_id(&connection, &session_id)?.expect("visitor session updated");
        return Ok((session.into_view(), event));
    }

    let id = format!("visitor_session_{}", Uuid::new_v4());
    let attribution = merge_attribution(
        sanitize_public_attribution(Some(record.attribution.clone())).unwrap_or_else(|| json!({})),
        additional_attribution,
    );

    transaction.execute(
        "INSERT INTO visitor_sessions (
            id, entry_point_id, entry_point_slug, status, destination_surface, destination_id,
            attribution_json, user_agent_hash, created_at, updated_at, last_seen_at, ended_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?9, NULL)",
        params![
            id,
            record.id,
            record.slug,
            VisitorSessionStatus::Active.as_str(),
            record.destination_surface.as_str(),
            record.destination_id,
            attribution.to_string(),
            user_agent_hash,
            now,
        ],
    )?;
    append_visitor_session_event_tx(
        &transaction,
        &format!("visitor_session_event_{}", Uuid::new_v4()),
        &id,
        &record.id,
        "visitor_session.started",
        json!({
            "sessionId": id,
            "entryPointId": record.id,
            "entryPointSlug": record.slug,
            "destinationSurface": record.destination_surface.as_str(),
            "destinationId": record.destination_id,
        }),
        &now,
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "visitor_session.started",
            json!({
                "sessionId": id,
                "entryPointId": record.id,
                "entryPointSlug": record.slug,
                "destinationSurface": record.destination_surface.as_str(),
                "destinationId": record.destination_id,
            }),
        ),
    )?;
    transaction.commit()?;
    let session =
        find_visitor_session_by_id(&connection, &id)?.expect("visitor session just inserted");
    Ok((session.into_view(), event))
}

pub fn create_or_refresh_qr_asset(
    db_path: &Path,
    entry_point_id: &str,
    actor_id: Option<&str>,
) -> Result<QrAssetView> {
    let mut connection = Connection::open(db_path)?;
    let entry_point = find_entry_point_by_id(&connection, entry_point_id)?
        .ok_or_else(|| anyhow::anyhow!("Tracked entry point was not found: {entry_point_id}"))?;
    ensure!(
        entry_point.status == EntryPointStatus::Active,
        "Tracked entry point is not active."
    );
    ensure_public_destination(
        db_path,
        entry_point.destination_surface,
        entry_point.destination_id.as_deref(),
    )?;

    let contract = qr_asset_contract(&entry_point, actor_id)?;
    let contract_json = serde_json::to_value(&contract)?;
    let content_hash = stable_json_hash(&contract_json)?;
    if let Some(existing) = load_existing_qr_asset(&connection, &entry_point.id, &content_hash)? {
        return Ok(QrAssetView {
            artifact: existing,
            version: None,
            entry_point_id: entry_point.id,
            entry_point_slug: entry_point.slug,
            destination_surface: entry_point.destination_surface,
            destination_id: entry_point.destination_id,
            public_path: entry_point.public_path.clone(),
            payload: contract.payload,
            payload_hash: content_hash,
        });
    }

    let transaction = connection.transaction()?;
    let (artifact, _) = record_artifact(
        &transaction,
        ArtifactInput {
            artifact_kind: QR_ASSET_ARTIFACT_KIND.to_string(),
            title: format!("QR asset for tracked entry `{}`", entry_point.slug),
            status: "ready".to_string(),
            visibility_ceiling: "public".to_string(),
            summary: format!(
                "Durable QR asset contract for public entry path `{}`.",
                entry_point.public_path
            ),
            source_kind: Some("tracked_entry_point".to_string()),
            source_id: Some(entry_point.id.clone()),
            evidence_refs: contract.evidence_refs.clone(),
            provenance: json!({
                "schemaVersion": QR_ASSET_CONTRACT_SCHEMA_VERSION,
                "generatedBy": "entry_points.create_or_refresh_qr_asset",
                "entryPointId": entry_point.id,
                "entryPointSlug": entry_point.slug,
                "contract": contract_json,
            }),
            content_hash: content_hash.clone(),
            storage_uri: Some(format!(
                "ordo://artifacts/qr-assets/{}/{}.svg",
                entry_point.id,
                content_hash_suffix(&content_hash)
            )),
            health_status: Some("contract_only".to_string()),
            created_by_job_id: None,
        },
    )?;
    let version = add_artifact_version(
        &transaction,
        &artifact.id,
        &content_hash,
        artifact.storage_uri.as_deref(),
        json!({
            "schemaVersion": QR_ASSET_CONTRACT_SCHEMA_VERSION,
            "contract": contract,
            "liveProviderCalled": false,
            "externalEncoderCalled": false,
        }),
    )?;
    let _ = link_artifact(
        &transaction,
        &artifact.id,
        ArtifactLinkInput {
            link_kind: "tracked_entry_qr_asset".to_string(),
            source_kind: "tracked_entry_point".to_string(),
            source_id: entry_point.id.clone(),
            relation: "renders_scan_route_for".to_string(),
            evidence_refs: vec![format!("entry_point:{}", entry_point.id)],
            provenance: json!({
                "schemaVersion": QR_ASSET_CONTRACT_SCHEMA_VERSION,
                "entryPointSlug": entry_point.slug,
                "publicPath": entry_point.public_path,
            }),
        },
    )?;
    transaction.commit()?;

    Ok(QrAssetView {
        artifact,
        version: Some(version),
        entry_point_id: entry_point.id,
        entry_point_slug: entry_point.slug,
        destination_surface: entry_point.destination_surface,
        destination_id: entry_point.destination_id,
        public_path: entry_point.public_path.clone(),
        payload: contract.payload,
        payload_hash: content_hash,
    })
}

fn append_visitor_session_event_tx(
    transaction: &rusqlite::Transaction<'_>,
    id: &str,
    session_id: &str,
    entry_point_id: &str,
    event_type: &str,
    payload: Value,
    occurred_at: &str,
) -> Result<()> {
    transaction.execute(
        "INSERT INTO visitor_session_events (
            id, session_id, entry_point_id, event_type, payload_json, occurred_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            session_id,
            entry_point_id,
            event_type,
            payload.to_string(),
            occurred_at
        ],
    )?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct QrAssetContract {
    schema_version: String,
    entry_point_id: String,
    entry_point_slug: String,
    public_path: String,
    payload: String,
    payload_kind: String,
    destination_surface: String,
    destination_id: Option<String>,
    artifact_format: String,
    encoding_status: String,
    public_context: Value,
    evidence_refs: Vec<String>,
    limitations: Vec<String>,
}

fn qr_asset_contract(
    entry_point: &TrackedEntryPointRecord,
    actor_id: Option<&str>,
) -> Result<QrAssetContract> {
    ensure_safe_qr_text(&entry_point.slug)?;
    ensure_safe_qr_text(&entry_point.public_path)?;
    if let Some(destination_id) = &entry_point.destination_id {
        ensure_safe_qr_text(destination_id)?;
    }
    if let Some(actor_id) = actor_id {
        ensure_safe_qr_text(actor_id)?;
    }
    let public_context = sanitize_public_attribution(Some(entry_point.attribution.clone()))
        .unwrap_or_else(|| {
            json!({
                "source": entry_point.source_kind,
            })
        });
    let payload = entry_point.public_path.clone();
    Ok(QrAssetContract {
        schema_version: QR_ASSET_CONTRACT_SCHEMA_VERSION.to_string(),
        entry_point_id: entry_point.id.clone(),
        entry_point_slug: entry_point.slug.clone(),
        public_path: entry_point.public_path.clone(),
        payload,
        payload_kind: "relative_url".to_string(),
        destination_surface: entry_point.destination_surface.as_str().to_string(),
        destination_id: entry_point.destination_id.clone(),
        artifact_format: "svg".to_string(),
        encoding_status: "contract_only".to_string(),
        public_context,
        evidence_refs: vec![
            format!("entry_point:{}", entry_point.id),
            format!("public_path:{}", entry_point.public_path),
        ],
        limitations: vec![
            "QR asset is a deterministic local artifact contract; no live image provider was called."
                .to_string(),
            "The first QR payload uses the tracked entry public path so server-side context owns meaning."
                .to_string(),
            "Private attribution, owner metadata, provider internals, prompts, and policy internals are omitted."
                .to_string(),
        ],
    })
}

fn load_existing_qr_asset(
    connection: &Connection,
    entry_point_id: &str,
    content_hash: &str,
) -> Result<Option<ArtifactView>> {
    let artifact_id = connection
        .query_row(
            "SELECT id FROM artifacts
             WHERE artifact_kind = ?1
               AND source_kind = 'tracked_entry_point'
               AND source_id = ?2
               AND content_hash = ?3
             ORDER BY created_at ASC
             LIMIT 1",
            params![QR_ASSET_ARTIFACT_KIND, entry_point_id, content_hash],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    artifact_id
        .map(|id| load_artifact(connection, &id))
        .transpose()
}

fn stable_json_hash(value: &Value) -> Result<String> {
    let serialized = serde_json::to_string(value)?;
    let mut hasher = Sha256::new();
    hasher.update(serialized.as_bytes());
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn content_hash_suffix(content_hash: &str) -> String {
    content_hash
        .strip_prefix("sha256:")
        .unwrap_or(content_hash)
        .chars()
        .take(16)
        .collect()
}

fn ensure_safe_qr_text(text: &str) -> Result<()> {
    let lower = text.to_ascii_lowercase();
    let blocked = [
        "staff routing",
        "provider internal",
        "prompt internal",
        "raw policy",
        "policy internal",
        "owner-only",
        "private artifact text",
        "compiled-plan",
        "task private payload",
        "secret",
        "api_key",
        "password",
        "bearer ",
        "graph certainty",
        "unsupported claim",
    ];
    ensure!(
        !blocked.iter().any(|needle| lower.contains(needle)),
        "QR asset text contains private/internal or unsupported claim text"
    );
    Ok(())
}

fn ensure_public_destination(
    db_path: &Path,
    surface: PublicDestinationSurface,
    destination_id: Option<&str>,
) -> Result<()> {
    let surfaces = public_surfaces(db_path)?;
    match surface {
        PublicDestinationSurface::About => {
            if destination_id.is_some() {
                bail!("About destinations do not accept a destination id.");
            }
            if !surfaces.about.readiness.ready {
                bail!("About destination is not publicly ready.");
            }
        }
        PublicDestinationSurface::Offers => {
            let mut offer_ids = surfaces
                .offers
                .items
                .iter()
                .map(|item| item.item_id.clone())
                .collect::<Vec<_>>();
            offer_ids.extend(
                list_public_available_offers(db_path)?
                    .offers
                    .into_iter()
                    .map(|offer| offer.id),
            );
            ensure_item_destination(
                "Offer",
                destination_id,
                surfaces.offers.readiness.ready || !offer_ids.is_empty(),
                offer_ids.iter().map(String::as_str),
            )?
        }
        PublicDestinationSurface::Asks => ensure_item_destination(
            "Ask",
            destination_id,
            surfaces.asks.readiness.ready,
            surfaces.asks.items.iter().map(|item| item.item_id.as_str()),
        )?,
        PublicDestinationSurface::Feed => ensure_item_destination(
            "Feed",
            destination_id,
            surfaces.feed.readiness.ready,
            surfaces.feed.items.iter().map(|item| item.item_id.as_str()),
        )?,
    }
    Ok(())
}

fn ensure_item_destination<'a>(
    label: &str,
    destination_id: Option<&str>,
    ready: bool,
    item_ids: impl Iterator<Item = &'a str>,
) -> Result<()> {
    if let Some(destination_id) = destination_id {
        if item_ids
            .into_iter()
            .any(|item_id| item_id == destination_id)
        {
            return Ok(());
        }
        bail!("{label} destination is not publicly available: {destination_id}");
    }
    if ready {
        Ok(())
    } else {
        bail!("{label} destination is not publicly ready.");
    }
}

fn find_entry_point_by_id(
    connection: &Connection,
    entry_point_id: &str,
) -> rusqlite::Result<Option<TrackedEntryPointRecord>> {
    connection
        .query_row(
            "SELECT id, slug, label, status, source_kind, source_label, destination_surface,
                    destination_id, public_path, qr_payload_json, attribution_json, metadata_json,
                    created_by_actor_id, created_at, updated_at, archived_at
             FROM tracked_entry_points
             WHERE id = ?1",
            [entry_point_id],
            tracked_entry_point_from_row,
        )
        .optional()
}

fn find_entry_point_by_slug(
    connection: &Connection,
    slug: &str,
) -> rusqlite::Result<Option<TrackedEntryPointRecord>> {
    connection
        .query_row(
            "SELECT id, slug, label, status, source_kind, source_label, destination_surface,
                    destination_id, public_path, qr_payload_json, attribution_json, metadata_json,
                    created_by_actor_id, created_at, updated_at, archived_at
             FROM tracked_entry_points
             WHERE slug = ?1",
            [slug],
            tracked_entry_point_from_row,
        )
        .optional()
}

fn find_visitor_session_by_id(
    connection: &Connection,
    session_id: &str,
) -> rusqlite::Result<Option<VisitorSessionRecord>> {
    connection
        .query_row(
            "SELECT id, entry_point_id, entry_point_slug, status, destination_surface, destination_id,
                    attribution_json, created_at, updated_at, last_seen_at, ended_at
             FROM visitor_sessions
             WHERE id = ?1",
            [session_id],
            visitor_session_from_row,
        )
        .optional()
}

fn tracked_entry_point_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<TrackedEntryPointRecord> {
    let status: String = row.get(3)?;
    let destination_surface: String = row.get(6)?;
    let qr_payload_json: String = row.get(9)?;
    let attribution_json: String = row.get(10)?;
    let metadata_json: String = row.get(11)?;
    Ok(TrackedEntryPointRecord {
        id: row.get(0)?,
        slug: row.get(1)?,
        label: row.get(2)?,
        status: EntryPointStatus::try_from(status.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, error.into())
        })?,
        source_kind: row.get(4)?,
        source_label: row.get(5)?,
        destination_surface: PublicDestinationSurface::try_from(destination_surface.as_str())
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    error.into(),
                )
            })?,
        destination_id: row.get(7)?,
        public_path: row.get(8)?,
        qr_payload: serde_json::from_str(&qr_payload_json).unwrap_or_else(|_| json!({})),
        attribution: serde_json::from_str(&attribution_json).unwrap_or_else(|_| json!({})),
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        created_by_actor_id: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        archived_at: row.get(15)?,
    })
}

fn visitor_session_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<VisitorSessionRecord> {
    let status: String = row.get(3)?;
    let destination_surface: String = row.get(4)?;
    let attribution_json: String = row.get(6)?;
    Ok(VisitorSessionRecord {
        id: row.get(0)?,
        entry_point_id: row.get(1)?,
        entry_point_slug: row.get(2)?,
        status: VisitorSessionStatus::try_from(status.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, error.into())
        })?,
        destination_surface: PublicDestinationSurface::try_from(destination_surface.as_str())
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    error.into(),
                )
            })?,
        destination_id: row.get(5)?,
        attribution: serde_json::from_str(&attribution_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        last_seen_at: row.get(9)?,
        ended_at: row.get(10)?,
    })
}

impl TrackedEntryPointRecord {
    fn into_view(self) -> TrackedEntryPointView {
        TrackedEntryPointView {
            id: self.id,
            slug: self.slug,
            label: self.label,
            status: self.status,
            source_kind: self.source_kind,
            source_label: self.source_label,
            destination_surface: self.destination_surface,
            destination_id: self.destination_id,
            public_path: self.public_path,
            qr_payload: self.qr_payload,
            attribution: self.attribution,
            metadata: self.metadata,
            created_by_actor_id: self.created_by_actor_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            archived_at: self.archived_at,
        }
    }

    fn into_public_view(self) -> PublicEntryPointView {
        PublicEntryPointView {
            slug: self.slug,
            label: self.label,
            destination_surface: self.destination_surface,
            destination_id: self.destination_id,
            public_path: self.public_path,
            qr_payload: self.qr_payload,
        }
    }
}

impl VisitorSessionRecord {
    fn into_view(self) -> VisitorSessionView {
        VisitorSessionView {
            id: self.id,
            entry_point_id: self.entry_point_id,
            entry_point_slug: self.entry_point_slug,
            status: self.status,
            destination_surface: self.destination_surface,
            destination_id: self.destination_id,
            attribution: self.attribution,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_seen_at: self.last_seen_at,
            ended_at: self.ended_at,
        }
    }
}

impl VisitorSessionView {
    pub fn into_public_view(self) -> PublicVisitorSessionView {
        PublicVisitorSessionView {
            id: self.id,
            entry_point_slug: self.entry_point_slug,
            status: self.status,
            destination_surface: self.destination_surface,
            destination_id: self.destination_id,
            attribution: sanitize_public_attribution(Some(self.attribution))
                .unwrap_or_else(|| json!({})),
            last_seen_at: self.last_seen_at,
        }
    }
}

fn public_path(slug: &str) -> String {
    format!("/public/e/{slug}")
}

fn qr_payload(
    slug: &str,
    public_path: &str,
    destination_surface: PublicDestinationSurface,
    destination_id: Option<&str>,
) -> Value {
    json!({
        "kind": "ordo.tracked_entry_point",
        "version": 1,
        "slug": slug,
        "path": public_path,
        "destination": {
            "surface": destination_surface.as_str(),
            "id": destination_id,
        }
    })
}

fn merge_attribution(base: Value, additional: Option<Value>) -> Value {
    match (base, additional) {
        (Value::Object(mut base), Some(Value::Object(additional))) => {
            for (key, value) in additional {
                base.insert(key, value);
            }
            Value::Object(base)
        }
        (base, _) => base,
    }
}

fn sanitize_public_attribution(value: Option<Value>) -> Option<Value> {
    let Value::Object(source) = value? else {
        return None;
    };
    let mut safe = Map::new();
    for key in [
        "campaign",
        "medium",
        "source",
        "referrer",
        "referralCode",
        "offerSlug",
        "utmSource",
        "utmMedium",
        "utmCampaign",
        "utmTerm",
        "utmContent",
        "scanOccurredAt",
        "localTime",
        "timeZone",
        "eventId",
        "eventLabel",
    ] {
        if let Some(value) = source.get(key).and_then(public_string_value) {
            safe.insert(key.to_string(), Value::String(value));
        }
    }
    if let Some(location) = sanitize_public_location(source.get("location")) {
        safe.insert("location".to_string(), location);
    } else if let Some(location) = sanitize_public_location_fields(&source) {
        safe.insert("location".to_string(), location);
    }
    (!safe.is_empty()).then_some(Value::Object(safe))
}

fn sanitize_public_location(value: Option<&Value>) -> Option<Value> {
    let Some(Value::Object(source)) = value else {
        return None;
    };
    sanitize_public_location_fields(source)
}

fn sanitize_public_location_fields(source: &Map<String, Value>) -> Option<Value> {
    let mut safe = Map::new();
    for (source_key, output_key) in [
        ("locationLabel", "label"),
        ("locationKind", "kind"),
        ("locationSource", "source"),
        ("label", "label"),
        ("kind", "kind"),
        ("source", "source"),
        ("country", "country"),
        ("region", "region"),
        ("locality", "locality"),
    ] {
        if let Some(value) = source.get(source_key).and_then(public_string_value) {
            safe.insert(output_key.to_string(), Value::String(value));
        }
    }
    if let Some(Value::Bool(coarse)) = source.get("coarse") {
        safe.insert("coarse".to_string(), Value::Bool(*coarse));
    }
    if let Some(precision) = source.get("precision").and_then(public_string_value) {
        if matches!(precision.as_str(), "coarse" | "event" | "manual") {
            safe.insert("precision".to_string(), Value::String(precision));
        }
    }
    (!safe.is_empty()).then_some(Value::Object(safe))
}

fn public_string_value(value: &Value) -> Option<String> {
    let Value::String(value) = value else {
        return None;
    };
    normalize_optional_string(Some(value.clone())).filter(|value| value.len() <= 240)
}

fn hash_optional_text(value: &str) -> Option<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return None;
    }
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    Some(format!("sha256:{:x}", hasher.finalize()))
}

fn require_identifier(value: &str, label: &str) -> Result<String> {
    let normalized = normalize_optional_string(Some(value.to_string()))
        .ok_or_else(|| anyhow::anyhow!("{label} is required."))?;
    if normalized.len() > 120
        || !normalized.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
    {
        bail!("{label} must be a stable identifier.");
    }
    Ok(normalized)
}

fn normalize_identifier(value: Option<String>, label: &str) -> Result<Option<String>> {
    value
        .map(|value| require_identifier(&value, label))
        .transpose()
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
    use crate::business::{
        create_business_fact, BusinessFactVisibility, BusinessFactWriteRequest, PublicationState,
    };
    use crate::policy::LOCAL_OWNER_ACTOR_ID;
    use crate::schema::init_database;
    use tempfile::TempDir;

    #[test]
    fn entry_point_creation_persists_public_payload_and_emits_event() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        seed_public_about(&db_path);

        let (entry_point, event) = create_entry_point(
            &db_path,
            EntryPointWriteRequest {
                slug: "front-door".to_string(),
                label: "Front Door QR".to_string(),
                status: None,
                source_kind: "qr".to_string(),
                source_label: Some("Counter card".to_string()),
                destination_surface: PublicDestinationSurface::About,
                destination_id: None,
                attribution: Some(json!({ "campaign": "spring" })),
                metadata: Some(json!({ "batch": "a" })),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(entry_point.slug, "front-door");
        assert_eq!(entry_point.public_path, "/public/e/front-door");
        assert_eq!(entry_point.qr_payload["kind"], "ordo.tracked_entry_point");
        assert_eq!(entry_point.attribution["campaign"], "spring");
        assert_eq!(event.event_type, "entry_point.created");
    }

    #[test]
    fn qr_asset_artifact_is_durable_public_safe_and_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        seed_public_about(&db_path);
        let (entry_point, _) = create_entry_point(
            &db_path,
            EntryPointWriteRequest {
                slug: "nyc-founder-table".to_string(),
                label: "NYC Founder Table QR".to_string(),
                status: None,
                source_kind: "event_qr".to_string(),
                source_label: Some("NYC Founder Table".to_string()),
                destination_surface: PublicDestinationSurface::About,
                destination_id: None,
                attribution: Some(json!({
                    "campaign": "nyc-pilot",
                    "medium": "qr",
                    "privateReferrerNote": "do not expose",
                })),
                metadata: Some(json!({
                    "ownerOnlyLeadScore": "hot",
                    "promptInternal": "do not expose",
                })),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let first =
            create_or_refresh_qr_asset(&db_path, &entry_point.id, Some(LOCAL_OWNER_ACTOR_ID))
                .unwrap();
        let second =
            create_or_refresh_qr_asset(&db_path, &entry_point.id, Some(LOCAL_OWNER_ACTOR_ID))
                .unwrap();

        assert_eq!(first.artifact.id, second.artifact.id);
        assert!(first.version.is_some());
        assert!(second.version.is_none());
        assert_eq!(first.artifact.artifact_kind, QR_ASSET_ARTIFACT_KIND);
        assert_eq!(first.artifact.status, "ready");
        assert_eq!(first.artifact.visibility_ceiling, "public");
        assert_eq!(first.entry_point_id, entry_point.id);
        assert_eq!(first.entry_point_slug, "nyc-founder-table");
        assert_eq!(first.payload, "/public/e/nyc-founder-table");
        assert_eq!(first.destination_surface, PublicDestinationSurface::About);
        assert!(first
            .artifact
            .evidence_refs
            .contains(&format!("entry_point:{}", entry_point.id)));
        assert_eq!(
            first.artifact.provenance["contract"]["publicPath"],
            "/public/e/nyc-founder-table"
        );
        assert_eq!(
            first.artifact.provenance["contract"]["encodingStatus"],
            "contract_only"
        );
        assert_eq!(
            first.artifact.provenance["contract"]["publicContext"]["campaign"],
            "nyc-pilot"
        );
        assert_eq!(
            first.artifact.provenance["contract"]["publicContext"]["medium"],
            "qr"
        );

        let serialized = serde_json::to_string(&first.artifact).unwrap();
        assert!(!serialized.contains("privateReferrerNote"));
        assert!(!serialized.contains("ownerOnlyLeadScore"));
        assert!(!serialized.contains("promptInternal"));

        let connection = Connection::open(&db_path).unwrap();
        let artifact_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE artifact_kind = ?1",
                [QR_ASSET_ARTIFACT_KIND],
                |row| row.get(0),
            )
            .unwrap();
        let version_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifact_versions WHERE artifact_id = ?1",
                [&first.artifact.id],
                |row| row.get(0),
            )
            .unwrap();
        let link_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifact_links WHERE artifact_id = ?1 AND source_kind = 'tracked_entry_point'",
                [&first.artifact.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(artifact_count, 1);
        assert_eq!(version_count, 1);
        assert_eq!(link_count, 1);
    }

    #[test]
    fn qr_asset_generation_rejects_inactive_or_private_destinations_without_partial_artifacts() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        seed_public_about(&db_path);
        let (entry_point, _) = create_entry_point(
            &db_path,
            EntryPointWriteRequest {
                slug: "closed-qr".to_string(),
                label: "Closed QR".to_string(),
                status: Some(EntryPointStatus::Disabled),
                source_kind: "event_qr".to_string(),
                source_label: Some("Closed event".to_string()),
                destination_surface: PublicDestinationSurface::About,
                destination_id: None,
                attribution: None,
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let error = create_or_refresh_qr_asset(&db_path, &entry_point.id, None).unwrap_err();
        assert!(error.to_string().contains("not active"));

        update_entry_point(
            &db_path,
            &entry_point.id,
            EntryPointWriteRequest {
                slug: "closed-qr".to_string(),
                label: "Closed QR".to_string(),
                status: Some(EntryPointStatus::Archived),
                source_kind: "event_qr".to_string(),
                source_label: Some("Closed event".to_string()),
                destination_surface: PublicDestinationSurface::About,
                destination_id: None,
                attribution: None,
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let error = create_or_refresh_qr_asset(&db_path, &entry_point.id, None).unwrap_err();
        assert!(error.to_string().contains("not active"));

        let connection = Connection::open(&db_path).unwrap();
        let artifact_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE artifact_kind = ?1",
                [QR_ASSET_ARTIFACT_KIND],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(artifact_count, 0);
    }

    #[test]
    fn entry_point_rejects_non_public_destination() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        create_business_fact(
            &db_path,
            BusinessFactWriteRequest {
                subject_type: None,
                subject_id: None,
                fact_key: "about.tagline".to_string(),
                value: json!("Private draft"),
                source_kind: None,
                source_label: None,
                source_uri: None,
                provenance: None,
                visibility: Some(BusinessFactVisibility::Owner),
                publication_state: Some(PublicationState::Draft),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let result = create_entry_point(
            &db_path,
            EntryPointWriteRequest {
                slug: "private-door".to_string(),
                label: "Private Door".to_string(),
                status: None,
                source_kind: "link".to_string(),
                source_label: None,
                destination_surface: PublicDestinationSurface::About,
                destination_id: None,
                attribution: None,
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        );

        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not publicly ready"));
    }

    #[test]
    fn visitor_session_preserves_entry_context_and_records_events() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        seed_public_about(&db_path);
        create_entry_point(
            &db_path,
            EntryPointWriteRequest {
                slug: "newsletter".to_string(),
                label: "Newsletter link".to_string(),
                status: None,
                source_kind: "campaign".to_string(),
                source_label: Some("May newsletter".to_string()),
                destination_surface: PublicDestinationSurface::About,
                destination_id: None,
                attribution: Some(json!({ "campaign": "may" })),
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let (session, event) = create_visitor_session(
            &db_path,
            VisitorSessionCreateRequest {
                entry_point_slug: "newsletter".to_string(),
                session_id: None,
                user_agent: Some("Test Browser".to_string()),
                attribution: Some(json!({ "medium": "email" })),
            },
        )
        .unwrap();

        assert_eq!(session.entry_point_slug, "newsletter");
        assert_eq!(session.attribution["campaign"], "may");
        assert_eq!(session.attribution["medium"], "email");
        assert_eq!(event.event_type, "visitor_session.started");

        let connection = Connection::open(&db_path).unwrap();
        let visitor_event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM visitor_session_events WHERE session_id = ?1",
                [&session.id],
                |row| row.get(0),
            )
            .unwrap();
        let realtime_event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM realtime_events WHERE event_type = 'visitor_session.started'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(visitor_event_count, 1);
        assert_eq!(realtime_event_count, 1);
    }

    #[test]
    fn visitor_session_resume_reuses_session_and_records_safe_context() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        seed_public_about(&db_path);
        create_entry_point(
            &db_path,
            EntryPointWriteRequest {
                slug: "nyc-meetup".to_string(),
                label: "NYC meetup QR".to_string(),
                status: None,
                source_kind: "qr".to_string(),
                source_label: Some("NYC Founder Meetup".to_string()),
                destination_surface: PublicDestinationSurface::About,
                destination_id: None,
                attribution: Some(json!({ "campaign": "nyc-pilot" })),
                metadata: Some(json!({ "internalBatch": "owner-only" })),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let (started, _) = create_visitor_session(
            &db_path,
            VisitorSessionCreateRequest {
                entry_point_slug: "nyc-meetup".to_string(),
                session_id: None,
                user_agent: Some("Test Browser".to_string()),
                attribution: Some(json!({
                    "medium": "qr",
                    "rawPrompt": "do not store this",
                    "location": {
                        "label": "NYC Founder Meetup",
                        "kind": "event",
                        "latitude": 40.7128,
                        "longitude": -74.0060,
                        "address": "do not store this"
                    }
                })),
            },
        )
        .unwrap();

        let (resumed, event) = create_visitor_session(
            &db_path,
            VisitorSessionCreateRequest {
                entry_point_slug: "nyc-meetup".to_string(),
                session_id: Some(started.id.clone()),
                user_agent: Some("Test Browser".to_string()),
                attribution: Some(json!({
                    "scanOccurredAt": "2026-05-13T11:00:00-04:00",
                    "timeZone": "America/New_York",
                    "location": {
                        "label": "NYC Founder Meetup",
                        "coarse": true,
                        "lat": 40.7128,
                        "lng": -74.0060
                    },
                    "providerSecret": "do not store this either"
                })),
            },
        )
        .unwrap();

        assert_eq!(resumed.id, started.id);
        assert_eq!(event.event_type, "visitor_session.resumed");
        assert_eq!(resumed.attribution["campaign"], "nyc-pilot");
        assert_eq!(resumed.attribution["medium"], "qr");
        assert_eq!(
            resumed.attribution["scanOccurredAt"],
            "2026-05-13T11:00:00-04:00"
        );
        assert_eq!(resumed.attribution["timeZone"], "America/New_York");
        assert_eq!(
            resumed.attribution["location"]["label"],
            "NYC Founder Meetup"
        );
        assert_eq!(resumed.attribution["location"]["coarse"], true);

        let serialized = serde_json::to_string(&resumed.attribution).unwrap();
        assert!(!serialized.contains("latitude"));
        assert!(!serialized.contains("longitude"));
        assert!(!serialized.contains("\"lat\""));
        assert!(!serialized.contains("\"lng\""));
        assert!(!serialized.contains("address"));
        assert!(!serialized.contains("rawPrompt"));
        assert!(!serialized.contains("providerSecret"));

        let connection = Connection::open(&db_path).unwrap();
        let session_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM visitor_sessions", [], |row| {
                row.get(0)
            })
            .unwrap();
        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM visitor_session_events WHERE session_id = ?1",
                [&started.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(session_count, 1);
        assert_eq!(event_count, 2);
    }

    #[test]
    fn visitor_session_rejects_archived_entry_points() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        seed_public_about(&db_path);
        let (entry_point, _) = create_entry_point(
            &db_path,
            EntryPointWriteRequest {
                slug: "closed-event".to_string(),
                label: "Closed event".to_string(),
                status: None,
                source_kind: "qr".to_string(),
                source_label: Some("Closed event".to_string()),
                destination_surface: PublicDestinationSurface::About,
                destination_id: None,
                attribution: None,
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        update_entry_point(
            &db_path,
            &entry_point.id,
            EntryPointWriteRequest {
                slug: "closed-event".to_string(),
                label: "Closed event".to_string(),
                status: Some(EntryPointStatus::Archived),
                source_kind: "qr".to_string(),
                source_label: Some("Closed event".to_string()),
                destination_surface: PublicDestinationSurface::About,
                destination_id: None,
                attribution: None,
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let result = create_visitor_session(
            &db_path,
            VisitorSessionCreateRequest {
                entry_point_slug: "closed-event".to_string(),
                session_id: None,
                user_agent: None,
                attribution: None,
            },
        );

        assert!(result.unwrap_err().to_string().contains("not active"));
    }

    fn seed_public_about(db_path: &Path) {
        create_business_fact(
            db_path,
            BusinessFactWriteRequest {
                subject_type: None,
                subject_id: None,
                fact_key: "about.tagline".to_string(),
                value: json!("Public story"),
                source_kind: None,
                source_label: None,
                source_uri: None,
                provenance: None,
                visibility: Some(BusinessFactVisibility::Public),
                publication_state: Some(PublicationState::Published),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
    }
}
