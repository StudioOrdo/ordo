use anyhow::{bail, ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::llm_gateway::PromptSlot;

pub const ETHICAL_BUSINESS_PERSUASION_SLOT_ID: &str = "ethical_business_persuasion";
pub const ETHICAL_BUSINESS_PERSUASION_SLOT_VERSION: &str = "v1";
pub const ETHICAL_PERSUASION_CAPABILITY_ID: &str = "ethical_persuasion.recommend";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EthicalPersuasionPrinciple {
    Reciprocity,
    CommitmentConsistency,
    SocialProof,
    Authority,
    Liking,
    Scarcity,
    Unity,
}

impl EthicalPersuasionPrinciple {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reciprocity => "reciprocity",
            Self::CommitmentConsistency => "commitment_consistency",
            Self::SocialProof => "social_proof",
            Self::Authority => "authority",
            Self::Liking => "liking",
            Self::Scarcity => "scarcity",
            Self::Unity => "unity",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthicalPersuasionEvidence {
    pub principle: EthicalPersuasionPrinciple,
    pub evidence_refs: Vec<String>,
    pub source_refs: Vec<String>,
    pub reasoning: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthicalPersuasionSlotInput {
    pub use_case: String,
    pub visibility_ceiling: String,
    pub policy_decision_id: Option<String>,
    pub privacy_transform_run_id: Option<String>,
    pub inclusion_reason: String,
    pub truncation_reason: Option<String>,
    pub principle_evidence: Vec<EthicalPersuasionEvidence>,
    pub staff_reasoning: String,
    pub client_safe_suggestion: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthicalPersuasionRecommendation {
    pub slot_id: String,
    pub slot_version: String,
    pub use_case: String,
    pub principles: Vec<String>,
    pub source_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub visibility_ceiling: String,
    pub policy_decision_id: Option<String>,
    pub privacy_transform_run_id: Option<String>,
    pub inclusion_reason: String,
    pub truncation_reason: Option<String>,
    pub staff_reasoning: String,
    pub client_safe_suggestion: String,
    pub prompt_slot: PromptSlot,
}

pub fn build_ethical_business_persuasion_slot(
    input: EthicalPersuasionSlotInput,
) -> Result<EthicalPersuasionRecommendation> {
    validate_input(&input)?;
    let mut principles = Vec::new();
    let mut evidence_refs = Vec::new();
    let mut source_refs = Vec::new();

    for evidence in &input.principle_evidence {
        principles.push(evidence.principle.as_str().to_string());
        evidence_refs.extend(evidence.evidence_refs.clone());
        source_refs.extend(evidence.source_refs.clone());
    }
    evidence_refs.sort();
    evidence_refs.dedup();
    source_refs.sort();
    source_refs.dedup();

    let content = json!({
        "slotId": ETHICAL_BUSINESS_PERSUASION_SLOT_ID,
        "slotVersion": ETHICAL_BUSINESS_PERSUASION_SLOT_VERSION,
        "useCase": input.use_case,
        "allowedPrinciples": principles,
        "guardrails": [
            "do not invent evidence",
            "do not exaggerate urgency",
            "do not exploit fear, shame, confusion, dependency, or pressure",
            "do not hide material limitations",
            "do not present candidates as facts",
            "do not override consent, privacy, safety, or policy",
            "keep client-facing language respectful, plain, and agency-preserving"
        ],
        "principleEvidence": input.principle_evidence,
        "staffReasoning": input.staff_reasoning,
        "clientSafeSuggestion": input.client_safe_suggestion,
        "policyDecisionId": input.policy_decision_id,
        "privacyTransformRunId": input.privacy_transform_run_id,
        "truncationReason": input.truncation_reason,
    })
    .to_string();

    let prompt_slot = PromptSlot::new(
        ETHICAL_BUSINESS_PERSUASION_SLOT_ID,
        "Ethical Business Persuasion",
        content,
        source_refs.clone(),
        input.inclusion_reason.clone(),
        input.visibility_ceiling.clone(),
    )?;

    Ok(EthicalPersuasionRecommendation {
        slot_id: ETHICAL_BUSINESS_PERSUASION_SLOT_ID.to_string(),
        slot_version: ETHICAL_BUSINESS_PERSUASION_SLOT_VERSION.to_string(),
        use_case: input.use_case,
        principles,
        source_refs,
        evidence_refs,
        visibility_ceiling: input.visibility_ceiling,
        policy_decision_id: input.policy_decision_id,
        privacy_transform_run_id: input.privacy_transform_run_id,
        inclusion_reason: input.inclusion_reason,
        truncation_reason: input.truncation_reason,
        staff_reasoning: input.staff_reasoning,
        client_safe_suggestion: input.client_safe_suggestion,
        prompt_slot,
    })
}

fn validate_input(input: &EthicalPersuasionSlotInput) -> Result<()> {
    require_text("use case", &input.use_case)?;
    require_text("visibility ceiling", &input.visibility_ceiling)?;
    require_text("inclusion reason", &input.inclusion_reason)?;
    require_text("staff reasoning", &input.staff_reasoning)?;
    require_text("client-safe suggestion", &input.client_safe_suggestion)?;
    ensure!(
        !input.principle_evidence.is_empty(),
        "ethical persuasion requires principle evidence"
    );
    for evidence in &input.principle_evidence {
        ensure!(
            !evidence.evidence_refs.is_empty(),
            "{} requires evidence refs",
            evidence.principle.as_str()
        );
        ensure!(
            !evidence.source_refs.is_empty(),
            "{} requires source refs",
            evidence.principle.as_str()
        );
        require_text("principle reasoning", &evidence.reasoning)?;
        reject_sensitive_refs(&evidence.evidence_refs)?;
        reject_sensitive_refs(&evidence.source_refs)?;
    }
    reject_unsupported_claims(input)?;
    reject_coercive_language("staff reasoning", &input.staff_reasoning)?;
    reject_coercive_language("client-safe suggestion", &input.client_safe_suggestion)?;
    reject_internal_client_language(&input.client_safe_suggestion)?;
    Ok(())
}

fn reject_unsupported_claims(input: &EthicalPersuasionSlotInput) -> Result<()> {
    let haystack = format!(
        "{}\n{}",
        input.staff_reasoning.to_lowercase(),
        input.client_safe_suggestion.to_lowercase()
    );
    let has_principle = |principle| {
        input
            .principle_evidence
            .iter()
            .any(|evidence| evidence.principle == principle)
    };
    if contains_any(
        &haystack,
        &[
            "everyone",
            "most customers",
            "proven by clients",
            "other customers",
        ],
    ) {
        ensure!(
            has_principle(EthicalPersuasionPrinciple::SocialProof),
            "social proof claims require social proof evidence"
        );
    }
    if contains_any(
        &haystack,
        &["expert", "certified", "authority", "credential"],
    ) {
        ensure!(
            has_principle(EthicalPersuasionPrinciple::Authority),
            "authority claims require authority evidence"
        );
    }
    if contains_any(
        &haystack,
        &[
            "limited time",
            "last chance",
            "only today",
            "act now",
            "urgent",
        ],
    ) {
        ensure!(
            has_principle(EthicalPersuasionPrinciple::Scarcity),
            "scarcity or urgency claims require real scarcity evidence"
        );
    }
    Ok(())
}

fn reject_coercive_language(field: &str, value: &str) -> Result<()> {
    let lower = value.to_lowercase();
    let blocked = [
        "you will regret",
        "shame",
        "afraid",
        "fear",
        "confused",
        "pressure",
        "dependent on us",
        "you have no choice",
        "must buy",
        "act now",
        "last chance",
        "only today",
        "guaranteed",
        "secret reason",
    ];
    if let Some(term) = blocked.iter().find(|term| lower.contains(**term)) {
        bail!("{field} contains disallowed pressure or manipulation language: {term}");
    }
    Ok(())
}

fn reject_internal_client_language(value: &str) -> Result<()> {
    let lower = value.to_lowercase();
    ensure!(
        !contains_any(
            &lower,
            &[
                "internal reasoning",
                "policy decision",
                "privacy transform",
                "source refs",
                "evidence refs",
                "prompt slot",
                "visibility ceiling",
            ],
        ),
        "client-facing language must not expose internal mechanics"
    );
    Ok(())
}

fn reject_sensitive_refs(refs: &[String]) -> Result<()> {
    for value in refs {
        let lower = value.to_lowercase();
        ensure!(
            !lower.contains('@')
                && !lower.contains("bearer ")
                && !lower.contains("sk-")
                && !lower.contains("api_key")
                && !lower.contains("secret"),
            "source and evidence refs must not contain sensitive values"
        );
    }
    Ok(())
}

fn require_text(label: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{label} is required");
    Ok(())
}

fn contains_any(value: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| value.contains(term))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::conversations::{
        create_conversation_participant, find_or_create_canonical_conversation,
        CanonicalConversationRequest, ConversationParticipantCreateRequest,
    };
    use crate::llm_accounting::{
        record_invocation_started, record_privacy_transform_runs, record_prompt_slot_usage,
        rollup_usage_by_prompt_slot,
    };
    use crate::llm_gateway::{compile_prompt, CompiledPrompt, LlmGatewayRequest};
    use crate::policy::{
        record_policy_decision, ActorContext, PolicyAction, PolicyDecision,
        PolicyDecisionCorrelation, PolicyOutcome, ResourceKind, ResourceRef,
    };
    use crate::schema::init_schema;
    use rusqlite::{params, Connection};

    #[test]
    fn slot_builder_requires_evidence_and_source_refs() {
        let mut input = baseline_input();
        input.principle_evidence[0].source_refs.clear();

        assert!(build_ethical_business_persuasion_slot(input).is_err());
    }

    #[test]
    fn slot_builder_records_version_principles_and_metadata() {
        let recommendation = build_ethical_business_persuasion_slot(baseline_input()).unwrap();

        assert_eq!(recommendation.slot_id, ETHICAL_BUSINESS_PERSUASION_SLOT_ID);
        assert_eq!(
            recommendation.slot_version,
            ETHICAL_BUSINESS_PERSUASION_SLOT_VERSION
        );
        assert!(recommendation
            .principles
            .contains(&"reciprocity".to_string()));
        assert!(recommendation
            .evidence_refs
            .contains(&"offer_view_1".to_string()));
        assert_eq!(
            recommendation.prompt_slot.id,
            ETHICAL_BUSINESS_PERSUASION_SLOT_ID
        );
        assert_eq!(
            recommendation.prompt_slot.visibility_ceiling,
            "staff_private"
        );
    }

    #[test]
    fn unsupported_social_proof_and_authority_are_rejected() {
        let mut social = baseline_input();
        social.staff_reasoning =
            "Most customers choose this because everyone trusts it.".to_string();
        assert!(build_ethical_business_persuasion_slot(social).is_err());

        let mut authority = baseline_input();
        authority.client_safe_suggestion =
            "Our certified experts say this is the only path.".to_string();
        assert!(build_ethical_business_persuasion_slot(authority).is_err());
    }

    #[test]
    fn invented_scarcity_and_pressure_language_are_rejected() {
        let mut scarcity = baseline_input();
        scarcity.client_safe_suggestion = "Act now because this is your last chance.".to_string();
        assert!(build_ethical_business_persuasion_slot(scarcity).is_err());

        let mut pressure = baseline_input();
        pressure.staff_reasoning = "Create pressure so the client feels they must buy.".to_string();
        assert!(build_ethical_business_persuasion_slot(pressure).is_err());
    }

    #[test]
    fn client_output_hides_internal_reasoning_while_staff_output_keeps_evidence() {
        let recommendation = build_ethical_business_persuasion_slot(baseline_input()).unwrap();

        assert!(recommendation.staff_reasoning.contains("offer_view_1"));
        assert!(recommendation
            .client_safe_suggestion
            .contains("digital proof"));
        assert!(!recommendation
            .client_safe_suggestion
            .contains("offer_view_1"));
        assert!(!recommendation
            .client_safe_suggestion
            .to_lowercase()
            .contains("prompt slot"));
    }

    #[test]
    fn slot_accounting_records_ethical_slot_without_sensitive_payloads() {
        let connection = test_connection();
        let recommendation = build_ethical_business_persuasion_slot(baseline_input()).unwrap();
        let (request, prompt, policy_decision_id) =
            request_prompt_and_policy(&connection, recommendation.prompt_slot);

        record_invocation_started(&connection, &request, &prompt, &policy_decision_id).unwrap();
        record_privacy_transform_runs(
            &connection,
            &request.run_id,
            &["privacy_transform_safe_1".to_string()],
        )
        .unwrap();
        let events =
            record_prompt_slot_usage(&connection, &request, &prompt, &policy_decision_id).unwrap();

        assert_eq!(events.len(), 1);
        let (slot_id, slot_version, source_refs_json, estimated_tokens, content_hash): (
            String,
            String,
            String,
            i64,
            String,
        ) = connection
            .query_row(
                "SELECT slot_id, slot_version, source_refs_json, estimated_tokens, content_hash
                 FROM llm_prompt_slot_usage",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(slot_id, ETHICAL_BUSINESS_PERSUASION_SLOT_ID);
        assert_eq!(slot_version, ETHICAL_BUSINESS_PERSUASION_SLOT_VERSION);
        assert!(source_refs_json.contains("message_ava_14"));
        assert!(estimated_tokens > 0);
        assert!(content_hash.starts_with("sha256:"));
        assert_eq!(
            rollup_usage_by_prompt_slot(&connection).unwrap()[0].key,
            ETHICAL_BUSINESS_PERSUASION_SLOT_ID
        );

        for raw in [
            "ava@example.com",
            "sk-secret",
            "The client asked about the digital proof.",
            "You can review the digital proof first.",
        ] {
            let pattern = format!("%{raw}%");
            let slot_leak_count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM llm_prompt_slot_usage
                     WHERE slot_id LIKE ?1
                        OR slot_version LIKE ?1
                        OR source_refs_json LIKE ?1
                        OR content_hash LIKE ?1",
                    params![pattern],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(slot_leak_count, 0, "llm_prompt_slot_usage leaked {raw}");

            let invocation_leak_count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM llm_invocations
                     WHERE prompt_hash LIKE ?1
                        OR metadata_json LIKE ?1",
                    params![pattern],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(invocation_leak_count, 0, "llm_invocations leaked {raw}");

            let event_leak_count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM conversation_events WHERE payload_json LIKE ?1",
                    params![pattern],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(event_leak_count, 0, "conversation_events leaked {raw}");
        }
    }

    fn baseline_input() -> EthicalPersuasionSlotInput {
        EthicalPersuasionSlotInput {
            use_case: "reply_draft".to_string(),
            visibility_ceiling: "staff_private".to_string(),
            policy_decision_id: Some("policy_1".to_string()),
            privacy_transform_run_id: Some("privacy_transform_safe_1".to_string()),
            inclusion_reason: "Draft reply can be clearer while preserving client agency.".to_string(),
            truncation_reason: None,
            principle_evidence: vec![EthicalPersuasionEvidence {
                principle: EthicalPersuasionPrinciple::Reciprocity,
                evidence_refs: vec!["offer_view_1".to_string()],
                source_refs: vec!["message_ava_14".to_string()],
                reasoning: "The client already received value in the digital proof explanation."
                    .to_string(),
            }],
            staff_reasoning:
                "Use reciprocity because offer_view_1 shows the client already received a digital proof explanation."
                    .to_string(),
            client_safe_suggestion:
                "You can review the digital proof first, then decide whether the card add-on is useful."
                    .to_string(),
        }
    }

    fn test_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
                 VALUES ('actor_staff', 'staff', 'Staff', 'active', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO connections (
                    id, connection_type, display_name, status, identity_json, scope_json, metadata_json, created_at, updated_at
                 ) VALUES ('connection_1', 'client', 'Client', 'active', '{}', '{}', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
    }

    fn request_prompt_and_policy(
        connection: &Connection,
        prompt_slot: PromptSlot,
    ) -> (LlmGatewayRequest, CompiledPrompt, String) {
        let conversation = find_or_create_canonical_conversation(
            connection,
            &CanonicalConversationRequest {
                surface: "client_portal".to_string(),
                subject_kind: "connection".to_string(),
                subject_id: "connection_1".to_string(),
                connection_id: Some("connection_1".to_string()),
                visitor_session_id: None,
                created_by_actor_id: Some("actor_staff".to_string()),
            },
        )
        .unwrap();
        let assistant = create_conversation_participant(
            connection,
            &ConversationParticipantCreateRequest {
                conversation_id: conversation.id.clone(),
                participant_kind: "assistant".to_string(),
                actor_id: None,
                connection_id: None,
                visitor_session_id: None,
                display_name: "Ordo".to_string(),
                role: "assistant".to_string(),
            },
        )
        .unwrap();
        let prompt = compile_prompt(&[prompt_slot]).unwrap();
        let request = LlmGatewayRequest {
            run_id: "llm_run_ethical_1".to_string(),
            conversation_id: conversation.id,
            segment_id: None,
            assistant_participant_id: assistant.id,
            client_id: Some("client_llm_ethical_1".to_string()),
            provider_id: "local_fake".to_string(),
            model_id: "fake-chat".to_string(),
            user_message: "ava@example.com asked about sk-secret next steps".to_string(),
            prompt_slots: prompt.slots.clone(),
        };
        let policy_decision_id = record_policy_decision(
            connection,
            &PolicyDecision {
                outcome: PolicyOutcome::Allowed,
                actor: ActorContext::local_owner("test"),
                action: PolicyAction::Generate,
                resource: ResourceRef::new(ResourceKind::LlmRun, &request.run_id),
                capability_id: Some("llm.invoke".to_string()),
                reason: "test policy".to_string(),
            },
            PolicyDecisionCorrelation {
                request_id: Some(request.run_id.clone()),
                ..PolicyDecisionCorrelation::default()
            },
        )
        .unwrap();
        (request, prompt, policy_decision_id)
    }
}
