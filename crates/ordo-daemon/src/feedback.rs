use anyhow::{bail, ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

use crate::events::{append_realtime_event, system_event, RealtimeEvent};
use crate::schema::db::ConnectionExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Candidate,
    Requested,
    Received,
    ConsentConfirmed,
    Approved,
    Published,
    Featured,
    Retired,
}

impl ReviewStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Requested => "requested",
            Self::Received => "received",
            Self::ConsentConfirmed => "consent_confirmed",
            Self::Approved => "approved",
            Self::Published => "published",
            Self::Featured => "featured",
            Self::Retired => "retired",
        }
    }
}

impl TryFrom<&str> for ReviewStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "candidate" => Ok(Self::Candidate),
            "requested" => Ok(Self::Requested),
            "received" => Ok(Self::Received),
            "consent_confirmed" => Ok(Self::ConsentConfirmed),
            "approved" => Ok(Self::Approved),
            "published" => Ok(Self::Published),
            "featured" => Ok(Self::Featured),
            "retired" => Ok(Self::Retired),
            other => bail!("Unsupported review status: {other}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerFeedbackView {
    pub id: String,
    pub connection_id: Option<String>,
    pub conversation_id: String,
    pub segment_id: Option<String>,
    pub message_id: Option<String>,
    pub feedback_kind: String,
    pub status: String,
    pub visibility: String,
    pub body_summary: String,
    pub is_starred: bool,
    pub source_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackTagView {
    pub id: String,
    pub feedback_id: String,
    pub tag: String,
    pub candidate_state: String,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub created_at: String,
    pub updated_at: String,
    pub state_changed_at: Option<String>,
    pub state_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerReviewView {
    pub id: String,
    pub feedback_id: String,
    pub connection_id: Option<String>,
    pub conversation_id: String,
    pub status: ReviewStatus,
    pub review_body: String,
    pub publication_visibility: String,
    pub consent_evidence_refs: Vec<String>,
    pub approval_evidence_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
    pub featured_at: Option<String>,
    pub retired_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CustomerFeedbackInput {
    pub connection_id: Option<String>,
    pub conversation_id: String,
    pub segment_id: Option<String>,
    pub message_id: Option<String>,
    pub feedback_kind: String,
    pub body_summary: String,
    pub source_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
}

#[derive(Debug, Clone)]
pub struct FeedbackTagInput {
    pub tag: String,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
}

#[derive(Debug, Clone)]
pub struct ReviewCandidateInput {
    pub review_body: String,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackRequestStatus {
    Open,
    Responded,
    FollowUpRequested,
    Accepted,
    Rejected,
    Expired,
    Canceled,
}

impl FeedbackRequestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Responded => "responded",
            Self::FollowUpRequested => "follow_up_requested",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
            Self::Canceled => "canceled",
        }
    }

    fn terminal(self) -> bool {
        matches!(
            self,
            Self::Accepted | Self::Rejected | Self::Expired | Self::Canceled
        )
    }
}

impl TryFrom<&str> for FeedbackRequestStatus {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "open" => Ok(Self::Open),
            "responded" => Ok(Self::Responded),
            "follow_up_requested" => Ok(Self::FollowUpRequested),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            "expired" => Ok(Self::Expired),
            "canceled" => Ok(Self::Canceled),
            other => bail!("Unsupported feedback request status: {other}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackRequestReviewDecision {
    Accepted,
    Rejected,
    FollowUpRequested,
}

impl FeedbackRequestReviewDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::FollowUpRequested => "follow_up_requested",
        }
    }
}

