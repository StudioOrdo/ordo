use anyhow::{ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::artifacts::{load_artifact, ArtifactView};
use crate::events::{append_realtime_event, system_event, RealtimeEvent};
use crate::schema::db::ConnectionExt;
use crate::security::redaction;

pub const GENERATED_CONTENT_MEMORY_SCHEMA_VERSION: &str = "generated_content_memory.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedContentMemoryKind {
    CandidateClaim,
    PreferenceMemory,
    NegativeMemory,
}

impl GeneratedContentMemoryKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::CandidateClaim => "candidate_claim",
            Self::PreferenceMemory => "preference_memory",
            Self::NegativeMemory => "negative_memory",
        }
    }

    fn memory_tier(self) -> &'static str {
        match self {
            Self::CandidateClaim => "candidate_memory",
            Self::PreferenceMemory => "preference_memory",
            Self::NegativeMemory => "negative_memory",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedContentMemoryState {
    Proposed,
    Approved,
    Rejected,
    Published,
    Superseded,
}

impl GeneratedContentMemoryState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Published => "published",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedContentMemoryIngestionInput {
    pub artifact_id: String,
    pub artifact_version_id: Option<String>,
    pub workflow_template_id: Option<String>,
    pub workflow_compilation_id: Option<String>,
    pub job_id: Option<String>,
    pub extraction_fixture_id: String,
    pub items: Vec<GeneratedContentMemoryItemInput>,
}

#[derive(Debug, Clone)]
pub struct GeneratedContentMemoryItemInput {
    pub memory_kind: GeneratedContentMemoryKind,
    pub candidate_state: Option<GeneratedContentMemoryState>,
    pub summary_text: String,
    pub body: Value,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub visibility: String,
    pub approval_evidence_refs: Vec<String>,
    pub publication_evidence_refs: Vec<String>,
    pub feedback_evidence_refs: Vec<String>,
    pub outcome_evidence_refs: Vec<String>,
    pub rejection_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedContentMemoryDecisionInput {
    pub decision: GeneratedContentMemoryState,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedContentMemoryCandidateView {
    pub id: String,
    pub artifact_id: String,
    pub artifact_version_id: Option<String>,
    pub source_artifact_kind: String,
    pub source_content_hash: String,
    pub workflow_template_id: Option<String>,
    pub workflow_compilation_id: Option<String>,
    pub job_id: Option<String>,
    pub extraction_fixture_id: String,
    pub memory_kind: String,
    pub memory_tier: String,
    pub candidate_state: String,
    pub confidence: f64,
    pub summary_text: String,
    pub body: Value,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub visibility: String,
    pub approval_evidence_refs: Vec<String>,
    pub publication_evidence_refs: Vec<String>,
    pub feedback_evidence_refs: Vec<String>,
    pub outcome_evidence_refs: Vec<String>,
    pub rejection_evidence_refs: Vec<String>,
    pub provenance: Value,
    pub content_hash: String,
    pub memory_effect: String,
    pub created_at: String,
    pub updated_at: String,
    pub state_changed_at: Option<String>,
    pub state_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedContentMemoryReviewAudience {
    Staff,
    Owner,
    Member,
    Public,
}

impl GeneratedContentMemoryReviewAudience {
    fn as_str(self) -> &'static str {
        match self {
            Self::Staff => "staff",
            Self::Owner => "owner",
            Self::Member => "member",
            Self::Public => "public",
        }
    }

    fn can_read_private_memory(self) -> bool {
        matches!(self, Self::Staff | Self::Owner)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedContentMemoryReviewPacket {
    pub schema_version: String,
    pub artifact_id: String,
    pub source_artifact_kind: String,
    pub audience: String,
    pub candidate_count: usize,
    pub source_artifact_refs: Vec<String>,
    pub workflow_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub items: Vec<GeneratedContentMemoryReviewItem>,
    pub promotion_readiness_packets: Vec<GeneratedContentMemoryPromotionReadinessPacket>,
    pub extension_points: Vec<String>,
    pub confirmed_graph_promotion: bool,
    pub live_provider_called: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedContentMemoryPromotionReadinessPacket {
    pub schema_version: String,
    pub candidate_id: String,
    pub artifact_id: String,
    pub artifact_version_id: Option<String>,
    pub source_artifact_kind: String,
    pub audience: String,
    pub read_only: bool,
    pub promotion_ready: bool,
    pub current_candidate_state: String,
    pub memory_kind: String,
    pub memory_tier: String,
    pub visibility_class: String,
    pub memory_effect: String,
    pub origin: GeneratedContentMemoryPromotionReadinessOrigin,
    pub evidence_refs: Vec<String>,
    pub decision_refs: Vec<String>,
    pub blockers: Vec<String>,
    pub allowed_next_action: String,
    pub limitations: Vec<String>,
    pub memory_promotion_performed: bool,
    pub confirmed_graph_promotion: bool,
    pub vector_mutation_performed: bool,
    pub pack_state_mutation_performed: bool,
    pub live_provider_called: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedContentMemoryPromotionReadinessOrigin {
    pub artifact_ref: String,
    pub artifact_version_ref: Option<String>,
    pub workflow_template_ref: Option<String>,
    pub workflow_compilation_ref: Option<String>,
    pub job_ref: Option<String>,
    pub actor_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedContentMemoryReviewItem {
    pub candidate_id: String,
    pub memory_kind: String,
    pub memory_tier: String,
    pub candidate_state: String,
    pub confidence: f64,
    pub summary_text: String,
    pub body: Value,
    pub body_redacted: bool,
    pub source_artifact_refs: Vec<String>,
    pub workflow_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub approval_evidence_refs: Vec<String>,
    pub publication_evidence_refs: Vec<String>,
    pub feedback_evidence_refs: Vec<String>,
    pub outcome_evidence_refs: Vec<String>,
    pub rejection_evidence_refs: Vec<String>,
    pub memory_effect: String,
    pub recommended_review_action: String,
    pub confirmed_graph_promotion: bool,
}

pub fn ingest_generated_content_memory_candidates(
    connection: &Connection,
    input: GeneratedContentMemoryIngestionInput,
) -> Result<(Vec<GeneratedContentMemoryCandidateView>, Vec<RealtimeEvent>)> {
    ensure!(
        !input.extraction_fixture_id.trim().is_empty(),
        "generated content memory requires an extraction fixture id"
    );
    ensure!(
        !input.items.is_empty(),
        "generated content memory requires candidate items"
    );
    let artifact = load_artifact(connection, &input.artifact_id)?;
    if let Some(version_id) = input.artifact_version_id.as_deref() {
        ensure!(
            artifact_version_belongs_to(connection, version_id, &artifact.id)?,
            "generated content memory artifact version must belong to source artifact"
        );
    }
    for item in &input.items {
        validate_memory_item(
            item,
            item.candidate_state
                .unwrap_or_else(|| default_state_for_kind(item.memory_kind)),
        )?;
    }

    let mut candidates = Vec::new();
    let mut events = Vec::new();
    let items = input.items.clone();
    for item in items {
        let (candidate, inserted) =
            upsert_generated_content_memory_candidate(connection, &artifact, &input, item)?;
        if inserted {
            events.push(append_realtime_event(
                connection,
                &system_event(
                    "generated_content_memory.candidate_proposed",
                    json!({
                        "candidateId": candidate.id,
                        "artifactId": candidate.artifact_id,
                        "memoryKind": candidate.memory_kind,
                        "memoryTier": candidate.memory_tier,
                        "candidateState": candidate.candidate_state,
                        "evidenceRefs": candidate.evidence_refs,
                        "contentHash": candidate.content_hash,
                    }),
                ),
            )?);
        }
        candidates.push(candidate);
    }
    Ok((candidates, events))
}

pub fn record_generated_content_memory_decision(
    connection: &Connection,
    candidate_id: &str,
    input: GeneratedContentMemoryDecisionInput,
) -> Result<(GeneratedContentMemoryCandidateView, RealtimeEvent)> {
    ensure!(
        matches!(
            input.decision,
            GeneratedContentMemoryState::Approved
                | GeneratedContentMemoryState::Rejected
                | GeneratedContentMemoryState::Published
                | GeneratedContentMemoryState::Superseded
        ),
        "generated content memory decision must be a review state"
    );
    ensure!(
        !input.reason.trim().is_empty(),
        "generated content memory decision reason is required"
    );
    ensure!(
        !input.evidence_refs.is_empty(),
        "generated content memory decision evidence is required"
    );
    let existing = load_generated_content_memory_candidate(connection, candidate_id)?;
    let mut approval_refs = existing.approval_evidence_refs.clone();
    let mut publication_refs = existing.publication_evidence_refs.clone();
    let mut rejection_refs = existing.rejection_evidence_refs.clone();
    match input.decision {
        GeneratedContentMemoryState::Approved => {
            append_unique(&mut approval_refs, &input.evidence_refs);
        }
        GeneratedContentMemoryState::Published => {
            append_unique(&mut publication_refs, &input.evidence_refs);
        }
        GeneratedContentMemoryState::Rejected | GeneratedContentMemoryState::Superseded => {
            append_unique(&mut rejection_refs, &input.evidence_refs);
        }
        GeneratedContentMemoryState::Proposed => unreachable!(),
    }
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE generated_content_memory_candidates
         SET candidate_state = ?2,
             approval_evidence_refs_json = ?3,
             publication_evidence_refs_json = ?4,
             rejection_evidence_refs_json = ?5,
             state_changed_at = ?6,
             state_reason = ?7,
             updated_at = ?6
         WHERE id = ?1",
        params![
            candidate_id,
            input.decision.as_str(),
            json!(approval_refs).to_string(),
            json!(publication_refs).to_string(),
            json!(rejection_refs).to_string(),
            now,
            safe_text(&input.reason),
        ],
    )?;
    let candidate = load_generated_content_memory_candidate(connection, candidate_id)?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "generated_content_memory.decision_recorded",
            json!({
                "candidateId": candidate.id,
                "artifactId": candidate.artifact_id,
                "candidateState": candidate.candidate_state,
                "evidenceRefs": input.evidence_refs,
                "memoryEffect": candidate.memory_effect,
            }),
        ),
    )?;
    Ok((candidate, event))
}

pub fn list_generated_content_memory_for_artifact(
    connection: &Connection,
    artifact_id: &str,
) -> Result<Vec<GeneratedContentMemoryCandidateView>> {
    connection.query_many(
        "SELECT id, artifact_id, artifact_version_id, source_artifact_kind, source_content_hash,
                workflow_template_id, workflow_compilation_id, job_id, extraction_fixture_id,
                memory_kind, memory_tier, candidate_state, confidence, summary_text, body_json,
                evidence_refs_json, limitations_json, visibility, approval_evidence_refs_json,
                publication_evidence_refs_json, feedback_evidence_refs_json,
                outcome_evidence_refs_json, rejection_evidence_refs_json, provenance_json,
                content_hash, created_at, updated_at, state_changed_at, state_reason
         FROM generated_content_memory_candidates
         WHERE artifact_id = ?1
         ORDER BY created_at ASC, id ASC",
        [artifact_id],
        generated_content_memory_from_row,
    )
}

pub fn generated_content_memory_review_packet_for_artifact(
    connection: &Connection,
    artifact_id: &str,
    audience: GeneratedContentMemoryReviewAudience,
) -> Result<GeneratedContentMemoryReviewPacket> {
    let artifact = load_artifact(connection, artifact_id)?;
    let candidates = list_generated_content_memory_for_artifact(connection, artifact_id)?;
    let items = candidates
        .iter()
        .filter(|candidate| is_story_memory_candidate(candidate))
        .map(|candidate| generated_content_memory_review_item(candidate, audience))
        .collect::<Vec<_>>();
    let promotion_readiness_packets = candidates
        .iter()
        .filter(|candidate| is_story_memory_candidate(candidate))
        .map(|candidate| generated_content_memory_promotion_readiness_packet(candidate, audience))
        .collect::<Vec<_>>();

    let mut source_artifact_refs = vec![format!("artifact:{}", artifact.id)];
    let mut workflow_refs = artifact.evidence_refs.clone();
    let mut evidence_refs = Vec::new();
    let mut limitations = Vec::new();
    for item in &items {
        append_unique(&mut source_artifact_refs, &item.source_artifact_refs);
        append_unique(&mut workflow_refs, &item.workflow_refs);
        append_unique(&mut evidence_refs, &item.evidence_refs);
        append_unique(&mut limitations, &item.limitations);
        append_unique(&mut evidence_refs, &item.approval_evidence_refs);
        append_unique(&mut evidence_refs, &item.publication_evidence_refs);
        append_unique(&mut evidence_refs, &item.feedback_evidence_refs);
        append_unique(&mut evidence_refs, &item.outcome_evidence_refs);
        append_unique(&mut evidence_refs, &item.rejection_evidence_refs);
    }
    for packet in &promotion_readiness_packets {
        append_unique(&mut evidence_refs, &packet.evidence_refs);
        append_unique(&mut evidence_refs, &packet.decision_refs);
        append_unique(&mut limitations, &packet.limitations);
    }
    append_unique(
        &mut limitations,
        &[
            "generated_content_memory_review_is_read_only".to_string(),
            "generated_content_memory_promotion_readiness_is_read_only".to_string(),
            "generated_content_memory_promotion_not_performed".to_string(),
            "generated_content_memory_candidates_do_not_confirm_graph_truth".to_string(),
            "future_owner_review_ui_and_graph_promotion_are_extension_points".to_string(),
        ],
    );
    if !audience.can_read_private_memory() {
        append_unique(
            &mut limitations,
            &[
                "member_safe_packet_redacts_candidate_bodies".to_string(),
                "member_safe_packet_omits_private_review_evidence".to_string(),
            ],
        );
    }

    Ok(GeneratedContentMemoryReviewPacket {
        schema_version: GENERATED_CONTENT_MEMORY_SCHEMA_VERSION.to_string(),
        artifact_id: artifact.id,
        source_artifact_kind: artifact.artifact_kind,
        audience: audience.as_str().to_string(),
        candidate_count: items.len(),
        source_artifact_refs,
        workflow_refs: sorted_unique(workflow_refs),
        evidence_refs: sorted_unique(evidence_refs),
        limitations: sorted_unique(limitations),
        items,
        promotion_readiness_packets,
        extension_points: vec![
            "owner_review_ui".to_string(),
            "authorized_graph_memory_promotion".to_string(),
        ],
        confirmed_graph_promotion: false,
        live_provider_called: false,
    })
}

pub fn load_generated_content_memory_candidate(
    connection: &Connection,
    candidate_id: &str,
) -> Result<GeneratedContentMemoryCandidateView> {
    connection
        .query_row(
            "SELECT id, artifact_id, artifact_version_id, source_artifact_kind, source_content_hash,
                    workflow_template_id, workflow_compilation_id, job_id, extraction_fixture_id,
                    memory_kind, memory_tier, candidate_state, confidence, summary_text, body_json,
                    evidence_refs_json, limitations_json, visibility, approval_evidence_refs_json,
                    publication_evidence_refs_json, feedback_evidence_refs_json,
                    outcome_evidence_refs_json, rejection_evidence_refs_json, provenance_json,
                    content_hash, created_at, updated_at, state_changed_at, state_reason
             FROM generated_content_memory_candidates
             WHERE id = ?1",
            [candidate_id],
            generated_content_memory_from_row,
        )
        .map_err(Into::into)
}

fn generated_content_memory_review_item(
    candidate: &GeneratedContentMemoryCandidateView,
    audience: GeneratedContentMemoryReviewAudience,
) -> GeneratedContentMemoryReviewItem {
    let private_audience = audience.can_read_private_memory();
    let body_redacted = !private_audience;
    let summary_text = if private_audience {
        candidate.summary_text.clone()
    } else {
        "Generated content memory candidate requires authorized review.".to_string()
    };
    let body = if body_redacted {
        json!({})
    } else {
        candidate.body.clone()
    };
    let mut source_artifact_refs = vec![format!("artifact:{}", candidate.artifact_id)];
    if let Some(version_id) = candidate.artifact_version_id.as_deref() {
        source_artifact_refs.push(format!("artifact_version:{version_id}"));
    }
    let workflow_refs = workflow_refs_for_candidate(candidate);
    let evidence_refs = audience_safe_refs(audience, &candidate.evidence_refs);
    let approval_evidence_refs = audience_safe_refs(audience, &candidate.approval_evidence_refs);
    let publication_evidence_refs =
        audience_safe_refs(audience, &candidate.publication_evidence_refs);
    let feedback_evidence_refs = audience_safe_refs(audience, &candidate.feedback_evidence_refs);
    let outcome_evidence_refs = audience_safe_refs(audience, &candidate.outcome_evidence_refs);
    let rejection_evidence_refs = audience_safe_refs(audience, &candidate.rejection_evidence_refs);
    let mut limitations = if private_audience {
        candidate.limitations.clone()
    } else {
        vec![
            "member_safe_packet_redacts_candidate_bodies".to_string(),
            "member_safe_packet_omits_private_review_evidence".to_string(),
        ]
    };
    append_unique(
        &mut limitations,
        &[
            "generated_content_memory_candidate_only".to_string(),
            "no_confirmed_graph_promotion".to_string(),
        ],
    );

    GeneratedContentMemoryReviewItem {
        candidate_id: candidate.id.clone(),
        memory_kind: candidate.memory_kind.clone(),
        memory_tier: candidate.memory_tier.clone(),
        candidate_state: candidate.candidate_state.clone(),
        confidence: candidate.confidence,
        summary_text,
        body,
        body_redacted,
        source_artifact_refs: sorted_unique(source_artifact_refs),
        workflow_refs,
        evidence_refs,
        limitations: sorted_unique(limitations),
        approval_evidence_refs,
        publication_evidence_refs,
        feedback_evidence_refs,
        outcome_evidence_refs,
        rejection_evidence_refs,
        memory_effect: candidate.memory_effect.clone(),
        recommended_review_action: recommended_review_action(candidate).to_string(),
        confirmed_graph_promotion: false,
    }
}

fn generated_content_memory_promotion_readiness_packet(
    candidate: &GeneratedContentMemoryCandidateView,
    audience: GeneratedContentMemoryReviewAudience,
) -> GeneratedContentMemoryPromotionReadinessPacket {
    let mut evidence_refs = audience_safe_refs(audience, &candidate.evidence_refs);
    let mut decision_refs = Vec::new();
    append_unique(
        &mut decision_refs,
        &audience_safe_refs(audience, &candidate.approval_evidence_refs),
    );
    append_unique(
        &mut decision_refs,
        &audience_safe_refs(audience, &candidate.publication_evidence_refs),
    );
    append_unique(
        &mut decision_refs,
        &audience_safe_refs(audience, &candidate.feedback_evidence_refs),
    );
    append_unique(
        &mut decision_refs,
        &audience_safe_refs(audience, &candidate.outcome_evidence_refs),
    );
    append_unique(
        &mut decision_refs,
        &audience_safe_refs(audience, &candidate.rejection_evidence_refs),
    );
    append_unique(&mut evidence_refs, &workflow_refs_for_candidate(candidate));

    let blockers = promotion_readiness_blockers(candidate);
    let promotion_ready = blockers.is_empty();
    let mut limitations = vec![
        "memory_promotion_readiness_packet_is_read_only".to_string(),
        "memory_promotion_not_performed".to_string(),
        "canonical_memory_not_mutated".to_string(),
        "confirmed_graph_promotion_not_performed".to_string(),
        "vector_index_not_mutated".to_string(),
        "pack_state_not_mutated".to_string(),
        "live_provider_not_called".to_string(),
    ];
    if audience.can_read_private_memory() {
        append_unique(&mut limitations, &candidate.limitations);
    } else {
        append_unique(
            &mut limitations,
            &[
                "member_safe_packet_redacts_candidate_bodies".to_string(),
                "member_safe_packet_omits_private_review_evidence".to_string(),
            ],
        );
    }

    GeneratedContentMemoryPromotionReadinessPacket {
        schema_version: "generated_content_memory_promotion_readiness.v1".to_string(),
        candidate_id: candidate.id.clone(),
        artifact_id: candidate.artifact_id.clone(),
        artifact_version_id: candidate.artifact_version_id.clone(),
        source_artifact_kind: candidate.source_artifact_kind.clone(),
        audience: audience.as_str().to_string(),
        read_only: true,
        promotion_ready,
        current_candidate_state: candidate.candidate_state.clone(),
        memory_kind: candidate.memory_kind.clone(),
        memory_tier: candidate.memory_tier.clone(),
        visibility_class: visibility_class(&candidate.visibility).to_string(),
        memory_effect: candidate.memory_effect.clone(),
        origin: GeneratedContentMemoryPromotionReadinessOrigin {
            artifact_ref: format!("artifact:{}", candidate.artifact_id),
            artifact_version_ref: candidate
                .artifact_version_id
                .as_deref()
                .map(|value| format!("artifact_version:{}", safe_identifier(value))),
            workflow_template_ref: candidate
                .workflow_template_id
                .as_deref()
                .map(|value| format!("workflow_template:{}", safe_identifier(value))),
            workflow_compilation_ref: candidate
                .workflow_compilation_id
                .as_deref()
                .map(|value| format!("workflow_compilation:{}", safe_identifier(value))),
            job_ref: candidate
                .job_id
                .as_deref()
                .map(|value| format!("job:{}", safe_identifier(value))),
            actor_ref: None,
        },
        evidence_refs: sorted_unique(evidence_refs),
        decision_refs: sorted_unique(decision_refs),
        blockers,
        allowed_next_action: if promotion_ready {
            "prepare_owner_memory_promotion_review".to_string()
        } else {
            "resolve_memory_readiness_blockers".to_string()
        },
        limitations: sorted_unique(limitations),
        memory_promotion_performed: false,
        confirmed_graph_promotion: false,
        vector_mutation_performed: false,
        pack_state_mutation_performed: false,
        live_provider_called: false,
    }
}

fn promotion_readiness_blockers(candidate: &GeneratedContentMemoryCandidateView) -> Vec<String> {
    let mut blockers = Vec::new();
    if candidate.candidate_state != "approved" {
        blockers.push(format!(
            "candidate_state_{}_blocks_promotion_readiness",
            safe_identifier(&candidate.candidate_state)
        ));
    }
    if candidate.approval_evidence_refs.is_empty() {
        blockers.push("approval_evidence_required".to_string());
    }
    if !matches!(
        candidate.memory_kind.as_str(),
        "candidate_claim" | "preference_memory"
    ) {
        blockers.push("unsupported_memory_kind_blocks_promotion_readiness".to_string());
    }
    if matches!(
        visibility_class(&candidate.visibility),
        "staff" | "owner" | "private"
    ) {
        blockers.push("private_visibility_blocks_promotion_readiness".to_string());
    }
    sorted_unique(blockers)
}

fn visibility_class(visibility: &str) -> &'static str {
    match visibility {
        "public" => "public",
        "authenticated" | "member" => "member",
        "staff" | "staff_private" => "staff",
        "owner" | "owner_private" => "owner",
        _ => "private",
    }
}

fn is_story_memory_candidate(candidate: &GeneratedContentMemoryCandidateView) -> bool {
    candidate.source_artifact_kind.starts_with("story.")
        || candidate
            .workflow_template_id
            .as_deref()
            .is_some_and(|value| value.contains("story"))
        || candidate
            .job_id
            .as_deref()
            .is_some_and(|value| value.contains("story"))
}

fn workflow_refs_for_candidate(candidate: &GeneratedContentMemoryCandidateView) -> Vec<String> {
    let mut refs = Vec::new();
    if let Some(value) = candidate.workflow_template_id.as_deref() {
        refs.push(format!("workflow_template:{}", safe_identifier(value)));
    }
    if let Some(value) = candidate.workflow_compilation_id.as_deref() {
        refs.push(format!("workflow_compilation:{}", safe_identifier(value)));
    }
    if let Some(value) = candidate.job_id.as_deref() {
        refs.push(format!("job:{}", safe_identifier(value)));
    }
    sorted_unique(refs)
}

fn audience_safe_refs(
    audience: GeneratedContentMemoryReviewAudience,
    refs: &[String],
) -> Vec<String> {
    if audience.can_read_private_memory() {
        return sorted_unique(refs.to_vec());
    }
    sorted_unique(
        refs.iter()
            .filter(|value| {
                value.starts_with("artifact:") || value.starts_with("artifact_version:")
            })
            .cloned()
            .collect(),
    )
}

fn recommended_review_action(candidate: &GeneratedContentMemoryCandidateView) -> &'static str {
    match candidate.candidate_state.as_str() {
        "published" => "review_for_memory_strengthening",
        "approved" => "consider_publication_or_memory_review",
        "rejected" if candidate.memory_kind == "negative_memory" => {
            "retain_as_negative_or_preference_memory"
        }
        "rejected" => "keep_rejected_as_limitation",
        "superseded" => "archive_superseded_candidate",
        "proposed" if candidate.confidence < 0.5 => "needs_more_evidence",
        "proposed" => "review_candidate",
        _ => "review_candidate",
    }
}

fn upsert_generated_content_memory_candidate(
    connection: &Connection,
    artifact: &ArtifactView,
    input: &GeneratedContentMemoryIngestionInput,
    item: GeneratedContentMemoryItemInput,
) -> Result<(GeneratedContentMemoryCandidateView, bool)> {
    let state = item
        .candidate_state
        .unwrap_or_else(|| default_state_for_kind(item.memory_kind));
    validate_memory_item(&item, state)?;
    let evidence_refs = memory_evidence_refs(artifact, input.artifact_version_id.as_deref(), &item);
    let summary_text = safe_text(&item.summary_text);
    let body = safe_json(item.body);
    let limitations = safe_vec(item.limitations);
    let provenance = json!({
        "schemaVersion": GENERATED_CONTENT_MEMORY_SCHEMA_VERSION,
        "source": "generated_content_artifact",
        "artifactId": artifact.id,
        "artifactKind": artifact.artifact_kind,
        "artifactContentHash": artifact.content_hash,
        "artifactVersionId": input.artifact_version_id,
        "workflowTemplateId": input.workflow_template_id,
        "workflowCompilationId": input.workflow_compilation_id,
        "jobId": input.job_id,
        "extractionFixtureId": safe_identifier(&input.extraction_fixture_id),
        "truthBoundary": "candidate_memory_only",
        "confirmedGraphPromotion": false,
        "liveProviderCalled": false,
    });
    let content_hash = stable_hash(&format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        artifact.id,
        input.artifact_version_id.as_deref().unwrap_or("none"),
        input.extraction_fixture_id,
        item.memory_kind.as_str(),
        state.as_str(),
        summary_text,
        body,
        json!(evidence_refs)
    ));
    let id = stable_id("generated_content_memory_candidate", &content_hash);
    let now = Utc::now().to_rfc3339();
    let inserted = connection.execute(
        "INSERT OR IGNORE INTO generated_content_memory_candidates (
            id, artifact_id, artifact_version_id, source_artifact_kind, source_content_hash,
            workflow_template_id, workflow_compilation_id, job_id, extraction_fixture_id,
            memory_kind, memory_tier, candidate_state, confidence, summary_text, body_json,
            evidence_refs_json, limitations_json, visibility, approval_evidence_refs_json,
            publication_evidence_refs_json, feedback_evidence_refs_json,
            outcome_evidence_refs_json, rejection_evidence_refs_json, provenance_json,
            content_hash, created_at, updated_at, state_changed_at, state_reason
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                   ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?26, ?27, ?28)",
        params![
            id,
            artifact.id,
            input.artifact_version_id,
            artifact.artifact_kind,
            artifact.content_hash,
            input.workflow_template_id,
            input.workflow_compilation_id,
            input.job_id,
            safe_identifier(&input.extraction_fixture_id),
            item.memory_kind.as_str(),
            item.memory_kind.memory_tier(),
            state.as_str(),
            item.confidence,
            summary_text,
            body.to_string(),
            json!(evidence_refs).to_string(),
            json!(limitations).to_string(),
            normalize_visibility(&item.visibility),
            json!(safe_vec(item.approval_evidence_refs)).to_string(),
            json!(safe_vec(item.publication_evidence_refs)).to_string(),
            json!(safe_vec(item.feedback_evidence_refs)).to_string(),
            json!(safe_vec(item.outcome_evidence_refs)).to_string(),
            json!(safe_vec(item.rejection_evidence_refs)).to_string(),
            provenance.to_string(),
            content_hash,
            now,
            if state == GeneratedContentMemoryState::Proposed {
                None::<String>
            } else {
                Some(now.clone())
            },
            if state == GeneratedContentMemoryState::Proposed {
                None::<String>
            } else {
                Some(format!(
                    "Initial {} evidence recorded from deterministic fixture.",
                    state.as_str()
                ))
            },
        ],
    )? == 1;
    Ok((
        load_generated_content_memory_candidate(connection, &id)?,
        inserted,
    ))
}

fn validate_memory_item(
    item: &GeneratedContentMemoryItemInput,
    state: GeneratedContentMemoryState,
) -> Result<()> {
    ensure!(
        !item.summary_text.trim().is_empty(),
        "generated content memory summary is required"
    );
    ensure!(
        (0.0..=1.0).contains(&item.confidence),
        "generated content memory confidence must be between 0 and 1"
    );
    ensure!(
        !item.evidence_refs.is_empty(),
        "generated content memory evidence refs are required"
    );
    ensure!(
        item.body.is_object(),
        "generated content memory body must be structured"
    );
    ensure!(
        matches!(
            item.memory_kind,
            GeneratedContentMemoryKind::CandidateClaim
                | GeneratedContentMemoryKind::PreferenceMemory
                | GeneratedContentMemoryKind::NegativeMemory
        ),
        "unsupported generated content memory kind"
    );
    if item.memory_kind == GeneratedContentMemoryKind::NegativeMemory {
        ensure!(
            state == GeneratedContentMemoryState::Rejected,
            "negative memory requires rejected candidate state"
        );
        ensure!(
            !item.rejection_evidence_refs.is_empty(),
            "negative memory requires rejection evidence"
        );
    }
    if state == GeneratedContentMemoryState::Approved {
        ensure!(
            !item.approval_evidence_refs.is_empty(),
            "approved memory candidates require approval evidence"
        );
    }
    if state == GeneratedContentMemoryState::Published {
        ensure!(
            !item.publication_evidence_refs.is_empty(),
            "published memory candidates require publication evidence"
        );
    }
    ensure_safe_memory_text(&item.summary_text)?;
    ensure_safe_memory_text(&item.body.to_string())?;
    Ok(())
}

fn memory_evidence_refs(
    artifact: &ArtifactView,
    artifact_version_id: Option<&str>,
    item: &GeneratedContentMemoryItemInput,
) -> Vec<String> {
    let mut refs = vec![format!("artifact:{}", artifact.id)];
    if let Some(version_id) = artifact_version_id {
        refs.push(format!("artifact_version:{version_id}"));
    }
    refs.extend(
        item.evidence_refs
            .iter()
            .map(|value| safe_identifier(value)),
    );
    append_unique(&mut refs, &artifact.evidence_refs);
    refs
}

fn artifact_version_belongs_to(
    connection: &Connection,
    version_id: &str,
    artifact_id: &str,
) -> Result<bool> {
    let matched = connection
        .query_row(
            "SELECT 1 FROM artifact_versions WHERE id = ?1 AND artifact_id = ?2",
            params![version_id, artifact_id],
            |_row| Ok(()),
        )
        .optional()?
        .is_some();
    Ok(matched)
}

fn generated_content_memory_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<GeneratedContentMemoryCandidateView> {
    let body_json: String = row.get(14)?;
    let evidence_json: String = row.get(15)?;
    let limitations_json: String = row.get(16)?;
    let approval_json: String = row.get(18)?;
    let publication_json: String = row.get(19)?;
    let feedback_json: String = row.get(20)?;
    let outcome_json: String = row.get(21)?;
    let rejection_json: String = row.get(22)?;
    let provenance_json: String = row.get(23)?;
    let candidate_state: String = row.get(11)?;
    Ok(GeneratedContentMemoryCandidateView {
        id: row.get(0)?,
        artifact_id: row.get(1)?,
        artifact_version_id: row.get(2)?,
        source_artifact_kind: row.get(3)?,
        source_content_hash: row.get(4)?,
        workflow_template_id: row.get(5)?,
        workflow_compilation_id: row.get(6)?,
        job_id: row.get(7)?,
        extraction_fixture_id: row.get(8)?,
        memory_kind: row.get(9)?,
        memory_tier: row.get(10)?,
        candidate_state: candidate_state.clone(),
        confidence: row.get(12)?,
        summary_text: row.get(13)?,
        body: serde_json::from_str(&body_json).unwrap_or_else(|_| json!({})),
        evidence_refs: serde_json::from_str(&evidence_json).unwrap_or_default(),
        limitations: serde_json::from_str(&limitations_json).unwrap_or_default(),
        visibility: row.get(17)?,
        approval_evidence_refs: serde_json::from_str(&approval_json).unwrap_or_default(),
        publication_evidence_refs: serde_json::from_str(&publication_json).unwrap_or_default(),
        feedback_evidence_refs: serde_json::from_str(&feedback_json).unwrap_or_default(),
        outcome_evidence_refs: serde_json::from_str(&outcome_json).unwrap_or_default(),
        rejection_evidence_refs: serde_json::from_str(&rejection_json).unwrap_or_default(),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        content_hash: row.get(24)?,
        memory_effect: memory_effect_for_state(&candidate_state),
        created_at: row.get(25)?,
        updated_at: row.get(26)?,
        state_changed_at: row.get(27)?,
        state_reason: row.get(28)?,
    })
}

fn default_state_for_kind(kind: GeneratedContentMemoryKind) -> GeneratedContentMemoryState {
    match kind {
        GeneratedContentMemoryKind::NegativeMemory => GeneratedContentMemoryState::Rejected,
        GeneratedContentMemoryKind::CandidateClaim
        | GeneratedContentMemoryKind::PreferenceMemory => GeneratedContentMemoryState::Proposed,
    }
}

fn memory_effect_for_state(state: &str) -> String {
    match state {
        "approved" | "published" => "candidate_stronger_evidence".to_string(),
        "rejected" | "superseded" => "candidate_weakened_or_negative".to_string(),
        _ => "candidate_only".to_string(),
    }
}

fn ensure_safe_memory_text(text: &str) -> Result<()> {
    let lower = text.to_ascii_lowercase();
    let blocked = [
        "prompt internal",
        "promptinternals",
        "provider internal",
        "provider payload",
        "raw policy",
        "policy internal",
        "owner-only",
        "private artifact text",
        "task private payload",
        "staff routing",
        "graph certainty",
        "confirmed graph truth",
        "unsupported claim",
        "secret",
        "api_key",
        "password",
        "bearer ",
    ];
    ensure!(
        !blocked.iter().any(|needle| lower.contains(needle)),
        "generated content memory contains private/internal or unsupported claim text"
    );
    ensure!(
        !redaction::contains_sensitive_text(text, &[]),
        "generated content memory contains sensitive text"
    );
    Ok(())
}

fn safe_text(text: &str) -> String {
    redaction::redact_public_text(text.trim())
}

fn safe_json(value: Value) -> Value {
    redaction::sanitize_json_strings(value)
}

fn safe_vec(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| safe_text(&value))
        .filter(|value| !value.trim().is_empty())
        .collect()
}

