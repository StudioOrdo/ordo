use anyhow::{bail, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

use crate::business::{BusinessFactVisibility, PublicationState};
use crate::events::{append_realtime_event_tx, system_event, RealtimeEvent};
use crate::public_surfaces::{public_surfaces, PublicSurfaceItem};

const DEFAULT_TRIAL_DAYS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfferStatus {
    Draft,
    Available,
    Paused,
    Archived,
}

impl OfferStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Available => "available",
            Self::Paused => "paused",
            Self::Archived => "archived",
        }
    }
}

impl TryFrom<&str> for OfferStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "draft" => Ok(Self::Draft),
            "available" => Ok(Self::Available),
            "paused" => Ok(Self::Paused),
            "archived" => Ok(Self::Archived),
            _ => bail!("Unsupported offer status: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceStatus {
    Accepted,
}

impl AcceptanceStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
        }
    }
}

impl TryFrom<&str> for AcceptanceStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "accepted" => Ok(Self::Accepted),
            _ => bail!("Unsupported acceptance status: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrialStatus {
    Started,
    Converted,
    Voided,
    Expired,
    FollowUpNeeded,
}

impl TrialStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Converted => "converted",
            Self::Voided => "voided",
            Self::Expired => "expired",
            Self::FollowUpNeeded => "follow_up_needed",
        }
    }
}