impl TryFrom<&str> for FeedbackRequestReviewDecision {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            "follow_up_requested" => Ok(Self::FollowUpRequested),
            other => bail!("Unsupported feedback request review decision: {other}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackRequestViewer {
    Public,
    Member,
    Staff,
    Growth,
    Owner,
    System,
}

impl Default for FeedbackRequestViewer {
    fn default() -> Self {
        Self::Member
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedbackRequestQuery {
    pub viewer: FeedbackRequestViewer,
    pub actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub target_kind: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackRequestListResponse {
    pub requests: Vec<FeedbackRequestView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackRequestView {
    pub id: String,
    pub target_kind: String,
    pub target_id: String,
    pub member_actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub conversation_id: Option<String>,
    pub source_kind: String,
    pub source_id: Option<String>,
    pub prompt: String,
    pub member_context_summary: String,
    pub status: FeedbackRequestStatus,
    pub due_at: Option<String>,
    pub priority: String,
    pub evidence_refs: Vec<String>,
    pub responses: Vec<FeedbackRequestResponseView>,
    pub reviews: Vec<FeedbackRequestReviewView>,
    pub reward_eligibility: Option<FeedbackRewardEligibilityView>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackRequestResponseView {
    pub id: String,
    pub request_id: String,
    pub actor_id: Option<String>,
    pub response_kind: String,
    pub body_summary: String,
    pub customer_feedback_id: Option<String>,
    pub evidence_refs: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackRequestReviewView {
    pub id: String,
    pub request_id: String,
    pub response_id: Option<String>,
    pub decision: FeedbackRequestReviewDecision,
    pub tags: Vec<String>,
    pub reason: Option<String>,
    pub evidence_refs: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackRewardEligibilityView {
    pub id: String,
    pub request_id: String,
    pub response_id: Option<String>,
    pub review_id: Option<String>,
    pub actor_id: Option<String>,
    pub state: String,
    pub reason: Option<String>,
    pub evidence_refs: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedbackRequestCreateRequest {
    pub target_kind: String,
    pub target_id: String,
    pub member_actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub conversation_id: Option<String>,
    pub source_kind: String,
    pub source_id: Option<String>,
    pub prompt: String,
    pub member_context_summary: String,
    pub due_at: Option<String>,
    pub priority: Option<String>,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub staff_context: Value,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeedbackRequestRespondRequest {
    pub response_kind: Option<String>,
    pub body_summary: String,
    pub idempotency_key: Option<String>,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackRequestReviewRequest {
    pub decision: FeedbackRequestReviewDecision,
    pub response_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub reason: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub provenance: Value,
}

#[derive(Debug, Clone)]
struct FeedbackRequestRecord {
    id: String,
    target_kind: String,
    target_id: String,
    member_actor_id: Option<String>,
    connection_id: Option<String>,
    conversation_id: Option<String>,
    source_kind: String,
    source_id: Option<String>,
    prompt: String,
    member_context_summary: String,
    status: FeedbackRequestStatus,
    due_at: Option<String>,
    priority: String,
    evidence_refs: Vec<String>,
    created_at: String,
    updated_at: String,
}

pub fn capture_feedback(
    connection: &Connection,
    input: CustomerFeedbackInput,
) -> Result<(CustomerFeedbackView, RealtimeEvent)> {
    validate_feedback_input(&input)?;
    let now = Utc::now().to_rfc3339();
    let id = format!("feedback_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO customer_feedback (
            id, connection_id, conversation_id, segment_id, message_id, feedback_kind,
            status, visibility, body_summary, is_starred, source_refs_json,
            evidence_refs_json, provenance_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'captured', 'private_business_intelligence',
            ?7, 0, ?8, ?9, ?10, ?11, ?11)",
        params![
            id,
            input.connection_id,
            input.conversation_id,
            input.segment_id,
            input.message_id,
            input.feedback_kind,
            input.body_summary,
            json!(input.source_refs).to_string(),
            json!(input.evidence_refs).to_string(),
            input.provenance.to_string(),
            now,
        ],
    )?;
    let feedback = load_feedback(connection, &id)?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "feedback.item.created",
            json!({
                "feedbackId": feedback.id,
                "conversationId": feedback.conversation_id,
                "visibility": feedback.visibility,
                "evidenceRefs": feedback.evidence_refs,
            }),
        ),
    )?;
    Ok((feedback, event))
}

pub fn set_feedback_starred(
    connection: &Connection,
    feedback_id: &str,
    starred: bool,
    evidence_refs: Vec<String>,
) -> Result<(CustomerFeedbackView, RealtimeEvent)> {
    ensure!(
        !evidence_refs.is_empty(),
        "feedback starring requires evidence refs"
    );
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE customer_feedback SET is_starred = ?1, updated_at = ?2 WHERE id = ?3",
        params![if starred { 1 } else { 0 }, now, feedback_id],
    )?;
    let feedback = load_feedback(connection, feedback_id)?;
    let event_type = if starred {
        "feedback.item.starred"
    } else {
        "feedback.item.unstarred"
    };
    let event = append_realtime_event(
        connection,
        &system_event(
            event_type,
            json!({
                "feedbackId": feedback.id,
                "staffSignalOnly": true,
                "notCustomerRating": true,
                "evidenceRefs": evidence_refs,
            }),
        ),
    )?;
    Ok((feedback, event))
}

pub fn propose_feedback_tag(
    connection: &Connection,
    feedback_id: &str,
    input: FeedbackTagInput,
) -> Result<(FeedbackTagView, RealtimeEvent)> {
    ensure_feedback_exists(connection, feedback_id)?;
    validate_tag_input(&input)?;
    let now = Utc::now().to_rfc3339();
    let id = format!("feedback_tag_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO feedback_tags (
            id, feedback_id, tag, candidate_state, confidence, evidence_refs_json,
            provenance_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, 'proposed', ?4, ?5, ?6, ?7, ?7)
         ON CONFLICT(feedback_id, tag) DO UPDATE SET
            candidate_state = 'proposed',
            confidence = excluded.confidence,
            evidence_refs_json = excluded.evidence_refs_json,
            provenance_json = excluded.provenance_json,
            updated_at = excluded.updated_at",
        params![
            id,
            feedback_id,
            input.tag,
            input.confidence,
            json!(input.evidence_refs).to_string(),
            input.provenance.to_string(),
            now,
        ],
    )?;
    let tag = load_feedback_tag(connection, feedback_id, &input.tag)?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "feedback.item.tagged",
            json!({
                "feedbackId": feedback_id,
                "tag": tag.tag,
                "candidateState": tag.candidate_state,
                "evidenceRefs": tag.evidence_refs,
            }),
        ),
    )?;
    Ok((tag, event))
}

pub fn transition_feedback_tag(
    connection: &Connection,
    tag_id: &str,
    new_state: &str,
    reason: &str,
) -> Result<FeedbackTagView> {
    ensure!(
        matches!(new_state, "confirmed" | "rejected" | "superseded"),
        "unsupported feedback tag state"
    );
    require_text("feedback tag state transition reason", reason)?;
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE feedback_tags
         SET candidate_state = ?1, state_changed_at = ?2, state_reason = ?3, updated_at = ?2
         WHERE id = ?4",
        params![new_state, now, reason, tag_id],
    )?;
    load_feedback_tag_by_id(connection, tag_id)
}

pub fn create_review_candidate(
    connection: &Connection,
    feedback_id: &str,
    input: ReviewCandidateInput,
) -> Result<(CustomerReviewView, RealtimeEvent)> {
    let feedback = load_feedback(connection, feedback_id)?;
    validate_review_input(&input)?;
    let now = Utc::now().to_rfc3339();
    let id = format!("review_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO customer_reviews (
            id, feedback_id, connection_id, conversation_id, status, review_body,
            publication_visibility, consent_evidence_refs_json, approval_evidence_refs_json,
            evidence_refs_json, provenance_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, 'candidate', ?5, 'private_until_approved',
            '[]', '[]', ?6, ?7, ?8, ?8)",
        params![
            id,
            feedback_id,
            feedback.connection_id,
            feedback.conversation_id,
            input.review_body,
            json!(input.evidence_refs).to_string(),
            input.provenance.to_string(),
            now,
        ],
    )?;
    let review = load_review(connection, &id)?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "feedback.item.review_candidate.marked",
            json!({
                "feedbackId": feedback_id,
                "reviewId": review.id,
                "status": review.status.as_str(),
                "evidenceRefs": review.evidence_refs,
            }),
        ),
    )?;
    Ok((review, event))
}

pub fn transition_review(
    connection: &Connection,
    review_id: &str,
    next_status: ReviewStatus,
    evidence_refs: Vec<String>,
    reason: &str,
) -> Result<(CustomerReviewView, RealtimeEvent)> {
    require_text("review transition reason", reason)?;
    ensure!(
        !evidence_refs.is_empty(),
        "review transition requires evidence refs"
    );
    let current = load_review(connection, review_id)?;
    ensure!(
        valid_review_transition(current.status, next_status),
        "Invalid review transition from {} to {}",
        current.status.as_str(),
        next_status.as_str()
    );
    if matches!(
        next_status,
        ReviewStatus::Published | ReviewStatus::Featured
    ) {
        ensure!(
            !current.consent_evidence_refs.is_empty()
                || matches!(
                    current.status,
                    ReviewStatus::ConsentConfirmed
                        | ReviewStatus::Approved
                        | ReviewStatus::Published
                ),
            "review publication requires consent evidence"
        );
        ensure!(
            !current.approval_evidence_refs.is_empty()
                || matches!(
                    current.status,
                    ReviewStatus::Approved | ReviewStatus::Published
                ),
            "review publication requires approval evidence"
        );
    }

    let now = Utc::now().to_rfc3339();
    let consent_refs = if next_status == ReviewStatus::ConsentConfirmed {
        evidence_refs.clone()
    } else {
        current.consent_evidence_refs.clone()
    };
    let approval_refs = if next_status == ReviewStatus::Approved {
        evidence_refs.clone()
    } else {
        current.approval_evidence_refs.clone()
    };
    connection.execute(
        "UPDATE customer_reviews
         SET status = ?1,
             publication_visibility = ?2,
             consent_evidence_refs_json = ?3,
             approval_evidence_refs_json = ?4,
             updated_at = ?5,
             published_at = CASE WHEN ?1 = 'published' AND published_at IS NULL THEN ?5 ELSE published_at END,
             featured_at = CASE WHEN ?1 = 'featured' AND featured_at IS NULL THEN ?5 ELSE featured_at END,
             retired_at = CASE WHEN ?1 = 'retired' AND retired_at IS NULL THEN ?5 ELSE retired_at END
         WHERE id = ?6",
        params![
            next_status.as_str(),
            if matches!(next_status, ReviewStatus::Published | ReviewStatus::Featured) {
                "public_review"
            } else {
                "private_until_approved"
            },
            json!(consent_refs).to_string(),
            json!(approval_refs).to_string(),
            now,
            review_id,
        ],
    )?;
    let review = load_review(connection, review_id)?;
    let event_type = format!("review.{}", next_status.as_str().replace('_', "."));
    let event = append_realtime_event(
        connection,
        &system_event(
            &event_type,
            json!({
                "reviewId": review.id,
                "feedbackId": review.feedback_id,
                "status": review.status.as_str(),
                "publicationVisibility": review.publication_visibility,
                "reason": reason,
                "evidenceRefs": evidence_refs,
            }),
        ),
    )?;
    Ok((review, event))
}

