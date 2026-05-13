use anyhow::{ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::events::{append_realtime_event, append_realtime_event_tx, system_event, RealtimeEvent};
use crate::security::redaction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributionCandidateTarget {
    Attribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferralRecordView {
    pub id: String,
    pub status: String,
    pub referrer_connection_id: Option<String>,
    pub referred_connection_id: Option<String>,
    pub conversation_id: Option<String>,
    pub entry_point_id: Option<String>,
    pub visitor_session_id: Option<String>,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessOutcomeView {
    pub id: String,
    pub outcome_kind: String,
    pub status: String,
    pub connection_id: Option<String>,
    pub conversation_id: Option<String>,
    pub segment_id: Option<String>,
    pub offer_id: Option<String>,
    pub ask_id: Option<String>,
    pub artifact_id: Option<String>,
    pub entry_point_id: Option<String>,
    pub visitor_session_id: Option<String>,
    pub referral_id: Option<String>,
    pub value_micros: Option<i64>,
    pub currency: Option<String>,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub occurred_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessOutcomeAttributionView {
    pub id: String,
    pub outcome_id: String,
    pub attribution_kind: String,
    pub source_id: String,
    pub influence_role: String,
    pub candidate_state: String,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub created_at: String,
    pub updated_at: String,
    pub state_changed_at: Option<String>,
    pub state_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BusinessOutcomeInput {
    pub outcome_kind: String,
    pub status: String,
    pub connection_id: Option<String>,
    pub conversation_id: Option<String>,
    pub segment_id: Option<String>,
    pub offer_id: Option<String>,
    pub ask_id: Option<String>,
    pub artifact_id: Option<String>,
    pub entry_point_id: Option<String>,
    pub visitor_session_id: Option<String>,
    pub referral_id: Option<String>,
    pub value_micros: Option<i64>,
    pub currency: Option<String>,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BusinessOutcomeAttributionInput {
    pub attribution_kind: String,
    pub source_id: String,
    pub influence_role: String,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
}

#[derive(Debug, Clone)]
pub struct ReferralRecordInput {
    pub status: String,
    pub referrer_connection_id: Option<String>,
    pub referred_connection_id: Option<String>,
    pub conversation_id: Option<String>,
    pub entry_point_id: Option<String>,
    pub visitor_session_id: Option<String>,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
}

#[derive(Debug, Clone)]
pub struct OfferAcceptanceOutcomeInput<'a> {
    pub acceptance_id: &'a str,
    pub trial_id: &'a str,
    pub offer_id: &'a str,
    pub offer_slug: &'a str,
    pub visitor_session_id: Option<&'a str>,
    pub entry_point_id: Option<&'a str>,
    pub occurred_at: &'a str,
}

pub fn record_referral(
    connection: &Connection,
    input: ReferralRecordInput,
) -> Result<(ReferralRecordView, RealtimeEvent)> {
    validate_evidence_and_provenance(&input.evidence_refs, &input.provenance)?;
    let now = Utc::now().to_rfc3339();
    let id = format!("referral_{}", Uuid::new_v4());
    let provenance = sanitize_json(input.provenance);
    connection.execute(
        "INSERT INTO referral_records (
            id, status, referrer_connection_id, referred_connection_id, conversation_id,
            entry_point_id, visitor_session_id, evidence_refs_json, provenance_json,
            created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
            id,
            input.status,
            input.referrer_connection_id,
            input.referred_connection_id,
            input.conversation_id,
            input.entry_point_id,
            input.visitor_session_id,
            json!(input.evidence_refs).to_string(),
            provenance.to_string(),
            now,
        ],
    )?;
    let referral = load_referral(connection, &id)?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "referral.captured",
            json!({
                "referralId": referral.id,
                "status": referral.status,
                "evidenceRefs": referral.evidence_refs,
            }),
        ),
    )?;
    Ok((referral, event))
}

pub fn record_outcome(
    connection: &Connection,
    input: BusinessOutcomeInput,
) -> Result<(BusinessOutcomeView, RealtimeEvent)> {
    let outcome = insert_outcome(connection, input)?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "business.outcome.recorded",
            json!({
                "outcomeId": outcome.id,
                "outcomeKind": outcome.outcome_kind,
                "status": outcome.status,
                "offerId": outcome.offer_id,
                "askId": outcome.ask_id,
                "entryPointId": outcome.entry_point_id,
                "visitorSessionId": outcome.visitor_session_id,
                "evidenceRefs": outcome.evidence_refs,
            }),
        ),
    )?;
    Ok((outcome, event))
}

