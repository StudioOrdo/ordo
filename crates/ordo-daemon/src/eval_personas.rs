use anyhow::{anyhow, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::eval_simulators::EvalPressureSubsystem;

pub const EVAL_PERSONA_SCHEMA_VERSION: &str = "ordo.live_eval_persona.v1";
pub const MINIMUM_COMMITTED_PERSONA_COUNT: usize = 10;

const REQUIRED_FIELDS: &[&str] = &[
    "schema_version",
    "persona_id",
    "display_name",
    "person_type",
    "event_context",
    "business_context",
    "personality_traits",
    "communication_style",
    "goals",
    "objections",
    "budget_sensitivity",
    "urgency_level",
    "privacy_sensitivity",
    "referral_tendency",
    "review_likelihood",
    "handoff_likelihood",
    "unsafe_or_edge_case_behaviors",
    "offer_interest",
    "trial_success_criteria",
    "expected_eval_pressure_subsystems",
    "ethical_persuasion_allowed_principles",
    "redaction_notes",
];

const ALLOWED_PERSON_TYPES: &[&str] = &[
    "solo_consultant",
    "local_service_business_owner",
    "creative_freelancer",
    "agency_operator",
    "nonprofit_community_organizer",
    "skeptical_technical_founder",
    "privacy_sensitive_professional",
    "budget_constrained_early_adopter",
    "affiliate_referrer",
    "dissatisfied_trial_user",
];

const ALLOWED_LEVELS: &[&str] = &["low", "medium", "high"];
const ALLOWED_REVIEW_LIKELIHOODS: &[&str] =
    &["unlikely", "low_until_value_is_clear", "medium", "high"];
const ALLOWED_PERSUASION_PRINCIPLES: &[&str] = &[
    "reciprocity",
    "commitment_consistency",
    "social_proof",
    "authority",
    "liking",
    "scarcity",
    "unity",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalPersona {
    pub schema_version: String,
    pub persona_id: String,
    pub display_name: String,
    pub person_type: String,
    pub event_context: String,
    pub business_context: String,
    pub personality_traits: Vec<String>,
    pub communication_style: String,
    pub goals: Vec<String>,
    pub objections: Vec<String>,
    pub budget_sensitivity: String,
    pub urgency_level: String,
    pub privacy_sensitivity: String,
    pub referral_tendency: String,
    pub review_likelihood: String,
    pub handoff_likelihood: String,
    pub unsafe_or_edge_case_behaviors: Vec<String>,
    pub offer_interest: String,
    pub trial_success_criteria: Vec<String>,
    pub expected_eval_pressure_subsystems: Vec<EvalPressureSubsystem>,
    pub ethical_persuasion_allowed_principles: Vec<String>,
    pub redaction_notes: Vec<String>,
    pub narrative_markdown: String,
    pub source_path: String,
    pub content_hash: String,
}

impl EvalPersona {
    pub fn validate(&self, private_terms: &[String]) -> Result<()> {
        ensure!(
            self.schema_version == EVAL_PERSONA_SCHEMA_VERSION,
            "unsupported persona schema version"
        );
        require_text("persona id", &self.persona_id)?;
        require_text("display name", &self.display_name)?;
        require_allowed("person type", &self.person_type, ALLOWED_PERSON_TYPES)?;
        require_text("event context", &self.event_context)?;
        require_text("business context", &self.business_context)?;
        require_nonempty_list("personality traits", &self.personality_traits)?;
        require_text("communication style", &self.communication_style)?;
        require_nonempty_list("goals", &self.goals)?;
        require_nonempty_list("objections", &self.objections)?;
        require_allowed(
            "budget sensitivity",
            &self.budget_sensitivity,
            ALLOWED_LEVELS,
        )?;
        require_allowed("urgency level", &self.urgency_level, ALLOWED_LEVELS)?;
        require_allowed(
            "privacy sensitivity",
            &self.privacy_sensitivity,
            ALLOWED_LEVELS,
        )?;
        require_allowed("referral tendency", &self.referral_tendency, ALLOWED_LEVELS)?;
        require_allowed(
            "review likelihood",
            &self.review_likelihood,
            ALLOWED_REVIEW_LIKELIHOODS,
        )?;
        require_allowed(
            "handoff likelihood",
            &self.handoff_likelihood,
            ALLOWED_LEVELS,
        )?;
        require_nonempty_list(
            "unsafe or edge-case behaviors",
            &self.unsafe_or_edge_case_behaviors,
        )?;
        require_text("offer interest", &self.offer_interest)?;
        require_nonempty_list("trial success criteria", &self.trial_success_criteria)?;
        ensure!(
            !self.expected_eval_pressure_subsystems.is_empty(),
            "expected eval pressure subsystems are required"
        );
        require_nonempty_list(
            "ethical persuasion allowed principles",
            &self.ethical_persuasion_allowed_principles,
        )?;
        for principle in &self.ethical_persuasion_allowed_principles {
            require_allowed(
                "ethical persuasion allowed principle",
                principle,
                ALLOWED_PERSUASION_PRINCIPLES,
            )?;
        }
        require_nonempty_list("redaction notes", &self.redaction_notes)?;
        require_text("persona narrative markdown", &self.narrative_markdown)?;
        ensure!(
            self.content_hash.starts_with("sha256:"),
            "persona content hash must be sha256-prefixed"
        );

        let encoded = serde_json::to_value(self)?;
        ensure!(
            !contains_sensitive_value(&encoded, private_terms),
            "persona contains raw sensitive value"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalPersonaValidationError {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalPersonaLibraryValidation {
    pub schema_version: String,
    pub persona_count: usize,
    pub persona_ids: Vec<String>,
    pub content_hash: String,
    pub errors: Vec<EvalPersonaValidationError>,
}

pub fn load_persona_file(path: &Path, private_terms: &[String]) -> Result<EvalPersona> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read eval persona {}", path.display()))?;
    parse_persona_markdown(&raw, path, private_terms)
}

pub fn load_persona_dir(path: &Path, private_terms: &[String]) -> Result<Vec<EvalPersona>> {
    let mut markdown_paths = fs::read_dir(path)
        .with_context(|| format!("read eval persona directory {}", path.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("README.md"))
        .collect::<Vec<_>>();
    markdown_paths.sort();

    let mut personas = markdown_paths
        .iter()
        .map(|path| load_persona_file(path, private_terms))
        .collect::<Result<Vec<_>>>()?;
    personas.sort_by(|left, right| left.persona_id.cmp(&right.persona_id));

    ensure_unique_persona_ids(&personas)?;
    Ok(personas)
}

pub fn validate_persona_library(
    path: &Path,
    private_terms: &[String],
) -> EvalPersonaLibraryValidation {
    match load_persona_dir(path, private_terms) {
        Ok(personas) => {
            let mut errors = Vec::new();
            if personas.len() < MINIMUM_COMMITTED_PERSONA_COUNT {
                errors.push(EvalPersonaValidationError {
                    path: path.to_string_lossy().to_string(),
                    message: format!(
                        "persona library must include at least {MINIMUM_COMMITTED_PERSONA_COUNT} personas"
                    ),
                });
            }
            let persona_ids = personas
                .iter()
                .map(|persona| persona.persona_id.clone())
                .collect::<Vec<_>>();
            let content_hash = persona_library_hash(&personas).unwrap_or_else(|error| {
                errors.push(EvalPersonaValidationError {
                    path: path.to_string_lossy().to_string(),
                    message: error.to_string(),
                });
                "sha256:error".to_string()
            });
            EvalPersonaLibraryValidation {
                schema_version: EVAL_PERSONA_SCHEMA_VERSION.to_string(),
                persona_count: personas.len(),
                persona_ids,
                content_hash,
                errors,
            }
        }
        Err(error) => EvalPersonaLibraryValidation {
            schema_version: EVAL_PERSONA_SCHEMA_VERSION.to_string(),
            persona_count: 0,
            persona_ids: vec![],
            content_hash: "sha256:error".to_string(),
            errors: vec![EvalPersonaValidationError {
                path: path.to_string_lossy().to_string(),
                message: error.to_string(),
            }],
        },
    }
}

fn parse_persona_markdown(raw: &str, path: &Path, private_terms: &[String]) -> Result<EvalPersona> {
    let (front_matter, narrative_markdown) = split_front_matter(raw)
        .with_context(|| format!("parse front matter for {}", path.display()))?;
    let parsed = parse_front_matter(front_matter)
        .with_context(|| format!("parse persona front matter for {}", path.display()))?;
    for field in REQUIRED_FIELDS {
        ensure!(
            parsed.contains_key(*field),
            "persona {} missing required field {field}",
            path.display()
        );
    }
    let expected_eval_pressure_subsystems =
        list_field(&parsed, "expected_eval_pressure_subsystems")?
            .into_iter()
            .map(|value| parse_pressure_subsystem(&value))
            .collect::<Result<Vec<_>>>()?;

    let content_hash = stable_hash(raw.as_bytes());
    let source_path = normalize_path(path);
    let persona = EvalPersona {
        schema_version: scalar_field(&parsed, "schema_version")?,
        persona_id: scalar_field(&parsed, "persona_id")?,
        display_name: scalar_field(&parsed, "display_name")?,
        person_type: scalar_field(&parsed, "person_type")?,
        event_context: scalar_field(&parsed, "event_context")?,
        business_context: scalar_field(&parsed, "business_context")?,
        personality_traits: list_field(&parsed, "personality_traits")?,
        communication_style: scalar_field(&parsed, "communication_style")?,
        goals: list_field(&parsed, "goals")?,
        objections: list_field(&parsed, "objections")?,
        budget_sensitivity: scalar_field(&parsed, "budget_sensitivity")?,
        urgency_level: scalar_field(&parsed, "urgency_level")?,
        privacy_sensitivity: scalar_field(&parsed, "privacy_sensitivity")?,
        referral_tendency: scalar_field(&parsed, "referral_tendency")?,
        review_likelihood: scalar_field(&parsed, "review_likelihood")?,
        handoff_likelihood: scalar_field(&parsed, "handoff_likelihood")?,
        unsafe_or_edge_case_behaviors: list_field(&parsed, "unsafe_or_edge_case_behaviors")?,
        offer_interest: scalar_field(&parsed, "offer_interest")?,
        trial_success_criteria: list_field(&parsed, "trial_success_criteria")?,
        expected_eval_pressure_subsystems,
        ethical_persuasion_allowed_principles: list_field(
            &parsed,
            "ethical_persuasion_allowed_principles",
        )?,
        redaction_notes: list_field(&parsed, "redaction_notes")?,
        narrative_markdown: narrative_markdown.trim().to_string(),
        source_path,
        content_hash,
    };
    persona.validate(private_terms)?;
    Ok(persona)
}

fn split_front_matter(raw: &str) -> Result<(&str, &str)> {
    let normalized = raw
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow!("persona markdown must begin with YAML front matter delimiter"))?;
    let Some(end_index) = normalized.find("\n---\n") else {
        return Err(anyhow!(
            "persona markdown must close YAML front matter with delimiter"
        ));
    };
    let front_matter = &normalized[..end_index];
    let narrative = &normalized[end_index + "\n---\n".len()..];
    Ok((front_matter, narrative))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FrontMatterValue {
    Scalar(String),
    List(Vec<String>),
}

fn parse_front_matter(front_matter: &str) -> Result<BTreeMap<String, FrontMatterValue>> {
    let mut parsed = BTreeMap::new();
    let mut active_list_key: Option<String> = None;

    for raw_line in front_matter.lines() {
        let line = raw_line.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if let Some(stripped) = line.strip_prefix("  - ") {
            let Some(key) = active_list_key.as_ref() else {
                return Err(anyhow!("front matter list item has no active key"));
            };
            let value = clean_scalar(stripped)?;
            match parsed.get_mut(key) {
                Some(FrontMatterValue::List(items)) => items.push(value),
                _ => return Err(anyhow!("front matter key {key} is not a list")),
            }
            continue;
        }

        let (key, raw_value) = line
            .split_once(':')
            .ok_or_else(|| anyhow!("front matter line missing ':' separator: {line}"))?;
        ensure!(
            !key.trim().is_empty(),
            "front matter field name cannot be empty"
        );
        ensure!(
            !line.starts_with(' '),
            "front matter only supports top-level fields and two-space list items"
        );
        let key = key.trim().to_string();
        ensure!(
            !parsed.contains_key(&key),
            "duplicate front matter field {key}"
        );
        let value = raw_value.trim();
        if value.is_empty() {
            parsed.insert(key.clone(), FrontMatterValue::List(Vec::new()));
            active_list_key = Some(key);
        } else {
            parsed.insert(key, FrontMatterValue::Scalar(clean_scalar(value)?));
            active_list_key = None;
        }
    }
    Ok(parsed)
}

fn scalar_field(parsed: &BTreeMap<String, FrontMatterValue>, key: &str) -> Result<String> {
    match parsed.get(key) {
        Some(FrontMatterValue::Scalar(value)) => {
            require_text(key, value)?;
            Ok(value.clone())
        }
        Some(FrontMatterValue::List(_)) => Err(anyhow!("front matter field {key} must be scalar")),
        None => Err(anyhow!("front matter missing field {key}")),
    }
}

fn list_field(parsed: &BTreeMap<String, FrontMatterValue>, key: &str) -> Result<Vec<String>> {
    match parsed.get(key) {
        Some(FrontMatterValue::List(values)) => {
            require_nonempty_list(key, values)?;
            Ok(values.clone())
        }
        Some(FrontMatterValue::Scalar(_)) => Err(anyhow!("front matter field {key} must be list")),
        None => Err(anyhow!("front matter missing field {key}")),
    }
}

fn parse_pressure_subsystem(value: &str) -> Result<EvalPressureSubsystem> {
    match value {
        "privacy" => Ok(EvalPressureSubsystem::Privacy),
        "policy" => Ok(EvalPressureSubsystem::Policy),
        "handoff" => Ok(EvalPressureSubsystem::Handoff),
        "delegation" => Ok(EvalPressureSubsystem::Delegation),
        "feedback_review" => Ok(EvalPressureSubsystem::FeedbackReview),
        "home_about" => Ok(EvalPressureSubsystem::HomeAbout),
        "offer_ask" => Ok(EvalPressureSubsystem::OfferAsk),
        "accounting_budget" => Ok(EvalPressureSubsystem::AccountingBudget),
        "provider" => Ok(EvalPressureSubsystem::Provider),
        "artifact_review" => Ok(EvalPressureSubsystem::ArtifactReview),
        "simulator_fixture" => Ok(EvalPressureSubsystem::SimulatorFixture),
        _ => Err(anyhow!("unknown eval pressure subsystem {value}")),
    }
}

fn ensure_unique_persona_ids(personas: &[EvalPersona]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for persona in personas {
        ensure!(
            seen.insert(persona.persona_id.clone()),
            "duplicate persona id {}",
            persona.persona_id
        );
    }
    Ok(())
}

fn persona_library_hash(personas: &[EvalPersona]) -> Result<String> {
    let stable = personas
        .iter()
        .map(|persona| {
            serde_json::json!({
                "personaId": persona.persona_id,
                "contentHash": persona.content_hash,
            })
        })
        .collect::<Vec<_>>();
    let encoded = serde_json::to_vec(&stable)?;
    Ok(stable_hash(&encoded))
}

fn require_text(label: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{label} is required");
    Ok(())
}

fn require_nonempty_list(label: &str, values: &[String]) -> Result<()> {
    ensure!(!values.is_empty(), "{label} is required");
    for value in values {
        require_text(label, value)?;
    }
    Ok(())
}

fn require_allowed(label: &str, value: &str, allowed: &[&str]) -> Result<()> {
    ensure!(
        allowed.contains(&value),
        "{label} has unsupported value {value}"
    );
    Ok(())
}

fn clean_scalar(raw_value: &str) -> Result<String> {
    let value = raw_value.trim();
    ensure!(!value.is_empty(), "front matter value cannot be empty");
    if value.starts_with('"') || value.starts_with('\'') {
        ensure!(
            value.len() >= 2 && value.ends_with(value.chars().next().unwrap()),
            "quoted front matter value is not closed"
        );
        Ok(value[1..value.len() - 1].to_string())
    } else {
        Ok(value.to_string())
    }
}

fn normalize_path(path: &Path) -> String {
    let relative = path
        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(path);
    PathBuf::from(relative).to_string_lossy().to_string()
}

fn stable_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn contains_sensitive_value(value: &Value, private_terms: &[String]) -> bool {
    match value {
        Value::String(text) => text_contains_sensitive_value(text, private_terms),
        Value::Array(items) => items
            .iter()
            .any(|item| contains_sensitive_value(item, private_terms)),
        Value::Object(map) => map
            .values()
            .any(|item| contains_sensitive_value(item, private_terms)),
        _ => false,
    }
}

fn text_contains_sensitive_value(text: &str, private_terms: &[String]) -> bool {
    let lower = text.to_ascii_lowercase();
    if private_terms.iter().any(|term| {
        let term = term.trim().to_ascii_lowercase();
        !term.is_empty() && lower.contains(&term)
    }) {
        return true;
    }

    text.split_whitespace().any(|token| {
        let trimmed = token.trim_matches(|character: char| {
            matches!(
                character,
                '"' | '\''
                    | ','
                    | '.'
                    | ';'
                    | ':'
                    | '{'
                    | '}'
                    | '['
                    | ']'
                    | '('
                    | ')'
                    | '<'
                    | '>'
                    | '!'
            )
        });
        looks_like_email(trimmed) || looks_like_phone(trimmed) || looks_like_secret(trimmed)
    })
}

fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
}

fn looks_like_phone(value: &str) -> bool {
    let digit_count = value
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count();
    digit_count >= 10
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || "()+-. ".contains(character))
}

fn looks_like_secret(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("sk-")
        || lower.starts_with("api_")
        || lower.starts_with("pat_")
        || lower.starts_with("ghp_")
        || lower == "bearer"
        || lower.starts_with("bearer_")
        || lower.starts_with("bearer-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn personas_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("docs/evals/personas")
    }

    #[test]
    fn committed_personas_parse_validate_and_order_deterministically() {
        let personas = load_persona_dir(&personas_dir(), &[]).unwrap();
        assert_eq!(personas.len(), MINIMUM_COMMITTED_PERSONA_COUNT);
        assert_eq!(personas[0].persona_id, "affiliate_referrer_community");
        assert_eq!(personas[9].persona_id, "solo_consultant_followup");

        let first_ids = personas
            .iter()
            .map(|persona| persona.persona_id.clone())
            .collect::<Vec<_>>();
        let second_ids = load_persona_dir(&personas_dir(), &[])
            .unwrap()
            .into_iter()
            .map(|persona| persona.persona_id)
            .collect::<Vec<_>>();
        assert_eq!(first_ids, second_ids);

        let validation = validate_persona_library(&personas_dir(), &[]);
        assert!(validation.errors.is_empty(), "{:?}", validation.errors);
        assert_eq!(validation.persona_count, MINIMUM_COMMITTED_PERSONA_COUNT);
        assert!(validation.content_hash.starts_with("sha256:"));
    }

    #[test]
    fn rejects_missing_required_fields() {
        let raw = valid_persona_markdown().replace("persona_id: synthetic_persona\n", "");
        let error = parse_persona_markdown(&raw, &PathBuf::from("synthetic.md"), &[]).unwrap_err();
        assert!(error
            .to_string()
            .contains("missing required field persona_id"));
    }

    #[test]
    fn rejects_unknown_pressure_subsystem() {
        let raw = valid_persona_markdown().replace("  - privacy\n", "  - unknown_gap\n");
        let error = parse_persona_markdown(&raw, &PathBuf::from("synthetic.md"), &[]).unwrap_err();
        assert!(error
            .to_string()
            .contains("unknown eval pressure subsystem unknown_gap"));
    }

    #[test]
    fn rejects_sensitive_fixture_values() {
        for replacement in [
            "Email me at alex@example.com.",
            "Call 212-555-0199.",
            "Use token sk-test-123456.",
            "Project Orchid should stay private.",
        ] {
            let raw = valid_persona_markdown().replace(
                "Synthetic narrative without private contact details.",
                replacement,
            );
            let error = parse_persona_markdown(
                &raw,
                &PathBuf::from("synthetic.md"),
                &["Project Orchid".to_string()],
            )
            .unwrap_err();
            assert!(error.to_string().contains("raw sensitive value"));
        }
    }

    #[test]
    fn rejects_duplicate_persona_ids() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("one.md"), valid_persona_markdown()).unwrap();
        fs::write(temp_dir.path().join("two.md"), valid_persona_markdown()).unwrap();
        let error = load_persona_dir(temp_dir.path(), &[]).unwrap_err();
        assert!(error.to_string().contains("duplicate persona id"));
    }

    #[test]
    fn library_validation_returns_structured_errors() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("bad.md"),
            valid_persona_markdown().replace("schema_version: ordo.live_eval_persona.v1\n", ""),
        )
        .unwrap();
        let validation = validate_persona_library(temp_dir.path(), &[]);
        assert_eq!(validation.persona_count, 0);
        assert_eq!(validation.errors.len(), 1);
        assert!(validation.errors[0]
            .message
            .contains("missing required field schema_version"));
    }

    fn valid_persona_markdown() -> String {
        r#"---
schema_version: ordo.live_eval_persona.v1
persona_id: synthetic_persona
display_name: Synthetic Persona
person_type: solo_consultant
event_context: Met at a synthetic business event after scanning the Studio Ordo QR code.
business_context: Runs a small advisory practice and tracks follow-up manually.
personality_traits:
  - skeptical
  - direct
communication_style: Short mobile messages with practical questions.
goals:
  - Understand whether OrdoStudio can reduce follow-up work.
objections:
  - Does not want another dashboard.
budget_sensitivity: high
urgency_level: medium
privacy_sensitivity: high
referral_tendency: medium
review_likelihood: low_until_value_is_clear
handoff_likelihood: medium
unsafe_or_edge_case_behaviors:
  - May paste private client details that must be redacted.
offer_interest: Interested in a bounded 30-day trial if setup is clear.
trial_success_criteria:
  - First follow-up brief is useful.
expected_eval_pressure_subsystems:
  - privacy
  - offer_ask
ethical_persuasion_allowed_principles:
  - reciprocity
  - commitment_consistency
redaction_notes:
  - Synthetic fixture only.
---

Synthetic narrative without private contact details.
"#
        .to_string()
    }
}
