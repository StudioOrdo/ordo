use anyhow::{bail, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

use crate::attribution::{record_offer_acceptance_outcome_tx, OfferAcceptanceOutcomeInput};
use crate::business::{BusinessFactVisibility, PublicationState};
use crate::events::{append_realtime_event, append_realtime_event_tx, system_event, RealtimeEvent};
use crate::public_surfaces::{public_surfaces, PublicSurfaceItem};
use crate::rewards::reward_program_is_active;

const DEFAULT_TRIAL_DAYS: i64 = 30;
const OFFER_RECEIPT_SCHEMA_VERSION: &str = "ordo.offer_acceptance.receipt.v1";
const HOSTED_TRIAL_RESOURCE_KIND: &str = "hosted_trial";
const HOSTED_TRIAL_ACTION: &str = "use";
const HOSTED_TRIAL_ACTIVE_SLOT_LIMIT: i64 = 10;
const HOSTED_TRIAL_POLICY_STATUS_ACTIVE: &str = "active";
const HOSTED_TRIAL_SLOT_STATUS_ACTIVE: &str = "active";
const HOSTED_TRIAL_BACKUP_STATUS_REQUIRED: &str = "required";
const HOSTED_TRIAL_BACKUP_STATUS_READY: &str = "ready";
const HOSTED_TRIAL_RESET_BLOCKED_UNTIL_EXPIRATION: &str = "blocked_until_expiration";
const HOSTED_TRIAL_RESET_BLOCKED_PENDING_BACKUP: &str = "blocked_pending_backup";
const HOSTED_TRIAL_RESET_READY_FOR_OWNER_REVIEW: &str = "ready_for_owner_review";
const HOSTED_TRIAL_WAITLIST_STATUS_WAITING: &str = "waiting";
const HOSTED_TRIAL_WAITLIST_REASON_CAPACITY_FULL: &str = "capacity_full";
const OFFER_BUILDER_SCHEMA_VERSION: &str = "ordo.offer_builder.v1";

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
    Waitlisted,
}

impl AcceptanceStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Waitlisted => "waitlisted",
        }
    }
}

