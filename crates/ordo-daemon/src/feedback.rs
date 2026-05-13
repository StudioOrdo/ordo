use anyhow::{bail, ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
    connection.query_many("SELECT id, connection_id, conversation_id, segment_id, message_id, feedback_kind,
                status, visibility, body_summary, is_starred, source_refs_json,
                evidence_refs_json, provenance_json, created_at, updated_at
         FROM customer_feedback
         WHERE conversation_id = ?1 AND visibility = 'private_business_intelligence'
         ORDER BY updated_at DESC", [conversation_id], feedback_from_row)
}

pub fn list_public_reviews(connection: &Connection) -> Result<Vec<CustomerReviewView>> {
    connection.query_many("SELECT id, feedback_id, connection_id, conversation_id, status, review_body,
                publication_visibility, consent_evidence_refs_json,
                approval_evidence_refs_json, evidence_refs_json, provenance_json,
                created_at, updated_at, published_at, featured_at, retired_at
         FROM customer_reviews
         WHERE publication_visibility = 'public_review'
           AND status IN ('published', 'featured')
         ORDER BY updated_at DESC", [], review_from_row)
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
}
