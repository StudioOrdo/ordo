use anyhow::{bail, ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::artifacts::{
    add_artifact_version, load_artifact, record_artifact, ArtifactInput, ArtifactVersionView,
    ArtifactView,
};
use crate::content_analytics::{
    record_content_analytics_event, ContentAnalyticsEventInput, ContentAnalyticsEventKind,
    ContentAnalyticsEventView, ContentAnalyticsSourceStatus,
};
use crate::public_surfaces::{HomepageNarrativeSlide, HomepageStoryDeckResponse};
use crate::security::redaction;

pub const STORY_HOMEPAGE_PUBLISH_APPROVAL_PACKAGE_ARTIFACT_KIND: &str =
    "story.homepage_publish_approval_package";
const CONTRACT_SCHEMA_VERSION: &str = "ordo.story_homepage_publish_approval_package.v1";

#[derive(Debug, Clone)]
pub struct HomepagePublishApprovalInput {
    pub package_id: String,
    pub idempotency_key: String,
    pub deck: HomepageStoryDeckResponse,
    pub source_artifact_ids: Vec<String>,
    pub image_artifact_ids: Vec<String>,
    pub approval_state: String,
    pub approval_actor_id: String,
    pub approval_evidence_refs: Vec<String>,
    pub manual_publish_evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub workflow_compilation_id: Option<String>,
    pub job_id: Option<String>,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomepagePublishApprovalPackageContract {
    pub schema_version: String,
    pub package_id: String,
    pub idempotency_key: String,
    pub deck_id: String,
    pub deck_version: i64,
    pub surface: String,
    pub approval_state: String,
    pub local_publication_state: String,
    pub claim_validation: HomepagePublishClaimValidation,
    pub public_derivative: HomepagePublishPublicDerivative,
    pub source_artifact_refs: Vec<String>,
    pub image_artifact_refs: Vec<String>,
    pub approval_actor_ref: String,
    pub approval_evidence_refs: Vec<String>,
    pub manual_publish_evidence_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub workflow_compilation_id: Option<String>,
    pub job_id: Option<String>,
    pub occurred_at: Option<String>,
    pub analytics_event_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomepagePublishClaimValidation {
    pub status: String,
    pub claim_count: usize,
    pub unsupported_claim_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomepagePublishPublicDerivative {
    pub deck_id: String,
    pub surface: String,
    pub positioning: String,
    pub slide_count: usize,
    pub sections: Vec<HomepagePublishSectionDerivative>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub visibility: String,
    pub external_publishing_claimed: bool,
    pub memory_effect: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomepagePublishSectionDerivative {
    pub slide_id: String,
    pub section_id: String,
    pub order: i64,
    pub title: String,
    pub body: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HomepagePublishApprovalOutcome {
    pub package_artifact: ArtifactView,
    pub version: Option<ArtifactVersionView>,
    pub contract: HomepagePublishApprovalPackageContract,
    pub analytics_event: Option<ContentAnalyticsEventView>,
}

pub fn record_homepage_publish_approval_package(
    connection: &Connection,
    input: HomepagePublishApprovalInput,
) -> Result<HomepagePublishApprovalOutcome> {
    let mut contract = homepage_publish_approval_contract(connection, input)?;
    let contract_json = serde_json::to_value(&contract)?;
    let content_hash = stable_json_hash(&contract_json)?;

    if let Some(existing) =
        load_existing_publish_package(connection, &contract.package_id, &contract.idempotency_key)?
    {
        if existing.content_hash != content_hash {
            bail!("homepage publish approval idempotency key conflicts with a different input");
        }
        let analytics_event = record_publish_analytics_event(connection, &existing, &contract)?.0;
        contract.analytics_event_id = Some(analytics_event.id.clone());
        return Ok(HomepagePublishApprovalOutcome {
            package_artifact: existing,
            version: None,
            contract,
            analytics_event: Some(analytics_event),
        });
    }

    let (package_artifact, _) = record_artifact(
        connection,
        ArtifactInput {
            artifact_kind: STORY_HOMEPAGE_PUBLISH_APPROVAL_PACKAGE_ARTIFACT_KIND.to_string(),
            title: format!("Homepage publish approval package {}", contract.package_id),
            status: "published".to_string(),
            visibility_ceiling: "staff".to_string(),
            summary: format!(
                "Manual local publish approval package for homepage story deck {}.",
                contract.deck_id
            ),
            source_kind: Some("homepage_publish_approval_package".to_string()),
            source_id: Some(contract.idempotency_key.clone()),
            evidence_refs: contract.evidence_refs.clone(),
            provenance: json!({
                "schemaVersion": CONTRACT_SCHEMA_VERSION,
                "generatedBy": "publish.requestApproval",
                "approvalState": contract.approval_state,
                "localPublicationState": contract.local_publication_state,
                "publicDerivative": contract.public_derivative,
                "contract": contract,
                "externalPublishingClaimed": false,
                "automaticMemoryTruthPromotion": false,
            }),
            content_hash: content_hash.clone(),
            storage_uri: Some(format!(
                "ordo://artifacts/story-homepage-publish-approvals/{}",
                safe_identifier(&contract.idempotency_key)
            )),
            health_status: Some("local_publish_evidence_recorded".to_string()),
            created_by_job_id: None,
        },
    )?;
    let (analytics_event, _) =
        record_publish_analytics_event(connection, &package_artifact, &contract)?;
    contract.analytics_event_id = Some(analytics_event.id.clone());
    let version = add_artifact_version(
        connection,
        &package_artifact.id,
        &content_hash,
        package_artifact.storage_uri.as_deref(),
        json!({
            "schemaVersion": CONTRACT_SCHEMA_VERSION,
            "contract": contract,
            "analyticsEventId": analytics_event.id,
            "localPublishOnly": true,
            "externalPublishingClaimed": false,
            "liveAnalyticsClaimed": false,
        }),
    )?;

    Ok(HomepagePublishApprovalOutcome {
        package_artifact,
        version: Some(version),
        contract,
        analytics_event: Some(analytics_event),
    })
}

fn homepage_publish_approval_contract(
    connection: &Connection,
    input: HomepagePublishApprovalInput,
) -> Result<HomepagePublishApprovalPackageContract> {
    let package_id = safe_identifier(&input.package_id);
    ensure!(
        !package_id.is_empty(),
        "homepage publish package id is required"
    );
    let idempotency_key = safe_identifier(&input.idempotency_key);
    ensure!(
        !idempotency_key.is_empty(),
        "homepage publish approval idempotency key is required"
    );
    ensure!(
        input.approval_state == "approved",
        "homepage publish requires approved approval state"
    );
    ensure!(
        !input.approval_actor_id.trim().is_empty(),
        "homepage publish approval requires an approval actor"
    );
    ensure!(
        input.deck.readiness.ready && !input.deck.deck.slides.is_empty(),
        "homepage publish requires a ready public story deck"
    );

    let approval_evidence_refs = public_safe_refs(input.approval_evidence_refs);
    let manual_publish_evidence_refs = public_safe_refs(input.manual_publish_evidence_refs);
    ensure!(
        !approval_evidence_refs.is_empty(),
        "homepage publish requires approval evidence"
    );
    ensure!(
        !manual_publish_evidence_refs.is_empty(),
        "homepage publish requires local manual publish evidence"
    );

    let source_artifact_refs = artifact_refs(connection, input.source_artifact_ids)?;
    ensure!(
        !source_artifact_refs.is_empty(),
        "homepage publish approval package requires source artifact refs"
    );
    let image_artifact_refs = artifact_refs(connection, input.image_artifact_ids)?;
    let public_derivative =
        public_derivative_for_deck(&input.deck, image_artifact_refs.is_empty())?;
    let claim_validation = validate_deck_claims(&input.deck)?;
    ensure!(
        claim_validation.status == "supported",
        "homepage publish requires supported public claim evidence"
    );

    let mut limitations = public_safe_strings(input.limitations);
    limitations.extend(public_derivative.limitations.clone());
    if image_artifact_refs.is_empty() {
        limitations.push(
            "Homepage publish package has no generated image candidates; missing images remain an explicit limitation."
                .to_string(),
        );
    }
    limitations.push(
        "Manual local publication evidence is recorded; no external platform publishing, live analytics, uptime, or provider behavior is claimed."
            .to_string(),
    );
    limitations = stable_strings(limitations);

    let evidence_refs = public_safe_refs(
        input
            .deck
            .deck
            .evidence_refs
            .iter()
            .cloned()
            .chain(input.deck.profile.evidence_refs.clone())
            .chain(
                input
                    .deck
                    .deck
                    .slides
                    .iter()
                    .flat_map(|slide| slide.evidence_refs.clone()),
            )
            .chain(source_artifact_refs.clone())
            .chain(image_artifact_refs.clone())
            .chain(approval_evidence_refs.clone())
            .chain(manual_publish_evidence_refs.clone())
            .collect(),
    );
    ensure!(
        !evidence_refs.is_empty(),
        "homepage publish approval package requires evidence refs"
    );

    Ok(HomepagePublishApprovalPackageContract {
        schema_version: CONTRACT_SCHEMA_VERSION.to_string(),
        package_id,
        idempotency_key,
        deck_id: safe_identifier(&input.deck.deck.deck_id),
        deck_version: input.deck.deck.version,
        surface: safe_identifier(&input.deck.deck.surface),
        approval_state: "approved".to_string(),
        local_publication_state: "published_local".to_string(),
        claim_validation,
        public_derivative,
        source_artifact_refs,
        image_artifact_refs,
        approval_actor_ref: format!("actor:{}", safe_identifier(&input.approval_actor_id)),
        approval_evidence_refs,
        manual_publish_evidence_refs,
        evidence_refs,
        limitations,
        workflow_compilation_id: input
            .workflow_compilation_id
            .map(|value| safe_identifier(&value)),
        job_id: input.job_id.map(|value| safe_identifier(&value)),
        occurred_at: input.occurred_at,
        analytics_event_id: None,
    })
}

fn public_derivative_for_deck(
    deck: &HomepageStoryDeckResponse,
    missing_images: bool,
) -> Result<HomepagePublishPublicDerivative> {
    ensure_safe_text(&deck.profile.positioning)?;
    let sections = deck
        .deck
        .slides
        .iter()
        .map(public_section_derivative)
        .collect::<Result<Vec<_>>>()?;
    let mut limitations = public_safe_strings(
        deck.deck
            .limitations
            .iter()
            .cloned()
            .chain(deck.profile.limitations.clone())
            .chain(deck.refresh.limitations.clone())
            .collect(),
    );
    if missing_images {
        limitations.push("Image-backed sections are not complete yet.".to_string());
    }
    Ok(HomepagePublishPublicDerivative {
        deck_id: safe_identifier(&deck.deck.deck_id),
        surface: safe_identifier(&deck.deck.surface),
        positioning: safe_public_text(&deck.profile.positioning),
        slide_count: sections.len(),
        sections,
        evidence_refs: public_safe_refs(
            deck.deck
                .evidence_refs
                .iter()
                .cloned()
                .chain(deck.profile.evidence_refs.clone())
                .collect(),
        ),
        limitations: stable_strings(limitations),
        visibility: "public_derivative".to_string(),
        external_publishing_claimed: false,
        memory_effect: "published_candidate_evidence_only".to_string(),
    })
}

fn public_section_derivative(
    slide: &HomepageNarrativeSlide,
) -> Result<HomepagePublishSectionDerivative> {
    ensure_safe_text(&slide.title)?;
    ensure_safe_text(&slide.body)?;
    ensure!(
        !slide.evidence_refs.is_empty(),
        "homepage publish section requires public evidence refs"
    );
    Ok(HomepagePublishSectionDerivative {
        slide_id: safe_identifier(&slide.slide_id),
        section_id: safe_identifier(&slide.section_id),
        order: slide.order,
        title: safe_public_text(&slide.title),
        body: safe_public_text(&slide.body),
        evidence_refs: public_safe_refs(slide.evidence_refs.clone()),
        limitations: public_safe_strings(slide.limitations.clone()),
    })
}

fn validate_deck_claims(
    deck: &HomepageStoryDeckResponse,
) -> Result<HomepagePublishClaimValidation> {
    let mut unsupported = Vec::new();
    let mut evidence_refs = Vec::new();
    for slide in &deck.deck.slides {
        if slide.evidence_refs.is_empty() {
            unsupported.push(format!("slide:{}", safe_identifier(&slide.slide_id)));
        }
        evidence_refs.extend(slide.evidence_refs.clone());
    }
    evidence_refs.extend(deck.deck.evidence_refs.clone());
    evidence_refs.extend(deck.profile.evidence_refs.clone());
    let evidence_refs = public_safe_refs(evidence_refs);
    Ok(HomepagePublishClaimValidation {
        status: if unsupported.is_empty() && !evidence_refs.is_empty() {
            "supported".to_string()
        } else {
            "blocked".to_string()
        },
        claim_count: deck.deck.slides.len(),
        unsupported_claim_refs: unsupported,
        evidence_refs,
        limitations: stable_strings(vec![
            "Claim validation is evidence-presence validation for this package; it does not promote graph truth."
                .to_string(),
        ]),
    })
}

fn record_publish_analytics_event(
    connection: &Connection,
    package_artifact: &ArtifactView,
    contract: &HomepagePublishApprovalPackageContract,
) -> Result<(
    ContentAnalyticsEventView,
    Option<crate::events::RealtimeEvent>,
)> {
    record_content_analytics_event(
        connection,
        ContentAnalyticsEventInput {
            event_kind: ContentAnalyticsEventKind::Published,
            content_ref_kind: "homepage_story_deck".to_string(),
            content_ref_id: contract.deck_id.clone(),
            content_version_id: Some(format!("{}:v{}", contract.deck_id, contract.deck_version)),
            artifact_id: Some(package_artifact.id.clone()),
            artifact_version_id: None,
            surface: "public_story".to_string(),
            section_id: None,
            cta_id: None,
            workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
            workflow_compilation_id: contract.workflow_compilation_id.clone(),
            job_id: contract.job_id.clone(),
            tracked_entry_point_id: None,
            visitor_session_id: None,
            referral_id: None,
            outcome_id: None,
            source_kind: "manual_homepage_publish".to_string(),
            source_id: package_artifact.id.clone(),
            idempotency_key: contract.idempotency_key.clone(),
            source_status: ContentAnalyticsSourceStatus::Manual,
            visibility: "staff".to_string(),
            evidence_refs: contract.manual_publish_evidence_refs.clone(),
            limitation_labels: vec![
                "manual_local_publish_evidence".to_string(),
                "external_publishing_deferred".to_string(),
                "external_analytics_missing".to_string(),
            ],
            payload: json!({
                "localPublishOnly": true,
                "externalPublishing": "not_called",
                "externalAnalytics": "not_measured",
                "approvalPackageArtifactId": package_artifact.id,
            }),
            occurred_at: contract.occurred_at.clone(),
        },
    )
}

fn artifact_refs(connection: &Connection, artifact_ids: Vec<String>) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    for artifact_id in artifact_ids {
        let artifact = load_artifact(connection, &artifact_id)?;
        ensure_safe_text(&artifact.summary)?;
        refs.push(format!("artifact:{}", safe_identifier(&artifact.id)));
    }
    Ok(stable_strings(refs))
}

fn load_existing_publish_package(
    connection: &Connection,
    package_id: &str,
    idempotency_key: &str,
) -> Result<Option<ArtifactView>> {
    let artifact_id = connection
        .query_row(
            "SELECT id FROM artifacts
             WHERE artifact_kind = ?1
               AND source_kind = 'homepage_publish_approval_package'
               AND source_id = ?2
             ORDER BY created_at ASC
             LIMIT 1",
            params![
                STORY_HOMEPAGE_PUBLISH_APPROVAL_PACKAGE_ARTIFACT_KIND,
                idempotency_key
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let _ = package_id;
    artifact_id
        .map(|id| load_artifact(connection, &id))
        .transpose()
}

fn ensure_safe_text(text: &str) -> Result<()> {
    ensure!(
        !redaction::contains_sensitive_text(text, &[]) && !contains_private_marker(text),
        "homepage publish approval contains private/internal or unsupported claim text"
    );
    Ok(())
}

fn safe_public_text(text: &str) -> String {
    redaction::redact_public_text(text.trim())
}

fn public_safe_strings(values: Vec<String>) -> Vec<String> {
    stable_strings(
        values
            .into_iter()
            .filter_map(|value| {
                let safe = safe_public_text(&value);
                if safe.trim().is_empty() || contains_private_marker(&safe) {
                    None
                } else {
                    Some(safe)
                }
            })
            .collect(),
    )
}

fn public_safe_refs(values: Vec<String>) -> Vec<String> {
    stable_strings(
        values
            .into_iter()
            .filter_map(|value| {
                let safe = safe_identifier(&value);
                if safe.trim().is_empty() || contains_private_marker(&safe) {
                    None
                } else {
                    Some(safe)
                }
            })
            .collect(),
    )
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
        "fakeanalytics",
        "fakepublishing",
        "externalpublishingsucceeded",
        "liveprovidersucceeded",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
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

fn stable_json_hash(value: &Value) -> Result<String> {
    let encoded = serde_json::to_string(value)?;
    let mut hasher = Sha256::new();
    hasher.update(encoded.as_bytes());
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::ArtifactInput;
    use crate::public_surfaces::{
        HomepageNarrativeDeck, HomepageStoryCopySlot, HomepageStoryCta, HomepageStoryProfile,
        HomepageStoryRefreshContract, PublicSurfaceReadiness,
    };
    use crate::schema::init_schema;

    #[test]
    fn approval_package_records_local_publish_and_analytics_idempotently() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let source = source_artifact(&connection, "intake", "Public-safe intake summary");
        let image = source_artifact(&connection, "image", "Reviewed image candidate");

        let first = record_homepage_publish_approval_package(
            &connection,
            input(
                deck(),
                vec![source.id.clone()],
                vec![image.id.clone()],
                "homepage-publish-v1",
            ),
        )
        .unwrap();
        let repeated = record_homepage_publish_approval_package(
            &connection,
            input(
                deck(),
                vec![source.id],
                vec![image.id],
                "homepage-publish-v1",
            ),
        )
        .unwrap();

        assert_eq!(first.package_artifact.id, repeated.package_artifact.id);
        assert!(first.version.is_some());
        assert_eq!(first.package_artifact.status, "published");
        assert_eq!(first.package_artifact.visibility_ceiling, "staff");
        assert_eq!(first.contract.approval_state, "approved");
        assert_eq!(first.contract.local_publication_state, "published_local");
        assert_eq!(first.contract.claim_validation.status, "supported");
        assert_eq!(
            first.contract.public_derivative.visibility,
            "public_derivative"
        );
        assert!(!first.contract.public_derivative.external_publishing_claimed);
        assert_eq!(
            first.analytics_event.as_ref().unwrap().event_kind,
            "published"
        );
        assert_eq!(
            first.analytics_event.as_ref().unwrap().source_status,
            "manual"
        );

        let artifact_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE artifact_kind = ?1",
                [STORY_HOMEPAGE_PUBLISH_APPROVAL_PACKAGE_ARTIFACT_KIND],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(artifact_count, 1);
        let analytics_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM content_analytics_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(analytics_count, 1);

        let serialized = serde_json::to_string(&first.contract).unwrap();
        assert!(!serialized.contains("external publishing succeeded"));
        assert!(!serialized.contains("provider internal"));
        assert!(!serialized.contains("private artifact text"));
    }

    #[test]
    fn publish_fails_closed_without_approval_evidence_or_safe_claims() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let source = source_artifact(&connection, "intake", "Public-safe intake summary");

        let mut missing_approval = input(
            deck(),
            vec![source.id.clone()],
            Vec::new(),
            "homepage-publish-missing-approval",
        );
        missing_approval.approval_state = "needs_review".to_string();
        let error =
            record_homepage_publish_approval_package(&connection, missing_approval).unwrap_err();
        assert!(error.to_string().contains("approved approval state"));

        let mut unsafe_deck = deck();
        unsafe_deck.deck.slides[0].body =
            "This contains prompt internal private artifact text".to_string();
        let error = record_homepage_publish_approval_package(
            &connection,
            input(
                unsafe_deck,
                vec![source.id],
                Vec::new(),
                "homepage-publish-unsafe",
            ),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("private/internal or unsupported claim text"));

        let artifact_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE artifact_kind = ?1",
                [STORY_HOMEPAGE_PUBLISH_APPROVAL_PACKAGE_ARTIFACT_KIND],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(artifact_count, 0);
        let analytics_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM content_analytics_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(analytics_count, 0);
    }

    #[test]
    fn missing_images_are_limitations_and_conflicting_idempotency_rejects() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let source = source_artifact(&connection, "intake", "Public-safe intake summary");

        let first = record_homepage_publish_approval_package(
            &connection,
            input(
                deck(),
                vec![source.id.clone()],
                Vec::new(),
                "homepage-publish-no-images",
            ),
        )
        .unwrap();
        assert!(first.contract.image_artifact_refs.is_empty());
        assert!(first
            .contract
            .limitations
            .iter()
            .any(|limitation| limitation.contains("no generated image candidates")));

        let mut changed = input(
            deck(),
            vec![source.id],
            Vec::new(),
            "homepage-publish-no-images",
        );
        changed.manual_publish_evidence_refs = vec!["manual_publish:changed".to_string()];
        let error = record_homepage_publish_approval_package(&connection, changed).unwrap_err();
        assert!(error.to_string().contains("idempotency key conflicts"));
    }

    fn source_artifact(connection: &Connection, source_id: &str, summary: &str) -> ArtifactView {
        record_artifact(
            connection,
            ArtifactInput {
                artifact_kind: "story.test_source".to_string(),
                title: format!("Source {source_id}"),
                status: "ready".to_string(),
                visibility_ceiling: "staff".to_string(),
                summary: summary.to_string(),
                source_kind: Some("test".to_string()),
                source_id: Some(source_id.to_string()),
                evidence_refs: vec![format!("evidence:{source_id}")],
                provenance: json!({"fixture": source_id}),
                content_hash: format!("sha256:{source_id}"),
                storage_uri: None,
                health_status: Some("available".to_string()),
                created_by_job_id: None,
            },
        )
        .unwrap()
        .0
    }

    fn input(
        deck: HomepageStoryDeckResponse,
        source_artifact_ids: Vec<String>,
        image_artifact_ids: Vec<String>,
        idempotency_key: &str,
    ) -> HomepagePublishApprovalInput {
        HomepagePublishApprovalInput {
            package_id: "homepage-v1".to_string(),
            idempotency_key: idempotency_key.to_string(),
            deck,
            source_artifact_ids,
            image_artifact_ids,
            approval_state: "approved".to_string(),
            approval_actor_id: "owner_1".to_string(),
            approval_evidence_refs: vec!["approval:owner_1".to_string()],
            manual_publish_evidence_refs: vec!["manual_publish:homepage_v1".to_string()],
            limitations: vec!["Manual local publish only.".to_string()],
            workflow_compilation_id: Some("workflow_compilation_story_v1".to_string()),
            job_id: Some("job_story_v1".to_string()),
            occurred_at: Some("2026-05-14T22:00:00Z".to_string()),
        }
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
}
