use anyhow::{bail, ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::security::redaction;

const CONTRACT_SCHEMA_VERSION: &str = "ordo.llm_method_contract.v1";
const LOOKUP_AUDIT_SCHEMA_VERSION: &str = "ordo.llm_method_lookup_audit.v1";
const CONTRACT_ONLY_EXECUTION_STATUS: &str = "contract_only";
const READ_ONLY: &str = "read_only";
const MUTATION: &str = "mutation";

const ALLOWED_FAMILIES: &[&str] = &[
    "access",
    "analytics",
    "artifact",
    "claim",
    "content",
    "graph",
    "growth",
    "homepage",
    "image",
    "job",
    "memory",
    "pack",
    "publish",
    "studio",
    "story",
    "support",
    "system",
    "tool",
    "workflow",
];

const DANGEROUS_METHOD_NAMES: &[&str] = &[
    "get_context",
    "query_sql",
    "run_tool",
    "search_database",
    "update_record",
];

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmMethodContractSeed {
    pub name: String,
    pub version: i64,
    pub purpose: String,
    pub authority: String,
    pub viewer_context: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub visibility_ceiling: String,
    pub policy_checks: Vec<String>,
    pub evidence_required: bool,
    pub limitations_required: bool,
    pub access_mode: String,
    pub provider_expectation: String,
    pub live_call_allowed: bool,
    pub events_emitted: Value,
    pub artifact_behavior: Value,
    pub graph_behavior: Value,
    pub deterministic_fixtures: Value,
    pub provenance: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmMethodContractView {
    pub name: String,
    pub family: String,
    pub version: i64,
    pub purpose: String,
    pub authority: String,
    pub viewer_context: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub visibility_ceiling: String,
    pub policy_checks: Vec<String>,
    pub evidence_required: bool,
    pub limitations_required: bool,
    pub access_mode: String,
    pub provider_expectation: String,
    pub live_call_allowed: bool,
    pub execution_status: String,
    pub events_emitted: Value,
    pub artifact_behavior: Value,
    pub graph_behavior: Value,
    pub deterministic_fixtures: Value,
    pub limitations: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmMethodContractList {
    pub contracts: Vec<LlmMethodContractView>,
    pub limitations: Vec<String>,
}

pub fn seed_builtin_llm_method_contracts(connection: &Connection) -> Result<()> {
    for contract in builtin_contracts() {
        register_llm_method_contract(connection, &contract)?;
    }
    Ok(())
}

pub fn register_llm_method_contract(
    connection: &Connection,
    contract: &LlmMethodContractSeed,
) -> Result<LlmMethodContractView> {
    validate_contract(contract)?;

    let family = method_family(&contract.name)?;
    let existing = load_contract_by_name_version(connection, &contract.name, contract.version)?;
    let incoming_fingerprint = contract_fingerprint(contract)?;

    if let Some(existing) = existing {
        let existing_fingerprint: String = connection.query_row(
            "SELECT content_hash FROM llm_method_contracts WHERE name = ?1 AND version = ?2",
            params![contract.name, contract.version],
            |row| row.get(0),
        )?;
        if existing_fingerprint != incoming_fingerprint {
            bail!(
                "LLM method contract {} v{} already exists with a different contract shape",
                contract.name,
                contract.version
            );
        }
        return Ok(existing);
    }

    let now = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO llm_method_contracts (
            id, name, family, version, purpose, authority, viewer_context,
            input_schema_json, output_schema_json, visibility_ceiling,
            policy_checks_json, evidence_required, limitations_required,
            access_mode, provider_expectation, live_call_allowed, execution_status,
            events_emitted_json, artifact_behavior_json, graph_behavior_json,
            deterministic_fixtures_json, provenance_json, content_hash,
            created_at, updated_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
            ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25
         )",
        params![
            contract_id(&contract.name, contract.version),
            contract.name,
            family,
            contract.version,
            contract.purpose,
            contract.authority,
            contract.viewer_context,
            contract.input_schema.to_string(),
            contract.output_schema.to_string(),
            contract.visibility_ceiling,
            serde_json::to_string(&contract.policy_checks)?,
            contract.evidence_required as i64,
            contract.limitations_required as i64,
            contract.access_mode,
            contract.provider_expectation,
            contract.live_call_allowed as i64,
            CONTRACT_ONLY_EXECUTION_STATUS,
            contract.events_emitted.to_string(),
            contract.artifact_behavior.to_string(),
            contract.graph_behavior.to_string(),
            contract.deterministic_fixtures.to_string(),
            contract.provenance.to_string(),
            incoming_fingerprint,
            now,
            now,
        ],
    )?;

    load_contract_by_name_version(connection, &contract.name, contract.version)?
        .ok_or_else(|| anyhow::anyhow!("registered LLM method contract could not be loaded"))
}

pub fn list_llm_method_contracts(db_path: &Path) -> Result<LlmMethodContractList> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT
            name, family, version, purpose, authority, viewer_context,
            input_schema_json, output_schema_json, visibility_ceiling,
            policy_checks_json, evidence_required, limitations_required,
            access_mode, provider_expectation, live_call_allowed, execution_status,
            events_emitted_json, artifact_behavior_json, graph_behavior_json,
            deterministic_fixtures_json, created_at, updated_at
         FROM llm_method_contracts
         ORDER BY family ASC, name ASC, version DESC",
    )?;
    let rows = statement.query_map([], contract_from_row)?;
    let contracts = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(LlmMethodContractList {
        contracts,
        limitations: safe_metadata_limitations(),
    })
}

