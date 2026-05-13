use anyhow::{bail, ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::artifacts::{
    add_artifact_version, link_artifact, record_artifact, stage_deliverable, ArtifactInput,
    ArtifactLinkInput, DeliverableInput,
};
use crate::backups::core::{
    insert_job_artifact, mark_job_failed, mark_job_succeeded, mark_task_running,
    mark_task_succeeded, run_task, set_job_running,
};
use crate::events::{append_realtime_event, system_event};
use crate::kernel::create_job_from_template;
use crate::templates::require_builtin_template;

pub const PROMO_VIDEO_PACKAGE_TEMPLATE_ID: &str = "studio.promo_video.package";

const DEFAULT_DURATION_SECONDS: u32 = 20;
const MIN_DURATION_SECONDS: u32 = 10;
const MAX_DURATION_SECONDS: u32 = 30;
const TARGET_ASPECT_RATIO: &str = "9:16";
const PACING_WORDS_PER_MINUTE: u32 = 150;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromoVideoPackageRequest {
    pub title: Option<String>,
    pub brief: String,
    pub audience: Option<String>,
    pub offer_id: Option<String>,
    pub duration_seconds: Option<u32>,
    pub platforms: Option<Vec<String>>,
    pub aspect_ratio: Option<String>,
    pub evidence_refs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromoVideoPackageReviewRequest {
    pub decision: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromoVideoPackageResponse {
    pub package: PromoVideoPackageView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromoVideoPackageReviewResponse {
    pub artifact_id: String,
    pub status: String,
    pub review_state: String,
    pub event_cursor: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoVideoPackageView {
    pub artifact_id: String,
    pub job_id: String,
    pub deliverable_id: String,
    pub artifact_version_id: String,
    pub status: String,
    pub review_state: String,
    pub publication_state: String,
    pub document: PromoVideoPackageDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoVideoPackageDocument {
    pub schema_version: String,
    pub title: String,
    pub brief: String,
    pub audience: String,
    pub duration_seconds: u32,
    pub pacing_words_per_minute: u32,
    pub target_aspect_ratio: String,
    pub platforms: Vec<PromoPlatformTarget>,
    pub script: PromoScript,
    pub media_plan: Vec<PromoMediaScene>,
    pub audio: PromoAudioPlan,
    pub captions: Vec<PromoCaptionCue>,
    pub metadata: PromoPublicationMetadata,
    pub review: PromoReviewPlan,
    pub publication: PromoPublicationPlan,
    pub limitations: Vec<String>,
    pub provenance: PromoPackageProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoPlatformTarget {
    pub name: String,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoScript {
    pub narration: String,
    pub estimated_words: usize,
    pub estimated_duration_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoMediaScene {
    pub index: usize,
    pub start_second: u32,
    pub end_second: u32,
    pub asset_kind: String,
    pub asset_status: String,
    pub prompt: String,
    pub alt_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoAudioPlan {
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoCaptionCue {
    pub index: usize,
    pub start_second: u32,
    pub end_second: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoPublicationMetadata {
    pub title: String,
    pub description: String,
    pub hashtags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoReviewPlan {
    pub state: String,
    pub supported_actions: Vec<String>,
    pub deferred_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoPublicationPlan {
    pub state: String,
    pub external_publishing: String,
    pub platform_analytics: String,
    pub owner_instruction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromoPackageProvenance {
    pub generated_by: String,
    pub job_id: String,
    pub template_id: String,
    pub origin: String,
    pub actor_id: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone)]
struct NormalizedPromoVideoPackageRequest {
    title: String,
    brief: String,
    audience: String,
    offer_id: Option<String>,
    duration_seconds: u32,
    target_aspect_ratio: String,
    platforms: Vec<PromoPlatformTarget>,
    limitations: Vec<String>,
    evidence_refs: Vec<String>,
}

pub fn create_promo_video_package(
    db_path: &Path,
    request: PromoVideoPackageRequest,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<PromoVideoPackageResponse> {
    let mut connection = Connection::open(db_path)?;
    create_promo_video_package_with_connection(&mut connection, request, origin, actor_id)
}

pub(crate) fn create_promo_video_package_with_connection(
    connection: &mut Connection,
    request: PromoVideoPackageRequest,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<PromoVideoPackageResponse> {
    let normalized = normalize_promo_video_package_request(request)?;
    let template = require_builtin_template(PROMO_VIDEO_PACKAGE_TEMPLATE_ID)?;
    let job_id = create_job_from_template(
        connection,
        &template,
        origin,
        actor_id,
        json!({
            "title": normalized.title,
            "durationSeconds": normalized.duration_seconds,
            "targetAspectRatio": normalized.target_aspect_ratio,
            "platforms": normalized.platforms.iter().map(|platform| platform.name.clone()).collect::<Vec<_>>(),
            "offerId": normalized.offer_id,
            "generator": "deterministic",
            "externalPublishing": "not_configured",
        }),
    )?;

    match complete_promo_video_package_job(connection, &job_id, normalized, origin, actor_id) {
        Ok(package) => Ok(PromoVideoPackageResponse { package }),
        Err(error) => {
            mark_job_failed(connection, &job_id, &error.to_string())?;
            Err(error)
        }
    }
}

pub fn review_promo_video_package(
    db_path: &Path,
    artifact_id: &str,
    request: PromoVideoPackageReviewRequest,
    actor_id: Option<&str>,
) -> Result<PromoVideoPackageReviewResponse> {
    let connection = Connection::open(db_path)?;
    review_promo_video_package_with_connection(&connection, artifact_id, request, actor_id)
}

pub(crate) fn review_promo_video_package_with_connection(
    connection: &Connection,
    artifact_id: &str,
    request: PromoVideoPackageReviewRequest,
    actor_id: Option<&str>,
) -> Result<PromoVideoPackageReviewResponse> {
    let (decision_status, review_state) = normalize_review_decision(&request.decision)?;
    ensure_promo_artifact(connection, artifact_id)?;
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE artifacts SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![decision_status, now, artifact_id],
    )?;
    connection.execute(
        "UPDATE artifact_deliverables
         SET status = ?1, summary = ?2, updated_at = ?3, published_at = NULL
         WHERE artifact_id = ?4",
        params![
            if decision_status == "approved" {
                "staged_manual_approved"
            } else {
                "revision_requested"
            },
            if decision_status == "approved" {
                "Owner approved the local staged promo package for manual export."
            } else {
                "Owner requested a revision before manual export."
            },
            now,
            artifact_id,
        ],
    )?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "studio.promo_video.reviewed",
            json!({
                "artifactId": artifact_id,
                "status": decision_status,
                "reviewState": review_state,
                "actorId": actor_id,
                "reasonLength": request.reason.as_deref().unwrap_or("").trim().len(),
                "externalPublishing": "not_performed",
            }),
        ),
    )?;
    Ok(PromoVideoPackageReviewResponse {
        artifact_id: artifact_id.to_string(),
        status: decision_status.to_string(),
        review_state: review_state.to_string(),
        event_cursor: event.cursor.unwrap_or_default(),
    })
}

fn complete_promo_video_package_job(
    connection: &Connection,
    job_id: &str,
    normalized: NormalizedPromoVideoPackageRequest,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<PromoVideoPackageView> {
    set_job_running(connection, job_id)?;
    run_task(
        connection,
        job_id,
        "brief.validate",
        json!({
            "durationSeconds": normalized.duration_seconds,
            "targetAspectRatio": normalized.target_aspect_ratio,
            "evidenceRefs": normalized.evidence_refs,
            "valid": true,
        }),
    )?;

    let script = build_script(&normalized);
    run_task(
        connection,
        job_id,
        "script.draft",
        json!({
            "estimatedWords": script.estimated_words,
            "estimatedDurationSeconds": script.estimated_duration_seconds,
            "providerCall": "not_performed",
        }),
    )?;

    let media_plan = build_media_plan(&normalized);
    run_task(
        connection,
        job_id,
        "media.plan",
        json!({
            "sceneCount": media_plan.len(),
            "assetStatus": "prompt_only",
            "rendering": "not_performed",
        }),
    )?;

    let captions = build_captions(&script, &media_plan);
    run_task(
        connection,
        job_id,
        "captions.prepare",
        json!({ "captionCount": captions.len() }),
    )?;

    mark_task_running(connection, job_id, "package.stage")?;
    let mut evidence_refs = normalized.evidence_refs.clone();
    evidence_refs.push(format!("job:{job_id}"));
    let document = PromoVideoPackageDocument {
        schema_version: "1".to_string(),
        title: normalized.title.clone(),
        brief: normalized.brief.clone(),
        audience: normalized.audience.clone(),
        duration_seconds: normalized.duration_seconds,
        pacing_words_per_minute: PACING_WORDS_PER_MINUTE,
        target_aspect_ratio: normalized.target_aspect_ratio.clone(),
        platforms: normalized.platforms.clone(),
        script,
        media_plan,
        audio: PromoAudioPlan {
            status: "placeholder".to_string(),
            reason: "Audio generation is deferred; use the script at 150 WPM for manual recording or a future governed audio tool.".to_string(),
        },
        captions,
        metadata: build_publication_metadata(&normalized),
        review: PromoReviewPlan {
            state: "ready_for_review".to_string(),
            supported_actions: vec!["review_artifact".to_string()],
            deferred_actions: vec![
                "generate_media".to_string(),
                "publish_external".to_string(),
                "platform_analytics".to_string(),
            ],
        },
        publication: PromoPublicationPlan {
            state: "staged_manual".to_string(),
            external_publishing: "not_configured".to_string(),
            platform_analytics: "not_available".to_string(),
            owner_instruction:
                "Download or copy the staged script, prompts, captions, and metadata for manual production and publication.".to_string(),
        },
        limitations: normalized.limitations.clone(),
        provenance: PromoPackageProvenance {
            generated_by: "studio.promo_video.package".to_string(),
            job_id: job_id.to_string(),
            template_id: PROMO_VIDEO_PACKAGE_TEMPLATE_ID.to_string(),
            origin: origin.to_string(),
            actor_id: actor_id.map(str::to_string),
            evidence_refs: evidence_refs.clone(),
        },
    };
    let document_value = serde_json::to_value(&document)?;
    let content_hash = content_hash(&document_value)?;
    let storage_uri = format!("ordo://studio/promos/{job_id}/package.json");
    let (artifact, _) = record_artifact(
        connection,
        ArtifactInput {
            artifact_kind: "studio.promo_video.package".to_string(),
            title: document.title.clone(),
            status: "ready_for_review".to_string(),
            visibility_ceiling: "staff".to_string(),
            summary: format!(
                "{} second staged vertical promo package. External publishing is manual.",
                document.duration_seconds
            ),
            source_kind: normalized.offer_id.as_ref().map(|_| "offer".to_string()),
            source_id: normalized.offer_id.clone(),
            evidence_refs: evidence_refs.clone(),
            provenance: json!({
                "generatedBy": "studio.promo_video.package",
                "jobId": job_id,
                "templateId": PROMO_VIDEO_PACKAGE_TEMPLATE_ID,
                "externalPublishing": "not_performed",
                "platformAnalytics": "not_available",
            }),
            content_hash: content_hash.clone(),
            storage_uri: Some(storage_uri.clone()),
            health_status: Some("staged_manual".to_string()),
            created_by_job_id: Some(job_id.to_string()),
        },
    )?;
    let version = add_artifact_version(
        connection,
        &artifact.id,
        &content_hash,
        Some(&storage_uri),
        json!({
            "package": document_value,
            "reviewState": "ready_for_review",
            "publicationState": "staged_manual",
            "externalPublishing": "not_configured",
            "platformAnalytics": "not_available",
        }),
    )?;
    let (job_link, _) = link_artifact(
        connection,
        &artifact.id,
        ArtifactLinkInput {
            link_kind: "production".to_string(),
            source_kind: "job".to_string(),
            source_id: job_id.to_string(),
            relation: "produced_by".to_string(),
            evidence_refs: vec![format!("job:{job_id}")],
            provenance: json!({ "generatedBy": "studio.promo_video.package" }),
        },
    )?;
    if let Some(offer_id) = normalized.offer_id.as_deref() {
        let _ = link_artifact(
            connection,
            &artifact.id,
            ArtifactLinkInput {
                link_kind: "source".to_string(),
                source_kind: "offer".to_string(),
                source_id: offer_id.to_string(),
                relation: "promotes".to_string(),
                evidence_refs: evidence_refs.clone(),
                provenance: json!({ "generatedBy": "studio.promo_video.package" }),
            },
        )?;
    }
    let (deliverable, _) = stage_deliverable(
        connection,
        &artifact.id,
        DeliverableInput {
            client_label: "Manual promo package".to_string(),
            status: "staged_manual".to_string(),
            visibility: "owner".to_string(),
            summary:
                "Local staged package with script, prompts, captions, metadata, and limitations; no external publication performed."
                    .to_string(),
        },
    )?;
    insert_job_artifact(
        connection,
        job_id,
        Some("package.stage"),
        "studio.promo_video.package",
        &storage_uri,
        "Promo video package",
        json!({
            "artifactId": artifact.id,
            "artifactVersionId": version.id,
            "deliverableId": deliverable.id,
            "artifactLinkId": job_link.id,
            "publicationState": "staged_manual",
            "externalPublishing": "not_performed",
        }),
    )?;
    append_realtime_event(
        connection,
        &system_event(
            "studio.promo_video.package.staged",
            json!({
                "artifactId": artifact.id,
                "jobId": job_id,
                "deliverableId": deliverable.id,
                "durationSeconds": document.duration_seconds,
                "publicationState": "staged_manual",
                "externalPublishing": "not_performed",
            }),
        ),
    )?;
    mark_task_succeeded(
        connection,
        job_id,
        "package.stage",
        json!({
            "artifactId": artifact.id,
            "versionId": version.id,
            "deliverableId": deliverable.id,
            "publicationState": "staged_manual",
        }),
    )?;
    mark_job_succeeded(
        connection,
        job_id,
        json!({
            "artifactId": artifact.id,
            "deliverableId": deliverable.id,
            "status": "ready_for_review",
        }),
    )?;

    Ok(PromoVideoPackageView {
        artifact_id: artifact.id,
        job_id: job_id.to_string(),
        deliverable_id: deliverable.id,
        artifact_version_id: version.id,
        status: "ready_for_review".to_string(),
        review_state: "ready_for_review".to_string(),
        publication_state: "staged_manual".to_string(),
        document,
    })
}

fn normalize_promo_video_package_request(
    request: PromoVideoPackageRequest,
) -> Result<NormalizedPromoVideoPackageRequest> {
    let brief = request.brief.trim().to_string();
    ensure!(!brief.is_empty(), "promo brief is required");
    let duration_seconds = request.duration_seconds.unwrap_or(DEFAULT_DURATION_SECONDS);
    ensure!(
        (MIN_DURATION_SECONDS..=MAX_DURATION_SECONDS).contains(&duration_seconds),
        "promo duration must be between 10 and 30 seconds"
    );
    let title =
        non_empty_optional(request.title).unwrap_or_else(|| "OrdoStudio pilot promo".to_string());
    let audience =
        non_empty_optional(request.audience).unwrap_or_else(|| "solopreneurs".to_string());
    let offer_id = non_empty_optional(request.offer_id);
    let requested_aspect = non_empty_optional(request.aspect_ratio);
    let mut limitations = vec![
        "This package is deterministic and local; no live LLM, audio, image, or render provider was called.".to_string(),
        "External publishing, OAuth, and platform analytics are not configured in this slice.".to_string(),
        "Audio and image assets are placeholders or prompts until governed media tools are connected.".to_string(),
    ];
    if let Some(aspect_ratio) = requested_aspect.as_deref() {
        if aspect_ratio != TARGET_ASPECT_RATIO {
            limitations.push(format!(
                "Requested aspect ratio `{aspect_ratio}` is unsupported; the staged package remains 9:16 vertical."
            ));
        }
    }
    let mut platforms =
        normalize_platforms(request.platforms.unwrap_or_default(), &mut limitations);
    if platforms.is_empty() {
        platforms = normalize_platforms(
            vec!["TikTok".to_string(), "YouTube Shorts".to_string()],
            &mut limitations,
        );
    }
    let mut evidence_refs: Vec<String> = request
        .evidence_refs
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    evidence_refs.push("studio_promo_request:deterministic".to_string());
    evidence_refs.sort();
    evidence_refs.dedup();
    Ok(NormalizedPromoVideoPackageRequest {
        title,
        brief,
        audience,
        offer_id,
        duration_seconds,
        target_aspect_ratio: TARGET_ASPECT_RATIO.to_string(),
        platforms,
        limitations,
        evidence_refs,
    })
}

fn normalize_platforms(
    requested_platforms: Vec<String>,
    limitations: &mut Vec<String>,
) -> Vec<PromoPlatformTarget> {
    let mut platforms = Vec::new();
    for requested in requested_platforms {
        let trimmed = requested.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_ascii_lowercase().replace(['_', '-'], " ");
        let supported = match key.as_str() {
            "tiktok" | "tik tok" => Some("TikTok"),
            "youtube shorts" | "youtube" | "yt shorts" => Some("YouTube Shorts"),
            "instagram reels" | "instagram" | "reels" => Some("Instagram Reels"),
            _ => None,
        };
        match supported {
            Some(name) => platforms.push(PromoPlatformTarget {
                name: name.to_string(),
                status: "manual_staged".to_string(),
                reason: "Package metadata is prepared for manual publication; no platform API was called.".to_string(),
            }),
            None => {
                platforms.push(PromoPlatformTarget {
                    name: trimmed.to_string(),
                    status: "unsupported_deferred".to_string(),
                    reason:
                        "No governed publishing or analytics adapter is configured for this platform.".to_string(),
                });
                limitations.push(format!(
                    "Requested platform `{trimmed}` is unsupported and remains deferred."
                ));
            }
        }
    }
    platforms.sort_by(|left, right| left.name.cmp(&right.name));
    platforms.dedup_by(|left, right| left.name == right.name);
    platforms
}

fn build_script(request: &NormalizedPromoVideoPackageRequest) -> PromoScript {
    let max_words = ((request.duration_seconds * PACING_WORDS_PER_MINUTE) / 60) as usize;
    let source = format!(
        "{}: {} For {}, open Ordo, review the offer, and decide whether a 30-day trial should run the next growth job.",
        request.title,
        request.brief,
        request.audience
    );
    let narration = trim_words(&source, max_words.max(18));
    PromoScript {
        estimated_words: narration.split_whitespace().count(),
        estimated_duration_seconds: request.duration_seconds,
        narration,
    }
}

fn build_media_plan(request: &NormalizedPromoVideoPackageRequest) -> Vec<PromoMediaScene> {
    let scene_count = ((request.duration_seconds + 5) / 6).clamp(2, 5);
    let mut scenes = Vec::new();
    for index in 0..scene_count {
        let start_second = (request.duration_seconds * index) / scene_count;
        let end_second = if index + 1 == scene_count {
            request.duration_seconds
        } else {
            (request.duration_seconds * (index + 1)) / scene_count
        };
        scenes.push(PromoMediaScene {
            index: (index + 1) as usize,
            start_second,
            end_second,
            asset_kind: "image_prompt".to_string(),
            asset_status: "prompt_only".to_string(),
            prompt: format!(
                "Create a 9:16 vertical image for `{}` aimed at {}. Scene {} should support this brief: {}. Avoid platform logos and leave caption-safe space.",
                request.title,
                request.audience,
                index + 1,
                trim_words(&request.brief, 22)
            ),
            alt_text: format!(
                "Prompt-only vertical visual plan scene {} for {}.",
                index + 1,
                request.title
            ),
        });
    }
    scenes
}

fn build_captions(script: &PromoScript, media_plan: &[PromoMediaScene]) -> Vec<PromoCaptionCue> {
    let words: Vec<&str> = script.narration.split_whitespace().collect();
    let cue_count = media_plan.len().max(1);
    media_plan
        .iter()
        .enumerate()
        .map(|(index, scene)| {
            let start = (words.len() * index) / cue_count;
            let end = (words.len() * (index + 1)) / cue_count;
            PromoCaptionCue {
                index: index + 1,
                start_second: scene.start_second,
                end_second: scene.end_second,
                text: words[start..end].join(" "),
            }
        })
        .collect()
}

fn build_publication_metadata(
    request: &NormalizedPromoVideoPackageRequest,
) -> PromoPublicationMetadata {
    PromoPublicationMetadata {
        title: trim_words(&request.title, 12),
        description: trim_words(
            &format!(
                "{} Built as a local staged OrdoStudio promo package for {}.",
                request.brief, request.audience
            ),
            34,
        ),
        hashtags: vec![
            "OrdoStudio".to_string(),
            "SoloFounder".to_string(),
            "AIAppliance".to_string(),
        ],
    }
}

fn ensure_promo_artifact(connection: &Connection, artifact_id: &str) -> Result<()> {
    let artifact_kind: Option<String> = connection
        .query_row(
            "SELECT artifact_kind FROM artifacts WHERE id = ?1",
            [artifact_id],
            |row| row.get(0),
        )
        .optional()?;
    match artifact_kind.as_deref() {
        Some("studio.promo_video.package") => Ok(()),
        Some(_) => bail!("artifact is not a promo video package"),
        None => bail!("promo video package artifact not found"),
    }
}

fn normalize_review_decision(decision: &str) -> Result<(&'static str, &'static str)> {
    match decision.trim().to_ascii_lowercase().as_str() {
        "approve" | "approved" => Ok(("approved", "approved")),
        "request_revision" | "revision_requested" | "revise" => {
            Ok(("revision_requested", "revision_requested"))
        }
        _ => bail!("unsupported promo review decision"),
    }
}

fn non_empty_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn trim_words(value: &str, max_words: usize) -> String {
    value
        .split_whitespace()
        .take(max_words)
        .collect::<Vec<_>>()
        .join(" ")
}

fn content_hash(value: &Value) -> Result<String> {
    let bytes = serde_json::to_vec(value)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::list_deliverables_for_artifact;
    use crate::schema::init_database;
    use crate::surface_work_items::{
        list_surface_work_items, SurfaceWorkItemQuery, SurfaceWorkItemViewer,
    };
    use tempfile::TempDir;

    #[test]
    fn promo_package_rejects_invalid_duration_before_side_effects() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        let result = create_promo_video_package(
            &db_path,
            PromoVideoPackageRequest {
                title: Some("Bad duration".to_string()),
                brief: "Make a promo".to_string(),
                audience: None,
                offer_id: None,
                duration_seconds: Some(45),
                platforms: None,
                aspect_ratio: None,
                evidence_refs: None,
            },
            "test",
            Some("actor_owner"),
        );

        assert!(result.is_err());
        let connection = Connection::open(&db_path).unwrap();
        let job_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM jobs", [], |row| row.get(0))
            .unwrap();
        let artifact_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM artifacts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(job_count, 0);
        assert_eq!(artifact_count, 0);
    }

    #[test]
    fn promo_package_creates_staged_artifact_visible_to_studio() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        let response = create_promo_video_package(
            &db_path,
            PromoVideoPackageRequest {
                title: Some("NYC founder trial".to_string()),
                brief:
                    "Show that Ordo turns a meetup conversation into a governed 30-day trial path."
                        .to_string(),
                audience: Some("solo founders at NYC meetups".to_string()),
                offer_id: Some("offer_nyc_trial".to_string()),
                duration_seconds: Some(18),
                platforms: Some(vec!["TikTok".to_string(), "YouTube Shorts".to_string()]),
                aspect_ratio: Some("9:16".to_string()),
                evidence_refs: Some(vec!["offer:offer_nyc_trial".to_string()]),
            },
            "test",
            Some("actor_owner"),
        )
        .unwrap();

        assert_eq!(response.package.status, "ready_for_review");
        assert_eq!(response.package.publication_state, "staged_manual");
        assert_eq!(response.package.document.duration_seconds, 18);
        assert_eq!(response.package.document.target_aspect_ratio, "9:16");
        assert_eq!(response.package.document.audio.status, "placeholder");
        assert!(response
            .package
            .document
            .limitations
            .iter()
            .any(|limitation| limitation.contains("External publishing")));
        assert_eq!(response.package.document.media_plan.len(), 3);

        let connection = Connection::open(&db_path).unwrap();
        let job_status: String = connection
            .query_row(
                "SELECT status FROM jobs WHERE id = ?1",
                [&response.package.job_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(job_status, "succeeded");
        let staged_events: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM realtime_events WHERE event_type = 'studio.promo_video.package.staged'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(staged_events, 1);

        let deliverables =
            list_deliverables_for_artifact(&connection, &response.package.artifact_id).unwrap();
        assert_eq!(deliverables.len(), 1);
        assert_eq!(deliverables[0].status, "staged_manual");
        assert!(deliverables[0].published_at.is_none());

        let studio_items = list_surface_work_items(
            &db_path,
            SurfaceWorkItemQuery {
                viewer: SurfaceWorkItemViewer::Staff,
                surface_kind: Some("studio".to_string()),
                room_kind: Some("artifacts".to_string()),
                actor_id: None,
                connection_id: None,
                limit: Some(20),
            },
        )
        .unwrap();
        let artifact_item = studio_items
            .items
            .iter()
            .find(|item| item.object_id == response.package.artifact_id)
            .unwrap();
        assert_eq!(artifact_item.status, "ready_for_review");
        assert!(artifact_item
            .actions
            .contains(&"review_artifact".to_string()));
        assert!(artifact_item
            .summary
            .contains("External publishing is manual"));
    }

    #[test]
    fn promo_package_marks_unsupported_platforms_and_aspects_as_limitations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        let response = create_promo_video_package(
            &db_path,
            PromoVideoPackageRequest {
                title: Some("Unsupported platform".to_string()),
                brief: "Explain the trial safely.".to_string(),
                audience: None,
                offer_id: None,
                duration_seconds: Some(12),
                platforms: Some(vec!["MarsFeed".to_string()]),
                aspect_ratio: Some("1:1".to_string()),
                evidence_refs: None,
            },
            "test",
            None,
        )
        .unwrap();

        assert_eq!(response.package.document.target_aspect_ratio, "9:16");
        assert_eq!(
            response.package.document.platforms[0].status,
            "unsupported_deferred"
        );
        assert!(response
            .package
            .document
            .limitations
            .iter()
            .any(|limitation| limitation.contains("MarsFeed")));
        assert!(response
            .package
            .document
            .limitations
            .iter()
            .any(|limitation| limitation.contains("aspect ratio")));
    }

    #[test]
    fn promo_review_updates_durable_artifact_state_without_publishing() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let response = create_promo_video_package(
            &db_path,
            PromoVideoPackageRequest {
                title: Some("Review me".to_string()),
                brief: "Create a promo package for review.".to_string(),
                audience: None,
                offer_id: None,
                duration_seconds: Some(10),
                platforms: None,
                aspect_ratio: None,
                evidence_refs: None,
            },
            "test",
            Some("actor_owner"),
        )
        .unwrap();

        let review = review_promo_video_package(
            &db_path,
            &response.package.artifact_id,
            PromoVideoPackageReviewRequest {
                decision: "request_revision".to_string(),
                reason: Some("Tone needs to be calmer.".to_string()),
            },
            Some("actor_owner"),
        )
        .unwrap();

        assert_eq!(review.status, "revision_requested");
        let connection = Connection::open(&db_path).unwrap();
        let artifact_status: String = connection
            .query_row(
                "SELECT status FROM artifacts WHERE id = ?1",
                [&response.package.artifact_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(artifact_status, "revision_requested");
        let deliverables =
            list_deliverables_for_artifact(&connection, &response.package.artifact_id).unwrap();
        assert_eq!(deliverables[0].status, "revision_requested");
        assert!(deliverables[0].published_at.is_none());
        let reviewed_events: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM realtime_events WHERE event_type = 'studio.promo_video.reviewed'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(reviewed_events, 1);
    }
}