impl TryFrom<&str> for TrialStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "started" => Ok(Self::Started),
            "converted" => Ok(Self::Converted),
            "voided" => Ok(Self::Voided),
            "expired" => Ok(Self::Expired),
            "follow_up_needed" => Ok(Self::FollowUpNeeded),
            _ => bail!("Unsupported trial status: {value}"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferListResponse {
    pub offers: Vec<OfferView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicOfferListResponse {
    pub offers: Vec<PublicOfferView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferAcceptanceListResponse {
    pub acceptances: Vec<OfferAcceptanceView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferAcceptanceResponse {
    pub acceptance: OfferAcceptanceView,
    pub trial: TrialView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialListResponse {
    pub trials: Vec<TrialView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferView {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub status: OfferStatus,
    pub visibility: BusinessFactVisibility,
    pub publication_state: PublicationState,
    pub trial_days: i64,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub terms: Value,
    pub metadata: Value,
    pub created_by_actor_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicOfferView {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub trial_days: i64,
    pub source_kind: String,
    pub source_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferAcceptanceView {
    pub id: String,
    pub offer_id: String,
    pub offer_slug: String,
    pub offer_title: String,
    pub visitor_session_id: Option<String>,
    pub entry_point_id: Option<String>,
    pub entry_point_slug: Option<String>,
    pub attribution: Value,
    pub acceptance_context: Value,
    pub status: AcceptanceStatus,
    pub accepted_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialView {
    pub id: String,
    pub acceptance_id: String,
    pub offer_id: String,
    pub offer_slug: String,
    pub visitor_session_id: Option<String>,
    pub status: TrialStatus,
    pub started_at: String,
    pub trial_ends_at: String,
    pub converted_at: Option<String>,
    pub voided_at: Option<String>,
    pub expired_at: Option<String>,
    pub follow_up_needed_at: Option<String>,
    pub decision_evidence: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferWriteRequest {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub status: Option<OfferStatus>,
    pub visibility: Option<BusinessFactVisibility>,
    pub publication_state: Option<PublicationState>,
    pub trial_days: Option<i64>,
    pub source_kind: Option<String>,
    pub source_ref: Option<String>,
    pub terms: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferAcceptanceCreateRequest {
    pub visitor_session_id: Option<String>,
    pub attribution: Option<Value>,
    pub acceptance_context: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialTransitionRequest {
    pub status: TrialStatus,
    pub decision_evidence: Option<Value>,
}

#[derive(Debug, Clone)]
struct OfferRecord {
    id: String,
    slug: String,
    title: String,
    summary: String,
    status: OfferStatus,
    visibility: BusinessFactVisibility,
    publication_state: PublicationState,
    trial_days: i64,
    source_kind: String,
    source_ref: Option<String>,
    terms: Value,
    metadata: Value,
    created_by_actor_id: Option<String>,
    created_at: String,
    updated_at: String,
    published_at: Option<String>,
    archived_at: Option<String>,
}

#[derive(Debug, Clone)]
struct PublicOfferRecord {
    id: String,
    slug: String,
    title: String,
    summary: String,
    trial_days: i64,
    source_kind: String,
    source_ref: Option<String>,
}

#[derive(Debug, Clone)]
struct OfferAcceptanceRecord {
    id: String,
    offer_id: String,
    offer_slug: String,
    offer_title: String,
    visitor_session_id: Option<String>,
    entry_point_id: Option<String>,
    entry_point_slug: Option<String>,
    attribution: Value,
    acceptance_context: Value,
    status: AcceptanceStatus,
    accepted_at: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct TrialRecord {
    id: String,
    acceptance_id: String,
    offer_id: String,
    offer_slug: String,
    visitor_session_id: Option<String>,
    status: TrialStatus,
    started_at: String,
    trial_ends_at: String,
    converted_at: Option<String>,
    voided_at: Option<String>,
    expired_at: Option<String>,
    follow_up_needed_at: Option<String>,
    decision_evidence: Value,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct VisitorSessionContext {
    id: String,
    entry_point_id: String,
    entry_point_slug: String,
    attribution: Value,
}

pub fn list_offers(db_path: &Path) -> Result<OfferListResponse> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, slug, title, summary, status, visibility, publication_state, trial_days,
                source_kind, source_ref, terms_json, metadata_json, created_by_actor_id,
                created_at, updated_at, published_at, archived_at
         FROM offers
         ORDER BY updated_at DESC, id DESC",
    )?;
    let offers = statement
        .query_map([], offer_from_row)?
        .map(|row| row.map(OfferRecord::into_view))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(OfferListResponse { offers })
}

pub fn list_public_available_offers(db_path: &Path) -> Result<PublicOfferListResponse> {
    let mut offers = explicit_public_offers(db_path)?;
    offers.extend(surface_public_offers(db_path)?);
    offers.sort_by(|left, right| left.slug.cmp(&right.slug));
    offers.dedup_by(|left, right| left.slug == right.slug);
    Ok(PublicOfferListResponse {
        offers: offers
            .into_iter()
            .map(PublicOfferRecord::into_view)
            .collect(),
    })
}

pub fn list_offer_acceptances(db_path: &Path) -> Result<OfferAcceptanceListResponse> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, offer_id, offer_slug, offer_title, visitor_session_id, entry_point_id,
                entry_point_slug, attribution_json, acceptance_context_json, status,
                accepted_at, created_at, updated_at
         FROM offer_acceptances
         ORDER BY accepted_at DESC, id DESC",
    )?;
    let acceptances = statement
        .query_map([], acceptance_from_row)?
        .map(|row| row.map(OfferAcceptanceRecord::into_view))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(OfferAcceptanceListResponse { acceptances })
}

pub fn list_trials(db_path: &Path) -> Result<TrialListResponse> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, acceptance_id, offer_id, offer_slug, visitor_session_id, status,
                started_at, trial_ends_at, converted_at, voided_at, expired_at,
                follow_up_needed_at, decision_evidence_json, created_at, updated_at
         FROM trials
         ORDER BY updated_at DESC, id DESC",
    )?;
    let trials = statement
        .query_map([], trial_from_row)?
        .map(|row| row.map(TrialRecord::into_view))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(TrialListResponse { trials })
}

pub fn create_offer(
    db_path: &Path,
    request: OfferWriteRequest,
    actor_id: Option<&str>,
) -> Result<(OfferView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let id = format!("offer_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();
    let slug = require_identifier(&request.slug, "Offer slug")?;
    let title = require_text(&request.title, "Offer title")?;
    let summary = require_text(&request.summary, "Offer summary")?;
    let status = request.status.unwrap_or(OfferStatus::Draft);
    let visibility = request.visibility.unwrap_or(BusinessFactVisibility::Owner);
    let publication_state = request.publication_state.unwrap_or(PublicationState::Draft);
    let trial_days = normalize_trial_days(request.trial_days)?;
    let published_at = (publication_state == PublicationState::Published).then(|| now.clone());
    let archived_at = (status == OfferStatus::Archived
        || matches!(
            publication_state,
            PublicationState::Archived | PublicationState::Revoked
        ))
    .then(|| now.clone());

    transaction.execute(
        "INSERT INTO offers (
            id, slug, title, summary, status, visibility, publication_state, trial_days,
            source_kind, source_ref, terms_json, metadata_json, created_by_actor_id,
            created_at, updated_at, published_at, archived_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14, ?15, ?16)",
        params![
            id,
            slug,
            title,
            summary,
            status.as_str(),
            visibility.as_str(),
            publication_state.as_str(),
            trial_days,
            normalize_optional_string(request.source_kind)
                .unwrap_or_else(|| "operator".to_string()),
            normalize_optional_string(request.source_ref),
            request.terms.unwrap_or_else(|| json!({})).to_string(),
            request.metadata.unwrap_or_else(|| json!({})).to_string(),
            actor_id,
            now,
            published_at,
            archived_at,
        ],
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "offer.created",
            json!({
                "offerId": id,
                "offerSlug": slug,
                "status": status.as_str(),
                "visibility": visibility.as_str(),
                "publicationState": publication_state.as_str(),
            }),
        ),
    )?;
    transaction.commit()?;
    let record = find_offer_by_id(&connection, &id)?.expect("offer just inserted");
    Ok((record.into_view(), event))
}

pub fn update_offer(
    db_path: &Path,
    offer_id: &str,
    request: OfferWriteRequest,
    actor_id: Option<&str>,
) -> Result<(OfferView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let existing = find_offer_by_id(&transaction, offer_id)?
        .ok_or_else(|| anyhow::anyhow!("Offer was not found: {offer_id}"))?;
    let now = Utc::now().to_rfc3339();
    let slug = require_identifier(&request.slug, "Offer slug")?;
    let title = require_text(&request.title, "Offer title")?;
    let summary = require_text(&request.summary, "Offer summary")?;
    let status = request.status.unwrap_or(existing.status);
    let visibility = request.visibility.unwrap_or(existing.visibility);
    let publication_state = request
        .publication_state
        .unwrap_or(existing.publication_state);
    let trial_days = normalize_trial_days(request.trial_days.or(Some(existing.trial_days)))?;
    let published_at = if publication_state == PublicationState::Published
        && existing.publication_state != PublicationState::Published
    {
        Some(now.clone())
    } else {
        existing.published_at
    };
    let archived_at = if status == OfferStatus::Archived
        || matches!(
            publication_state,
            PublicationState::Archived | PublicationState::Revoked
        ) {
        existing.archived_at.or_else(|| Some(now.clone()))
    } else {
        None
    };

    transaction.execute(
        "UPDATE offers
         SET slug = ?1,
             title = ?2,
             summary = ?3,
             status = ?4,
             visibility = ?5,
             publication_state = ?6,
             trial_days = ?7,
             source_kind = ?8,
             source_ref = ?9,
             terms_json = ?10,
             metadata_json = ?11,
             created_by_actor_id = COALESCE(created_by_actor_id, ?12),
             updated_at = ?13,
             published_at = ?14,
             archived_at = ?15
         WHERE id = ?16",
        params![
            slug,
            title,
            summary,
            status.as_str(),
            visibility.as_str(),
            publication_state.as_str(),
            trial_days,
            normalize_optional_string(request.source_kind).unwrap_or(existing.source_kind),
            normalize_optional_string(request.source_ref).or(existing.source_ref),
            request.terms.unwrap_or(existing.terms).to_string(),
            request.metadata.unwrap_or(existing.metadata).to_string(),
            actor_id,
            now,
            published_at,
            archived_at,
            offer_id,
        ],
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "offer.updated",
            json!({
                "offerId": offer_id,
                "offerSlug": slug,
                "status": status.as_str(),
                "visibility": visibility.as_str(),
                "publicationState": publication_state.as_str(),
            }),
        ),
    )?;
    transaction.commit()?;
    let record = find_offer_by_id(&connection, offer_id)?.expect("offer just updated");
    Ok((record.into_view(), event))
}

pub fn accept_public_offer(
    db_path: &Path,
    offer_slug: &str,
    request: OfferAcceptanceCreateRequest,
) -> Result<(OfferAcceptanceView, TrialView, RealtimeEvent)> {
    let offer = find_public_offer(db_path, offer_slug)?;
    let mut connection = Connection::open(db_path)?;
    let session_context = request
        .visitor_session_id
        .as_deref()
        .map(|session_id| find_visitor_session_context(&connection, session_id))
        .transpose()?;
    let transaction = connection.transaction()?;
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let trial_ends_at = (now + Duration::days(offer.trial_days)).to_rfc3339();
    let acceptance_id = format!("offer_acceptance_{}", Uuid::new_v4());
    let trial_id = format!("trial_{}", Uuid::new_v4());
    let attribution = merge_attribution(
        session_context
            .as_ref()
            .map(|session| session.attribution.clone())
            .unwrap_or_else(|| json!({})),
        request.attribution,
    );
    let acceptance_context = request.acceptance_context.unwrap_or_else(|| json!({}));
    let visitor_session_id = session_context.as_ref().map(|session| session.id.clone());
    let entry_point_id = session_context
        .as_ref()
        .map(|session| session.entry_point_id.clone());
    let entry_point_slug = session_context
        .as_ref()
        .map(|session| session.entry_point_slug.clone());

    transaction.execute(
        "INSERT INTO offer_acceptances (
            id, offer_id, offer_slug, offer_title, visitor_session_id, entry_point_id,
            entry_point_slug, attribution_json, acceptance_context_json, status,
            accepted_at, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, ?11)",
        params![
            acceptance_id,
            offer.id,
            offer.slug,
            offer.title,
            visitor_session_id,
            entry_point_id,
            entry_point_slug,
            attribution.to_string(),
            acceptance_context.to_string(),
            AcceptanceStatus::Accepted.as_str(),
            now_text,
        ],
    )?;
    transaction.execute(
        "INSERT INTO trials (
            id, acceptance_id, offer_id, offer_slug, visitor_session_id, status, started_at,
            trial_ends_at, converted_at, voided_at, expired_at, follow_up_needed_at,
            decision_evidence_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, NULL, NULL, '{}', ?7, ?7)",
        params![
            trial_id,
            acceptance_id,
            offer.id,
            offer.slug,
            visitor_session_id,
            TrialStatus::Started.as_str(),
            now_text,
            trial_ends_at,
        ],
    )?;
    append_trial_event_tx(
        &transaction,
        &trial_id,
        &acceptance_id,
        "trial.started",
        json!({
            "trialId": trial_id,
            "acceptanceId": acceptance_id,
            "offerId": offer.id,
            "offerSlug": offer.slug,
            "visitorSessionId": visitor_session_id,
        }),
        &now_text,
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "offer.accepted",
            json!({
                "acceptanceId": acceptance_id,
                "trialId": trial_id,
                "offerId": offer.id,
                "offerSlug": offer.slug,
                "visitorSessionId": visitor_session_id,
            }),
        ),
    )?;
    transaction.commit()?;
    let acceptance =
        find_acceptance_by_id(&connection, &acceptance_id)?.expect("acceptance inserted");
    let trial = find_trial_by_id(&connection, &trial_id)?.expect("trial inserted");
    Ok((acceptance.into_view(), trial.into_view(), event))
}

pub fn transition_trial(
    db_path: &Path,
    trial_id: &str,
    request: TrialTransitionRequest,
) -> Result<(TrialView, RealtimeEvent)> {
    if request.status == TrialStatus::Started {
        bail!("Trial transitions must move away from started.");
    }
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let existing = find_trial_by_id(&transaction, trial_id)?
        .ok_or_else(|| anyhow::anyhow!("Trial was not found: {trial_id}"))?;
    let now = Utc::now().to_rfc3339();
    let decision_evidence = request
        .decision_evidence
        .unwrap_or(existing.decision_evidence.clone());
    let converted_at = timestamp_for_transition(
        request.status,
        TrialStatus::Converted,
        &now,
        existing.converted_at,
    );
    let voided_at = timestamp_for_transition(
        request.status,
        TrialStatus::Voided,
        &now,
        existing.voided_at,
    );
    let expired_at = timestamp_for_transition(
        request.status,
        TrialStatus::Expired,
        &now,
        existing.expired_at,
    );
    let follow_up_needed_at = timestamp_for_transition(
        request.status,
        TrialStatus::FollowUpNeeded,
        &now,
        existing.follow_up_needed_at,
    );

    transaction.execute(
        "UPDATE trials
         SET status = ?1,
             converted_at = ?2,
             voided_at = ?3,
             expired_at = ?4,
             follow_up_needed_at = ?5,
             decision_evidence_json = ?6,
             updated_at = ?7
         WHERE id = ?8",
        params![
            request.status.as_str(),
            converted_at,
            voided_at,
            expired_at,
            follow_up_needed_at,
            decision_evidence.to_string(),
            now,
            trial_id,
        ],
    )?;
    append_trial_event_tx(
        &transaction,
        trial_id,
        &existing.acceptance_id,
        &format!("trial.{}", request.status.as_str()),
        json!({
            "trialId": trial_id,
            "acceptanceId": existing.acceptance_id,
            "offerId": existing.offer_id,
            "offerSlug": existing.offer_slug,
            "status": request.status.as_str(),
            "decisionEvidence": decision_evidence,
        }),
        &now,
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            &format!("trial.{}", request.status.as_str()),
            json!({
                "trialId": trial_id,
                "acceptanceId": existing.acceptance_id,
                "offerId": existing.offer_id,
                "offerSlug": existing.offer_slug,
                "status": request.status.as_str(),
            }),
        ),
    )?;
    transaction.commit()?;
    let trial = find_trial_by_id(&connection, trial_id)?.expect("trial just updated");
    Ok((trial.into_view(), event))
}

fn explicit_public_offers(db_path: &Path) -> Result<Vec<PublicOfferRecord>> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, slug, title, summary, trial_days, source_kind, source_ref
         FROM offers
         WHERE status = 'available' AND visibility = 'public' AND publication_state = 'published'
         ORDER BY updated_at DESC, id DESC",
    )?;
    let offers = statement
        .query_map([], public_offer_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(offers)
}

fn surface_public_offers(db_path: &Path) -> Result<Vec<PublicOfferRecord>> {
    Ok(public_surfaces(db_path)?
        .offers
        .items
        .iter()
        .filter(|item| item.readiness.ready)
        .map(public_offer_from_surface_item)
        .collect())
}

fn find_public_offer(db_path: &Path, offer_slug: &str) -> Result<PublicOfferRecord> {
    let offer_slug = require_identifier(offer_slug, "Offer slug")?;
    if let Some(record) = find_explicit_public_offer(db_path, &offer_slug)? {
        return Ok(record);
    }
    surface_public_offers(db_path)?
        .into_iter()
        .find(|offer| offer.slug == offer_slug)
        .ok_or_else(|| anyhow::anyhow!("Offer is not publicly available: {offer_slug}"))
}

fn find_explicit_public_offer(
    db_path: &Path,
    offer_slug: &str,
) -> Result<Option<PublicOfferRecord>> {
    let connection = Connection::open(db_path)?;
    Ok(connection
        .query_row(
            "SELECT id, slug, title, summary, trial_days, source_kind, source_ref
             FROM offers
             WHERE slug = ?1 AND status = 'available' AND visibility = 'public' AND publication_state = 'published'",
            [offer_slug],
            public_offer_from_row,
        )
        .optional()?)
}

fn find_offer_by_id(
    connection: &Connection,
    offer_id: &str,
) -> rusqlite::Result<Option<OfferRecord>> {
    connection
        .query_row(
            "SELECT id, slug, title, summary, status, visibility, publication_state, trial_days,
                    source_kind, source_ref, terms_json, metadata_json, created_by_actor_id,
                    created_at, updated_at, published_at, archived_at
             FROM offers
             WHERE id = ?1",
            [offer_id],
            offer_from_row,
        )
        .optional()
}

fn find_acceptance_by_id(
    connection: &Connection,
    acceptance_id: &str,
) -> rusqlite::Result<Option<OfferAcceptanceRecord>> {
    connection
        .query_row(
            "SELECT id, offer_id, offer_slug, offer_title, visitor_session_id, entry_point_id,
                    entry_point_slug, attribution_json, acceptance_context_json, status,
                    accepted_at, created_at, updated_at
             FROM offer_acceptances
             WHERE id = ?1",
            [acceptance_id],
            acceptance_from_row,
        )
        .optional()
}

fn find_trial_by_id(
    connection: &Connection,
    trial_id: &str,
) -> rusqlite::Result<Option<TrialRecord>> {
    connection
        .query_row(
            "SELECT id, acceptance_id, offer_id, offer_slug, visitor_session_id, status,
                    started_at, trial_ends_at, converted_at, voided_at, expired_at,
                    follow_up_needed_at, decision_evidence_json, created_at, updated_at
             FROM trials
             WHERE id = ?1",
            [trial_id],
            trial_from_row,
        )
        .optional()
}

fn find_visitor_session_context(
    connection: &Connection,
    session_id: &str,
) -> Result<VisitorSessionContext> {
    let session_id = require_identifier(session_id, "Visitor session id")?;
    let context = connection
        .query_row(
            "SELECT id, entry_point_id, entry_point_slug, attribution_json
             FROM visitor_sessions
             WHERE id = ?1",
            [&session_id],
            |row| {
                let attribution_json: String = row.get(3)?;
                Ok(VisitorSessionContext {
                    id: row.get(0)?,
                    entry_point_id: row.get(1)?,
                    entry_point_slug: row.get(2)?,
                    attribution: serde_json::from_str(&attribution_json)
                        .unwrap_or_else(|_| json!({})),
                })
            },
        )
        .optional()?;
    context.ok_or_else(|| anyhow::anyhow!("Visitor session was not found: {session_id}"))
}

fn offer_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<OfferRecord> {
    let status: String = row.get(4)?;
    let visibility: String = row.get(5)?;
    let publication_state: String = row.get(6)?;
    let terms_json: String = row.get(10)?;
    let metadata_json: String = row.get(11)?;
    Ok(OfferRecord {
        id: row.get(0)?,
        slug: row.get(1)?,
        title: row.get(2)?,
        summary: row.get(3)?,
        status: OfferStatus::try_from(status.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, error.into())
        })?,
        visibility: BusinessFactVisibility::try_from(visibility.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, error.into())
        })?,
        publication_state: PublicationState::try_from(publication_state.as_str()).map_err(
            |error| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    error.into(),
                )
            },
        )?,
        trial_days: row.get(7)?,
        source_kind: row.get(8)?,
        source_ref: row.get(9)?,
        terms: serde_json::from_str(&terms_json).unwrap_or_else(|_| json!({})),
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        created_by_actor_id: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        published_at: row.get(15)?,
        archived_at: row.get(16)?,
    })
}

