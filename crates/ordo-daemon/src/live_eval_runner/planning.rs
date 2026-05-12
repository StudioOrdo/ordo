use super::*;
use anyhow::{anyhow, ensure, Result};
use rusqlite::{params, Connection};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub fn plan_live_journey_from_env_map(
    env_values: &BTreeMap<String, String>,
    request: LiveJourneyPlanRequest,
) -> Result<LiveJourneyRunSummary> {
    let personas = load_persona_dir(&request.persona_dir, &request.private_terms)?;
    let (guard, config) = LiveJourneyConfig::from_env_map(env_values);
    let selected_personas = select_personas(&personas, &request.selected_persona_ids)?;

    match config {
        Some(config) => write_live_journey_manifest(LiveJourneyManifestInput {
            guard,
            provider_id: Some(config.provider_id),
            model_id: Some(config.model_id),
            max_cases: config.max_cases,
            budget_micros: config.budget_micros,
            persona_library_count: personas.len(),
            selected_personas,
            request,
        }),
        None => write_live_journey_manifest(LiveJourneyManifestInput {
            guard,
            provider_id: None,
            model_id: None,
            max_cases: DEFAULT_MAX_CASES,
            budget_micros: 0,
            persona_library_count: personas.len(),
            selected_personas,
            request,
        }),
    }
}

pub(crate) struct LiveJourneyManifestInput {
    guard: LiveEvalGuardDecision,
    provider_id: Option<String>,
    model_id: Option<String>,
    max_cases: u32,
    budget_micros: u64,
    persona_library_count: usize,
    selected_personas: Vec<EvalPersona>,
    request: LiveJourneyPlanRequest,
}

pub(crate) fn write_live_journey_manifest(input: LiveJourneyManifestInput) -> Result<LiveJourneyRunSummary> {
    let capped_personas = input
        .selected_personas
        .iter()
        .take(input.max_cases as usize)
        .cloned()
        .collect::<Vec<_>>();
    let estimated_total_cost_micros =
        ESTIMATED_JOURNEY_CASE_COST_MICROS.saturating_mul(capped_personas.len() as u64);

    let mut manifest_guard = input.guard;
    if manifest_guard.status == LiveEvalStatus::Allowed
        && input.budget_micros < estimated_total_cost_micros
    {
        manifest_guard = blocked(format!(
            "live journey budget would be exceeded before execution: estimated {estimated_total_cost_micros} micros for {} cases with budget {} micros",
            capped_personas.len(),
            input.budget_micros
        ));
    }

    let planned_cases = capped_personas
        .iter()
        .map(|persona| planned_case_for_persona(persona, &manifest_guard))
        .collect::<Vec<_>>();
    let selected_persona_ids = input
        .selected_personas
        .iter()
        .map(|persona| persona.persona_id.clone())
        .collect::<Vec<_>>();

    let manifest = LiveJourneyRunManifest {
        schema_version: LIVE_JOURNEY_RUNNER_SCHEMA_VERSION.to_string(),
        source_commit: input.request.source_commit,
        guard: manifest_guard.clone(),
        provider_id: input.provider_id.clone(),
        model_id: input.model_id.clone(),
        persona_library_count: input.persona_library_count,
        selected_persona_ids,
        budget: LiveJourneyBudgetSummary {
            max_cases: input.max_cases,
            selected_persona_count: input.selected_personas.len(),
            planned_case_count: planned_cases.len(),
            budget_micros: input.budget_micros,
            estimated_case_cost_micros: ESTIMATED_JOURNEY_CASE_COST_MICROS,
            estimated_total_cost_micros,
        },
        planned_cases,
        redaction_detectors: vec![
            "email".to_string(),
            "phone".to_string(),
            "auth-token-shaped".to_string(),
            "api-key-shaped".to_string(),
            "private_term".to_string(),
        ],
    };

    ensure_manifest_is_safe(&manifest, &input.request.private_terms)?;
    fs::create_dir_all(&input.request.output_dir)?;
    let manifest_path = input.request.output_dir.join("live-journey-manifest.json");
    let encoded = serde_json::to_string_pretty(&manifest)?;
    fs::write(&manifest_path, encoded)?;

    Ok(LiveJourneyRunSummary {
        schema_version: LIVE_JOURNEY_RUNNER_SCHEMA_VERSION.to_string(),
        status: manifest_guard.status.clone(),
        guard: manifest_guard,
        provider_id: input.provider_id,
        model_id: input.model_id,
        persona_library_count: input.persona_library_count,
        selected_persona_count: manifest.budget.selected_persona_count,
        planned_case_count: manifest.budget.planned_case_count,
        budget_micros: Some(input.budget_micros),
        estimated_total_cost_micros,
        manifest_path: Some(manifest_path.to_string_lossy().to_string()),
        message: match manifest.status_label() {
            "allowed" => "live journey cases planned; execution remains deferred to later phases",
            "blocked" => "live journey planning blocked before provider execution",
            "skipped" => "live journey planning skipped before provider execution",
            _ => "live journey planning did not execute provider work",
        }
        .to_string(),
    })
}