pub fn list_private_feedback(
    connection: &Connection,
    conversation_id: &str,
) -> Result<Vec<CustomerFeedbackView>> {
    connection.query_many(
        "SELECT id, connection_id, conversation_id, segment_id, message_id, feedback_kind,
                status, visibility, body_summary, is_starred, source_refs_json,
                evidence_refs_json, provenance_json, created_at, updated_at
         FROM customer_feedback
         WHERE conversation_id = ?1 AND visibility = 'private_business_intelligence'
         ORDER BY updated_at DESC",
        [conversation_id],
        feedback_from_row,
    )
}

pub fn list_public_reviews(connection: &Connection) -> Result<Vec<CustomerReviewView>> {
    connection.query_many(
        "SELECT id, feedback_id, connection_id, conversation_id, status, review_body,
                publication_visibility, consent_evidence_refs_json,
                approval_evidence_refs_json, evidence_refs_json, provenance_json,
                created_at, updated_at, published_at, featured_at, retired_at
         FROM customer_reviews
         WHERE publication_visibility = 'public_review'
           AND status IN ('published', 'featured')
         ORDER BY updated_at DESC",
        [],
        review_from_row,
    )
}

pub fn list_feedback_requests(
    db_path: &Path,
    query: FeedbackRequestQuery,
) -> Result<FeedbackRequestListResponse> {
    let connection = Connection::open(db_path)?;
    list_feedback_requests_in_connection(&connection, query)
}

pub fn create_feedback_request(
    db_path: &Path,
    input: FeedbackRequestCreateRequest,
    actor_id: Option<&str>,
) -> Result<(FeedbackRequestView, RealtimeEvent)> {
    let connection = Connection::open(db_path)?;
    create_feedback_request_in_connection(&connection, input, actor_id)
}

pub fn respond_to_feedback_request(
    db_path: &Path,
    request_id: &str,
    input: FeedbackRequestRespondRequest,
    actor_id: Option<&str>,
) -> Result<(FeedbackRequestView, RealtimeEvent)> {
    let connection = Connection::open(db_path)?;
    respond_to_feedback_request_in_connection(&connection, request_id, input, actor_id)
}

pub fn review_feedback_request(
    db_path: &Path,
    request_id: &str,
    input: FeedbackRequestReviewRequest,
    actor_id: Option<&str>,
) -> Result<(FeedbackRequestView, RealtimeEvent)> {
    let connection = Connection::open(db_path)?;
    review_feedback_request_in_connection(&connection, request_id, input, actor_id)
}

pub fn list_feedback_requests_in_connection(
    connection: &Connection,
    query: FeedbackRequestQuery,
) -> Result<FeedbackRequestListResponse> {
    let mut statement = connection.prepare(
        "SELECT id, target_kind, target_id, member_actor_id, connection_id,
                conversation_id, source_kind, source_id, prompt, member_context_summary,
                status, due_at, priority, evidence_refs_json, created_at, updated_at
         FROM feedback_requests
         ORDER BY updated_at DESC, id ASC",
    )?;
    let records = statement
        .query_map([], feedback_request_record_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let limit = query.limit.unwrap_or(100).min(500);
    let requests = records
        .into_iter()
        .filter(|record| feedback_request_matches_query(record, &query))
        .take(limit)
        .map(|record| feedback_request_view(connection, record, query.viewer, &query))
        .collect::<Result<Vec<_>>>()?;
    Ok(FeedbackRequestListResponse { requests })
}

pub fn create_feedback_request_in_connection(
    connection: &Connection,
    input: FeedbackRequestCreateRequest,
    actor_id: Option<&str>,
) -> Result<(FeedbackRequestView, RealtimeEvent)> {
    validate_feedback_request_create(&input)?;
    let now = Utc::now().to_rfc3339();
    let id = format!("feedback_request_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO feedback_requests (
            id, target_kind, target_id, member_actor_id, connection_id, conversation_id,
            source_kind, source_id, prompt, member_context_summary, status, due_at,
            priority, created_by_actor_id, evidence_refs_json, provenance_json,
            staff_context_json, created_at, updated_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'open', ?11, ?12, ?13,
            ?14, ?15, ?16, ?17, ?17
         )",
        params![
            id,
            normalize_kind(&input.target_kind),
            input.target_id.trim(),
            input.member_actor_id.as_deref().map(str::trim),
            input.connection_id.as_deref().map(str::trim),
            input.conversation_id.as_deref().map(str::trim),
            normalize_kind(&input.source_kind),
            input.source_id.as_deref().map(str::trim),
            input.prompt.trim(),
            input.member_context_summary.trim(),
            input.due_at.as_deref().map(str::trim),
            input.priority.as_deref().unwrap_or("normal").trim(),
            actor_id,
            json!(input.evidence_refs).to_string(),
            input.provenance.to_string(),
            input.staff_context.to_string(),
            now,
        ],
    )?;
    let view = load_feedback_request_view(
        connection,
        &id,
        FeedbackRequestViewer::Staff,
        &FeedbackRequestQuery {
            viewer: FeedbackRequestViewer::Staff,
            ..FeedbackRequestQuery::default()
        },
    )?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "feedback.request.created",
            json!({
                "requestId": view.id,
                "targetKind": view.target_kind,
                "targetId": view.target_id,
                "status": view.status.as_str(),
                "evidenceRefs": view.evidence_refs,
            }),
        ),
    )?;
    Ok((view, event))
}

pub fn respond_to_feedback_request_in_connection(
    connection: &Connection,
    request_id: &str,
    input: FeedbackRequestRespondRequest,
    actor_id: Option<&str>,
) -> Result<(FeedbackRequestView, RealtimeEvent)> {
    validate_feedback_request_response(&input)?;
    if let Some(key) = normalized_optional(input.idempotency_key.as_deref()) {
        if let Some(existing) = load_response_by_idempotency_key(connection, request_id, &key)? {
            let view = load_feedback_request_view(
                connection,
                request_id,
                FeedbackRequestViewer::Staff,
                &FeedbackRequestQuery {
                    viewer: FeedbackRequestViewer::Staff,
                    ..FeedbackRequestQuery::default()
                },
            )?;
            let event = append_realtime_event(
                connection,
                &system_event(
                    "feedback.request.response.replayed",
                    json!({
                        "requestId": request_id,
                        "responseId": existing.id,
                        "idempotencyKey": key,
                    }),
                ),
            )?;
            return Ok((view, event));
        }
    }

    let record = load_feedback_request_record(connection, request_id)?;
    ensure!(
        matches!(
            record.status,
            FeedbackRequestStatus::Open | FeedbackRequestStatus::FollowUpRequested
        ),
        "feedback request is not waiting for a member response"
    );
    let now = Utc::now().to_rfc3339();
    let response_kind = input
        .response_kind
        .as_deref()
        .map(normalize_kind)
        .unwrap_or_else(|| "answer".to_string());
    let response_id = format!("feedback_request_response_{}", Uuid::new_v4());
    let customer_feedback_id = if let Some(conversation_id) = record.conversation_id.as_deref() {
        let (feedback, _) = capture_feedback(
            connection,
            CustomerFeedbackInput {
                connection_id: record.connection_id.clone(),
                conversation_id: conversation_id.to_string(),
                segment_id: None,
                message_id: None,
                feedback_kind: response_kind.clone(),
                body_summary: input.body_summary.trim().to_string(),
                source_refs: vec![
                    format!("feedback_request:{request_id}"),
                    format!("{}:{}", record.target_kind, record.target_id),
                ],
                evidence_refs: input.evidence_refs.clone(),
                provenance: json!({
                    "source": "feedback_request",
                    "requestId": request_id,
                    "responseProvenance": input.provenance.clone(),
                }),
            },
        )?;
        Some(feedback.id)
    } else {
        None
    };
    let idempotency_key = normalized_optional(input.idempotency_key.as_deref());
    connection.execute(
        "INSERT INTO feedback_request_responses (
            id, request_id, actor_id, response_kind, body_summary, customer_feedback_id,
            idempotency_key, evidence_refs_json, provenance_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
            response_id,
            request_id,
            actor_id,
            response_kind,
            input.body_summary.trim(),
            customer_feedback_id,
            idempotency_key,
            json!(input.evidence_refs).to_string(),
            input.provenance.to_string(),
            now,
        ],
    )?;
    connection.execute(
        "UPDATE feedback_requests
         SET status = 'responded', updated_at = ?1
         WHERE id = ?2",
        params![now, request_id],
    )?;
    let view = load_feedback_request_view(
        connection,
        request_id,
        FeedbackRequestViewer::Staff,
        &FeedbackRequestQuery {
            viewer: FeedbackRequestViewer::Staff,
            ..FeedbackRequestQuery::default()
        },
    )?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "feedback.request.responded",
            json!({
                "requestId": request_id,
                "responseId": response_id,
                "status": view.status.as_str(),
                "evidenceRefs": input.evidence_refs,
            }),
        ),
    )?;
    Ok((view, event))
}