pub fn propose_attribution(
    connection: &Connection,
    outcome_id: &str,
    input: BusinessOutcomeAttributionInput,
) -> Result<(BusinessOutcomeAttributionView, RealtimeEvent)> {
    let attribution = insert_attribution(connection, outcome_id, input)?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "business.attribution.proposed",
            json!({
                "attributionId": attribution.id,
                "outcomeId": attribution.outcome_id,
                "attributionKind": attribution.attribution_kind,
                "sourceId": attribution.source_id,
                "influenceRole": attribution.influence_role,
                "candidateState": attribution.candidate_state,
                "evidenceRefs": attribution.evidence_refs,
            }),
        ),
    )?;
    Ok((attribution, event))
}

pub fn transition_attribution(
    connection: &Connection,
    attribution_id: &str,
    new_state: &str,
    reason: &str,
) -> Result<RealtimeEvent> {
    ensure!(
        matches!(new_state, "confirmed" | "rejected" | "superseded"),
        "unsupported attribution candidate state"
    );
    ensure!(
        !reason.trim().is_empty(),
        "attribution transition reason is required"
    );
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE business_outcome_attributions
         SET candidate_state = ?2, state_changed_at = ?3, state_reason = ?4, updated_at = ?3
         WHERE id = ?1",
        params![attribution_id, new_state, now, sanitize_text(reason)],
    )?;
    let attribution = load_attribution(connection, attribution_id)?;
    append_realtime_event(
        connection,
        &system_event(
            &format!("business.attribution.{new_state}"),
            json!({
                "attributionId": attribution.id,
                "outcomeId": attribution.outcome_id,
                "candidateState": attribution.candidate_state,
                "reason": attribution.state_reason,
            }),
        ),
    )
}