impl TryFrom<&str> for AcceptanceStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "accepted" => Ok(Self::Accepted),
            "waitlisted" => Ok(Self::Waitlisted),
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
    pub access_grant: AccessGrantView,
    pub receipt: OfferAcceptanceReceipt,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialListResponse {
    pub trials: Vec<TrialView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedTrialCapacityResponse {
    pub policies: Vec<HostedTrialCapacityPolicyView>,
    pub slots: Vec<HostedTrialSlotView>,
    pub waitlist: Vec<HostedTrialWaitlistEntryView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferBuilderResponse {
    pub offers: Vec<OfferBuilderOfferView>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferBuilderSaveResponse {
    pub offer: OfferView,
    pub public_preview: Option<PublicOfferView>,
    pub validation: OfferBuilderValidationView,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferBuilderOfferView {
    pub offer: OfferView,
    pub public_preview: Option<PublicOfferView>,
    pub validation: OfferBuilderValidationView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferBuilderValidationView {
    pub publishable: bool,
    pub state: String,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub supported_references: Vec<OfferBuilderReferenceView>,
    pub deferred_references: Vec<OfferBuilderReferenceView>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferBuilderReferenceView {
    pub key: String,
    pub label: String,
    pub status: String,
    pub detail: String,
    pub evidence_refs: Vec<String>,
    pub blocked_by: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedTrialCapacityPolicyView {
    pub id: String,
    pub offer_id: String,
    pub offer_slug: String,
    pub status: String,
    pub active_slot_limit: i64,
    pub active_slot_count: i64,
    pub waitlist_count: i64,
    pub trial_days: i64,
    pub backup_before_wipe_required: bool,
    pub reset_grace_days: i64,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedTrialSlotView {
    pub id: String,
    pub policy_id: String,
    pub trial_id: String,
    pub acceptance_id: String,
    pub offer_id: String,
    pub offer_slug: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub status: String,
    pub allocated_at: String,
    pub expires_at: String,
    pub released_at: Option<String>,
    pub release_reason: Option<String>,
    pub backup_required: bool,
    pub backup_status: String,
    pub backup_evidence_refs: Vec<String>,
    pub reset_eligible_at: Option<String>,
    pub reset_state: String,
    pub reset_guard: Value,
    pub owner_override: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedTrialWaitlistEntryView {
    pub id: String,
    pub policy_id: String,
    pub acceptance_id: String,
    pub offer_id: String,
    pub offer_slug: String,
    pub visitor_session_id: Option<String>,
    pub subject_kind: String,
    pub subject_id: String,
    pub status: String,
    pub position: i64,
    pub reason: String,
    pub receipt: Value,
    pub evidence_refs: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
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
    pub terms: Value,
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
    pub idempotency_key: Option<String>,
    pub access_grant_id: Option<String>,
    pub receipt: Value,
    pub status: AcceptanceStatus,
    pub accepted_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessGrantView {
    pub id: String,
    pub resource_kind: String,
    pub resource_id: String,
    pub action: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub effect: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferAcceptanceReceipt {
    pub schema_version: String,
    pub status: String,
    pub offer_slug: String,
    pub trial_id: String,
    pub trial_days: i64,
    pub trial_ends_at: String,
    pub access_grant_id: String,
    #[serde(default)]
    pub terms_snapshot: Value,
    pub expectations: Vec<String>,
    pub support: String,
    pub evidence_refs: Vec<String>,
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
pub struct OfferBuilderSaveRequest {
    pub offer: OfferWriteRequest,
    pub references: Option<OfferBuilderReferencesRequest>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferBuilderReferencesRequest {
    pub tracked_entry_point_slug: Option<String>,
    pub support_handoff_cta: Option<bool>,
    pub reward_program_id: Option<String>,
    pub pack_ids: Option<Vec<String>>,
    pub external_publishing: Option<bool>,
    pub payment: Option<bool>,
    pub oauth: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferAcceptanceCreateRequest {
    pub visitor_session_id: Option<String>,
    pub local_session_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub attribution: Option<Value>,
    pub acceptance_context: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialTransitionRequest {
    pub status: TrialStatus,
    pub decision_evidence: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedTrialResetRequest {
    pub backup_evidence_refs: Vec<String>,
    pub owner_decision: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedTrialResetPlanView {
    pub trial_id: String,
    pub slot_id: String,
    pub backup_status: String,
    pub backup_evidence_refs: Vec<String>,
    pub reset_state: String,
    pub reset_eligible_at: Option<String>,
    pub owner_override: Value,
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
    terms: Value,
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
    idempotency_key: Option<String>,
    access_grant_id: Option<String>,
    receipt: Value,
    status: AcceptanceStatus,
    accepted_at: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct AccessGrantRecord {
    id: String,
    resource_kind: String,
    resource_id: String,
    action: String,
    subject_kind: String,
    subject_id: String,
    effect: String,
    created_at: String,
    expires_at: Option<String>,
    metadata: Value,
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

#[derive(Debug, Clone)]
struct LocalSessionContext {
    session_id: String,
    actor_id: String,
}

#[derive(Debug, Clone)]
struct AccessSubject {
    subject_kind: String,
    subject_id: String,
    local_session_id: Option<String>,
}

#[derive(Debug, Clone)]
struct HostedTrialCapacityPolicyRecord {
    id: String,
    offer_id: String,
    offer_slug: String,
    status: String,
    active_slot_limit: i64,
    active_slot_count: i64,
    waitlist_count: i64,
    trial_days: i64,
    backup_before_wipe_required: bool,
    reset_grace_days: i64,
    metadata: Value,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct HostedTrialSlotRecord {
    id: String,
    policy_id: String,
    trial_id: String,
    acceptance_id: String,
    offer_id: String,
    offer_slug: String,
    subject_kind: String,
    subject_id: String,
    status: String,
    allocated_at: String,
    expires_at: String,
    released_at: Option<String>,
    release_reason: Option<String>,
    backup_required: bool,
    backup_status: String,
    backup_evidence_refs: Vec<String>,
    reset_eligible_at: Option<String>,
    reset_state: String,
    reset_guard: Value,
    owner_override: Value,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct HostedTrialWaitlistEntryRecord {
    id: String,
    policy_id: String,
    acceptance_id: String,
    offer_id: String,
    offer_slug: String,
    visitor_session_id: Option<String>,
    subject_kind: String,
    subject_id: String,
    status: String,
    position: i64,
    reason: String,
    receipt: Value,
    evidence_refs: Vec<String>,
    created_at: String,
    updated_at: String,
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
                entry_point_slug, attribution_json, acceptance_context_json, idempotency_key,
                access_grant_id, receipt_json, status,
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

pub fn list_hosted_trial_capacity(db_path: &Path) -> Result<HostedTrialCapacityResponse> {
    let connection = Connection::open(db_path)?;
    let now = Utc::now().to_rfc3339();
    let mut policies_statement = connection.prepare(
        "SELECT p.id, p.offer_id, p.offer_slug, p.status, p.active_slot_limit,
                (SELECT COUNT(*)
                 FROM hosted_trial_slots s
                 WHERE s.policy_id = p.id AND s.status = 'active' AND s.expires_at > ?1),
                (SELECT COUNT(*)
                 FROM hosted_trial_waitlist_entries w
                 WHERE w.policy_id = p.id AND w.status = 'waiting'),
                p.trial_days, p.backup_before_wipe_required, p.reset_grace_days,
                p.metadata_json, p.created_at, p.updated_at
         FROM hosted_trial_capacity_policies p
         ORDER BY p.updated_at DESC, p.id DESC",
    )?;
    let policies = policies_statement
        .query_map([now.as_str()], hosted_trial_policy_from_row)?
        .map(|row| row.map(HostedTrialCapacityPolicyRecord::into_view))
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut slots_statement = connection.prepare(
        "SELECT id, policy_id, trial_id, acceptance_id, offer_id, offer_slug,
                subject_kind, subject_id, status, allocated_at, expires_at, released_at,
                release_reason, backup_required, backup_status, backup_evidence_json,
                reset_eligible_at, reset_state, reset_guard_json, owner_override_json,
                created_at, updated_at
         FROM hosted_trial_slots
         ORDER BY allocated_at ASC, id ASC",
    )?;
    let slots = slots_statement
        .query_map([], hosted_trial_slot_from_row)?
        .map(|row| row.map(HostedTrialSlotRecord::into_view))
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut waitlist_statement = connection.prepare(
        "SELECT id, policy_id, acceptance_id, offer_id, offer_slug, visitor_session_id,
                subject_kind, subject_id, status, position, reason, receipt_json,
                evidence_refs_json, created_at, updated_at
         FROM hosted_trial_waitlist_entries
         ORDER BY position ASC, created_at ASC",
    )?;
    let waitlist = waitlist_statement
        .query_map([], hosted_trial_waitlist_from_row)?
        .map(|row| row.map(HostedTrialWaitlistEntryRecord::into_view))
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(HostedTrialCapacityResponse {
        policies,
        slots,
        waitlist,
    })
}

pub fn inspect_offer_builder(db_path: &Path) -> Result<OfferBuilderResponse> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, slug, title, summary, status, visibility, publication_state, trial_days,
                source_kind, source_ref, terms_json, metadata_json, created_by_actor_id,
                created_at, updated_at, published_at, archived_at
         FROM offers
         ORDER BY updated_at DESC, id DESC",
    )?;
    let records = statement
        .query_map([], offer_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let offers = records
        .into_iter()
        .map(|record| offer_builder_view(&connection, record))
        .collect::<Result<Vec<_>>>()?;
    Ok(OfferBuilderResponse {
        offers,
        generated_at: Utc::now().to_rfc3339(),
    })
}

pub fn save_offer_builder_offer(
    db_path: &Path,
    request: OfferBuilderSaveRequest,
    actor_id: Option<&str>,
) -> Result<(OfferBuilderSaveResponse, RealtimeEvent)> {
    let connection = Connection::open(db_path)?;
    let mut offer = normalize_offer_builder_request(request.offer, request.references.as_ref())?;
    let slug = require_identifier(&offer.slug, "Offer slug")?;
    let existing = find_offer_by_slug(&connection, &slug)?;
    let validation =
        validate_offer_builder_write(&connection, &offer, request.references.as_ref())?;
    if is_public_publication_request(&offer) && !validation.publishable {
        bail!(
            "Offer Builder cannot publish unsupported pilot offer: {}",
            validation.blockers.join("; ")
        );
    }
    merge_offer_builder_metadata(&mut offer, request.references.as_ref(), &validation)?;

    let (saved, _) = if let Some(existing) = existing {
        update_offer(db_path, &existing.id, offer, actor_id)?
    } else {
        create_offer(db_path, offer, actor_id)?
    };

    let connection = Connection::open(db_path)?;
    let event = append_realtime_event(
        &connection,
        &system_event(
            "offer_builder.saved",
            json!({
                "offerId": saved.id,
                "offerSlug": saved.slug,
                "publicationState": saved.publication_state.as_str(),
                "status": saved.status.as_str(),
            }),
        ),
    )?;
    let record = find_offer_by_id(&connection, &saved.id)?.expect("offer just saved");
    let builder_view = offer_builder_view(&connection, record)?;
    Ok((
        OfferBuilderSaveResponse {
            offer: saved,
            public_preview: builder_view.public_preview,
            validation: builder_view.validation,
            generated_at: Utc::now().to_rfc3339(),
        },
        event,
    ))
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
    let terms = normalize_offer_terms(request.terms, trial_days);
    let metadata = request.metadata.unwrap_or_else(|| json!({}));
    validate_offer_publication_fields(
        &transaction,
        publication_state,
        visibility,
        status,
        trial_days,
        &terms,
        &metadata,
    )?;
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
            terms.to_string(),
            metadata.to_string(),
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
    let terms = normalize_offer_terms(request.terms.or(Some(existing.terms.clone())), trial_days);
    let metadata = request.metadata.unwrap_or(existing.metadata.clone());
    validate_offer_publication_fields(
        &transaction,
        publication_state,
        visibility,
        status,
        trial_days,
        &terms,
        &metadata,
    )?;
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
            terms.to_string(),
            metadata.to_string(),
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
) -> Result<(
    OfferAcceptanceView,
    TrialView,
    AccessGrantView,
    OfferAcceptanceReceipt,
    RealtimeEvent,
)> {
    let offer = find_public_offer(db_path, offer_slug)?;
    let mut connection = Connection::open(db_path)?;
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let session_context = request
        .visitor_session_id
        .as_deref()
        .map(|session_id| find_visitor_session_context(&connection, session_id))
        .transpose()?;
    let local_session_context = request
        .local_session_id
        .as_deref()
        .map(|session_id| find_local_session_context(&connection, session_id, &now_text))
        .transpose()?;
    let idempotency_key = acceptance_idempotency_key(
        request.idempotency_key.as_deref(),
        local_session_context.as_ref(),
        session_context.as_ref(),
    )?;
    if let Some(key) = idempotency_key.as_deref() {
        if let Some(existing) =
            find_acceptance_by_offer_and_idempotency(&connection, &offer.id, key)?
        {
            if existing.status == AcceptanceStatus::Waitlisted {
                let waitlist = find_waitlist_by_acceptance_id(&connection, &existing.id)?
                    .ok_or_else(|| {
                        anyhow::anyhow!("Waitlisted offer is missing waitlist state.")
                    })?;
                bail!(
                    "Hosted trial capacity is full; acceptance is already waitlisted at position {}.",
                    waitlist.position
                );
            }
            let trial = find_trial_by_acceptance_id(&connection, &existing.id)?
                .ok_or_else(|| anyhow::anyhow!("Accepted offer is missing trial state."))?;
            let access_grant_id = existing
                .access_grant_id
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Accepted offer is missing Access grant state."))?;
            let access_grant = find_access_grant_by_id(&connection, access_grant_id)?
                .ok_or_else(|| anyhow::anyhow!("Accepted offer Access grant was not found."))?;
            let receipt = receipt_from_value(existing.receipt.clone())?;
            let replay_event = system_event(
                "offer.acceptance.replayed",
                json!({
                    "acceptanceId": existing.id,
                    "trialId": trial.id,
                    "accessGrantId": access_grant.id,
                    "offerId": offer.id,
                    "offerSlug": offer.slug,
                }),
            );
            return Ok((
                existing.into_view(),
                trial.into_view(),
                access_grant.into_view(),
                receipt,
                replay_event,
            ));
        }
    }
    let transaction = connection.transaction()?;
    let trial_ends_at = (now + Duration::days(offer.trial_days)).to_rfc3339();
    let acceptance_id = format!("offer_acceptance_{}", Uuid::new_v4());
    let trial_id = format!("trial_{}", Uuid::new_v4());
    let access_grant_id = format!("resource_grant_{}", Uuid::new_v4());
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
    let subject = access_subject_for_acceptance(
        local_session_context.as_ref(),
        visitor_session_id.as_deref(),
        &acceptance_id,
    );
    ensure_access_subject_actor_tx(
        &transaction,
        &subject,
        &acceptance_id,
        visitor_session_id.as_deref(),
        &now_text,
    )?;
    let capacity_policy = ensure_hosted_trial_capacity_policy_tx(&transaction, &offer, &now_text)?;
    let active_slot_count =
        active_hosted_trial_slot_count_tx(&transaction, &capacity_policy.id, &now_text)?;
    if active_slot_count >= capacity_policy.active_slot_limit {
        let waitlist_id = format!("hosted_trial_waitlist_{}", Uuid::new_v4());
        let position = next_waitlist_position_tx(&transaction, &capacity_policy.id)?;
        let receipt_json = hosted_trial_waitlist_receipt(
            &offer,
            &acceptance_id,
            &waitlist_id,
            position,
            capacity_policy.active_slot_limit,
        );
        transaction.execute(
            "INSERT INTO offer_acceptances (
                id, offer_id, offer_slug, offer_title, visitor_session_id, entry_point_id,
                entry_point_slug, attribution_json, acceptance_context_json, idempotency_key,
                access_grant_id, receipt_json, status,
                accepted_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, ?11, ?12, ?13, ?13, ?13)",
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
                idempotency_key.as_deref(),
                receipt_json.to_string(),
                AcceptanceStatus::Waitlisted.as_str(),
                now_text,
            ],
        )?;
        transaction.execute(
            "INSERT INTO hosted_trial_waitlist_entries (
                id, policy_id, acceptance_id, offer_id, offer_slug, visitor_session_id,
                subject_kind, subject_id, status, position, reason, receipt_json,
                evidence_refs_json, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
            params![
                waitlist_id,
                capacity_policy.id,
                acceptance_id,
                offer.id,
                offer.slug,
                visitor_session_id,
                subject.subject_kind,
                subject.subject_id,
                HOSTED_TRIAL_WAITLIST_STATUS_WAITING,
                position,
                HOSTED_TRIAL_WAITLIST_REASON_CAPACITY_FULL,
                receipt_json.to_string(),
                json!([
                    format!("offer:{}", offer.id),
                    format!("offer_acceptance:{acceptance_id}"),
                    format!("hosted_trial_waitlist:{waitlist_id}")
                ])
                .to_string(),
                now_text,
            ],
        )?;
        append_realtime_event_tx(
            &transaction,
            &system_event(
                "offer.waitlisted",
                json!({
                    "acceptanceId": acceptance_id,
                    "waitlistEntryId": waitlist_id,
                    "offerId": offer.id,
                    "offerSlug": offer.slug,
                    "position": position,
                    "reason": HOSTED_TRIAL_WAITLIST_REASON_CAPACITY_FULL,
                }),
            ),
        )?;
        transaction.commit()?;
        bail!(
            "Hosted trial capacity is full; the acceptance was added to the waitlist at position {position}."
        );
    }
    let slot_id = format!("hosted_trial_slot_{}", Uuid::new_v4());
    let receipt = offer_acceptance_receipt(
        &offer,
        &trial_id,
        offer.trial_days,
        &trial_ends_at,
        &access_grant_id,
        &acceptance_id,
        Some(&slot_id),
    );
    let receipt_json = serde_json::to_value(&receipt)?;

    transaction.execute(
        "INSERT INTO offer_acceptances (
            id, offer_id, offer_slug, offer_title, visitor_session_id, entry_point_id,
            entry_point_slug, attribution_json, acceptance_context_json, idempotency_key,
            access_grant_id, receipt_json, status,
            accepted_at, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14, ?14)",
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
            idempotency_key.as_deref(),
            access_grant_id,
            receipt_json.to_string(),
            AcceptanceStatus::Accepted.as_str(),
            now_text,
        ],
    )?;
    transaction.execute(
        "INSERT INTO trials (
            id, acceptance_id, offer_id, offer_slug, visitor_session_id, status, started_at,
            trial_ends_at, converted_at, voided_at, expired_at, follow_up_needed_at,
            decision_evidence_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, NULL, NULL, ?9, ?7, ?7)",
        params![
            trial_id,
            acceptance_id,
            offer.id,
            offer.slug,
            visitor_session_id,
            TrialStatus::Started.as_str(),
            now_text,
            trial_ends_at,
            json!({
                "accessGrantId": access_grant_id,
                "hostedTrialSlotId": slot_id,
                "capacityPolicyId": capacity_policy.id,
                "grantKind": "accepted_offer",
                "hostedTrial": {
                    "experimental": true,
                    "backupBeforeWipeRequired": true,
                    "backupStatus": HOSTED_TRIAL_BACKUP_STATUS_REQUIRED,
                    "resetState": HOSTED_TRIAL_RESET_BLOCKED_UNTIL_EXPIRATION
                }
            })
            .to_string(),
        ],
    )?;
    transaction.execute(
        "INSERT INTO hosted_trial_slots (
            id, policy_id, trial_id, acceptance_id, offer_id, offer_slug, subject_kind,
            subject_id, status, allocated_at, expires_at, released_at, release_reason,
            backup_required, backup_status, backup_evidence_json, reset_eligible_at,
            reset_state, reset_guard_json, owner_override_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, NULL, 1, ?12, '[]', ?11, ?13, ?14, '{}', ?10, ?10)",
        params![
            slot_id,
            capacity_policy.id,
            trial_id,
            acceptance_id,
            offer.id,
            offer.slug,
            subject.subject_kind,
            subject.subject_id,
            HOSTED_TRIAL_SLOT_STATUS_ACTIVE,
            now_text,
            trial_ends_at,
            HOSTED_TRIAL_BACKUP_STATUS_REQUIRED,
            HOSTED_TRIAL_RESET_BLOCKED_UNTIL_EXPIRATION,
            json!({
                "backupBeforeWipeRequired": true,
                "destructiveWipeAllowed": false,
                "reason": "trial_active",
                "requires": ["trial_expired_or_voided", "backup_export_evidence", "owner_review"]
            })
            .to_string(),
        ],
    )?;
    transaction.execute(
        "INSERT INTO resource_grants (
            id, resource_kind, resource_id, action, subject_kind, subject_id, effect, created_at,
            expires_at, metadata_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'allow', ?7, ?8, ?9)",
        params![
            access_grant_id,
            HOSTED_TRIAL_RESOURCE_KIND,
            trial_id,
            HOSTED_TRIAL_ACTION,
            subject.subject_kind,
            subject.subject_id,
            now_text,
            trial_ends_at,
            json!({
                "grantKind": "accepted_offer",
                "offerId": offer.id,
                "offerSlug": offer.slug,
                "acceptanceId": acceptance_id,
                "trialId": trial_id,
                "hostedTrialSlotId": slot_id,
                "capacityPolicyId": capacity_policy.id,
                "visitorSessionId": visitor_session_id,
                "entryPointId": entry_point_id,
                "entryPointSlug": entry_point_slug,
                "localSessionId": subject.local_session_id,
                "receipt": receipt_json,
                "experimentalHosting": true,
                "backupBeforeWipeRequired": true,
                "backupStatus": HOSTED_TRIAL_BACKUP_STATUS_REQUIRED,
                "resetState": HOSTED_TRIAL_RESET_BLOCKED_UNTIL_EXPIRATION,
            })
            .to_string(),
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
            "accessGrantId": access_grant_id,
            "hostedTrialSlotId": slot_id,
            "capacityPolicyId": capacity_policy.id,
            "offerId": offer.id,
            "offerSlug": offer.slug,
            "visitorSessionId": visitor_session_id,
        }),
        &now_text,
    )?;
    record_offer_acceptance_outcome_tx(
        &transaction,
        OfferAcceptanceOutcomeInput {
            acceptance_id: &acceptance_id,
            trial_id: &trial_id,
            offer_id: &offer.id,
            offer_slug: &offer.slug,
            visitor_session_id: visitor_session_id.as_deref(),
            entry_point_id: entry_point_id.as_deref(),
            occurred_at: &now_text,
        },
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "offer.accepted",
            json!({
                "acceptanceId": acceptance_id,
                "trialId": trial_id,
                "accessGrantId": access_grant_id,
                "hostedTrialSlotId": slot_id,
                "capacityPolicyId": capacity_policy.id,
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
    let access_grant =
        find_access_grant_by_id(&connection, &access_grant_id)?.expect("grant inserted");
    Ok((
        acceptance.into_view(),
        trial.into_view(),
        access_grant.into_view(),
        receipt,
        event,
    ))
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
        existing.converted_at.clone(),
    );
    let voided_at = timestamp_for_transition(
        request.status,
        TrialStatus::Voided,
        &now,
        existing.voided_at.clone(),
    );
    let expired_at = timestamp_for_transition(
        request.status,
        TrialStatus::Expired,
        &now,
        existing.expired_at.clone(),
    );
    let follow_up_needed_at = timestamp_for_transition(
        request.status,
        TrialStatus::FollowUpNeeded,
        &now,
        existing.follow_up_needed_at.clone(),
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
    update_hosted_trial_slot_for_transition_tx(
        &transaction,
        &existing,
        request.status,
        &decision_evidence,
        &now,
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

pub fn request_hosted_trial_reset(
    db_path: &Path,
    trial_id: &str,
    request: HostedTrialResetRequest,
) -> Result<(HostedTrialResetPlanView, RealtimeEvent)> {
    let trial_id = require_identifier(trial_id, "Trial id")?;
    let evidence_refs = normalize_evidence_refs(request.backup_evidence_refs)?;
    if evidence_refs.is_empty() {
        bail!("Hosted trial reset/wipe requires backup/export evidence.");
    }
    let owner_decision = request.owner_decision.ok_or_else(|| {
        anyhow::anyhow!("Hosted trial reset/wipe requires owner decision evidence.")
    })?;
    let owner_decision_object = owner_decision.as_object().ok_or_else(|| {
        anyhow::anyhow!("Hosted trial reset/wipe owner decision evidence must be an object.")
    })?;
    if owner_decision_object.is_empty() {
        bail!("Hosted trial reset/wipe requires owner decision evidence.");
    }
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let trial = find_trial_by_id(&transaction, &trial_id)?
        .ok_or_else(|| anyhow::anyhow!("Trial was not found: {trial_id}"))?;
    let slot = find_hosted_trial_slot_by_trial_id(&transaction, &trial_id)?
        .ok_or_else(|| anyhow::anyhow!("Hosted trial slot was not found for trial: {trial_id}"))?;
    match trial.status {
        TrialStatus::Expired | TrialStatus::Voided => {}
        TrialStatus::Converted => {
            bail!(
                "Converted hosted trials are retained and cannot be marked ready for reset/wipe through the hosted trial expiration guard."
            );
        }
        TrialStatus::Started | TrialStatus::FollowUpNeeded => {
            bail!("Hosted trial reset/wipe is blocked until the trial is expired or voided.");
        }
    }
    if slot.status == HOSTED_TRIAL_SLOT_STATUS_ACTIVE {
        bail!("Hosted trial reset/wipe is blocked until the trial is expired or voided.");
    }
    if slot.status == TrialStatus::Converted.as_str() || slot.reset_state == "converted_no_wipe" {
        bail!(
            "Converted hosted trials are retained and cannot be marked ready for reset/wipe through the hosted trial expiration guard."
        );
    }
    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "UPDATE hosted_trial_slots
         SET backup_status = ?1,
             backup_evidence_json = ?2,
             reset_state = ?3,
             reset_guard_json = ?4,
             owner_override_json = ?5,
             updated_at = ?6
         WHERE trial_id = ?7",
        params![
            HOSTED_TRIAL_BACKUP_STATUS_READY,
            serde_json::to_string(&evidence_refs)?,
            HOSTED_TRIAL_RESET_READY_FOR_OWNER_REVIEW,
            json!({
                "backupBeforeWipeRequired": true,
                "destructiveWipeAllowed": false,
                "reason": "backup_ready_owner_review_required",
                "requires": ["explicit_destructive_action"]
            })
            .to_string(),
            owner_decision.to_string(),
            now,
            trial_id,
        ],
    )?;
    append_trial_event_tx(
        &transaction,
        &trial_id,
        &trial.acceptance_id,
        "trial.reset.ready",
        json!({
            "trialId": trial_id,
            "acceptanceId": trial.acceptance_id,
            "slotId": slot.id,
            "backupEvidenceRefs": evidence_refs,
            "resetState": HOSTED_TRIAL_RESET_READY_FOR_OWNER_REVIEW,
            "ownerDecision": owner_decision,
        }),
        &now,
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "trial.reset.ready",
            json!({
                "trialId": trial_id,
                "acceptanceId": trial.acceptance_id,
                "slotId": slot.id,
                "resetState": HOSTED_TRIAL_RESET_READY_FOR_OWNER_REVIEW,
            }),
        ),
    )?;
    transaction.commit()?;
    let slot = find_hosted_trial_slot_by_trial_id(&connection, &trial_id)?
        .expect("hosted trial slot just updated");
    Ok((slot.into_reset_plan(), event))
}

fn offer_builder_view(
    connection: &Connection,
    record: OfferRecord,
) -> Result<OfferBuilderOfferView> {
    let validation = validate_offer_builder_record(connection, &record)?;
    let public_preview = if record.status == OfferStatus::Available
        && record.visibility == BusinessFactVisibility::Public
        && record.publication_state == PublicationState::Published
    {
        Some(PublicOfferRecord::from_offer_record(&record).into_view())
    } else {
        None
    };
    Ok(OfferBuilderOfferView {
        offer: record.into_view(),
        public_preview,
        validation,
    })
}

fn validate_offer_builder_record(
    connection: &Connection,
    record: &OfferRecord,
) -> Result<OfferBuilderValidationView> {
    validate_offer_builder_fields(
        connection,
        Some(&record.id),
        &record.slug,
        record.status,
        record.visibility,
        record.publication_state,
        record.trial_days,
        &record.terms,
        &record.metadata,
        offer_builder_references_from_metadata(&record.metadata),
    )
}

fn validate_offer_builder_write(
    connection: &Connection,
    request: &OfferWriteRequest,
    references: Option<&OfferBuilderReferencesRequest>,
) -> Result<OfferBuilderValidationView> {
    let slug = require_identifier(&request.slug, "Offer slug")?;
    let status = request.status.unwrap_or(OfferStatus::Draft);
    let visibility = request.visibility.unwrap_or(BusinessFactVisibility::Owner);
    let publication_state = request.publication_state.unwrap_or(PublicationState::Draft);
    let trial_days = normalize_trial_days(request.trial_days)?;
    let terms = normalize_offer_terms(request.terms.clone(), trial_days);
    let metadata = request.metadata.clone().unwrap_or_else(|| json!({}));
    validate_offer_builder_fields(
        connection,
        None,
        &slug,
        status,
        visibility,
        publication_state,
        trial_days,
        &terms,
        &metadata,
        references.cloned(),
    )
}

#[allow(clippy::too_many_arguments)]
fn validate_offer_builder_fields(
    connection: &Connection,
    offer_id: Option<&str>,
    slug: &str,
    status: OfferStatus,
    visibility: BusinessFactVisibility,
    publication_state: PublicationState,
    trial_days: i64,
    terms: &Value,
    metadata: &Value,
    references: Option<OfferBuilderReferencesRequest>,
) -> Result<OfferBuilderValidationView> {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut supported_references = Vec::new();
    let mut deferred_references = Vec::new();
    let mut evidence_refs = vec![format!("offer_slug:{slug}")];
    if let Some(offer_id) = offer_id {
        evidence_refs.push(format!("offer:{offer_id}"));
    }

    if trial_days != DEFAULT_TRIAL_DAYS {
        blockers.push("The NYC pilot offer must publish as a 30-day hosted trial.".to_string());
    }
    if !terms
        .get("experimentalHosting")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        blockers.push("Published pilot terms must disclose experimental hosting.".to_string());
    }
    if !terms
        .get("backupBeforeWipeRequired")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        blockers.push(
            "Published pilot terms must require backup/export before reset or wipe.".to_string(),
        );
    }
    if contains_internal_or_secret_key(terms) || contains_internal_or_secret_key(metadata) {
        blockers.push(
            "Public offers cannot include internal or secret-bearing keys in terms or metadata."
                .to_string(),
        );
    }

    supported_references.push(reference(
        "access_grant",
        "Accepted-offer Access grant",
        "available",
        "Public acceptance creates a hosted_trial/use resource grant backed by accepted offer evidence.",
        vec!["resource_grants".to_string(), "offer_acceptances".to_string()],
        None,
    ));
    supported_references.push(reference(
        "hosted_trial_capacity",
        "Hosted trial capacity",
        "available",
        "Hosted trial acceptance allocates at most 10 active slots and records waitlist state.",
        vec![
            "hosted_trial_capacity_policies".to_string(),
            "hosted_trial_slots".to_string(),
        ],
        None,
    ));

    let references = references.unwrap_or_default();
    if let Some(entry_slug) = references.tracked_entry_point_slug.as_deref() {
        match find_active_entry_point(connection, entry_slug)? {
            Some(entry_id) => {
                supported_references.push(reference(
                    "tracked_entry_point",
                    "Tracked QR entry point",
                    "available",
                    "The referenced active tracked entry point can carry visitor/session attribution into acceptance.",
                    vec![format!("tracked_entry_point:{entry_id}")],
                    None,
                ));
            }
            None => blockers.push(format!(
                "Tracked entry point is not active or does not exist: {entry_slug}"
            )),
        }
    } else {
        warnings.push(
            "No tracked entry point is attached; QR/session attribution will not be offer-specific."
                .to_string(),
        );
    }

    if references.support_handoff_cta.unwrap_or(false) {
        supported_references.push(reference(
            "support_handoff_cta",
            "Support handoff CTA",
            "available",
            "Support handoff CTA is backed by the local handoff inbox foundation and remains policy-gated.",
            vec!["handoff_inbox_items".to_string()],
            None,
        ));
    } else {
        warnings.push("Support handoff CTA is not attached to this offer.".to_string());
    }

    if let Some(reward_program_id) = references.reward_program_id.as_deref() {
        if reward_program_is_active(connection, reward_program_id)? {
            supported_references.push(reference(
                "reward_ledger",
                "Feedback/referral rewards",
                "available",
                "Reward program, ledger, hosted-time benefit grants, balances, and qualification review are available for evidence-backed referral and feedback rewards.",
                vec![format!("reward_program:{reward_program_id}")],
                None,
            ));
        } else {
            blockers.push(format!(
                "Reward program is not active or does not exist: {reward_program_id}"
            ));
        }
    } else if has_active_reward_reference(terms) || has_active_reward_reference(metadata) {
        blockers.push(
            "Active reward references must name an active reward program; unsupported reward ids cannot be published."
                .to_string(),
        );
    } else {
        warnings.push("No reward program is attached to this offer.".to_string());
    }

    if references
        .pack_ids
        .as_ref()
        .map(|pack_ids| !pack_ids.is_empty())
        .unwrap_or(false)
        || has_active_pack_reference(terms)
        || has_active_pack_reference(metadata)
    {
        blockers.push(
            "Product/workforce pack offer bindings are not available yet; do not save pack claims as active offer behavior."
                .to_string(),
        );
    }
    deferred_references.push(reference(
        "product_workforce_packs",
        "Product/workforce packs",
        "not_available_yet",
        "MCP packs exist as governed capability metadata, but durable offer-pack binding is not implemented.",
        vec!["mcp_packs".to_string()],
        Some("offer_pack_binding".to_string()),
    ));

    if references.external_publishing.unwrap_or(false)
        || references.payment.unwrap_or(false)
        || references.oauth.unwrap_or(false)
    {
        blockers.push(
            "External publishing, payment processing, and OAuth are outside the Offer Builder baseline."
                .to_string(),
        );
    }
    deferred_references.push(reference(
        "external_platforms",
        "External publishing/payments/OAuth",
        "out_of_scope",
        "This slice is deterministic and network-free; live platform integrations are not part of the baseline.",
        vec![],
        Some("future_guarded_adapters".to_string()),
    ));

    let wants_publication = status == OfferStatus::Available
        || visibility == BusinessFactVisibility::Public
        || publication_state == PublicationState::Published;
    let publishable = blockers.is_empty()
        && status == OfferStatus::Available
        && visibility == BusinessFactVisibility::Public
        && publication_state == PublicationState::Published;
    let state = if publishable {
        "ready"
    } else if wants_publication && !blockers.is_empty() {
        "blocked"
    } else {
        "draft"
    }
    .to_string();

    Ok(OfferBuilderValidationView {
        publishable,
        state,
        blockers,
        warnings,
        supported_references,
        deferred_references,
        evidence_refs,
    })
}

fn validate_offer_publication_fields(
    connection: &Connection,
    publication_state: PublicationState,
    visibility: BusinessFactVisibility,
    status: OfferStatus,
    trial_days: i64,
    terms: &Value,
    metadata: &Value,
) -> Result<()> {
    if status != OfferStatus::Available
        && visibility != BusinessFactVisibility::Public
        && publication_state != PublicationState::Published
    {
        return Ok(());
    }
    let validation = validate_offer_builder_fields(
        connection,
        None,
        "pending",
        status,
        visibility,
        publication_state,
        trial_days,
        terms,
        metadata,
        offer_builder_references_from_metadata(metadata),
    )?;
    if !validation.blockers.is_empty() {
        bail!("{}", validation.blockers.join("; "));
    }
    Ok(())
}

fn normalize_offer_builder_request(
    mut offer: OfferWriteRequest,
    references: Option<&OfferBuilderReferencesRequest>,
) -> Result<OfferWriteRequest> {
    let trial_days = normalize_trial_days(offer.trial_days.or(Some(DEFAULT_TRIAL_DAYS)))?;
    offer.trial_days = Some(trial_days);
    offer.source_kind = offer
        .source_kind
        .or_else(|| Some("offer_builder".to_string()));
    offer.terms = Some(normalize_offer_terms(offer.terms, trial_days));
    let mut metadata = offer.metadata.unwrap_or_else(|| json!({}));
    if !metadata.is_object() {
        bail!("Offer Builder metadata must be a JSON object.");
    }
    if let Some(references) = references {
        metadata["offerBuilder"] = json!({
            "schemaVersion": OFFER_BUILDER_SCHEMA_VERSION,
            "trackedEntryPointSlug": references.tracked_entry_point_slug.as_deref(),
            "supportHandoffCta": references.support_handoff_cta.unwrap_or(false),
            "accessGrant": {
                "status": "available",
                "resourceKind": HOSTED_TRIAL_RESOURCE_KIND,
                "action": HOSTED_TRIAL_ACTION,
                "source": "accepted_offer"
            },
            "capacity": {
                "status": "available",
                "activeSlotLimit": HOSTED_TRIAL_ACTIVE_SLOT_LIMIT,
                "waitlist": true
            },
            "rewards": {
                "status": if references.reward_program_id.is_some() { "available" } else { "not_configured" },
                "rewardProgramId": references.reward_program_id.as_deref()
            },
            "packs": {
                "status": "not_available_yet",
                "blockedBy": "offer_pack_binding"
            },
            "externalPublishing": {
                "status": "out_of_scope"
            }
        });
    }
    offer.metadata = Some(metadata);
    Ok(offer)
}

fn normalize_offer_terms(terms: Option<Value>, trial_days: i64) -> Value {
    let mut terms = terms.unwrap_or_else(|| json!({}));
    if !terms.is_object() {
        terms = json!({});
    }
    if terms.get("trialDays").is_none() {
        terms["trialDays"] = json!(trial_days);
    }
    if terms.get("experimentalHosting").is_none() {
        terms["experimentalHosting"] = json!(true);
    }
    if terms.get("backupBeforeWipeRequired").is_none() {
        terms["backupBeforeWipeRequired"] = json!(true);
    }
    if terms.get("humanReviewRequired").is_none() {
        terms["humanReviewRequired"] = json!(true);
    }
    if terms.get("rewards").is_none() {
        terms["rewards"] = json!({
            "status": "not_configured"
        });
    }
    if terms.get("packs").is_none() {
        terms["packs"] = json!({
            "status": "not_available_yet",
            "blockedBy": "offer_pack_binding"
        });
    }
    terms
}

fn merge_offer_builder_metadata(
    offer: &mut OfferWriteRequest,
    references: Option<&OfferBuilderReferencesRequest>,
    validation: &OfferBuilderValidationView,
) -> Result<()> {
    let mut metadata = offer.metadata.take().unwrap_or_else(|| json!({}));
    if !metadata.is_object() {
        bail!("Offer Builder metadata must be a JSON object.");
    }
    metadata["offerBuilderValidation"] = json!({
        "schemaVersion": OFFER_BUILDER_SCHEMA_VERSION,
        "state": validation.state,
        "publishable": validation.publishable,
        "blockers": validation.blockers,
        "warnings": validation.warnings,
        "supportedReferences": validation.supported_references,
        "deferredReferences": validation.deferred_references,
    });
    if let Some(references) = references {
        metadata["offerBuilderReferences"] = serde_json::to_value(references)?;
    }
    offer.metadata = Some(metadata);
    Ok(())
}

fn offer_builder_references_from_metadata(
    metadata: &Value,
) -> Option<OfferBuilderReferencesRequest> {
    let builder = metadata.get("offerBuilder")?;
    Some(OfferBuilderReferencesRequest {
        tracked_entry_point_slug: builder
            .get("trackedEntryPointSlug")
            .and_then(Value::as_str)
            .map(str::to_string),
        support_handoff_cta: builder.get("supportHandoffCta").and_then(Value::as_bool),
        reward_program_id: builder
            .get("rewardProgramId")
            .and_then(Value::as_str)
            .or_else(|| {
                builder
                    .get("rewards")
                    .and_then(|rewards| rewards.get("rewardProgramId"))
                    .and_then(Value::as_str)
            })
            .map(str::to_string),
        pack_ids: builder.get("packIds").and_then(|value| {
            value.as_array().map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
        }),
        external_publishing: builder.get("externalPublishing").and_then(|value| {
            value
                .as_bool()
                .or_else(|| value.get("enabled").and_then(Value::as_bool))
        }),
        payment: builder.get("payment").and_then(Value::as_bool),
        oauth: builder.get("oauth").and_then(Value::as_bool),
    })
}

fn reference(
    key: &str,
    label: &str,
    status: &str,
    detail: &str,
    evidence_refs: Vec<String>,
    blocked_by: Option<String>,
) -> OfferBuilderReferenceView {
    OfferBuilderReferenceView {
        key: key.to_string(),
        label: label.to_string(),
        status: status.to_string(),
        detail: detail.to_string(),
        evidence_refs,
        blocked_by,
    }
}

fn is_public_publication_request(request: &OfferWriteRequest) -> bool {
    request.status == Some(OfferStatus::Available)
        || request.visibility == Some(BusinessFactVisibility::Public)
        || request.publication_state == Some(PublicationState::Published)
}

fn find_active_entry_point(connection: &Connection, slug: &str) -> Result<Option<String>> {
    let slug = require_identifier(slug, "Tracked entry point slug")?;
    Ok(connection
        .query_row(
            "SELECT id FROM tracked_entry_points WHERE slug = ?1 AND status = 'active'",
            [slug],
            |row| row.get(0),
        )
        .optional()?)
}

fn contains_internal_or_secret_key(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, value)| {
            is_internal_or_secret_key(key) || contains_internal_or_secret_key(value)
        }),
        Value::Array(items) => items.iter().any(contains_internal_or_secret_key),
        _ => false,
    }
}

fn is_internal_or_secret_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    [
        "secret",
        "token",
        "apikey",
        "api_key",
        "providersecret",
        "provider_secret",
        "rawprompt",
        "raw_prompt",
        "systemprompt",
        "system_prompt",
        "staffnote",
        "staff_note",
        "staffnotes",
        "staff_notes",
        "policyinternal",
        "policy_internal",
        "owneronly",
        "owner_only",
    ]
    .iter()
    .any(|blocked| normalized.contains(blocked))
}

fn has_active_reward_reference(value: &Value) -> bool {
    has_active_reference(
        value,
        &[
            "rewardProgramId",
            "rewardLedgerId",
            "benefitGrantId",
            "hostedTimeExtension",
            "reward_program_id",
        ],
    )
}

fn has_active_pack_reference(value: &Value) -> bool {
    has_active_reference(
        value,
        &[
            "packIds",
            "packId",
            "productPackId",
            "workforcePackId",
            "pack_ids",
        ],
    )
}

fn has_active_reference(value: &Value, keys: &[&str]) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, child)| {
            let key_matches = keys
                .iter()
                .any(|candidate| key.eq_ignore_ascii_case(candidate));
            if key_matches && !is_inactive_reference(child) {
                return true;
            }
            has_active_reference(child, keys)
        }),
        Value::Array(items) => items.iter().any(|item| has_active_reference(item, keys)),
        _ => false,
    }
}

fn is_deferred_reference(value: &Value) -> bool {
    value
        .get("status")
        .and_then(Value::as_str)
        .map(|status| status == "not_available_yet" || status == "deferred")
        .unwrap_or(false)
}

fn is_inactive_reference(value: &Value) -> bool {
    value.is_null()
        || value
            .as_array()
            .map(|items| items.is_empty())
            .unwrap_or(false)
        || is_deferred_reference(value)
}

fn explicit_public_offers(db_path: &Path) -> Result<Vec<PublicOfferRecord>> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, slug, title, summary, trial_days, source_kind, source_ref, terms_json
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
            "SELECT id, slug, title, summary, trial_days, source_kind, source_ref, terms_json
             FROM offers
             WHERE slug = ?1 AND status = 'available' AND visibility = 'public' AND publication_state = 'published'",
            [offer_slug],
            public_offer_from_row,
        )
        .optional()?)
}

fn find_offer_by_slug(
    connection: &Connection,
    offer_slug: &str,
) -> rusqlite::Result<Option<OfferRecord>> {
    connection
        .query_row(
            "SELECT id, slug, title, summary, status, visibility, publication_state, trial_days,
                    source_kind, source_ref, terms_json, metadata_json, created_by_actor_id,
                    created_at, updated_at, published_at, archived_at
             FROM offers
             WHERE slug = ?1",
            [offer_slug],
            offer_from_row,
        )
        .optional()
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
                    entry_point_slug, attribution_json, acceptance_context_json, idempotency_key,
                    access_grant_id, receipt_json, status,
                    accepted_at, created_at, updated_at
             FROM offer_acceptances
             WHERE id = ?1",
            [acceptance_id],
            acceptance_from_row,
        )
        .optional()
}

fn find_acceptance_by_offer_and_idempotency(
    connection: &Connection,
    offer_id: &str,
    idempotency_key: &str,
) -> rusqlite::Result<Option<OfferAcceptanceRecord>> {
    connection
        .query_row(
            "SELECT id, offer_id, offer_slug, offer_title, visitor_session_id, entry_point_id,
                    entry_point_slug, attribution_json, acceptance_context_json, idempotency_key,
                    access_grant_id, receipt_json, status,
                    accepted_at, created_at, updated_at
             FROM offer_acceptances
             WHERE offer_id = ?1 AND idempotency_key = ?2",
            params![offer_id, idempotency_key],
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

fn find_trial_by_acceptance_id(
    connection: &Connection,
    acceptance_id: &str,
) -> rusqlite::Result<Option<TrialRecord>> {
    connection
        .query_row(
            "SELECT id, acceptance_id, offer_id, offer_slug, visitor_session_id, status,
                    started_at, trial_ends_at, converted_at, voided_at, expired_at,
                    follow_up_needed_at, decision_evidence_json, created_at, updated_at
             FROM trials
             WHERE acceptance_id = ?1",
            [acceptance_id],
            trial_from_row,
        )
        .optional()
}

fn find_access_grant_by_id(
    connection: &Connection,
    access_grant_id: &str,
) -> rusqlite::Result<Option<AccessGrantRecord>> {
    connection
        .query_row(
            "SELECT id, resource_kind, resource_id, action, subject_kind, subject_id, effect,
                    created_at, expires_at, metadata_json
             FROM resource_grants
             WHERE id = ?1",
            [access_grant_id],
            access_grant_from_row,
        )
        .optional()
}

fn find_waitlist_by_acceptance_id(
    connection: &Connection,
    acceptance_id: &str,
) -> rusqlite::Result<Option<HostedTrialWaitlistEntryRecord>> {
    connection
        .query_row(
            "SELECT id, policy_id, acceptance_id, offer_id, offer_slug, visitor_session_id,
                    subject_kind, subject_id, status, position, reason, receipt_json,
                    evidence_refs_json, created_at, updated_at
             FROM hosted_trial_waitlist_entries
             WHERE acceptance_id = ?1",
            [acceptance_id],
            hosted_trial_waitlist_from_row,
        )
        .optional()
}

fn find_hosted_trial_slot_by_trial_id(
    connection: &Connection,
    trial_id: &str,
) -> rusqlite::Result<Option<HostedTrialSlotRecord>> {
    connection
        .query_row(
            "SELECT id, policy_id, trial_id, acceptance_id, offer_id, offer_slug,
                    subject_kind, subject_id, status, allocated_at, expires_at, released_at,
                    release_reason, backup_required, backup_status, backup_evidence_json,
                    reset_eligible_at, reset_state, reset_guard_json, owner_override_json,
                    created_at, updated_at
             FROM hosted_trial_slots
             WHERE trial_id = ?1",
            [trial_id],
            hosted_trial_slot_from_row,
        )
        .optional()
}

fn ensure_hosted_trial_capacity_policy_tx(
    transaction: &Transaction<'_>,
    offer: &PublicOfferRecord,
    now: &str,
) -> Result<HostedTrialCapacityPolicyRecord> {
    let id = format!("hosted_trial_capacity_policy_{}", Uuid::new_v4());
    transaction.execute(
        "INSERT INTO hosted_trial_capacity_policies (
            id, offer_id, offer_slug, status, active_slot_limit, trial_days,
            backup_before_wipe_required, reset_grace_days, metadata_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 0, ?7, ?8, ?8)
         ON CONFLICT(offer_id) DO UPDATE SET
             offer_slug = excluded.offer_slug,
             trial_days = excluded.trial_days,
             active_slot_limit = excluded.active_slot_limit,
             backup_before_wipe_required = 1,
             updated_at = excluded.updated_at",
        params![
            id,
            offer.id,
            offer.slug,
            HOSTED_TRIAL_POLICY_STATUS_ACTIVE,
            HOSTED_TRIAL_ACTIVE_SLOT_LIMIT,
            offer.trial_days,
            json!({
                "source": "public_offer_acceptance",
                "experimentalHosting": true,
                "rewardExtensionsSource": "reward_ledger_benefit_grants"
            })
            .to_string(),
            now,
        ],
    )?;
    Ok(
        find_hosted_trial_capacity_policy_by_offer_id(transaction, &offer.id)?
            .expect("hosted trial capacity policy just upserted"),
    )
}

fn find_hosted_trial_capacity_policy_by_offer_id(
    connection: &Connection,
    offer_id: &str,
) -> rusqlite::Result<Option<HostedTrialCapacityPolicyRecord>> {
    let now = Utc::now().to_rfc3339();
    connection
        .query_row(
            "SELECT p.id, p.offer_id, p.offer_slug, p.status, p.active_slot_limit,
                    (SELECT COUNT(*)
                     FROM hosted_trial_slots s
                     WHERE s.policy_id = p.id AND s.status = 'active' AND s.expires_at > ?1),
                    (SELECT COUNT(*)
                     FROM hosted_trial_waitlist_entries w
                     WHERE w.policy_id = p.id AND w.status = 'waiting'),
                    p.trial_days, p.backup_before_wipe_required, p.reset_grace_days,
                    p.metadata_json, p.created_at, p.updated_at
             FROM hosted_trial_capacity_policies p
             WHERE p.offer_id = ?2",
            params![now, offer_id],
            hosted_trial_policy_from_row,
        )
        .optional()
}

fn active_hosted_trial_slot_count_tx(
    transaction: &Transaction<'_>,
    policy_id: &str,
    now: &str,
) -> Result<i64> {
    Ok(transaction.query_row(
        "SELECT COUNT(*)
         FROM hosted_trial_slots
         WHERE policy_id = ?1 AND status = 'active' AND expires_at > ?2",
        params![policy_id, now],
        |row| row.get(0),
    )?)
}

fn next_waitlist_position_tx(transaction: &Transaction<'_>, policy_id: &str) -> Result<i64> {
    let max_position: Option<i64> = transaction.query_row(
        "SELECT MAX(position)
         FROM hosted_trial_waitlist_entries
         WHERE policy_id = ?1",
        [policy_id],
        |row| row.get(0),
    )?;
    Ok(max_position.unwrap_or(0) + 1)
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

fn find_local_session_context(
    connection: &Connection,
    session_id: &str,
    now: &str,
) -> Result<LocalSessionContext> {
    let session_id = require_identifier(session_id, "Local session id")?;
    let context = connection
        .query_row(
            "SELECT session_id, actor_id
             FROM local_account_sessions
             WHERE session_id = ?1 AND expires_at > ?2",
            params![session_id, now],
            |row| {
                Ok(LocalSessionContext {
                    session_id: row.get(0)?,
                    actor_id: row.get(1)?,
                })
            },
        )
        .optional()?;
    context.ok_or_else(|| anyhow::anyhow!("Local session was not found or has expired."))
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
    let terms_json: String = row.get(7)?;
    let trial_days: i64 = row.get(4)?;
    Ok(PublicOfferRecord {
        id: row.get(0)?,
        slug: row.get(1)?,
        title: row.get(2)?,
        summary: row.get(3)?,
        trial_days,
        source_kind: row.get(5)?,
        source_ref: row.get(6)?,
        terms: public_offer_terms(
            serde_json::from_str(&terms_json).unwrap_or_else(|_| json!({})),
            trial_days,
        ),
    })
}

fn acceptance_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<OfferAcceptanceRecord> {
    let attribution_json: String = row.get(7)?;
    let context_json: String = row.get(8)?;
    let receipt_json: String = row.get(11)?;
    let status: String = row.get(12)?;
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
        idempotency_key: row.get(9)?,
        access_grant_id: row.get(10)?,
        receipt: serde_json::from_str(&receipt_json).unwrap_or_else(|_| json!({})),
        status: AcceptanceStatus::try_from(status.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(12, rusqlite::types::Type::Text, error.into())
        })?,
        accepted_at: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn access_grant_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccessGrantRecord> {
    let metadata_json: String = row.get(9)?;
    Ok(AccessGrantRecord {
        id: row.get(0)?,
        resource_kind: row.get(1)?,
        resource_id: row.get(2)?,
        action: row.get(3)?,
        subject_kind: row.get(4)?,
        subject_id: row.get(5)?,
        effect: row.get(6)?,
        created_at: row.get(7)?,
        expires_at: row.get(8)?,
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
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

fn hosted_trial_policy_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<HostedTrialCapacityPolicyRecord> {
    let metadata_json: String = row.get(10)?;
    let backup_required: i64 = row.get(8)?;
    Ok(HostedTrialCapacityPolicyRecord {
        id: row.get(0)?,
        offer_id: row.get(1)?,
        offer_slug: row.get(2)?,
        status: row.get(3)?,
        active_slot_limit: row.get(4)?,
        active_slot_count: row.get(5)?,
        waitlist_count: row.get(6)?,
        trial_days: row.get(7)?,
        backup_before_wipe_required: backup_required != 0,
        reset_grace_days: row.get(9)?,
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn hosted_trial_slot_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HostedTrialSlotRecord> {
    let backup_required: i64 = row.get(13)?;
    let backup_evidence_json: String = row.get(15)?;
    let reset_guard_json: String = row.get(18)?;
    let owner_override_json: String = row.get(19)?;
    Ok(HostedTrialSlotRecord {
        id: row.get(0)?,
        policy_id: row.get(1)?,
        trial_id: row.get(2)?,
        acceptance_id: row.get(3)?,
        offer_id: row.get(4)?,
        offer_slug: row.get(5)?,
        subject_kind: row.get(6)?,
        subject_id: row.get(7)?,
        status: row.get(8)?,
        allocated_at: row.get(9)?,
        expires_at: row.get(10)?,
        released_at: row.get(11)?,
        release_reason: row.get(12)?,
        backup_required: backup_required != 0,
        backup_status: row.get(14)?,
        backup_evidence_refs: serde_json::from_str(&backup_evidence_json)
            .unwrap_or_else(|_| Vec::new()),
        reset_eligible_at: row.get(16)?,
        reset_state: row.get(17)?,
        reset_guard: serde_json::from_str(&reset_guard_json).unwrap_or_else(|_| json!({})),
        owner_override: serde_json::from_str(&owner_override_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(20)?,
        updated_at: row.get(21)?,
    })
}

fn hosted_trial_waitlist_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<HostedTrialWaitlistEntryRecord> {
    let receipt_json: String = row.get(11)?;
    let evidence_refs_json: String = row.get(12)?;
    Ok(HostedTrialWaitlistEntryRecord {
        id: row.get(0)?,
        policy_id: row.get(1)?,
        acceptance_id: row.get(2)?,
        offer_id: row.get(3)?,
        offer_slug: row.get(4)?,
        visitor_session_id: row.get(5)?,
        subject_kind: row.get(6)?,
        subject_id: row.get(7)?,
        status: row.get(8)?,
        position: row.get(9)?,
        reason: row.get(10)?,
        receipt: serde_json::from_str(&receipt_json).unwrap_or_else(|_| json!({})),
        evidence_refs: serde_json::from_str(&evidence_refs_json).unwrap_or_else(|_| Vec::new()),
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
        terms: public_offer_terms(json!({}), DEFAULT_TRIAL_DAYS),
    }
}

fn public_offer_terms(terms: Value, trial_days: i64) -> Value {
    json!({
        "trialDays": terms
            .get("trialDays")
            .and_then(Value::as_i64)
            .unwrap_or(trial_days),
        "termsVersion": terms
            .get("termsVersion")
            .and_then(Value::as_str)
            .unwrap_or("current"),
        "experimentalHosting": terms
            .get("experimentalHosting")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "backupBeforeWipeRequired": terms
            .get("backupBeforeWipeRequired")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "humanReviewRequired": terms
            .get("humanReviewRequired")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "rewards": {
            "status": "not_configured"
        },
        "packs": {
            "status": "not_available_yet",
            "blockedBy": "offer_pack_binding"
        }
    })
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
    fn from_offer_record(record: &OfferRecord) -> Self {
        Self {
            id: record.id.clone(),
            slug: record.slug.clone(),
            title: record.title.clone(),
            summary: record.summary.clone(),
            trial_days: record.trial_days,
            source_kind: record.source_kind.clone(),
            source_ref: record.source_ref.clone(),
            terms: public_offer_terms(record.terms.clone(), record.trial_days),
        }
    }

    fn into_view(self) -> PublicOfferView {
        PublicOfferView {
            id: self.id,
            slug: self.slug,
            title: self.title,
            summary: self.summary,
            trial_days: self.trial_days,
            source_kind: self.source_kind,
            source_ref: self.source_ref,
            terms: self.terms,
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
            idempotency_key: self.idempotency_key,
            access_grant_id: self.access_grant_id,
            receipt: self.receipt,
            status: self.status,
            accepted_at: self.accepted_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl AccessGrantRecord {
    fn into_view(self) -> AccessGrantView {
        AccessGrantView {
            id: self.id,
            resource_kind: self.resource_kind,
            resource_id: self.resource_id,
            action: self.action,
            subject_kind: self.subject_kind,
            subject_id: self.subject_id,
            effect: self.effect,
            created_at: self.created_at,
            expires_at: self.expires_at,
            metadata: self.metadata,
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

impl HostedTrialCapacityPolicyRecord {
    fn into_view(self) -> HostedTrialCapacityPolicyView {
        HostedTrialCapacityPolicyView {
            id: self.id,
            offer_id: self.offer_id,
            offer_slug: self.offer_slug,
            status: self.status,
            active_slot_limit: self.active_slot_limit,
            active_slot_count: self.active_slot_count,
            waitlist_count: self.waitlist_count,
            trial_days: self.trial_days,
            backup_before_wipe_required: self.backup_before_wipe_required,
            reset_grace_days: self.reset_grace_days,
            metadata: self.metadata,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl HostedTrialSlotRecord {
    fn into_view(self) -> HostedTrialSlotView {
        HostedTrialSlotView {
            id: self.id,
            policy_id: self.policy_id,
            trial_id: self.trial_id,
            acceptance_id: self.acceptance_id,
            offer_id: self.offer_id,
            offer_slug: self.offer_slug,
            subject_kind: self.subject_kind,
            subject_id: self.subject_id,
            status: self.status,
            allocated_at: self.allocated_at,
            expires_at: self.expires_at,
            released_at: self.released_at,
            release_reason: self.release_reason,
            backup_required: self.backup_required,
            backup_status: self.backup_status,
            backup_evidence_refs: self.backup_evidence_refs,
            reset_eligible_at: self.reset_eligible_at,
            reset_state: self.reset_state,
            reset_guard: self.reset_guard,
            owner_override: self.owner_override,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn into_reset_plan(self) -> HostedTrialResetPlanView {
        HostedTrialResetPlanView {
            trial_id: self.trial_id,
            slot_id: self.id,
            backup_status: self.backup_status,
            backup_evidence_refs: self.backup_evidence_refs,
            reset_state: self.reset_state,
            reset_eligible_at: self.reset_eligible_at,
            owner_override: self.owner_override,
        }
    }
}

impl HostedTrialWaitlistEntryRecord {
    fn into_view(self) -> HostedTrialWaitlistEntryView {
        HostedTrialWaitlistEntryView {
            id: self.id,
            policy_id: self.policy_id,
            acceptance_id: self.acceptance_id,
            offer_id: self.offer_id,
            offer_slug: self.offer_slug,
            visitor_session_id: self.visitor_session_id,
            subject_kind: self.subject_kind,
            subject_id: self.subject_id,
            status: self.status,
            position: self.position,
            reason: self.reason,
            receipt: self.receipt,
            evidence_refs: self.evidence_refs,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn acceptance_idempotency_key(
    explicit_key: Option<&str>,
    local_session: Option<&LocalSessionContext>,
    visitor_session: Option<&VisitorSessionContext>,
) -> Result<Option<String>> {
    if let Some(key) = explicit_key {
        return require_idempotency_key(key).map(Some);
    }
    if let Some(session) = local_session {
        return Ok(Some(format!("local_session:{}", session.session_id)));
    }
    if let Some(session) = visitor_session {
        return Ok(Some(format!("visitor_session:{}", session.id)));
    }
    Ok(None)
}

fn access_subject_for_acceptance(
    local_session: Option<&LocalSessionContext>,
    visitor_session_id: Option<&str>,
    acceptance_id: &str,
) -> AccessSubject {
    if let Some(session) = local_session {
        return AccessSubject {
            subject_kind: "actor".to_string(),
            subject_id: session.actor_id.clone(),
            local_session_id: Some(session.session_id.clone()),
        };
    }

    if let Some(visitor_session_id) = visitor_session_id {
        return AccessSubject {
            subject_kind: "actor".to_string(),
            subject_id: format!("actor_{visitor_session_id}"),
            local_session_id: None,
        };
    }

    AccessSubject {
        subject_kind: "actor".to_string(),
        subject_id: format!("actor_{acceptance_id}"),
        local_session_id: None,
    }
}

fn ensure_access_subject_actor_tx(
    transaction: &Transaction<'_>,
    subject: &AccessSubject,
    acceptance_id: &str,
    visitor_session_id: Option<&str>,
    now: &str,
) -> Result<()> {
    if subject.subject_kind != "actor" {
        return Ok(());
    }

    transaction.execute(
        "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
         VALUES (?1, 'browser_operator', 'Hosted trial member', 'active', ?2, ?3, ?3)
         ON CONFLICT(id) DO UPDATE SET status = 'active', updated_at = excluded.updated_at",
        params![
            subject.subject_id,
            json!({
                "source": "offer_acceptance",
                "acceptanceId": acceptance_id,
                "visitorSessionId": visitor_session_id,
                "localSessionId": subject.local_session_id,
            })
            .to_string(),
            now,
        ],
    )?;
    Ok(())
}

fn offer_acceptance_receipt(
    offer: &PublicOfferRecord,
    trial_id: &str,
    trial_days: i64,
    trial_ends_at: &str,
    access_grant_id: &str,
    acceptance_id: &str,
    hosted_trial_slot_id: Option<&str>,
) -> OfferAcceptanceReceipt {
    let mut evidence_refs = vec![
        format!("offer:{}", offer.id),
        format!("offer_acceptance:{acceptance_id}"),
        format!("trial:{trial_id}"),
        format!("resource_grant:{access_grant_id}"),
    ];
    if let Some(slot_id) = hosted_trial_slot_id {
        evidence_refs.push(format!("hosted_trial_slot:{slot_id}"));
    }

    OfferAcceptanceReceipt {
        schema_version: OFFER_RECEIPT_SCHEMA_VERSION.to_string(),
        status: "accepted".to_string(),
        offer_slug: offer.slug.clone(),
        trial_id: trial_id.to_string(),
        trial_days,
        trial_ends_at: trial_ends_at.to_string(),
        access_grant_id: access_grant_id.to_string(),
        terms_snapshot: public_offer_terms(offer.terms.clone(), offer.trial_days),
        expectations: vec![
            "This is an experimental hosted pilot, not production-critical infrastructure."
                .to_string(),
            "AI outputs require human review.".to_string(),
            "Export a backup before the hosted trial expires, resets, or is wiped.".to_string(),
            "Rewards, extensions, and capacity rules are governed separately and can be reviewed."
                .to_string(),
        ],
        support: "You can request support or a strategy handoff from Ordo; human attention remains policy-gated.".to_string(),
        evidence_refs,
    }
}

fn hosted_trial_waitlist_receipt(
    offer: &PublicOfferRecord,
    acceptance_id: &str,
    waitlist_id: &str,
    position: i64,
    active_slot_limit: i64,
) -> Value {
    json!({
        "schemaVersion": OFFER_RECEIPT_SCHEMA_VERSION,
        "status": "waitlisted",
        "offerSlug": offer.slug,
        "waitlistEntryId": waitlist_id,
        "waitlistPosition": position,
        "activeSlotLimit": active_slot_limit,
        "expectations": [
            "The hosted pilot is currently at capacity.",
            "No hosted-trial Access has been granted yet.",
            "Ordo recorded your request without creating an unsupported active trial.",
            "Rewards and extensions are governed separately and do not bypass capacity."
        ],
        "support": "Ordo can explain your waitlist status; human attention remains policy-gated.",
        "evidenceRefs": [
            format!("offer:{}", offer.id),
            format!("offer_acceptance:{acceptance_id}"),
            format!("hosted_trial_waitlist:{waitlist_id}")
        ]
    })
}

fn receipt_from_value(value: Value) -> Result<OfferAcceptanceReceipt> {
    serde_json::from_value(value).map_err(Into::into)
}

fn update_hosted_trial_slot_for_transition_tx(
    transaction: &Transaction<'_>,
    trial: &TrialRecord,
    status: TrialStatus,
    decision_evidence: &Value,
    now: &str,
) -> Result<()> {
    let Some(slot) = find_hosted_trial_slot_by_trial_id(transaction, &trial.id)? else {
        return Ok(());
    };

    match status {
        TrialStatus::Converted | TrialStatus::Voided | TrialStatus::Expired => {
            let reset_state = match status {
                TrialStatus::Converted => "converted_no_wipe",
                TrialStatus::Voided | TrialStatus::Expired => {
                    HOSTED_TRIAL_RESET_BLOCKED_PENDING_BACKUP
                }
                TrialStatus::Started | TrialStatus::FollowUpNeeded => unreachable!(),
            };
            transaction.execute(
                "UPDATE hosted_trial_slots
                 SET status = ?1,
                     released_at = COALESCE(released_at, ?2),
                     release_reason = COALESCE(release_reason, ?1),
                     reset_state = ?3,
                     reset_guard_json = ?4,
                     owner_override_json = ?5,
                     updated_at = ?2
                 WHERE id = ?6",
                params![
                    status.as_str(),
                    now,
                    reset_state,
                    json!({
                        "backupBeforeWipeRequired": true,
                        "destructiveWipeAllowed": false,
                        "reason": reset_state,
                        "requires": ["backup_export_evidence", "owner_review"]
                    })
                    .to_string(),
                    decision_evidence.to_string(),
                    slot.id,
                ],
            )?;
            transaction.execute(
                "UPDATE resource_grants
                 SET expires_at = ?1
                 WHERE resource_kind = ?2 AND resource_id = ?3 AND action = ?4",
                params![
                    now,
                    HOSTED_TRIAL_RESOURCE_KIND,
                    trial.id,
                    HOSTED_TRIAL_ACTION,
                ],
            )?;
        }
        TrialStatus::FollowUpNeeded => {
            transaction.execute(
                "UPDATE hosted_trial_slots
                 SET owner_override_json = ?1,
                     updated_at = ?2
                 WHERE id = ?3",
                params![decision_evidence.to_string(), now, slot.id],
            )?;
        }
        TrialStatus::Started => {}
    }
    Ok(())
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

fn require_idempotency_key(value: &str) -> Result<String> {
    let normalized = normalize_optional_string(Some(value.to_string()))
        .ok_or_else(|| anyhow::anyhow!("Idempotency key is required."))?;
    if normalized.len() > 200
        || !normalized.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':')
        })
    {
        bail!("Idempotency key must be a stable identifier.");
    }
    Ok(normalized)
}

fn require_text(value: &str, label: &str) -> Result<String> {
    normalize_optional_string(Some(value.to_string()))
        .ok_or_else(|| anyhow::anyhow!("{label} is required."))
}

fn normalize_evidence_refs(values: Vec<String>) -> Result<Vec<String>> {
    let mut normalized = Vec::new();
    for value in values {
        let value = normalize_optional_string(Some(value))
            .ok_or_else(|| anyhow::anyhow!("Backup evidence ref cannot be empty."))?;
        if value.len() > 240
            || !value.chars().all(|character| {
                character.is_ascii_alphanumeric()
                    || matches!(character, '_' | '-' | '.' | ':' | '/' | '#')
            })
        {
            bail!("Backup evidence ref must be a stable evidence reference.");
        }
        if !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    Ok(normalized)
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
    use crate::local_sessions::{create_or_restore_local_session, LocalSessionCreateRequest};
    use crate::policy::{
        authorize_resource_access, ActorContext, ActorKind, PolicyAction, PolicyOutcome,
        ResourceKind, ResourceRef, LOCAL_OWNER_ACTOR_ID,
    };
    use crate::schema::init_database;
    use crate::surface_work_items::{
        list_surface_work_items, SurfaceWorkItemQuery, SurfaceWorkItemViewer,
    };
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

        let (local_session, _) = create_or_restore_local_session(
            &db_path,
            LocalSessionCreateRequest {
                mode: "register".to_string(),
                name: Some("Maya Pilot".to_string()),
                email: "maya@example.com".to_string(),
                password: "local-password".to_string(),
            },
        )
        .unwrap();

        let (acceptance, trial, access_grant, receipt, event) = accept_public_offer(
            &db_path,
            &offer.slug,
            OfferAcceptanceCreateRequest {
                visitor_session_id: None,
                local_session_id: Some(local_session.session.session_id.clone()),
                idempotency_key: None,
                attribution: Some(json!({ "source": "direct" })),
                acceptance_context: Some(json!({ "note": "ready" })),
            },
        )
        .unwrap();

        assert_eq!(acceptance.offer_slug, "trial-pack");
        assert_eq!(acceptance.attribution["source"], "direct");
        assert_eq!(
            acceptance.access_grant_id.as_deref(),
            Some(access_grant.id.as_str())
        );
        assert_eq!(trial.status, TrialStatus::Started);
        assert_eq!(trial.offer_slug, "trial-pack");
        assert_eq!(trial.decision_evidence["accessGrantId"], access_grant.id);
        assert_eq!(access_grant.resource_kind, HOSTED_TRIAL_RESOURCE_KIND);
        assert_eq!(access_grant.resource_id, trial.id);
        assert_eq!(access_grant.action, HOSTED_TRIAL_ACTION);
        assert_eq!(access_grant.subject_kind, "actor");
        assert_eq!(access_grant.subject_id, local_session.session.actor_id);
        assert_eq!(
            access_grant.expires_at.as_deref(),
            Some(trial.trial_ends_at.as_str())
        );
        assert_eq!(receipt.access_grant_id, access_grant.id);
        assert_eq!(receipt.trial_id, trial.id);
        let receipt_json = serde_json::to_string(&receipt).unwrap();
        assert!(!receipt_json.contains("provider"));
        assert!(!receipt_json.contains("secret"));
        assert!(!receipt_json.contains("rawPrompt"));
        assert!(!receipt_json.contains("SLA"));
        assert_eq!(event.event_type, "offer.accepted");
        let connection = Connection::open(&db_path).unwrap();
        let grant_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM resource_grants
                 WHERE id = ?1
                   AND metadata_json LIKE ?2
                   AND metadata_json NOT LIKE '%Sensitive Browser%'",
                params![access_grant.id, format!("%{}%", acceptance.id)],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(grant_count, 1);
        let access_decision = authorize_resource_access(
            &connection,
            ActorContext::new(
                ActorKind::BrowserOperator,
                "test",
                Some(local_session.session.actor_id.clone()),
            ),
            PolicyAction::Use,
            ResourceRef::new(ResourceKind::HostedTrial, trial.id.clone()),
            None,
        );
        assert_eq!(access_decision.outcome, PolicyOutcome::Allowed);
        let other_actor_decision = authorize_resource_access(
            &connection,
            ActorContext::new(
                ActorKind::BrowserOperator,
                "test",
                Some("actor_other_member".to_string()),
            ),
            PolicyAction::Use,
            ResourceRef::new(ResourceKind::HostedTrial, trial.id.clone()),
            None,
        );
        assert_eq!(other_actor_decision.outcome, PolicyOutcome::Denied);
        let trial_events: i64 = connection
            .query_row("SELECT COUNT(*) FROM trial_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(trial_events, 1);
        let outcome_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM business_outcomes
                 WHERE outcome_kind = 'offer_acceptance'
                   AND offer_id = ?1
                   AND evidence_refs_json LIKE ?2",
                params![offer.id, format!("%{}%", acceptance.id)],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(outcome_count, 1);
        let attribution_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM business_outcome_attributions
                 WHERE attribution_kind = 'offer'
                   AND source_id = ?1",
                [offer.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(attribution_count, 1);
    }

    #[test]
    fn offer_builder_publishes_pilot_offer_with_supported_references() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        seed_public_about(&db_path);
        create_entry_point(
            &db_path,
            EntryPointWriteRequest {
                slug: "nyc-meetup".to_string(),
                label: "NYC Meetup".to_string(),
                status: None,
                source_kind: "qr".to_string(),
                source_label: Some("NYC meetup".to_string()),
                destination_surface: PublicDestinationSurface::About,
                destination_id: None,
                attribution: Some(json!({ "campaign": "nyc-pilot" })),
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let (saved, event) = save_offer_builder_offer(
            &db_path,
            OfferBuilderSaveRequest {
                offer: OfferWriteRequest {
                    slug: "nyc-pilot".to_string(),
                    title: "OrdoStudio NYC Pilot".to_string(),
                    summary: "30 days of experimental hosted OrdoStudio access.".to_string(),
                    status: Some(OfferStatus::Available),
                    visibility: Some(BusinessFactVisibility::Public),
                    publication_state: Some(PublicationState::Published),
                    trial_days: Some(30),
                    source_kind: Some("offer_builder".to_string()),
                    source_ref: Some("nyc-pilot".to_string()),
                    terms: Some(json!({
                        "termsVersion": "2026-05-13",
                        "trialDays": 30,
                        "experimentalHosting": true,
                        "backupBeforeWipeRequired": true,
                        "humanReviewRequired": true
                    })),
                    metadata: None,
                },
                references: Some(OfferBuilderReferencesRequest {
                    tracked_entry_point_slug: Some("nyc-meetup".to_string()),
                    support_handoff_cta: Some(true),
                    reward_program_id: Some("reward_program_ordostudio_nyc_pilot".to_string()),
                    pack_ids: None,
                    external_publishing: None,
                    payment: None,
                    oauth: None,
                }),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(saved.offer.slug, "nyc-pilot");
        assert_eq!(event.event_type, "offer_builder.saved");
        assert!(saved.validation.publishable);
        assert!(saved.validation.blockers.is_empty());
        assert!(saved.public_preview.is_some());
        assert!(saved
            .validation
            .supported_references
            .iter()
            .any(|reference| reference.key == "hosted_trial_capacity"));
        assert!(saved
            .validation
            .supported_references
            .iter()
            .any(|reference| reference.key == "tracked_entry_point"));
        assert!(saved
            .validation
            .supported_references
            .iter()
            .any(|reference| reference.key == "reward_ledger" && reference.status == "available"));

        let public_offers = list_public_available_offers(&db_path).unwrap();
        let public_offer = public_offers
            .offers
            .iter()
            .find(|offer| offer.slug == "nyc-pilot")
            .unwrap();
        assert_eq!(public_offer.title, "OrdoStudio NYC Pilot");
        assert_eq!(public_offer.terms["trialDays"], 30);
        let public_json = serde_json::to_string(public_offer).unwrap();
        assert!(!public_json.contains("rewardProgramId"));
        assert!(!public_json.contains("packIds"));
        assert!(!public_json.contains("provider"));
        assert!(!public_json.contains("secret"));
        assert!(!public_json.contains("rawPrompt"));
    }

    #[test]
    fn offer_builder_blocks_unsupported_reward_pack_and_internal_references() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        let blocked = save_offer_builder_offer(
            &db_path,
            OfferBuilderSaveRequest {
                offer: OfferWriteRequest {
                    slug: "unsafe-pilot".to_string(),
                    title: "Unsafe Pilot".to_string(),
                    summary: "Should not publish.".to_string(),
                    status: Some(OfferStatus::Available),
                    visibility: Some(BusinessFactVisibility::Public),
                    publication_state: Some(PublicationState::Published),
                    trial_days: Some(30),
                    source_kind: Some("offer_builder".to_string()),
                    source_ref: None,
                    terms: Some(json!({
                        "trialDays": 30,
                        "experimentalHosting": true,
                        "providerSecret": "sk_live_leak"
                    })),
                    metadata: Some(json!({ "rawPrompt": "internal system prompt" })),
                },
                references: Some(OfferBuilderReferencesRequest {
                    tracked_entry_point_slug: None,
                    support_handoff_cta: Some(true),
                    reward_program_id: Some("reward_program_fake".to_string()),
                    pack_ids: Some(vec!["pack.fake".to_string()]),
                    external_publishing: Some(true),
                    payment: Some(true),
                    oauth: Some(true),
                }),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap_err()
        .to_string();

        assert!(blocked.contains("Reward program is not active or does not exist"));
        assert!(blocked.contains("Product/workforce pack offer bindings are not available yet"));
        assert!(blocked.contains("Public offers cannot include internal or secret-bearing keys"));
        let offers = list_offers(&db_path).unwrap();
        assert!(offers.offers.is_empty());
    }

    #[test]
    fn offer_builder_edits_existing_offer_by_slug_without_duplicate() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        let (created, _) = save_offer_builder_offer(
            &db_path,
            OfferBuilderSaveRequest {
                offer: OfferWriteRequest {
                    slug: "nyc-pilot".to_string(),
                    title: "OrdoStudio NYC Pilot".to_string(),
                    summary: "Initial pilot summary.".to_string(),
                    status: Some(OfferStatus::Draft),
                    visibility: Some(BusinessFactVisibility::Owner),
                    publication_state: Some(PublicationState::Draft),
                    trial_days: Some(30),
                    source_kind: Some("offer_builder".to_string()),
                    source_ref: Some("nyc-pilot".to_string()),
                    terms: Some(json!({
                        "termsVersion": "draft",
                        "trialDays": 30,
                        "experimentalHosting": true,
                        "backupBeforeWipeRequired": true
                    })),
                    metadata: None,
                },
                references: Some(OfferBuilderReferencesRequest {
                    tracked_entry_point_slug: None,
                    support_handoff_cta: Some(false),
                    reward_program_id: None,
                    pack_ids: None,
                    external_publishing: None,
                    payment: None,
                    oauth: None,
                }),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let (edited, _) = save_offer_builder_offer(
            &db_path,
            OfferBuilderSaveRequest {
                offer: OfferWriteRequest {
                    slug: "nyc-pilot".to_string(),
                    title: "OrdoStudio NYC Pilot".to_string(),
                    summary: "Published pilot summary.".to_string(),
                    status: Some(OfferStatus::Available),
                    visibility: Some(BusinessFactVisibility::Public),
                    publication_state: Some(PublicationState::Published),
                    trial_days: Some(30),
                    source_kind: Some("offer_builder".to_string()),
                    source_ref: Some("nyc-pilot".to_string()),
                    terms: Some(json!({
                        "termsVersion": "published",
                        "trialDays": 30,
                        "experimentalHosting": true,
                        "backupBeforeWipeRequired": true
                    })),
                    metadata: None,
                },
                references: Some(OfferBuilderReferencesRequest {
                    tracked_entry_point_slug: None,
                    support_handoff_cta: Some(true),
                    reward_program_id: None,
                    pack_ids: None,
                    external_publishing: None,
                    payment: None,
                    oauth: None,
                }),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(created.offer.id, edited.offer.id);
        assert_eq!(edited.offer.summary, "Published pilot summary.");
        assert!(edited.validation.publishable);
        assert!(edited.public_preview.is_some());

        let offers = list_offers(&db_path).unwrap();
        assert_eq!(offers.offers.len(), 1);
        assert_eq!(offers.offers[0].terms["termsVersion"], "published");
    }

    #[test]
    fn acceptance_receipt_preserves_terms_snapshot_after_offer_edit() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (offer, _) = create_offer(
            &db_path,
            OfferWriteRequest {
                terms: Some(json!({
                    "termsVersion": "v1",
                    "trialDays": 30,
                    "experimentalHosting": true,
                    "backupBeforeWipeRequired": true
                })),
                ..public_offer_request("terms-pack")
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let (acceptance, _, _, receipt, _) = accept_public_offer(
            &db_path,
            &offer.slug,
            OfferAcceptanceCreateRequest {
                visitor_session_id: None,
                local_session_id: None,
                idempotency_key: Some("terms-snapshot".to_string()),
                attribution: None,
                acceptance_context: None,
            },
        )
        .unwrap();
        assert_eq!(receipt.terms_snapshot["termsVersion"], "v1");

        update_offer(
            &db_path,
            &offer.id,
            OfferWriteRequest {
                terms: Some(json!({
                    "termsVersion": "v2",
                    "trialDays": 30,
                    "experimentalHosting": true,
                    "backupBeforeWipeRequired": true
                })),
                ..public_offer_request("terms-pack")
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let connection = Connection::open(&db_path).unwrap();
        let stored_receipt: String = connection
            .query_row(
                "SELECT receipt_json FROM offer_acceptances WHERE id = ?1",
                [acceptance.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        let stored_receipt: OfferAcceptanceReceipt = serde_json::from_str(&stored_receipt).unwrap();
        assert_eq!(stored_receipt.terms_snapshot["termsVersion"], "v1");
        let current_public = list_public_available_offers(&db_path)
            .unwrap()
            .offers
            .into_iter()
            .find(|offer| offer.slug == "terms-pack")
            .unwrap();
        assert_eq!(current_public.terms["termsVersion"], "v2");
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
                local_session_id: None,
                idempotency_key: Some("private-denied".to_string()),
                attribution: None,
                acceptance_context: None,
            },
        );

        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not publicly available"));
        let connection = Connection::open(&db_path).unwrap();
        let grant_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM resource_grants
                 WHERE metadata_json LIKE '%private-denied%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(grant_count, 0);
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
                session_id: None,
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

        let (acceptance, _, access_grant, _, _) = accept_public_offer(
            &db_path,
            "session-pack",
            OfferAcceptanceCreateRequest {
                visitor_session_id: Some(session.id.clone()),
                local_session_id: None,
                idempotency_key: None,
                attribution: Some(json!({ "intent": "trial" })),
                acceptance_context: None,
            },
        )
        .unwrap();

        assert_eq!(acceptance.visitor_session_id, Some(session.id.clone()));
        assert_eq!(
            acceptance.entry_point_slug,
            Some("partner-link".to_string())
        );
        assert_eq!(acceptance.attribution["campaign"], "partner");
        assert_eq!(acceptance.attribution["medium"], "qr");
        assert_eq!(acceptance.attribution["intent"], "trial");
        assert_eq!(access_grant.subject_id, format!("actor_{}", session.id));
        assert_eq!(access_grant.metadata["entryPointSlug"], "partner-link");
        assert!(!access_grant
            .metadata
            .to_string()
            .contains("Sensitive Browser"));
    }

    #[test]
    fn public_offer_acceptance_is_idempotent_for_retry_key() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (offer, _) = create_offer(
            &db_path,
            public_offer_request("retry-pack"),
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        let request = OfferAcceptanceCreateRequest {
            visitor_session_id: None,
            local_session_id: None,
            idempotency_key: Some("retry-direct-1".to_string()),
            attribution: Some(json!({ "source": "direct" })),
            acceptance_context: None,
        };

        let (first_acceptance, first_trial, first_grant, _, first_event) =
            accept_public_offer(&db_path, &offer.slug, request.clone()).unwrap();
        let (second_acceptance, second_trial, second_grant, _, second_event) =
            accept_public_offer(&db_path, &offer.slug, request).unwrap();

        assert_eq!(first_acceptance.id, second_acceptance.id);
        assert_eq!(first_trial.id, second_trial.id);
        assert_eq!(first_grant.id, second_grant.id);
        assert_eq!(first_event.event_type, "offer.accepted");
        assert_eq!(second_event.event_type, "offer.acceptance.replayed");
        let connection = Connection::open(&db_path).unwrap();
        let acceptance_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM offer_acceptances WHERE offer_id = ?1",
                [offer.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        let trial_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM trials WHERE offer_id = ?1",
                [offer.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        let grant_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM resource_grants
                 WHERE metadata_json LIKE ?1",
                [format!("%{}%", first_acceptance.id)],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(acceptance_count, 1);
        assert_eq!(trial_count, 1);
        assert_eq!(grant_count, 1);
    }

    #[test]
    fn accepted_offer_access_projects_to_member_view_for_subject() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (offer, _) = create_offer(
            &db_path,
            public_offer_request("member-access-pack"),
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        let (local_session, _) = create_or_restore_local_session(
            &db_path,
            LocalSessionCreateRequest {
                mode: "register".to_string(),
                name: Some("Ava Pilot".to_string()),
                email: "ava@example.com".to_string(),
                password: "local-password".to_string(),
            },
        )
        .unwrap();

        let (_, _, access_grant, _, _) = accept_public_offer(
            &db_path,
            &offer.slug,
            OfferAcceptanceCreateRequest {
                visitor_session_id: None,
                local_session_id: Some(local_session.session.session_id.clone()),
                idempotency_key: None,
                attribution: None,
                acceptance_context: None,
            },
        )
        .unwrap();

        let items = list_surface_work_items(
            &db_path,
            SurfaceWorkItemQuery {
                viewer: SurfaceWorkItemViewer::Member,
                surface_kind: Some("member".to_string()),
                room_kind: Some("access".to_string()),
                actor_id: Some(local_session.session.actor_id.clone()),
                ..SurfaceWorkItemQuery::default()
            },
        )
        .unwrap()
        .items;

        assert!(items.iter().any(|item| {
            item.source_kind == "resource_grant"
                && item.source_id == access_grant.id
                && item.actor_context["subjectId"] == local_session.session.actor_id
        }));
        assert!(items.iter().all(|item| item.visibility == "authenticated"));
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
        let (_, trial, _, _, _) = accept_public_offer(
            &db_path,
            "conversion-pack",
            OfferAcceptanceCreateRequest {
                visitor_session_id: None,
                local_session_id: None,
                idempotency_key: Some("conversion-pack-direct".to_string()),
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
    fn hosted_trial_capacity_allocates_ten_and_waitlists_eleventh() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (offer, _) = create_offer(
            &db_path,
            public_offer_request("capacity-pack"),
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        for index in 0..10 {
            let (acceptance, trial, access_grant, receipt, _) = accept_public_offer(
                &db_path,
                &offer.slug,
                OfferAcceptanceCreateRequest {
                    visitor_session_id: None,
                    local_session_id: None,
                    idempotency_key: Some(format!("capacity-active-{index}")),
                    attribution: None,
                    acceptance_context: None,
                },
            )
            .unwrap();
            assert_eq!(acceptance.status, AcceptanceStatus::Accepted);
            assert_eq!(
                acceptance.access_grant_id.as_deref(),
                Some(access_grant.id.as_str())
            );
            assert_eq!(receipt.status, "accepted");
            assert_eq!(trial.status, TrialStatus::Started);
        }

        let capacity = list_hosted_trial_capacity(&db_path).unwrap();
        let policy = capacity
            .policies
            .iter()
            .find(|policy| policy.offer_slug == "capacity-pack")
            .unwrap();
        assert_eq!(policy.active_slot_limit, 10);
        assert_eq!(policy.active_slot_count, 10);
        assert_eq!(policy.waitlist_count, 0);
        assert_eq!(capacity.slots.len(), 10);
        assert!(capacity.slots.iter().all(|slot| {
            slot.status == "active"
                && slot.backup_required
                && slot.backup_status == "required"
                && slot.reset_state == "blocked_until_expiration"
        }));

        let waitlist_request = OfferAcceptanceCreateRequest {
            visitor_session_id: None,
            local_session_id: None,
            idempotency_key: Some("capacity-waitlist-11".to_string()),
            attribution: None,
            acceptance_context: Some(json!({ "note": "please hold a spot" })),
        };
        let first_waitlist = accept_public_offer(&db_path, &offer.slug, waitlist_request.clone())
            .unwrap_err()
            .to_string();
        let second_waitlist = accept_public_offer(&db_path, &offer.slug, waitlist_request)
            .unwrap_err()
            .to_string();
        assert!(first_waitlist.contains("capacity is full"));
        assert!(second_waitlist.contains("already waitlisted"));

        let capacity = list_hosted_trial_capacity(&db_path).unwrap();
        let policy = capacity
            .policies
            .iter()
            .find(|policy| policy.offer_slug == "capacity-pack")
            .unwrap();
        assert_eq!(policy.active_slot_count, 10);
        assert_eq!(policy.waitlist_count, 1);
        assert_eq!(capacity.waitlist.len(), 1);
        assert_eq!(capacity.waitlist[0].position, 1);
        assert_eq!(capacity.waitlist[0].status, "waiting");
        assert_eq!(capacity.waitlist[0].reason, "capacity_full");

        let connection = Connection::open(&db_path).unwrap();
        let waitlisted_acceptance_id: String = connection
            .query_row(
                "SELECT id FROM offer_acceptances WHERE idempotency_key = 'capacity-waitlist-11'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let waitlisted_status: String = connection
            .query_row(
                "SELECT status FROM offer_acceptances WHERE id = ?1",
                [waitlisted_acceptance_id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        let waitlisted_trials: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM trials WHERE acceptance_id = ?1",
                [waitlisted_acceptance_id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        let waitlisted_grants: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM resource_grants
                 WHERE metadata_json LIKE ?1",
                [format!("%{}%", waitlisted_acceptance_id)],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(waitlisted_status, "waitlisted");
        assert_eq!(waitlisted_trials, 0);
        assert_eq!(waitlisted_grants, 0);
    }

    #[test]
    fn expired_trial_releases_capacity_and_reset_requires_backup_evidence() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (offer, _) = create_offer(
            &db_path,
            public_offer_request("lifecycle-pack"),
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let mut first_trial_id = String::new();
        for index in 0..10 {
            let (_, trial, _, _, _) = accept_public_offer(
                &db_path,
                &offer.slug,
                OfferAcceptanceCreateRequest {
                    visitor_session_id: None,
                    local_session_id: None,
                    idempotency_key: Some(format!("lifecycle-active-{index}")),
                    attribution: None,
                    acceptance_context: None,
                },
            )
            .unwrap();
            if index == 0 {
                first_trial_id = trial.id;
            }
        }

        let (expired, _) = transition_trial(
            &db_path,
            &first_trial_id,
            TrialTransitionRequest {
                status: TrialStatus::Expired,
                decision_evidence: Some(json!({ "reason": "window_elapsed" })),
            },
        )
        .unwrap();
        assert_eq!(expired.status, TrialStatus::Expired);

        let capacity = list_hosted_trial_capacity(&db_path).unwrap();
        let expired_slot = capacity
            .slots
            .iter()
            .find(|slot| slot.trial_id == first_trial_id)
            .unwrap();
        assert_eq!(expired_slot.status, "expired");
        assert_eq!(expired_slot.backup_status, "required");
        assert_eq!(expired_slot.reset_state, "blocked_pending_backup");
        assert_eq!(expired_slot.release_reason.as_deref(), Some("expired"));

        let blocked = request_hosted_trial_reset(
            &db_path,
            &first_trial_id,
            HostedTrialResetRequest {
                backup_evidence_refs: vec![],
                owner_decision: Some(json!({ "actorId": LOCAL_OWNER_ACTOR_ID })),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(blocked.contains("backup/export evidence"));

        let missing_owner_decision = request_hosted_trial_reset(
            &db_path,
            &first_trial_id,
            HostedTrialResetRequest {
                backup_evidence_refs: vec!["backup:export_1".to_string()],
                owner_decision: None,
            },
        )
        .unwrap_err()
        .to_string();
        assert!(missing_owner_decision.contains("owner decision evidence"));

        let (plan, event) = request_hosted_trial_reset(
            &db_path,
            &first_trial_id,
            HostedTrialResetRequest {
                backup_evidence_refs: vec!["backup:export_1".to_string()],
                owner_decision: Some(json!({
                    "actorId": LOCAL_OWNER_ACTOR_ID,
                    "reason": "operator reviewed backup before reset"
                })),
            },
        )
        .unwrap();
        assert_eq!(plan.backup_status, "ready");
        assert_eq!(plan.reset_state, "ready_for_owner_review");
        assert_eq!(plan.backup_evidence_refs, vec!["backup:export_1"]);
        assert_eq!(event.event_type, "trial.reset.ready");

        let (_, new_trial, _, _, _) = accept_public_offer(
            &db_path,
            &offer.slug,
            OfferAcceptanceCreateRequest {
                visitor_session_id: None,
                local_session_id: None,
                idempotency_key: Some("lifecycle-after-expiry".to_string()),
                attribution: None,
                acceptance_context: None,
            },
        )
        .unwrap();
        assert_ne!(new_trial.id, first_trial_id);

        let capacity = list_hosted_trial_capacity(&db_path).unwrap();
        let policy = capacity
            .policies
            .iter()
            .find(|policy| policy.offer_slug == "lifecycle-pack")
            .unwrap();
        assert_eq!(policy.active_slot_count, 10);
        assert_eq!(policy.waitlist_count, 0);
    }

    #[test]
    fn converted_hosted_trial_cannot_be_marked_reset_ready() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let (offer, _) = create_offer(
            &db_path,
            public_offer_request("converted-pack"),
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let (_, trial, _, _, _) = accept_public_offer(
            &db_path,
            &offer.slug,
            OfferAcceptanceCreateRequest {
                visitor_session_id: None,
                local_session_id: None,
                idempotency_key: Some("converted-active".to_string()),
                attribution: None,
                acceptance_context: None,
            },
        )
        .unwrap();

        transition_trial(
            &db_path,
            &trial.id,
            TrialTransitionRequest {
                status: TrialStatus::Converted,
                decision_evidence: Some(json!({
                    "actorId": LOCAL_OWNER_ACTOR_ID,
                    "reason": "user converted to retained account"
                })),
            },
        )
        .unwrap();

        let blocked = request_hosted_trial_reset(
            &db_path,
            &trial.id,
            HostedTrialResetRequest {
                backup_evidence_refs: vec!["backup:converted_export".to_string()],
                owner_decision: Some(json!({
                    "actorId": LOCAL_OWNER_ACTOR_ID,
                    "reason": "operator attempted converted reset"
                })),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(blocked.contains("Converted hosted trials are retained"));

        let capacity = list_hosted_trial_capacity(&db_path).unwrap();
        let slot = capacity
            .slots
            .iter()
            .find(|slot| slot.trial_id == trial.id)
            .unwrap();
        assert_eq!(slot.status, "converted");
        assert_eq!(slot.reset_state, "converted_no_wipe");
        assert_eq!(slot.backup_status, "required");
        assert!(slot.backup_evidence_refs.is_empty());
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

        let (acceptance, trial, _, _, _) = accept_public_offer(
            &db_path,
            "bootstrap",
            OfferAcceptanceCreateRequest {
                visitor_session_id: None,
                local_session_id: None,
                idempotency_key: Some("bootstrap-direct".to_string()),
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