pub fn review_feedback_request_in_connection(
    connection: &Connection,
    request_id: &str,
    input: FeedbackRequestReviewRequest,
    actor_id: Option<&str>,
) -> Result<(FeedbackRequestView, RealtimeEvent)> {
    validate_feedback_request_review(&input)?;
    let record = load_feedback_request_record(connection, request_id)?;
    ensure!(
        !record.status.terminal(),
        "feedback request already has a terminal review decision"
    );
    let response_id = match input.decision {
        FeedbackRequestReviewDecision::Accepted | FeedbackRequestReviewDecision::Rejected => input
            .response_id
            .clone()
            .or_else(|| latest_response_id(connection, request_id).ok().flatten())
            .ok_or_else(|| anyhow::anyhow!("review decision requires a response"))
            .map(Some)?,
        FeedbackRequestReviewDecision::FollowUpRequested => input.response_id.clone(),
    };
    if let Some(response_id) = response_id.as_deref() {
        ensure_response_belongs_to_request(connection, request_id, response_id)?;
    }

    let now = Utc::now().to_rfc3339();
    let review_id = format!("feedback_request_review_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO feedback_request_reviews (
            id, request_id, response_id, reviewer_actor_id, decision, tags_json, reason,
            evidence_refs_json, provenance_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
            review_id,
            request_id,
            response_id,
            actor_id,
            input.decision.as_str(),
            json!(input.tags).to_string(),
            input.reason.trim(),
            json!(input.evidence_refs).to_string(),
            input.provenance.to_string(),
            now,
        ],
    )?;
    connection.execute(
        "UPDATE feedback_requests
         SET status = ?1,
             updated_at = ?2,
             closed_at = CASE WHEN ?1 IN ('accepted', 'rejected') THEN ?2 ELSE closed_at END
         WHERE id = ?3",
        params![input.decision.as_str(), now, request_id],
    )?;
    record_reward_eligibility(
        connection,
        request_id,
        response_id.as_deref(),
        &review_id,
        actor_id,
        input.decision,
        &input.evidence_refs,
        &now,
    )?;
    let view = load_feedback_request_view(
        connection,
        request_id,
        FeedbackRequestViewer::Staff,
        &FeedbackRequestQuery {
            viewer: FeedbackRequestViewer::Staff,
            ..FeedbackRequestQuery::default()
        },
    )?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "feedback.request.reviewed",
            json!({
                "requestId": request_id,
                "reviewId": review_id,
                "decision": input.decision.as_str(),
                "status": view.status.as_str(),
                "rewardLedgerDeferred": true,
                "evidenceRefs": input.evidence_refs,
            }),
        ),
    )?;
    Ok((view, event))
}

fn load_feedback(connection: &Connection, feedback_id: &str) -> Result<CustomerFeedbackView> {
    connection
        .query_row(
            "SELECT id, connection_id, conversation_id, segment_id, message_id, feedback_kind,
                    status, visibility, body_summary, is_starred, source_refs_json,
                    evidence_refs_json, provenance_json, created_at, updated_at
             FROM customer_feedback WHERE id = ?1",
            [feedback_id],
            feedback_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("Customer feedback was not found: {feedback_id}"))
}

fn load_feedback_tag(
    connection: &Connection,
    feedback_id: &str,
    tag: &str,
) -> Result<FeedbackTagView> {
    connection
        .query_row(
            "SELECT id, feedback_id, tag, candidate_state, confidence, evidence_refs_json,
                    provenance_json, created_at, updated_at, state_changed_at, state_reason
             FROM feedback_tags WHERE feedback_id = ?1 AND tag = ?2",
            params![feedback_id, tag],
            feedback_tag_from_row,
        )
        .map_err(Into::into)
}

fn load_feedback_tag_by_id(connection: &Connection, tag_id: &str) -> Result<FeedbackTagView> {
    connection
        .query_row(
            "SELECT id, feedback_id, tag, candidate_state, confidence, evidence_refs_json,
                    provenance_json, created_at, updated_at, state_changed_at, state_reason
             FROM feedback_tags WHERE id = ?1",
            [tag_id],
            feedback_tag_from_row,
        )
        .map_err(Into::into)
}

fn load_review(connection: &Connection, review_id: &str) -> Result<CustomerReviewView> {
    connection
        .query_row(
            "SELECT id, feedback_id, connection_id, conversation_id, status, review_body,
                    publication_visibility, consent_evidence_refs_json,
                    approval_evidence_refs_json, evidence_refs_json, provenance_json,
                    created_at, updated_at, published_at, featured_at, retired_at
             FROM customer_reviews WHERE id = ?1",
            [review_id],
            review_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("Customer review was not found: {review_id}"))
}

fn ensure_feedback_exists(connection: &Connection, feedback_id: &str) -> Result<()> {
    load_feedback(connection, feedback_id).map(|_| ())
}

fn load_feedback_request_record(
    connection: &Connection,
    request_id: &str,
) -> Result<FeedbackRequestRecord> {
    connection
        .query_row(
            "SELECT id, target_kind, target_id, member_actor_id, connection_id,
                    conversation_id, source_kind, source_id, prompt, member_context_summary,
                    status, due_at, priority, evidence_refs_json, created_at, updated_at
             FROM feedback_requests WHERE id = ?1",
            [request_id],
            feedback_request_record_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("Feedback request was not found: {request_id}"))
}

fn load_feedback_request_view(
    connection: &Connection,
    request_id: &str,
    viewer: FeedbackRequestViewer,
    query: &FeedbackRequestQuery,
) -> Result<FeedbackRequestView> {
    let record = load_feedback_request_record(connection, request_id)?;
    feedback_request_view(connection, record, viewer, query)
}