pub fn record_offer_acceptance_outcome_tx(
    transaction: &Transaction<'_>,
    input: OfferAcceptanceOutcomeInput<'_>,
) -> Result<BusinessOutcomeView> {
    let persisted_offer_id = existing_offer_id(transaction, input.offer_id)?;
    let evidence_refs = vec![
        format!("offer_acceptance:{}", input.acceptance_id),
        format!("trial:{}", input.trial_id),
        format!("offer:{}", input.offer_id),
    ];
    let outcome = insert_outcome_tx(
        transaction,
        BusinessOutcomeInput {
            outcome_kind: "offer_acceptance".to_string(),
            status: "recorded".to_string(),
            connection_id: None,
            conversation_id: None,
            segment_id: None,
            offer_id: persisted_offer_id,
            ask_id: None,
            artifact_id: None,
            entry_point_id: input.entry_point_id.map(str::to_string),
            visitor_session_id: input.visitor_session_id.map(str::to_string),
            referral_id: None,
            value_micros: None,
            currency: None,
            evidence_refs: evidence_refs.clone(),
            provenance: json!({
                "generator": "offer.accept_public_offer",
                "offerSlug": input.offer_slug,
                "acceptanceId": input.acceptance_id,
                "trialId": input.trial_id,
            }),
            occurred_at: Some(input.occurred_at.to_string()),
        },
    )?;
    insert_attribution_tx(
        transaction,
        &outcome.id,
        BusinessOutcomeAttributionInput {
            attribution_kind: "offer".to_string(),
            source_id: input.offer_id.to_string(),
            influence_role: "direct".to_string(),
            confidence: 1.0,
            evidence_refs: evidence_refs.clone(),
            provenance: json!({
                "generator": "offer.accept_public_offer",
                "reason": "The accepted offer is the direct business instrument.",
            }),
        },
    )?;
    if let Some(session_id) = input.visitor_session_id {
        insert_attribution_tx(
            transaction,
            &outcome.id,
            BusinessOutcomeAttributionInput {
                attribution_kind: "visitor_session".to_string(),
                source_id: session_id.to_string(),
                influence_role: "assisted".to_string(),
                confidence: 0.85,
                evidence_refs: evidence_refs.clone(),
                provenance: json!({
                    "generator": "offer.accept_public_offer",
                    "reason": "Visitor session was explicitly attached to the acceptance.",
                }),
            },
        )?;
    }
    if let Some(entry_point_id) = input.entry_point_id {
        insert_attribution_tx(
            transaction,
            &outcome.id,
            BusinessOutcomeAttributionInput {
                attribution_kind: "entry_point".to_string(),
                source_id: entry_point_id.to_string(),
                influence_role: "first_touch".to_string(),
                confidence: 0.85,
                evidence_refs,
                provenance: json!({
                    "generator": "offer.accept_public_offer",
                    "reason": "Entry point came from the accepted visitor session.",
                }),
            },
        )?;
    }
    append_realtime_event_tx(
        transaction,
        &system_event(
            "business.outcome.recorded",
            json!({
                "outcomeId": outcome.id,
                "outcomeKind": outcome.outcome_kind,
                "status": outcome.status,
                "offerId": outcome.offer_id,
                "entryPointId": outcome.entry_point_id,
                "visitorSessionId": outcome.visitor_session_id,
                "evidenceRefs": outcome.evidence_refs,
            }),
        ),
    )?;
    Ok(outcome)
}

fn existing_offer_id(transaction: &Transaction<'_>, offer_id: &str) -> Result<Option<String>> {
    transaction
        .query_row("SELECT id FROM offers WHERE id = ?1", [offer_id], |row| {
            row.get(0)
        })
        .optional()
        .map_err(Into::into)
}

pub fn list_outcomes_by_offer(
    connection: &Connection,
    offer_id: &str,
) -> Result<Vec<BusinessOutcomeView>> {
    list_outcomes(connection, "offer_id", offer_id)
}

pub fn list_outcomes_by_conversation(
    connection: &Connection,
    conversation_id: &str,
) -> Result<Vec<BusinessOutcomeView>> {
    list_outcomes(connection, "conversation_id", conversation_id)
}

pub fn list_outcomes_by_connection(
    connection: &Connection,
    connection_id: &str,
) -> Result<Vec<BusinessOutcomeView>> {
    list_outcomes(connection, "connection_id", connection_id)
}

pub fn list_outcomes_by_entry_point(
    connection: &Connection,
    entry_point_id: &str,
) -> Result<Vec<BusinessOutcomeView>> {
    list_outcomes(connection, "entry_point_id", entry_point_id)
}