fn public_offer_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PublicOfferRecord> {
    Ok(PublicOfferRecord {
        id: row.get(0)?,
        slug: row.get(1)?,
        title: row.get(2)?,
        summary: row.get(3)?,
        trial_days: row.get(4)?,
        source_kind: row.get(5)?,
        source_ref: row.get(6)?,
    })
}

fn acceptance_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<OfferAcceptanceRecord> {
    let attribution_json: String = row.get(7)?;
    let context_json: String = row.get(8)?;
    let status: String = row.get(9)?;
    Ok(OfferAcceptanceRecord {
        id: row.get(0)?,
        offer_id: row.get(1)?,
        offer_slug: row.get(2)?,
        offer_title: row.get(3)?,
        visitor_session_id: row.get(4)?,
        entry_point_id: row.get(5)?,
        entry_point_slug: row.get(6)?,
        attribution: serde_json::from_str(&attribution_json).unwrap_or_else(|_| json!({})),
        acceptance_context: serde_json::from_str(&context_json).unwrap_or_else(|_| json!({})),
        status: AcceptanceStatus::try_from(status.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, error.into())
        })?,
        accepted_at: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn trial_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TrialRecord> {
    let status: String = row.get(5)?;
    let decision_evidence_json: String = row.get(12)?;
    Ok(TrialRecord {
        id: row.get(0)?,
        acceptance_id: row.get(1)?,
        offer_id: row.get(2)?,
        offer_slug: row.get(3)?,
        visitor_session_id: row.get(4)?,
        status: TrialStatus::try_from(status.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, error.into())
        })?,
        started_at: row.get(6)?,
        trial_ends_at: row.get(7)?,
        converted_at: row.get(8)?,
        voided_at: row.get(9)?,
        expired_at: row.get(10)?,
        follow_up_needed_at: row.get(11)?,
        decision_evidence: serde_json::from_str(&decision_evidence_json)
            .unwrap_or_else(|_| json!({})),
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn public_offer_from_surface_item(item: &PublicSurfaceItem) -> PublicOfferRecord {
    PublicOfferRecord {
        id: format!("public_surface_offer_{}", item.item_id),
        slug: item.item_id.clone(),
        title: surface_field_text(item, "title").unwrap_or_else(|| item.item_id.clone()),
        summary: surface_field_text(item, "summary").unwrap_or_default(),
        trial_days: DEFAULT_TRIAL_DAYS,
        source_kind: "public_surface".to_string(),
        source_ref: Some(item.item_id.clone()),
    }
}

fn surface_field_text(item: &PublicSurfaceItem, key: &str) -> Option<String> {
    item.fields
        .iter()
        .find(|field| field.key == key)
        .and_then(|field| field.value.as_str().map(str::to_string))
}

fn append_trial_event_tx(
    transaction: &rusqlite::Transaction<'_>,
    trial_id: &str,
    acceptance_id: &str,
    event_type: &str,
    payload: Value,
    occurred_at: &str,
) -> Result<()> {
    transaction.execute(
        "INSERT INTO trial_events (id, trial_id, acceptance_id, event_type, payload_json, occurred_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            format!("trial_event_{}", Uuid::new_v4()),
            trial_id,
            acceptance_id,
            event_type,
            payload.to_string(),
            occurred_at,
        ],
    )?;
    Ok(())
}

impl OfferRecord {
    fn into_view(self) -> OfferView {
        OfferView {
            id: self.id,
            slug: self.slug,
            title: self.title,
            summary: self.summary,
            status: self.status,
            visibility: self.visibility,
            publication_state: self.publication_state,
            trial_days: self.trial_days,
            source_kind: self.source_kind,
            source_ref: self.source_ref,
            terms: self.terms,
            metadata: self.metadata,
            created_by_actor_id: self.created_by_actor_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            published_at: self.published_at,
            archived_at: self.archived_at,
        }
    }
}

impl PublicOfferRecord {
    fn into_view(self) -> PublicOfferView {
        PublicOfferView {
            id: self.id,
            slug: self.slug,
            title: self.title,
            summary: self.summary,
            trial_days: self.trial_days,
            source_kind: self.source_kind,
            source_ref: self.source_ref,
        }
    }
}

impl OfferAcceptanceRecord {
    fn into_view(self) -> OfferAcceptanceView {
        OfferAcceptanceView {
            id: self.id,
            offer_id: self.offer_id,
            offer_slug: self.offer_slug,
            offer_title: self.offer_title,
            visitor_session_id: self.visitor_session_id,
            entry_point_id: self.entry_point_id,
            entry_point_slug: self.entry_point_slug,
            attribution: self.attribution,
            acceptance_context: self.acceptance_context,
            status: self.status,
            accepted_at: self.accepted_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl TrialRecord {
    fn into_view(self) -> TrialView {
        TrialView {
            id: self.id,
            acceptance_id: self.acceptance_id,
            offer_id: self.offer_id,
            offer_slug: self.offer_slug,
            visitor_session_id: self.visitor_session_id,
            status: self.status,
            started_at: self.started_at,
            trial_ends_at: self.trial_ends_at,
            converted_at: self.converted_at,
            voided_at: self.voided_at,
            expired_at: self.expired_at,
            follow_up_needed_at: self.follow_up_needed_at,
            decision_evidence: self.decision_evidence,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn timestamp_for_transition(
    actual: TrialStatus,
    target: TrialStatus,
    now: &str,
    existing: Option<String>,
) -> Option<String> {
    if actual == target {
        Some(now.to_string())
    } else {
        existing
    }
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

fn normalize_trial_days(value: Option<i64>) -> Result<i64> {
    let value = value.unwrap_or(DEFAULT_TRIAL_DAYS);
    if !(1..=365).contains(&value) {
        bail!("Trial days must be between 1 and 365.");
    }
    Ok(value)
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
    use crate::business::{create_business_fact, BusinessFactWriteRequest};
    use crate::entry_points::{
        create_entry_point, create_visitor_session, EntryPointWriteRequest,
        PublicDestinationSurface, VisitorSessionCreateRequest,
    };
    use crate::policy::LOCAL_OWNER_ACTOR_ID;
    use crate::schema::init_database;
    use tempfile::TempDir;

    #[test]
    fn public_offer_acceptance_creates_trial_and_events() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (offer, _) = create_offer(
            &db_path,
            public_offer_request("trial-pack"),
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let (acceptance, trial, event) = accept_public_offer(
            &db_path,
            &offer.slug,
            OfferAcceptanceCreateRequest {
                visitor_session_id: None,
                attribution: Some(json!({ "source": "direct" })),
                acceptance_context: Some(json!({ "note": "ready" })),
            },
        )
        .unwrap();

        assert_eq!(acceptance.offer_slug, "trial-pack");
        assert_eq!(acceptance.attribution["source"], "direct");
        assert_eq!(trial.status, TrialStatus::Started);
        assert_eq!(trial.offer_slug, "trial-pack");
        assert_eq!(event.event_type, "offer.accepted");
        let connection = Connection::open(&db_path).unwrap();
        let trial_events: i64 = connection
            .query_row("SELECT COUNT(*) FROM trial_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(trial_events, 1);
    }

    #[test]
    fn public_offer_acceptance_rejects_private_or_unpublished_offers() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        create_offer(
            &db_path,
            OfferWriteRequest {
                slug: "private-pack".to_string(),
                title: "Private Pack".to_string(),
                summary: "Not public".to_string(),
                status: Some(OfferStatus::Available),
                visibility: Some(BusinessFactVisibility::Owner),
                publication_state: Some(PublicationState::Published),
                trial_days: Some(30),
                source_kind: None,
                source_ref: None,
                terms: None,
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let result = accept_public_offer(
            &db_path,
            "private-pack",
            OfferAcceptanceCreateRequest {
                visitor_session_id: None,
                attribution: None,
                acceptance_context: None,
            },
        );

        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not publicly available"));
    }

    #[test]
    fn acceptance_preserves_visitor_session_attribution() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        seed_public_about(&db_path);
        create_entry_point(
            &db_path,
            EntryPointWriteRequest {
                slug: "partner-link".to_string(),
                label: "Partner Link".to_string(),
                status: None,
                source_kind: "affiliate".to_string(),
                source_label: Some("Partner".to_string()),
                destination_surface: PublicDestinationSurface::About,
                destination_id: None,
                attribution: Some(json!({ "campaign": "partner" })),
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        let (session, _) = create_visitor_session(
            &db_path,
            VisitorSessionCreateRequest {
                entry_point_slug: "partner-link".to_string(),
                user_agent: Some("Sensitive Browser".to_string()),
                attribution: Some(json!({ "medium": "qr" })),
            },
        )
        .unwrap();
        create_offer(
            &db_path,
            public_offer_request("session-pack"),
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let (acceptance, _, _) = accept_public_offer(
            &db_path,
            "session-pack",
            OfferAcceptanceCreateRequest {
                visitor_session_id: Some(session.id.clone()),
                attribution: Some(json!({ "intent": "trial" })),
                acceptance_context: None,
            },
        )
        .unwrap();

        assert_eq!(acceptance.visitor_session_id, Some(session.id));
        assert_eq!(
            acceptance.entry_point_slug,
            Some("partner-link".to_string())
        );
        assert_eq!(acceptance.attribution["campaign"], "partner");
        assert_eq!(acceptance.attribution["medium"], "qr");
        assert_eq!(acceptance.attribution["intent"], "trial");
    }

    #[test]
    fn trial_lifecycle_transitions_record_evidence_and_event() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        create_offer(
            &db_path,
            public_offer_request("conversion-pack"),
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        let (_, trial, _) = accept_public_offer(
            &db_path,
            "conversion-pack",
            OfferAcceptanceCreateRequest {
                visitor_session_id: None,
                attribution: None,
                acceptance_context: None,
            },
        )
        .unwrap();

        let (converted, event) = transition_trial(
            &db_path,
            &trial.id,
            TrialTransitionRequest {
                status: TrialStatus::Converted,
                decision_evidence: Some(json!({ "reason": "paid" })),
            },
        )
        .unwrap();

        assert_eq!(converted.status, TrialStatus::Converted);
        assert!(converted.converted_at.is_some());
        assert_eq!(converted.decision_evidence["reason"], "paid");
        assert_eq!(event.event_type, "trial.converted");
    }

    #[test]
    fn public_surface_offer_can_be_accepted_when_fact_is_published_public() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        create_business_fact(
            &db_path,
            BusinessFactWriteRequest {
                subject_type: None,
                subject_id: None,
                fact_key: "offers.bootstrap.title".to_string(),
                value: json!("Bootstrap Pack"),
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

        let (acceptance, trial, _) = accept_public_offer(
            &db_path,
            "bootstrap",
            OfferAcceptanceCreateRequest {
                visitor_session_id: None,
                attribution: None,
                acceptance_context: None,
            },
        )
        .unwrap();

        assert_eq!(acceptance.offer_id, "public_surface_offer_bootstrap");
        assert_eq!(trial.offer_id, "public_surface_offer_bootstrap");
    }

    fn public_offer_request(slug: &str) -> OfferWriteRequest {
        OfferWriteRequest {
            slug: slug.to_string(),
            title: "30 Day Trial".to_string(),
            summary: "Try Ordo for 30 days.".to_string(),
            status: Some(OfferStatus::Available),
            visibility: Some(BusinessFactVisibility::Public),
            publication_state: Some(PublicationState::Published),
            trial_days: Some(30),
            source_kind: None,
            source_ref: None,
            terms: Some(json!({ "trialDays": 30 })),
            metadata: None,
        }
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