pub fn lookup_llm_method_contract(
    db_path: &Path,
    method_name: &str,
    viewer_context: Value,
) -> Result<Option<LlmMethodContractView>> {
    validate_method_name(method_name)?;
    let connection = Connection::open(db_path)?;
    let contract = load_latest_contract_by_name(&connection, method_name)?;
    record_lookup_audit(&connection, method_name, &viewer_context, contract.as_ref())?;
    Ok(contract)
}

fn load_contract_by_name_version(
    connection: &Connection,
    name: &str,
    version: i64,
) -> Result<Option<LlmMethodContractView>> {
    connection
        .query_row(
            "SELECT
                name, family, version, purpose, authority, viewer_context,
                input_schema_json, output_schema_json, visibility_ceiling,
                policy_checks_json, evidence_required, limitations_required,
                access_mode, provider_expectation, live_call_allowed, execution_status,
                events_emitted_json, artifact_behavior_json, graph_behavior_json,
                deterministic_fixtures_json, created_at, updated_at
             FROM llm_method_contracts
             WHERE name = ?1 AND version = ?2",
            params![name, version],
            contract_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn load_latest_contract_by_name(
    connection: &Connection,
    name: &str,
) -> Result<Option<LlmMethodContractView>> {
    connection
        .query_row(
            "SELECT
                name, family, version, purpose, authority, viewer_context,
                input_schema_json, output_schema_json, visibility_ceiling,
                policy_checks_json, evidence_required, limitations_required,
                access_mode, provider_expectation, live_call_allowed, execution_status,
                events_emitted_json, artifact_behavior_json, graph_behavior_json,
                deterministic_fixtures_json, created_at, updated_at
             FROM llm_method_contracts
             WHERE name = ?1
             ORDER BY version DESC
             LIMIT 1",
            [name],
            contract_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn contract_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LlmMethodContractView> {
    let policy_checks_json: String = row.get(9)?;
    Ok(LlmMethodContractView {
        name: row.get(0)?,
        family: row.get(1)?,
        version: row.get(2)?,
        purpose: row.get(3)?,
        authority: row.get(4)?,
        viewer_context: row.get(5)?,
        input_schema: parse_json_row(row, 6)?,
        output_schema: parse_json_row(row, 7)?,
        visibility_ceiling: row.get(8)?,
        policy_checks: serde_json::from_str(&policy_checks_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                9,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        evidence_required: row.get::<_, i64>(10)? == 1,
        limitations_required: row.get::<_, i64>(11)? == 1,
        access_mode: row.get(12)?,
        provider_expectation: row.get(13)?,
        live_call_allowed: row.get::<_, i64>(14)? == 1,
        execution_status: row.get(15)?,
        events_emitted: parse_json_row(row, 16)?,
        artifact_behavior: parse_json_row(row, 17)?,
        graph_behavior: parse_json_row(row, 18)?,
        deterministic_fixtures: parse_json_row(row, 19)?,
        limitations: safe_metadata_limitations(),
        created_at: row.get(20)?,
        updated_at: row.get(21)?,
    })
}

fn parse_json_row(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Value> {
    let raw: String = row.get(index)?;
    serde_json::from_str(&raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn validate_contract(contract: &LlmMethodContractSeed) -> Result<()> {
    validate_method_name(&contract.name)?;
    ensure!(
        contract.version > 0,
        "method contract version must be positive"
    );
    ensure!(
        !contract.purpose.trim().is_empty(),
        "method contract purpose is required"
    );
    ensure!(
        !contract.authority.trim().is_empty(),
        "method contract authority is required"
    );
    ensure!(
        !contract.viewer_context.trim().is_empty(),
        "method contract viewer context is required"
    );
    ensure!(
        contract.input_schema.is_object(),
        "method contract input schema must be an object"
    );
    ensure!(
        contract.output_schema.is_object(),
        "method contract output schema must be an object"
    );
    ensure!(
        matches!(
            contract.visibility_ceiling.as_str(),
            "public" | "authenticated" | "staff" | "owner"
        ),
        "method contract visibility ceiling must be public, authenticated, staff, or owner"
    );
    ensure!(
        !contract.policy_checks.is_empty(),
        "method contract policy checks are required"
    );
    ensure!(
        contract.evidence_required,
        "method contract must require evidence refs"
    );
    ensure!(
        contract.limitations_required,
        "method contract must require limitations"
    );
    ensure!(
        matches!(contract.access_mode.as_str(), READ_ONLY | MUTATION),
        "method contract access mode must be read_only or mutation"
    );
    ensure!(
        !contract.provider_expectation.trim().is_empty(),
        "method contract provider expectation is required"
    );
    ensure!(
        !contract.live_call_allowed,
        "default method contracts must not allow live provider calls"
    );
    Ok(())
}

fn validate_method_name(name: &str) -> Result<()> {
    let trimmed = name.trim();
    ensure!(!trimmed.is_empty(), "method name is required");
    ensure!(
        trimmed == name,
        "method name must not contain surrounding whitespace"
    );

    if DANGEROUS_METHOD_NAMES.contains(&trimmed) {
        bail!("dangerous generic method name is not allowed: {trimmed}");
    }

    let parts: Vec<&str> = trimmed.split('.').collect();
    ensure!(
        parts.len() == 2,
        "method name must use an explicit product-shaped family prefix"
    );
    let family = parts[0];
    let method = parts[1];
    ensure!(
        ALLOWED_FAMILIES.contains(&family),
        "method family is not supported: {family}"
    );
    ensure!(
        !DANGEROUS_METHOD_NAMES.contains(&method),
        "dangerous generic method name is not allowed: {method}"
    );
    ensure!(
        !method.is_empty()
            && method
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_'),
        "method name must contain only ASCII letters, numbers, and underscores after the family"
    );
    Ok(())
}

fn method_family(name: &str) -> Result<&str> {
    validate_method_name(name)?;
    Ok(name
        .split_once('.')
        .map(|(family, _)| family)
        .expect("validated method name has family"))
}

fn record_lookup_audit(
    connection: &Connection,
    method_name: &str,
    viewer_context: &Value,
    contract: Option<&LlmMethodContractView>,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let safe_viewer_context = sanitize_viewer_context(viewer_context.clone());
    let viewer_context_json = safe_viewer_context.to_string();
    let output = match contract {
        Some(contract) => json!({
            "status": "found",
            "methodName": contract.name,
            "version": contract.version,
            "visibilityCeiling": contract.visibility_ceiling,
            "executionStatus": contract.execution_status,
        }),
        None => json!({
            "status": "missing",
            "methodName": method_name,
        }),
    };
    connection.execute(
        "INSERT INTO llm_method_contract_lookup_audit (
            id, method_name, viewer_context_json, input_hash, output_hash,
            result_status, schema_version, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            lookup_audit_id(method_name, &viewer_context_json, &output.to_string(), &now),
            method_name,
            viewer_context_json,
            stable_hash(&format!("{method_name}|{safe_viewer_context}")),
            stable_hash(&output.to_string()),
            if contract.is_some() {
                "found"
            } else {
                "missing"
            },
            LOOKUP_AUDIT_SCHEMA_VERSION,
            now,
        ],
    )?;
    Ok(())
}

fn sanitize_viewer_context(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| {
                    if is_sensitive_context_key(&key) {
                        (
                            "redactedContextField".to_string(),
                            json!("[REDACTED_CONTEXT]"),
                        )
                    } else {
                        (key, sanitize_viewer_context(value))
                    }
                })
                .collect(),
        ),
        Value::Array(values) => {
            Value::Array(values.into_iter().map(sanitize_viewer_context).collect())
        }
        other => redaction::sanitize_json_strings(other),
    }
}

fn is_sensitive_context_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    [
        "apikey",
        "compiledplanprivateinputs",
        "ownersonly",
        "owneronly",
        "password",
        "policyinternal",
        "privateartifacttext",
        "privatepayload",
        "promptinternal",
        "providerinternal",
        "providersecret",
        "rawpolicyinternal",
        "secret",
        "taskprivatepayload",
        "token",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn contract_id(name: &str, version: i64) -> String {
    format!(
        "llm_method_contract_{}",
        stable_hash(&format!("{name}|{version}"))
    )
}

fn lookup_audit_id(method_name: &str, input: &str, output: &str, created_at: &str) -> String {
    format!(
        "llm_method_lookup_{}",
        stable_hash(&format!("{method_name}|{input}|{output}|{created_at}"))
    )
}

fn contract_fingerprint(contract: &LlmMethodContractSeed) -> Result<String> {
    let canonical = json!({
        "schemaVersion": CONTRACT_SCHEMA_VERSION,
        "name": contract.name,
        "version": contract.version,
        "purpose": contract.purpose,
        "authority": contract.authority,
        "viewerContext": contract.viewer_context,
        "inputSchema": contract.input_schema,
        "outputSchema": contract.output_schema,
        "visibilityCeiling": contract.visibility_ceiling,
        "policyChecks": contract.policy_checks,
        "evidenceRequired": contract.evidence_required,
        "limitationsRequired": contract.limitations_required,
        "accessMode": contract.access_mode,
        "providerExpectation": contract.provider_expectation,
        "liveCallAllowed": contract.live_call_allowed,
        "eventsEmitted": contract.events_emitted,
        "artifactBehavior": contract.artifact_behavior,
        "graphBehavior": contract.graph_behavior,
        "deterministicFixtures": contract.deterministic_fixtures,
    });
    Ok(stable_hash(&serde_json::to_string(&canonical)?))
}

fn stable_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn safe_metadata_limitations() -> Vec<String> {
    vec![
        "Contract metadata only; no method execution is performed.".to_string(),
        "Prompt internals, provider secrets, raw policy internals, owner-only data, and private artifact text are not exposed.".to_string(),
    ]
}

fn builtin_contracts() -> Vec<LlmMethodContractSeed> {
    vec![
        read_contract(
            "graph.get_resource_neighborhood",
            "Return a bounded, access-aware graph neighborhood for a resource.",
            "graph_read",
            "staff",
            json!({"type": "object", "required": ["resourceKind", "resourceId"], "properties": {"resourceKind": {"type": "string"}, "resourceId": {"type": "string"}, "depth": {"type": "integer", "maximum": 1}}}),
            json!({"type": "object", "required": ["status", "nodes", "edges", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "nodes": {"type": "array"}, "edges": {"type": "array"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "none"}}}),
            json!({"reads": ["graph_nodes", "graph_edges", "graph_edge_evidence"], "writes": ["graph_query_audit"]}),
        ),
        read_contract(
            "claim.validate_public_claim",
            "Check whether a public claim has sufficient approved evidence.",
            "claim_read",
            "public",
            json!({"type": "object", "required": ["claimText"], "properties": {"claimText": {"type": "string"}, "surface": {"type": "string"}}}),
            json!({"type": "object", "required": ["status", "summary", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "summary": {"type": "string"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"const": "public"}, "memoryEffect": {"const": "none"}}}),
            json!({"reads": ["business_facts", "artifacts", "graph_nodes"], "writes": []}),
        ),
        mutation_contract(
            "homepage.createNarrativeDeck",
            "Create a governed narrative deck draft from public-safe Story Pack evidence.",
            "homepage_story_mutation",
            "staff",
            json!({"type": "object", "required": ["businessPositioning", "evidenceRefs"], "properties": {"businessPositioning": {"type": "object"}, "founderProfile": {"type": "object"}, "evidenceRefs": {"type": "object"}, "limitations": {"type": "object"}}}),
            json!({"type": "object", "required": ["status", "deckArtifactRef", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "deckArtifactRef": {"type": "string"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "candidate"}}}),
            json!({"reads": ["business_facts", "artifacts"], "writes": ["artifacts"]}),
            true,
        ),
        read_contract(
            "homepage.prepare_image_briefs",
            "Prepare public-safe image brief metadata for homepage story sections.",
            "homepage_story_read",
            "staff",
            json!({"type": "object", "required": ["storyDeckId"], "properties": {"storyDeckId": {"type": "string"}, "sectionIds": {"type": "array", "items": {"type": "string"}}}}),
            json!({"type": "object", "required": ["status", "briefs", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "briefs": {"type": "array"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "none"}}}),
            json!({"reads": ["future_homepage_story_deck", "artifacts"], "writes": []}),
        ),
        mutation_contract(
            "story.createImageBriefs",
            "Create typed Story image brief artifacts for bounded homepage sections.",
            "story_artifact_mutation",
            "staff",
            json!({"type": "object", "required": ["section"], "properties": {"section": {"type": "object"}, "evidenceRefs": {"type": "object"}}}),
            json!({"type": "object", "required": ["status", "briefArtifactRef", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "briefArtifactRef": {"type": "string"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "candidate"}}}),
            json!({"reads": ["artifacts", "business_facts"], "writes": ["artifacts"]}),
            true,
        ),
        mutation_contract(
            "image.generateVariants",
            "Record deterministic image generation request envelopes and generated candidate artifact refs.",
            "image_artifact_mutation",
            "staff",
            json!({"type": "object", "required": ["section", "briefArtifactRef"], "properties": {"section": {"type": "object"}, "briefArtifactRef": {"type": "object"}, "evidenceRefs": {"type": "object"}, "limitations": {"type": "object"}}}),
            json!({"type": "object", "required": ["status", "requestEnvelopeArtifactRef", "candidateArtifactRefs", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "requestEnvelopeArtifactRef": {"type": "string"}, "candidateArtifactRefs": {"type": "array"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "candidate"}}}),
            json!({"reads": ["artifacts"], "writes": ["artifacts"]}),
            true,
        ),
        read_contract(
            "pack.inspect_manifest",
            "Inspect a product pack manifest for declared permissions, bindings, and review needs.",
            "pack_read",
            "staff",
            json!({"type": "object", "required": ["packId"], "properties": {"packId": {"type": "string"}}}),
            json!({"type": "object", "required": ["status", "summary", "bindings", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "summary": {"type": "string"}, "bindings": {"type": "array"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "none"}}}),
            json!({"reads": ["product_packs", "product_pack_bindings"], "writes": []}),
        ),
        read_contract(
            "workflow.resolveVariables",
            "Explain typed workflow variable resolution without exposing private compiled-plan inputs.",
            "workflow_read",
            "staff",
            json!({"type": "object", "required": ["templateId", "variables"], "properties": {"templateId": {"type": "string"}, "variables": {"type": "object"}}}),
            json!({"type": "object", "required": ["status", "resolvedVariables", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "resolvedVariables": {"type": "array"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "none"}}}),
            json!({"reads": ["process_templates", "jobs"], "writes": []}),
        ),
        mutation_contract(
            "image.reviewAgainstBrief",
            "Review generated image candidates against an approved image brief without invoking a live provider.",
            "image_review_mutation",
            "staff",
            json!({"type": "object", "required": ["artifactId", "briefArtifactId"], "properties": {"artifactId": {"type": "string"}, "briefArtifactId": {"type": "string"}}}),
            json!({"type": "object", "required": ["status", "summary", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "summary": {"type": "string"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "candidate"}}}),
            json!({"reads": ["artifacts", "artifact_versions"], "writes": ["artifacts"]}),
            true,
        ),
        mutation_contract(
            "artifact.preparePublicDerivative",
            "Prepare a public-safe derivative from approved generated Story artifacts.",
            "artifact_mutation",
            "staff",
            json!({"type": "object", "required": ["candidateArtifactRef", "reviewArtifactRef"], "properties": {"candidateArtifactRef": {"type": "object"}, "reviewArtifactRef": {"type": "object"}, "visibility": {"type": "string"}, "evidenceRefs": {"type": "object"}}}),
            json!({"type": "object", "required": ["status", "publicDerivativeArtifactRef", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "publicDerivativeArtifactRef": {"type": "string"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "candidate"}}}),
            json!({"reads": ["artifacts", "artifact_versions"], "writes": ["artifacts"]}),
            true,
        ),
        mutation_contract(
            "homepage.compileScrollytellingDraft",
            "Compile a Story homepage draft from deck and public derivative artifact refs.",
            "homepage_story_mutation",
            "staff",
            json!({"type": "object", "required": ["deckArtifactRef", "sectionDerivativeRefs"], "properties": {"deckArtifactRef": {"type": "object"}, "sectionDerivativeRefs": {"type": "object"}, "evidenceRefs": {"type": "object"}, "limitations": {"type": "object"}}}),
            json!({"type": "object", "required": ["status", "homepageVersionArtifactRef", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "homepageVersionArtifactRef": {"type": "string"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "candidate"}}}),
            json!({"reads": ["artifacts", "business_facts"], "writes": ["artifacts"]}),
            true,
        ),
        mutation_contract(
            "publish.requestApproval",
            "Request governed manual publish approval without external publishing authority.",
            "publish_mutation",
            "staff",
            json!({"type": "object", "required": ["publishMode", "sourceArtifactRefs", "evidenceRefs"], "properties": {"publishMode": {"type": "object"}, "sourceArtifactRefs": {"type": "object"}, "evidenceRefs": {"type": "object"}, "readinessMissing": {"type": "object"}}}),
            json!({"type": "object", "required": ["status", "approvalArtifactRef", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "approvalArtifactRef": {"type": "string"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "candidate"}}}),
            json!({"reads": ["artifacts"], "writes": ["artifacts"]}),
            true,
        ),
        mutation_contract(
            "analytics.recordContentEvent",
            "Record local-first content analytics events with explicit limitations.",
            "analytics_mutation",
            "staff",
            json!({"type": "object", "required": ["contentRef", "eventKind", "surface", "evidenceRefs"], "properties": {"contentRef": {"type": "object"}, "eventKind": {"type": "string"}, "surface": {"type": "string"}, "evidenceRefs": {"type": "object"}, "limitations": {"type": "array"}}}),
            json!({"type": "object", "required": ["status", "analyticsEventRef", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "analyticsEventRef": {"type": "string"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "none"}}}),
            json!({"reads": ["artifacts"], "writes": ["content_analytics_events"]}),
            false,
        ),
        mutation_contract(
            "memory.proposeCandidateClaims",
            "Propose generated-content memory candidates without promoting graph truth.",
            "memory_mutation",
            "staff",
            json!({"type": "object", "required": ["sourceArtifactRefs", "evidenceRefs"], "properties": {"sourceArtifactRefs": {"type": "array"}, "evidenceRefs": {"type": "object"}, "limitations": {"type": "object"}, "memoryEffect": {"type": "string"}}}),
            json!({"type": "object", "required": ["status", "candidateRefs", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "candidateRefs": {"type": "array"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "candidate"}}}),
            json!({"reads": ["artifacts", "content_analytics_events"], "writes": ["generated_content_memory_candidates"]}),
            false,
        ),
        read_contract(
            "memory.prepareReviewPacket",
            "Prepare an authorized generated-content memory review packet without exposing candidate text publicly.",
            "memory_read",
            "staff",
            json!({"type": "object", "required": ["sourceArtifactRefs", "audience", "evidenceRefs"], "properties": {"sourceArtifactRefs": {"type": "array"}, "audience": {"type": "string"}, "evidenceRefs": {"type": "object"}, "memoryPromotionAllowed": {"type": "boolean"}}}),
            json!({"type": "object", "required": ["status", "reviewPacket", "evidenceRefs", "limitations", "visibilityClass", "memoryEffect"], "properties": {"status": {"type": "string"}, "reviewPacket": {"type": "object"}, "evidenceRefs": {"type": "array"}, "limitations": {"type": "array"}, "visibilityClass": {"type": "string"}, "memoryEffect": {"const": "none"}}}),
            json!({"reads": ["generated_content_memory_candidates", "artifacts"], "writes": []}),
        ),
    ]
}

fn read_contract(
    name: &str,
    purpose: &str,
    authority: &str,
    visibility_ceiling: &str,
    input_schema: Value,
    output_schema: Value,
    evidence_sources: Value,
) -> LlmMethodContractSeed {
    method_contract(
        name,
        purpose,
        authority,
        visibility_ceiling,
        input_schema,
        output_schema,
        evidence_sources,
        READ_ONLY,
        false,
    )
}

fn mutation_contract(
    name: &str,
    purpose: &str,
    authority: &str,
    visibility_ceiling: &str,
    input_schema: Value,
    output_schema: Value,
    evidence_sources: Value,
    creates_artifacts: bool,
) -> LlmMethodContractSeed {
    method_contract(
        name,
        purpose,
        authority,
        visibility_ceiling,
        input_schema,
        output_schema,
        evidence_sources,
        MUTATION,
        creates_artifacts,
    )
}

fn method_contract(
    name: &str,
    purpose: &str,
    authority: &str,
    visibility_ceiling: &str,
    input_schema: Value,
    output_schema: Value,
    evidence_sources: Value,
    access_mode: &str,
    creates_artifacts: bool,
) -> LlmMethodContractSeed {
    LlmMethodContractSeed {
        name: name.to_string(),
        version: 1,
        purpose: purpose.to_string(),
        authority: authority.to_string(),
        viewer_context: "role_and_surface_context_required".to_string(),
        input_schema,
        output_schema,
        visibility_ceiling: visibility_ceiling.to_string(),
        policy_checks: vec![
            "capability_registered".to_string(),
            "viewer_visibility_ceiling".to_string(),
            "evidence_refs_required".to_string(),
        ],
        evidence_required: true,
        limitations_required: true,
        access_mode: access_mode.to_string(),
        provider_expectation: "no_live_provider_contract_metadata_only".to_string(),
        live_call_allowed: false,
        events_emitted: json!([]),
        artifact_behavior: json!({"createsArtifacts": creates_artifacts, "exposesPrivateText": false}),
        graph_behavior: evidence_sources,
        deterministic_fixtures: json!({"defaultValidation": "sqlite_fixture_only"}),
        provenance: json!({
            "schemaVersion": CONTRACT_SCHEMA_VERSION,
            "source": "docs/architecture/llm-method-contracts.md",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn changed_graph_contract() -> LlmMethodContractSeed {
        let mut contract = builtin_contracts()
            .into_iter()
            .find(|contract| contract.name == "graph.get_resource_neighborhood")
            .unwrap();
        contract.authority = "changed_authority".to_string();
        contract
    }

    #[test]
    fn method_name_validation_rejects_generic_or_dangerous_names() {
        let connection = Connection::open_in_memory().unwrap();
        crate::schema::init_schema(&connection).unwrap();

        for name in [
            "query_sql",
            "search_database",
            "get_context",
            "run_tool",
            "update_record",
            "tool.run_tool",
            "database.search",
        ] {
            let mut contract = builtin_contracts()[0].clone();
            contract.name = name.to_string();
            let error = register_llm_method_contract(&connection, &contract)
                .expect_err("dangerous method name should be rejected");
            assert!(
                error.to_string().contains("method"),
                "unexpected error for {name}: {error}"
            );
        }
    }

    #[test]
    fn seed_registers_safe_contract_metadata_without_execution_claims() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("ordo-test.sqlite3");
        crate::schema::init_database(&db_path).unwrap();

        let listed = list_llm_method_contracts(&db_path).unwrap();
        let names: Vec<String> = listed
            .contracts
            .iter()
            .map(|contract| contract.name.clone())
            .collect();

        assert!(names.contains(&"graph.get_resource_neighborhood".to_string()));
        assert!(names.contains(&"claim.validate_public_claim".to_string()));
        assert!(names.contains(&"homepage.prepare_image_briefs".to_string()));
        assert!(names.contains(&"pack.inspect_manifest".to_string()));
        assert!(names.contains(&"workflow.resolveVariables".to_string()));
        assert!(names.contains(&"image.reviewAgainstBrief".to_string()));

        for contract in listed.contracts {
            assert_eq!(contract.execution_status, CONTRACT_ONLY_EXECUTION_STATUS);
            assert!(!contract.live_call_allowed);
            assert!(contract.evidence_required);
            assert!(contract.limitations_required);
            assert!(!contract.policy_checks.is_empty());
            assert!(contract
                .limitations
                .iter()
                .any(|limitation| limitation.contains("Prompt internals, provider secrets")));
            assert_ne!(contract.provider_expectation, "live_provider");
        }
    }

    #[test]
    fn story_workflow_method_contracts_preserve_mutation_boundaries() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("ordo-test.sqlite3");
        crate::schema::init_database(&db_path).unwrap();

        let listed = list_llm_method_contracts(&db_path).unwrap();
        let contract = |name: &str| {
            listed
                .contracts
                .iter()
                .find(|contract| contract.name == name)
                .unwrap_or_else(|| panic!("missing contract {name}"))
        };

        for name in [
            "homepage.createNarrativeDeck",
            "story.createImageBriefs",
            "image.generateVariants",
            "image.reviewAgainstBrief",
            "artifact.preparePublicDerivative",
            "homepage.compileScrollytellingDraft",
            "publish.requestApproval",
            "analytics.recordContentEvent",
            "memory.proposeCandidateClaims",
        ] {
            let contract = contract(name);
            assert_eq!(contract.access_mode, MUTATION, "{name} must be mutation");
            assert!(
                contract.graph_behavior["writes"]
                    .as_array()
                    .is_some_and(|writes| !writes.is_empty()),
                "{name} must declare write targets"
            );
        }

        for name in [
            "homepage.createNarrativeDeck",
            "story.createImageBriefs",
            "image.generateVariants",
            "image.reviewAgainstBrief",
            "artifact.preparePublicDerivative",
            "homepage.compileScrollytellingDraft",
            "publish.requestApproval",
        ] {
            assert_eq!(
                contract(name).artifact_behavior["createsArtifacts"],
                json!(true),
                "{name} must declare artifact creation"
            );
        }

        let memory_review = contract("memory.prepareReviewPacket");
        assert_eq!(memory_review.access_mode, READ_ONLY);
        assert_eq!(
            memory_review.artifact_behavior["createsArtifacts"],
            json!(false)
        );
    }

    #[test]
    fn identical_reregister_is_idempotent_but_shape_change_rejects() {
        let connection = Connection::open_in_memory().unwrap();
        crate::schema::init_schema(&connection).unwrap();
        let contract = builtin_contracts()[0].clone();

        register_llm_method_contract(&connection, &contract).unwrap();
        register_llm_method_contract(&connection, &contract).unwrap();

        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM llm_method_contracts WHERE name = ?1",
                [&contract.name],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        let error = register_llm_method_contract(&connection, &changed_graph_contract())
            .expect_err("changed contract shape without version change should reject");
        assert!(error.to_string().contains("different contract shape"));
    }

    #[test]
    fn lookup_returns_safe_metadata_and_records_audit() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("ordo-test.sqlite3");
        crate::schema::init_database(&db_path).unwrap();

        let contract = lookup_llm_method_contract(
            &db_path,
            "graph.get_resource_neighborhood",
            json!({
                "viewerRole": "staff",
                "surface": "studio",
                "providerSecret": "sk-test-secret-value",
                "promptInternal": "do not store raw prompt notes",
                "nested": {
                    "privateArtifactText": "Project Orchid private artifact text"
                }
            }),
        )
        .unwrap()
        .expect("seeded contract should be found");

        assert_eq!(contract.name, "graph.get_resource_neighborhood");
        assert_eq!(contract.access_mode, READ_ONLY);
        assert_eq!(contract.graph_behavior["reads"][0], "graph_nodes");
        assert_eq!(contract.input_schema["required"][0], "resourceKind");
        assert_eq!(contract.output_schema["required"][0], "status");
        assert!(contract
            .limitations
            .iter()
            .any(|limitation| limitation.contains("Contract metadata only")));

        let connection = Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM llm_method_contract_lookup_audit
                 WHERE method_name = 'graph.get_resource_neighborhood'
                   AND result_status = 'found'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 1);

        let stored_context: String = connection
            .query_row(
                "SELECT viewer_context_json FROM llm_method_contract_lookup_audit
                 WHERE method_name = 'graph.get_resource_neighborhood'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(stored_context.contains("viewerRole"));
        assert!(!stored_context.contains("providerSecret"));
        assert!(!stored_context.contains("sk-test-secret-value"));
        assert!(!stored_context.contains("promptInternal"));
        assert!(!stored_context.contains("do not store raw prompt notes"));
        assert!(!stored_context.contains("privateArtifactText"));
        assert!(!stored_context.contains("Project Orchid"));
        assert!(stored_context.contains("redactedContextField"));
    }

    #[test]
    fn missing_contract_lookup_audits_without_creating_contract() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("ordo-test.sqlite3");
        crate::schema::init_database(&db_path).unwrap();

        let missing =
            lookup_llm_method_contract(&db_path, "graph.find_evidence_path", json!({})).unwrap();
        assert!(missing.is_none());

        let connection = Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM llm_method_contract_lookup_audit
                 WHERE method_name = 'graph.find_evidence_path'
                   AND result_status = 'missing'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 1);

        let contract_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM llm_method_contracts
                 WHERE name = 'graph.find_evidence_path'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(contract_count, 0);
    }
}