pub fn list_attributions_for_outcome(
    connection: &Connection,
    outcome_id: &str,
) -> Result<Vec<BusinessOutcomeAttributionView>> {
    let mut statement = connection.prepare(
        "SELECT id, outcome_id, attribution_kind, source_id, influence_role, candidate_state,
                confidence, evidence_refs_json, provenance_json, created_at, updated_at,
                state_changed_at, state_reason
         FROM business_outcome_attributions
         WHERE outcome_id = ?1
         ORDER BY created_at ASC",
    )?;
    let rows = statement.query_map([outcome_id], attribution_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn load_outcome(connection: &Connection, outcome_id: &str) -> Result<BusinessOutcomeView> {
    connection
        .query_row(
            "SELECT id, outcome_kind, status, connection_id, conversation_id, segment_id,
                    offer_id, ask_id, artifact_id, entry_point_id, visitor_session_id,
                    referral_id, value_micros, currency, evidence_refs_json, provenance_json,
                    occurred_at, created_at, updated_at
             FROM business_outcomes
             WHERE id = ?1",
            [outcome_id],
            outcome_from_row,
        )
        .map_err(Into::into)
}

pub fn load_attribution(
    connection: &Connection,
    attribution_id: &str,
) -> Result<BusinessOutcomeAttributionView> {
    connection
        .query_row(
            "SELECT id, outcome_id, attribution_kind, source_id, influence_role, candidate_state,
                    confidence, evidence_refs_json, provenance_json, created_at, updated_at,
                    state_changed_at, state_reason
             FROM business_outcome_attributions
             WHERE id = ?1",
            [attribution_id],
            attribution_from_row,
        )
        .map_err(Into::into)
}

fn load_referral(connection: &Connection, referral_id: &str) -> Result<ReferralRecordView> {
    connection
        .query_row(
            "SELECT id, status, referrer_connection_id, referred_connection_id, conversation_id,
                    entry_point_id, visitor_session_id, evidence_refs_json, provenance_json,
                    created_at, updated_at, closed_at
             FROM referral_records
             WHERE id = ?1",
            [referral_id],
            referral_from_row,
        )
        .map_err(Into::into)
}

fn insert_outcome(
    connection: &Connection,
    input: BusinessOutcomeInput,
) -> Result<BusinessOutcomeView> {
    validate_outcome_input(&input)?;
    let id = format!("business_outcome_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();
    let occurred_at = input.occurred_at.unwrap_or_else(|| now.clone());
    let provenance = sanitize_json(input.provenance);
    connection.execute(
        "INSERT INTO business_outcomes (
            id, outcome_kind, status, connection_id, conversation_id, segment_id, offer_id,
            ask_id, artifact_id, entry_point_id, visitor_session_id, referral_id, value_micros,
            currency, evidence_refs_json, provenance_json, occurred_at, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?18)",
        params![
            id,
            input.outcome_kind,
            input.status,
            input.connection_id,
            input.conversation_id,
            input.segment_id,
            input.offer_id,
            input.ask_id,
            input.artifact_id,
            input.entry_point_id,
            input.visitor_session_id,
            input.referral_id,
            input.value_micros,
            input.currency,
            json!(input.evidence_refs).to_string(),
            provenance.to_string(),
            occurred_at,
            now,
        ],
    )?;
    load_outcome(connection, &id)
}

fn insert_outcome_tx(
    transaction: &Transaction<'_>,
    input: BusinessOutcomeInput,
) -> Result<BusinessOutcomeView> {
    validate_outcome_input(&input)?;
    let id = stable_outcome_id(&input);
    let now = Utc::now().to_rfc3339();
    let occurred_at = input.occurred_at.unwrap_or_else(|| now.clone());
    let provenance = sanitize_json(input.provenance);
    transaction.execute(
        "INSERT OR IGNORE INTO business_outcomes (
            id, outcome_kind, status, connection_id, conversation_id, segment_id, offer_id,
            ask_id, artifact_id, entry_point_id, visitor_session_id, referral_id, value_micros,
            currency, evidence_refs_json, provenance_json, occurred_at, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?18)",
        params![
            id,
            input.outcome_kind,
            input.status,
            input.connection_id,
            input.conversation_id,
            input.segment_id,
            input.offer_id,
            input.ask_id,
            input.artifact_id,
            input.entry_point_id,
            input.visitor_session_id,
            input.referral_id,
            input.value_micros,
            input.currency,
            json!(input.evidence_refs).to_string(),
            provenance.to_string(),
            occurred_at,
            now,
        ],
    )?;
    load_outcome(transaction, &id)
}

fn insert_attribution(
    connection: &Connection,
    outcome_id: &str,
    input: BusinessOutcomeAttributionInput,
) -> Result<BusinessOutcomeAttributionView> {
    validate_attribution_input(&input)?;
    let id = format!("business_attribution_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();
    let provenance = sanitize_json(input.provenance);
    connection.execute(
        "INSERT INTO business_outcome_attributions (
            id, outcome_id, attribution_kind, source_id, influence_role, candidate_state,
            confidence, evidence_refs_json, provenance_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, 'proposed', ?6, ?7, ?8, ?9, ?9)",
        params![
            id,
            outcome_id,
            input.attribution_kind,
            input.source_id,
            input.influence_role,
            input.confidence,
            json!(input.evidence_refs).to_string(),
            provenance.to_string(),
            now,
        ],
    )?;
    load_attribution(connection, &id)
}

fn insert_attribution_tx(
    transaction: &Transaction<'_>,
    outcome_id: &str,
    input: BusinessOutcomeAttributionInput,
) -> Result<BusinessOutcomeAttributionView> {
    validate_attribution_input(&input)?;
    let id = stable_attribution_id(outcome_id, &input);
    let now = Utc::now().to_rfc3339();
    let provenance = sanitize_json(input.provenance);
    transaction.execute(
        "INSERT OR IGNORE INTO business_outcome_attributions (
            id, outcome_id, attribution_kind, source_id, influence_role, candidate_state,
            confidence, evidence_refs_json, provenance_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, 'proposed', ?6, ?7, ?8, ?9, ?9)",
        params![
            id,
            outcome_id,
            input.attribution_kind,
            input.source_id,
            input.influence_role,
            input.confidence,
            json!(input.evidence_refs).to_string(),
            provenance.to_string(),
            now,
        ],
    )?;
    load_attribution(transaction, &id)
}

