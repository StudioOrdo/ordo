use anyhow::{ensure, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::artifacts::{
    add_artifact_version, load_artifact, record_artifact, ArtifactInput, ArtifactVersionView,
    ArtifactView,
};
use crate::events::RealtimeEvent;
use crate::security::redaction;

pub const STORY_FOUNDER_INTAKE_ARTIFACT_KIND: &str = "story.founder_intake";
const CONTRACT_SCHEMA_VERSION: &str = "ordo.story_founder_intake_contract.v1";
const MAX_TEXT_LEN: usize = 4096;

#[derive(Debug, Clone)]
pub struct StoryFounderIntakeInput {
    pub intake_id: String,
    pub founder_story: String,
    pub business_stance: String,
    pub audience: Option<String>,
    pub public_claims: Vec<StoryIntakeClaimInput>,
    pub proof_evidence_refs: Vec<String>,
    pub private_notes: Vec<String>,
    pub style_preferences: Vec<String>,
    pub offer_refs: Vec<String>,
    pub cta_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    pub created_by_job_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StoryIntakeClaimInput {
    pub claim: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryFounderIntakeContract {
    pub schema_version: String,
    pub intake_id: String,
    pub founder_story: String,
    pub business_stance: String,
    pub audience: Option<String>,
    pub public_claims: Vec<StoryIntakeClaim>,
    pub proof_evidence_refs: Vec<String>,
    pub private_notes: Vec<String>,
    pub style_preferences: Vec<String>,
    pub offer_refs: Vec<String>,
    pub cta_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    pub created_by_job_id: Option<String>,
    pub public_derivative: StoryFounderIntakePublicDerivative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryIntakeClaim {
    pub claim: String,
    pub evidence_refs: Vec<String>,
    pub review_state: String,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryFounderIntakePublicDerivative {
    pub intake_id: String,
    pub summary: String,
    pub audience: Option<String>,
    pub claims: Vec<StoryIntakeClaim>,
    pub style_preferences: Vec<String>,
    pub offer_refs: Vec<String>,
    pub cta_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub visibility: String,
    pub memory_effect: String,
}

#[derive(Debug, Clone)]
pub struct StoryFounderIntakeArtifact {
    pub artifact: ArtifactView,
    pub version: Option<ArtifactVersionView>,
    pub contract: StoryFounderIntakeContract,
    pub public_derivative: StoryFounderIntakePublicDerivative,
    pub event: Option<RealtimeEvent>,
}

pub fn record_story_founder_intake_artifact(
    connection: &Connection,
    input: StoryFounderIntakeInput,
) -> Result<StoryFounderIntakeArtifact> {
    let contract = story_founder_intake_contract(input)?;
    let contract_json = serde_json::to_value(&contract)?;
    let content_hash = stable_json_hash(&contract_json)?;

    if let Some(existing) =
        load_existing_story_founder_intake_artifact(connection, &contract.intake_id, &content_hash)?
    {
        return Ok(StoryFounderIntakeArtifact {
            artifact: existing,
            version: None,
            public_derivative: contract.public_derivative.clone(),
            contract,
            event: None,
        });
    }

    let (artifact, event) = record_artifact(
        connection,
        ArtifactInput {
            artifact_kind: STORY_FOUNDER_INTAKE_ARTIFACT_KIND.to_string(),
            title: format!("Story founder intake {}", contract.intake_id),
            status: "ready_for_review".to_string(),
            visibility_ceiling: "owner".to_string(),
            summary: contract.public_derivative.summary.clone(),
            source_kind: Some("story_pack_intake".to_string()),
            source_id: Some(contract.intake_id.clone()),
            evidence_refs: contract.public_derivative.evidence_refs.clone(),
            provenance: json!({
                "schemaVersion": CONTRACT_SCHEMA_VERSION,
                "generatedBy": "story.captureFounderIntake",
                "intakeId": contract.intake_id,
                "sourceKind": contract.source_kind,
                "sourceId": contract.source_id,
                "approvalState": "needs_review",
                "publicDerivative": contract.public_derivative,
            }),
            content_hash: content_hash.clone(),
            storage_uri: Some(format!(
                "ordo://artifacts/story-founder-intakes/{}",
                safe_identifier(&contract.intake_id)
            )),
            health_status: Some("contract_only".to_string()),
            created_by_job_id: contract.created_by_job_id.clone(),
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
            "publicDerivative": contract.public_derivative,
            "liveModelCalled": false,
            "automaticMemoryTruthPromotion": false,
        }),
    )?;

    Ok(StoryFounderIntakeArtifact {
        artifact,
        version: Some(version),
        public_derivative: contract.public_derivative.clone(),
        contract,
        event: Some(event),
    })
}

fn story_founder_intake_contract(
    input: StoryFounderIntakeInput,
) -> Result<StoryFounderIntakeContract> {
    ensure!(
        !input.intake_id.trim().is_empty(),
        "story intake requires an intake id"
    );
    ensure!(
        !input.founder_story.trim().is_empty(),
        "story intake requires a founder story"
    );
    ensure!(
        !input.business_stance.trim().is_empty(),
        "story intake requires a business stance"
    );

    let intake_id = safe_identifier(&input.intake_id);
    let founder_story = bounded_text(&input.founder_story);
    let business_stance = bounded_text(&input.business_stance);
    let audience = input.audience.map(|value| bounded_text(&value));
    let proof_evidence_refs = stable_refs(input.proof_evidence_refs);
    let private_notes = input
        .private_notes
        .into_iter()
        .map(|value| bounded_text(&value))
        .collect::<Vec<_>>();
    let style_preferences = stable_public_values(input.style_preferences);
    let offer_refs = stable_refs(input.offer_refs);
    let cta_refs = stable_refs(input.cta_refs);
    let mut limitations = stable_public_values(input.limitations);
    let source_kind = input.source_kind.map(|value| safe_identifier(&value));
    let source_id = input.source_id.map(|value| safe_identifier(&value));
    let created_by_job_id = input.created_by_job_id.map(|value| safe_identifier(&value));
    limitations.push(
        "Story intake is owner-visible source material; public output uses the reviewed derivative only."
            .to_string(),
    );
    limitations.push(
        "Generated content memory candidates may be proposed later, but this intake does not promote truth."
            .to_string(),
    );
    limitations = stable_strings(limitations);

    let public_claims = input
        .public_claims
        .into_iter()
        .map(normalize_claim)
        .collect::<Result<Vec<_>>>()?;
    let public_derivative = public_derivative(
        &intake_id,
        &founder_story,
        &business_stance,
        audience.as_deref(),
        &public_claims,
        &proof_evidence_refs,
        &style_preferences,
        &offer_refs,
        &cta_refs,
        &limitations,
    )?;

    Ok(StoryFounderIntakeContract {
        schema_version: CONTRACT_SCHEMA_VERSION.to_string(),
        intake_id,
        founder_story,
        business_stance,
        audience,
        public_claims,
        proof_evidence_refs,
        private_notes,
        style_preferences,
        offer_refs,
        cta_refs,
        limitations,
        source_kind,
        source_id,
        created_by_job_id,
        public_derivative,
    })
}

fn normalize_claim(input: StoryIntakeClaimInput) -> Result<StoryIntakeClaim> {
    ensure!(
        !input.claim.trim().is_empty(),
        "story intake public claim cannot be blank"
    );
    let claim = public_text(&bounded_text(&input.claim));
    let evidence_refs = stable_refs(input.evidence_refs);
    let mut limitations = Vec::new();
    let review_state = if claim == "[REDACTED_POLICY_BOUNDARY]" {
        limitations.push("Claim withheld from public derivative pending review.".to_string());
        "needs_review"
    } else if evidence_refs.is_empty() {
        limitations.push("Public claim needs evidence before publication.".to_string());
        "needs_review"
    } else {
        "evidence_backed"
    };
    Ok(StoryIntakeClaim {
        claim,
        evidence_refs,
        review_state: review_state.to_string(),
        limitations,
    })
}

fn public_derivative(
    intake_id: &str,
    founder_story: &str,
    business_stance: &str,
    audience: Option<&str>,
    public_claims: &[StoryIntakeClaim],
    proof_evidence_refs: &[String],
    style_preferences: &[String],
    offer_refs: &[String],
    cta_refs: &[String],
    limitations: &[String],
) -> Result<StoryFounderIntakePublicDerivative> {
    let founder = public_text(founder_story);
    let stance = public_text(business_stance);
    ensure!(
        founder != "[REDACTED_POLICY_BOUNDARY]" || stance != "[REDACTED_POLICY_BOUNDARY]",
        "story intake public derivative requires public-safe founder story or business stance"
    );
    let summary = if founder == "[REDACTED_POLICY_BOUNDARY]" {
        stance.clone()
    } else if stance == "[REDACTED_POLICY_BOUNDARY]" {
        founder.clone()
    } else {
        format!("{founder} {stance}")
    };
    let claims = public_claims
        .iter()
        .map(|claim| StoryIntakeClaim {
            claim: claim.claim.clone(),
            evidence_refs: claim.evidence_refs.clone(),
            review_state: claim.review_state.clone(),
            limitations: claim.limitations.clone(),
        })
        .collect::<Vec<_>>();
    let mut evidence_refs = stable_refs(
        proof_evidence_refs
            .iter()
            .cloned()
            .chain(claims.iter().flat_map(|claim| claim.evidence_refs.clone()))
            .collect(),
    );
    if evidence_refs.is_empty() {
        evidence_refs.push("story_intake:needs_evidence".to_string());
    }
    let mut all_limitations = stable_public_values(
        limitations
            .iter()
            .cloned()
            .chain(
                claims
                    .iter()
                    .flat_map(|claim| claim.limitations.iter().cloned()),
            )
            .collect(),
    );
    if claims
        .iter()
        .any(|claim| claim.review_state == "needs_review")
    {
        all_limitations
            .push("One or more public claims need review before publication.".to_string());
    }
    all_limitations = stable_strings(all_limitations);
    Ok(StoryFounderIntakePublicDerivative {
        intake_id: intake_id.to_string(),
        summary: public_text(&summary),
        audience: audience.map(public_text),
        claims,
        style_preferences: stable_public_values(style_preferences.to_vec()),
        offer_refs: stable_refs(offer_refs.to_vec()),
        cta_refs: stable_refs(cta_refs.to_vec()),
        evidence_refs,
        limitations: all_limitations,
        visibility: "public_derivative".to_string(),
        memory_effect: "candidate_only".to_string(),
    })
}

fn load_existing_story_founder_intake_artifact(
    connection: &Connection,
    intake_id: &str,
    content_hash: &str,
) -> Result<Option<ArtifactView>> {
    let artifact_id = connection
        .query_row(
            "SELECT id FROM artifacts
             WHERE artifact_kind = ?1
               AND source_kind = 'story_pack_intake'
               AND source_id = ?2
               AND content_hash = ?3
             ORDER BY created_at ASC
             LIMIT 1",
            params![STORY_FOUNDER_INTAKE_ARTIFACT_KIND, intake_id, content_hash],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    artifact_id
        .map(|id| load_artifact(connection, &id))
        .transpose()
}

fn bounded_text(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= MAX_TEXT_LEN {
        trimmed.to_string()
    } else {
        trimmed.chars().take(MAX_TEXT_LEN).collect::<String>()
    }
}

fn public_text(value: &str) -> String {
    let redacted = replace_policy_markers(&redaction::redact_public_text(value));
    if contains_policy_marker(&redacted) {
        "[REDACTED_POLICY_BOUNDARY]".to_string()
    } else {
        redacted
    }
}

fn stable_public_values(values: Vec<String>) -> Vec<String> {
    stable_strings(
        values
            .into_iter()
            .map(|value| public_text(&value))
            .collect(),
    )
}

fn stable_refs(values: Vec<String>) -> Vec<String> {
    stable_strings(
        values
            .into_iter()
            .map(|value| safe_identifier(&value))
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

fn replace_policy_markers(input: &str) -> String {
    let mut redacted = input.to_string();
    for marker in [
        "staff routing",
        "provider internal",
        "provider secret",
        "prompt internal",
        "raw policy",
        "policy internal",
        "owner-only",
        "owner only",
        "private artifact text",
        "compiled plan private input",
        "task private payload",
        "graph certainty",
        "unsupported claim",
    ] {
        redacted = replace_ascii_case_insensitive(&redacted, marker, "[REDACTED_POLICY_BOUNDARY]");
    }
    redacted
}

fn contains_policy_marker(value: &str) -> bool {
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
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
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

fn stable_json_hash(value: &Value) -> Result<String> {
    let encoded = serde_json::to_string(value)?;
    let mut hasher = Sha256::new();
    hasher.update(encoded.as_bytes());
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;

    #[test]
    fn founder_intake_records_durable_artifact_and_public_derivative() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        let recorded = record_story_founder_intake_artifact(&connection, valid_intake()).unwrap();

        assert_eq!(
            recorded.artifact.artifact_kind,
            STORY_FOUNDER_INTAKE_ARTIFACT_KIND
        );
        assert_eq!(recorded.artifact.visibility_ceiling, "owner");
        assert_eq!(recorded.artifact.status, "ready_for_review");
        assert_eq!(
            recorded.artifact.source_kind.as_deref(),
            Some("story_pack_intake")
        );
        assert_eq!(recorded.artifact.source_id.as_deref(), Some("keith-v1"));
        assert!(recorded.version.is_some());
        assert!(recorded.event.is_some());
        assert_eq!(recorded.public_derivative.memory_effect, "candidate_only");
        assert!(recorded
            .public_derivative
            .summary
            .contains("local-first operating appliance"));
        assert_eq!(
            recorded.public_derivative.claims[0].review_state,
            "evidence_backed"
        );
        assert_eq!(
            recorded.public_derivative.evidence_refs,
            vec![
                "business_fact:homepage.positioning",
                "business_fact:ordo.local_first"
            ]
        );
    }

    #[test]
    fn public_derivative_redacts_private_notes_and_policy_markers() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let mut input = valid_intake();
        input.founder_story =
            "Keith is building in public. staff routing should never leak.".to_string();
        input.business_stance =
            "Provider internal prompt internal raw policy owner-only private artifact text."
                .to_string();
        input.private_notes =
            vec!["Call 555-555-5555 with sk-live-secret and owner-only pricing.".to_string()];
        input.public_claims = vec![StoryIntakeClaimInput {
            claim: "Unsupported claim with graph certainty.".to_string(),
            evidence_refs: vec![],
        }];

        let recorded = record_story_founder_intake_artifact(&connection, input).unwrap();
        let public_json = serde_json::to_string(&recorded.public_derivative).unwrap();
        let event_json = serde_json::to_string(&recorded.event.unwrap().payload).unwrap();

        for forbidden in [
            "staff routing",
            "Provider internal",
            "prompt internal",
            "raw policy",
            "owner-only",
            "private artifact text",
            "555-555-5555",
            "sk-live-secret",
            "graph certainty",
            "Unsupported claim",
        ] {
            assert!(
                !public_json.contains(forbidden),
                "public derivative leaked {forbidden}: {public_json}"
            );
            assert!(
                !event_json.contains(forbidden),
                "event payload leaked {forbidden}: {event_json}"
            );
        }
        assert!(public_json.contains("[REDACTED_POLICY_BOUNDARY]"));
        assert_eq!(
            recorded.public_derivative.claims[0].review_state,
            "needs_review"
        );
        assert!(recorded
            .public_derivative
            .limitations
            .iter()
            .any(|value| value.contains("need review")));
    }

    #[test]
    fn public_claims_without_evidence_remain_needs_review() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let mut input = valid_intake();
        input.public_claims = vec![StoryIntakeClaimInput {
            claim: "Ordo helps small operators answer enshittification.".to_string(),
            evidence_refs: vec![],
        }];

        let recorded = record_story_founder_intake_artifact(&connection, input).unwrap();

        assert_eq!(
            recorded.public_derivative.claims[0].review_state,
            "needs_review"
        );
        assert!(recorded.public_derivative.claims[0]
            .limitations
            .iter()
            .any(|limitation| limitation.contains("needs evidence")));
        assert_eq!(recorded.public_derivative.memory_effect, "candidate_only");
    }

    #[test]
    fn repeated_same_intake_reuses_existing_artifact_without_duplicate_event() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        let first = record_story_founder_intake_artifact(&connection, valid_intake()).unwrap();
        let second = record_story_founder_intake_artifact(&connection, valid_intake()).unwrap();

        assert_eq!(first.artifact.id, second.artifact.id);
        assert!(first.version.is_some());
        assert!(second.version.is_none());
        assert!(second.event.is_none());

        let artifact_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE artifact_kind = ?1",
                [STORY_FOUNDER_INTAKE_ARTIFACT_KIND],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(artifact_count, 1);
    }

    fn valid_intake() -> StoryFounderIntakeInput {
        StoryFounderIntakeInput {
            intake_id: "keith-v1".to_string(),
            founder_story: "Keith is building Studio Ordo as a local-first operating appliance."
                .to_string(),
            business_stance:
                "Ordo is a practical answer to enshittification for relationship-led businesses."
                    .to_string(),
            audience: Some("Solopreneurs and small operators".to_string()),
            public_claims: vec![StoryIntakeClaimInput {
                claim: "Ordo keeps business motion grounded in local evidence.".to_string(),
                evidence_refs: vec!["business_fact:ordo.local_first".to_string()],
            }],
            proof_evidence_refs: vec![
                "business_fact:homepage.positioning".to_string(),
                "business_fact:homepage.positioning".to_string(),
            ],
            private_notes: vec!["Do not publish this private founder note.".to_string()],
            style_preferences: vec![
                "cinematic editorial".to_string(),
                "cinematic editorial".to_string(),
            ],
            offer_refs: vec!["offer:hosted-30-day-trial".to_string()],
            cta_refs: vec!["cta:talk-with-ordo".to_string()],
            limitations: vec!["Draft intake requires owner review.".to_string()],
            source_kind: Some("manual_owner_intake".to_string()),
            source_id: Some("owner_keith".to_string()),
            created_by_job_id: None,
        }
    }
}
