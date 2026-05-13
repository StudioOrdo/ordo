use anyhow::{anyhow, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::eval_artifact_review::{
    EvalArtifactFindingSeverity, EvalArtifactReview, EVAL_ARTIFACT_REVIEW_SCHEMA_VERSION,
};
use crate::eval_harness::{EvalArtifactPacket, EvalFindingCategory};
use crate::live_eval_runner::{
    ADMIN_STAFF_JOURNEY_SCHEMA_VERSION, AFFILIATE_REFERRAL_JOURNEY_SCHEMA_VERSION,
    QR_TO_TRIAL_JOURNEY_SCHEMA_VERSION, REVIEW_RETURN_JOURNEY_SCHEMA_VERSION,
};
use crate::security::{
    artifact_boundary::resolve_existing_artifact_path, markdown::sanitize_markdown_links, redaction,
};

pub const LIVE_JOURNEY_REPORT_SCHEMA_VERSION: &str = "ordo.live_journey_report.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveJourneyReportRequest {
    pub journey_manifest_paths: Vec<PathBuf>,
    pub artifact_review_paths: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub source_commit: String,
    pub generated_at: String,
    pub private_terms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveJourneyReportPaths {
    pub report_json_path: PathBuf,
    pub report_markdown_path: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyReport {
    pub schema_version: String,
    pub source_commit: String,
    pub generated_at: String,
    pub input_artifacts: Vec<LiveJourneyInputArtifactRef>,
    pub persona_roster: Vec<LiveJourneyPersonaSummary>,
    pub case_summaries: Vec<LiveJourneyCaseSummary>,
    pub qr_entry_summary: LiveJourneyCountSummary,
    pub visitor_session_summary: LiveJourneyCountSummary,
    pub conversation_summary: LiveJourneyCountSummary,
    pub conversion_summary: LiveJourneyConversionSummary,
    pub review_summary: LiveJourneyReviewSummary,
    pub referral_summary: LiveJourneyReferralSummary,
    pub handoff_moderation_summary: LiveJourneyHandoffSummary,
    pub privacy_summary: LiveJourneyPrivacySummary,
    pub accounting_summary: LiveJourneyAccountingSummary,
    pub artifact_finding_summary: LiveJourneyArtifactFindingSummary,
    pub persuasion_boundary_summary: LiveJourneyPersuasionSummary,
    pub unexercised_gaps: Vec<LiveJourneyGap>,
    pub follow_up_issue_drafts: Vec<LiveJourneyIssueDraft>,
    pub issue_filing: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyInputArtifactRef {
    pub path: String,
    pub artifact_kind: String,
    pub schema_version: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyPersonaSummary {
    pub persona_id: String,
    pub content_hash: Option<String>,
    pub case_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyCaseSummary {
    pub case_id: String,
    pub journey_kind: String,
    pub persona_id: String,
    pub status: String,
    pub provider_mode: String,
    pub network_enabled: bool,
    pub packet_path: Option<String>,
    pub scorecard_path: Option<String>,
    pub journey_manifest_path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyCountSummary {
    pub total: usize,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyConversionSummary {
    pub qr_cases: usize,
    pub offer_acceptances: usize,
    pub trials_started: usize,
    pub outcomes: usize,
    pub attributions: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyReviewSummary {
    pub review_return_cases: usize,
    pub simulated_email_artifacts: usize,
    pub feedback_items: usize,
    pub review_candidates: usize,
    pub blocked_before_consent_or_approval: usize,
    pub published_reviews: usize,
    pub retired_reviews: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyReferralSummary {
    pub affiliate_referral_cases: usize,
    pub affiliate_connections: usize,
    pub referral_entry_points: usize,
    pub referral_records: usize,
    pub referral_outcomes: usize,
    pub attribution_count: usize,
    pub unrelated_access_denials: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyHandoffSummary {
    pub admin_staff_cases: usize,
    pub handoffs: usize,
    pub closed_handoffs: usize,
    pub human_led_blocks: usize,
    pub delegation_allows: usize,
    pub returned_mode_allows: usize,
    pub review_moderation_publications: usize,
    pub affiliate_grants_revoked_and_denied: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyPrivacySummary {
    pub packet_count: usize,
    pub redaction_applied_count: usize,
    pub redacted_value_count: usize,
    pub private_term_count: usize,
    pub detectors: Vec<String>,
    pub report_redacted: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyAccountingSummary {
    pub packets_with_prompt_slots: usize,
    pub prompt_slot_entries: usize,
    pub packets_with_privacy_transforms: usize,
    pub privacy_transform_entries: usize,
    pub packets_with_token_ledger: usize,
    pub token_ledger_entries: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyArtifactFindingSummary {
    pub review_artifact_count: usize,
    pub finding_count: usize,
    pub highest_severity: Option<EvalArtifactFindingSeverity>,
    pub by_category: BTreeMap<String, usize>,
    pub by_severity: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyPersuasionSummary {
    pub evidence_backed_offer_language_cases: usize,
    pub no_fake_urgency_cases: usize,
    pub no_fake_scarcity_cases: usize,
    pub no_fake_review_or_metric_cases: usize,
    pub agency_preserving_cases: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyGap {
    pub gap_id: String,
    pub category: String,
    pub severity: String,
    pub summary: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJourneyIssueDraft {
    pub title: String,
    pub body: String,
    pub evidence_refs: Vec<String>,
    pub filing_status: String,
}

#[derive(Debug, Default)]
struct ReportAccumulator {
    input_artifacts: Vec<LiveJourneyInputArtifactRef>,
    persona_map: BTreeMap<String, LiveJourneyPersonaSummary>,
    case_summaries: Vec<LiveJourneyCaseSummary>,
    qr_entry_evidence: BTreeSet<String>,
    visitor_session_evidence: BTreeSet<String>,
    conversation_evidence: BTreeSet<String>,
    conversion: ConversionAccumulator,
    review: LiveJourneyReviewSummary,
    referral: LiveJourneyReferralSummary,
    handoff: LiveJourneyHandoffSummary,
    privacy: PrivacyAccumulator,
    accounting: LiveJourneyAccountingSummary,
    persuasion: LiveJourneyPersuasionSummary,
    artifact_reviews: Vec<EvalArtifactReview>,
}

#[derive(Debug, Default)]
struct ConversionAccumulator {
    qr_cases: usize,
    offer_acceptances: usize,
    trials_started: usize,
    outcomes: usize,
    attributions: usize,
}

#[derive(Debug, Default)]
struct PrivacyAccumulator {
    packet_count: usize,
    redaction_applied_count: usize,
    redacted_value_count: usize,
    private_term_count: usize,
    detectors: BTreeSet<String>,
}

pub fn generate_live_journey_report(
    request: LiveJourneyReportRequest,
) -> Result<LiveJourneyReportPaths> {
    ensure!(
        !request.journey_manifest_paths.is_empty(),
        "live journey report requires at least one journey manifest"
    );
    fs::create_dir_all(&request.output_dir).with_context(|| {
        format!(
            "create live journey report output dir {}",
            request.output_dir.display()
        )
    })?;
    let report = build_live_journey_report(&request)?;
    let report_json_path = request.output_dir.join("live-journey-report.json");
    let report_markdown_path = request.output_dir.join("live-journey-report.md");
    write_json(&report_json_path, &report)?;
    fs::write(
        &report_markdown_path,
        sanitize_markdown_links(&render_live_journey_report_markdown(&report)),
    )
    .with_context(|| {
        format!(
            "write live journey report {}",
            report_markdown_path.display()
        )
    })?;
    validate_report_outputs(
        &report_json_path,
        &report_markdown_path,
        &request.private_terms,
    )?;
    Ok(LiveJourneyReportPaths {
        report_json_path,
        report_markdown_path,
    })
}

pub fn build_live_journey_report(request: &LiveJourneyReportRequest) -> Result<LiveJourneyReport> {
    let mut accumulator = ReportAccumulator::default();
    for path in &request.journey_manifest_paths {
        read_journey_manifest(path, &mut accumulator)?;
    }
    for path in &request.artifact_review_paths {
        read_artifact_review(path, &mut accumulator)?;
    }

    let mut persona_roster = accumulator.persona_map.into_values().collect::<Vec<_>>();
    for persona in &mut persona_roster {
        persona.case_ids.sort();
        persona.case_ids.dedup();
    }
    let mut case_summaries = accumulator.case_summaries;
    case_summaries.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    let artifact_finding_summary = artifact_finding_summary(&accumulator.artifact_reviews);
    let unexercised_gaps = unexercised_gaps(&case_summaries);
    let follow_up_issue_drafts =
        follow_up_issue_drafts(&unexercised_gaps, &accumulator.artifact_reviews);

    let report = LiveJourneyReport {
        schema_version: LIVE_JOURNEY_REPORT_SCHEMA_VERSION.to_string(),
        source_commit: request.source_commit.clone(),
        generated_at: request.generated_at.clone(),
        input_artifacts: accumulator.input_artifacts,
        persona_roster,
        case_summaries,
        qr_entry_summary: LiveJourneyCountSummary {
            total: accumulator.qr_entry_evidence.len(),
            evidence_refs: accumulator.qr_entry_evidence.into_iter().collect(),
        },
        visitor_session_summary: LiveJourneyCountSummary {
            total: accumulator.visitor_session_evidence.len(),
            evidence_refs: accumulator.visitor_session_evidence.into_iter().collect(),
        },
        conversation_summary: LiveJourneyCountSummary {
            total: accumulator.conversation_evidence.len(),
            evidence_refs: accumulator.conversation_evidence.into_iter().collect(),
        },
        conversion_summary: LiveJourneyConversionSummary {
            qr_cases: accumulator.conversion.qr_cases,
            offer_acceptances: accumulator.conversion.offer_acceptances,
            trials_started: accumulator.conversion.trials_started,
            outcomes: accumulator.conversion.outcomes,
            attributions: accumulator.conversion.attributions,
        },
        review_summary: accumulator.review,
        referral_summary: accumulator.referral,
        handoff_moderation_summary: accumulator.handoff,
        privacy_summary: LiveJourneyPrivacySummary {
            packet_count: accumulator.privacy.packet_count,
            redaction_applied_count: accumulator.privacy.redaction_applied_count,
            redacted_value_count: accumulator.privacy.redacted_value_count,
            private_term_count: accumulator.privacy.private_term_count,
            detectors: accumulator.privacy.detectors.into_iter().collect(),
            report_redacted: true,
        },
        accounting_summary: accumulator.accounting,
        artifact_finding_summary,
        persuasion_boundary_summary: accumulator.persuasion,
        unexercised_gaps,
        follow_up_issue_drafts,
        issue_filing: "not_performed; local redacted drafts only".to_string(),
    };
    validate_report_value(&report, &request.private_terms)?;
    Ok(report)
}

fn read_journey_manifest(path: &Path, accumulator: &mut ReportAccumulator) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("read journey manifest {}", path.display()))?;
    let value = serde_json::from_str::<Value>(&content)
        .with_context(|| format!("parse journey manifest {}", path.display()))?;
    let schema_version = value
        .get("schemaVersion")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("journey manifest {} missing schemaVersion", path.display()))?;
    accumulator
        .input_artifacts
        .push(LiveJourneyInputArtifactRef {
            path: path.to_string_lossy().to_string(),
            artifact_kind: "journey_manifest".to_string(),
            schema_version: schema_version.to_string(),
            content_hash: stable_text_hash(&content),
        });
    match schema_version {
        QR_TO_TRIAL_JOURNEY_SCHEMA_VERSION => record_qr_manifest(path, &value, accumulator),
        REVIEW_RETURN_JOURNEY_SCHEMA_VERSION => {
            record_review_return_manifest(path, &value, accumulator)
        }
        AFFILIATE_REFERRAL_JOURNEY_SCHEMA_VERSION => {
            record_affiliate_manifest(path, &value, accumulator)
        }
        ADMIN_STAFF_JOURNEY_SCHEMA_VERSION => {
            record_admin_staff_manifest(path, &value, accumulator)
        }
        other => Err(anyhow!(
            "unsupported live journey manifest schema {other} at {}",
            path.display()
        )),
    }
}

fn record_qr_manifest(
    path: &Path,
    value: &Value,
    accumulator: &mut ReportAccumulator,
) -> Result<()> {
    let evidence = evidence(value)?;
    let persona_id = string_field(evidence, "personaId")?;
    let case_id = string_field(evidence, "caseId")?;
    record_case(path, value, accumulator, "qr_to_trial", persona_id, case_id)?;
    record_persona(accumulator, persona_id, case_id, None);
    insert_id(
        &mut accumulator.qr_entry_evidence,
        "entry_point",
        evidence,
        "entryPointId",
    );
    insert_id(
        &mut accumulator.visitor_session_evidence,
        "visitor_session",
        evidence,
        "visitorSessionId",
    );
    insert_id(
        &mut accumulator.conversation_evidence,
        "conversation",
        evidence,
        "conversationId",
    );
    accumulator.conversion.qr_cases += 1;
    accumulator.conversion.offer_acceptances += present(evidence, "acceptanceId");
    accumulator.conversion.trials_started += present(evidence, "trialId");
    accumulator.conversion.outcomes += evidence
        .get("outcomeIds")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    accumulator.conversion.attributions += usize_field(evidence, "attributionCount")?;
    accumulator.persuasion.evidence_backed_offer_language_cases += 1;
    accumulator.persuasion.no_fake_urgency_cases += 1;
    accumulator.persuasion.no_fake_scarcity_cases += 1;
    accumulator.persuasion.no_fake_review_or_metric_cases += 1;
    accumulator.persuasion.agency_preserving_cases += 1;
    record_packet(path, value, accumulator)?;
    Ok(())
}

fn record_review_return_manifest(
    path: &Path,
    value: &Value,
    accumulator: &mut ReportAccumulator,
) -> Result<()> {
    let evidence = evidence(value)?;
    let persona_id = string_field(evidence, "personaId")?;
    let case_id = string_field(evidence, "caseId")?;
    record_case(
        path,
        value,
        accumulator,
        "review_return",
        persona_id,
        case_id,
    )?;
    record_persona(accumulator, persona_id, case_id, None);
    insert_id(
        &mut accumulator.visitor_session_evidence,
        "visitor_session",
        evidence,
        "returnVisitorSessionId",
    );
    insert_id(
        &mut accumulator.conversation_evidence,
        "conversation",
        evidence,
        "conversationId",
    );
    accumulator.review.review_return_cases += 1;
    accumulator.review.simulated_email_artifacts += present(evidence, "simulatedEmailArtifactId");
    accumulator.review.feedback_items += present(evidence, "feedbackId");
    accumulator.review.review_candidates += present(evidence, "reviewId");
    if bool_field(evidence, "blockedPublishWithoutConsentOrApproval") {
        accumulator.review.blocked_before_consent_or_approval += 1;
    }
    accumulator.review.published_reviews += usize_field(evidence, "publicReviewCountAfterPublish")?;
    if usize_field(evidence, "publicReviewCountAfterRetire")? == 0
        && string_field(evidence, "finalReviewStatus")? == "retired"
    {
        accumulator.review.retired_reviews += 1;
    }
    accumulator.persuasion.no_fake_review_or_metric_cases += 1;
    record_packet(path, value, accumulator)?;
    Ok(())
}

fn record_affiliate_manifest(
    path: &Path,
    value: &Value,
    accumulator: &mut ReportAccumulator,
) -> Result<()> {
    let evidence = evidence(value)?;
    let persona_id = string_field(evidence, "personaId")?;
    let case_id = string_field(evidence, "caseId")?;
    record_case(
        path,
        value,
        accumulator,
        "affiliate_referral",
        persona_id,
        case_id,
    )?;
    record_persona(accumulator, persona_id, case_id, None);
    insert_id(
        &mut accumulator.qr_entry_evidence,
        "entry_point",
        evidence,
        "referralEntryPointId",
    );
    insert_id(
        &mut accumulator.visitor_session_evidence,
        "visitor_session",
        evidence,
        "referredVisitorSessionId",
    );
    insert_id(
        &mut accumulator.conversation_evidence,
        "conversation",
        evidence,
        "conversationId",
    );
    accumulator.conversion.offer_acceptances += present(evidence, "acceptanceId");
    accumulator.conversion.trials_started += present(evidence, "trialId");
    accumulator.referral.affiliate_referral_cases += 1;
    accumulator.referral.affiliate_connections += present(evidence, "affiliateConnectionId");
    accumulator.referral.referral_entry_points += present(evidence, "referralEntryPointId");
    accumulator.referral.referral_records += present(evidence, "referralId");
    accumulator.referral.referral_outcomes += present(evidence, "referralOutcomeId");
    accumulator.referral.attribution_count += usize_field(evidence, "attributionCount")?;
    if bool_field(evidence, "affiliateDeniedUnrelatedConversationRead") {
        accumulator.referral.unrelated_access_denials += 1;
    }
    accumulator.persuasion.no_fake_urgency_cases += 1;
    accumulator.persuasion.no_fake_scarcity_cases += 1;
    accumulator.persuasion.no_fake_review_or_metric_cases += 1;
    record_packet(path, value, accumulator)?;
    Ok(())
}

fn record_admin_staff_manifest(
    path: &Path,
    value: &Value,
    accumulator: &mut ReportAccumulator,
) -> Result<()> {
    let evidence = evidence(value)?;
    let persona_id = string_field(evidence, "personaId")?;
    let case_id = string_field(evidence, "caseId")?;
    record_case(path, value, accumulator, "admin_staff", persona_id, case_id)?;
    record_persona(accumulator, persona_id, case_id, None);
    insert_id(
        &mut accumulator.conversation_evidence,
        "conversation",
        evidence,
        "conversationId",
    );
    accumulator.handoff.admin_staff_cases += 1;
    accumulator.handoff.handoffs += present(evidence, "handoffId");
    if string_field(evidence, "finalHandoffStatus")? == "closed" {
        accumulator.handoff.closed_handoffs += 1;
    }
    if bool_field(evidence, "humanLedBlockedPublicAgentPost") {
        accumulator.handoff.human_led_blocks += 1;
    }
    if bool_field(evidence, "delegatedAllowsPublicAgentPost") {
        accumulator.handoff.delegation_allows += 1;
    }
    if bool_field(evidence, "returnedModeAllowsPublicAgentPost") {
        accumulator.handoff.returned_mode_allows += 1;
    }
    accumulator.handoff.review_moderation_publications +=
        usize_field(evidence, "reviewPublicCountAfterPublish")?;
    if bool_field(evidence, "affiliateDeniedAfterRevoke") {
        accumulator.handoff.affiliate_grants_revoked_and_denied += 1;
    }
    record_packet(path, value, accumulator)?;
    Ok(())
}

fn record_case(
    path: &Path,
    value: &Value,
    accumulator: &mut ReportAccumulator,
    journey_kind: &str,
    persona_id: &str,
    case_id: &str,
) -> Result<()> {
    accumulator.case_summaries.push(LiveJourneyCaseSummary {
        case_id: case_id.to_string(),
        journey_kind: journey_kind.to_string(),
        persona_id: persona_id.to_string(),
        status: value
            .pointer("/guard/status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        provider_mode: string_field(value, "providerMode")?.to_string(),
        network_enabled: value
            .get("networkEnabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        packet_path: value
            .get("packetPath")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        scorecard_path: value
            .get("scorecardPath")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        journey_manifest_path: path.to_string_lossy().to_string(),
    });
    Ok(())
}

fn record_persona(
    accumulator: &mut ReportAccumulator,
    persona_id: &str,
    case_id: &str,
    content_hash: Option<String>,
) {
    let entry = accumulator
        .persona_map
        .entry(persona_id.to_string())
        .or_insert(LiveJourneyPersonaSummary {
            persona_id: persona_id.to_string(),
            content_hash,
            case_ids: Vec::new(),
        });
    entry.case_ids.push(case_id.to_string());
}

fn record_packet(
    journey_manifest_path: &Path,
    value: &Value,
    accumulator: &mut ReportAccumulator,
) -> Result<()> {
    let Some(packet_path) = value.get("packetPath").and_then(Value::as_str) else {
        return Ok(());
    };
    let artifact_boundary = journey_manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let packet_path = resolve_existing_artifact_path(artifact_boundary, packet_path, "packetPath")?;
    let packet_json = fs::read_to_string(&packet_path).context("read packet artifact")?;
    let packet = serde_json::from_str::<EvalArtifactPacket>(&packet_json)
        .context("parse packet artifact")?;
    accumulator
        .input_artifacts
        .push(LiveJourneyInputArtifactRef {
            path: packet_path.to_string_lossy().to_string(),
            artifact_kind: "packet".to_string(),
            schema_version: packet.schema_version.clone(),
            content_hash: stable_text_hash(&packet_json),
        });
    accumulator.privacy.packet_count += 1;
    if packet.redaction_summary.redaction_applied {
        accumulator.privacy.redaction_applied_count += 1;
    }
    accumulator.privacy.redacted_value_count += packet.redaction_summary.redacted_value_count;
    accumulator.privacy.private_term_count += packet.redaction_summary.private_term_count;
    accumulator
        .privacy
        .detectors
        .extend(packet.redaction_summary.detectors.iter().cloned());

    if !packet.prompt_slot_ledger.is_empty() {
        accumulator.accounting.packets_with_prompt_slots += 1;
    }
    accumulator.accounting.prompt_slot_entries += packet.prompt_slot_ledger.len();
    if !packet.privacy_transform_ledger.is_empty() {
        accumulator.accounting.packets_with_privacy_transforms += 1;
    }
    accumulator.accounting.privacy_transform_entries += packet.privacy_transform_ledger.len();
    if !packet.token_ledger.is_empty() {
        accumulator.accounting.packets_with_token_ledger += 1;
    }
    accumulator.accounting.token_ledger_entries += packet.token_ledger.len();
    Ok(())
}

fn read_artifact_review(path: &Path, accumulator: &mut ReportAccumulator) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("read artifact review {}", path.display()))?;
    let review = serde_json::from_str::<EvalArtifactReview>(&content)
        .with_context(|| format!("parse artifact review {}", path.display()))?;
    ensure!(
        review.schema_version == EVAL_ARTIFACT_REVIEW_SCHEMA_VERSION,
        "unsupported artifact review schema {} at {}",
        review.schema_version,
        path.display()
    );
    accumulator
        .input_artifacts
        .push(LiveJourneyInputArtifactRef {
            path: path.to_string_lossy().to_string(),
            artifact_kind: "artifact_review".to_string(),
            schema_version: review.schema_version.clone(),
            content_hash: stable_text_hash(&content),
        });
    accumulator.artifact_reviews.push(review);
    Ok(())
}

fn artifact_finding_summary(reviews: &[EvalArtifactReview]) -> LiveJourneyArtifactFindingSummary {
    let mut by_category = BTreeMap::new();
    let mut by_severity = BTreeMap::new();
    let mut highest_severity = None;
    let mut finding_count = 0;
    for review in reviews {
        finding_count += review.findings.len();
        for finding in &review.findings {
            *by_category
                .entry(category_slug(finding.category.clone()).to_string())
                .or_insert(0) += 1;
            *by_severity
                .entry(severity_slug(&finding.severity).to_string())
                .or_insert(0) += 1;
            highest_severity = highest_severity.max(Some(finding.severity.clone()));
        }
    }
    LiveJourneyArtifactFindingSummary {
        review_artifact_count: reviews.len(),
        finding_count,
        highest_severity,
        by_category,
        by_severity,
    }
}

fn unexercised_gaps(case_summaries: &[LiveJourneyCaseSummary]) -> Vec<LiveJourneyGap> {
    let completed = case_summaries
        .iter()
        .map(|case| case.journey_kind.as_str())
        .collect::<BTreeSet<_>>();
    [
        ("qr_to_trial", "conversion"),
        ("review_return", "review"),
        ("affiliate_referral", "referral"),
        ("admin_staff", "handoff"),
    ]
    .into_iter()
    .filter(|(kind, _)| !completed.contains(kind))
    .map(|(kind, category)| LiveJourneyGap {
        gap_id: format!("missing_{kind}"),
        category: category.to_string(),
        severity: "warning".to_string(),
        summary: format!("No {kind} journey manifest was included in this report."),
        evidence_refs: vec![format!("report.caseSummaries.missing.{kind}")],
    })
    .collect()
}

fn follow_up_issue_drafts(
    gaps: &[LiveJourneyGap],
    reviews: &[EvalArtifactReview],
) -> Vec<LiveJourneyIssueDraft> {
    let mut drafts = Vec::new();
    for gap in gaps {
        drafts.push(LiveJourneyIssueDraft {
            title: format!("Investigate live journey report gap: {}", gap.gap_id),
            body: format!(
                "## Goal\nInvestigate the `{}` live journey report gap.\n\n## Evidence\n- {}\n\n## Scope\nConfirm whether the missing artifact is expected for this run or points to a report/input issue.\n\n## Acceptance Criteria\n- Gap is accepted, rejected, or superseded with evidence.\n\n## Validation\n- Regenerate the live journey report with the intended artifacts.\n\n## Non-Goals\n- Do not file this automatically; this is a local redacted draft.\n\n## Closeout Evidence\n- Link the PR or report note that resolves the gap.\n",
                gap.gap_id,
                gap.evidence_refs.join("\n- ")
            ),
            evidence_refs: gap.evidence_refs.clone(),
            filing_status: "local_draft_only".to_string(),
        });
    }
    for review in reviews {
        for finding in review.findings.iter().filter(|finding| {
            matches!(
                finding.severity,
                EvalArtifactFindingSeverity::Failure | EvalArtifactFindingSeverity::Blocker
            )
        }) {
            if let (Some(title), Some(body)) = (
                finding.suggested_issue_title.clone(),
                finding.suggested_issue_body.clone(),
            ) {
                drafts.push(LiveJourneyIssueDraft {
                    title,
                    body,
                    evidence_refs: finding.evidence_refs.clone(),
                    filing_status: "local_draft_only".to_string(),
                });
            }
        }
    }
    drafts.sort_by(|left, right| left.title.cmp(&right.title));
    drafts
}

pub fn render_live_journey_report_markdown(report: &LiveJourneyReport) -> String {
    let mut output = String::new();
    output.push_str("# Live Product Journey Report\n\n");
    output.push_str(&format!("- Schema: `{}`\n", report.schema_version));
    output.push_str(&format!("- Source commit: `{}`\n", report.source_commit));
    output.push_str(&format!("- Generated at: `{}`\n", report.generated_at));
    output.push_str(&format!("- Cases: `{}`\n", report.case_summaries.len()));
    output.push_str(&format!("- Personas: `{}`\n", report.persona_roster.len()));
    output.push_str(&format!("- Issue filing: `{}`\n\n", report.issue_filing));

    output.push_str("## Outcome Summary\n\n");
    output.push_str(&format!(
        "- Trials started: `{}`\n",
        report.conversion_summary.trials_started
    ));
    output.push_str(&format!(
        "- Offer acceptances: `{}`\n",
        report.conversion_summary.offer_acceptances
    ));
    output.push_str(&format!(
        "- Review return cases: `{}`\n",
        report.review_summary.review_return_cases
    ));
    output.push_str(&format!(
        "- Referral records: `{}`\n",
        report.referral_summary.referral_records
    ));
    output.push_str(&format!(
        "- Closed handoffs: `{}`\n\n",
        report.handoff_moderation_summary.closed_handoffs
    ));

    output.push_str("## Evidence Summary\n\n");
    output.push_str(&format!(
        "- QR/entry points: `{}`\n",
        report.qr_entry_summary.total
    ));
    output.push_str(&format!(
        "- Visitor sessions: `{}`\n",
        report.visitor_session_summary.total
    ));
    output.push_str(&format!(
        "- Conversations: `{}`\n",
        report.conversation_summary.total
    ));
    output.push_str(&format!(
        "- Prompt slots: `{}`\n",
        report.accounting_summary.prompt_slot_entries
    ));
    output.push_str(&format!(
        "- Token ledger entries: `{}`\n",
        report.accounting_summary.token_ledger_entries
    ));
    output.push_str(&format!(
        "- Redacted values: `{}`\n\n",
        report.privacy_summary.redacted_value_count
    ));

    output.push_str("## Artifact Review\n\n");
    output.push_str(&format!(
        "- Review artifacts: `{}`\n",
        report.artifact_finding_summary.review_artifact_count
    ));
    output.push_str(&format!(
        "- Findings: `{}`\n",
        report.artifact_finding_summary.finding_count
    ));
    if report.artifact_finding_summary.by_category.is_empty() {
        output.push_str("- Findings by category: none\n\n");
    } else {
        for (category, count) in &report.artifact_finding_summary.by_category {
            output.push_str(&format!("- `{category}`: `{count}`\n"));
        }
        output.push('\n');
    }

    output.push_str("## Cases\n\n");
    for case in &report.case_summaries {
        output.push_str(&format!(
            "- `{}`: `{}` for `{}` status `{}`\n",
            case.case_id, case.journey_kind, case.persona_id, case.status
        ));
    }
    output.push('\n');

    output.push_str("## Gaps And Drafts\n\n");
    if report.unexercised_gaps.is_empty() {
        output.push_str("- Unexercised gaps: none\n");
    } else {
        for gap in &report.unexercised_gaps {
            output.push_str(&format!("- `{}`: {}\n", gap.gap_id, gap.summary));
        }
    }
    output.push_str(&format!(
        "- Local issue drafts: `{}`\n",
        report.follow_up_issue_drafts.len()
    ));
    output
}

fn evidence(value: &Value) -> Result<&Value> {
    value
        .get("evidence")
        .ok_or_else(|| anyhow!("journey manifest missing evidence"))
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing string field {field}"))
}

fn usize_field(value: &Value, field: &str) -> Result<usize> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .map(|number| number as usize)
        .ok_or_else(|| anyhow!("missing numeric field {field}"))
}

fn bool_field(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_bool).unwrap_or(false)
}

fn present(value: &Value, field: &str) -> usize {
    value
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty()) as usize
}

fn insert_id(target: &mut BTreeSet<String>, prefix: &str, value: &Value, field: &str) {
    if let Some(id) = value.get(field).and_then(Value::as_str) {
        if !id.trim().is_empty() {
            target.insert(format!("{prefix}:{id}"));
        }
    }
}

fn validate_report_outputs(
    json_path: &Path,
    markdown_path: &Path,
    private_terms: &[String],
) -> Result<()> {
    for path in [json_path, markdown_path] {
        let content = fs::read_to_string(path)
            .with_context(|| format!("read report output {}", path.display()))?;
        ensure!(
            !contains_sensitive_text(&content, private_terms),
            "live journey report output contains raw sensitive value: {}",
            path.display()
        );
    }
    Ok(())
}

fn validate_report_value(report: &LiveJourneyReport, private_terms: &[String]) -> Result<()> {
    let encoded = serde_json::to_string(report)?;
    ensure!(
        !contains_sensitive_text(&encoded, private_terms),
        "live journey report contains raw sensitive value"
    );
    Ok(())
}

fn contains_sensitive_text(content: &str, private_terms: &[String]) -> bool {
    redaction::contains_sensitive_text(content, private_terms)
}

fn category_slug(category: EvalFindingCategory) -> &'static str {
    match category {
        EvalFindingCategory::SchemaGap => "schema_gap",
        EvalFindingCategory::EventGap => "event_gap",
        EvalFindingCategory::PolicyGap => "policy_gap",
        EvalFindingCategory::PrivacyGap => "privacy_gap",
        EvalFindingCategory::PromptGap => "prompt_gap",
        EvalFindingCategory::HandoffGap => "handoff_gap",
        EvalFindingCategory::AnalysisGap => "analysis_gap",
        EvalFindingCategory::AccountingGap => "accounting_gap",
        EvalFindingCategory::UxContractGap => "ux_contract_gap",
        EvalFindingCategory::ProviderGap => "provider_gap",
        EvalFindingCategory::TestFixtureGap => "test_fixture_gap",
    }
}

fn severity_slug(severity: &EvalArtifactFindingSeverity) -> &'static str {
    match severity {
        EvalArtifactFindingSeverity::Info => "info",
        EvalArtifactFindingSeverity::Warning => "warning",
        EvalArtifactFindingSeverity::Failure => "failure",
        EvalArtifactFindingSeverity::Blocker => "blocker",
    }
}

fn stable_text_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let encoded = serde_json::to_string_pretty(value)?;
    fs::write(path, encoded).with_context(|| format!("write JSON artifact {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::eval_artifact_review::write_review_artifacts;
    use crate::live_eval_runner::{
        run_admin_staff_journey_eval, run_affiliate_referral_journey_eval,
        run_qr_to_trial_journey_eval, run_review_return_journey_eval,
    };
    use crate::schema::init_schema;
    use rusqlite::Connection;
    use serde_json::json;

    fn file_backed_eval_store() -> (tempfile::NamedTempFile, Connection) {
        let file = tempfile::NamedTempFile::new().unwrap();
        let connection = Connection::open(file.path()).unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        (file, connection)
    }

    fn personas_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("docs/evals/personas")
    }

    fn private_terms() -> Vec<String> {
        vec![
            "Project Orchid".to_string(),
            "sk-live-journey-fixture".to_string(),
            "admin-staff-secret".to_string(),
            "affiliate-referral-secret".to_string(),
            "review-return-secret".to_string(),
            "alex@example.com".to_string(),
        ]
    }

    #[test]
    fn report_generator_aggregates_all_live_journey_artifacts() {
        let temp_dir = tempfile::tempdir().unwrap();
        let qr_dir = temp_dir.path().join("qr");
        let review_dir = temp_dir.path().join("review");
        let affiliate_dir = temp_dir.path().join("affiliate");
        let admin_dir = temp_dir.path().join("admin");

        let (qr_db_file, qr_connection) = file_backed_eval_store();
        let qr = run_qr_to_trial_journey_eval(
            qr_db_file.path(),
            &qr_connection,
            &personas_dir(),
            Some("solo_consultant_followup"),
            &qr_dir,
            "test-commit",
            private_terms(),
        )
        .unwrap();
        let (review_db_file, review_connection) = file_backed_eval_store();
        let review = run_review_return_journey_eval(
            review_db_file.path(),
            &review_connection,
            &personas_dir(),
            Some("solo_consultant_followup"),
            &review_dir,
            "test-commit",
            private_terms(),
        )
        .unwrap();
        let (affiliate_db_file, affiliate_connection) = file_backed_eval_store();
        let affiliate = run_affiliate_referral_journey_eval(
            affiliate_db_file.path(),
            &affiliate_connection,
            &personas_dir(),
            Some("affiliate_referrer_community"),
            &affiliate_dir,
            "test-commit",
            private_terms(),
        )
        .unwrap();
        let (admin_db_file, admin_connection) = file_backed_eval_store();
        let admin = run_admin_staff_journey_eval(
            admin_db_file.path(),
            &admin_connection,
            &personas_dir(),
            Some("dissatisfied_trial_user"),
            &admin_dir,
            "test-commit",
            private_terms(),
        )
        .unwrap();
        let review_artifacts_dir = temp_dir.path().join("artifact_reviews");
        let qr_review = write_review_artifacts(&qr.packet_path, &review_artifacts_dir).unwrap();
        let admin_review =
            write_review_artifacts(&admin.packet_path, &review_artifacts_dir).unwrap();

        let report_dir = temp_dir.path().join("report");
        let paths = generate_live_journey_report(LiveJourneyReportRequest {
            journey_manifest_paths: vec![
                PathBuf::from(&qr.journey_manifest_path),
                PathBuf::from(&review.journey_manifest_path),
                PathBuf::from(&affiliate.journey_manifest_path),
                PathBuf::from(&admin.journey_manifest_path),
            ],
            artifact_review_paths: vec![
                qr_review.review_json_path.clone(),
                admin_review.review_json_path.clone(),
            ],
            output_dir: report_dir,
            source_commit: "test-commit".to_string(),
            generated_at: "2026-05-09T00:00:00Z".to_string(),
            private_terms: private_terms(),
        })
        .unwrap();

        assert!(paths.report_json_path.exists());
        assert!(paths.report_markdown_path.exists());
        let report_json = fs::read_to_string(&paths.report_json_path).unwrap();
        let report = serde_json::from_str::<LiveJourneyReport>(&report_json).unwrap();
        assert_eq!(report.schema_version, LIVE_JOURNEY_REPORT_SCHEMA_VERSION);
        assert_eq!(report.case_summaries.len(), 4);
        assert_eq!(report.conversion_summary.qr_cases, 1);
        assert!(report.conversion_summary.trials_started >= 2);
        assert_eq!(report.review_summary.review_return_cases, 1);
        assert_eq!(report.review_summary.simulated_email_artifacts, 1);
        assert_eq!(report.review_summary.blocked_before_consent_or_approval, 1);
        assert_eq!(report.referral_summary.affiliate_referral_cases, 1);
        assert_eq!(report.referral_summary.referral_records, 1);
        assert_eq!(report.handoff_moderation_summary.admin_staff_cases, 1);
        assert_eq!(report.handoff_moderation_summary.closed_handoffs, 1);
        assert_eq!(report.handoff_moderation_summary.human_led_blocks, 1);
        assert!(report.accounting_summary.prompt_slot_entries >= 6);
        assert!(report.accounting_summary.token_ledger_entries >= 6);
        assert_eq!(report.artifact_finding_summary.review_artifact_count, 2);
        assert!(report.unexercised_gaps.is_empty());
        assert!(report.follow_up_issue_drafts.is_empty());

        let report_markdown = fs::read_to_string(&paths.report_markdown_path).unwrap();
        assert!(report_markdown.contains("Live Product Journey Report"));
        assert!(report_markdown.contains("Trials started"));
        assert!(report_markdown.contains("Closed handoffs"));
        for forbidden in private_terms() {
            assert!(!report_json.contains(&forbidden));
            assert!(!report_markdown.contains(&forbidden));
        }
    }

    #[test]
    fn report_generator_records_missing_journey_gaps_as_local_drafts() {
        let (db_file, connection) = file_backed_eval_store();
        let temp_dir = tempfile::tempdir().unwrap();
        let qr = run_qr_to_trial_journey_eval(
            db_file.path(),
            &connection,
            &personas_dir(),
            Some("solo_consultant_followup"),
            temp_dir.path().join("qr"),
            "test-commit",
            private_terms(),
        )
        .unwrap();
        let report = build_live_journey_report(&LiveJourneyReportRequest {
            journey_manifest_paths: vec![PathBuf::from(&qr.journey_manifest_path)],
            artifact_review_paths: vec![],
            output_dir: temp_dir.path().join("report"),
            source_commit: "test-commit".to_string(),
            generated_at: "2026-05-09T00:00:00Z".to_string(),
            private_terms: private_terms(),
        })
        .unwrap();

        assert_eq!(report.case_summaries.len(), 1);
        assert_eq!(report.unexercised_gaps.len(), 3);
        assert!(report
            .unexercised_gaps
            .iter()
            .any(|gap| gap.gap_id == "missing_review_return"));
        assert_eq!(report.follow_up_issue_drafts.len(), 3);
        assert!(report
            .follow_up_issue_drafts
            .iter()
            .all(|draft| draft.filing_status == "local_draft_only"));
        let encoded = serde_json::to_string(&report).unwrap();
        for forbidden in private_terms() {
            assert!(!encoded.contains(&forbidden));
        }
    }

    #[test]
    fn report_generator_rejects_manifest_packet_path_escape() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manifest_dir = temp_dir.path().join("journey");
        fs::create_dir(&manifest_dir).unwrap();
        let outside_dir = temp_dir.path().join("outside");
        fs::create_dir(&outside_dir).unwrap();
        let outside_packet = outside_dir.join("packet.json");
        fs::write(&outside_packet, "{}").unwrap();
        let journey_manifest = manifest_dir.join("qr-journey.json");
        fs::write(
            &journey_manifest,
            serde_json::to_string_pretty(&json!({
                "schemaVersion": QR_TO_TRIAL_JOURNEY_SCHEMA_VERSION,
                "guard": { "status": "completed" },
                "providerMode": "deterministic",
                "networkEnabled": false,
                "evidence": {
                    "personaId": "persona_1",
                    "caseId": "case_1",
                    "entryPointId": "entry_1",
                    "visitorSessionId": "session_1",
                    "conversationId": "conversation_1",
                    "acceptanceId": "acceptance_1",
                    "trialId": "trial_1",
                    "outcomeIds": [],
                    "attributionCount": 0
                },
                "packetPath": outside_packet.to_string_lossy()
            }))
            .unwrap(),
        )
        .unwrap();

        let error = generate_live_journey_report(LiveJourneyReportRequest {
            journey_manifest_paths: vec![journey_manifest],
            artifact_review_paths: Vec::new(),
            output_dir: temp_dir.path().join("report"),
            source_commit: "test".to_string(),
            generated_at: "2026-05-13T00:00:00Z".to_string(),
            private_terms: Vec::new(),
        })
        .unwrap_err();

        assert!(error.to_string().contains("escapes artifact boundary"));
    }
}