fn list_outcomes(
    connection: &Connection,
    column: &str,
    value: &str,
) -> Result<Vec<BusinessOutcomeView>> {
    ensure!(
        matches!(
            column,
            "conversation_id" | "connection_id" | "offer_id" | "entry_point_id"
        ),
        "unsupported outcome listing column"
    );
    let mut statement = connection.prepare(&format!(
        "SELECT id, outcome_kind, status, connection_id, conversation_id, segment_id,
                offer_id, ask_id, artifact_id, entry_point_id, visitor_session_id, referral_id,
                value_micros, currency, evidence_refs_json, provenance_json, occurred_at,
                created_at, updated_at
         FROM business_outcomes
         WHERE {column} = ?1
         ORDER BY occurred_at DESC, id DESC"
    ))?;
    let rows = statement.query_map([value], outcome_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn validate_outcome_input(input: &BusinessOutcomeInput) -> Result<()> {
    ensure!(
        !input.outcome_kind.trim().is_empty(),
        "outcome kind is required"
    );
    ensure!(
        !input.status.trim().is_empty(),
        "outcome status is required"
    );
    validate_evidence_and_provenance(&input.evidence_refs, &input.provenance)?;
    Ok(())
}

fn validate_attribution_input(input: &BusinessOutcomeAttributionInput) -> Result<()> {
    ensure!(
        !input.attribution_kind.trim().is_empty(),
        "attribution kind is required"
    );
    ensure!(!input.source_id.trim().is_empty(), "source id is required");
    ensure!(
        matches!(
            input.influence_role.as_str(),
            "first_touch" | "assisted" | "direct" | "confirming" | "excluded"
        ),
        "unsupported influence role"
    );
    ensure!(
        (0.0..=1.0).contains(&input.confidence),
        "attribution confidence must be 0.0..=1.0"
    );
    validate_evidence_and_provenance(&input.evidence_refs, &input.provenance)?;
    Ok(())
}

fn validate_evidence_and_provenance(evidence_refs: &[String], provenance: &Value) -> Result<()> {
    ensure!(!evidence_refs.is_empty(), "evidence refs are required");
    ensure!(
        !provenance
            .as_object()
            .map(|object| object.is_empty())
            .unwrap_or(true),
        "provenance is required"
    );
    Ok(())
}

fn referral_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ReferralRecordView> {
    Ok(ReferralRecordView {
        id: row.get(0)?,
        status: row.get(1)?,
        referrer_connection_id: row.get(2)?,
        referred_connection_id: row.get(3)?,
        conversation_id: row.get(4)?,
        entry_point_id: row.get(5)?,
        visitor_session_id: row.get(6)?,
        evidence_refs: json_string_array(row.get(7)?),
        provenance: json_object(row.get(8)?),
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        closed_at: row.get(11)?,
    })
}

fn outcome_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BusinessOutcomeView> {
    Ok(BusinessOutcomeView {
        id: row.get(0)?,
        outcome_kind: row.get(1)?,
        status: row.get(2)?,
        connection_id: row.get(3)?,
        conversation_id: row.get(4)?,
        segment_id: row.get(5)?,
        offer_id: row.get(6)?,
        ask_id: row.get(7)?,
        artifact_id: row.get(8)?,
        entry_point_id: row.get(9)?,
        visitor_session_id: row.get(10)?,
        referral_id: row.get(11)?,
        value_micros: row.get(12)?,
        currency: row.get(13)?,
        evidence_refs: json_string_array(row.get(14)?),
        provenance: json_object(row.get(15)?),
        occurred_at: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

fn attribution_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<BusinessOutcomeAttributionView> {
    Ok(BusinessOutcomeAttributionView {
        id: row.get(0)?,
        outcome_id: row.get(1)?,
        attribution_kind: row.get(2)?,
        source_id: row.get(3)?,
        influence_role: row.get(4)?,
        candidate_state: row.get(5)?,
        confidence: row.get(6)?,
        evidence_refs: json_string_array(row.get(7)?),
        provenance: json_object(row.get(8)?),
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        state_changed_at: row.get(11)?,
        state_reason: row.get(12)?,
    })
}

fn sanitize_json(value: Value) -> Value {
    redaction::sanitize_json_strings(value)
}

fn sanitize_text(text: &str) -> String {
    redaction::redact_public_text(text)
}

fn json_string_array(raw: String) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
}

fn json_object(raw: String) -> Value {
    serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}))
}

fn stable_outcome_id(input: &BusinessOutcomeInput) -> String {
    let hash = stable_hash(&format!(
        "{}|{}|{:?}|{:?}|{:?}|{:?}|{:?}",
        input.outcome_kind,
        input.status,
        input.offer_id,
        input.ask_id,
        input.entry_point_id,
        input.visitor_session_id,
        input.evidence_refs
    ));
    stable_id("business_outcome", &hash)
}

fn stable_attribution_id(outcome_id: &str, input: &BusinessOutcomeAttributionInput) -> String {
    let hash = stable_hash(&format!(
        "{outcome_id}|{}|{}|{}|{:?}",
        input.attribution_kind, input.source_id, input.influence_role, input.evidence_refs
    ));
    stable_id("business_attribution", &hash)
}

fn stable_id(prefix: &str, content_hash: &str) -> String {
    let suffix = content_hash.strip_prefix("sha256:").unwrap_or(content_hash);
    format!("{prefix}_{}", &suffix[..24.min(suffix.len())])
}

fn stable_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;

    fn test_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO connections (
                    id, connection_type, display_name, status, identity_json, scope_json, metadata_json, created_at, updated_at
                 ) VALUES ('connection_1', 'client', 'Client', 'active', '{}', '{}', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO conversations (
                    id, surface, subject_kind, subject_id, connection_id, visitor_session_id,
                    status, visibility, privacy_scope, current_segment_id, last_meaningful_change,
                    unread_count, action_count, summary_json, metadata_json, created_by_actor_id,
                    created_at, updated_at, closed_at, archived_at
                 ) VALUES (
                    'conversation_1', 'client_portal', 'connection', 'connection_1', 'connection_1', NULL,
                    'open', 'participants', 'local', NULL, 'seeded', 0, 0, '{}', '{}', NULL,
                    'now', 'now', NULL, NULL
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO tracked_entry_points (
                    id, slug, label, status, source_kind, source_label, destination_surface,
                    destination_id, public_path, qr_payload_json, attribution_json, metadata_json,
                    created_by_actor_id, created_at, updated_at, archived_at
                 ) VALUES (
                    'entry_1', 'entry-1', 'Entry 1', 'active', 'campaign', 'Campaign',
                    'offers', NULL, '/e/entry-1', '{}', '{}', '{}', NULL, 'now', 'now', NULL
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO offers (
                    id, slug, title, summary, status, visibility, publication_state, trial_days,
                    source_kind, source_ref, terms_json, metadata_json, created_by_actor_id,
                    created_at, updated_at, published_at, archived_at
                 ) VALUES (
                    'offer_1', 'offer-1', 'Offer 1', 'Summary', 'available', 'public',
                    'published', 30, 'manual', NULL, '{}', '{}', NULL, 'now', 'now', 'now', NULL
                 )",
                [],
            )
            .unwrap();
        connection
    }

    #[test]
    fn outcomes_and_referrals_require_evidence_and_provenance() {
        let connection = test_connection();

        let bad_outcome = record_outcome(
            &connection,
            BusinessOutcomeInput {
                outcome_kind: "conversion".to_string(),
                status: "recorded".to_string(),
                connection_id: None,
                conversation_id: None,
                segment_id: None,
                offer_id: None,
                ask_id: None,
                artifact_id: None,
                entry_point_id: None,
                visitor_session_id: None,
                referral_id: None,
                value_micros: None,
                currency: None,
                evidence_refs: vec![],
                provenance: json!({"generator": "test"}),
                occurred_at: None,
            },
        );
        assert!(bad_outcome.is_err());

        let bad_referral = record_referral(
            &connection,
            ReferralRecordInput {
                status: "captured".to_string(),
                referrer_connection_id: None,
                referred_connection_id: None,
                conversation_id: None,
                entry_point_id: None,
                visitor_session_id: None,
                evidence_refs: vec!["conversation_message:message_1".to_string()],
                provenance: json!({}),
            },
        );
        assert!(bad_referral.is_err());
    }

    #[test]
    fn attribution_lifecycle_and_listing_are_durable() {
        let connection = test_connection();
        let (outcome, _) = record_outcome(
            &connection,
            BusinessOutcomeInput {
                outcome_kind: "ask_response".to_string(),
                status: "recorded".to_string(),
                connection_id: Some("connection_1".to_string()),
                conversation_id: Some("conversation_1".to_string()),
                segment_id: None,
                offer_id: Some("offer_1".to_string()),
                ask_id: Some("ask_1".to_string()),
                artifact_id: None,
                entry_point_id: Some("entry_1".to_string()),
                visitor_session_id: None,
                referral_id: None,
                value_micros: None,
                currency: None,
                evidence_refs: vec!["conversation_message:message_1".to_string()],
                provenance: json!({"generator": "test"}),
                occurred_at: Some("2026-05-09T00:00:00Z".to_string()),
            },
        )
        .unwrap();

        let (attribution, proposed) = propose_attribution(
            &connection,
            &outcome.id,
            BusinessOutcomeAttributionInput {
                attribution_kind: "conversation".to_string(),
                source_id: "conversation_1".to_string(),
                influence_role: "assisted".to_string(),
                confidence: 0.7,
                evidence_refs: outcome.evidence_refs.clone(),
                provenance: json!({"generator": "test"}),
            },
        )
        .unwrap();
        assert_eq!(attribution.candidate_state, "proposed");
        assert_eq!(proposed.event_type, "business.attribution.proposed");

        let confirmed =
            transition_attribution(&connection, &attribution.id, "confirmed", "verified").unwrap();
        assert_eq!(confirmed.event_type, "business.attribution.confirmed");
        assert_eq!(
            load_attribution(&connection, &attribution.id)
                .unwrap()
                .candidate_state,
            "confirmed"
        );
        assert_eq!(
            list_outcomes_by_conversation(&connection, "conversation_1")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            list_outcomes_by_offer(&connection, "offer_1")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            list_outcomes_by_entry_point(&connection, "entry_1")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            list_outcomes_by_connection(&connection, "connection_1")
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn attribution_rejects_missing_source_and_unknown_influence() {
        let connection = test_connection();
        let (outcome, _) = record_outcome(
            &connection,
            BusinessOutcomeInput {
                outcome_kind: "offer_acceptance".to_string(),
                status: "recorded".to_string(),
                connection_id: None,
                conversation_id: None,
                segment_id: None,
                offer_id: Some("offer_1".to_string()),
                ask_id: None,
                artifact_id: None,
                entry_point_id: None,
                visitor_session_id: None,
                referral_id: None,
                value_micros: None,
                currency: None,
                evidence_refs: vec!["offer_acceptance:acceptance_1".to_string()],
                provenance: json!({"generator": "test"}),
                occurred_at: None,
            },
        )
        .unwrap();

        let missing_source = propose_attribution(
            &connection,
            &outcome.id,
            BusinessOutcomeAttributionInput {
                attribution_kind: "campaign".to_string(),
                source_id: "".to_string(),
                influence_role: "assisted".to_string(),
                confidence: 0.5,
                evidence_refs: outcome.evidence_refs.clone(),
                provenance: json!({"generator": "test"}),
            },
        );
        assert!(missing_source.is_err());
        let unknown_role = propose_attribution(
            &connection,
            &outcome.id,
            BusinessOutcomeAttributionInput {
                attribution_kind: "campaign".to_string(),
                source_id: "campaign_1".to_string(),
                influence_role: "magic".to_string(),
                confidence: 0.5,
                evidence_refs: outcome.evidence_refs.clone(),
                provenance: json!({"generator": "test"}),
            },
        );
        assert!(unknown_role.is_err());
    }

    #[test]
    fn outcomes_and_attributions_do_not_store_sensitive_fixture_text() {
        let connection = test_connection();
        let (outcome, _) = record_outcome(
            &connection,
            BusinessOutcomeInput {
                outcome_kind: "conversion".to_string(),
                status: "recorded".to_string(),
                connection_id: None,
                conversation_id: None,
                segment_id: None,
                offer_id: Some("offer_1".to_string()),
                ask_id: None,
                artifact_id: None,
                entry_point_id: None,
                visitor_session_id: None,
                referral_id: None,
                value_micros: None,
                currency: None,
                evidence_refs: vec!["offer_acceptance:acceptance_1".to_string()],
                provenance: json!({
                    "note": "ada@example.com Bearer tok_abcdef123456 sk-test-123456"
                }),
                occurred_at: None,
            },
        )
        .unwrap();
        propose_attribution(
            &connection,
            &outcome.id,
            BusinessOutcomeAttributionInput {
                attribution_kind: "offer".to_string(),
                source_id: "offer_1".to_string(),
                influence_role: "direct".to_string(),
                confidence: 1.0,
                evidence_refs: outcome.evidence_refs.clone(),
                provenance: json!({
                    "note": "ada@example.com Bearer tok_abcdef123456 sk-test-123456"
                }),
            },
        )
        .unwrap();

        for raw in ["ada@example.com", "tok_abcdef123456", "sk-test-123456"] {
            for (table, columns) in [
                ("business_outcomes", "provenance_json || evidence_refs_json"),
                (
                    "business_outcome_attributions",
                    "provenance_json || evidence_refs_json",
                ),
                ("realtime_events", "payload_json"),
            ] {
                let leaked_count: i64 = connection
                    .query_row(
                        &format!("SELECT COUNT(*) FROM {table} WHERE {columns} LIKE ?1"),
                        [format!("%{raw}%")],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(leaked_count, 0, "{table} leaked {raw}");
            }
        }
    }
}
