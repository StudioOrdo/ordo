use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::artifacts::{load_artifact, ArtifactView};
use crate::content_analytics::{summarize_content_analytics_for_content, ContentAnalyticsSummary};
use crate::generated_content_memory::{
    generated_content_memory_review_packet_for_artifact, GeneratedContentMemoryReviewAudience,
    GeneratedContentMemoryReviewPacket,
};
use crate::security::redaction;
use crate::story_image_artifacts::{
    STORY_GENERATED_IMAGE_CANDIDATE_ARTIFACT_KIND, STORY_IMAGE_BRIEF_ARTIFACT_KIND,
    STORY_IMAGE_PROVIDER_REQUEST_ENVELOPE_ARTIFACT_KIND, STORY_IMAGE_REVIEW_ARTIFACT_KIND,
};
use crate::story_intake_artifacts::STORY_FOUNDER_INTAKE_ARTIFACT_KIND;
use crate::story_publish_approvals::STORY_HOMEPAGE_PUBLISH_APPROVAL_PACKAGE_ARTIFACT_KIND;

pub const STORY_PRODUCTION_REVIEW_SCHEMA_VERSION: &str = "ordo.story_production_review_packet.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoryProductionReviewAudience {
    Staff,
    Owner,
    Member,
    Public,
}

impl StoryProductionReviewAudience {
    fn as_str(self) -> &'static str {
        match self {
            Self::Staff => "staff",
            Self::Owner => "owner",
            Self::Member => "member",
            Self::Public => "public",
        }
    }

    fn can_read_staff_packet(self) -> bool {
        matches!(self, Self::Staff | Self::Owner)
    }

    fn memory_audience(self) -> GeneratedContentMemoryReviewAudience {
        match self {
            Self::Staff => GeneratedContentMemoryReviewAudience::Staff,
            Self::Owner => GeneratedContentMemoryReviewAudience::Owner,
            Self::Member => GeneratedContentMemoryReviewAudience::Member,
            Self::Public => GeneratedContentMemoryReviewAudience::Public,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StoryProductionReviewPacketRequest {
    pub audience: StoryProductionReviewAudience,
    pub artifact_ids: Vec<String>,
    pub deck_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryProductionReviewPacket {
    pub schema_version: String,
    pub status: String,
    pub audience: String,
    pub read_only: bool,
    pub mutation_performed: bool,
    pub confirmed_graph_promotion: bool,
    pub live_provider_called: bool,
    pub external_publishing_claimed: bool,
    pub deck_id: Option<String>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub missing_prerequisites: Vec<String>,
    pub recommended_next_actions: Vec<String>,
    pub components: Vec<StoryProductionReviewComponent>,
    pub analytics_summary: Option<ContentAnalyticsSummary>,
    pub memory_review_packets: Vec<GeneratedContentMemoryReviewPacket>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryProductionReviewComponent {
    pub key: String,
    pub status: String,
    pub artifact_ref: Option<String>,
    pub artifact_kind: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub visibility: String,
    pub evidence_status: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub recommended_next_action: String,
}

pub fn story_production_review_packet(
    connection: &Connection,
    request: StoryProductionReviewPacketRequest,
) -> Result<StoryProductionReviewPacket> {
    let mut components = Vec::new();
    let mut evidence_refs = Vec::new();
    let mut limitations = vec![
        "story_production_review_is_read_only".to_string(),
        "canonical_tables_remain_truth".to_string(),
        "events_remain_audit_replay".to_string(),
        "graph_and_memory_candidates_are_not_promoted".to_string(),
    ];
    let mut deck_id = request.deck_id.map(|value| safe_identifier(&value));
    let mut memory_review_packets = Vec::new();

    for artifact_id in request.artifact_ids {
        match load_artifact(connection, &artifact_id) {
            Ok(artifact) => {
                if !request.audience.can_read_staff_packet()
                    && !is_member_visible_artifact(&artifact)
                {
                    components.push(restricted_component(&artifact, request.audience));
                    append_unique(
                        &mut limitations,
                        &["member_safe_packet_omits_staff_artifact_details".to_string()],
                    );
                    continue;
                }

                if deck_id.is_none() {
                    deck_id = deck_id_from_artifact(&artifact);
                }
                append_unique(&mut evidence_refs, &artifact.evidence_refs);
                components.push(component_for_artifact(&artifact, request.audience));

                if request.audience.can_read_staff_packet() {
                    let memory_packet = generated_content_memory_review_packet_for_artifact(
                        connection,
                        &artifact.id,
                        request.audience.memory_audience(),
                    )?;
                    if memory_packet.candidate_count > 0 {
                        append_unique(&mut evidence_refs, &memory_packet.evidence_refs);
                        append_unique(&mut limitations, &memory_packet.limitations);
                        memory_review_packets.push(memory_packet);
                    }
                }
            }
            Err(_) => {
                components.push(StoryProductionReviewComponent {
                    key: "missing_artifact".to_string(),
                    status: "missing".to_string(),
                    artifact_ref: None,
                    artifact_kind: None,
                    title: None,
                    summary: None,
                    visibility: request.audience.as_str().to_string(),
                    evidence_status: "missing".to_string(),
                    evidence_refs: Vec::new(),
                    limitations: vec!["requested_artifact_not_available".to_string()],
                    recommended_next_action: "supply_known_story_artifact_ref".to_string(),
                });
            }
        }
    }

    let analytics_summary = if request.audience.can_read_staff_packet() {
        deck_id
            .as_deref()
            .map(|deck_id| {
                summarize_content_analytics_for_content(connection, "homepage_story_deck", deck_id)
            })
            .transpose()?
    } else {
        None
    };
    if let Some(summary) = analytics_summary.as_ref() {
        append_unique(&mut evidence_refs, &summary.evidence_refs);
        append_unique(
            &mut limitations,
            &summary
                .limitations
                .iter()
                .map(|limitation| limitation.key.clone())
                .collect::<Vec<_>>(),
        );
    }

    let missing_prerequisites = missing_prerequisites(
        &components,
        analytics_summary.as_ref(),
        &memory_review_packets,
    );
    let recommended_next_actions = recommended_next_actions(&missing_prerequisites);
    let status = if missing_prerequisites.is_empty() {
        "complete"
    } else {
        "partial"
    };
    if !request.audience.can_read_staff_packet() {
        append_unique(
            &mut limitations,
            &[
                "member_public_packet_is_availability_only".to_string(),
                "generated_content_candidate_text_is_redacted".to_string(),
                "private_artifact_text_is_not_returned".to_string(),
            ],
        );
    }

    Ok(StoryProductionReviewPacket {
        schema_version: STORY_PRODUCTION_REVIEW_SCHEMA_VERSION.to_string(),
        status: status.to_string(),
        audience: request.audience.as_str().to_string(),
        read_only: true,
        mutation_performed: false,
        confirmed_graph_promotion: false,
        live_provider_called: false,
        external_publishing_claimed: false,
        deck_id,
        evidence_refs: sorted_unique(evidence_refs),
        limitations: sorted_unique(limitations),
        missing_prerequisites,
        recommended_next_actions,
        components,
        analytics_summary,
        memory_review_packets,
    })
}

fn component_for_artifact(
    artifact: &ArtifactView,
    audience: StoryProductionReviewAudience,
) -> StoryProductionReviewComponent {
    let key = component_key(&artifact.artifact_kind);
    let evidence_status = evidence_status_for_artifact(artifact);
    let mut limitations = limitations_for_artifact(artifact);
    let summary = safe_component_summary(artifact, audience, &mut limitations);
    StoryProductionReviewComponent {
        key: key.to_string(),
        status: artifact.status.clone(),
        artifact_ref: Some(format!("artifact:{}", artifact.id)),
        artifact_kind: Some(artifact.artifact_kind.clone()),
        title: safe_component_title(artifact, audience),
        summary,
        visibility: if audience.can_read_staff_packet() {
            artifact.visibility_ceiling.clone()
        } else {
            audience.as_str().to_string()
        },
        evidence_status: evidence_status.to_string(),
        evidence_refs: audience_safe_refs(audience, &artifact.evidence_refs),
        limitations,
        recommended_next_action: recommended_action_for_artifact(artifact).to_string(),
    }
}

fn restricted_component(
    artifact: &ArtifactView,
    audience: StoryProductionReviewAudience,
) -> StoryProductionReviewComponent {
    StoryProductionReviewComponent {
        key: component_key(&artifact.artifact_kind).to_string(),
        status: "not_available".to_string(),
        artifact_ref: None,
        artifact_kind: None,
        title: None,
        summary: None,
        visibility: audience.as_str().to_string(),
        evidence_status: "restricted".to_string(),
        evidence_refs: Vec::new(),
        limitations: vec![
            "artifact_details_require_staff_or_owner_audience".to_string(),
            "private_artifact_text_is_not_returned".to_string(),
        ],
        recommended_next_action: "request_authorized_staff_review".to_string(),
    }
}

fn component_key(artifact_kind: &str) -> &'static str {
    match artifact_kind {
        STORY_FOUNDER_INTAKE_ARTIFACT_KIND => "intake",
        STORY_IMAGE_BRIEF_ARTIFACT_KIND => "image_brief",
        STORY_IMAGE_PROVIDER_REQUEST_ENVELOPE_ARTIFACT_KIND => "image_generation",
        STORY_GENERATED_IMAGE_CANDIDATE_ARTIFACT_KIND => "generated_image_candidate",
        STORY_IMAGE_REVIEW_ARTIFACT_KIND => "image_review",
        STORY_HOMEPAGE_PUBLISH_APPROVAL_PACKAGE_ARTIFACT_KIND => "publish_approval",
        "story.narrative_deck" | "narrative_deck" => "narrative_deck",
        "story.homepage_version" => "homepage_draft",
        _ => "story_artifact",
    }
}

fn evidence_status_for_artifact(artifact: &ArtifactView) -> &'static str {
    if artifact.artifact_kind == STORY_HOMEPAGE_PUBLISH_APPROVAL_PACKAGE_ARTIFACT_KIND {
        return "manual";
    }
    if artifact.artifact_kind == STORY_IMAGE_REVIEW_ARTIFACT_KIND
        || artifact.artifact_kind == STORY_IMAGE_PROVIDER_REQUEST_ENVELOPE_ARTIFACT_KIND
        || artifact.artifact_kind == STORY_GENERATED_IMAGE_CANDIDATE_ARTIFACT_KIND
    {
        return "fixture";
    }
    if artifact
        .evidence_refs
        .iter()
        .any(|value| value.starts_with("content_analytics_event:"))
    {
        return "measured";
    }
    if artifact.evidence_refs.is_empty() {
        "missing"
    } else {
        "manual"
    }
}

fn safe_component_title(
    artifact: &ArtifactView,
    audience: StoryProductionReviewAudience,
) -> Option<String> {
    if !audience.can_read_staff_packet() {
        return None;
    }
    Some(safe_text_or_withheld(&artifact.title))
}

fn safe_component_summary(
    artifact: &ArtifactView,
    audience: StoryProductionReviewAudience,
    limitations: &mut Vec<String>,
) -> Option<String> {
    if !audience.can_read_staff_packet() {
        limitations.push("summary_redacted_for_member_public_audience".to_string());
        return None;
    }
    let safe = safe_text_or_withheld(&artifact.summary);
    if safe == "Artifact summary withheld by production review packet." {
        limitations.push("artifact_summary_withheld_for_safety".to_string());
    }
    Some(safe)
}

fn limitations_for_artifact(artifact: &ArtifactView) -> Vec<String> {
    let mut limitations = Vec::new();
    if let Some(health) = artifact.health_status.as_deref() {
        limitations.push(format!("health_status:{}", safe_identifier(health)));
    }
    if artifact.artifact_kind == STORY_HOMEPAGE_PUBLISH_APPROVAL_PACKAGE_ARTIFACT_KIND {
        limitations.push("manual_local_publish_evidence_only".to_string());
        limitations.push("external_publishing_not_claimed".to_string());
    }
    if artifact.artifact_kind == STORY_IMAGE_PROVIDER_REQUEST_ENVELOPE_ARTIFACT_KIND {
        limitations.push("deterministic_fixture_provider_request_only".to_string());
    }
    if artifact.artifact_kind == STORY_IMAGE_REVIEW_ARTIFACT_KIND {
        limitations.push("image_review_is_publication_evidence_only".to_string());
    }
    sorted_unique(limitations)
}

fn recommended_action_for_artifact(artifact: &ArtifactView) -> &'static str {
    match component_key(&artifact.artifact_kind) {
        "intake" => "review_public_derivative_and_claim_evidence",
        "narrative_deck" => "review_deck_readiness_and_claim_support",
        "image_brief" => "generate_or_review_image_candidates",
        "image_generation" => "review_generated_image_candidates",
        "generated_image_candidate" => "record_image_review_before_public_derivative",
        "image_review" => "prepare_public_derivative_or_request_revision",
        "publish_approval" => "confirm_manual_publish_evidence_and_review_analytics",
        "homepage_draft" => "request_publish_approval_after_review",
        _ => "inspect_story_artifact_evidence",
    }
}

fn is_member_visible_artifact(artifact: &ArtifactView) -> bool {
    matches!(
        artifact.visibility_ceiling.as_str(),
        "public" | "authenticated"
    )
}

fn deck_id_from_artifact(artifact: &ArtifactView) -> Option<String> {
    artifact
        .provenance
        .pointer("/contract/deckId")
        .and_then(Value::as_str)
        .or_else(|| {
            artifact
                .provenance
                .pointer("/publicDerivative/deckId")
                .and_then(Value::as_str)
        })
        .or_else(|| {
            artifact
                .provenance
                .pointer("/contract/publicDerivative/deckId")
                .and_then(Value::as_str)
        })
        .map(safe_identifier)
}

fn missing_prerequisites(
    components: &[StoryProductionReviewComponent],
    analytics_summary: Option<&ContentAnalyticsSummary>,
    memory_packets: &[GeneratedContentMemoryReviewPacket],
) -> Vec<String> {
    let has_component = |key: &str| {
        components
            .iter()
            .any(|component| component.key == key && component.status != "not_available")
    };
    let mut missing = Vec::new();
    for (key, label) in [
        ("intake", "story_founder_intake"),
        ("narrative_deck", "narrative_deck"),
        ("image_brief", "image_briefs"),
        ("generated_image_candidate", "generated_image_candidates"),
        ("image_review", "image_reviews"),
        ("publish_approval", "publish_approval_package"),
    ] {
        if !has_component(key) {
            missing.push(label.to_string());
        }
    }
    if analytics_summary
        .map(|summary| summary.event_counts.is_empty())
        .unwrap_or(true)
    {
        missing.push("content_analytics_summary".to_string());
    }
    if memory_packets.is_empty() {
        missing.push("generated_content_memory_review_packet".to_string());
    }
    sorted_unique(missing)
}

fn recommended_next_actions(missing: &[String]) -> Vec<String> {
    if missing.is_empty() {
        return vec!["review_packet_ready_for_owner_decision".to_string()];
    }
    missing
        .iter()
        .map(|item| match item.as_str() {
            "story_founder_intake" => "capture_or_attach_story_founder_intake",
            "narrative_deck" => "compile_or_attach_narrative_deck",
            "image_briefs" => "prepare_story_image_briefs",
            "generated_image_candidates" => "record_deterministic_image_generation_envelopes",
            "image_reviews" => "record_story_image_reviews",
            "publish_approval_package" => "record_homepage_publish_approval_package",
            "content_analytics_summary" => "record_or_link_content_analytics_evidence",
            "generated_content_memory_review_packet" => {
                "propose_generated_content_memory_candidates_for_review"
            }
            _ => "inspect_missing_story_production_evidence",
        })
        .map(str::to_string)
        .collect()
}

fn audience_safe_refs(audience: StoryProductionReviewAudience, refs: &[String]) -> Vec<String> {
    if audience.can_read_staff_packet() {
        return sorted_unique(refs.iter().map(|value| safe_identifier(value)).collect());
    }
    sorted_unique(
        refs.iter()
            .filter(|value| {
                value.starts_with("artifact:")
                    || value.starts_with("artifact_version:")
                    || value.starts_with("homepage_story_deck:")
            })
            .map(|value| safe_identifier(value))
            .collect(),
    )
}

fn safe_text_or_withheld(value: &str) -> String {
    let redacted = redaction::redact_public_text(value.trim());
    if redacted.is_empty() || contains_private_marker(&redacted) {
        "Artifact summary withheld by production review packet.".to_string()
    } else {
        redacted
    }
}

fn contains_private_marker(value: &str) -> bool {
    let normalized = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    [
        "staffrouting",
        "providerinternal",
        "providersecret",
        "promptinternal",
        "rawpolicy",
        "policyinternal",
        "owneronly",
        "privateartifacttext",
        "compiledplanprivateinput",
        "taskprivatepayload",
        "graphcertainty",
        "unsupportedclaim",
        "generatedcontentcandidatetext",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn append_unique(values: &mut Vec<String>, additions: &[String]) {
    for value in additions {
        if !value.trim().is_empty() && !values.contains(value) {
            values.push(value.clone());
        }
    }
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    values.sort();
    values.dedup();
    values
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::{record_artifact, ArtifactInput};
    use crate::generated_content_memory::{
        ingest_generated_content_memory_candidates, GeneratedContentMemoryIngestionInput,
        GeneratedContentMemoryItemInput, GeneratedContentMemoryKind, GeneratedContentMemoryState,
    };
    use crate::public_surfaces::{
        HomepageNarrativeDeck, HomepageNarrativeSlide, HomepageStoryCopySlot, HomepageStoryCta,
        HomepageStoryDeckResponse, HomepageStoryProfile, HomepageStoryRefreshContract,
        PublicSurfaceReadiness,
    };
    use crate::schema::init_schema;
    use crate::story_image_artifacts::{
        prepare_story_image_brief_artifacts, record_story_image_provider_request_envelope,
        record_story_image_review_artifact, StoryImageProviderRequestInput, StoryImageReviewInput,
        StoryImageReviewState,
    };
    use crate::story_intake_artifacts::{
        record_story_founder_intake_artifact, StoryFounderIntakeInput, StoryIntakeClaimInput,
    };
    use crate::story_publish_approvals::{
        record_homepage_publish_approval_package, HomepagePublishApprovalInput,
    };
    use serde_json::json;

    #[test]
    fn complete_staff_packet_aggregates_story_production_state_without_mutation() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let fixture = complete_fixture(&connection);
        let before_artifacts = count(&connection, "artifacts");
        let before_memory = count(&connection, "generated_content_memory_candidates");

        let packet = story_production_review_packet(
            &connection,
            StoryProductionReviewPacketRequest {
                audience: StoryProductionReviewAudience::Staff,
                artifact_ids: fixture.artifact_ids,
                deck_id: Some("homepage.story.v1".to_string()),
            },
        )
        .unwrap();

        assert_eq!(packet.status, "complete");
        assert_eq!(packet.audience, "staff");
        assert_eq!(packet.read_only, true);
        assert_eq!(packet.mutation_performed, false);
        assert_eq!(packet.confirmed_graph_promotion, false);
        assert_eq!(packet.live_provider_called, false);
        assert_eq!(packet.external_publishing_claimed, false);
        assert!(packet.missing_prerequisites.is_empty());
        assert!(packet
            .recommended_next_actions
            .contains(&"review_packet_ready_for_owner_decision".to_string()));
        for key in [
            "intake",
            "narrative_deck",
            "image_brief",
            "image_generation",
            "generated_image_candidate",
            "image_review",
            "publish_approval",
            "homepage_draft",
        ] {
            assert!(packet
                .components
                .iter()
                .any(|component| component.key == key));
        }
        assert!(packet
            .components
            .iter()
            .any(|component| component.evidence_status == "fixture"));
        assert!(packet
            .components
            .iter()
            .any(|component| component.evidence_status == "manual"));
        assert!(packet.analytics_summary.is_some());
        assert!(!packet
            .analytics_summary
            .as_ref()
            .unwrap()
            .event_counts
            .is_empty());
        assert_eq!(packet.memory_review_packets.len(), 1);
        assert_eq!(count(&connection, "artifacts"), before_artifacts);
        assert_eq!(
            count(&connection, "generated_content_memory_candidates"),
            before_memory
        );
    }

    #[test]
    fn partial_packet_reports_missing_prerequisites_and_unknown_artifacts_safely() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let intake = record_intake(&connection);

        let packet = story_production_review_packet(
            &connection,
            StoryProductionReviewPacketRequest {
                audience: StoryProductionReviewAudience::Owner,
                artifact_ids: vec![
                    intake.artifact.id,
                    "artifact_does_not_exist_provider_internal_secret".to_string(),
                ],
                deck_id: None,
            },
        )
        .unwrap();

        assert_eq!(packet.status, "partial");
        assert!(packet
            .missing_prerequisites
            .contains(&"narrative_deck".to_string()));
        assert!(packet
            .missing_prerequisites
            .contains(&"image_reviews".to_string()));
        assert!(packet
            .components
            .iter()
            .any(|component| component.key == "missing_artifact" && component.status == "missing"));
        let encoded = serde_json::to_string(&packet).unwrap();
        assert!(!encoded.contains("provider internal"));
        assert!(!encoded.contains("secret"));
    }

    #[test]
    fn member_safe_packet_redacts_staff_artifacts_and_memory_candidate_text() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let fixture = complete_fixture(&connection);

        let packet = story_production_review_packet(
            &connection,
            StoryProductionReviewPacketRequest {
                audience: StoryProductionReviewAudience::Member,
                artifact_ids: fixture.artifact_ids,
                deck_id: Some("homepage.story.v1".to_string()),
            },
        )
        .unwrap();

        assert_eq!(packet.audience, "member");
        assert!(packet
            .components
            .iter()
            .any(|component| component.status == "not_available"));
        assert!(
            packet
                .components
                .iter()
                .filter(|component| component.status == "not_available")
                .all(|component| component.artifact_ref.is_none()
                    && component.artifact_kind.is_none())
        );
        assert!(packet.analytics_summary.is_none());
        assert!(packet.memory_review_packets.is_empty());
        assert!(packet
            .limitations
            .contains(&"generated_content_candidate_text_is_redacted".to_string()));
        let encoded = serde_json::to_string(&packet).unwrap();
        assert!(!encoded.contains("Generated story claim draft for owner review"));
        assert!(!encoded.contains("Sensitive reviewer note should not leak"));
        assert!(!encoded.contains("prompt internal"));
        assert!(!encoded.contains("provider internal"));
        assert!(!encoded.contains("staff routing"));
    }

    struct CompleteFixture {
        artifact_ids: Vec<String>,
    }

    fn complete_fixture(connection: &Connection) -> CompleteFixture {
        let deck = deck();
        let intake = record_intake(connection);
        let deck_artifact = record_story_artifact(
            connection,
            "story.narrative_deck",
            "Narrative deck",
            "Ready public narrative deck.",
            json!({
                "contract": {
                    "deckId": "homepage.story.v1",
                    "version": 1
                }
            }),
        );
        let brief = prepare_story_image_brief_artifacts(connection, &deck)
            .unwrap()
            .remove(0);
        let generation = record_story_image_provider_request_envelope(
            connection,
            StoryImageProviderRequestInput {
                brief_artifact_id: brief.artifact.id.clone(),
                idempotency_key: "production-review-image".to_string(),
                provider_name: "openai".to_string(),
                model_hint: "gpt-image-2".to_string(),
                provider_mode: "deterministic_fixture".to_string(),
                requested_size: "1024x576".to_string(),
                requested_aspect_ratio: "16:9".to_string(),
                requested_count: 1,
                fixture_status: "generated".to_string(),
            },
        )
        .unwrap();
        let candidate = generation.candidates.first().unwrap().clone();
        let review = record_story_image_review_artifact(
            connection,
            StoryImageReviewInput {
                candidate_artifact_id: candidate.artifact.id.clone(),
                brief_artifact_id: brief.artifact.id.clone(),
                idempotency_key: "production-review-image-review".to_string(),
                fixture_id: "fixture:image.reviewAgainstBrief:approved".to_string(),
                review_state: StoryImageReviewState::Approved,
                reviewer_ref: "fixture:image-reviewer".to_string(),
                evidence_refs: vec!["review_fixture:approved".to_string()],
                limitations: vec![],
                revision_guidance: None,
                visibility: "staff".to_string(),
            },
        )
        .unwrap();
        let homepage_draft = record_story_artifact(
            connection,
            "story.homepage_version",
            "Homepage version",
            "Draft homepage version for review.",
            json!({
                "publicDerivative": {
                    "deckId": "homepage.story.v1"
                }
            }),
        );
        let publish = record_homepage_publish_approval_package(
            connection,
            HomepagePublishApprovalInput {
                package_id: "homepage-v1".to_string(),
                idempotency_key: "homepage-production-review".to_string(),
                deck,
                source_artifact_ids: vec![intake.artifact.id.clone(), homepage_draft.id.clone()],
                image_artifact_ids: vec![review.artifact.id.clone()],
                approval_state: "approved".to_string(),
                approval_actor_id: "owner_1".to_string(),
                approval_evidence_refs: vec!["approval:owner_1".to_string()],
                manual_publish_evidence_refs: vec!["manual_publish:homepage_v1".to_string()],
                limitations: vec!["Manual local publish only.".to_string()],
                workflow_compilation_id: Some("workflow_compilation_story_v1".to_string()),
                job_id: None,
                occurred_at: Some("2026-05-14T22:00:00Z".to_string()),
            },
        )
        .unwrap();
        ingest_generated_content_memory_candidates(
            connection,
            GeneratedContentMemoryIngestionInput {
                artifact_id: homepage_draft.id.clone(),
                artifact_version_id: None,
                workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
                workflow_compilation_id: Some("workflow_compilation_story_v1".to_string()),
                job_id: None,
                extraction_fixture_id: "fixture.story.production_review.v1".to_string(),
                items: vec![GeneratedContentMemoryItemInput {
                    memory_kind: GeneratedContentMemoryKind::CandidateClaim,
                    candidate_state: Some(GeneratedContentMemoryState::Proposed),
                    summary_text: "Generated story claim draft for owner review.".to_string(),
                    body: json!({
                        "claim": "Generated story claim draft for owner review.",
                        "reviewerNote": "Sensitive reviewer note should not leak"
                    }),
                    confidence: 0.72,
                    evidence_refs: vec!["artifact:homepage_version".to_string()],
                    limitations: vec!["Needs owner review.".to_string()],
                    visibility: "staff".to_string(),
                    approval_evidence_refs: vec![],
                    publication_evidence_refs: vec![],
                    feedback_evidence_refs: vec![],
                    outcome_evidence_refs: vec![],
                    rejection_evidence_refs: vec![],
                }],
            },
        )
        .unwrap();

        CompleteFixture {
            artifact_ids: vec![
                intake.artifact.id,
                deck_artifact.id,
                brief.artifact.id,
                generation.envelope_artifact.id,
                candidate.artifact.id,
                review.artifact.id,
                homepage_draft.id,
                publish.package_artifact.id,
            ],
        }
    }

    fn record_intake(
        connection: &Connection,
    ) -> crate::story_intake_artifacts::StoryFounderIntakeArtifact {
        record_story_founder_intake_artifact(
            connection,
            StoryFounderIntakeInput {
                intake_id: "keith-v1".to_string(),
                founder_story: "Keith is building Studio Ordo in public.".to_string(),
                business_stance: "Ordo is a practical answer to enshittification.".to_string(),
                audience: Some("Solopreneurs".to_string()),
                public_claims: vec![StoryIntakeClaimInput {
                    claim: "Ordo keeps story work grounded in local evidence.".to_string(),
                    evidence_refs: vec!["business_fact:homepage.positioning".to_string()],
                }],
                proof_evidence_refs: vec!["business_fact:homepage.positioning".to_string()],
                private_notes: vec!["owner-only private note".to_string()],
                style_preferences: vec!["cinematic editorial".to_string()],
                offer_refs: vec!["offer:trial".to_string()],
                cta_refs: vec!["cta:talk-with-ordo".to_string()],
                limitations: vec!["Requires owner review before publish.".to_string()],
                source_kind: Some("manual_owner_intake".to_string()),
                source_id: Some("owner_keith".to_string()),
                created_by_job_id: None,
            },
        )
        .unwrap()
    }

    fn record_story_artifact(
        connection: &Connection,
        artifact_kind: &str,
        title: &str,
        summary: &str,
        provenance: Value,
    ) -> ArtifactView {
        record_artifact(
            connection,
            ArtifactInput {
                artifact_kind: artifact_kind.to_string(),
                title: title.to_string(),
                status: "ready".to_string(),
                visibility_ceiling: "staff".to_string(),
                summary: summary.to_string(),
                source_kind: Some("story_production_review_fixture".to_string()),
                source_id: Some(artifact_kind.to_string()),
                evidence_refs: vec![format!("artifact_kind:{artifact_kind}")],
                provenance,
                content_hash: format!("sha256:{}", safe_identifier(artifact_kind)),
                storage_uri: None,
                health_status: Some("available".to_string()),
                created_by_job_id: None,
            },
        )
        .unwrap()
        .0
    }

    fn deck() -> HomepageStoryDeckResponse {
        HomepageStoryDeckResponse {
            profile: HomepageStoryProfile {
                positioning: "Studio Ordo is a local-first operating appliance.".to_string(),
                audience: Some("business owners".to_string()),
                primary_cta: Some(HomepageStoryCta {
                    label: "Start trial".to_string(),
                    href: "/offers/nyc-pilot".to_string(),
                    evidence_refs: vec!["business_fact:cta".to_string()],
                }),
                evidence_refs: vec!["business_fact:profile".to_string()],
                limitations: vec![],
            },
            deck: HomepageNarrativeDeck {
                deck_id: "homepage.story.v1".to_string(),
                version: 1,
                surface: "homepage".to_string(),
                slides: vec![HomepageNarrativeSlide {
                    slide_id: "identity".to_string(),
                    section_id: "identity".to_string(),
                    order: 1,
                    title: "A practical answer to enshittification".to_string(),
                    body: "Ordo keeps the owner in control of evidence-backed work.".to_string(),
                    copy_slots: vec![HomepageStoryCopySlot {
                        slot: "sourceLine".to_string(),
                        value: json!("Published public homepage profile"),
                    }],
                    cta_refs: vec![],
                    evidence_refs: vec!["business_fact:slide.identity".to_string()],
                    limitations: vec![],
                    motion_profile: "cinematic".to_string(),
                    reduced_motion_fallback: "Owner-controlled local-first work.".to_string(),
                    image_brief_method: Some("homepage.prepare_image_briefs".to_string()),
                }],
                evidence_refs: vec!["business_fact:deck".to_string()],
                limitations: vec![],
            },
            readiness: PublicSurfaceReadiness {
                surface: "homepage.story".to_string(),
                ready: true,
                fact_count: 3,
                missing: Vec::new(),
            },
            refresh: HomepageStoryRefreshContract {
                manual_refresh_supported: true,
                scheduled_refresh_supported: false,
                image_brief_method: "homepage.prepare_image_briefs".to_string(),
                live_provider_required: false,
                limitations: vec![],
            },
        }
    }

    fn count(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }
}