impl LiveJourneyRunManifest {
    fn status_label(&self) -> &'static str {
        match self.guard.status {
            LiveEvalStatus::Allowed => "allowed",
            LiveEvalStatus::Blocked => "blocked",
            LiveEvalStatus::Skipped => "skipped",
            LiveEvalStatus::Completed | LiveEvalStatus::Failed => "terminal",
        }
    }
}

pub(crate) fn select_personas(personas: &[EvalPersona], selected_ids: &[String]) -> Result<Vec<EvalPersona>> {
    if selected_ids.is_empty() {
        return Ok(personas.to_vec());
    }

    let by_id = personas
        .iter()
        .map(|persona| (persona.persona_id.as_str(), persona))
        .collect::<BTreeMap<_, _>>();
    let mut selected = Vec::new();
    for id in selected_ids {
        let Some(persona) = by_id.get(id.as_str()) else {
            return Err(anyhow!("unknown live journey persona id {id}"));
        };
        selected.push((*persona).clone());
    }
    Ok(selected)
}

pub(crate) fn planned_case_for_persona(
    persona: &EvalPersona,
    guard: &LiveEvalGuardDecision,
) -> PlannedLiveJourneyCase {
    let case_status = match guard.status {
        LiveEvalStatus::Allowed => LiveJourneyCaseStatus::Planned,
        LiveEvalStatus::Skipped => LiveJourneyCaseStatus::Skipped,
        LiveEvalStatus::Blocked | LiveEvalStatus::Completed | LiveEvalStatus::Failed => {
            LiveJourneyCaseStatus::Blocked
        }
    };
    PlannedLiveJourneyCase {
        case_id: format!("live_journey_{}", persona.persona_id),
        persona_id: persona.persona_id.clone(),
        persona_content_hash: persona.content_hash.clone(),
        person_type: persona.person_type.clone(),
        expected_pressure_subsystems: persona
            .expected_eval_pressure_subsystems
            .iter()
            .map(|subsystem| subsystem.as_str().to_string())
            .collect(),
        status: case_status,
        estimated_case_cost_micros: ESTIMATED_JOURNEY_CASE_COST_MICROS,
        note: "Planning only; QR-to-trial execution uses the separate #165 journey runner."
            .to_string(),
    }
}

pub(crate) fn ensure_manifest_is_safe(
    manifest: &LiveJourneyRunManifest,
    private_terms: &[String],
) -> Result<()> {
    let value = serde_json::to_value(manifest)?;
    ensure!(
        !contains_sensitive_value(&value, private_terms),
        "live journey manifest contains raw sensitive value"
    );
    Ok(())
}

