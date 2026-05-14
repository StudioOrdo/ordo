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
pub const STORY_GENERATED_IMAGE_CANDIDATE_ARTIFACT_KIND: &str = "story.generated_image_candidate";
const CONTRACT_SCHEMA_VERSION: &str = "ordo.story_image_artifact_contract.v1";
const DEFAULT_ASPECT_RATIO: &str = "16:9";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryImageBriefArtifact {
    pub artifact: ArtifactView,
    pub version: Option<ArtifactVersionView>,
    pub contract: StoryImageBriefContract,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedImageCandidateArtifact {
    pub artifact: ArtifactView,
    pub version: Option<ArtifactVersionView>,
    pub contract: GeneratedImageCandidateContract,
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
}