fn feedback_request_record_from_row(row: &Row<'_>) -> rusqlite::Result<FeedbackRequestRecord> {
    let status_raw: String = row.get(10)?;
    let evidence_refs_json: String = row.get(13)?;
    Ok(FeedbackRequestRecord {
        id: row.get(0)?,
        target_kind: row.get(1)?,
        target_id: row.get(2)?,
        member_actor_id: row.get(3)?,
        connection_id: row.get(4)?,
        conversation_id: row.get(5)?,
        source_kind: row.get(6)?,
        source_id: row.get(7)?,
        prompt: row.get(8)?,
        member_context_summary: row.get(9)?,
        status: FeedbackRequestStatus::try_from(status_raw.as_str()).map_err(to_sql_error)?,
        due_at: row.get(11)?,
        priority: row.get(12)?,
        evidence_refs: parse_string_vec(&evidence_refs_json),
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn feedback_request_view(
    connection: &Connection,
    record: FeedbackRequestRecord,
    viewer: FeedbackRequestViewer,
    query: &FeedbackRequestQuery,
) -> Result<FeedbackRequestView> {
    let safe_member_view = matches!(
        viewer,
        FeedbackRequestViewer::Member | FeedbackRequestViewer::Public
    );
    let growth_view = viewer == FeedbackRequestViewer::Growth;
    let responses = if safe_member_view {
        load_feedback_request_responses(connection, &record.id)?
            .into_iter()
            .filter(|response| response_matches_member_query(response, query))
            .collect()
    } else if growth_view {
        Vec::new()
    } else {
        load_feedback_request_responses(connection, &record.id)?
    };
    let reviews = if safe_member_view || growth_view {
        Vec::new()
    } else {
        load_feedback_request_reviews(connection, &record.id)?
    };
    let reward_eligibility =
        load_feedback_reward_eligibility(connection, &record.id, safe_member_view || growth_view)?;
    let evidence_refs = if safe_member_view {
        vec![format!("feedback_request:{}", record.id)]
    } else {
        record.evidence_refs.clone()
    };
    let prompt = if growth_view {
        String::new()
    } else {
        record.prompt.clone()
    };
    let member_context_summary = if growth_view {
        String::new()
    } else {
        record.member_context_summary.clone()
    };

    Ok(FeedbackRequestView {
        id: record.id,
        target_kind: record.target_kind,
        target_id: record.target_id,
        member_actor_id: record.member_actor_id,
        connection_id: record.connection_id,
        conversation_id: record.conversation_id,
        source_kind: record.source_kind,
        source_id: record.source_id,
        prompt,
        member_context_summary,
        status: record.status,
        due_at: record.due_at,
        priority: record.priority,
        evidence_refs,
        responses,
        reviews,
        reward_eligibility,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

fn load_feedback_request_responses(
    connection: &Connection,
    request_id: &str,
) -> Result<Vec<FeedbackRequestResponseView>> {
    connection.query_many(
        "SELECT id, request_id, actor_id, response_kind, body_summary, customer_feedback_id,
                evidence_refs_json, created_at, updated_at
         FROM feedback_request_responses
         WHERE request_id = ?1
         ORDER BY created_at ASC, id ASC",
        [request_id],
        feedback_request_response_from_row,
    )
}

fn load_response_by_idempotency_key(
    connection: &Connection,
    request_id: &str,
    idempotency_key: &str,
) -> Result<Option<FeedbackRequestResponseView>> {
    connection
        .query_row(
            "SELECT id, request_id, actor_id, response_kind, body_summary, customer_feedback_id,
                    evidence_refs_json, created_at, updated_at
             FROM feedback_request_responses
             WHERE request_id = ?1 AND idempotency_key = ?2",
            params![request_id, idempotency_key],
            feedback_request_response_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn feedback_request_response_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<FeedbackRequestResponseView> {
    let evidence_refs_json: String = row.get(6)?;
    Ok(FeedbackRequestResponseView {
        id: row.get(0)?,
        request_id: row.get(1)?,
        actor_id: row.get(2)?,
        response_kind: row.get(3)?,
        body_summary: row.get(4)?,
        customer_feedback_id: row.get(5)?,
        evidence_refs: parse_string_vec(&evidence_refs_json),
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn load_feedback_request_reviews(
    connection: &Connection,
    request_id: &str,
) -> Result<Vec<FeedbackRequestReviewView>> {
    connection.query_many(
        "SELECT id, request_id, response_id, decision, tags_json, reason,
                evidence_refs_json, created_at, updated_at
         FROM feedback_request_reviews
         WHERE request_id = ?1
         ORDER BY created_at ASC, id ASC",
        [request_id],
        feedback_request_review_from_row,
    )
}

fn feedback_request_review_from_row(row: &Row<'_>) -> rusqlite::Result<FeedbackRequestReviewView> {
    let decision_raw: String = row.get(3)?;
    let tags_json: String = row.get(4)?;
    let evidence_refs_json: String = row.get(6)?;
    Ok(FeedbackRequestReviewView {
        id: row.get(0)?,
        request_id: row.get(1)?,
        response_id: row.get(2)?,
        decision: FeedbackRequestReviewDecision::try_from(decision_raw.as_str())
            .map_err(to_sql_error)?,
        tags: parse_string_vec(&tags_json),
        reason: Some(row.get(5)?),
        evidence_refs: parse_string_vec(&evidence_refs_json),
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn load_feedback_reward_eligibility(
    connection: &Connection,
    request_id: &str,
    safe_reason: bool,
) -> Result<Option<FeedbackRewardEligibilityView>> {
    connection
        .query_row(
            "SELECT id, request_id, response_id, review_id, actor_id, state, reason,
                    evidence_refs_json, created_at, updated_at
             FROM feedback_reward_eligibility
             WHERE request_id = ?1
             ORDER BY updated_at DESC, id ASC
             LIMIT 1",
            [request_id],
            |row| feedback_reward_eligibility_from_row(row, safe_reason),
        )
        .optional()
        .map_err(Into::into)
}

fn feedback_reward_eligibility_from_row(
    row: &Row<'_>,
    safe_reason: bool,
) -> rusqlite::Result<FeedbackRewardEligibilityView> {
    let evidence_refs_json: String = row.get(7)?;
    Ok(FeedbackRewardEligibilityView {
        id: row.get(0)?,
        request_id: row.get(1)?,
        response_id: row.get(2)?,
        review_id: row.get(3)?,
        actor_id: row.get(4)?,
        state: row.get(5)?,
        reason: if safe_reason { None } else { Some(row.get(6)?) },
        evidence_refs: if safe_reason {
            Vec::new()
        } else {
            parse_string_vec(&evidence_refs_json)
        },
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn latest_response_id(connection: &Connection, request_id: &str) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT id FROM feedback_request_responses
             WHERE request_id = ?1
             ORDER BY updated_at DESC, id DESC
             LIMIT 1",
            [request_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

fn ensure_response_belongs_to_request(
    connection: &Connection,
    request_id: &str,
    response_id: &str,
) -> Result<()> {
    let exists: Option<String> = connection
        .query_row(
            "SELECT id FROM feedback_request_responses WHERE id = ?1 AND request_id = ?2",
            params![response_id, request_id],
            |row| row.get(0),
        )
        .optional()?;
    ensure!(
        exists.is_some(),
        "feedback response does not belong to request"
    );
    Ok(())
}

fn record_reward_eligibility(
    connection: &Connection,
    request_id: &str,
    response_id: Option<&str>,
    review_id: &str,
    actor_id: Option<&str>,
    decision: FeedbackRequestReviewDecision,
    evidence_refs: &[String],
    now: &str,
) -> Result<()> {
    let (state, reason) = match decision {
        FeedbackRequestReviewDecision::Accepted => (
            "pending_qualification",
            "Feedback was accepted; reward grant is deferred until issue #248 adds the reward ledger.",
        ),
        FeedbackRequestReviewDecision::Rejected => (
            "not_qualified",
            "Feedback was rejected by Support review evidence.",
        ),
        FeedbackRequestReviewDecision::FollowUpRequested => (
            "needs_follow_up",
            "Support requested follow-up before reward qualification.",
        ),
    };
    let id = format!("feedback_reward_eligibility_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO feedback_reward_eligibility (
            id, request_id, response_id, review_id, actor_id, state, reason,
            evidence_refs_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
         ON CONFLICT(request_id, response_id) DO UPDATE SET
            review_id = excluded.review_id,
            actor_id = excluded.actor_id,
            state = excluded.state,
            reason = excluded.reason,
            evidence_refs_json = excluded.evidence_refs_json,
            updated_at = excluded.updated_at",
        params![
            id,
            request_id,
            response_id,
            review_id,
            actor_id,
            state,
            reason,
            json!(evidence_refs).to_string(),
            now,
        ],
    )?;
    Ok(())
}

fn feedback_request_matches_query(
    record: &FeedbackRequestRecord,
    query: &FeedbackRequestQuery,
) -> bool {
    if matches!(query.viewer, FeedbackRequestViewer::Public) {
        return false;
    }
    if query
        .target_kind
        .as_deref()
        .is_some_and(|target_kind| normalize_kind(target_kind) != record.target_kind)
    {
        return false;
    }
    if query
        .status
        .as_deref()
        .is_some_and(|status| normalize_kind(status) != record.status.as_str())
    {
        return false;
    }
    if matches!(query.viewer, FeedbackRequestViewer::Member) {
        return member_request_matches_query(record, query);
    }
    true
}

fn member_request_matches_query(
    record: &FeedbackRequestRecord,
    query: &FeedbackRequestQuery,
) -> bool {
    query.actor_id.as_deref().is_some_and(|actor_id| {
        record
            .member_actor_id
            .as_deref()
            .is_some_and(|member_actor_id| member_actor_id == actor_id)
    }) || query.connection_id.as_deref().is_some_and(|connection_id| {
        record
            .connection_id
            .as_deref()
            .is_some_and(|record_connection_id| record_connection_id == connection_id)
    })
}

fn response_matches_member_query(
    response: &FeedbackRequestResponseView,
    query: &FeedbackRequestQuery,
) -> bool {
    query.actor_id.as_deref().is_some_and(|actor_id| {
        response
            .actor_id
            .as_deref()
            .is_some_and(|response_actor_id| response_actor_id == actor_id)
    }) || query.actor_id.is_none()
}

fn validate_feedback_request_create(input: &FeedbackRequestCreateRequest) -> Result<()> {
    require_text("target_kind", &input.target_kind)?;
    require_text("target_id", &input.target_id)?;
    require_text("source_kind", &input.source_kind)?;
    require_text("prompt", &input.prompt)?;
    require_text("member_context_summary", &input.member_context_summary)?;
    ensure!(
        input.member_actor_id.as_deref().is_some_and(has_text)
            || input.connection_id.as_deref().is_some_and(has_text),
        "feedback request requires a member actor or connection assignment"
    );
    ensure!(
        matches!(
            normalize_kind(&input.target_kind).as_str(),
            "account"
                | "member"
                | "trial"
                | "hosted_trial"
                | "artifact"
                | "offer"
                | "workflow"
                | "conversation"
        ),
        "unsupported feedback request target kind"
    );
    ensure_safe_member_text("prompt", &input.prompt)?;
    ensure_safe_member_text("member_context_summary", &input.member_context_summary)?;
    validate_evidence_and_provenance(&input.evidence_refs, &input.provenance)
}

fn validate_feedback_request_response(input: &FeedbackRequestRespondRequest) -> Result<()> {
    require_text("body_summary", &input.body_summary)?;
    if let Some(response_kind) = input.response_kind.as_deref() {
        require_text("response_kind", response_kind)?;
    }
    validate_evidence_and_provenance(&input.evidence_refs, &input.provenance)
}

fn validate_feedback_request_review(input: &FeedbackRequestReviewRequest) -> Result<()> {
    require_text("review reason", &input.reason)?;
    ensure!(
        !input.evidence_refs.is_empty(),
        "feedback request review requires evidence refs"
    );
    ensure!(
        input
            .provenance
            .as_object()
            .is_some_and(|object| !object.is_empty()),
        "feedback request review provenance is required"
    );
    for tag in &input.tags {
        require_text("feedback request review tag", tag)?;
    }
    Ok(())
}

fn ensure_safe_member_text(label: &str, value: &str) -> Result<()> {
    let normalized = value.to_ascii_lowercase().replace(['_', '-', ' '], "");
    for blocked in [
        "apikey",
        "password",
        "providersecret",
        "rawprompt",
        "secret",
        "staffnote",
        "policyinternal",
        "token",
    ] {
        ensure!(
            !normalized.contains(blocked),
            "{label} contains staff-only or secret-bearing context"
        );
    }
    Ok(())
}

fn has_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn normalize_kind(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('-', "_")
}

fn normalized_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn parse_string_vec(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

fn feedback_from_row(row: &Row<'_>) -> rusqlite::Result<CustomerFeedbackView> {
    let source_refs_json: String = row.get(10)?;
    let evidence_refs_json: String = row.get(11)?;
    let provenance_json: String = row.get(12)?;
    Ok(CustomerFeedbackView {
        id: row.get(0)?,
        connection_id: row.get(1)?,
        conversation_id: row.get(2)?,
        segment_id: row.get(3)?,
        message_id: row.get(4)?,
        feedback_kind: row.get(5)?,
        status: row.get(6)?,
        visibility: row.get(7)?,
        body_summary: row.get(8)?,
        is_starred: row.get::<_, i64>(9)? == 1,
        source_refs: serde_json::from_str(&source_refs_json).unwrap_or_default(),
        evidence_refs: serde_json::from_str(&evidence_refs_json).unwrap_or_default(),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn feedback_tag_from_row(row: &Row<'_>) -> rusqlite::Result<FeedbackTagView> {
    let evidence_refs_json: String = row.get(5)?;
    let provenance_json: String = row.get(6)?;
    Ok(FeedbackTagView {
        id: row.get(0)?,
        feedback_id: row.get(1)?,
        tag: row.get(2)?,
        candidate_state: row.get(3)?,
        confidence: row.get(4)?,
        evidence_refs: serde_json::from_str(&evidence_refs_json).unwrap_or_default(),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        state_changed_at: row.get(9)?,
        state_reason: row.get(10)?,
    })
}

fn review_from_row(row: &Row<'_>) -> rusqlite::Result<CustomerReviewView> {
    let status_raw: String = row.get(4)?;
    let consent_refs_json: String = row.get(7)?;
    let approval_refs_json: String = row.get(8)?;
    let evidence_refs_json: String = row.get(9)?;
    let provenance_json: String = row.get(10)?;
    Ok(CustomerReviewView {
        id: row.get(0)?,
        feedback_id: row.get(1)?,
        connection_id: row.get(2)?,
        conversation_id: row.get(3)?,
        status: ReviewStatus::try_from(status_raw.as_str()).map_err(to_sql_error)?,
        review_body: row.get(5)?,
        publication_visibility: row.get(6)?,
        consent_evidence_refs: serde_json::from_str(&consent_refs_json).unwrap_or_default(),
        approval_evidence_refs: serde_json::from_str(&approval_refs_json).unwrap_or_default(),
        evidence_refs: serde_json::from_str(&evidence_refs_json).unwrap_or_default(),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        published_at: row.get(13)?,
        featured_at: row.get(14)?,
        retired_at: row.get(15)?,
    })
}

fn validate_feedback_input(input: &CustomerFeedbackInput) -> Result<()> {
    require_text("conversation_id", &input.conversation_id)?;
    require_text("feedback_kind", &input.feedback_kind)?;
    require_text("body_summary", &input.body_summary)?;
    validate_evidence_and_provenance(&input.evidence_refs, &input.provenance)
}

fn validate_tag_input(input: &FeedbackTagInput) -> Result<()> {
    require_text("feedback tag", &input.tag)?;
    ensure!(
        (0.0..=1.0).contains(&input.confidence),
        "feedback tag confidence must be between 0 and 1"
    );
    validate_evidence_and_provenance(&input.evidence_refs, &input.provenance)
}

fn validate_review_input(input: &ReviewCandidateInput) -> Result<()> {
    require_text("review body", &input.review_body)?;
    validate_evidence_and_provenance(&input.evidence_refs, &input.provenance)
}

fn validate_evidence_and_provenance(evidence_refs: &[String], provenance: &Value) -> Result<()> {
    ensure!(!evidence_refs.is_empty(), "evidence refs are required");
    ensure!(
        provenance
            .as_object()
            .is_some_and(|object| !object.is_empty()),
        "provenance is required"
    );
    Ok(())
}

fn valid_review_transition(from: ReviewStatus, to: ReviewStatus) -> bool {
    matches!(
        (from, to),
        (ReviewStatus::Candidate, ReviewStatus::Requested)
            | (ReviewStatus::Requested, ReviewStatus::Received)
            | (ReviewStatus::Received, ReviewStatus::ConsentConfirmed)
            | (ReviewStatus::ConsentConfirmed, ReviewStatus::Approved)
            | (ReviewStatus::Approved, ReviewStatus::Published)
            | (ReviewStatus::Published, ReviewStatus::Featured)
            | (ReviewStatus::Published, ReviewStatus::Retired)
            | (ReviewStatus::Featured, ReviewStatus::Retired)
    )
}

fn require_text(label: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{label} is required");
    Ok(())
}

fn to_sql_error(error: anyhow::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::<dyn std::error::Error + Send + Sync>::from(error.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        crate::schema::init_schema(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO conversations (
                    id, surface, subject_kind, subject_id, connection_id, visitor_session_id,
                    status, visibility, privacy_scope, summary_json, unread_count, action_count,
                    last_meaningful_change, created_by_actor_id, created_at, updated_at
                 ) VALUES (
                    'conversation_1', 'client_portal', 'connection', 'connection_1',
                    NULL, NULL, 'active', 'participants', 'connection', '{}', 0, 0, 'created',
                    NULL, 'now', 'now'
                 )",
                [],
            )
            .unwrap();
        connection
    }

    #[test]
    fn feedback_requires_evidence_and_remains_private() {
        let connection = connection();
        let err = capture_feedback(
            &connection,
            CustomerFeedbackInput {
                connection_id: Some("connection_1".to_string()),
                conversation_id: "conversation_1".to_string(),
                segment_id: None,
                message_id: None,
                feedback_kind: "praise".to_string(),
                body_summary: "Client likes the clear scope.".to_string(),
                source_refs: vec!["conversation_1".to_string()],
                evidence_refs: vec![],
                provenance: json!({"source": "test"}),
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("evidence refs"));

        let (feedback, _) = capture_feedback(
            &connection,
            CustomerFeedbackInput {
                connection_id: Some("connection_1".to_string()),
                conversation_id: "conversation_1".to_string(),
                segment_id: None,
                message_id: None,
                feedback_kind: "praise".to_string(),
                body_summary: "Client likes the clear scope.".to_string(),
                source_refs: vec!["conversation_1".to_string()],
                evidence_refs: vec!["message_1".to_string()],
                provenance: json!({"source": "test"}),
            },
        )
        .unwrap();

        assert_eq!(feedback.visibility, "private_business_intelligence");
        assert_eq!(list_public_reviews(&connection).unwrap().len(), 0);
    }

    #[test]
    fn review_publication_requires_consent_and_approval() {
        let connection = connection();
        let (feedback, _) = capture_feedback(
            &connection,
            CustomerFeedbackInput {
                connection_id: Some("connection_1".to_string()),
                conversation_id: "conversation_1".to_string(),
                segment_id: None,
                message_id: None,
                feedback_kind: "praise".to_string(),
                body_summary: "Client says the work made decisions easier.".to_string(),
                source_refs: vec!["conversation_1".to_string()],
                evidence_refs: vec!["message_1".to_string()],
                provenance: json!({"source": "test"}),
            },
        )
        .unwrap();
        let (review, _) = create_review_candidate(
            &connection,
            &feedback.id,
            ReviewCandidateInput {
                review_body: "The work made decisions easier.".to_string(),
                evidence_refs: vec![feedback.id.clone()],
                provenance: json!({"source": "feedback"}),
            },
        )
        .unwrap();

        assert!(transition_review(
            &connection,
            &review.id,
            ReviewStatus::Published,
            vec!["message_1".to_string()],
            "attempt early publish"
        )
        .is_err());

        let (requested, _) = transition_review(
            &connection,
            &review.id,
            ReviewStatus::Requested,
            vec!["request_1".to_string()],
            "request review",
        )
        .unwrap();
        let (received, _) = transition_review(
            &connection,
            &requested.id,
            ReviewStatus::Received,
            vec!["review_response_1".to_string()],
            "review received",
        )
        .unwrap();
        let (consented, _) = transition_review(
            &connection,
            &received.id,
            ReviewStatus::ConsentConfirmed,
            vec!["consent_1".to_string()],
            "consent confirmed",
        )
        .unwrap();
        let (approved, _) = transition_review(
            &connection,
            &consented.id,
            ReviewStatus::Approved,
            vec!["approval_1".to_string()],
            "staff approved",
        )
        .unwrap();
        let (published, _) = transition_review(
            &connection,
            &approved.id,
            ReviewStatus::Published,
            vec!["publish_1".to_string()],
            "publish",
        )
        .unwrap();

        assert_eq!(published.publication_visibility, "public_review");
        assert_eq!(list_public_reviews(&connection).unwrap().len(), 1);
    }

    #[test]
    fn feedback_request_loop_keeps_response_private_and_records_reward_eligibility() {
        let connection = connection();
        let (request, _) = create_feedback_request_in_connection(
            &connection,
            FeedbackRequestCreateRequest {
                target_kind: "trial".to_string(),
                target_id: "trial_1".to_string(),
                member_actor_id: Some("actor_member_1".to_string()),
                connection_id: Some("connection_1".to_string()),
                conversation_id: Some("conversation_1".to_string()),
                source_kind: "support".to_string(),
                source_id: Some("handoff_1".to_string()),
                prompt: "What would make Ordo more useful this week?".to_string(),
                member_context_summary: "Keith is asking for feedback on the NYC pilot trial."
                    .to_string(),
                due_at: Some("2026-05-15T10:00:00Z".to_string()),
                priority: Some("high".to_string()),
                evidence_refs: vec!["handoff:handoff_1".to_string()],
                provenance: json!({"source": "test"}),
                staff_context: json!({"staffNotes": "internal only"}),
            },
            Some("actor_keith"),
        )
        .unwrap();
        assert_eq!(request.status, FeedbackRequestStatus::Open);

        let unscoped_member = list_feedback_requests_in_connection(
            &connection,
            FeedbackRequestQuery {
                viewer: FeedbackRequestViewer::Member,
                ..FeedbackRequestQuery::default()
            },
        )
        .unwrap();
        assert!(unscoped_member.requests.is_empty());

        let (responded, _) = respond_to_feedback_request_in_connection(
            &connection,
            &request.id,
            FeedbackRequestRespondRequest {
                response_kind: Some("answer".to_string()),
                body_summary: "The strategy handoff should say what changed.".to_string(),
                idempotency_key: Some("member-response-1".to_string()),
                evidence_refs: vec!["message:member_response_1".to_string()],
                provenance: json!({"source": "member_chat"}),
            },
            Some("actor_member_1"),
        )
        .unwrap();
        assert_eq!(responded.status, FeedbackRequestStatus::Responded);
        assert_eq!(responded.responses.len(), 1);
        assert!(responded.responses[0].customer_feedback_id.is_some());
        assert_eq!(
            list_private_feedback(&connection, "conversation_1")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(list_public_reviews(&connection).unwrap().len(), 0);

        let (replayed, _) = respond_to_feedback_request_in_connection(
            &connection,
            &request.id,
            FeedbackRequestRespondRequest {
                response_kind: Some("answer".to_string()),
                body_summary: "The strategy handoff should say what changed.".to_string(),
                idempotency_key: Some("member-response-1".to_string()),
                evidence_refs: vec!["message:member_response_1".to_string()],
                provenance: json!({"source": "member_chat"}),
            },
            Some("actor_member_1"),
        )
        .unwrap();
        assert_eq!(replayed.responses.len(), 1);

        let (reviewed, _) = review_feedback_request_in_connection(
            &connection,
            &request.id,
            FeedbackRequestReviewRequest {
                decision: FeedbackRequestReviewDecision::Accepted,
                response_id: None,
                tags: vec!["strategy".to_string(), "pilot-learning".to_string()],
                reason: "Support accepted this as actionable pilot feedback.".to_string(),
                evidence_refs: vec!["support_review:1".to_string()],
                provenance: json!({"source": "support"}),
            },
            Some("actor_keith"),
        )
        .unwrap();
        assert_eq!(reviewed.status, FeedbackRequestStatus::Accepted);
        assert_eq!(
            reviewed.reward_eligibility.as_ref().unwrap().state,
            "pending_qualification"
        );
        assert_eq!(list_public_reviews(&connection).unwrap().len(), 0);

        let member_scoped = list_feedback_requests_in_connection(
            &connection,
            FeedbackRequestQuery {
                viewer: FeedbackRequestViewer::Member,
                actor_id: Some("actor_member_1".to_string()),
                ..FeedbackRequestQuery::default()
            },
        )
        .unwrap();
        assert_eq!(member_scoped.requests.len(), 1);
        assert!(member_scoped.requests[0].reviews.is_empty());
        assert!(member_scoped.requests[0]
            .reward_eligibility
            .as_ref()
            .unwrap()
            .reason
            .is_none());
        let serialized = serde_json::to_string(&member_scoped.requests[0]).unwrap();
        assert!(!serialized.contains("internal only"));
        assert!(!serialized.contains("Support accepted"));
        assert!(!serialized.contains("support_review"));
    }

    #[test]
    fn feedback_request_validation_rejects_unsafe_context_and_terminal_rereview() {
        let connection = connection();
        let unsafe_request = create_feedback_request_in_connection(
            &connection,
            FeedbackRequestCreateRequest {
                target_kind: "trial".to_string(),
                target_id: "trial_1".to_string(),
                member_actor_id: Some("actor_member_1".to_string()),
                connection_id: None,
                conversation_id: Some("conversation_1".to_string()),
                source_kind: "support".to_string(),
                source_id: None,
                prompt: "Please respond using this providerSecret value.".to_string(),
                member_context_summary: "NYC pilot trial feedback.".to_string(),
                due_at: None,
                priority: None,
                evidence_refs: vec!["handoff:handoff_1".to_string()],
                provenance: json!({"source": "test"}),
                staff_context: json!({}),
            },
            Some("actor_keith"),
        )
        .unwrap_err();
        assert!(unsafe_request.to_string().contains("secret-bearing"));

        let invalid_target = create_feedback_request_in_connection(
            &connection,
            FeedbackRequestCreateRequest {
                target_kind: "provider_secret".to_string(),
                target_id: "secret_1".to_string(),
                member_actor_id: Some("actor_member_1".to_string()),
                connection_id: None,
                conversation_id: Some("conversation_1".to_string()),
                source_kind: "support".to_string(),
                source_id: None,
                prompt: "What should improve?".to_string(),
                member_context_summary: "NYC pilot trial feedback.".to_string(),
                due_at: None,
                priority: None,
                evidence_refs: vec!["handoff:handoff_1".to_string()],
                provenance: json!({"source": "test"}),
                staff_context: json!({}),
            },
            Some("actor_keith"),
        )
        .unwrap_err();
        assert!(invalid_target.to_string().contains("unsupported"));

        let (request, _) = create_feedback_request_in_connection(
            &connection,
            FeedbackRequestCreateRequest {
                target_kind: "conversation".to_string(),
                target_id: "conversation_1".to_string(),
                member_actor_id: Some("actor_member_1".to_string()),
                connection_id: Some("connection_1".to_string()),
                conversation_id: Some("conversation_1".to_string()),
                source_kind: "support".to_string(),
                source_id: None,
                prompt: "What should improve?".to_string(),
                member_context_summary: "NYC pilot conversation feedback.".to_string(),
                due_at: None,
                priority: None,
                evidence_refs: vec!["conversation:conversation_1".to_string()],
                provenance: json!({"source": "test"}),
                staff_context: json!({}),
            },
            Some("actor_keith"),
        )
        .unwrap();
        respond_to_feedback_request_in_connection(
            &connection,
            &request.id,
            FeedbackRequestRespondRequest {
                response_kind: Some("answer".to_string()),
                body_summary: "Make the request shorter.".to_string(),
                idempotency_key: None,
                evidence_refs: vec!["message:member_response_2".to_string()],
                provenance: json!({"source": "member_chat"}),
            },
            Some("actor_member_1"),
        )
        .unwrap();
        review_feedback_request_in_connection(
            &connection,
            &request.id,
            FeedbackRequestReviewRequest {
                decision: FeedbackRequestReviewDecision::Rejected,
                response_id: None,
                tags: vec!["not-actionable".to_string()],
                reason: "Not enough detail.".to_string(),
                evidence_refs: vec!["support_review:2".to_string()],
                provenance: json!({"source": "support"}),
            },
            Some("actor_keith"),
        )
        .unwrap();
        let rereview = review_feedback_request_in_connection(
            &connection,
            &request.id,
            FeedbackRequestReviewRequest {
                decision: FeedbackRequestReviewDecision::Accepted,
                response_id: None,
                tags: vec!["later".to_string()],
                reason: "Try to reverse a terminal decision.".to_string(),
                evidence_refs: vec!["support_review:3".to_string()],
                provenance: json!({"source": "support"}),
            },
            Some("actor_keith"),
        )
        .unwrap_err();
        assert!(rereview.to_string().contains("terminal review"));
    }
}