pub fn run_live_openai_eval_from_env(
    db_path: &Path,
    connection: &Connection,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<LiveEvalRunSummary> {
    let (guard, config) = LiveEvalConfig::from_env();
    let Some(config) = config else {
        return Ok(LiveEvalRunSummary::skipped_or_blocked(guard));
    };
    run_live_openai_eval_with_transport(
        db_path,
        connection,
        config,
        ReqwestOpenAiTransport,
        output_dir,
        source_commit,
    )
}

pub fn run_live_openai_eval_with_transport<T: OpenAiCompatibleTransport>(
    db_path: &Path,
    connection: &Connection,
    config: LiveEvalConfig,
    transport: T,
    output_dir: impl Into<PathBuf>,
    source_commit: impl Into<String>,
) -> Result<LiveEvalRunSummary> {
    ensure!(
        config.max_cases >= 1,
        "live eval config must allow at least one case"
    );
    ensure!(
        config.budget_micros >= ESTIMATED_CASE_COST_MICROS,
        "live eval budget is below conservative estimate"
    );
    let guard = LiveEvalGuardDecision {
        status: LiveEvalStatus::Allowed,
        reason: "live LLM eval guards satisfied".to_string(),
        network_enabled: true,
    };
    let start = Instant::now();
    let case = live_openai_compatible_smoke_case()?;
    let packet_path = output_dir
        .into()
        .join(format!("{LIVE_OPENAI_SMOKE_CASE_ID}-packet.json"));
    let provider = OpenAiCompatibleProvider::with_transport(config.openai_config()?, transport);
    let mut harness = DeterministicEvalHarness::new(DeterministicEvalClock::fixed())
        .with_artifact_path(packet_path.to_string_lossy());
    let mut scorecard = harness.run_case(connection, &case, |connection, step| {
        run_live_openai_compatible_smoke_step(db_path, connection, step, &provider)
    })?;
    scorecard.provider_mode = "live_openai_compatible".to_string();
    scorecard.network_enabled = true;
    let output_dir = packet_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let writer = EvalArtifactWriter::new(output_dir, source_commit).with_private_terms(vec![
        "Project Orchid".to_string(),
        "sk-live-fixture".to_string(),
    ]);
    let artifact_paths = writer.write_packet(connection, &case, &scorecard)?;
    let (input_tokens, output_tokens) =
        token_usage_for_invocation(connection, "live_eval_openai_smoke_run")?;
    Ok(completed_summary(
        guard,
        &config,
        scorecard.passed,
        start.elapsed().as_millis(),
        input_tokens,
        output_tokens,
        artifact_paths,
    ))
}

pub(crate) fn completed_summary(
    guard: LiveEvalGuardDecision,
    config: &LiveEvalConfig,
    passed: bool,
    latency_ms: u128,
    input_tokens: i64,
    output_tokens: i64,
    artifact_paths: EvalArtifactPaths,
) -> LiveEvalRunSummary {
    LiveEvalRunSummary {
        schema_version: LIVE_EVAL_RUNNER_SCHEMA_VERSION.to_string(),
        status: if passed {
            LiveEvalStatus::Completed
        } else {
            LiveEvalStatus::Failed
        },
        guard,
        case_id: Some(LIVE_OPENAI_SMOKE_CASE_ID.to_string()),
        provider_id: Some(config.provider_id.clone()),
        model_id: Some(config.model_id.clone()),
        max_cases: Some(config.max_cases),
        budget_micros: Some(config.budget_micros),
        estimated_case_cost_micros: ESTIMATED_CASE_COST_MICROS,
        attempted_cases: 1,
        completed_cases: if passed { 1 } else { 0 },
        latency_ms: Some(latency_ms),
        input_tokens,
        output_tokens,
        packet_path: Some(artifact_paths.packet_path.to_string_lossy().to_string()),
        scorecard_path: Some(artifact_paths.scorecard_path.to_string_lossy().to_string()),
        manifest_path: Some(artifact_paths.manifest_path.to_string_lossy().to_string()),
        message: if passed {
            "live OpenAI-compatible smoke eval completed".to_string()
        } else {
            "live OpenAI-compatible smoke eval completed with failed assertions".to_string()
        },
    }
}

pub(crate) fn live_openai_compatible_smoke_case() -> Result<EvalCase> {
    EvalCase::new(
        LIVE_OPENAI_SMOKE_CASE_ID,
        "Live OpenAI-compatible smoke",
        &json!({
            "fixture": LIVE_OPENAI_SMOKE_CASE_ID,
            "version": 1,
            "providerMode": "live_openai_compatible",
            "networkRequired": true,
            "estimatedCaseCostMicros": ESTIMATED_CASE_COST_MICROS,
        }),
        vec![
            EvalActorRole::Staff,
            EvalActorRole::OrdoAgent,
            EvalActorRole::LlmToolProviderBoundary,
        ],
        vec![EvalStep::new(
            "run_live_openai_compatible_completion",
            EvalActorRole::LlmToolProviderBoundary,
            "llm.run.request.live_openai_compatible",
            vec![
                EvalEvidenceChannel::ConversationEvents,
                EvalEvidenceChannel::PolicyDecisions,
                EvalEvidenceChannel::PromptSlotAccounting,
                EvalEvidenceChannel::PrivacyTransforms,
                EvalEvidenceChannel::TokenLedger,
                EvalEvidenceChannel::RealtimeReplay,
            ],
        )?],
        vec![
            EvalAssertion::minimum_count(
                "policy_decision_recorded",
                EvalEvidenceChannel::PolicyDecisions,
                1,
            )?,
            EvalAssertion::minimum_count(
                "prompt_slots_accounted",
                EvalEvidenceChannel::PromptSlotAccounting,
                1,
            )?,
            EvalAssertion::minimum_count(
                "privacy_transform_recorded",
                EvalEvidenceChannel::PrivacyTransforms,
                1,
            )?,
            EvalAssertion::minimum_count(
                "token_ledger_recorded",
                EvalEvidenceChannel::TokenLedger,
                2,
            )?,
            EvalAssertion::minimum_count(
                "conversation_events_recorded",
                EvalEvidenceChannel::ConversationEvents,
                7,
            )?,
        ],
    )
}

pub(crate) fn run_live_openai_compatible_smoke_step<T: OpenAiCompatibleTransport>(
    db_path: &Path,
    connection: &Connection,
    step: &EvalStep,
    provider: &OpenAiCompatibleProvider<T>,
) -> Result<()> {
    match step.id.as_str() {
        "run_live_openai_compatible_completion" => {
            let (conversation_id, assistant_id) = live_eval_conversation_and_assistant(connection)?;
            let gateway = LlmGateway::new(provider.clone())
                .with_private_terms(vec!["Project Orchid".to_string()]);
            let result = gateway.run_completion(
                db_path,
                connection,
                &ActorContext::local_owner("live_eval_runner"),
                LlmGatewayRequest {
                    run_id: "live_eval_openai_smoke_run".to_string(),
                    conversation_id,
                    segment_id: None,
                    assistant_participant_id: assistant_id,
                    client_id: Some("live-eval-openai-smoke-1".to_string()),
                    provider_id: provider.provider_id().to_string(),
                    model_id: provider.model_id().to_string(),
                    user_message: "Write one short, respectful next-step candidate for Project Orchid. Contact alex@example.com. sk-live-fixture".to_string(),
                    prompt_slots: vec![PromptSlot::new(
                        "conversation_brief",
                        "Conversation Brief",
                        "Evidence: client asked for a concise next step. Do not invent facts.",
                        vec!["live_eval:evidence:conversation_brief".to_string()],
                        "Tiny live eval smoke evidence.",
                        "participants",
                    )?],
                },
            )?;
            ensure!(
                result.final_message.is_some(),
                "live eval provider did not produce a final assistant candidate"
            );
        }
        other => anyhow::bail!("unsupported live eval step: {other}"),
    }
    Ok(())
}

pub(crate) fn live_eval_conversation_and_assistant(connection: &Connection) -> Result<(String, String)> {
    let conversation = find_or_create_canonical_conversation(
        connection,
        &CanonicalConversationRequest {
            surface: "chat".to_string(),
            subject_kind: "connection".to_string(),
            subject_id: "connection_eval_1".to_string(),
            connection_id: Some("connection_eval_1".to_string()),
            visitor_session_id: Some("visitor_session_eval_1".to_string()),
            created_by_actor_id: Some("actor_staff_eval_1".to_string()),
        },
    )?;
    let assistant = create_conversation_participant(
        connection,
        &ConversationParticipantCreateRequest {
            conversation_id: conversation.id.clone(),
            participant_kind: "agent".to_string(),
            actor_id: None,
            connection_id: None,
            visitor_session_id: None,
            display_name: "Ordo".to_string(),
            role: "assistant".to_string(),
        },
    )?;
    Ok((conversation.id, assistant.id))
}

pub(crate) fn token_usage_for_invocation(connection: &Connection, invocation_id: &str) -> Result<(i64, i64)> {
    let input_tokens = token_usage_for_kind(connection, invocation_id, "provider_input")?;
    let output_tokens = token_usage_for_kind(connection, invocation_id, "provider_output")?;
    Ok((input_tokens, output_tokens))
}

pub(crate) fn token_usage_for_kind(
    connection: &Connection,
    invocation_id: &str,
    usage_kind: &str,
) -> Result<i64> {
    Ok(connection.query_row(
        "SELECT COALESCE(SUM(token_count), 0)
         FROM llm_token_ledger_entries
         WHERE invocation_id = ?1 AND usage_kind = ?2",
        params![invocation_id, usage_kind],
        |row| row.get(0),
    )?)
}

