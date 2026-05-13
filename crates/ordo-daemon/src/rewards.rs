use anyhow::{bail, ensure, Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

use crate::events::{append_realtime_event_tx, system_event, RealtimeEvent};

const PILOT_REWARD_PROGRAM_ID: &str = "reward_program_ordostudio_nyc_pilot";
const PILOT_REWARD_PROGRAM_SLUG: &str = "ordostudio-nyc-pilot";
const REFERRAL_REWARD_RULE_ID: &str = "reward_rule_ordostudio_referral_trial_activation";
const FEEDBACK_REWARD_RULE_ID: &str = "reward_rule_ordostudio_accepted_feedback";
const REFERRAL_TRIGGER_KIND: &str = "referral_trial_activation";
const FEEDBACK_TRIGGER_KIND: &str = "accepted_feedback";
const HOSTED_TIME_BENEFIT_KIND: &str = "hosted_trial_time";
const HOSTED_TIME_UNIT: &str = "day";
const REFERRAL_HOSTED_DAYS: i64 = 7;
const FEEDBACK_HOSTED_DAYS: i64 = 3;
const HOSTED_DAYS_CAP: i64 = 30;
const HOSTED_TRIAL_RESOURCE_KIND: &str = "hosted_trial";
const HOSTED_TRIAL_ACTION: &str = "use";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RewardViewer {
    Member,
    Growth,
    Owner,
    System,
}

impl Default for RewardViewer {
    fn default() -> Self {
        Self::Owner
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RewardQuery {
    pub viewer: RewardViewer,
    pub actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RewardQualificationRequest {
    pub trial_id: Option<String>,
    pub activation_trial_id: Option<String>,
    pub actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub evidence_refs: Vec<String>,
    pub reason: Option<String>,
    pub amount: Option<i64>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewardEventTransitionRequest {
    pub state: String,
    pub reason: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewardSummaryResponse {
    pub programs: Vec<RewardProgramView>,
    pub events: Vec<RewardEventView>,
    pub ledger_entries: Vec<RewardLedgerEntryView>,
    pub benefit_grants: Vec<BenefitGrantView>,
    pub benefit_balances: Vec<BenefitBalanceView>,
    pub generated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewardQualificationResponse {
    pub event: RewardEventView,
    pub ledger_entry: Option<RewardLedgerEntryView>,
    pub benefit_grant: Option<BenefitGrantView>,
    pub benefit_balance: Option<BenefitBalanceView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewardProgramView {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub status: String,
    pub visibility: String,
    pub terms: Value,
    pub policy: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewardEventView {
    pub id: String,
    pub program_id: String,
    pub rule_id: String,
    pub actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub source_kind: String,
    pub source_id: Option<String>,
    pub state: String,
    pub reason: Option<String>,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub qualified_at: Option<String>,
    pub granted_at: Option<String>,
    pub rejected_at: Option<String>,
    pub expired_at: Option<String>,
    pub capped_at: Option<String>,
    pub reversed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewardLedgerEntryView {
    pub id: String,
    pub event_id: String,
    pub program_id: String,
    pub rule_id: String,
    pub actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub entry_kind: String,
    pub amount: i64,
    pub unit: String,
    pub benefit_grant_id: Option<String>,
    pub reason: Option<String>,
    pub evidence_refs: Vec<String>,
    pub created_at: String,
    pub reversed_at: Option<String>,
    pub reversal_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenefitGrantView {
    pub id: String,
    pub event_id: String,
    pub ledger_entry_id: Option<String>,
    pub actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub access_grant_id: Option<String>,
    pub trial_id: Option<String>,
    pub benefit_kind: String,
    pub amount: i64,
    pub unit: String,
    pub state: String,
    pub starts_at: String,
    pub expires_at: Option<String>,
    pub consumed_at: Option<String>,
    pub revoked_at: Option<String>,
    pub reversed_at: Option<String>,
    pub evidence_refs: Vec<String>,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenefitBalanceView {
    pub id: String,
    pub program_id: String,
    pub actor_id: Option<String>,
    pub connection_id: Option<String>,
    pub benefit_kind: String,
    pub unit: String,
    pub total_earned: i64,
    pub total_active: i64,
    pub total_reversed: i64,
    pub cap_quantity: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
struct RewardProgramRecord {
    id: String,
    slug: String,
    name: String,
    status: String,
    visibility: String,
    terms: Value,
    policy: Value,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct RewardEventRecord {
    id: String,
    program_id: String,
    rule_id: String,
    actor_id: Option<String>,
    connection_id: Option<String>,
    source_kind: String,
    source_id: String,
    state: String,
    reason: String,
    evidence_refs: Vec<String>,
    provenance: Value,
    qualified_at: Option<String>,
    granted_at: Option<String>,
    rejected_at: Option<String>,
    expired_at: Option<String>,
    capped_at: Option<String>,
    reversed_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct RewardLedgerEntryRecord {
    id: String,
    event_id: String,
    program_id: String,
    rule_id: String,
    actor_id: Option<String>,
    connection_id: Option<String>,
    entry_kind: String,
    amount: i64,
    unit: String,
    benefit_grant_id: Option<String>,
    reason: String,
    evidence_refs: Vec<String>,
    created_at: String,
    reversed_at: Option<String>,
    reversal_reason: Option<String>,
}

#[derive(Debug, Clone)]
struct BenefitGrantRecord {
    id: String,
    event_id: String,
    ledger_entry_id: Option<String>,
    actor_id: Option<String>,
    connection_id: Option<String>,
    access_grant_id: Option<String>,
    trial_id: Option<String>,
    benefit_kind: String,
    amount: i64,
    unit: String,
    state: String,
    starts_at: String,
    expires_at: Option<String>,
    consumed_at: Option<String>,
    revoked_at: Option<String>,
    reversed_at: Option<String>,
    evidence_refs: Vec<String>,
    metadata: Value,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct BenefitBalanceRecord {
    id: String,
    program_id: String,
    actor_id: Option<String>,
    connection_id: Option<String>,
    benefit_kind: String,
    unit: String,
    total_earned: i64,
    total_active: i64,
    total_reversed: i64,
    cap_quantity: i64,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct RewardRuleRecord {
    id: String,
    program_id: String,
    trigger_kind: String,
    benefit_kind: String,
    benefit_quantity: i64,
    benefit_unit: String,
    max_quantity_per_actor: i64,
}

#[derive(Debug, Clone)]
struct TrialAccessContext {
    trial_id: String,
    acceptance_id: String,
    status: String,
    trial_ends_at: String,
    decision_evidence: Value,
    access_grant_id: String,
    subject_kind: String,
    subject_id: String,
    access_expires_at: Option<String>,
    slot_id: Option<String>,
}

#[derive(Debug, Clone)]
struct RewardQualificationContext {
    rule: RewardRuleRecord,
    source_kind: String,
    source_id: String,
    actor_id: Option<String>,
    connection_id: Option<String>,
    trial: TrialAccessContext,
    requested_amount: i64,
    reason: String,
    evidence_refs: Vec<String>,
    idempotency_key: String,
    feedback_eligibility_id: Option<String>,
}

#[derive(Debug, Clone)]
struct FeedbackEligibilityContext {
    request_id: String,
    review_id: Option<String>,
    state: String,
    request_status: String,
    member_actor_id: Option<String>,
    connection_id: Option<String>,
    evidence_refs: Vec<String>,
}

pub fn list_rewards(db_path: &Path, query: RewardQuery) -> Result<RewardSummaryResponse> {
    let connection = Connection::open(db_path)?;
    ensure_pilot_reward_program(&connection)?;
    let limit = query.limit.unwrap_or(100).min(500);
    let programs = load_reward_programs(&connection)?
        .into_iter()
        .map(RewardProgramRecord::into_view)
        .collect();
    let events = load_reward_events(&connection, limit)?
        .into_iter()
        .filter(|event| {
            reward_subject_matches(
                &query,
                event.actor_id.as_deref(),
                event.connection_id.as_deref(),
            )
        })
        .map(|event| event.into_view(query.viewer))
        .collect();
    let ledger_entries = load_reward_ledger_entries(&connection, limit)?
        .into_iter()
        .filter(|entry| {
            reward_subject_matches(
                &query,
                entry.actor_id.as_deref(),
                entry.connection_id.as_deref(),
            )
        })
        .map(|entry| entry.into_view(query.viewer))
        .collect();
    let benefit_grants = load_benefit_grants(&connection, limit)?
        .into_iter()
        .filter(|grant| {
            reward_subject_matches(
                &query,
                grant.actor_id.as_deref(),
                grant.connection_id.as_deref(),
            )
        })
        .map(|grant| grant.into_view(query.viewer))
        .collect();
    let benefit_balances = load_benefit_balances(&connection, limit)?
        .into_iter()
        .filter(|balance| {
            reward_subject_matches(
                &query,
                balance.actor_id.as_deref(),
                balance.connection_id.as_deref(),
            )
        })
        .map(|balance| balance.into_view(query.viewer))
        .collect();

    Ok(RewardSummaryResponse {
        programs,
        events,
        ledger_entries,
        benefit_grants,
        benefit_balances,
        generated_at: Utc::now().to_rfc3339(),
    })
}

pub fn qualify_referral_reward(
    db_path: &Path,
    referral_id: &str,
    request: RewardQualificationRequest,
    reviewer_actor_id: Option<&str>,
) -> Result<(RewardQualificationResponse, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    ensure_pilot_reward_program(&connection)?;
    let context = referral_qualification_context(&connection, referral_id, request)?;
    if let Some(existing) = load_reward_event_by_idempotency(&connection, &context.idempotency_key)?
    {
        let response = load_qualification_response(&connection, &existing.id, RewardViewer::Owner)?;
        return Ok((response, replay_event(&existing)));
    }
    let event_type =
        if current_active_total(&connection, &context)? >= context.rule.max_quantity_per_actor {
            "reward.capped"
        } else {
            "reward.granted"
        };
    let (event_id, event) =
        insert_reward_qualification(&mut connection, context, reviewer_actor_id, event_type)?;
    let response = load_qualification_response(&connection, &event_id, RewardViewer::Owner)?;
    Ok((response, event))
}

pub fn qualify_feedback_reward(
    db_path: &Path,
    eligibility_id: &str,
    request: RewardQualificationRequest,
    reviewer_actor_id: Option<&str>,
) -> Result<(RewardQualificationResponse, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    ensure_pilot_reward_program(&connection)?;
    let context = feedback_qualification_context(&connection, eligibility_id, request)?;
    if let Some(existing) = load_reward_event_by_idempotency(&connection, &context.idempotency_key)?
    {
        let response = load_qualification_response(&connection, &existing.id, RewardViewer::Owner)?;
        return Ok((response, replay_event(&existing)));
    }
    let event_type =
        if current_active_total(&connection, &context)? >= context.rule.max_quantity_per_actor {
            "reward.capped"
        } else {
            "reward.granted"
        };
    let (event_id, event) =
        insert_reward_qualification(&mut connection, context, reviewer_actor_id, event_type)?;
    let response = load_qualification_response(&connection, &event_id, RewardViewer::Owner)?;
    Ok((response, event))
}

pub fn transition_reward_event(
    db_path: &Path,
    event_id: &str,
    request: RewardEventTransitionRequest,
    reviewer_actor_id: Option<&str>,
) -> Result<(RewardQualificationResponse, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    ensure!(
        matches!(
            request.state.as_str(),
            "rejected" | "expired" | "capped" | "reversed"
        ),
        "unsupported reward event transition"
    );
    ensure!(
        !request.reason.trim().is_empty(),
        "reward transition reason is required"
    );
    let existing = load_reward_event(&connection, event_id)?;
    let now = Utc::now().to_rfc3339();
    let transaction = connection.transaction()?;
    match request.state.as_str() {
        "rejected" => reject_reward_event_tx(
            &transaction,
            &existing,
            &request.reason,
            &request.evidence_refs,
            reviewer_actor_id,
            &now,
        )?,
        "expired" => expire_reward_event_tx(
            &transaction,
            &existing,
            &request.reason,
            &request.evidence_refs,
            reviewer_actor_id,
            &now,
        )?,
        "capped" => cap_reward_event_tx(
            &transaction,
            &existing,
            &request.reason,
            &request.evidence_refs,
            reviewer_actor_id,
            &now,
        )?,
        "reversed" => reverse_reward_event_tx(
            &transaction,
            &existing,
            &request.reason,
            &request.evidence_refs,
            reviewer_actor_id,
            &now,
        )?,
        _ => unreachable!(),
    }
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            &format!("reward.{}", request.state),
            json!({
                "rewardEventId": existing.id,
                "state": request.state,
                "reason": safe_text(&request.reason),
            }),
        ),
    )?;
    transaction.commit()?;
    let response = load_qualification_response(&connection, event_id, RewardViewer::Owner)?;
    Ok((response, event))
}

pub fn reward_program_is_active(connection: &Connection, program_id: &str) -> Result<bool> {
    ensure_pilot_reward_program(connection)?;
    let active = connection
        .query_row(
            "SELECT COUNT(*) FROM reward_programs WHERE id = ?1 AND status = 'active'",
            [program_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .unwrap_or(0)
        > 0;
    Ok(active)
}

fn ensure_pilot_reward_program(connection: &Connection) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO reward_programs (
            id, slug, name, status, visibility, terms_json, policy_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, 'active', 'authenticated', ?4, ?5, ?6, ?6)
         ON CONFLICT(id) DO UPDATE SET
            slug = excluded.slug,
            name = excluded.name,
            status = excluded.status,
            visibility = excluded.visibility,
            terms_json = excluded.terms_json,
            policy_json = excluded.policy_json,
            updated_at = excluded.updated_at",
        params![
            PILOT_REWARD_PROGRAM_ID,
            PILOT_REWARD_PROGRAM_SLUG,
            "OrdoStudio NYC Pilot Rewards",
            json!({
                "referralHostedDays": REFERRAL_HOSTED_DAYS,
                "acceptedFeedbackHostedDays": FEEDBACK_HOSTED_DAYS,
                "hostedDaysCap": HOSTED_DAYS_CAP,
                "noCashPayout": true,
                "noScanOnlyReward": true,
                "publicLeaderboard": false,
            })
            .to_string(),
            json!({
                "benefitKind": HOSTED_TIME_BENEFIT_KIND,
                "unit": HOSTED_TIME_UNIT,
                "capQuantity": HOSTED_DAYS_CAP,
                "requiresAccessGrant": true,
                "requiresLedgerEvidence": true,
            })
            .to_string(),
            now,
        ],
    )?;
    seed_reward_rule(
        connection,
        REFERRAL_REWARD_RULE_ID,
        REFERRAL_TRIGGER_KIND,
        REFERRAL_HOSTED_DAYS,
    )?;
    seed_reward_rule(
        connection,
        FEEDBACK_REWARD_RULE_ID,
        FEEDBACK_TRIGGER_KIND,
        FEEDBACK_HOSTED_DAYS,
    )?;
    Ok(())
}

fn seed_reward_rule(
    connection: &Connection,
    rule_id: &str,
    trigger_kind: &str,
    benefit_quantity: i64,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO reward_rules (
            id, program_id, trigger_kind, status, benefit_kind, benefit_quantity,
            benefit_unit, max_quantity_per_actor, qualification_policy_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, 'active', ?4, ?5, ?6, ?7, ?8, ?9, ?9)
         ON CONFLICT(program_id, trigger_kind) DO UPDATE SET
            status = excluded.status,
            benefit_kind = excluded.benefit_kind,
            benefit_quantity = excluded.benefit_quantity,
            benefit_unit = excluded.benefit_unit,
            max_quantity_per_actor = excluded.max_quantity_per_actor,
            qualification_policy_json = excluded.qualification_policy_json,
            updated_at = excluded.updated_at",
        params![
            rule_id,
            PILOT_REWARD_PROGRAM_ID,
            trigger_kind,
            HOSTED_TIME_BENEFIT_KIND,
            benefit_quantity,
            HOSTED_TIME_UNIT,
            HOSTED_DAYS_CAP,
            json!({
                "requiresHumanQualification": true,
                "requiresHostedTrialAccess": true,
                "noScanOnlyReward": true,
                "noCashPayout": true,
            })
            .to_string(),
            now,
        ],
    )?;
    Ok(())
}

fn referral_qualification_context(
    connection: &Connection,
    referral_id: &str,
    request: RewardQualificationRequest,
) -> Result<RewardQualificationContext> {
    let referral_id = require_identifier(referral_id, "Referral id")?;
    ensure!(
        request
            .activation_trial_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
        "referral reward requires referred activation trial evidence"
    );
    let trial_id = require_identifier(
        request.trial_id.as_deref().unwrap_or_default(),
        "Benefit trial id",
    )?;
    let referral = load_referral(connection, &referral_id)?;
    let referrer_connection_id = referral
        .referrer_connection_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("referral reward requires a referrer connection"))?;
    let referred_connection_id = referral
        .referred_connection_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("referral reward requires a referred connection"))?;
    ensure!(
        referrer_connection_id != referred_connection_id,
        "self-referrals cannot qualify for rewards"
    );
    if let Some(connection_id) = request.connection_id.as_deref() {
        ensure!(
            connection_id == referrer_connection_id,
            "reward recipient does not match referral referrer"
        );
    }

    let benefit_trial = load_trial_access_context(connection, &trial_id)?;
    ensure_active_hosted_trial(&benefit_trial, "benefit trial")?;
    ensure!(
        benefit_trial.subject_kind == "connection"
            && benefit_trial.subject_id == referrer_connection_id,
        "benefit trial Access subject must match the referral referrer"
    );

    let activation_trial_id = require_identifier(
        request.activation_trial_id.as_deref().unwrap_or_default(),
        "Activation trial id",
    )?;
    let activation_trial = load_trial_access_context(connection, &activation_trial_id)?;
    ensure_active_hosted_trial(&activation_trial, "activation trial")?;
    ensure!(
        activation_trial.subject_kind == "connection"
            && activation_trial.subject_id == referred_connection_id,
        "activation trial Access subject must match the referred connection"
    );

    let rule = load_reward_rule(connection, REFERRAL_REWARD_RULE_ID)?;
    let amount = qualified_amount(&rule, request.amount)?;
    let mut evidence_refs = referral.evidence_refs;
    evidence_refs.extend(request.evidence_refs);
    evidence_refs.push(format!("referral_record:{referral_id}"));
    evidence_refs.push(format!("trial:{trial_id}"));
    evidence_refs.push(format!("activation_trial:{activation_trial_id}"));
    evidence_refs.push(format!("resource_grant:{}", benefit_trial.access_grant_id));
    evidence_refs = dedupe_nonempty(evidence_refs);
    ensure!(
        evidence_refs.len() >= 4,
        "reward qualification requires referral, activation, trial, and Access evidence"
    );
    let reason = request
        .reason
        .as_deref()
        .unwrap_or("Qualified referral activation grants hosted trial time.")
        .trim()
        .to_string();
    let idempotency_key = request.idempotency_key.unwrap_or_else(|| {
        stable_key(&[
            PILOT_REWARD_PROGRAM_ID,
            REFERRAL_REWARD_RULE_ID,
            "referral_record",
            &referral_id,
            &trial_id,
            &activation_trial_id,
        ])
    });

    Ok(RewardQualificationContext {
        rule,
        source_kind: "referral_record".to_string(),
        source_id: referral_id,
        actor_id: None,
        connection_id: Some(referrer_connection_id),
        trial: benefit_trial,
        requested_amount: amount,
        reason,
        evidence_refs,
        idempotency_key,
        feedback_eligibility_id: None,
    })
}

fn feedback_qualification_context(
    connection: &Connection,
    eligibility_id: &str,
    request: RewardQualificationRequest,
) -> Result<RewardQualificationContext> {
    let eligibility_id = require_identifier(eligibility_id, "Feedback reward eligibility id")?;
    let trial_id = require_identifier(
        request.trial_id.as_deref().unwrap_or_default(),
        "Benefit trial id",
    )?;
    let eligibility = load_feedback_eligibility(connection, &eligibility_id)?;
    ensure!(
        eligibility.request_status == "accepted",
        "feedback reward requires an accepted feedback request"
    );
    ensure!(
        eligibility.state == "pending_qualification",
        "feedback reward eligibility is not pending qualification"
    );
    let benefit_trial = load_trial_access_context(connection, &trial_id)?;
    ensure_active_hosted_trial(&benefit_trial, "benefit trial")?;
    if let Some(actor_id) = eligibility.member_actor_id.as_deref() {
        ensure!(
            benefit_trial.subject_kind == "actor" && benefit_trial.subject_id == actor_id,
            "benefit trial Access subject must match the feedback member"
        );
    } else if let Some(connection_id) = eligibility.connection_id.as_deref() {
        ensure!(
            benefit_trial.subject_kind == "connection" && benefit_trial.subject_id == connection_id,
            "benefit trial Access subject must match the feedback connection"
        );
    } else {
        bail!("feedback reward requires a member actor or connection recipient");
    }
    if let Some(connection_id) = request.connection_id.as_deref() {
        ensure!(
            eligibility.connection_id.as_deref() == Some(connection_id),
            "reward recipient does not match feedback connection"
        );
    }
    if let Some(actor_id) = request.actor_id.as_deref() {
        ensure!(
            eligibility.member_actor_id.as_deref() == Some(actor_id),
            "reward recipient does not match feedback actor"
        );
    }
    let rule = load_reward_rule(connection, FEEDBACK_REWARD_RULE_ID)?;
    let amount = qualified_amount(&rule, request.amount)?;
    let mut evidence_refs = eligibility.evidence_refs;
    evidence_refs.extend(request.evidence_refs);
    evidence_refs.push(format!("feedback_reward_eligibility:{eligibility_id}"));
    evidence_refs.push(format!("feedback_request:{}", eligibility.request_id));
    if let Some(review_id) = eligibility.review_id.as_deref() {
        evidence_refs.push(format!("feedback_request_review:{review_id}"));
    }
    evidence_refs.push(format!("trial:{trial_id}"));
    evidence_refs.push(format!("resource_grant:{}", benefit_trial.access_grant_id));
    evidence_refs = dedupe_nonempty(evidence_refs);
    let reason = request
        .reason
        .as_deref()
        .unwrap_or("Accepted feedback grants hosted trial time.")
        .trim()
        .to_string();
    let idempotency_key = request.idempotency_key.unwrap_or_else(|| {
        stable_key(&[
            PILOT_REWARD_PROGRAM_ID,
            FEEDBACK_REWARD_RULE_ID,
            "feedback_reward_eligibility",
            &eligibility_id,
            &trial_id,
        ])
    });

    Ok(RewardQualificationContext {
        rule,
        source_kind: "feedback_reward_eligibility".to_string(),
        source_id: eligibility_id.clone(),
        actor_id: eligibility.member_actor_id,
        connection_id: eligibility.connection_id,
        trial: benefit_trial,
        requested_amount: amount,
        reason,
        evidence_refs,
        idempotency_key,
        feedback_eligibility_id: Some(eligibility_id),
    })
}

fn insert_reward_qualification(
    connection: &mut Connection,
    context: RewardQualificationContext,
    reviewer_actor_id: Option<&str>,
    event_type_hint: &str,
) -> Result<(String, RealtimeEvent)> {
    let current_total = current_active_total(connection, &context)?;
    let remaining = (context.rule.max_quantity_per_actor - current_total).max(0);
    let granted_amount = context.requested_amount.min(remaining);
    let state = if granted_amount <= 0 {
        "capped"
    } else if granted_amount < context.requested_amount || event_type_hint == "reward.capped" {
        "capped"
    } else {
        "granted"
    };
    let now = Utc::now().to_rfc3339();
    let event_id = format!("reward_event_{}", Uuid::new_v4());
    let benefit_grant_id =
        (granted_amount > 0).then(|| format!("benefit_grant_{}", Uuid::new_v4()));
    let ledger_entry_id =
        (granted_amount > 0).then(|| format!("reward_ledger_entry_{}", Uuid::new_v4()));
    let event_type = if state == "capped" && granted_amount == 0 {
        "reward.capped"
    } else {
        "reward.granted"
    };
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO reward_events (
            id, program_id, rule_id, actor_id, connection_id, source_kind, source_id, state,
            idempotency_key, reason, evidence_refs_json, provenance_json, qualified_at, granted_at,
            capped_at, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?13, ?13)",
        params![
            event_id,
            context.rule.program_id,
            context.rule.id,
            context.actor_id,
            context.connection_id,
            context.source_kind,
            context.source_id,
            state,
            context.idempotency_key,
            safe_text(&context.reason),
            json!(context.evidence_refs).to_string(),
            json!({
                "generator": "rewards.qualify",
                "triggerKind": context.rule.trigger_kind,
                "benefitKind": context.rule.benefit_kind,
                "benefitUnit": context.rule.benefit_unit,
                "requestedAmount": context.requested_amount,
                "grantedAmount": granted_amount,
                "capQuantity": context.rule.max_quantity_per_actor,
                "trialId": context.trial.trial_id,
            })
            .to_string(),
            now,
            (granted_amount > 0).then_some(now.as_str()),
            (state == "capped").then_some(now.as_str()),
        ],
    )?;
    if granted_amount > 0 {
        let ledger_entry_id_ref = ledger_entry_id.as_deref().expect("ledger id");
        let benefit_grant_id_ref = benefit_grant_id.as_deref().expect("benefit id");
        transaction.execute(
            "INSERT INTO reward_ledger_entries (
                id, event_id, program_id, rule_id, actor_id, connection_id, entry_kind, amount,
                unit, benefit_grant_id, reason, evidence_refs_json, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'earn', ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                ledger_entry_id_ref,
                event_id,
                context.rule.program_id,
                context.rule.id,
                context.actor_id,
                context.connection_id,
                granted_amount,
                context.rule.benefit_unit,
                benefit_grant_id_ref,
                safe_text(&context.reason),
                json!(context.evidence_refs).to_string(),
                now,
            ],
        )?;
        let new_trial_ends_at = add_days_to_rfc3339(&context.trial.trial_ends_at, granted_amount)?;
        let new_access_expires_at = context
            .trial
            .access_expires_at
            .as_deref()
            .map(|value| add_days_to_rfc3339(value, granted_amount))
            .transpose()?
            .unwrap_or_else(|| new_trial_ends_at.clone());
        extend_hosted_trial_access_tx(
            &transaction,
            &context,
            &event_id,
            benefit_grant_id_ref,
            ledger_entry_id_ref,
            granted_amount,
            &new_trial_ends_at,
            &new_access_expires_at,
            &now,
        )?;
    }
    upsert_balance_tx(
        &transaction,
        context.rule.program_id.as_str(),
        context.actor_id.as_deref(),
        context.connection_id.as_deref(),
        context.rule.benefit_kind.as_str(),
        context.rule.benefit_unit.as_str(),
        context.rule.max_quantity_per_actor,
        &now,
    )?;
    transaction.execute(
        "INSERT INTO qualification_reviews (
            id, event_id, reviewer_actor_id, decision, reason, evidence_refs_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            format!("qualification_review_{}", Uuid::new_v4()),
            event_id,
            reviewer_actor_id,
            state,
            safe_text(&context.reason),
            json!(context.evidence_refs).to_string(),
            now,
        ],
    )?;
    if let Some(eligibility_id) = context.feedback_eligibility_id.as_deref() {
        transaction.execute(
            "UPDATE feedback_reward_eligibility
             SET state = ?2, reason = ?3, updated_at = ?4
             WHERE id = ?1",
            params![
                eligibility_id,
                if granted_amount > 0 {
                    "qualified"
                } else {
                    "capped"
                },
                if granted_amount > 0 {
                    "Feedback reward was qualified and granted through the reward ledger."
                } else {
                    "Feedback reward was qualified but capped by reward policy."
                },
                now,
            ],
        )?;
    }
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            event_type,
            json!({
                "rewardEventId": event_id,
                "programId": context.rule.program_id,
                "ruleId": context.rule.id,
                "sourceKind": context.source_kind,
                "trialId": context.trial.trial_id,
                "benefitGrantId": benefit_grant_id,
                "ledgerEntryId": ledger_entry_id,
                "state": state,
                "amount": granted_amount,
                "unit": context.rule.benefit_unit,
            }),
        ),
    )?;
    transaction.commit()?;
    Ok((event_id, event))
}

fn extend_hosted_trial_access_tx(
    transaction: &Transaction<'_>,
    context: &RewardQualificationContext,
    event_id: &str,
    benefit_grant_id: &str,
    ledger_entry_id: &str,
    granted_amount: i64,
    new_trial_ends_at: &str,
    new_access_expires_at: &str,
    now: &str,
) -> Result<()> {
    let metadata = json!({
        "schemaVersion": "ordo.reward_benefit_grant.v1",
        "grantKind": "reward_ledger",
        "rewardEventSource": {
            "sourceKind": context.source_kind,
            "sourceId": context.source_id,
        },
        "trialId": context.trial.trial_id,
        "accessGrantId": context.trial.access_grant_id,
        "previousTrialEndsAt": context.trial.trial_ends_at,
        "newTrialEndsAt": new_trial_ends_at,
        "previousAccessExpiresAt": context.trial.access_expires_at,
        "newAccessExpiresAt": new_access_expires_at,
        "benefitGrantId": benefit_grant_id,
        "ledgerEntryId": ledger_entry_id,
        "amount": granted_amount,
        "unit": context.rule.benefit_unit,
    });
    transaction.execute(
        "INSERT INTO benefit_grants (
            id, event_id, ledger_entry_id, actor_id, connection_id, access_grant_id, trial_id,
            benefit_kind, amount, unit, state, starts_at, expires_at, evidence_refs_json,
            metadata_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'active', ?11, ?12, ?13, ?14, ?11, ?11)",
        params![
            benefit_grant_id,
            event_id,
            ledger_entry_id,
            context.actor_id,
            context.connection_id,
            context.trial.access_grant_id,
            context.trial.trial_id,
            context.rule.benefit_kind,
            granted_amount,
            context.rule.benefit_unit,
            now,
            new_trial_ends_at,
            json!(context.evidence_refs).to_string(),
            metadata.to_string(),
        ],
    )?;
    transaction.execute(
        "UPDATE reward_ledger_entries
         SET benefit_grant_id = ?2
         WHERE id = ?1",
        params![ledger_entry_id, benefit_grant_id],
    )?;
    let reward_evidence = json!({
        "rewardBenefitGrantId": benefit_grant_id,
        "rewardLedgerEntryId": ledger_entry_id,
        "previousTrialEndsAt": context.trial.trial_ends_at,
        "newTrialEndsAt": new_trial_ends_at,
    });
    let mut trial_evidence = if context.trial.decision_evidence.is_object() {
        context.trial.decision_evidence.clone()
    } else {
        json!({})
    };
    if let Some(object) = trial_evidence.as_object_mut() {
        object.insert("latestRewardBenefit".to_string(), reward_evidence.clone());
        let reward_benefits = object
            .entry("rewardBenefits".to_string())
            .or_insert_with(|| json!([]));
        if let Some(items) = reward_benefits.as_array_mut() {
            items.push(reward_evidence);
        } else {
            *reward_benefits = json!([reward_evidence]);
        }
    }
    transaction.execute(
        "UPDATE trials
         SET trial_ends_at = ?2,
             decision_evidence_json = ?3,
             updated_at = ?4
         WHERE id = ?1",
        params![
            context.trial.trial_id,
            new_trial_ends_at,
            trial_evidence.to_string(),
            now,
        ],
    )?;
    transaction.execute(
        "UPDATE resource_grants
         SET expires_at = ?2,
             metadata_json = ?3
         WHERE id = ?1",
        params![
            context.trial.access_grant_id,
            new_access_expires_at,
            json!({
                "grantKind": "reward_extended",
                "rewardBenefitGrantId": benefit_grant_id,
                "rewardLedgerEntryId": ledger_entry_id,
                "previousAccessExpiresAt": context.trial.access_expires_at,
                "newAccessExpiresAt": new_access_expires_at,
            })
            .to_string(),
        ],
    )?;
    if let Some(slot_id) = context.trial.slot_id.as_deref() {
        transaction.execute(
            "UPDATE hosted_trial_slots
             SET expires_at = ?2,
                 reset_eligible_at = ?2,
                 updated_at = ?3
             WHERE id = ?1",
            params![slot_id, new_trial_ends_at, now],
        )?;
    }
    transaction.execute(
        "INSERT INTO trial_events (id, trial_id, acceptance_id, event_type, payload_json, occurred_at)
         VALUES (?1, ?2, ?3, 'trial.reward_extended', ?4, ?5)",
        params![
            format!("trial_event_{}", Uuid::new_v4()),
            context.trial.trial_id,
            context.trial.acceptance_id,
            json!({
                "trialId": context.trial.trial_id,
                "accessGrantId": context.trial.access_grant_id,
                "benefitGrantId": benefit_grant_id,
                "ledgerEntryId": ledger_entry_id,
                "previousTrialEndsAt": context.trial.trial_ends_at,
                "newTrialEndsAt": new_trial_ends_at,
                "amount": granted_amount,
                "unit": context.rule.benefit_unit,
            })
            .to_string(),
            now,
        ],
    )?;
    Ok(())
}

fn reject_reward_event_tx(
    transaction: &Transaction<'_>,
    event: &RewardEventRecord,
    reason: &str,
    evidence_refs: &[String],
    reviewer_actor_id: Option<&str>,
    now: &str,
) -> Result<()> {
    ensure!(
        !matches!(event.state.as_str(), "granted" | "reversed"),
        "granted rewards must be reversed instead of rejected"
    );
    transaction.execute(
        "UPDATE reward_events
         SET state = 'rejected', rejected_at = ?2, reason = ?3, updated_at = ?2
         WHERE id = ?1",
        params![event.id, now, safe_text(reason)],
    )?;
    insert_transition_review_tx(
        transaction,
        &event.id,
        reviewer_actor_id,
        "rejected",
        reason,
        evidence_refs,
        now,
    )
}

fn expire_reward_event_tx(
    transaction: &Transaction<'_>,
    event: &RewardEventRecord,
    reason: &str,
    evidence_refs: &[String],
    reviewer_actor_id: Option<&str>,
    now: &str,
) -> Result<()> {
    ensure!(
        !matches!(event.state.as_str(), "granted" | "reversed"),
        "granted rewards must be reversed instead of expired"
    );
    transaction.execute(
        "UPDATE reward_events
         SET state = 'expired', expired_at = ?2, reason = ?3, updated_at = ?2
         WHERE id = ?1",
        params![event.id, now, safe_text(reason)],
    )?;
    transaction.execute(
        "UPDATE benefit_grants
         SET state = 'expired', updated_at = ?2
         WHERE event_id = ?1 AND state = 'active'",
        params![event.id, now],
    )?;
    rebuild_balance_for_event_subject_tx(transaction, event, now)?;
    insert_transition_review_tx(
        transaction,
        &event.id,
        reviewer_actor_id,
        "expired",
        reason,
        evidence_refs,
        now,
    )
}

fn cap_reward_event_tx(
    transaction: &Transaction<'_>,
    event: &RewardEventRecord,
    reason: &str,
    evidence_refs: &[String],
    reviewer_actor_id: Option<&str>,
    now: &str,
) -> Result<()> {
    ensure!(
        !matches!(event.state.as_str(), "granted" | "reversed"),
        "granted rewards must be reversed instead of capped"
    );
    transaction.execute(
        "UPDATE reward_events
         SET state = 'capped', capped_at = ?2, reason = ?3, updated_at = ?2
         WHERE id = ?1",
        params![event.id, now, safe_text(reason)],
    )?;
    insert_transition_review_tx(
        transaction,
        &event.id,
        reviewer_actor_id,
        "capped",
        reason,
        evidence_refs,
        now,
    )
}

fn reverse_reward_event_tx(
    transaction: &Transaction<'_>,
    event: &RewardEventRecord,
    reason: &str,
    evidence_refs: &[String],
    reviewer_actor_id: Option<&str>,
    now: &str,
) -> Result<()> {
    ensure!(
        event.state != "reversed",
        "reward event is already reversed"
    );
    let grant = load_benefit_grant_for_event_tx(transaction, &event.id)?;
    if let Some(grant) = grant.as_ref() {
        let previous_trial_ends_at = grant
            .metadata
            .get("previousTrialEndsAt")
            .and_then(Value::as_str)
            .map(str::to_string);
        let previous_access_expires_at = grant
            .metadata
            .get("previousAccessExpiresAt")
            .and_then(Value::as_str)
            .map(str::to_string);
        if let (Some(trial_id), Some(previous_trial_ends_at)) =
            (grant.trial_id.as_deref(), previous_trial_ends_at.as_deref())
        {
            transaction.execute(
                "UPDATE trials SET trial_ends_at = ?2, updated_at = ?3 WHERE id = ?1",
                params![trial_id, previous_trial_ends_at, now],
            )?;
            transaction.execute(
                "INSERT INTO trial_events (id, trial_id, acceptance_id, event_type, payload_json, occurred_at)
                 SELECT ?1, t.id, t.acceptance_id, 'trial.reward_reversed', ?2, ?3
                 FROM trials t WHERE t.id = ?4",
                params![
                    format!("trial_event_{}", Uuid::new_v4()),
                    json!({
                        "trialId": trial_id,
                        "benefitGrantId": grant.id,
                        "previousTrialEndsAt": previous_trial_ends_at,
                        "reason": safe_text(reason),
                    })
                    .to_string(),
                    now,
                    trial_id,
                ],
            )?;
        }
        if let Some(access_grant_id) = grant.access_grant_id.as_deref() {
            transaction.execute(
                "UPDATE resource_grants SET expires_at = ?2 WHERE id = ?1",
                params![access_grant_id, previous_access_expires_at],
            )?;
        }
        if let (Some(trial_id), Some(previous_trial_ends_at)) =
            (grant.trial_id.as_deref(), previous_trial_ends_at.as_deref())
        {
            transaction.execute(
                "UPDATE hosted_trial_slots
                 SET expires_at = ?2, reset_eligible_at = ?2, updated_at = ?3
                 WHERE trial_id = ?1",
                params![trial_id, previous_trial_ends_at, now],
            )?;
        }
        transaction.execute(
            "UPDATE benefit_grants
             SET state = 'reversed', reversed_at = ?2, updated_at = ?2
             WHERE id = ?1",
            params![grant.id, now],
        )?;
    }
    let reversal_id = format!("reward_ledger_entry_{}", Uuid::new_v4());
    let amount: i64 = transaction
        .query_row(
            "SELECT COALESCE(SUM(amount), 0)
             FROM reward_ledger_entries
             WHERE event_id = ?1 AND entry_kind = 'earn' AND reversed_at IS NULL",
            [event.id.as_str()],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if amount > 0 {
        transaction.execute(
            "INSERT INTO reward_ledger_entries (
                id, event_id, program_id, rule_id, actor_id, connection_id, entry_kind, amount,
                unit, benefit_grant_id, reason, evidence_refs_json, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'reverse', ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                reversal_id,
                event.id,
                event.program_id,
                event.rule_id,
                event.actor_id,
                event.connection_id,
                -amount,
                HOSTED_TIME_UNIT,
                grant.as_ref().map(|grant| grant.id.as_str()),
                safe_text(reason),
                json!(evidence_refs).to_string(),
                now,
            ],
        )?;
        transaction.execute(
            "UPDATE reward_ledger_entries
             SET reversed_at = ?2, reversal_reason = ?3
             WHERE event_id = ?1 AND entry_kind = 'earn' AND reversed_at IS NULL",
            params![event.id, now, safe_text(reason)],
        )?;
    }
    transaction.execute(
        "UPDATE reward_events
         SET state = 'reversed', reversed_at = ?2, reason = ?3, updated_at = ?2
         WHERE id = ?1",
        params![event.id, now, safe_text(reason)],
    )?;
    rebuild_balance_for_event_subject_tx(transaction, event, now)?;
    insert_transition_review_tx(
        transaction,
        &event.id,
        reviewer_actor_id,
        "reversed",
        reason,
        evidence_refs,
        now,
    )
}

fn insert_transition_review_tx(
    transaction: &Transaction<'_>,
    event_id: &str,
    reviewer_actor_id: Option<&str>,
    decision: &str,
    reason: &str,
    evidence_refs: &[String],
    now: &str,
) -> Result<()> {
    transaction.execute(
        "INSERT INTO qualification_reviews (
            id, event_id, reviewer_actor_id, decision, reason, evidence_refs_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            format!("qualification_review_{}", Uuid::new_v4()),
            event_id,
            reviewer_actor_id,
            decision,
            safe_text(reason),
            json!(evidence_refs).to_string(),
            now,
        ],
    )?;
    Ok(())
}

fn upsert_balance_tx(
    transaction: &Transaction<'_>,
    program_id: &str,
    actor_id: Option<&str>,
    connection_id: Option<&str>,
    benefit_kind: &str,
    unit: &str,
    cap_quantity: i64,
    now: &str,
) -> Result<()> {
    let balance_id = balance_id(program_id, actor_id, connection_id, benefit_kind, unit);
    let (total_earned, total_active, total_reversed) = ledger_totals_tx(
        transaction,
        program_id,
        actor_id,
        connection_id,
        benefit_kind,
        unit,
    )?;
    transaction.execute(
        "INSERT INTO benefit_balances (
            id, program_id, actor_id, connection_id, benefit_kind, unit,
            total_earned, total_active, total_reversed, cap_quantity, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(id) DO UPDATE SET
            total_earned = excluded.total_earned,
            total_active = excluded.total_active,
            total_reversed = excluded.total_reversed,
            cap_quantity = excluded.cap_quantity,
            updated_at = excluded.updated_at",
        params![
            balance_id,
            program_id,
            actor_id,
            connection_id,
            benefit_kind,
            unit,
            total_earned,
            total_active,
            total_reversed,
            cap_quantity,
            now,
        ],
    )?;
    Ok(())
}

fn rebuild_balance_for_event_subject_tx(
    transaction: &Transaction<'_>,
    event: &RewardEventRecord,
    now: &str,
) -> Result<()> {
    upsert_balance_tx(
        transaction,
        &event.program_id,
        event.actor_id.as_deref(),
        event.connection_id.as_deref(),
        HOSTED_TIME_BENEFIT_KIND,
        HOSTED_TIME_UNIT,
        HOSTED_DAYS_CAP,
        now,
    )
}

fn ledger_totals_tx(
    transaction: &Transaction<'_>,
    program_id: &str,
    actor_id: Option<&str>,
    connection_id: Option<&str>,
    benefit_kind: &str,
    unit: &str,
) -> Result<(i64, i64, i64)> {
    let total_earned = ledger_total_tx(
        transaction,
        program_id,
        actor_id,
        connection_id,
        benefit_kind,
        unit,
        "earn",
    )?;
    let total_reversed = -ledger_total_tx(
        transaction,
        program_id,
        actor_id,
        connection_id,
        benefit_kind,
        unit,
        "reverse",
    )?;
    let total_active: i64 = transaction.query_row(
        "SELECT COALESCE(SUM(g.amount), 0)
         FROM benefit_grants g
         JOIN reward_events e ON e.id = g.event_id
         WHERE e.program_id = ?1
           AND (?2 IS NULL OR g.actor_id = ?2)
           AND (?3 IS NULL OR g.connection_id = ?3)
           AND g.benefit_kind = ?4
           AND g.unit = ?5
           AND g.state = 'active'",
        params![program_id, actor_id, connection_id, benefit_kind, unit],
        |row| row.get(0),
    )?;
    Ok((total_earned, total_active, total_reversed))
}

fn ledger_total_tx(
    transaction: &Transaction<'_>,
    program_id: &str,
    actor_id: Option<&str>,
    connection_id: Option<&str>,
    benefit_kind: &str,
    unit: &str,
    entry_kind: &str,
) -> Result<i64> {
    transaction
        .query_row(
            "SELECT COALESCE(SUM(l.amount), 0)
             FROM reward_ledger_entries l
             JOIN reward_rules r ON r.id = l.rule_id
             WHERE l.program_id = ?1
               AND (?2 IS NULL OR l.actor_id = ?2)
               AND (?3 IS NULL OR l.connection_id = ?3)
               AND r.benefit_kind = ?4
               AND l.unit = ?5
               AND l.entry_kind = ?6",
            params![
                program_id,
                actor_id,
                connection_id,
                benefit_kind,
                unit,
                entry_kind
            ],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn current_active_total(
    connection: &Connection,
    context: &RewardQualificationContext,
) -> Result<i64> {
    connection
        .query_row(
            "SELECT COALESCE(SUM(g.amount), 0)
             FROM benefit_grants g
             JOIN reward_events e ON e.id = g.event_id
             WHERE e.program_id = ?1
               AND (?2 IS NULL OR g.actor_id = ?2)
               AND (?3 IS NULL OR g.connection_id = ?3)
               AND g.benefit_kind = ?4
               AND g.unit = ?5
               AND g.state = 'active'",
            params![
                context.rule.program_id,
                context.actor_id,
                context.connection_id,
                context.rule.benefit_kind,
                context.rule.benefit_unit
            ],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn load_reward_programs(connection: &Connection) -> Result<Vec<RewardProgramRecord>> {
    let mut statement = connection.prepare(
        "SELECT id, slug, name, status, visibility, terms_json, policy_json, created_at, updated_at
         FROM reward_programs
         ORDER BY updated_at DESC, id ASC",
    )?;
    let rows = statement.query_map([], reward_program_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn load_reward_rule(connection: &Connection, rule_id: &str) -> Result<RewardRuleRecord> {
    connection
        .query_row(
            "SELECT id, program_id, trigger_kind, benefit_kind, benefit_quantity,
                    benefit_unit, max_quantity_per_actor
             FROM reward_rules
             WHERE id = ?1 AND status = 'active'",
            [rule_id],
            |row| {
                Ok(RewardRuleRecord {
                    id: row.get(0)?,
                    program_id: row.get(1)?,
                    trigger_kind: row.get(2)?,
                    benefit_kind: row.get(3)?,
                    benefit_quantity: row.get(4)?,
                    benefit_unit: row.get(5)?,
                    max_quantity_per_actor: row.get(6)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("reward rule is not active: {rule_id}"))
}

fn load_reward_events(connection: &Connection, limit: usize) -> Result<Vec<RewardEventRecord>> {
    let mut statement = connection.prepare(
        "SELECT id, program_id, rule_id, actor_id, connection_id, source_kind, source_id,
                state, reason, evidence_refs_json, provenance_json, qualified_at, granted_at,
                rejected_at, expired_at, capped_at, reversed_at, created_at, updated_at
         FROM reward_events
         ORDER BY updated_at DESC, id ASC
         LIMIT ?1",
    )?;
    let rows = statement.query_map([limit as i64], reward_event_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn load_reward_event(connection: &Connection, event_id: &str) -> Result<RewardEventRecord> {
    connection
        .query_row(
            "SELECT id, program_id, rule_id, actor_id, connection_id, source_kind, source_id,
                    state, reason, evidence_refs_json, provenance_json, qualified_at, granted_at,
                    rejected_at, expired_at, capped_at, reversed_at, created_at, updated_at
             FROM reward_events
             WHERE id = ?1",
            [event_id],
            reward_event_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("reward event was not found: {event_id}"))
}

fn load_reward_event_by_idempotency(
    connection: &Connection,
    idempotency_key: &str,
) -> Result<Option<RewardEventRecord>> {
    connection
        .query_row(
            "SELECT id, program_id, rule_id, actor_id, connection_id, source_kind, source_id,
                    state, reason, evidence_refs_json, provenance_json, qualified_at, granted_at,
                    rejected_at, expired_at, capped_at, reversed_at, created_at, updated_at
             FROM reward_events
             WHERE idempotency_key = ?1",
            [idempotency_key],
            reward_event_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn load_reward_ledger_entries(
    connection: &Connection,
    limit: usize,
) -> Result<Vec<RewardLedgerEntryRecord>> {
    let mut statement = connection.prepare(
        "SELECT id, event_id, program_id, rule_id, actor_id, connection_id, entry_kind,
                amount, unit, benefit_grant_id, reason, evidence_refs_json, created_at,
                reversed_at, reversal_reason
         FROM reward_ledger_entries
         ORDER BY created_at DESC, id ASC
         LIMIT ?1",
    )?;
    let rows = statement.query_map([limit as i64], reward_ledger_entry_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn load_reward_ledger_entry_for_event(
    connection: &Connection,
    event_id: &str,
) -> Result<Option<RewardLedgerEntryRecord>> {
    connection
        .query_row(
            "SELECT id, event_id, program_id, rule_id, actor_id, connection_id, entry_kind,
                    amount, unit, benefit_grant_id, reason, evidence_refs_json, created_at,
                    reversed_at, reversal_reason
             FROM reward_ledger_entries
             WHERE event_id = ?1 AND entry_kind = 'earn'
             ORDER BY created_at DESC, id ASC
             LIMIT 1",
            [event_id],
            reward_ledger_entry_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn load_benefit_grants(connection: &Connection, limit: usize) -> Result<Vec<BenefitGrantRecord>> {
    let mut statement = connection.prepare(
        "SELECT id, event_id, ledger_entry_id, actor_id, connection_id, access_grant_id,
                trial_id, benefit_kind, amount, unit, state, starts_at, expires_at,
                consumed_at, revoked_at, reversed_at, evidence_refs_json, metadata_json,
                created_at, updated_at
         FROM benefit_grants
         ORDER BY updated_at DESC, id ASC
         LIMIT ?1",
    )?;
    let rows = statement.query_map([limit as i64], benefit_grant_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn load_benefit_grant_for_event(
    connection: &Connection,
    event_id: &str,
) -> Result<Option<BenefitGrantRecord>> {
    connection
        .query_row(
            "SELECT id, event_id, ledger_entry_id, actor_id, connection_id, access_grant_id,
                    trial_id, benefit_kind, amount, unit, state, starts_at, expires_at,
                    consumed_at, revoked_at, reversed_at, evidence_refs_json, metadata_json,
                    created_at, updated_at
             FROM benefit_grants
             WHERE event_id = ?1
             ORDER BY updated_at DESC
             LIMIT 1",
            [event_id],
            benefit_grant_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn load_benefit_grant_for_event_tx(
    transaction: &Transaction<'_>,
    event_id: &str,
) -> Result<Option<BenefitGrantRecord>> {
    transaction
        .query_row(
            "SELECT id, event_id, ledger_entry_id, actor_id, connection_id, access_grant_id,
                    trial_id, benefit_kind, amount, unit, state, starts_at, expires_at,
                    consumed_at, revoked_at, reversed_at, evidence_refs_json, metadata_json,
                    created_at, updated_at
             FROM benefit_grants
             WHERE event_id = ?1
             ORDER BY updated_at DESC
             LIMIT 1",
            [event_id],
            benefit_grant_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn load_benefit_balances(
    connection: &Connection,
    limit: usize,
) -> Result<Vec<BenefitBalanceRecord>> {
    let mut statement = connection.prepare(
        "SELECT id, program_id, actor_id, connection_id, benefit_kind, unit, total_earned,
                total_active, total_reversed, cap_quantity, updated_at
         FROM benefit_balances
         ORDER BY updated_at DESC, id ASC
         LIMIT ?1",
    )?;
    let rows = statement.query_map([limit as i64], benefit_balance_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn load_benefit_balance(
    connection: &Connection,
    event: &RewardEventRecord,
) -> Result<Option<BenefitBalanceRecord>> {
    connection
        .query_row(
            "SELECT id, program_id, actor_id, connection_id, benefit_kind, unit, total_earned,
                    total_active, total_reversed, cap_quantity, updated_at
             FROM benefit_balances
             WHERE program_id = ?1
               AND (?2 IS NULL OR actor_id = ?2)
               AND (?3 IS NULL OR connection_id = ?3)
               AND benefit_kind = ?4
               AND unit = ?5
             LIMIT 1",
            params![
                event.program_id,
                event.actor_id,
                event.connection_id,
                HOSTED_TIME_BENEFIT_KIND,
                HOSTED_TIME_UNIT,
            ],
            benefit_balance_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn load_qualification_response(
    connection: &Connection,
    event_id: &str,
    viewer: RewardViewer,
) -> Result<RewardQualificationResponse> {
    let event = load_reward_event(connection, event_id)?;
    let ledger_entry = load_reward_ledger_entry_for_event(connection, event_id)?
        .map(|entry| entry.into_view(viewer));
    let benefit_grant =
        load_benefit_grant_for_event(connection, event_id)?.map(|grant| grant.into_view(viewer));
    let benefit_balance =
        load_benefit_balance(connection, &event)?.map(|balance| balance.into_view(viewer));
    Ok(RewardQualificationResponse {
        event: event.into_view(viewer),
        ledger_entry,
        benefit_grant,
        benefit_balance,
    })
}

fn load_referral(connection: &Connection, referral_id: &str) -> Result<ReferralRecord> {
    connection
        .query_row(
            "SELECT referrer_connection_id, referred_connection_id, evidence_refs_json
             FROM referral_records
             WHERE id = ?1",
            [referral_id],
            |row| {
                let evidence_refs_json: String = row.get(2)?;
                Ok(ReferralRecord {
                    referrer_connection_id: row.get(0)?,
                    referred_connection_id: row.get(1)?,
                    evidence_refs: parse_string_vec(&evidence_refs_json),
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("referral record was not found: {referral_id}"))
}

#[derive(Debug, Clone)]
struct ReferralRecord {
    referrer_connection_id: Option<String>,
    referred_connection_id: Option<String>,
    evidence_refs: Vec<String>,
}

fn load_feedback_eligibility(
    connection: &Connection,
    eligibility_id: &str,
) -> Result<FeedbackEligibilityContext> {
    connection
        .query_row(
            "SELECT e.request_id, e.review_id, e.state,
                    e.evidence_refs_json, r.status, r.member_actor_id, r.connection_id
             FROM feedback_reward_eligibility e
             JOIN feedback_requests r ON r.id = e.request_id
             WHERE e.id = ?1",
            [eligibility_id],
            |row| {
                let evidence_refs_json: String = row.get(3)?;
                Ok(FeedbackEligibilityContext {
                    request_id: row.get(0)?,
                    review_id: row.get(1)?,
                    state: row.get(2)?,
                    evidence_refs: parse_string_vec(&evidence_refs_json),
                    request_status: row.get(4)?,
                    member_actor_id: row.get(5)?,
                    connection_id: row.get(6)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| {
            anyhow::anyhow!("feedback reward eligibility was not found: {eligibility_id}")
        })
}

fn load_trial_access_context(
    connection: &Connection,
    trial_id: &str,
) -> Result<TrialAccessContext> {
    connection
        .query_row(
            "SELECT t.id, t.acceptance_id, t.status, t.trial_ends_at, t.decision_evidence_json,
                    rg.id, rg.subject_kind, rg.subject_id, rg.expires_at,
                    s.id
             FROM trials t
             JOIN resource_grants rg
               ON rg.resource_kind = ?2
              AND rg.resource_id = t.id
              AND rg.action = ?3
              AND rg.effect = 'allow'
             LEFT JOIN hosted_trial_slots s ON s.trial_id = t.id
             WHERE t.id = ?1
             ORDER BY rg.created_at DESC
             LIMIT 1",
            params![trial_id, HOSTED_TRIAL_RESOURCE_KIND, HOSTED_TRIAL_ACTION],
            |row| {
                Ok(TrialAccessContext {
                    trial_id: row.get(0)?,
                    acceptance_id: row.get(1)?,
                    status: row.get(2)?,
                    trial_ends_at: row.get(3)?,
                    decision_evidence: parse_json_object(&row.get::<_, String>(4)?),
                    access_grant_id: row.get(5)?,
                    subject_kind: row.get(6)?,
                    subject_id: row.get(7)?,
                    access_expires_at: row.get(8)?,
                    slot_id: row.get(9)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("hosted trial Access was not found: {trial_id}"))
}

fn ensure_active_hosted_trial(trial: &TrialAccessContext, label: &str) -> Result<()> {
    ensure!(
        matches!(trial.status.as_str(), "started" | "follow_up_needed"),
        "{label} must be active or in follow-up"
    );
    let ends_at = parse_rfc3339(&trial.trial_ends_at)?;
    ensure!(ends_at > Utc::now(), "{label} is already expired");
    Ok(())
}

fn qualified_amount(rule: &RewardRuleRecord, requested: Option<i64>) -> Result<i64> {
    let amount = requested.unwrap_or(rule.benefit_quantity);
    ensure!(amount > 0, "reward amount must be positive");
    ensure!(
        amount <= rule.benefit_quantity,
        "reward amount cannot exceed the active reward rule quantity"
    );
    Ok(amount)
}

fn reward_subject_matches(
    query: &RewardQuery,
    actor_id: Option<&str>,
    connection_id: Option<&str>,
) -> bool {
    if matches!(
        query.viewer,
        RewardViewer::Growth | RewardViewer::Owner | RewardViewer::System
    ) {
        return true;
    }
    query
        .actor_id
        .as_deref()
        .is_some_and(|query_actor_id| actor_id == Some(query_actor_id))
        || query
            .connection_id
            .as_deref()
            .is_some_and(|query_connection_id| connection_id == Some(query_connection_id))
}

fn replay_event(event: &RewardEventRecord) -> RealtimeEvent {
    system_event(
        "reward.qualification.replayed",
        json!({
            "rewardEventId": event.id,
            "state": event.state,
            "sourceKind": event.source_kind,
        }),
    )
}

fn parse_rfc3339(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("invalid RFC3339 timestamp: {value}"))?
        .with_timezone(&Utc))
}

fn add_days_to_rfc3339(value: &str, days: i64) -> Result<String> {
    Ok((parse_rfc3339(value)? + Duration::days(days)).to_rfc3339())
}

fn balance_id(
    program_id: &str,
    actor_id: Option<&str>,
    connection_id: Option<&str>,
    benefit_kind: &str,
    unit: &str,
) -> String {
    format!(
        "benefit_balance_{}",
        stable_key(&[
            program_id,
            actor_id.unwrap_or(""),
            connection_id.unwrap_or(""),
            benefit_kind,
            unit,
        ])
    )
}

fn stable_key(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"\0");
    }
    format!("{:x}", hasher.finalize())
}

fn require_identifier(value: &str, label: &str) -> Result<String> {
    let value = value.trim();
    ensure!(!value.is_empty(), "{label} is required");
    ensure!(
        value
            .chars()
            .all(|character| character.is_ascii_alphanumeric()
                || matches!(character, '_' | '-' | '.')),
        "{label} contains unsupported characters"
    );
    Ok(value.to_string())
}

fn safe_text(value: &str) -> String {
    value.trim().chars().take(280).collect()
}

fn dedupe_nonempty(values: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for value in values {
        let value = value.trim();
        if !value.is_empty() && !result.iter().any(|existing| existing == value) {
            result.push(value.to_string());
        }
    }
    result
}

fn parse_string_vec(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

fn parse_json_object(value: &str) -> Value {
    serde_json::from_str(value).unwrap_or_else(|_| json!({}))
}

fn reward_program_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RewardProgramRecord> {
    let terms_json: String = row.get(5)?;
    let policy_json: String = row.get(6)?;
    Ok(RewardProgramRecord {
        id: row.get(0)?,
        slug: row.get(1)?,
        name: row.get(2)?,
        status: row.get(3)?,
        visibility: row.get(4)?,
        terms: parse_json_object(&terms_json),
        policy: parse_json_object(&policy_json),
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn reward_event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RewardEventRecord> {
    let evidence_refs_json: String = row.get(9)?;
    let provenance_json: String = row.get(10)?;
    Ok(RewardEventRecord {
        id: row.get(0)?,
        program_id: row.get(1)?,
        rule_id: row.get(2)?,
        actor_id: row.get(3)?,
        connection_id: row.get(4)?,
        source_kind: row.get(5)?,
        source_id: row.get(6)?,
        state: row.get(7)?,
        reason: row.get(8)?,
        evidence_refs: parse_string_vec(&evidence_refs_json),
        provenance: parse_json_object(&provenance_json),
        qualified_at: row.get(11)?,
        granted_at: row.get(12)?,
        rejected_at: row.get(13)?,
        expired_at: row.get(14)?,
        capped_at: row.get(15)?,
        reversed_at: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

fn reward_ledger_entry_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<RewardLedgerEntryRecord> {
    let evidence_refs_json: String = row.get(11)?;
    Ok(RewardLedgerEntryRecord {
        id: row.get(0)?,
        event_id: row.get(1)?,
        program_id: row.get(2)?,
        rule_id: row.get(3)?,
        actor_id: row.get(4)?,
        connection_id: row.get(5)?,
        entry_kind: row.get(6)?,
        amount: row.get(7)?,
        unit: row.get(8)?,
        benefit_grant_id: row.get(9)?,
        reason: row.get(10)?,
        evidence_refs: parse_string_vec(&evidence_refs_json),
        created_at: row.get(12)?,
        reversed_at: row.get(13)?,
        reversal_reason: row.get(14)?,
    })
}

fn benefit_grant_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BenefitGrantRecord> {
    let evidence_refs_json: String = row.get(16)?;
    let metadata_json: String = row.get(17)?;
    Ok(BenefitGrantRecord {
        id: row.get(0)?,
        event_id: row.get(1)?,
        ledger_entry_id: row.get(2)?,
        actor_id: row.get(3)?,
        connection_id: row.get(4)?,
        access_grant_id: row.get(5)?,
        trial_id: row.get(6)?,
        benefit_kind: row.get(7)?,
        amount: row.get(8)?,
        unit: row.get(9)?,
        state: row.get(10)?,
        starts_at: row.get(11)?,
        expires_at: row.get(12)?,
        consumed_at: row.get(13)?,
        revoked_at: row.get(14)?,
        reversed_at: row.get(15)?,
        evidence_refs: parse_string_vec(&evidence_refs_json),
        metadata: parse_json_object(&metadata_json),
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

fn benefit_balance_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BenefitBalanceRecord> {
    Ok(BenefitBalanceRecord {
        id: row.get(0)?,
        program_id: row.get(1)?,
        actor_id: row.get(2)?,
        connection_id: row.get(3)?,
        benefit_kind: row.get(4)?,
        unit: row.get(5)?,
        total_earned: row.get(6)?,
        total_active: row.get(7)?,
        total_reversed: row.get(8)?,
        cap_quantity: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

impl RewardProgramRecord {
    fn into_view(self) -> RewardProgramView {
        RewardProgramView {
            id: self.id,
            slug: self.slug,
            name: self.name,
            status: self.status,
            visibility: self.visibility,
            terms: self.terms,
            policy: self.policy,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl RewardEventRecord {
    fn into_view(self, viewer: RewardViewer) -> RewardEventView {
        let safe = viewer == RewardViewer::Member;
        RewardEventView {
            id: self.id,
            program_id: self.program_id,
            rule_id: self.rule_id,
            actor_id: if safe { None } else { self.actor_id },
            connection_id: if safe { None } else { self.connection_id },
            source_kind: self.source_kind,
            source_id: if safe { None } else { Some(self.source_id) },
            state: self.state,
            reason: if safe { None } else { Some(self.reason) },
            evidence_refs: if safe { Vec::new() } else { self.evidence_refs },
            provenance: if safe { json!({}) } else { self.provenance },
            qualified_at: self.qualified_at,
            granted_at: self.granted_at,
            rejected_at: self.rejected_at,
            expired_at: self.expired_at,
            capped_at: self.capped_at,
            reversed_at: self.reversed_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl RewardLedgerEntryRecord {
    fn into_view(self, viewer: RewardViewer) -> RewardLedgerEntryView {
        let safe = viewer == RewardViewer::Member;
        RewardLedgerEntryView {
            id: self.id,
            event_id: self.event_id,
            program_id: self.program_id,
            rule_id: self.rule_id,
            actor_id: if safe { None } else { self.actor_id },
            connection_id: if safe { None } else { self.connection_id },
            entry_kind: self.entry_kind,
            amount: self.amount,
            unit: self.unit,
            benefit_grant_id: self.benefit_grant_id,
            reason: if safe { None } else { Some(self.reason) },
            evidence_refs: if safe { Vec::new() } else { self.evidence_refs },
            created_at: self.created_at,
            reversed_at: self.reversed_at,
            reversal_reason: if safe { None } else { self.reversal_reason },
        }
    }
}

impl BenefitGrantRecord {
    fn into_view(self, viewer: RewardViewer) -> BenefitGrantView {
        let safe = viewer == RewardViewer::Member;
        BenefitGrantView {
            id: self.id,
            event_id: self.event_id,
            ledger_entry_id: self.ledger_entry_id,
            actor_id: if safe { None } else { self.actor_id },
            connection_id: if safe { None } else { self.connection_id },
            access_grant_id: if safe { None } else { self.access_grant_id },
            trial_id: if safe { None } else { self.trial_id },
            benefit_kind: self.benefit_kind,
            amount: self.amount,
            unit: self.unit,
            state: self.state,
            starts_at: self.starts_at,
            expires_at: self.expires_at,
            consumed_at: self.consumed_at,
            revoked_at: self.revoked_at,
            reversed_at: self.reversed_at,
            evidence_refs: if safe { Vec::new() } else { self.evidence_refs },
            metadata: if safe { json!({}) } else { self.metadata },
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl BenefitBalanceRecord {
    fn into_view(self, viewer: RewardViewer) -> BenefitBalanceView {
        let safe = viewer == RewardViewer::Member;
        BenefitBalanceView {
            id: self.id,
            program_id: self.program_id,
            actor_id: if safe { None } else { self.actor_id },
            connection_id: if safe { None } else { self.connection_id },
            benefit_kind: self.benefit_kind,
            unit: self.unit,
            total_earned: self.total_earned,
            total_active: self.total_active,
            total_reversed: self.total_reversed,
            cap_quantity: self.cap_quantity,
            updated_at: self.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_database;
    use rusqlite::params;

    fn setup_db() -> (tempfile::TempDir, std::path::PathBuf) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        insert_reward_fixture(&db_path);
        (temp_dir, db_path)
    }

    fn insert_reward_fixture(db_path: &Path) {
        let connection = Connection::open(db_path).unwrap();
        let now = Utc::now();
        let now_text = now.to_rfc3339();
        let ends_at = (now + Duration::days(30)).to_rfc3339();
        connection
            .execute_batch(
                r#"
                INSERT INTO connections (
                    id, connection_type, display_name, status, identity_json, scope_json,
                    metadata_json, created_at, updated_at, activated_at
                ) VALUES
                    ('connection_referrer', 'member', 'Referrer', 'active', '{}', '{}', '{}', '2026-05-13T00:00:00Z', '2026-05-13T00:00:00Z', '2026-05-13T00:00:00Z'),
                    ('connection_referred', 'member', 'Referred', 'active', '{}', '{}', '{}', '2026-05-13T00:00:00Z', '2026-05-13T00:00:00Z', '2026-05-13T00:00:00Z');

                INSERT INTO offers (
                    id, slug, title, summary, status, visibility, publication_state, trial_days,
                    source_kind, terms_json, metadata_json, created_at, updated_at, published_at
                ) VALUES (
                    'offer_pilot', 'nyc-pilot', 'NYC Pilot', 'Thirty day hosted Ordo trial.',
                    'available', 'public', 'published', 30, 'test', '{}', '{}',
                    '2026-05-13T00:00:00Z', '2026-05-13T00:00:00Z', '2026-05-13T00:00:00Z'
                );

                INSERT INTO hosted_trial_capacity_policies (
                    id, offer_id, offer_slug, status, active_slot_limit, trial_days,
                    backup_before_wipe_required, reset_grace_days, metadata_json, created_at, updated_at
                ) VALUES (
                    'capacity_policy_1', 'offer_pilot', 'nyc-pilot', 'active', 10, 30, 1, 0, '{}',
                    '2026-05-13T00:00:00Z', '2026-05-13T00:00:00Z'
                );
                "#,
            )
            .unwrap();
        insert_trial(
            &connection,
            "benefit",
            "connection_referrer",
            &now_text,
            &ends_at,
        );
        insert_trial(
            &connection,
            "activation",
            "connection_referred",
            &now_text,
            &ends_at,
        );
        connection
            .execute(
                "INSERT INTO referral_records (
                    id, status, referrer_connection_id, referred_connection_id, evidence_refs_json,
                    provenance_json, created_at, updated_at
                 ) VALUES (
                    'referral_1', 'captured', 'connection_referrer', 'connection_referred',
                    '[\"tracked_entry_point:entry_1\", \"visitor_session:session_1\"]',
                    '{\"generator\":\"test\"}', ?1, ?1
                 )",
                [now_text.as_str()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO feedback_requests (
                    id, target_kind, target_id, connection_id, source_kind, source_id,
                    prompt, member_context_summary, status, priority, evidence_refs_json,
                    provenance_json, created_at, updated_at, closed_at
                 ) VALUES (
                    'feedback_request_1', 'trial', 'trial_benefit', 'connection_referrer',
                    'trial', 'trial_benefit', 'How did the pilot go?', 'Feedback on the active trial.',
                    'accepted', 'normal', '[\"feedback_request:feedback_request_1\"]',
                    '{\"generator\":\"test\"}', ?1, ?1, ?1
                 )",
                [now_text.as_str()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO feedback_reward_eligibility (
                    id, request_id, actor_id, state, reason, evidence_refs_json, created_at, updated_at
                 ) VALUES (
                    'feedback_reward_eligibility_1', 'feedback_request_1', 'actor_local_owner',
                    'pending_qualification', 'Accepted feedback pending reward qualification.',
                    '[\"feedback_request_review:review_1\"]', ?1, ?1
                 )",
                [now_text.as_str()],
            )
            .unwrap();
    }

    fn insert_trial(
        connection: &Connection,
        suffix: &str,
        subject_id: &str,
        now: &str,
        ends_at: &str,
    ) {
        let acceptance_id = format!("offer_acceptance_{suffix}");
        let trial_id = format!("trial_{suffix}");
        let grant_id = format!("resource_grant_{suffix}");
        let slot_id = format!("hosted_trial_slot_{suffix}");
        connection
            .execute(
                "INSERT INTO offer_acceptances (
                    id, offer_id, offer_slug, offer_title, attribution_json,
                    acceptance_context_json, idempotency_key, access_grant_id, receipt_json,
                    status, accepted_at, created_at, updated_at
                 ) VALUES (?1, 'offer_pilot', 'nyc-pilot', 'NYC Pilot', '{}', '{}', ?2, ?3, '{}', 'accepted', ?4, ?4, ?4)",
                params![acceptance_id, format!("key_{suffix}"), grant_id, now],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO trials (
                    id, acceptance_id, offer_id, offer_slug, status, started_at, trial_ends_at,
                    decision_evidence_json, created_at, updated_at
                 ) VALUES (?1, ?2, 'offer_pilot', 'nyc-pilot', 'started', ?3, ?4, '{}', ?3, ?3)",
                params![trial_id, acceptance_id, now, ends_at],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO hosted_trial_slots (
                    id, policy_id, trial_id, acceptance_id, offer_id, offer_slug,
                    subject_kind, subject_id, status, allocated_at, expires_at,
                    backup_required, backup_status, backup_evidence_json, reset_eligible_at,
                    reset_state, reset_guard_json, owner_override_json, created_at, updated_at
                 ) VALUES (
                    ?1, 'capacity_policy_1', ?2, ?3, 'offer_pilot', 'nyc-pilot',
                    'connection', ?4, 'active', ?5, ?6, 1, 'required', '[]', ?6,
                    'blocked_until_expiration', '{}', '{}', ?5, ?5
                 )",
                params![slot_id, trial_id, acceptance_id, subject_id, now, ends_at],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO resource_grants (
                    id, resource_kind, resource_id, action, subject_kind, subject_id,
                    effect, created_at, expires_at, metadata_json
                 ) VALUES (?1, ?2, ?3, ?4, 'connection', ?5, 'allow', ?6, ?7, '{}')",
                params![
                    grant_id,
                    HOSTED_TRIAL_RESOURCE_KIND,
                    trial_id,
                    HOSTED_TRIAL_ACTION,
                    subject_id,
                    now,
                    ends_at
                ],
            )
            .unwrap();
    }

    fn insert_existing_active_reward_at_cap(connection: &Connection) {
        connection
            .execute_batch(
                r#"
                INSERT INTO reward_events (
                    id, program_id, rule_id, connection_id, source_kind, source_id, state,
                    idempotency_key, reason, evidence_refs_json, provenance_json,
                    qualified_at, granted_at, created_at, updated_at
                ) VALUES (
                    'reward_event_existing_cap', 'reward_program_ordostudio_nyc_pilot',
                    'reward_rule_ordostudio_referral_trial_activation', 'connection_referrer',
                    'referral_record', 'referral_existing_cap', 'granted',
                    'existing_cap_key', 'Existing capped rewards.',
                    '["reward_event:existing_cap"]', '{"generator":"test"}',
                    '2026-05-13T00:00:00Z', '2026-05-13T00:00:00Z',
                    '2026-05-13T00:00:00Z', '2026-05-13T00:00:00Z'
                );
                INSERT INTO reward_ledger_entries (
                    id, event_id, program_id, rule_id, connection_id, entry_kind, amount,
                    unit, benefit_grant_id, reason, evidence_refs_json, created_at
                ) VALUES (
                    'reward_ledger_entry_existing_cap', 'reward_event_existing_cap',
                    'reward_program_ordostudio_nyc_pilot',
                    'reward_rule_ordostudio_referral_trial_activation', 'connection_referrer',
                    'earn', 30, 'day', 'benefit_grant_existing_cap',
                    'Existing capped rewards.', '["reward_event:existing_cap"]',
                    '2026-05-13T00:00:00Z'
                );
                INSERT INTO benefit_grants (
                    id, event_id, ledger_entry_id, connection_id, access_grant_id, trial_id,
                    benefit_kind, amount, unit, state, starts_at, expires_at,
                    evidence_refs_json, metadata_json, created_at, updated_at
                ) VALUES (
                    'benefit_grant_existing_cap', 'reward_event_existing_cap',
                    'reward_ledger_entry_existing_cap', 'connection_referrer',
                    'resource_grant_benefit', 'trial_benefit', 'hosted_trial_time',
                    30, 'day', 'active', '2026-05-13T00:00:00Z',
                    '2026-06-12T00:00:00Z', '["reward_event:existing_cap"]',
                    '{}', '2026-05-13T00:00:00Z', '2026-05-13T00:00:00Z'
                );
                "#,
            )
            .unwrap();
    }

    #[test]
    fn qualified_referral_grants_hosted_days_through_access_and_is_idempotent() {
        let (_temp_dir, db_path) = setup_db();
        let connection = Connection::open(&db_path).unwrap();
        connection
            .execute(
                "UPDATE trials
                 SET decision_evidence_json = '{\"acceptedOfferReceiptId\":\"receipt_1\"}'
                 WHERE id = 'trial_benefit'",
                [],
            )
            .unwrap();
        drop(connection);

        let (response, event) = qualify_referral_reward(
            &db_path,
            "referral_1",
            RewardQualificationRequest {
                trial_id: Some("trial_benefit".to_string()),
                activation_trial_id: Some("trial_activation".to_string()),
                connection_id: Some("connection_referrer".to_string()),
                evidence_refs: vec!["business_outcome:accepted_trial".to_string()],
                reason: Some("Referred user activated a trial.".to_string()),
                ..RewardQualificationRequest::default()
            },
            Some("actor_local_owner"),
        )
        .unwrap();

        assert_eq!(event.event_type, "reward.granted");
        assert_eq!(response.event.state, "granted");
        assert_eq!(
            response.ledger_entry.as_ref().unwrap().amount,
            REFERRAL_HOSTED_DAYS
        );
        assert_eq!(response.benefit_grant.as_ref().unwrap().state, "active");
        assert_eq!(
            response.benefit_balance.as_ref().unwrap().total_active,
            REFERRAL_HOSTED_DAYS
        );

        let connection = Connection::open(&db_path).unwrap();
        let trial_ends_at: String = connection
            .query_row(
                "SELECT trial_ends_at FROM trials WHERE id = 'trial_benefit'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let access_expires_at: String = connection
            .query_row(
                "SELECT expires_at FROM resource_grants WHERE id = 'resource_grant_benefit'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(trial_ends_at, access_expires_at);
        let reward_extension_events: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM trial_events WHERE event_type = 'trial.reward_extended'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(reward_extension_events, 1);
        let decision_evidence_json: String = connection
            .query_row(
                "SELECT decision_evidence_json FROM trials WHERE id = 'trial_benefit'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let decision_evidence = parse_json_object(&decision_evidence_json);
        assert_eq!(decision_evidence["acceptedOfferReceiptId"], "receipt_1");
        assert_eq!(
            decision_evidence["latestRewardBenefit"]["rewardLedgerEntryId"],
            response.ledger_entry.as_ref().unwrap().id
        );
        assert_eq!(
            decision_evidence["rewardBenefits"].as_array().map(Vec::len),
            Some(1)
        );

        let replay = qualify_referral_reward(
            &db_path,
            "referral_1",
            RewardQualificationRequest {
                trial_id: Some("trial_benefit".to_string()),
                activation_trial_id: Some("trial_activation".to_string()),
                connection_id: Some("connection_referrer".to_string()),
                ..RewardQualificationRequest::default()
            },
            Some("actor_local_owner"),
        )
        .unwrap();
        assert_eq!(replay.0.event.id, response.event.id);
        assert_eq!(replay.1.event_type, "reward.qualification.replayed");
        let ledger_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM reward_ledger_entries", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(ledger_count, 1);
    }

    #[test]
    fn accepted_feedback_grants_benefit_and_member_view_is_redacted() {
        let (_temp_dir, db_path) = setup_db();

        let (response, _) = qualify_feedback_reward(
            &db_path,
            "feedback_reward_eligibility_1",
            RewardQualificationRequest {
                trial_id: Some("trial_benefit".to_string()),
                connection_id: Some("connection_referrer".to_string()),
                evidence_refs: vec!["feedback_request_response:response_private".to_string()],
                ..RewardQualificationRequest::default()
            },
            Some("actor_local_owner"),
        )
        .unwrap();

        assert_eq!(response.event.state, "granted");
        assert_eq!(
            response.ledger_entry.as_ref().unwrap().amount,
            FEEDBACK_HOSTED_DAYS
        );
        let connection = Connection::open(&db_path).unwrap();
        let eligibility_state: String = connection
            .query_row(
                "SELECT state FROM feedback_reward_eligibility WHERE id = 'feedback_reward_eligibility_1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(eligibility_state, "qualified");

        let member_rewards = list_rewards(
            &db_path,
            RewardQuery {
                viewer: RewardViewer::Member,
                connection_id: Some("connection_referrer".to_string()),
                ..RewardQuery::default()
            },
        )
        .unwrap();
        assert_eq!(member_rewards.events.len(), 1);
        let member_json = serde_json::to_string(&member_rewards).unwrap();
        assert!(!member_json.contains("response_private"));
        assert!(!member_json.contains("feedback_reward_eligibility_1"));
        assert!(!member_json.contains("actor_local_owner"));
        assert!(!member_json.contains("connection_referrer"));
        assert!(member_rewards.events[0].source_id.is_none());
        assert!(member_rewards.events[0].reason.is_none());
        assert!(member_rewards.events[0].evidence_refs.is_empty());
    }

    #[test]
    fn capped_feedback_reward_records_event_without_mutating_access_or_balance() {
        let (_temp_dir, db_path) = setup_db();
        let connection = Connection::open(&db_path).unwrap();
        assert!(reward_program_is_active(&connection, PILOT_REWARD_PROGRAM_ID).unwrap());
        insert_existing_active_reward_at_cap(&connection);
        let trial_ends_before: String = connection
            .query_row(
                "SELECT trial_ends_at FROM trials WHERE id = 'trial_benefit'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);

        let (response, event) = qualify_feedback_reward(
            &db_path,
            "feedback_reward_eligibility_1",
            RewardQualificationRequest {
                trial_id: Some("trial_benefit".to_string()),
                connection_id: Some("connection_referrer".to_string()),
                ..RewardQualificationRequest::default()
            },
            Some("actor_local_owner"),
        )
        .unwrap();

        assert_eq!(event.event_type, "reward.capped");
        assert_eq!(response.event.state, "capped");
        assert!(response.ledger_entry.is_none());
        assert!(response.benefit_grant.is_none());
        assert_eq!(
            response.benefit_balance.as_ref().unwrap().total_active,
            HOSTED_DAYS_CAP
        );
        let connection = Connection::open(&db_path).unwrap();
        let trial_ends_after: String = connection
            .query_row(
                "SELECT trial_ends_at FROM trials WHERE id = 'trial_benefit'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(trial_ends_after, trial_ends_before);
        let eligibility_state: String = connection
            .query_row(
                "SELECT state FROM feedback_reward_eligibility WHERE id = 'feedback_reward_eligibility_1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(eligibility_state, "capped");
        let ledger_amount: i64 = connection
            .query_row(
                "SELECT COALESCE(SUM(amount), 0) FROM reward_ledger_entries WHERE connection_id = 'connection_referrer'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(ledger_amount, HOSTED_DAYS_CAP);
    }

    #[test]
    fn referral_cap_is_connection_scoped_when_request_supplies_actor() {
        let (_temp_dir, db_path) = setup_db();
        let connection = Connection::open(&db_path).unwrap();
        assert!(reward_program_is_active(&connection, PILOT_REWARD_PROGRAM_ID).unwrap());
        insert_existing_active_reward_at_cap(&connection);
        let trial_ends_before: String = connection
            .query_row(
                "SELECT trial_ends_at FROM trials WHERE id = 'trial_benefit'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);

        let (response, event) = qualify_referral_reward(
            &db_path,
            "referral_1",
            RewardQualificationRequest {
                trial_id: Some("trial_benefit".to_string()),
                activation_trial_id: Some("trial_activation".to_string()),
                actor_id: Some("actor_untrusted_request_body".to_string()),
                connection_id: Some("connection_referrer".to_string()),
                ..RewardQualificationRequest::default()
            },
            Some("actor_local_owner"),
        )
        .unwrap();

        assert_eq!(event.event_type, "reward.capped");
        assert_eq!(response.event.state, "capped");
        assert!(response.event.actor_id.is_none());
        assert!(response.ledger_entry.is_none());
        assert!(response.benefit_grant.is_none());
        assert_eq!(
            response.benefit_balance.as_ref().unwrap().total_active,
            HOSTED_DAYS_CAP
        );
        let connection = Connection::open(&db_path).unwrap();
        let trial_ends_after: String = connection
            .query_row(
                "SELECT trial_ends_at FROM trials WHERE id = 'trial_benefit'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(trial_ends_after, trial_ends_before);
        let leaked_actor_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM reward_events WHERE actor_id = 'actor_untrusted_request_body'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(leaked_actor_count, 0);
    }

    #[test]
    fn granted_reward_cannot_be_expired_or_capped_without_reversal() {
        let (_temp_dir, db_path) = setup_db();
        let (response, _) = qualify_referral_reward(
            &db_path,
            "referral_1",
            RewardQualificationRequest {
                trial_id: Some("trial_benefit".to_string()),
                activation_trial_id: Some("trial_activation".to_string()),
                connection_id: Some("connection_referrer".to_string()),
                ..RewardQualificationRequest::default()
            },
            Some("actor_local_owner"),
        )
        .unwrap();
        assert_eq!(response.event.state, "granted");

        let connection = Connection::open(&db_path).unwrap();
        let trial_ends_after_grant: String = connection
            .query_row(
                "SELECT trial_ends_at FROM trials WHERE id = 'trial_benefit'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);

        let expire_error = transition_reward_event(
            &db_path,
            &response.event.id,
            RewardEventTransitionRequest {
                state: "expired".to_string(),
                reason: "Reward aged out.".to_string(),
                evidence_refs: vec!["manual_review:expire".to_string()],
            },
            Some("actor_local_owner"),
        )
        .unwrap_err()
        .to_string();
        assert!(expire_error.contains("granted rewards must be reversed"));

        let cap_error = transition_reward_event(
            &db_path,
            &response.event.id,
            RewardEventTransitionRequest {
                state: "capped".to_string(),
                reason: "Reward should be capped.".to_string(),
                evidence_refs: vec!["manual_review:cap".to_string()],
            },
            Some("actor_local_owner"),
        )
        .unwrap_err()
        .to_string();
        assert!(cap_error.contains("granted rewards must be reversed"));

        let connection = Connection::open(&db_path).unwrap();
        let event_state: String = connection
            .query_row(
                "SELECT state FROM reward_events WHERE id = ?1",
                [response.event.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_state, "granted");
        let grant_state: String = connection
            .query_row(
                "SELECT state FROM benefit_grants WHERE event_id = ?1",
                [response.event.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(grant_state, "active");
        let trial_ends_after_failed_transition: String = connection
            .query_row(
                "SELECT trial_ends_at FROM trials WHERE id = 'trial_benefit'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(trial_ends_after_failed_transition, trial_ends_after_grant);
        let active_balance: i64 = connection
            .query_row(
                "SELECT total_active FROM benefit_balances WHERE connection_id = 'connection_referrer'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(active_balance, REFERRAL_HOSTED_DAYS);
    }

    #[test]
    fn scan_only_self_referral_and_reversal_do_not_fake_balances() {
        let (_temp_dir, db_path) = setup_db();

        let scan_only = qualify_referral_reward(
            &db_path,
            "referral_1",
            RewardQualificationRequest {
                trial_id: Some("trial_benefit".to_string()),
                connection_id: Some("connection_referrer".to_string()),
                ..RewardQualificationRequest::default()
            },
            Some("actor_local_owner"),
        )
        .unwrap_err()
        .to_string();
        assert!(scan_only.contains("activation trial"));

        let connection = Connection::open(&db_path).unwrap();
        connection
            .execute(
                "INSERT INTO referral_records (
                    id, status, referrer_connection_id, referred_connection_id, evidence_refs_json,
                    provenance_json, created_at, updated_at
                 ) VALUES (
                    'referral_self', 'captured', 'connection_referrer', 'connection_referrer',
                    '[\"tracked_entry_point:entry_1\"]', '{\"generator\":\"test\"}',
                    '2026-05-13T00:00:00Z', '2026-05-13T00:00:00Z'
                 )",
                [],
            )
            .unwrap();
        let self_referral = qualify_referral_reward(
            &db_path,
            "referral_self",
            RewardQualificationRequest {
                trial_id: Some("trial_benefit".to_string()),
                activation_trial_id: Some("trial_activation".to_string()),
                connection_id: Some("connection_referrer".to_string()),
                ..RewardQualificationRequest::default()
            },
            Some("actor_local_owner"),
        )
        .unwrap_err()
        .to_string();
        assert!(self_referral.contains("self-referrals"));

        let (response, _) = qualify_referral_reward(
            &db_path,
            "referral_1",
            RewardQualificationRequest {
                trial_id: Some("trial_benefit".to_string()),
                activation_trial_id: Some("trial_activation".to_string()),
                connection_id: Some("connection_referrer".to_string()),
                ..RewardQualificationRequest::default()
            },
            Some("actor_local_owner"),
        )
        .unwrap();
        let before_reversal: String = connection
            .query_row(
                "SELECT trial_ends_at FROM trials WHERE id = 'trial_benefit'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let (reversed, _) = transition_reward_event(
            &db_path,
            &response.event.id,
            RewardEventTransitionRequest {
                state: "reversed".to_string(),
                reason: "Referral was later disqualified.".to_string(),
                evidence_refs: vec!["manual_review:reversal".to_string()],
            },
            Some("actor_local_owner"),
        )
        .unwrap();
        assert_eq!(reversed.event.state, "reversed");
        assert_eq!(reversed.benefit_balance.as_ref().unwrap().total_active, 0);
        assert_eq!(
            reversed.benefit_balance.as_ref().unwrap().total_reversed,
            REFERRAL_HOSTED_DAYS
        );
        let after_reversal: String = connection
            .query_row(
                "SELECT trial_ends_at FROM trials WHERE id = 'trial_benefit'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_ne!(before_reversal, after_reversal);
    }
}