fn safe_identifier(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | ':' | '.' | '/')
            {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
}

fn normalize_visibility(value: &str) -> String {
    match value.trim() {
        "public" => "public",
        "authenticated" | "member" => "authenticated",
        "staff" | "staff_private" => "staff",
        "owner" | "owner_private" => "owner",
        _ => "staff",
    }
    .to_string()
}

fn append_unique(target: &mut Vec<String>, refs: &[String]) {
    for value in refs {
        let safe = safe_identifier(value);
        if !safe.trim().is_empty() && !target.contains(&safe) {
            target.push(safe);
        }
    }
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values.retain(|value| !value.trim().is_empty());
    values.sort();
    values.dedup();
    values
}

fn stable_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn stable_id(prefix: &str, content_hash: &str) -> String {
    let suffix = content_hash
        .strip_prefix("sha256:")
        .unwrap_or(content_hash)
        .chars()
        .take(24)
        .collect::<String>();
    format!("{prefix}_{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::{add_artifact_version, record_artifact, ArtifactInput};
    use crate::schema::init_schema;

    fn generated_artifact(connection: &Connection, content_hash: &str) -> ArtifactView {
        record_artifact(
            connection,
            ArtifactInput {
                artifact_kind: "story.homepage_draft".to_string(),
                title: "Homepage draft".to_string(),
                status: "draft".to_string(),
                visibility_ceiling: "staff".to_string(),
                summary: "Generated public story draft awaiting review.".to_string(),
                source_kind: Some("workflow_task".to_string()),
                source_id: Some("task_story_draft".to_string()),
                evidence_refs: vec!["workflow:story.scrollytelling_homepage".to_string()],
                provenance: json!({
                    "schemaVersion": "test.generated_artifact.v1",
                    "generatedBy": "content.preparePublicStoryDraft",
                    "liveProviderCalled": false,
                }),
                content_hash: content_hash.to_string(),
                storage_uri: Some(format!("ordo://artifacts/story/{content_hash}")),
                health_status: Some("contract_only".to_string()),
                created_by_job_id: None,
            },
        )
        .unwrap()
        .0
    }

    fn claim_item(summary: &str) -> GeneratedContentMemoryItemInput {
        GeneratedContentMemoryItemInput {
            memory_kind: GeneratedContentMemoryKind::CandidateClaim,
            candidate_state: None,
            summary_text: summary.to_string(),
            body: json!({
                "claim": summary,
                "source": "deterministic_fixture",
            }),
            confidence: 0.64,
            evidence_refs: vec!["artifact_version:v1".to_string()],
            limitations: vec![
                "Generated draft requires owner review before becoming truth.".to_string(),
            ],
            visibility: "staff".to_string(),
            approval_evidence_refs: vec![],
            publication_evidence_refs: vec![],
            feedback_evidence_refs: vec![],
            outcome_evidence_refs: vec![],
            rejection_evidence_refs: vec![],
        }
    }

    #[test]
    fn generated_artifact_proposes_candidate_memory_without_confirming_graph_truth() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let artifact = generated_artifact(&connection, "sha256:generated-story-v1");
        let version = add_artifact_version(
            &connection,
            &artifact.id,
            "sha256:generated-story-v1-version",
            artifact.storage_uri.as_deref(),
            json!({"fixture": "story"}),
        )
        .unwrap();

        let (candidates, events) = ingest_generated_content_memory_candidates(
            &connection,
            GeneratedContentMemoryIngestionInput {
                artifact_id: artifact.id.clone(),
                artifact_version_id: Some(version.id.clone()),
                workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
                workflow_compilation_id: Some("workflow_compilation_story_1".to_string()),
                job_id: Some("job_story_1".to_string()),
                extraction_fixture_id: "fixture.story.claims.v1".to_string(),
                items: vec![claim_item(
                    "Ordo helps the owner turn approved story evidence into a homepage.",
                )],
            },
        )
        .unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(events.len(), 1);
        let candidate = &candidates[0];
        assert_eq!(candidate.artifact_id, artifact.id);
        assert_eq!(candidate.artifact_version_id, Some(version.id));
        assert_eq!(candidate.memory_kind, "candidate_claim");
        assert_eq!(candidate.memory_tier, "candidate_memory");
        assert_eq!(candidate.candidate_state, "proposed");
        assert_eq!(candidate.memory_effect, "candidate_only");
        assert!(candidate
            .limitations
            .contains(&"Generated draft requires owner review before becoming truth.".to_string()));
        assert_eq!(
            candidate.provenance["confirmedGraphPromotion"],
            json!(false)
        );
        assert!(candidate
            .evidence_refs
            .iter()
            .any(|reference| reference == &format!("artifact:{}", candidate.artifact_id)));

        let graph_node_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM graph_nodes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(graph_node_count, 0);
    }

    #[test]
    fn rejects_private_provider_prompt_policy_graph_certainty_and_sensitive_text() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let artifact = generated_artifact(&connection, "sha256:generated-story-private");

        let mut item = claim_item("Provider internal prompt internals prove graph certainty.");
        item.body = json!({
            "claim": "Use raw policy internals and sk-live-secret in public memory",
        });
        let error = ingest_generated_content_memory_candidates(
            &connection,
            GeneratedContentMemoryIngestionInput {
                artifact_id: artifact.id,
                artifact_version_id: None,
                workflow_template_id: None,
                workflow_compilation_id: None,
                job_id: None,
                extraction_fixture_id: "fixture.story.claims.v1".to_string(),
                items: vec![item],
            },
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("private/internal or unsupported claim"));
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM generated_content_memory_candidates",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn rejects_invalid_batch_without_partial_memory_or_events() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let artifact = generated_artifact(&connection, "sha256:generated-story-batch-private");

        let valid_item =
            claim_item("A safe candidate claim should not persist from a failed batch.");
        let mut invalid_item =
            claim_item("Provider internal payload should reject the whole batch.");
        invalid_item.body = json!({
            "claim": "This includes raw policy internals and task private payload data",
        });

        let error = ingest_generated_content_memory_candidates(
            &connection,
            GeneratedContentMemoryIngestionInput {
                artifact_id: artifact.id,
                artifact_version_id: None,
                workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
                workflow_compilation_id: Some("workflow_compilation_story_batch".to_string()),
                job_id: Some("job_story_batch".to_string()),
                extraction_fixture_id: "fixture.story.claims.batch.v1".to_string(),
                items: vec![valid_item, invalid_item],
            },
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("private/internal or unsupported claim"));
        let candidate_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM generated_content_memory_candidates",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(candidate_count, 0);
        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM realtime_events WHERE event_type LIKE 'generated_content_memory.%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 0);
    }

    #[test]
    fn rejected_content_records_negative_memory_without_public_truth() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let artifact = generated_artifact(&connection, "sha256:generated-story-rejected");

        let (candidates, _) = ingest_generated_content_memory_candidates(
            &connection,
            GeneratedContentMemoryIngestionInput {
                artifact_id: artifact.id,
                artifact_version_id: None,
                workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
                workflow_compilation_id: None,
                job_id: Some("job_story_2".to_string()),
                extraction_fixture_id: "fixture.story.rejections.v1".to_string(),
                items: vec![GeneratedContentMemoryItemInput {
                    memory_kind: GeneratedContentMemoryKind::NegativeMemory,
                    candidate_state: None,
                    summary_text: "Do not reuse the rejected heavy jargon direction.".to_string(),
                    body: json!({"preference": "avoid heavy jargon"}),
                    confidence: 0.8,
                    evidence_refs: vec!["artifact_review:review_1".to_string()],
                    limitations: vec![
                        "Rejected content should guide style, not public claims.".to_string()
                    ],
                    visibility: "staff".to_string(),
                    approval_evidence_refs: vec![],
                    publication_evidence_refs: vec![],
                    feedback_evidence_refs: vec![],
                    outcome_evidence_refs: vec![],
                    rejection_evidence_refs: vec!["artifact_review:review_1".to_string()],
                }],
            },
        )
        .unwrap();

        let candidate = &candidates[0];
        assert_eq!(candidate.memory_kind, "negative_memory");
        assert_eq!(candidate.memory_tier, "negative_memory");
        assert_eq!(candidate.candidate_state, "rejected");
        assert_eq!(candidate.memory_effect, "candidate_weakened_or_negative");
        assert!(candidate
            .rejection_evidence_refs
            .contains(&"artifact_review:review_1".to_string()));

        let graph_node_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM graph_nodes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(graph_node_count, 0);
    }

    #[test]
    fn reingestion_is_idempotent_and_new_artifact_version_is_distinct() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let artifact = generated_artifact(&connection, "sha256:generated-story-idempotent");
        let first_version = add_artifact_version(
            &connection,
            &artifact.id,
            "sha256:generated-story-version-1",
            artifact.storage_uri.as_deref(),
            json!({"version": 1}),
        )
        .unwrap();
        let input = || GeneratedContentMemoryIngestionInput {
            artifact_id: artifact.id.clone(),
            artifact_version_id: Some(first_version.id.clone()),
            workflow_template_id: None,
            workflow_compilation_id: None,
            job_id: Some("job_story_3".to_string()),
            extraction_fixture_id: "fixture.story.claims.v1".to_string(),
            items: vec![claim_item(
                "The story draft can propose memory without becoming truth.",
            )],
        };

        let first = ingest_generated_content_memory_candidates(&connection, input()).unwrap();
        let second = ingest_generated_content_memory_candidates(&connection, input()).unwrap();
        assert_eq!(first.0[0].id, second.0[0].id);

        let second_version = add_artifact_version(
            &connection,
            &artifact.id,
            "sha256:generated-story-version-2",
            artifact.storage_uri.as_deref(),
            json!({"version": 2}),
        )
        .unwrap();
        let third = ingest_generated_content_memory_candidates(
            &connection,
            GeneratedContentMemoryIngestionInput {
                artifact_id: artifact.id.clone(),
                artifact_version_id: Some(second_version.id),
                workflow_template_id: None,
                workflow_compilation_id: None,
                job_id: Some("job_story_3".to_string()),
                extraction_fixture_id: "fixture.story.claims.v1".to_string(),
                items: vec![claim_item(
                    "The story draft can propose memory without becoming truth.",
                )],
            },
        )
        .unwrap();
        assert_ne!(first.0[0].id, third.0[0].id);

        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM generated_content_memory_candidates",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn approval_publication_feedback_and_outcome_evidence_strengthen_candidate_only_memory() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let artifact = generated_artifact(&connection, "sha256:generated-story-decision");
        let (candidates, _) = ingest_generated_content_memory_candidates(
            &connection,
            GeneratedContentMemoryIngestionInput {
                artifact_id: artifact.id,
                artifact_version_id: None,
                workflow_template_id: None,
                workflow_compilation_id: None,
                job_id: Some("job_story_4".to_string()),
                extraction_fixture_id: "fixture.story.claims.v1".to_string(),
                items: vec![claim_item(
                    "Ordo public story claims require evidence before publication.",
                )],
            },
        )
        .unwrap();

        let (approved, event) = record_generated_content_memory_decision(
            &connection,
            &candidates[0].id,
            GeneratedContentMemoryDecisionInput {
                decision: GeneratedContentMemoryState::Approved,
                reason: "Owner approved this as candidate memory evidence.".to_string(),
                evidence_refs: vec!["approval:owner_1".to_string()],
            },
        )
        .unwrap();

        assert_eq!(approved.candidate_state, "approved");
        assert_eq!(approved.memory_effect, "candidate_stronger_evidence");
        assert!(approved
            .approval_evidence_refs
            .contains(&"approval:owner_1".to_string()));
        assert_eq!(
            event.event_type,
            "generated_content_memory.decision_recorded"
        );
        let graph_node_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM graph_nodes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(graph_node_count, 0);
    }

    #[test]
    fn publication_feedback_and_outcome_evidence_are_retained_without_truth_promotion() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let artifact = generated_artifact(&connection, "sha256:generated-story-outcome");
        let mut item =
            claim_item("Published story content can become stronger candidate evidence.");
        item.candidate_state = Some(GeneratedContentMemoryState::Published);
        item.publication_evidence_refs = vec!["publication:homepage_v1".to_string()];
        item.feedback_evidence_refs = vec!["feedback:member_1".to_string()];
        item.outcome_evidence_refs = vec!["outcome:trial_started_1".to_string()];

        let (candidates, _) = ingest_generated_content_memory_candidates(
            &connection,
            GeneratedContentMemoryIngestionInput {
                artifact_id: artifact.id,
                artifact_version_id: None,
                workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
                workflow_compilation_id: Some("workflow_compilation_story_2".to_string()),
                job_id: Some("job_story_5".to_string()),
                extraction_fixture_id: "fixture.story.published.v1".to_string(),
                items: vec![item],
            },
        )
        .unwrap();

        let candidate = &candidates[0];
        assert_eq!(candidate.candidate_state, "published");
        assert_eq!(candidate.memory_effect, "candidate_stronger_evidence");
        assert!(candidate
            .publication_evidence_refs
            .contains(&"publication:homepage_v1".to_string()));
        assert!(candidate
            .feedback_evidence_refs
            .contains(&"feedback:member_1".to_string()));
        assert!(candidate
            .outcome_evidence_refs
            .contains(&"outcome:trial_started_1".to_string()));

        let graph_node_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM graph_nodes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(graph_node_count, 0);
    }

    #[test]
    fn story_memory_review_packet_summarizes_states_and_evidence_without_truth_promotion() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let artifact = generated_artifact(&connection, "sha256:generated-story-review-packet");

        let mut published = claim_item("Published story content can inform candidate memory.");
        published.candidate_state = Some(GeneratedContentMemoryState::Published);
        published.publication_evidence_refs = vec!["publication:homepage_v1".to_string()];
        published.feedback_evidence_refs = vec!["feedback:visitor_response_1".to_string()];
        published.outcome_evidence_refs = vec!["outcome:trial_started_1".to_string()];
        let mut superseded =
            claim_item("Earlier story framing was replaced by a stronger owner-approved version.");
        superseded.confidence = 0.72;
        let mut low_confidence =
            claim_item("Weak draft claim needs more evidence before any memory review.");
        low_confidence.confidence = 0.32;
        let rejected = GeneratedContentMemoryItemInput {
            memory_kind: GeneratedContentMemoryKind::NegativeMemory,
            candidate_state: None,
            summary_text: "Do not reuse the rejected heavy jargon direction.".to_string(),
            body: json!({"preference": "avoid heavy jargon"}),
            confidence: 0.8,
            evidence_refs: vec!["artifact_review:review_1".to_string()],
            limitations: vec!["Rejected content guides preference memory only.".to_string()],
            visibility: "staff".to_string(),
            approval_evidence_refs: vec![],
            publication_evidence_refs: vec![],
            feedback_evidence_refs: vec![],
            outcome_evidence_refs: vec![],
            rejection_evidence_refs: vec!["artifact_review:review_1".to_string()],
        };

        let (candidates, _) = ingest_generated_content_memory_candidates(
            &connection,
            GeneratedContentMemoryIngestionInput {
                artifact_id: artifact.id.clone(),
                artifact_version_id: None,
                workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
                workflow_compilation_id: Some("workflow_compilation_story_packet".to_string()),
                job_id: Some("job_story_packet".to_string()),
                extraction_fixture_id: "fixture.story.review_packet.v1".to_string(),
                items: vec![published, superseded, low_confidence, rejected],
            },
        )
        .unwrap();
        let superseded_candidate = candidates
            .iter()
            .find(|candidate| {
                candidate
                    .summary_text
                    .starts_with("Earlier story framing was replaced")
            })
            .unwrap();
        record_generated_content_memory_decision(
            &connection,
            &superseded_candidate.id,
            GeneratedContentMemoryDecisionInput {
                decision: GeneratedContentMemoryState::Superseded,
                reason: "Owner approved newer story framing.".to_string(),
                evidence_refs: vec!["artifact_review:superseded_by_v2".to_string()],
            },
        )
        .unwrap();

        let packet = generated_content_memory_review_packet_for_artifact(
            &connection,
            &artifact.id,
            GeneratedContentMemoryReviewAudience::Staff,
        )
        .unwrap();

        assert_eq!(
            packet.schema_version,
            GENERATED_CONTENT_MEMORY_SCHEMA_VERSION
        );
        assert_eq!(packet.artifact_id, artifact.id);
        assert_eq!(packet.audience, "staff");
        assert_eq!(packet.candidate_count, 4);
        assert_eq!(packet.confirmed_graph_promotion, false);
        assert_eq!(packet.live_provider_called, false);
        assert!(packet
            .workflow_refs
            .contains(&"workflow_template:studio.story.scrollytelling_homepage".to_string()));
        assert!(packet
            .evidence_refs
            .contains(&"publication:homepage_v1".to_string()));
        assert!(packet
            .evidence_refs
            .contains(&"feedback:visitor_response_1".to_string()));
        assert!(packet
            .evidence_refs
            .contains(&"outcome:trial_started_1".to_string()));
        assert!(packet
            .evidence_refs
            .contains(&"artifact_review:review_1".to_string()));
        assert!(packet
            .evidence_refs
            .contains(&"artifact_review:superseded_by_v2".to_string()));

        let published_item = packet
            .items
            .iter()
            .find(|item| item.candidate_state == "published")
            .unwrap();
        assert_eq!(
            published_item.recommended_review_action,
            "review_for_memory_strengthening"
        );
        assert_eq!(published_item.body_redacted, false);
        assert!(published_item.body.is_object());

        let rejected_item = packet
            .items
            .iter()
            .find(|item| item.memory_kind == "negative_memory")
            .unwrap();
        assert_eq!(rejected_item.memory_tier, "negative_memory");
        assert_eq!(
            rejected_item.recommended_review_action,
            "retain_as_negative_or_preference_memory"
        );

        let superseded_item = packet
            .items
            .iter()
            .find(|item| item.candidate_state == "superseded")
            .unwrap();
        assert_eq!(
            superseded_item.recommended_review_action,
            "archive_superseded_candidate"
        );

        let low_confidence_item = packet
            .items
            .iter()
            .find(|item| item.confidence < 0.5)
            .unwrap();
        assert_eq!(
            low_confidence_item.recommended_review_action,
            "needs_more_evidence"
        );

        let graph_node_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM graph_nodes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(graph_node_count, 0);
    }

    #[test]
    fn approved_candidate_exposes_read_only_promotion_readiness_packet_without_mutation() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let artifact =
            generated_artifact(&connection, "sha256:generated-story-promotion-readiness");
        let version = add_artifact_version(
            &connection,
            &artifact.id,
            "sha256:generated-story-promotion-readiness-version",
            artifact.storage_uri.as_deref(),
            json!({"fixture": "story"}),
        )
        .unwrap();
        let mut approved = claim_item("Owner approved homepage positioning for readiness review.");
        approved.candidate_state = Some(GeneratedContentMemoryState::Approved);
        approved.visibility = "public".to_string();
        approved.approval_evidence_refs = vec!["approval:owner_1".to_string()];

        let (candidates, _) = ingest_generated_content_memory_candidates(
            &connection,
            GeneratedContentMemoryIngestionInput {
                artifact_id: artifact.id.clone(),
                artifact_version_id: Some(version.id.clone()),
                workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
                workflow_compilation_id: Some("workflow_compilation_readiness_1".to_string()),
                job_id: Some("job_story_readiness_1".to_string()),
                extraction_fixture_id: "fixture.story.promotion_readiness.v1".to_string(),
                items: vec![approved],
            },
        )
        .unwrap();
        let candidate_id = candidates[0].id.clone();
        let candidate_count_before: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM generated_content_memory_candidates",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let graph_node_count_before: i64 = connection
            .query_row("SELECT COUNT(*) FROM graph_nodes", [], |row| row.get(0))
            .unwrap();
        let graph_edge_count_before: i64 = connection
            .query_row("SELECT COUNT(*) FROM graph_edges", [], |row| row.get(0))
            .unwrap();
        let pack_count_before: i64 = connection
            .query_row("SELECT COUNT(*) FROM product_packs", [], |row| row.get(0))
            .unwrap();
        let vector_table_count_before: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND lower(name) LIKE '%vector%'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let packet = generated_content_memory_review_packet_for_artifact(
            &connection,
            &artifact.id,
            GeneratedContentMemoryReviewAudience::Staff,
        )
        .unwrap();

        assert_eq!(packet.promotion_readiness_packets.len(), 1);
        let readiness = &packet.promotion_readiness_packets[0];
        assert_eq!(readiness.candidate_id, candidate_id);
        assert_eq!(readiness.current_candidate_state, "approved");
        assert_eq!(readiness.visibility_class, "public");
        assert!(readiness.read_only);
        assert!(readiness.promotion_ready);
        assert!(readiness.blockers.is_empty());
        assert_eq!(
            readiness.allowed_next_action,
            "prepare_owner_memory_promotion_review"
        );
        assert!(readiness
            .evidence_refs
            .contains(&format!("artifact:{}", artifact.id)));
        assert!(readiness
            .decision_refs
            .contains(&"approval:owner_1".to_string()));
        assert_eq!(
            readiness.origin.workflow_template_ref.as_deref(),
            Some("workflow_template:studio.story.scrollytelling_homepage")
        );
        assert_eq!(
            readiness.origin.workflow_compilation_ref.as_deref(),
            Some("workflow_compilation:workflow_compilation_readiness_1")
        );
        assert_eq!(
            readiness.origin.job_ref.as_deref(),
            Some("job:job_story_readiness_1")
        );
        assert!(!readiness.memory_promotion_performed);
        assert!(!readiness.confirmed_graph_promotion);
        assert!(!readiness.vector_mutation_performed);
        assert!(!readiness.pack_state_mutation_performed);
        assert!(!readiness.live_provider_called);
        assert!(readiness
            .limitations
            .contains(&"memory_promotion_not_performed".to_string()));

        let reloaded = load_generated_content_memory_candidate(&connection, &candidate_id).unwrap();
        assert_eq!(reloaded.candidate_state, "approved");
        assert_eq!(
            candidate_count_before,
            connection
                .query_row(
                    "SELECT COUNT(*) FROM generated_content_memory_candidates",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap()
        );
        assert_eq!(
            graph_node_count_before,
            connection
                .query_row("SELECT COUNT(*) FROM graph_nodes", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap()
        );
        assert_eq!(
            graph_edge_count_before,
            connection
                .query_row("SELECT COUNT(*) FROM graph_edges", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap()
        );
        assert_eq!(
            pack_count_before,
            connection
                .query_row("SELECT COUNT(*) FROM product_packs", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap()
        );
        assert_eq!(
            vector_table_count_before,
            connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND lower(name) LIKE '%vector%'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap()
        );
    }

    #[test]
    fn readiness_packet_blocks_unapproved_rejected_and_private_candidates() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let artifact = generated_artifact(&connection, "sha256:generated-story-readiness-blocked");
        let mut proposed = claim_item("Proposed story claim still requires owner review.");
        proposed.visibility = "public".to_string();
        let mut private_approved =
            claim_item("Owner approved private story detail for staff review only.");
        private_approved.candidate_state = Some(GeneratedContentMemoryState::Approved);
        private_approved.approval_evidence_refs = vec!["approval:owner_private".to_string()];
        private_approved.visibility = "staff".to_string();
        let mut rejected = claim_item("Rejected story claim should stay blocked.");
        rejected.candidate_state = Some(GeneratedContentMemoryState::Rejected);
        rejected.rejection_evidence_refs = vec!["artifact_review:rejected".to_string()];
        rejected.visibility = "public".to_string();

        ingest_generated_content_memory_candidates(
            &connection,
            GeneratedContentMemoryIngestionInput {
                artifact_id: artifact.id.clone(),
                artifact_version_id: None,
                workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
                workflow_compilation_id: Some("workflow_compilation_readiness_blocked".to_string()),
                job_id: Some("job_story_readiness_blocked".to_string()),
                extraction_fixture_id: "fixture.story.promotion_readiness_blocked.v1".to_string(),
                items: vec![proposed, private_approved, rejected],
            },
        )
        .unwrap();

        let packet = generated_content_memory_review_packet_for_artifact(
            &connection,
            &artifact.id,
            GeneratedContentMemoryReviewAudience::Staff,
        )
        .unwrap();

        assert_eq!(packet.promotion_readiness_packets.len(), 3);
        assert!(packet
            .promotion_readiness_packets
            .iter()
            .all(|readiness| !readiness.promotion_ready));
        assert!(packet.promotion_readiness_packets.iter().any(|readiness| {
            readiness
                .blockers
                .contains(&"candidate_state_proposed_blocks_promotion_readiness".to_string())
        }));
        assert!(packet.promotion_readiness_packets.iter().any(|readiness| {
            readiness
                .blockers
                .contains(&"private_visibility_blocks_promotion_readiness".to_string())
        }));
        assert!(packet.promotion_readiness_packets.iter().any(|readiness| {
            readiness
                .blockers
                .contains(&"candidate_state_rejected_blocks_promotion_readiness".to_string())
        }));
        assert!(packet.promotion_readiness_packets.iter().all(|readiness| {
            readiness.allowed_next_action == "resolve_memory_readiness_blockers"
                && !readiness.memory_promotion_performed
                && !readiness.confirmed_graph_promotion
                && !readiness.vector_mutation_performed
                && !readiness.pack_state_mutation_performed
        }));
    }

    #[test]
    fn member_safe_story_memory_review_packet_redacts_private_fields_and_remains_read_only() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let artifact = generated_artifact(&connection, "sha256:generated-story-member-packet");
        let mut approved = claim_item("Owner approved concise homepage positioning.");
        approved.candidate_state = Some(GeneratedContentMemoryState::Approved);
        approved.approval_evidence_refs = vec!["approval:owner_1".to_string()];
        approved.visibility = "public".to_string();
        approved.body = json!({
            "claim": "Owner approved concise homepage positioning.",
            "privateReviewerNote": "Internal note should not appear in member packet."
        });

        ingest_generated_content_memory_candidates(
            &connection,
            GeneratedContentMemoryIngestionInput {
                artifact_id: artifact.id.clone(),
                artifact_version_id: None,
                workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
                workflow_compilation_id: Some("workflow_compilation_member_packet".to_string()),
                job_id: Some("job_member_packet".to_string()),
                extraction_fixture_id: "fixture.story.member_packet.v1".to_string(),
                items: vec![approved],
            },
        )
        .unwrap();
        let before_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM generated_content_memory_candidates",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let packet = generated_content_memory_review_packet_for_artifact(
            &connection,
            &artifact.id,
            GeneratedContentMemoryReviewAudience::Member,
        )
        .unwrap();

        let after_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM generated_content_memory_candidates",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(before_count, after_count);
        assert_eq!(packet.audience, "member");
        assert_eq!(packet.candidate_count, 1);
        assert!(packet
            .limitations
            .contains(&"member_safe_packet_redacts_candidate_bodies".to_string()));
        assert_eq!(
            packet.items[0].summary_text,
            "Generated content memory candidate requires authorized review."
        );
        assert_eq!(packet.items[0].body, json!({}));
        assert_eq!(packet.items[0].body_redacted, true);
        assert!(packet.items[0].evidence_refs.iter().all(|reference| {
            reference.starts_with("artifact:") || reference.starts_with("artifact_version:")
        }));
        let encoded = serde_json::to_string(&packet).unwrap();
        assert!(!encoded.contains("Owner approved concise homepage positioning"));
        assert!(!encoded.contains("Internal note"));
        assert!(!encoded.contains("privateReviewerNote"));
        assert!(!encoded.contains("provider internal"));
        assert!(!encoded.contains("prompt internal"));
        assert!(!encoded.contains("graph certainty"));
        assert_eq!(packet.confirmed_graph_promotion, false);
        assert_eq!(packet.live_provider_called, false);
    }
}
