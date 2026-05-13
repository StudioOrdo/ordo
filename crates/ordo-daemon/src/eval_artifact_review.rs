use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::eval_harness::{
    EvalArtifactPacket, EvalEvidenceChannel, EvalFindingCategory, EvalRedactionSummary,
};
use crate::security::{
    artifact_boundary::resolve_artifact_output_path, markdown::sanitize_markdown_links, redaction,
};

pub const EVAL_ARTIFACT_REVIEW_SCHEMA_VERSION: &str = "ordo.eval_artifact_review.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvalArtifactFindingSeverity {
    Info,
    Warning,
    Failure,
    Blocker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalArtifactFindingStatus {
    Candidate,
    Accepted,
    Rejected,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalArtifactFinding {
    pub id: String,
    pub category: EvalFindingCategory,
    pub severity: EvalArtifactFindingSeverity,
    pub status: EvalArtifactFindingStatus,
    pub source_artifact_hash: String,
    pub case_id: String,
    pub evidence_refs: Vec<String>,
    pub summary: String,
    pub suggested_owner_subsystem: String,
    pub suggested_issue_title: Option<String>,
    pub suggested_issue_body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalArtifactReview {
    pub schema_version: String,
    pub status: String,
    pub source_artifact_path: Option<String>,
    pub source_artifact_hash: String,
    pub case_id: String,
    pub finding_count: usize,
    pub highest_severity: Option<EvalArtifactFindingSeverity>,
    pub findings: Vec<EvalArtifactFinding>,
    pub redaction_summary: EvalRedactionSummary,
    pub issue_filing: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalArtifactReviewPaths {
    pub review_json_path: PathBuf,
    pub review_markdown_path: PathBuf,
}

pub fn review_packet(packet: &EvalArtifactPacket) -> Result<EvalArtifactReview> {
    review_packet_with_source(packet, None)
}

pub fn review_packet_file(packet_path: impl AsRef<Path>) -> Result<EvalArtifactReview> {
    let packet_path = packet_path.as_ref();
    let packet_json = fs::read_to_string(packet_path)
        .with_context(|| format!("read eval packet {}", packet_path.display()))?;
    let packet = serde_json::from_str::<EvalArtifactPacket>(&packet_json)
        .with_context(|| format!("parse eval packet {}", packet_path.display()))?;
    review_packet_with_source(&packet, Some(packet_path.to_string_lossy().to_string()))
}

pub fn write_review_artifacts(
    packet_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> Result<EvalArtifactReviewPaths> {
    let packet_path = packet_path.as_ref();
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)
        .with_context(|| format!("create eval artifact review dir {}", output_dir.display()))?;
    let review = review_packet_file(packet_path)?;
    let review_json_path = resolve_artifact_output_path(
        output_dir,
        format!("{}-artifact-review.json", review.case_id),
        "artifact review JSON",
    )?;
    let review_markdown_path =
        resolve_artifact_output_path(output_dir, "artifact-review.md", "artifact review markdown")?;
    write_json(&review_json_path, &review)?;
    fs::write(
        &review_markdown_path,
        sanitize_markdown_links(&review_markdown(&review)),
    )
    .with_context(|| format!("write artifact review {}", review_markdown_path.display()))?;
    Ok(EvalArtifactReviewPaths {
        review_json_path,
        review_markdown_path,
    })
}

fn review_packet_with_source(
    packet: &EvalArtifactPacket,
    source_path: Option<String>,
) -> Result<EvalArtifactReview> {
    let source_artifact_hash = stable_json_hash(packet)?;
    let mut findings = Vec::new();

    classify_raw_sensitive_values(packet, &source_artifact_hash, &mut findings)?;
    classify_failed_assertions(packet, &source_artifact_hash, &mut findings);
    classify_missing_expected_ledgers(packet, &source_artifact_hash, &mut findings);
    classify_provider_failures(packet, &source_artifact_hash, &mut findings);
    classify_redaction_markers(packet, &source_artifact_hash, &mut findings);

    findings.sort_by(|left, right| {
        finding_sort_key(left)
            .cmp(&finding_sort_key(right))
            .then_with(|| left.summary.cmp(&right.summary))
    });
    for (index, finding) in findings.iter_mut().enumerate() {
        finding.id = format!(
            "finding_{}_{}",
            packet.case_id,
            stable_text_hash(&format!(
                "{}:{}:{}:{}",
                index,
                category_slug(finding.category.clone()),
                severity_slug(&finding.severity),
                finding.summary
            ))
            .chars()
            .take(12)
            .collect::<String>()
        );
    }

    let highest_severity = findings
        .iter()
        .map(|finding| finding.severity.clone())
        .max();
    let mut redaction_summary = packet.redaction_summary.clone();
    let mut review = EvalArtifactReview {
        schema_version: EVAL_ARTIFACT_REVIEW_SCHEMA_VERSION.to_string(),
        status: "reviewed".to_string(),
        source_artifact_path: source_path,
        source_artifact_hash,
        case_id: packet.case_id.clone(),
        finding_count: findings.len(),
        highest_severity,
        findings,
        redaction_summary: redaction_summary.clone(),
        issue_filing: "not_performed; review output is a local redacted draft only".to_string(),
    };
    redact_review(&mut review, &mut redaction_summary)?;
    review.redaction_summary = redaction_summary;
    Ok(review)
}

fn classify_failed_assertions(
    packet: &EvalArtifactPacket,
    source_artifact_hash: &str,
    findings: &mut Vec<EvalArtifactFinding>,
) {
    for assertion in packet
        .scorecard
        .assertion_results
        .iter()
        .filter(|assertion| !assertion.passed)
    {
        let category = category_for_channel(assertion.channel);
        findings.push(finding(
            category.clone(),
            EvalArtifactFindingSeverity::Failure,
            source_artifact_hash,
            &packet.case_id,
            vec![format!(
                "scorecard.assertionResults.{}",
                assertion.assertion_id
            )],
            format!(
                "{} expected at least {} but found {}",
                assertion.assertion_id, assertion.expected_minimum, assertion.actual_count
            ),
        ));
    }
}

fn classify_missing_expected_ledgers(
    packet: &EvalArtifactPacket,
    source_artifact_hash: &str,
    findings: &mut Vec<EvalArtifactFinding>,
) {
    let mut expected_channels = Vec::new();
    for step in &packet.steps {
        for channel in &step.expected_evidence {
            expected_channels.push(*channel);
        }
    }
    expected_channels.sort_by_key(|channel| channel.as_str());
    expected_channels.dedup();
    for channel in expected_channels {
        if ledger_count_for_channel(packet, channel) == 0
            && packet.scorecard.evidence_after.count_for(channel) == 0
        {
            let category = category_for_channel(channel);
            findings.push(finding(
                category,
                EvalArtifactFindingSeverity::Warning,
                source_artifact_hash,
                &packet.case_id,
                vec![format!("steps.expectedEvidence.{}", channel.as_str())],
                format!(
                    "Expected evidence channel {} is empty in packet review.",
                    channel.as_str()
                ),
            ));
        }
    }
}

fn classify_redaction_markers(
    packet: &EvalArtifactPacket,
    source_artifact_hash: &str,
    findings: &mut Vec<EvalArtifactFinding>,
) {
    if packet.redaction_summary.redaction_applied
        || packet.redaction_summary.redacted_value_count > 0
    {
        findings.push(finding(
            EvalFindingCategory::PrivacyGap,
            EvalArtifactFindingSeverity::Info,
            source_artifact_hash,
            &packet.case_id,
            vec!["packet.redactionSummary".to_string()],
            format!(
                "Packet contains {} redacted value marker(s); this is containment evidence, not a failure.",
                packet.redaction_summary.redacted_value_count
            ),
        ));
    }
}

fn classify_provider_failures(
    packet: &EvalArtifactPacket,
    source_artifact_hash: &str,
    findings: &mut Vec<EvalArtifactFinding>,
) {
    for entry in packet
        .conversation_event_ledger
        .iter()
        .chain(packet.timeline.iter())
        .filter(|entry| entry.entry_type == "llm.run.failed")
    {
        let has_safe_code = entry.payload.get("code").and_then(Value::as_str).is_some()
            || entry
                .payload
                .get("failureCode")
                .and_then(Value::as_str)
                .is_some();
        let severity = if has_safe_code {
            EvalArtifactFindingSeverity::Warning
        } else {
            EvalArtifactFindingSeverity::Failure
        };
        findings.push(finding(
            EvalFindingCategory::ProviderGap,
            severity,
            source_artifact_hash,
            &packet.case_id,
            vec![format!("conversationEventLedger.{}", entry.id)],
            "Provider failure evidence requires review of safe code/message metadata.".to_string(),
        ));
    }
}

fn classify_raw_sensitive_values(
    packet: &EvalArtifactPacket,
    source_artifact_hash: &str,
    findings: &mut Vec<EvalArtifactFinding>,
) -> Result<()> {
    let encoded = serde_json::to_string(packet)?;
    let mut labels = Vec::new();
    for token in encoded.split_whitespace() {
        let trimmed = token.trim_matches(|character: char| {
            matches!(
                character,
                '"' | '\'' | ',' | '.' | ';' | ':' | '{' | '}' | '[' | ']' | '(' | ')'
            )
        });
        if looks_like_email(trimmed) {
            labels.push("raw email");
        } else if looks_like_phone(trimmed) {
            labels.push("raw phone");
        } else if looks_like_secret(trimmed) {
            labels.push("raw secret");
        }
    }
    if encoded.to_ascii_lowercase().contains("project orchid") {
        labels.push("raw private term");
    }
    labels.sort_unstable();
    labels.dedup();
    for label in labels {
        findings.push(finding(
            EvalFindingCategory::PrivacyGap,
            EvalArtifactFindingSeverity::Blocker,
            source_artifact_hash,
            &packet.case_id,
            vec!["packet.rawPrivacyScan".to_string()],
            format!("Artifact packet contains {label}; review output redacts the value."),
        ));
    }
    Ok(())
}

fn category_for_channel(channel: EvalEvidenceChannel) -> EvalFindingCategory {
    match channel {
        EvalEvidenceChannel::SqliteRows => EvalFindingCategory::SchemaGap,
        EvalEvidenceChannel::ConversationEvents | EvalEvidenceChannel::RealtimeReplay => {
            EvalFindingCategory::EventGap
        }
        EvalEvidenceChannel::PolicyDecisions => EvalFindingCategory::PolicyGap,
        EvalEvidenceChannel::PromptSlotAccounting => EvalFindingCategory::PromptGap,
        EvalEvidenceChannel::PrivacyTransforms => EvalFindingCategory::PrivacyGap,
        EvalEvidenceChannel::TokenLedger => EvalFindingCategory::AccountingGap,
        EvalEvidenceChannel::AnalysisCandidates => EvalFindingCategory::AnalysisGap,
        EvalEvidenceChannel::HandoffState => EvalFindingCategory::HandoffGap,
        EvalEvidenceChannel::ArtifactRecords
        | EvalEvidenceChannel::SurfaceBriefRecords
        | EvalEvidenceChannel::FeedbackReviewRecords
        | EvalEvidenceChannel::ProductSurfaceRecords => EvalFindingCategory::UxContractGap,
    }
}

fn ledger_count_for_channel(packet: &EvalArtifactPacket, channel: EvalEvidenceChannel) -> usize {
    match channel {
        EvalEvidenceChannel::SqliteRows => packet.timeline.len(),
        EvalEvidenceChannel::ConversationEvents => packet.conversation_event_ledger.len(),
        EvalEvidenceChannel::RealtimeReplay => packet.realtime_replay_ledger.len(),
        EvalEvidenceChannel::PolicyDecisions => packet.policy_decision_ledger.len(),
        EvalEvidenceChannel::PromptSlotAccounting => packet.prompt_slot_ledger.len(),
        EvalEvidenceChannel::PrivacyTransforms => packet.privacy_transform_ledger.len(),
        EvalEvidenceChannel::TokenLedger => packet.token_ledger.len(),
        EvalEvidenceChannel::AnalysisCandidates => packet.analysis_candidate_ledger.len(),
        EvalEvidenceChannel::HandoffState => packet.handoff_ledger.len(),
        EvalEvidenceChannel::ArtifactRecords => packet.artifact_ledger.len(),
        EvalEvidenceChannel::SurfaceBriefRecords => packet.surface_brief_ledger.len(),
        EvalEvidenceChannel::FeedbackReviewRecords => {
            packet.feedback_ledger.len() + packet.review_ledger.len()
        }
        EvalEvidenceChannel::ProductSurfaceRecords => packet.product_surface_ledger.len(),
    }
}

fn finding(
    category: EvalFindingCategory,
    severity: EvalArtifactFindingSeverity,
    source_artifact_hash: &str,
    case_id: &str,
    evidence_refs: Vec<String>,
    summary: String,
) -> EvalArtifactFinding {
    let owner = owner_for_category(&category).to_string();
    let issue_title = if matches!(
        severity,
        EvalArtifactFindingSeverity::Failure | EvalArtifactFindingSeverity::Blocker
    ) {
        Some(format!(
            "{}: {}",
            category_slug(category.clone()),
            summary.chars().take(72).collect::<String>()
        ))
    } else {
        None
    };
    let issue_body = issue_title.as_ref().map(|_| {
        format!(
            "## Goal\nInvestigate artifact review finding for `{case_id}`.\n\n## Evidence\n- Category: `{}`\n- Severity: `{}`\n- Evidence refs: {}\n\n## Scope\nConfirm the smallest responsible subsystem and implement a focused fix if the finding is accepted.\n\n## Acceptance Criteria\n- Finding is accepted, rejected, or superseded with evidence.\n- Any accepted fix includes deterministic validation.\n\n## Validation\n- Re-run the originating eval packet review.\n\n## Non-Goals\n- Do not file this automatically; this is a local redacted draft.\n\n## Closeout Evidence\n- Link PR or review note that resolves this finding.\n",
            category_slug(category.clone()),
            severity_slug(&severity),
            evidence_refs.join(", ")
        )
    });
    EvalArtifactFinding {
        id: String::new(),
        category,
        severity,
        status: EvalArtifactFindingStatus::Candidate,
        source_artifact_hash: source_artifact_hash.to_string(),
        case_id: case_id.to_string(),
        evidence_refs,
        summary,
        suggested_owner_subsystem: owner,
        suggested_issue_title: issue_title,
        suggested_issue_body: issue_body,
    }
}

fn review_markdown(review: &EvalArtifactReview) -> String {
    let mut output = String::new();
    output.push_str("# Artifact Review\n\n");
    output.push_str(&format!("- Schema: `{}`\n", review.schema_version));
    output.push_str(&format!("- Case: `{}`\n", review.case_id));
    output.push_str(&format!("- Status: `{}`\n", review.status));
    output.push_str(&format!("- Findings: `{}`\n", review.finding_count));
    output.push_str(&format!("- Issue filing: `{}`\n\n", review.issue_filing));
    if review.findings.is_empty() {
        output.push_str("No candidate findings.\n");
        return output;
    }
    for finding in &review.findings {
        output.push_str(&format!(
            "## {} `{}`\n\n",
            severity_slug(&finding.severity),
            finding.id
        ));
        output.push_str(&format!(
            "- Category: `{}`\n",
            category_slug(finding.category.clone())
        ));
        output.push_str(&format!(
            "- Owner: `{}`\n",
            finding.suggested_owner_subsystem
        ));
        output.push_str(&format!("- Summary: {}\n", finding.summary));
        output.push_str(&format!(
            "- Evidence: `{}`\n\n",
            finding.evidence_refs.join("`, `")
        ));
    }
    output
}

fn owner_for_category(category: &EvalFindingCategory) -> &'static str {
    match category {
        EvalFindingCategory::SchemaGap => "schema",
        EvalFindingCategory::EventGap => "conversation_protocol",
        EvalFindingCategory::PolicyGap => "policy",
        EvalFindingCategory::PrivacyGap => "privacy_egress",
        EvalFindingCategory::PromptGap => "llm_gateway_prompt_slots",
        EvalFindingCategory::HandoffGap => "conversation_handoff",
        EvalFindingCategory::AnalysisGap => "conversation_analysis",
        EvalFindingCategory::AccountingGap => "llm_accounting",
        EvalFindingCategory::UxContractGap => "product_contract",
        EvalFindingCategory::ProviderGap => "llm_provider_adapter",
        EvalFindingCategory::TestFixtureGap => "eval_fixture",
    }
}

fn finding_sort_key(finding: &EvalArtifactFinding) -> (u8, String, String) {
    (
        match finding.severity {
            EvalArtifactFindingSeverity::Blocker => 0,
            EvalArtifactFindingSeverity::Failure => 1,
            EvalArtifactFindingSeverity::Warning => 2,
            EvalArtifactFindingSeverity::Info => 3,
        },
        category_slug(finding.category.clone()).to_string(),
        finding.summary.clone(),
    )
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

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let encoded = serde_json::to_string_pretty(value)?;
    fs::write(path, encoded).with_context(|| format!("write JSON artifact {}", path.display()))?;
    Ok(())
}

fn redact_review(
    review: &mut EvalArtifactReview,
    summary: &mut EvalRedactionSummary,
) -> Result<()> {
    let mut value = serde_json::to_value(&*review)?;
    redact_value(&mut value, summary);
    *review = serde_json::from_value(value)?;
    summary.redaction_applied = summary.redacted_value_count > 0 || summary.redaction_applied;
    Ok(())
}

fn redact_value(value: &mut Value, summary: &mut EvalRedactionSummary) {
    match value {
        Value::String(text) => {
            *text = redact_text(text, summary);
        }
        Value::Array(items) => {
            for item in items {
                redact_value(item, summary);
            }
        }
        Value::Object(map) => {
            for item in map.values_mut() {
                redact_value(item, summary);
            }
        }
        _ => {}
    }
}

fn redact_text(text: &str, summary: &mut EvalRedactionSummary) -> String {
    let redacted = redaction::redact_artifact_review_text(text);
    summary.redacted_value_count += redacted.redacted_count;
    redacted.text
}

fn looks_like_email(value: &str) -> bool {
    redaction::looks_like_email(value)
}

fn looks_like_phone(value: &str) -> bool {
    redaction::looks_like_phone(value)
}

fn looks_like_secret(value: &str) -> bool {
    redaction::looks_like_secret(value)
}

fn stable_json_hash<T: Serialize>(value: &T) -> Result<String> {
    let encoded = serde_json::to_string(value)?;
    Ok(stable_text_hash(&encoded))
}

fn stable_text_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval_harness::{
        EvalActorRole, EvalArtifactReviewPlaceholder, EvalAssertionResult, EvalEvidenceCount,
        EvalEvidenceSnapshot, EvalLedgerEntry, EvalRedactionSummary, EvalScorecardSummary,
        EvalStep, EVAL_ARTIFACT_PACKET_SCHEMA_VERSION,
    };
    use serde_json::json;

    fn base_packet() -> EvalArtifactPacket {
        EvalArtifactPacket {
            schema_version: EVAL_ARTIFACT_PACKET_SCHEMA_VERSION.to_string(),
            case_id: "review_case".to_string(),
            case_title: "Review case".to_string(),
            fixture_hash: "fixture_hash".to_string(),
            actor_roles: vec![EvalActorRole::Staff],
            steps: vec![EvalStep::new(
                "step_1",
                EvalActorRole::Staff,
                "run",
                vec![EvalEvidenceChannel::ConversationEvents],
            )
            .unwrap()],
            scorecard: EvalScorecardSummary {
                schema_version: "ordo.eval_harness.v1".to_string(),
                case_id: "review_case".to_string(),
                title: "Review case".to_string(),
                fixture_hash: "fixture_hash".to_string(),
                actor_roles: vec![EvalActorRole::Staff],
                step_count: 1,
                provider_mode: "deterministic_only".to_string(),
                network_enabled: false,
                evidence_before: snapshot(vec![]),
                evidence_after: snapshot(vec![(EvalEvidenceChannel::ConversationEvents, 1)]),
                assertion_results: vec![EvalAssertionResult {
                    assertion_id: "conversation_events_exist".to_string(),
                    channel: EvalEvidenceChannel::ConversationEvents,
                    expected_minimum: 1,
                    actual_count: 1,
                    passed: true,
                    note: "ok".to_string(),
                }],
                passed: true,
                artifact_path: None,
                generated_at: "2026-05-09T00:00:00Z".to_string(),
            },
            transcript: vec![],
            timeline: vec![],
            conversation_event_ledger: vec![ledger(
                "conversation_events",
                "event_1",
                "message.created",
                json!({"messageId": "message_1"}),
            )],
            realtime_replay_ledger: vec![],
            policy_decision_ledger: vec![],
            prompt_slot_ledger: vec![],
            privacy_transform_ledger: vec![],
            token_ledger: vec![],
            analysis_candidate_ledger: vec![],
            handoff_ledger: vec![],
            artifact_ledger: vec![],
            surface_brief_ledger: vec![],
            feedback_ledger: vec![],
            review_ledger: vec![],
            product_surface_ledger: vec![],
            redaction_summary: EvalRedactionSummary {
                redaction_applied: false,
                redacted_value_count: 0,
                private_term_count: 0,
                detectors: vec!["email".to_string(), "secret".to_string()],
            },
            artifact_review: EvalArtifactReviewPlaceholder {
                status: "not_run".to_string(),
                finding_categories: EvalFindingCategory::all(),
                note: "placeholder".to_string(),
            },
        }
    }

    fn snapshot(channels: Vec<(EvalEvidenceChannel, i64)>) -> EvalEvidenceSnapshot {
        EvalEvidenceSnapshot {
            captured_at: "2026-05-09T00:00:00Z".to_string(),
            channels: channels
                .into_iter()
                .map(|(channel, count)| EvalEvidenceCount { channel, count })
                .collect(),
            conversation_event_max_sequence: Some(1),
            realtime_replay_max_cursor: Some(1),
        }
    }

    fn ledger(ledger: &str, id: &str, entry_type: &str, payload: Value) -> EvalLedgerEntry {
        EvalLedgerEntry {
            ledger: ledger.to_string(),
            id: id.to_string(),
            occurred_at: Some("2026-05-09T00:00:00Z".to_string()),
            entry_type: entry_type.to_string(),
            payload,
        }
    }

    #[test]
    fn passing_packet_has_no_failure_or_blocker_findings() {
        let review = review_packet(&base_packet()).unwrap();
        assert_eq!(review.status, "reviewed");
        assert!(review.findings.iter().all(|finding| !matches!(
            finding.severity,
            EvalArtifactFindingSeverity::Failure | EvalArtifactFindingSeverity::Blocker
        )));
    }

    #[test]
    fn failed_assertion_maps_to_channel_category() {
        let mut packet = base_packet();
        packet.scorecard.passed = false;
        packet.scorecard.assertion_results = vec![EvalAssertionResult {
            assertion_id: "token_ledger_recorded".to_string(),
            channel: EvalEvidenceChannel::TokenLedger,
            expected_minimum: 2,
            actual_count: 0,
            passed: false,
            note: "missing".to_string(),
        }];
        let review = review_packet(&packet).unwrap();
        assert!(review.findings.iter().any(|finding| {
            finding.category == EvalFindingCategory::AccountingGap
                && finding.severity == EvalArtifactFindingSeverity::Failure
                && finding
                    .evidence_refs
                    .contains(&"scorecard.assertionResults.token_ledger_recorded".to_string())
        }));
    }

    #[test]
    fn raw_sensitive_values_are_privacy_blockers_and_review_output_is_redacted() {
        let mut packet = base_packet();
        packet.conversation_event_ledger.push(ledger(
            "conversation_events",
            "event_secret",
            "message.created",
            json!({
                "body": "Email alex@example.com about Project Orchid with sk-secret-value and 555-123-4567"
            }),
        ));
        let review = review_packet(&packet).unwrap();
        assert!(review.findings.iter().any(|finding| {
            finding.category == EvalFindingCategory::PrivacyGap
                && finding.severity == EvalArtifactFindingSeverity::Blocker
        }));
        let encoded = serde_json::to_string(&review).unwrap();
        assert!(!encoded.contains("alex@example.com"));
        assert!(!encoded.contains("Project Orchid"));
        assert!(!encoded.contains("sk-secret-value"));
        assert!(!encoded.contains("555-123-4567"));
        assert!(encoded.contains("privacy_gap"));
    }

    #[test]
    fn missing_expected_ledgers_map_to_subsystem_categories() {
        let mut packet = base_packet();
        packet.steps = vec![
            EvalStep::new(
                "handoff_step",
                EvalActorRole::Staff,
                "handoff",
                vec![EvalEvidenceChannel::HandoffState],
            )
            .unwrap(),
            EvalStep::new(
                "analysis_step",
                EvalActorRole::Staff,
                "analysis",
                vec![EvalEvidenceChannel::AnalysisCandidates],
            )
            .unwrap(),
            EvalStep::new(
                "token_step",
                EvalActorRole::Staff,
                "llm",
                vec![EvalEvidenceChannel::TokenLedger],
            )
            .unwrap(),
        ];
        packet.scorecard.evidence_after = snapshot(vec![]);
        let review = review_packet(&packet).unwrap();
        assert!(review
            .findings
            .iter()
            .any(|finding| finding.category == EvalFindingCategory::HandoffGap));
        assert!(review
            .findings
            .iter()
            .any(|finding| finding.category == EvalFindingCategory::AnalysisGap));
        assert!(review
            .findings
            .iter()
            .any(|finding| finding.category == EvalFindingCategory::AccountingGap));
    }

    #[test]
    fn provider_failure_maps_to_provider_gap_with_safe_metadata() {
        let mut packet = base_packet();
        packet.conversation_event_ledger.push(ledger(
            "conversation_events",
            "event_provider_failure",
            "llm.run.failed",
            json!({
                "runId": "run_1",
                "code": "provider_timeout",
                "message": "Provider timed out"
            }),
        ));
        let review = review_packet(&packet).unwrap();
        assert!(review.findings.iter().any(|finding| {
            finding.category == EvalFindingCategory::ProviderGap
                && finding.severity == EvalArtifactFindingSeverity::Warning
        }));
    }

    #[test]
    fn review_artifacts_are_written_deterministically_without_network_or_github() {
        let temp_dir = tempfile::tempdir().unwrap();
        let packet = base_packet();
        let packet_path = temp_dir.path().join("packet.json");
        fs::write(&packet_path, serde_json::to_string_pretty(&packet).unwrap()).unwrap();
        let paths = write_review_artifacts(&packet_path, temp_dir.path()).unwrap();
        assert!(paths.review_json_path.exists());
        assert!(paths.review_markdown_path.exists());

        let first = fs::read_to_string(&paths.review_json_path).unwrap();
        let first_md = fs::read_to_string(&paths.review_markdown_path).unwrap();
        let second_paths = write_review_artifacts(&packet_path, temp_dir.path()).unwrap();
        let second = fs::read_to_string(&second_paths.review_json_path).unwrap();
        assert_eq!(first, second);
        assert!(first.contains(EVAL_ARTIFACT_REVIEW_SCHEMA_VERSION));
        assert!(first.contains("not_performed"));
        assert!(first_md.contains("# Artifact Review"));
    }
}
