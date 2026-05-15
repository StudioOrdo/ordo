use anyhow::{bail, ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::artifacts::{
    add_artifact_version, load_artifact, record_artifact, ArtifactInput, ArtifactVersionView,
    ArtifactView,
};
use crate::public_surfaces::{HomepageNarrativeSlide, HomepageStoryDeckResponse};
use crate::security::redaction;

pub const STORY_IMAGE_BRIEF_ARTIFACT_KIND: &str = "story.image_brief";
pub const STORY_IMAGE_PROVIDER_REQUEST_ENVELOPE_ARTIFACT_KIND: &str =
    "story.image_provider_request_envelope";
pub const STORY_GENERATED_IMAGE_CANDIDATE_ARTIFACT_KIND: &str = "story.generated_image_candidate";
pub const STORY_IMAGE_REVIEW_ARTIFACT_KIND: &str = "story.image_review";
const CONTRACT_SCHEMA_VERSION: &str = "ordo.story_image_artifact_contract.v1";
const DEFAULT_ASPECT_RATIO: &str = "16:9";
const DEFAULT_IMAGE_SIZE: &str = "1024x576";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedImageCandidateState {
    Draft,
    Requested,
    Generated,
    Failed,
    Approved,
    Rejected,
}

impl GeneratedImageCandidateState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Requested => "requested",
            Self::Generated => "generated",
            Self::Failed => "failed",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoryImageReviewState {
    Approved,
    NeedsRevision,
    Rejected,
}

impl StoryImageReviewState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::NeedsRevision => "needs_revision",
            Self::Rejected => "rejected",
        }
    }

    fn health_status(&self) -> &'static str {
        match self {
            Self::Approved => "review_approved",
            Self::NeedsRevision => "review_needs_revision",
            Self::Rejected => "review_rejected",
        }
    }

    fn publication_effect(&self) -> &'static str {
        match self {
            Self::Approved => "eligible_for_public_derivative",
            Self::NeedsRevision | Self::Rejected => "not_publishable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryImageProviderPromptPayload {
    pub method: String,
    pub prompt: String,
    pub aspect_ratio: String,
    pub usage: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryImageBriefContract {
    pub schema_version: String,
    pub brief_id: String,
    pub deck_id: String,
    pub slide_id: String,
    pub section_id: String,
    pub scene_intent: String,
    pub visual_direction: String,
    pub aspect_ratio: String,
    pub usage: String,
    pub visibility: String,
    pub approval_state: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub provider_payload: StoryImageProviderPromptPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedImageCandidateContract {
    pub schema_version: String,
    pub candidate_id: String,
    pub brief_artifact_id: String,
    pub state: GeneratedImageCandidateState,
    pub provider_status: String,
    pub storage_uri: Option<String>,
    pub visibility: String,
    pub approval_state: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryImageProviderRequestEnvelope {
    pub schema_version: String,
    pub request_id: String,
    pub method: String,
    pub provider_name: String,
    pub model_hint: String,
    pub provider_mode: String,
    pub requested_size: String,
    pub requested_aspect_ratio: String,
    pub requested_count: i64,
    pub fixture_status: String,
    pub prompt_payload_ref: String,
    pub idempotency_key: String,
    pub source_artifact_refs: Vec<String>,
    pub visibility: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryImageProviderResponseEnvelope {
    pub schema_version: String,
    pub request_id: String,
    pub status: String,
    pub provider_status: String,
    pub candidate_artifact_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub live_provider_called: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryImageReviewContract {
    pub schema_version: String,
    pub review_id: String,
    pub method: String,
    pub candidate_artifact_id: String,
    pub brief_artifact_id: String,
    pub source_candidate_id: String,
    pub review_state: StoryImageReviewState,
    pub reviewer_ref: String,
    pub fixture_id: String,
    pub visibility: String,
    pub revision_guidance: Option<String>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub publication_effect: String,
    pub memory_effect: String,
    pub live_provider_called: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryImageProviderRequestOutcome {
    pub envelope_artifact: ArtifactView,
    pub request: StoryImageProviderRequestEnvelope,
    pub response: StoryImageProviderResponseEnvelope,
    pub candidates: Vec<GeneratedImageCandidateArtifact>,
}

#[derive(Debug, Clone)]
pub struct StoryImageProviderRequestInput {
    pub brief_artifact_id: String,
    pub idempotency_key: String,
    pub provider_name: String,
    pub model_hint: String,
    pub provider_mode: String,
    pub requested_size: String,
    pub requested_aspect_ratio: String,
    pub requested_count: i64,
    pub fixture_status: String,
}

#[derive(Debug, Clone)]
pub struct StoryImageReviewInput {
    pub candidate_artifact_id: String,
    pub brief_artifact_id: String,
    pub idempotency_key: String,
    pub fixture_id: String,
    pub review_state: StoryImageReviewState,
    pub reviewer_ref: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub revision_guidance: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryImageBriefArtifact {
    pub artifact: ArtifactView,
    pub version: Option<ArtifactVersionView>,
    pub contract: StoryImageBriefContract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedImageCandidateArtifact {
    pub artifact: ArtifactView,
    pub version: Option<ArtifactVersionView>,
    pub contract: GeneratedImageCandidateContract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryImageReviewArtifact {
    pub artifact: ArtifactView,
    pub version: Option<ArtifactVersionView>,
    pub contract: StoryImageReviewContract,
}

#[derive(Debug, Clone)]
pub struct GeneratedImageCandidateInput {
    pub brief_artifact_id: String,
    pub candidate_id: String,
    pub state: GeneratedImageCandidateState,
    pub provider_status: String,
    pub storage_uri: Option<String>,
    pub visibility: String,
    pub approval_state: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
}

pub fn prepare_story_image_brief_artifacts(
    connection: &Connection,
    deck: &HomepageStoryDeckResponse,
) -> Result<Vec<StoryImageBriefArtifact>> {
    deck.deck
        .slides
        .iter()
        .map(|slide| prepare_story_image_brief_artifact(connection, deck, slide))
        .collect()
}

pub fn prepare_story_image_brief_artifact(
    connection: &Connection,
    deck: &HomepageStoryDeckResponse,
    slide: &HomepageNarrativeSlide,
) -> Result<StoryImageBriefArtifact> {
    let contract = story_image_brief_contract(deck, slide)?;
    let contract_json = serde_json::to_value(&contract)?;
    let content_hash = stable_json_hash(&contract_json)?;

    if let Some(existing) = load_existing_story_image_artifact(
        connection,
        STORY_IMAGE_BRIEF_ARTIFACT_KIND,
        "homepage_story_slide",
        &slide.slide_id,
        &content_hash,
    )? {
        return Ok(StoryImageBriefArtifact {
            artifact: existing,
            version: None,
            contract,
        });
    }

    let (artifact, _) = record_artifact(
        connection,
        ArtifactInput {
            artifact_kind: STORY_IMAGE_BRIEF_ARTIFACT_KIND.to_string(),
            title: format!("Image brief for {}", safe_provider_text(&slide.title)),
            status: "draft".to_string(),
            visibility_ceiling: "staff".to_string(),
            summary: format!(
                "Provider-safe image brief for homepage section `{}`.",
                safe_identifier(&slide.section_id)
            ),
            source_kind: Some("homepage_story_slide".to_string()),
            source_id: Some(slide.slide_id.clone()),
            evidence_refs: contract.evidence_refs.clone(),
            provenance: json!({
                "schemaVersion": CONTRACT_SCHEMA_VERSION,
                "generatedBy": "homepage.prepare_image_briefs",
                "deckId": deck.deck.deck_id,
                "slideId": slide.slide_id,
                "contract": contract_json,
            }),
            content_hash: content_hash.clone(),
            storage_uri: Some(format!(
                "ordo://artifacts/story-image-briefs/{}/{}",
                safe_identifier(&deck.deck.deck_id),
                safe_identifier(&slide.slide_id)
            )),
            health_status: Some("contract_only".to_string()),
            created_by_job_id: None,
        },
    )?;
    let version = add_artifact_version(
        connection,
        &artifact.id,
        &content_hash,
        artifact.storage_uri.as_deref(),
        json!({
            "schemaVersion": CONTRACT_SCHEMA_VERSION,
            "contract": contract,
            "providerPayload": contract.provider_payload,
            "liveProviderCalled": false,
        }),
    )?;

    Ok(StoryImageBriefArtifact {
        artifact,
        version: Some(version),
        contract,
    })
}

pub fn record_generated_image_candidate_artifact(
    connection: &Connection,
    input: GeneratedImageCandidateInput,
) -> Result<GeneratedImageCandidateArtifact> {
    let contract = generated_image_candidate_contract(input)?;
    let brief_artifact =
        require_story_image_brief_artifact(connection, &contract.brief_artifact_id)?;
    let contract_json = serde_json::to_value(&contract)?;
    let content_hash = stable_json_hash(&contract_json)?;

    if let Some(existing) = load_existing_story_image_artifact(
        connection,
        STORY_GENERATED_IMAGE_CANDIDATE_ARTIFACT_KIND,
        "story_image_brief",
        &contract.brief_artifact_id,
        &content_hash,
    )? {
        return Ok(GeneratedImageCandidateArtifact {
            artifact: existing,
            version: None,
            contract,
        });
    }

    let status = match contract.state {
        GeneratedImageCandidateState::Draft => "draft",
        GeneratedImageCandidateState::Requested => "requested",
        GeneratedImageCandidateState::Generated => "ready",
        GeneratedImageCandidateState::Failed => "failed",
        GeneratedImageCandidateState::Approved => "approved",
        GeneratedImageCandidateState::Rejected => "rejected",
    };
    let health_status = match contract.state {
        GeneratedImageCandidateState::Requested => "provider_pending",
        GeneratedImageCandidateState::Failed => "provider_failed",
        GeneratedImageCandidateState::Generated | GeneratedImageCandidateState::Approved => {
            "candidate_available"
        }
        GeneratedImageCandidateState::Draft | GeneratedImageCandidateState::Rejected => {
            "contract_only"
        }
    };
    let (artifact, _) = record_artifact(
        connection,
        ArtifactInput {
            artifact_kind: STORY_GENERATED_IMAGE_CANDIDATE_ARTIFACT_KIND.to_string(),
            title: format!("Generated image candidate {}", contract.candidate_id),
            status: status.to_string(),
            visibility_ceiling: contract.visibility.clone(),
            summary: format!(
                "Generated image candidate is {} with provider status `{}`.",
                contract.state.as_str(),
                safe_provider_text(&contract.provider_status)
            ),
            source_kind: Some("story_image_brief".to_string()),
            source_id: Some(contract.brief_artifact_id.clone()),
            evidence_refs: contract.evidence_refs.clone(),
            provenance: json!({
                "schemaVersion": CONTRACT_SCHEMA_VERSION,
                "generatedBy": "image.generateVariants.guard",
                "briefArtifactId": brief_artifact.id,
                "briefArtifactContentHash": brief_artifact.content_hash,
                "contract": contract_json,
                "liveProviderCalled": false,
            }),
            content_hash: content_hash.clone(),
            storage_uri: contract.storage_uri.clone(),
            health_status: Some(health_status.to_string()),
            created_by_job_id: None,
        },
    )?;
    let version = add_artifact_version(
        connection,
        &artifact.id,
        &content_hash,
        artifact.storage_uri.as_deref(),
        json!({
            "schemaVersion": CONTRACT_SCHEMA_VERSION,
            "contract": contract,
            "liveProviderCalled": false,
        }),
    )?;

    Ok(GeneratedImageCandidateArtifact {
        artifact,
        version: Some(version),
        contract,
    })
}

pub fn record_story_image_provider_request_envelope(
    connection: &Connection,
    input: StoryImageProviderRequestInput,
) -> Result<StoryImageProviderRequestOutcome> {
    let brief_artifact = require_story_image_brief_artifact(connection, &input.brief_artifact_id)?;
    let brief_contract = story_image_brief_contract_from_artifact(&brief_artifact)?;
    let request = story_image_provider_request_envelope(&brief_artifact, &brief_contract, input)?;
    let request_json = serde_json::to_value(&request)?;
    let request_hash = stable_json_hash(&request_json)?;

    if let Some(existing) =
        load_existing_provider_request_by_idempotency(connection, &request.idempotency_key)?
    {
        if existing.content_hash != request_hash {
            bail!("image provider request idempotency key conflicts with a different input");
        }
        let (existing_request, mut existing_response) =
            provider_envelopes_from_artifact(&existing)?;
        let candidates = load_generated_candidates_for_request(connection, &existing.id)?;
        existing_response.candidate_artifact_ids = candidates
            .iter()
            .map(|candidate| candidate.artifact.id.clone())
            .collect();
        return Ok(StoryImageProviderRequestOutcome {
            envelope_artifact: existing,
            request: existing_request,
            response: existing_response,
            candidates,
        });
    }

    let (
        initial_status,
        provider_status,
        candidate_state,
        storage_prefix,
        mut response_limitations,
    ) = fixture_result_shape(&request)?;
    let (envelope_artifact, _) = record_artifact(
        connection,
        ArtifactInput {
            artifact_kind: STORY_IMAGE_PROVIDER_REQUEST_ENVELOPE_ARTIFACT_KIND.to_string(),
            title: format!("Image provider request {}", request.request_id),
            status: initial_status.to_string(),
            visibility_ceiling: request.visibility.clone(),
            summary: format!(
                "Deterministic image provider request envelope for {} variants.",
                request.requested_count
            ),
            source_kind: Some("story_image_provider_request".to_string()),
            source_id: Some(request.idempotency_key.clone()),
            evidence_refs: request.evidence_refs.clone(),
            provenance: json!({
                "schemaVersion": CONTRACT_SCHEMA_VERSION,
                "generatedBy": "image.generateVariants.envelope",
                "briefArtifactId": brief_artifact.id,
                "requestEnvelope": request,
                "liveProviderCalled": false,
            }),
            content_hash: request_hash.clone(),
            storage_uri: Some(format!(
                "ordo://artifacts/story-image-provider-requests/{}",
                safe_identifier(&request.idempotency_key)
            )),
            health_status: Some(provider_status.to_string()),
            created_by_job_id: None,
        },
    )?;

    let mut candidates = Vec::new();
    let envelope_ref = format!("story_image_provider_request:{}", envelope_artifact.id);
    for index in 0..request.requested_count {
        let variant = index + 1;
        let storage_uri = storage_prefix
            .as_ref()
            .map(|prefix| format!("{prefix}/variant-{variant}.png"));
        let mut candidate_limitations = vec![
            "Generated image candidate was recorded from a deterministic fixture envelope; no live provider was called."
                .to_string(),
        ];
        candidate_limitations.extend(response_limitations.clone());
        let candidate = record_generated_image_candidate_artifact(
            connection,
            GeneratedImageCandidateInput {
                brief_artifact_id: brief_artifact.id.clone(),
                candidate_id: format!("{}:variant:{variant}", request.request_id),
                state: candidate_state.clone(),
                provider_status: provider_status.to_string(),
                storage_uri,
                visibility: "staff".to_string(),
                approval_state: if candidate_state == GeneratedImageCandidateState::Generated {
                    "pending_review".to_string()
                } else {
                    "draft".to_string()
                },
                evidence_refs: stable_strings(vec![
                    brief_artifact.id.clone(),
                    envelope_ref.clone(),
                    "provider_fixture:image.generateVariants".to_string(),
                ]),
                limitations: candidate_limitations,
            },
        )?;
        candidates.push(candidate);
    }

    let response_status = match candidate_state {
        GeneratedImageCandidateState::Generated => "generated",
        GeneratedImageCandidateState::Failed => "failed",
        GeneratedImageCandidateState::Requested => "requested",
        _ => initial_status,
    };
    let response = StoryImageProviderResponseEnvelope {
        schema_version: CONTRACT_SCHEMA_VERSION.to_string(),
        request_id: request.request_id.clone(),
        status: response_status.to_string(),
        provider_status: provider_status.to_string(),
        candidate_artifact_ids: candidates
            .iter()
            .map(|candidate| candidate.artifact.id.clone())
            .collect(),
        evidence_refs: stable_strings(vec![
            brief_artifact.id.clone(),
            envelope_ref,
            "provider_fixture:image.generateVariants".to_string(),
        ]),
        limitations: {
            response_limitations.push(
                "No live GPT image provider, network request, publication, or analytics event was run."
                    .to_string(),
            );
            stable_strings(response_limitations)
        },
        live_provider_called: false,
    };

    connection.execute(
        "UPDATE artifacts
         SET provenance_json = json_set(provenance_json, '$.responseEnvelope', json(?2)),
             updated_at = datetime('now')
         WHERE id = ?1",
        params![envelope_artifact.id, serde_json::to_string(&response)?],
    )?;
    add_artifact_version(
        connection,
        &envelope_artifact.id,
        &request_hash,
        envelope_artifact.storage_uri.as_deref(),
        json!({
            "schemaVersion": CONTRACT_SCHEMA_VERSION,
            "requestEnvelope": request,
            "responseEnvelope": response,
            "candidateArtifactIds": response.candidate_artifact_ids,
            "liveProviderCalled": false,
        }),
    )?;

    Ok(StoryImageProviderRequestOutcome {
        envelope_artifact,
        request,
        response,
        candidates,
    })
}

pub fn record_story_image_review_artifact(
    connection: &Connection,
    input: StoryImageReviewInput,
) -> Result<StoryImageReviewArtifact> {
    let contract = story_image_review_contract(connection, input)?;
    let contract_json = serde_json::to_value(&contract)?;
    let content_hash = stable_json_hash(&contract_json)?;

    if let Some(existing) =
        load_existing_image_review_by_idempotency(connection, &contract.review_id)?
    {
        if existing.content_hash != content_hash {
            bail!("image review idempotency key conflicts with a different input");
        }
        return Ok(StoryImageReviewArtifact {
            artifact: existing,
            version: None,
            contract,
        });
    }

    let (artifact, _) = record_artifact(
        connection,
        ArtifactInput {
            artifact_kind: STORY_IMAGE_REVIEW_ARTIFACT_KIND.to_string(),
            title: format!("Story image review {}", contract.review_id),
            status: contract.review_state.as_str().to_string(),
            visibility_ceiling: contract.visibility.clone(),
            summary: format!(
                "Deterministic image review is {} for generated image candidate `{}`.",
                contract.review_state.as_str(),
                safe_identifier(&contract.source_candidate_id)
            ),
            source_kind: Some("story_image_review".to_string()),
            source_id: Some(contract.review_id.clone()),
            evidence_refs: contract.evidence_refs.clone(),
            provenance: json!({
                "schemaVersion": CONTRACT_SCHEMA_VERSION,
                "generatedBy": "image.reviewAgainstBrief.fixture",
                "contract": contract_json,
                "candidateArtifactId": contract.candidate_artifact_id,
                "briefArtifactId": contract.brief_artifact_id,
                "publicationEffect": contract.publication_effect,
                "memoryEffect": contract.memory_effect,
                "liveProviderCalled": false,
                "automaticPublicDerivative": false,
                "automaticMemoryTruthPromotion": false,
            }),
            content_hash: content_hash.clone(),
            storage_uri: Some(format!(
                "ordo://artifacts/story-image-reviews/{}",
                safe_identifier(&contract.review_id)
            )),
            health_status: Some(contract.review_state.health_status().to_string()),
            created_by_job_id: None,
        },
    )?;
    let version = add_artifact_version(
        connection,
        &artifact.id,
        &content_hash,
        artifact.storage_uri.as_deref(),
        json!({
            "schemaVersion": CONTRACT_SCHEMA_VERSION,
            "contract": contract,
            "liveProviderCalled": false,
        }),
    )?;

    Ok(StoryImageReviewArtifact {
        artifact,
        version: Some(version),
        contract,
    })
}

fn require_story_image_brief_artifact(
    connection: &Connection,
    artifact_id: &str,
) -> Result<ArtifactView> {
    let artifact = load_artifact(connection, artifact_id).map_err(|_| {
        anyhow::anyhow!("generated image candidate requires a known image brief artifact")
    })?;
    ensure!(
        artifact.artifact_kind == STORY_IMAGE_BRIEF_ARTIFACT_KIND,
        "generated image candidate source must be a story image brief artifact"
    );
    Ok(artifact)
}

fn require_generated_image_candidate_artifact(
    connection: &Connection,
    artifact_id: &str,
) -> Result<GeneratedImageCandidateArtifact> {
    let artifact = load_artifact(connection, artifact_id).map_err(|_| {
        anyhow::anyhow!("image review requires a known generated image candidate artifact")
    })?;
    ensure!(
        artifact.artifact_kind == STORY_GENERATED_IMAGE_CANDIDATE_ARTIFACT_KIND,
        "image review source must be a generated image candidate artifact"
    );
    let contract_value = artifact
        .provenance
        .get("contract")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("generated image candidate artifact is missing contract"))?;
    let contract = serde_json::from_value(contract_value)?;
    Ok(GeneratedImageCandidateArtifact {
        artifact,
        version: None,
        contract,
    })
}

fn story_image_brief_contract_from_artifact(
    artifact: &ArtifactView,
) -> Result<StoryImageBriefContract> {
    let contract_value = artifact
        .provenance
        .get("contract")
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!("story image brief artifact is missing provider contract")
        })?;
    Ok(serde_json::from_value(contract_value)?)
}

fn story_image_review_contract(
    connection: &Connection,
    input: StoryImageReviewInput,
) -> Result<StoryImageReviewContract> {
    ensure!(
        matches!(
            input.visibility.as_str(),
            "staff" | "owner" | "authenticated" | "public"
        ),
        "image review visibility must be explicit"
    );
    let review_id = normalize_review_idempotency_key(&input.idempotency_key)?;
    ensure!(
        !input.fixture_id.trim().is_empty(),
        "image review fixture id is required"
    );
    ensure!(
        input
            .fixture_id
            .starts_with("fixture:image.reviewAgainstBrief:"),
        "image review must use deterministic fixture evidence"
    );
    ensure!(
        !input.reviewer_ref.trim().is_empty(),
        "image review reviewer ref is required"
    );
    ensure!(
        !input.evidence_refs.is_empty(),
        "image review evidence refs are required"
    );
    let raw_review_inputs = serde_json::to_string(&json!({
        "reviewerRef": input.reviewer_ref,
        "fixtureId": input.fixture_id,
        "revisionGuidance": input.revision_guidance,
        "limitations": input.limitations,
        "evidenceRefs": input.evidence_refs,
    }))?;
    ensure!(
        !contains_forbidden_provider_marker(&raw_review_inputs),
        "image review contains private or unsupported markers"
    );
    if input.review_state == StoryImageReviewState::NeedsRevision {
        ensure!(
            input
                .revision_guidance
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()),
            "revision image review requires revision guidance"
        );
    }
    if input.review_state != StoryImageReviewState::NeedsRevision {
        ensure!(
            input.revision_guidance.is_none(),
            "only revision image reviews may include revision guidance"
        );
    }

    let candidate =
        require_generated_image_candidate_artifact(connection, &input.candidate_artifact_id)?;
    let brief_artifact = require_story_image_brief_artifact(connection, &input.brief_artifact_id)?;
    ensure!(
        candidate.contract.brief_artifact_id == brief_artifact.id,
        "image review source brief must match generated candidate brief"
    );
    ensure!(
        matches!(
            candidate.contract.state,
            GeneratedImageCandidateState::Generated | GeneratedImageCandidateState::Approved
        ),
        "image review requires generated candidate storage evidence"
    );
    ensure!(
        candidate.contract.storage_uri.is_some(),
        "image review requires generated candidate storage evidence"
    );

    let reviewer_ref = safe_provider_text(&input.reviewer_ref);
    let fixture_id = safe_provider_text(&input.fixture_id);
    let revision_guidance = input
        .revision_guidance
        .map(|guidance| safe_provider_text(&guidance))
        .filter(|guidance| !guidance.trim().is_empty());
    let input_limitations = input
        .limitations
        .into_iter()
        .map(|limitation| safe_provider_text(&limitation))
        .collect::<Vec<_>>();
    let input_refs = input
        .evidence_refs
        .into_iter()
        .map(|reference| safe_identifier(&reference))
        .collect::<Vec<_>>();
    let serialized_review_inputs = serde_json::to_string(&json!({
        "reviewerRef": reviewer_ref,
        "fixtureId": fixture_id,
        "revisionGuidance": revision_guidance,
        "limitations": input_limitations,
        "evidenceRefs": input_refs,
    }))?;
    ensure!(
        !contains_forbidden_provider_marker(&serialized_review_inputs),
        "image review contains private or unsupported markers"
    );

    let mut evidence_refs = vec![
        candidate.artifact.id.clone(),
        brief_artifact.id.clone(),
        fixture_id.clone(),
    ];
    evidence_refs.extend(input_refs);
    evidence_refs.extend(candidate.contract.evidence_refs.clone());
    let mut limitations = vec![
        "Image review was recorded from a deterministic fixture; no live provider was called."
            .to_string(),
        "Review artifact is evidence for publication preparation only; it does not publish, promote graph truth, or promote memory truth."
            .to_string(),
    ];
    limitations.extend(candidate.contract.limitations.clone());
    limitations.extend(input_limitations);

    Ok(StoryImageReviewContract {
        schema_version: CONTRACT_SCHEMA_VERSION.to_string(),
        review_id,
        method: "image.reviewAgainstBrief".to_string(),
        candidate_artifact_id: candidate.artifact.id,
        brief_artifact_id: brief_artifact.id,
        source_candidate_id: candidate.contract.candidate_id,
        review_state: input.review_state.clone(),
        reviewer_ref,
        fixture_id,
        visibility: input.visibility,
        revision_guidance,
        evidence_refs: public_safe_values(evidence_refs),
        limitations: public_safe_values(limitations),
        publication_effect: input.review_state.publication_effect().to_string(),
        memory_effect: "review_evidence_only".to_string(),
        live_provider_called: false,
    })
}

fn story_image_provider_request_envelope(
    brief_artifact: &ArtifactView,
    brief: &StoryImageBriefContract,
    input: StoryImageProviderRequestInput,
) -> Result<StoryImageProviderRequestEnvelope> {
    let idempotency_key = normalize_idempotency_key(&input.idempotency_key)?;
    ensure!(
        input.provider_mode == "deterministic_fixture",
        "image provider request mode must be deterministic_fixture for default validation"
    );
    ensure!(
        matches!(
            input.fixture_status.as_str(),
            "requested" | "generated" | "failed"
        ),
        "image provider fixture status is unsupported"
    );
    ensure!(
        input.requested_count >= 1 && input.requested_count <= 4,
        "image provider request count must be between 1 and 4"
    );
    ensure!(
        !contains_forbidden_provider_marker(&input.provider_name)
            && !contains_forbidden_provider_marker(&input.model_hint),
        "image provider request contains private or unsupported markers"
    );
    let provider_name = safe_provider_text(&input.provider_name);
    let model_hint = safe_provider_text(&input.model_hint);
    ensure!(
        !provider_name.trim().is_empty() && !model_hint.trim().is_empty(),
        "image provider request requires provider and model hint"
    );
    ensure!(
        !contains_forbidden_provider_marker(&provider_name)
            && !contains_forbidden_provider_marker(&model_hint),
        "image provider request contains private or unsupported markers"
    );

    let (requested_size, size_limitation) = normalize_image_size(&input.requested_size);
    let (requested_aspect_ratio, aspect_limitation) =
        normalize_provider_aspect_ratio(&input.requested_aspect_ratio);
    let mut limitations = vec![
        "Image provider request envelope is deterministic fixture mode; no live GPT image provider was called."
            .to_string(),
        "Prompt payload is referenced by artifact id and is not serialized into the request envelope."
            .to_string(),
        "Generated candidates require review before any public derivative or publication."
            .to_string(),
    ];
    limitations.extend(brief.limitations.clone());
    limitations.extend(size_limitation);
    limitations.extend(aspect_limitation);
    limitations = public_safe_values(limitations);
    let request_id = format!(
        "story_image_provider_request:{}",
        safe_identifier(&idempotency_key)
    );
    let evidence_refs = stable_strings(
        brief
            .evidence_refs
            .iter()
            .cloned()
            .chain([
                brief_artifact.id.clone(),
                "provider_fixture:image.generateVariants".to_string(),
            ])
            .collect(),
    );

    Ok(StoryImageProviderRequestEnvelope {
        schema_version: CONTRACT_SCHEMA_VERSION.to_string(),
        request_id,
        method: "image.generateVariants".to_string(),
        provider_name,
        model_hint,
        provider_mode: input.provider_mode,
        requested_size,
        requested_aspect_ratio,
        requested_count: input.requested_count,
        fixture_status: input.fixture_status,
        prompt_payload_ref: format!("artifact:{}:providerPayload", brief_artifact.id),
        idempotency_key,
        source_artifact_refs: vec![brief_artifact.id.clone()],
        visibility: "staff".to_string(),
        evidence_refs,
        limitations,
    })
}

fn normalize_idempotency_key(idempotency_key: &str) -> Result<String> {
    let key = idempotency_key.trim();
    ensure!(
        !key.is_empty(),
        "image provider request idempotency key cannot be blank"
    );
    ensure!(
        key.len() <= 200,
        "image provider request idempotency key is too long"
    );
    Ok(safe_identifier(key))
}

fn normalize_review_idempotency_key(idempotency_key: &str) -> Result<String> {
    let key = idempotency_key.trim();
    ensure!(
        !key.is_empty(),
        "image review idempotency key cannot be blank"
    );
    ensure!(key.len() <= 200, "image review idempotency key is too long");
    Ok(safe_identifier(key))
}

fn normalize_image_size(size: &str) -> (String, Vec<String>) {
    match size.trim() {
        DEFAULT_IMAGE_SIZE | "" => (DEFAULT_IMAGE_SIZE.to_string(), Vec::new()),
        other => (
            DEFAULT_IMAGE_SIZE.to_string(),
            vec![format!(
                "Unsupported image size `{}` defaulted to {}; no live provider call was made.",
                safe_provider_text(other),
                DEFAULT_IMAGE_SIZE
            )],
        ),
    }
}

fn normalize_provider_aspect_ratio(aspect_ratio: &str) -> (String, Vec<String>) {
    match aspect_ratio.trim() {
        DEFAULT_ASPECT_RATIO | "" => (DEFAULT_ASPECT_RATIO.to_string(), Vec::new()),
        other => (
            DEFAULT_ASPECT_RATIO.to_string(),
            vec![format!(
                "Unsupported image aspect ratio `{}` defaulted to {}; no live provider call was made.",
                safe_provider_text(other),
                DEFAULT_ASPECT_RATIO
            )],
        ),
    }
}

fn fixture_result_shape(
    request: &StoryImageProviderRequestEnvelope,
) -> Result<(
    &'static str,
    &'static str,
    GeneratedImageCandidateState,
    Option<String>,
    Vec<String>,
)> {
    let request_hash = stable_json_hash(&serde_json::to_value(request)?)?;
    let suffix = request_hash.trim_start_matches("sha256:");
    match request.fixture_status.as_str() {
        "requested" => Ok((
            "requested",
            "provider_request_recorded",
            GeneratedImageCandidateState::Requested,
            None,
            vec![
                "Deterministic fixture recorded request state only; no generated storage URI is available."
                    .to_string(),
            ],
        )),
        "failed" => Ok((
            "failed",
            "deterministic_fixture_failed",
            GeneratedImageCandidateState::Failed,
            None,
            vec!["Deterministic fixture recorded provider failure; no generated storage URI is available.".to_string()],
        )),
        "generated" => Ok((
            "ready",
            "deterministic_fixture_generated",
            GeneratedImageCandidateState::Generated,
            Some(format!(
                "ordo://fixtures/story-images/{}/{}",
                safe_identifier(&request.idempotency_key),
                suffix.get(0..12).unwrap_or(suffix)
            )),
            vec![
                "Generated state is backed by deterministic fixture storage evidence, not live provider success."
                    .to_string(),
            ],
        )),
        _ => bail!("image provider fixture status is unsupported"),
    }
}

fn load_existing_provider_request_by_idempotency(
    connection: &Connection,
    idempotency_key: &str,
) -> Result<Option<ArtifactView>> {
    let artifact_id = connection
        .query_row(
            "SELECT id FROM artifacts
             WHERE artifact_kind = ?1
               AND source_kind = 'story_image_provider_request'
               AND source_id = ?2
             ORDER BY created_at ASC
             LIMIT 1",
            params![
                STORY_IMAGE_PROVIDER_REQUEST_ENVELOPE_ARTIFACT_KIND,
                idempotency_key
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    artifact_id
        .map(|id| load_artifact(connection, &id))
        .transpose()
}

fn load_existing_image_review_by_idempotency(
    connection: &Connection,
    review_id: &str,
) -> Result<Option<ArtifactView>> {
    let artifact_id = connection
        .query_row(
            "SELECT id FROM artifacts
             WHERE artifact_kind = ?1
               AND source_kind = 'story_image_review'
               AND source_id = ?2
             ORDER BY created_at ASC
             LIMIT 1",
            params![STORY_IMAGE_REVIEW_ARTIFACT_KIND, review_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    artifact_id
        .map(|id| load_artifact(connection, &id))
        .transpose()
}

fn provider_envelopes_from_artifact(
    artifact: &ArtifactView,
) -> Result<(
    StoryImageProviderRequestEnvelope,
    StoryImageProviderResponseEnvelope,
)> {
    let request = artifact
        .provenance
        .get("requestEnvelope")
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!("image provider request artifact is missing request envelope")
        })?;
    let response = artifact
        .provenance
        .get("responseEnvelope")
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "schemaVersion": CONTRACT_SCHEMA_VERSION,
                "requestId": artifact.id,
                "status": artifact.status,
                "providerStatus": artifact.health_status,
                "candidateArtifactIds": [],
                "evidenceRefs": artifact.evidence_refs,
                "limitations": ["Existing request envelope predates response snapshot hydration."],
                "liveProviderCalled": false
            })
        });
    Ok((
        serde_json::from_value(request)?,
        serde_json::from_value(response)?,
    ))
}

fn load_generated_candidates_for_request(
    connection: &Connection,
    envelope_artifact_id: &str,
) -> Result<Vec<GeneratedImageCandidateArtifact>> {
    let needle = format!("story_image_provider_request:{envelope_artifact_id}");
    let mut statement = connection.prepare(
        "SELECT id FROM artifacts
         WHERE artifact_kind = ?1
           AND evidence_refs_json LIKE ?2
         ORDER BY created_at ASC, id ASC",
    )?;
    let ids = statement
        .query_map(
            params![
                STORY_GENERATED_IMAGE_CANDIDATE_ARTIFACT_KIND,
                format!("%{}%", needle)
            ],
            |row| row.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.into_iter()
        .map(|id| {
            let artifact = load_artifact(connection, &id)?;
            let contract_value = artifact
                .provenance
                .get("contract")
                .cloned()
                .ok_or_else(|| {
                    anyhow::anyhow!("generated candidate artifact is missing contract")
                })?;
            let contract = serde_json::from_value(contract_value)?;
            Ok(GeneratedImageCandidateArtifact {
                artifact,
                version: None,
                contract,
            })
        })
        .collect()
}

fn story_image_brief_contract(
    deck: &HomepageStoryDeckResponse,
    slide: &HomepageNarrativeSlide,
) -> Result<StoryImageBriefContract> {
    ensure!(
        !slide.slide_id.trim().is_empty(),
        "image brief requires a slide id"
    );
    ensure!(
        !slide.section_id.trim().is_empty(),
        "image brief requires a section id"
    );
    let evidence_refs = non_empty_evidence_refs(&slide.evidence_refs, &deck.deck.evidence_refs);
    let scene_intent = safe_provider_text(&slide.title);
    let visual_direction = safe_provider_text(&slide.body);
    let (aspect_ratio, mut aspect_limitations) = image_aspect_ratio_for_slide(slide);
    ensure!(
        !scene_intent.trim().is_empty() || !visual_direction.trim().is_empty(),
        "image brief requires public-safe scene intent or visual direction"
    );
    let mut limitations = vec![
        "Contract-only image brief; no live GPT image provider was called.".to_string(),
        "Provider payload is derived from published public-safe homepage story fields only."
            .to_string(),
        "Generated image candidates require explicit provider evidence before approval."
            .to_string(),
    ];
    limitations.append(&mut aspect_limitations);
    let prompt = safe_provider_text(&format!(
        "Create a cinematic public-safe homepage image for section `{}`. Scene intent: {}. Visual direction: {}. Avoid visible text, private people, account data, dashboards, provider UI, unsupported product claims, and anything that implies live generation already succeeded.",
        slide.section_id, slide.title, slide.body
    ));
    let provider_payload = StoryImageProviderPromptPayload {
        method: "image.generateVariants".to_string(),
        prompt,
        aspect_ratio: aspect_ratio.clone(),
        usage: "homepage.scrollytelling.section".to_string(),
        evidence_refs: evidence_refs.clone(),
        limitations: limitations.clone(),
    };
    ensure_provider_payload_safe(&provider_payload)?;

    Ok(StoryImageBriefContract {
        schema_version: CONTRACT_SCHEMA_VERSION.to_string(),
        brief_id: format!(
            "story_image_brief:{}:{}",
            safe_identifier(&deck.deck.deck_id),
            safe_identifier(&slide.slide_id)
        ),
        deck_id: deck.deck.deck_id.clone(),
        slide_id: slide.slide_id.clone(),
        section_id: slide.section_id.clone(),
        scene_intent,
        visual_direction,
        aspect_ratio,
        usage: "homepage.scrollytelling.section".to_string(),
        visibility: "staff".to_string(),
        approval_state: "draft".to_string(),
        evidence_refs,
        limitations,
        provider_payload,
    })
}

fn image_aspect_ratio_for_slide(slide: &HomepageNarrativeSlide) -> (String, Vec<String>) {
    let requested = slide
        .copy_slots
        .iter()
        .find(|slot| {
            matches!(
                slot.slot.as_str(),
                "imageAspectRatio" | "image_aspect_ratio"
            )
        })
        .and_then(|slot| slot.value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match requested {
        Some(DEFAULT_ASPECT_RATIO) | None => (DEFAULT_ASPECT_RATIO.to_string(), Vec::new()),
        Some(other) => (
            DEFAULT_ASPECT_RATIO.to_string(),
            vec![format!(
                "Unsupported image aspect ratio `{}` defaulted to {}; no provider call was made.",
                safe_provider_text(other),
                DEFAULT_ASPECT_RATIO
            )],
        ),
    }
}

fn generated_image_candidate_contract(
    input: GeneratedImageCandidateInput,
) -> Result<GeneratedImageCandidateContract> {
    ensure!(
        !input.brief_artifact_id.trim().is_empty(),
        "generated image candidate requires a brief artifact id"
    );
    ensure!(
        !input.candidate_id.trim().is_empty(),
        "generated image candidate requires a candidate id"
    );
    ensure!(
        !input.provider_status.trim().is_empty(),
        "generated image candidate requires provider status"
    );
    ensure!(
        matches!(
            input.visibility.as_str(),
            "staff" | "owner" | "authenticated" | "public"
        ),
        "generated image candidate visibility must be explicit"
    );
    ensure!(
        matches!(
            input.approval_state.as_str(),
            "draft" | "pending_review" | "approved" | "rejected"
        ),
        "generated image candidate approval state is unsupported"
    );
    ensure!(
        !input.evidence_refs.is_empty(),
        "generated image candidate requires evidence refs"
    );
    if matches!(
        input.state,
        GeneratedImageCandidateState::Generated | GeneratedImageCandidateState::Approved
    ) && input
        .storage_uri
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        bail!("generated or approved image candidates require storage evidence");
    }
    if matches!(input.state, GeneratedImageCandidateState::Approved) {
        ensure!(
            input.approval_state == "approved",
            "approved image candidates require approved approval state"
        );
    }
    if matches!(input.state, GeneratedImageCandidateState::Requested)
        && input.provider_status != "provider_request_recorded"
    {
        bail!("requested candidates must not claim provider completion");
    }
    let limitations = if input.limitations.is_empty() {
        vec!["No live provider success is implied by this contract.".to_string()]
    } else {
        input
            .limitations
            .into_iter()
            .map(|limitation| safe_provider_text(&limitation))
            .collect()
    };
    Ok(GeneratedImageCandidateContract {
        schema_version: CONTRACT_SCHEMA_VERSION.to_string(),
        candidate_id: safe_identifier(&input.candidate_id),
        brief_artifact_id: input.brief_artifact_id,
        state: input.state,
        provider_status: safe_provider_text(&input.provider_status),
        storage_uri: input.storage_uri,
        visibility: input.visibility,
        approval_state: input.approval_state,
        evidence_refs: input.evidence_refs,
        limitations,
    })
}

fn load_existing_story_image_artifact(
    connection: &Connection,
    artifact_kind: &str,
    source_kind: &str,
    source_id: &str,
    content_hash: &str,
) -> Result<Option<ArtifactView>> {
    let artifact_id = connection
        .query_row(
            "SELECT id FROM artifacts
             WHERE artifact_kind = ?1
               AND source_kind = ?2
               AND source_id = ?3
               AND content_hash = ?4
             ORDER BY created_at ASC
             LIMIT 1",
            params![artifact_kind, source_kind, source_id, content_hash],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    artifact_id
        .map(|id| load_artifact(connection, &id))
        .transpose()
}

fn public_safe_values(values: Vec<String>) -> Vec<String> {
    stable_strings(
        values
            .into_iter()
            .map(|value| safe_provider_text(&value))
            .filter(|value| !value.trim().is_empty() && !contains_forbidden_provider_marker(value))
            .collect(),
    )
}

fn stable_strings(values: Vec<String>) -> Vec<String> {
    let mut values = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn non_empty_evidence_refs(slide_refs: &[String], deck_refs: &[String]) -> Vec<String> {
    let mut refs = slide_refs
        .iter()
        .chain(deck_refs.iter())
        .filter(|reference| !reference.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    refs.sort();
    refs.dedup();
    if refs.is_empty() {
        refs.push("homepage.story.public_fields".to_string());
    }
    refs
}

fn ensure_provider_payload_safe(payload: &StoryImageProviderPromptPayload) -> Result<()> {
    ensure!(
        payload.aspect_ratio == DEFAULT_ASPECT_RATIO,
        "unsupported image brief aspect ratio"
    );
    let serialized = serde_json::to_string(payload)?;
    ensure!(
        !contains_forbidden_provider_marker(&serialized),
        "provider payload contains private or unsupported markers"
    );
    Ok(())
}

fn safe_provider_text(text: &str) -> String {
    let mut redacted = redaction::redact_public_text(text);
    for marker in [
        "staff routing",
        "staffRouting",
        "provider internal",
        "providerInternal",
        "prompt internal",
        "promptInternal",
        "provider secret",
        "providerSecret",
        "raw policy internal",
        "rawPolicyInternal",
        "policy internal",
        "policyInternal",
        "owner-only",
        "owner only",
        "ownerOnly",
        "private artifact text",
        "privateArtifactText",
        "compiled plan private input",
        "compiledPlanPrivateInput",
        "task private payload",
        "taskPrivatePayload",
        "graph certainty",
        "graphCertainty",
        "unsupported claim",
        "unsupportedClaim",
        "live provider succeeded",
        "liveProviderSucceeded",
    ] {
        redacted = replace_ascii_case_insensitive(&redacted, marker, "[REDACTED_POLICY_BOUNDARY]");
    }
    redacted
}

fn replace_ascii_case_insensitive(input: &str, needle: &str, replacement: &str) -> String {
    let mut output = String::new();
    let mut remainder = input;
    let needle_lower = needle.to_ascii_lowercase();
    while let Some(index) = remainder.to_ascii_lowercase().find(&needle_lower) {
        output.push_str(&remainder[..index]);
        output.push_str(replacement);
        remainder = &remainder[index + needle.len()..];
    }
    output.push_str(remainder);
    output
}

fn contains_forbidden_provider_marker(text: &str) -> bool {
    let normalized = text
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    [
        "staffrouting",
        "providerinternal",
        "promptinternal",
        "providersecret",
        "rawpolicyinternal",
        "policyinternal",
        "owneronly",
        "privateartifacttext",
        "compiledplanprivateinput",
        "taskprivatepayload",
        "graphcertainty",
        "unsupportedclaim",
        "liveprovidersucceeded",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn safe_identifier(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
}

fn stable_json_hash(value: &Value) -> Result<String> {
    let encoded = serde_json::to_string(value)?;
    let mut hasher = Sha256::new();
    hasher.update(encoded.as_bytes());
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public_surfaces::{
        HomepageNarrativeDeck, HomepageStoryCopySlot, HomepageStoryProfile,
        HomepageStoryRefreshContract, PublicSurfaceReadiness,
    };
    use crate::schema::init_schema;

    #[test]
    fn story_image_briefs_are_durable_idempotent_and_provider_safe() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let deck = deck_with_slide("hero", "Studio Ordo", "A local-first operating appliance.");

        let first = prepare_story_image_brief_artifacts(&connection, &deck).unwrap();
        let second = prepare_story_image_brief_artifacts(&connection, &deck).unwrap();

        assert_eq!(first.len(), 1);
        assert_eq!(first[0].artifact.id, second[0].artifact.id);
        assert_eq!(
            first[0].artifact.artifact_kind,
            STORY_IMAGE_BRIEF_ARTIFACT_KIND
        );
        assert_eq!(first[0].artifact.status, "draft");
        assert_eq!(first[0].artifact.visibility_ceiling, "staff");
        assert!(first[0].version.is_some());
        assert!(second[0].version.is_none());
        assert_eq!(
            first[0].contract.provider_payload.method,
            "image.generateVariants"
        );
        assert!(first[0]
            .contract
            .provider_payload
            .prompt
            .contains("Studio Ordo"));

        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE artifact_kind = ?1",
                [STORY_IMAGE_BRIEF_ARTIFACT_KIND],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn provider_payload_omits_private_prompt_provider_and_policy_markers() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let deck = deck_with_slide(
            "hero",
            "Owner-only promptInternal providerSecret",
            "Call 555-555-5555 with sk-live-secret and raw policy internal notes.",
        );

        let artifacts = prepare_story_image_brief_artifacts(&connection, &deck).unwrap();
        let serialized = serde_json::to_string(&artifacts[0].contract).unwrap();

        assert!(!serialized.to_ascii_lowercase().contains("owner-only"));
        assert!(!serialized.contains("promptInternal"));
        assert!(!serialized.contains("providerSecret"));
        assert!(!serialized.contains("raw policy internal"));
        assert!(!serialized.contains("555-555-5555"));
        assert!(!serialized.contains("sk-live-secret"));
        assert!(serialized.contains("[REDACTED_POLICY_BOUNDARY]"));
        assert!(serialized.contains("[REDACTED_PHONE]"));
        assert!(serialized.contains("[REDACTED_SECRET]"));
    }

    #[test]
    fn generated_candidate_states_do_not_pretend_provider_success() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let deck = deck_with_slide("hero", "Studio Ordo", "A public proof scene.");
        let brief = prepare_story_image_brief_artifacts(&connection, &deck)
            .unwrap()
            .remove(0);

        let requested = record_generated_image_candidate_artifact(
            &connection,
            GeneratedImageCandidateInput {
                brief_artifact_id: brief.artifact.id.clone(),
                candidate_id: "candidate_1".to_string(),
                state: GeneratedImageCandidateState::Requested,
                provider_status: "provider_request_recorded".to_string(),
                storage_uri: None,
                visibility: "staff".to_string(),
                approval_state: "draft".to_string(),
                evidence_refs: vec![brief.artifact.id.clone()],
                limitations: vec![],
            },
        )
        .unwrap();
        assert_eq!(requested.artifact.status, "requested");
        assert_eq!(
            requested.artifact.health_status.as_deref(),
            Some("provider_pending")
        );

        let fake_generated = record_generated_image_candidate_artifact(
            &connection,
            GeneratedImageCandidateInput {
                brief_artifact_id: brief.artifact.id,
                candidate_id: "candidate_2".to_string(),
                state: GeneratedImageCandidateState::Generated,
                provider_status: "provider_completed".to_string(),
                storage_uri: None,
                visibility: "staff".to_string(),
                approval_state: "pending_review".to_string(),
                evidence_refs: vec!["provider_call:missing".to_string()],
                limitations: vec![],
            },
        );
        assert!(fake_generated
            .unwrap_err()
            .to_string()
            .contains("storage evidence"));
    }

    #[test]
    fn generated_candidate_requires_known_story_image_brief_artifact() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        let missing = record_generated_image_candidate_artifact(
            &connection,
            GeneratedImageCandidateInput {
                brief_artifact_id: "artifact_missing".to_string(),
                candidate_id: "candidate_missing".to_string(),
                state: GeneratedImageCandidateState::Requested,
                provider_status: "provider_request_recorded".to_string(),
                storage_uri: None,
                visibility: "staff".to_string(),
                approval_state: "draft".to_string(),
                evidence_refs: vec!["artifact_missing".to_string()],
                limitations: vec![],
            },
        );
        assert!(missing
            .unwrap_err()
            .to_string()
            .contains("known image brief artifact"));

        let (wrong_kind, _) = record_artifact(
            &connection,
            ArtifactInput {
                artifact_kind: "homepage.storyboard".to_string(),
                title: "Storyboard".to_string(),
                status: "draft".to_string(),
                visibility_ceiling: "staff".to_string(),
                summary: "Wrong source kind for generated image candidate.".to_string(),
                source_kind: Some("homepage_story_slide".to_string()),
                source_id: Some("hero".to_string()),
                evidence_refs: vec!["business_fact:homepage".to_string()],
                provenance: json!({"test": true}),
                content_hash: "sha256:storyboard".to_string(),
                storage_uri: None,
                health_status: Some("contract_only".to_string()),
                created_by_job_id: None,
            },
        )
        .unwrap();

        let wrong_kind_result = record_generated_image_candidate_artifact(
            &connection,
            GeneratedImageCandidateInput {
                brief_artifact_id: wrong_kind.id,
                candidate_id: "candidate_wrong_kind".to_string(),
                state: GeneratedImageCandidateState::Requested,
                provider_status: "provider_request_recorded".to_string(),
                storage_uri: None,
                visibility: "staff".to_string(),
                approval_state: "draft".to_string(),
                evidence_refs: vec!["business_fact:homepage".to_string()],
                limitations: vec![],
            },
        );
        assert!(wrong_kind_result
            .unwrap_err()
            .to_string()
            .contains("story image brief artifact"));
    }

    #[test]
    fn unsupported_aspect_ratio_defaults_with_limitation_without_provider_claim() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let mut deck = deck_with_slide("hero", "Studio Ordo", "A public proof scene.");
        deck.deck.slides[0].copy_slots = vec![HomepageStoryCopySlot {
            slot: "imageAspectRatio".to_string(),
            value: json!("21:9"),
        }];

        let brief = prepare_story_image_brief_artifacts(&connection, &deck)
            .unwrap()
            .remove(0);

        assert_eq!(brief.contract.aspect_ratio, "16:9");
        assert_eq!(brief.contract.provider_payload.aspect_ratio, "16:9");
        assert!(brief
            .contract
            .limitations
            .iter()
            .any(|limitation| limitation.contains("Unsupported image aspect ratio")));
        assert_eq!(
            brief.artifact.health_status.as_deref(),
            Some("contract_only")
        );
    }

    #[test]
    fn provider_request_envelope_records_deterministic_generated_candidates_idempotently() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let deck = deck_with_slide("hero", "Studio Ordo", "A public proof scene.");
        let brief = prepare_story_image_brief_artifacts(&connection, &deck)
            .unwrap()
            .remove(0);

        let first = record_story_image_provider_request_envelope(
            &connection,
            StoryImageProviderRequestInput {
                brief_artifact_id: brief.artifact.id.clone(),
                idempotency_key: "hero-image-v1".to_string(),
                provider_name: "openai".to_string(),
                model_hint: "gpt-image-2".to_string(),
                provider_mode: "deterministic_fixture".to_string(),
                requested_size: "1024x576".to_string(),
                requested_aspect_ratio: "16:9".to_string(),
                requested_count: 2,
                fixture_status: "generated".to_string(),
            },
        )
        .unwrap();
        let repeated = record_story_image_provider_request_envelope(
            &connection,
            StoryImageProviderRequestInput {
                brief_artifact_id: brief.artifact.id.clone(),
                idempotency_key: "hero-image-v1".to_string(),
                provider_name: "openai".to_string(),
                model_hint: "gpt-image-2".to_string(),
                provider_mode: "deterministic_fixture".to_string(),
                requested_size: "1024x576".to_string(),
                requested_aspect_ratio: "16:9".to_string(),
                requested_count: 2,
                fixture_status: "generated".to_string(),
            },
        )
        .unwrap();

        assert_eq!(first.envelope_artifact.id, repeated.envelope_artifact.id);
        assert_eq!(first.candidates.len(), 2);
        assert_eq!(repeated.candidates.len(), 2);
        assert_eq!(first.response.status, "generated");
        assert!(!first.response.live_provider_called);
        assert_eq!(first.request.method, "image.generateVariants");
        assert_eq!(first.request.provider_mode, "deterministic_fixture");
        assert_eq!(first.request.model_hint, "gpt-image-2");
        assert_eq!(
            first.request.prompt_payload_ref,
            format!("artifact:{}:providerPayload", brief.artifact.id)
        );
        assert!(first
            .candidates
            .iter()
            .all(|candidate| candidate.contract.state == GeneratedImageCandidateState::Generated));
        assert!(first
            .candidates
            .iter()
            .all(|candidate| candidate.contract.storage_uri.is_some()));

        let serialized = serde_json::to_string(&first).unwrap();
        assert!(!serialized.contains(&brief.contract.provider_payload.prompt));
        assert!(!serialized.contains("live provider succeeded"));

        let envelope_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE artifact_kind = ?1",
                [STORY_IMAGE_PROVIDER_REQUEST_ENVELOPE_ARTIFACT_KIND],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(envelope_count, 1);
        let candidate_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE artifact_kind = ?1",
                [STORY_GENERATED_IMAGE_CANDIDATE_ARTIFACT_KIND],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(candidate_count, 2);
    }

    #[test]
    fn provider_request_envelope_records_fixture_failure_without_fake_storage() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let deck = deck_with_slide("hero", "Studio Ordo", "A public proof scene.");
        let brief = prepare_story_image_brief_artifacts(&connection, &deck)
            .unwrap()
            .remove(0);

        let outcome = record_story_image_provider_request_envelope(
            &connection,
            StoryImageProviderRequestInput {
                brief_artifact_id: brief.artifact.id,
                idempotency_key: "hero-image-failed".to_string(),
                provider_name: "openai".to_string(),
                model_hint: "gpt-image-2".to_string(),
                provider_mode: "deterministic_fixture".to_string(),
                requested_size: "2048x2048".to_string(),
                requested_aspect_ratio: "1:1".to_string(),
                requested_count: 1,
                fixture_status: "failed".to_string(),
            },
        )
        .unwrap();

        assert_eq!(outcome.response.status, "failed");
        assert_eq!(outcome.request.requested_size, "1024x576");
        assert_eq!(outcome.request.requested_aspect_ratio, "16:9");
        assert!(outcome
            .request
            .limitations
            .iter()
            .any(|limitation| limitation.contains("Unsupported image size")));
        assert!(outcome
            .request
            .limitations
            .iter()
            .any(|limitation| limitation.contains("Unsupported image aspect ratio")));
        assert_eq!(
            outcome.candidates[0].contract.state,
            GeneratedImageCandidateState::Failed
        );
        assert!(outcome.candidates[0].contract.storage_uri.is_none());
        assert!(!outcome.response.live_provider_called);
    }

    #[test]
    fn provider_request_envelope_rejects_live_mode_conflicts_and_private_markers() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let deck = deck_with_slide("hero", "Studio Ordo", "A public proof scene.");
        let brief = prepare_story_image_brief_artifacts(&connection, &deck)
            .unwrap()
            .remove(0);

        let live = record_story_image_provider_request_envelope(
            &connection,
            StoryImageProviderRequestInput {
                brief_artifact_id: brief.artifact.id.clone(),
                idempotency_key: "hero-image-live".to_string(),
                provider_name: "openai".to_string(),
                model_hint: "gpt-image-2".to_string(),
                provider_mode: "live".to_string(),
                requested_size: "1024x576".to_string(),
                requested_aspect_ratio: "16:9".to_string(),
                requested_count: 1,
                fixture_status: "requested".to_string(),
            },
        );
        assert!(live
            .unwrap_err()
            .to_string()
            .contains("deterministic_fixture"));

        let unsafe_provider = record_story_image_provider_request_envelope(
            &connection,
            StoryImageProviderRequestInput {
                brief_artifact_id: brief.artifact.id.clone(),
                idempotency_key: "hero-image-secret".to_string(),
                provider_name: "providerSecret".to_string(),
                model_hint: "gpt-image-2".to_string(),
                provider_mode: "deterministic_fixture".to_string(),
                requested_size: "1024x576".to_string(),
                requested_aspect_ratio: "16:9".to_string(),
                requested_count: 1,
                fixture_status: "requested".to_string(),
            },
        );
        assert!(unsafe_provider
            .unwrap_err()
            .to_string()
            .contains("private or unsupported markers"));

        record_story_image_provider_request_envelope(
            &connection,
            StoryImageProviderRequestInput {
                brief_artifact_id: brief.artifact.id.clone(),
                idempotency_key: "hero-image-conflict".to_string(),
                provider_name: "openai".to_string(),
                model_hint: "gpt-image-2".to_string(),
                provider_mode: "deterministic_fixture".to_string(),
                requested_size: "1024x576".to_string(),
                requested_aspect_ratio: "16:9".to_string(),
                requested_count: 1,
                fixture_status: "requested".to_string(),
            },
        )
        .unwrap();
        let conflict = record_story_image_provider_request_envelope(
            &connection,
            StoryImageProviderRequestInput {
                brief_artifact_id: brief.artifact.id,
                idempotency_key: "hero-image-conflict".to_string(),
                provider_name: "openai".to_string(),
                model_hint: "gpt-image-2".to_string(),
                provider_mode: "deterministic_fixture".to_string(),
                requested_size: "1024x576".to_string(),
                requested_aspect_ratio: "16:9".to_string(),
                requested_count: 2,
                fixture_status: "requested".to_string(),
            },
        );
        assert!(conflict
            .unwrap_err()
            .to_string()
            .contains("idempotency key conflicts"));
    }

    #[test]
    fn image_review_records_approved_candidate_without_live_provider() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let (_brief, candidate) = generated_candidate(&connection, "hero-image-review");

        let first = record_story_image_review_artifact(
            &connection,
            StoryImageReviewInput {
                candidate_artifact_id: candidate.artifact.id.clone(),
                brief_artifact_id: candidate.contract.brief_artifact_id.clone(),
                idempotency_key: "hero-image-review-approved".to_string(),
                fixture_id: "fixture:image.reviewAgainstBrief:approved".to_string(),
                review_state: StoryImageReviewState::Approved,
                reviewer_ref: "fixture:image-reviewer".to_string(),
                evidence_refs: vec!["review_fixture:approved".to_string()],
                limitations: vec!["Deterministic fixture review only.".to_string()],
                revision_guidance: None,
                visibility: "staff".to_string(),
            },
        )
        .unwrap();
        let repeated = record_story_image_review_artifact(
            &connection,
            StoryImageReviewInput {
                candidate_artifact_id: candidate.artifact.id.clone(),
                brief_artifact_id: candidate.contract.brief_artifact_id.clone(),
                idempotency_key: "hero-image-review-approved".to_string(),
                fixture_id: "fixture:image.reviewAgainstBrief:approved".to_string(),
                review_state: StoryImageReviewState::Approved,
                reviewer_ref: "fixture:image-reviewer".to_string(),
                evidence_refs: vec!["review_fixture:approved".to_string()],
                limitations: vec!["Deterministic fixture review only.".to_string()],
                revision_guidance: None,
                visibility: "staff".to_string(),
            },
        )
        .unwrap();

        assert_eq!(first.artifact.id, repeated.artifact.id);
        assert_eq!(
            first.artifact.artifact_kind,
            STORY_IMAGE_REVIEW_ARTIFACT_KIND
        );
        assert_eq!(first.artifact.status, "approved");
        assert_eq!(
            first.artifact.health_status.as_deref(),
            Some("review_approved")
        );
        assert_eq!(first.contract.method, "image.reviewAgainstBrief");
        assert_eq!(first.contract.review_state, StoryImageReviewState::Approved);
        assert_eq!(
            first.contract.publication_effect,
            "eligible_for_public_derivative"
        );
        assert_eq!(first.contract.memory_effect, "review_evidence_only");
        assert!(!first.contract.live_provider_called);
        assert!(first
            .contract
            .evidence_refs
            .iter()
            .any(|reference| reference == &candidate.artifact.id));

        let serialized = serde_json::to_string(&first).unwrap();
        assert!(!serialized.contains("liveProviderSucceeded"));
        assert!(!serialized.contains("providerSecret"));
        assert!(!serialized.contains("promptInternal"));

        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE artifact_kind = ?1",
                [STORY_IMAGE_REVIEW_ARTIFACT_KIND],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn image_review_rejects_unknown_mismatched_and_unsafe_inputs() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let (brief, candidate) = generated_candidate(&connection, "hero-image-review-negative");
        let other_brief = prepare_story_image_brief_artifacts(
            &connection,
            &deck_with_slide("proof", "Proof", "A second public proof scene."),
        )
        .unwrap()
        .remove(0);

        let unknown = record_story_image_review_artifact(
            &connection,
            StoryImageReviewInput {
                candidate_artifact_id: "missing_candidate".to_string(),
                brief_artifact_id: brief.artifact.id.clone(),
                idempotency_key: "unknown-review".to_string(),
                fixture_id: "fixture:image.reviewAgainstBrief:approved".to_string(),
                review_state: StoryImageReviewState::Approved,
                reviewer_ref: "fixture:image-reviewer".to_string(),
                evidence_refs: vec!["review_fixture:approved".to_string()],
                limitations: vec![],
                revision_guidance: None,
                visibility: "staff".to_string(),
            },
        );
        assert!(unknown
            .unwrap_err()
            .to_string()
            .contains("known generated image candidate artifact"));

        let mismatch = record_story_image_review_artifact(
            &connection,
            StoryImageReviewInput {
                candidate_artifact_id: candidate.artifact.id.clone(),
                brief_artifact_id: other_brief.artifact.id,
                idempotency_key: "mismatched-review".to_string(),
                fixture_id: "fixture:image.reviewAgainstBrief:approved".to_string(),
                review_state: StoryImageReviewState::Approved,
                reviewer_ref: "fixture:image-reviewer".to_string(),
                evidence_refs: vec!["review_fixture:approved".to_string()],
                limitations: vec![],
                revision_guidance: None,
                visibility: "staff".to_string(),
            },
        );
        assert!(mismatch
            .unwrap_err()
            .to_string()
            .contains("source brief must match"));

        let unsafe_review = record_story_image_review_artifact(
            &connection,
            StoryImageReviewInput {
                candidate_artifact_id: candidate.artifact.id,
                brief_artifact_id: brief.artifact.id,
                idempotency_key: "unsafe-review".to_string(),
                fixture_id: "fixture:image.reviewAgainstBrief:approved".to_string(),
                review_state: StoryImageReviewState::Approved,
                reviewer_ref: "providerSecret".to_string(),
                evidence_refs: vec!["review_fixture:approved".to_string()],
                limitations: vec!["liveProviderSucceeded".to_string()],
                revision_guidance: None,
                visibility: "staff".to_string(),
            },
        );
        assert!(unsafe_review
            .unwrap_err()
            .to_string()
            .contains("private or unsupported markers"));
    }

    #[test]
    fn image_review_revision_and_rejection_do_not_mark_publishable() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let (_brief, candidate) = generated_candidate(&connection, "hero-image-review-revise");

        let revision = record_story_image_review_artifact(
            &connection,
            StoryImageReviewInput {
                candidate_artifact_id: candidate.artifact.id.clone(),
                brief_artifact_id: candidate.contract.brief_artifact_id.clone(),
                idempotency_key: "hero-image-review-revision".to_string(),
                fixture_id: "fixture:image.reviewAgainstBrief:revision".to_string(),
                review_state: StoryImageReviewState::NeedsRevision,
                reviewer_ref: "fixture:image-reviewer".to_string(),
                evidence_refs: vec!["review_fixture:revision".to_string()],
                limitations: vec![],
                revision_guidance: Some("Use a calmer public-safe composition.".to_string()),
                visibility: "staff".to_string(),
            },
        )
        .unwrap();
        assert_eq!(revision.artifact.status, "needs_revision");
        assert_eq!(revision.contract.publication_effect, "not_publishable");
        assert_eq!(
            revision.contract.revision_guidance.as_deref(),
            Some("Use a calmer public-safe composition.")
        );

        let rejected = record_story_image_review_artifact(
            &connection,
            StoryImageReviewInput {
                candidate_artifact_id: candidate.artifact.id,
                brief_artifact_id: candidate.contract.brief_artifact_id,
                idempotency_key: "hero-image-review-rejected".to_string(),
                fixture_id: "fixture:image.reviewAgainstBrief:rejected".to_string(),
                review_state: StoryImageReviewState::Rejected,
                reviewer_ref: "fixture:image-reviewer".to_string(),
                evidence_refs: vec!["review_fixture:rejected".to_string()],
                limitations: vec!["Candidate does not match brief.".to_string()],
                revision_guidance: None,
                visibility: "staff".to_string(),
            },
        )
        .unwrap();
        assert_eq!(rejected.artifact.status, "rejected");
        assert_eq!(rejected.contract.publication_effect, "not_publishable");
        assert!(rejected
            .contract
            .limitations
            .iter()
            .any(|limitation| limitation.contains("does not match")));
    }

    #[test]
    fn image_review_idempotency_rejects_conflicting_input() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let (_brief, candidate) = generated_candidate(&connection, "hero-image-review-conflict");

        record_story_image_review_artifact(
            &connection,
            StoryImageReviewInput {
                candidate_artifact_id: candidate.artifact.id.clone(),
                brief_artifact_id: candidate.contract.brief_artifact_id.clone(),
                idempotency_key: "hero-image-review-conflict".to_string(),
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
        let conflict = record_story_image_review_artifact(
            &connection,
            StoryImageReviewInput {
                candidate_artifact_id: candidate.artifact.id,
                brief_artifact_id: candidate.contract.brief_artifact_id,
                idempotency_key: "hero-image-review-conflict".to_string(),
                fixture_id: "fixture:image.reviewAgainstBrief:approved".to_string(),
                review_state: StoryImageReviewState::Rejected,
                reviewer_ref: "fixture:image-reviewer".to_string(),
                evidence_refs: vec!["review_fixture:approved".to_string()],
                limitations: vec![],
                revision_guidance: None,
                visibility: "staff".to_string(),
            },
        );
        assert!(conflict
            .unwrap_err()
            .to_string()
            .contains("idempotency key conflicts"));
    }

    fn deck_with_slide(slide_id: &str, title: &str, body: &str) -> HomepageStoryDeckResponse {
        HomepageStoryDeckResponse {
            profile: HomepageStoryProfile {
                positioning: "A practical answer to enshittification.".to_string(),
                audience: Some("Solopreneurs".to_string()),
                primary_cta: None,
                evidence_refs: vec!["business_fact:homepage.profile".to_string()],
                limitations: vec![],
            },
            deck: HomepageNarrativeDeck {
                deck_id: "homepage.story.v1".to_string(),
                version: 1,
                surface: "homepage".to_string(),
                slides: vec![HomepageNarrativeSlide {
                    slide_id: slide_id.to_string(),
                    section_id: slide_id.to_string(),
                    order: 1,
                    title: title.to_string(),
                    body: body.to_string(),
                    copy_slots: vec![],
                    cta_refs: vec![],
                    evidence_refs: vec!["business_fact:homepage.slides.hero".to_string()],
                    limitations: vec![],
                    motion_profile: "cinematic".to_string(),
                    reduced_motion_fallback: body.to_string(),
                    image_brief_method: Some("homepage.prepare_image_briefs".to_string()),
                }],
                evidence_refs: vec!["artifact:storyboard".to_string()],
                limitations: vec![],
            },
            readiness: PublicSurfaceReadiness {
                surface: "homepage.story".to_string(),
                ready: true,
                fact_count: 1,
                missing: vec![],
            },
            refresh: HomepageStoryRefreshContract {
                manual_refresh_supported: true,
                scheduled_refresh_supported: true,
                image_brief_method: "homepage.prepare_image_briefs".to_string(),
                live_provider_required: false,
                limitations: vec![],
            },
        }
    }

    fn generated_candidate(
        connection: &Connection,
        idempotency_key: &str,
    ) -> (StoryImageBriefArtifact, GeneratedImageCandidateArtifact) {
        let deck = deck_with_slide("hero", "Studio Ordo", "A public proof scene.");
        let brief = prepare_story_image_brief_artifacts(connection, &deck)
            .unwrap()
            .remove(0);
        let outcome = record_story_image_provider_request_envelope(
            connection,
            StoryImageProviderRequestInput {
                brief_artifact_id: brief.artifact.id.clone(),
                idempotency_key: idempotency_key.to_string(),
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
        (brief, outcome.candidates.into_iter().next().unwrap())
    }
}
