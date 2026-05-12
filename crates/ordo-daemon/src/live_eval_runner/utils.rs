use super::*;
use anyhow::Result;
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) fn skipped(reason: impl Into<String>) -> LiveEvalGuardDecision {
    LiveEvalGuardDecision {
        status: LiveEvalStatus::Skipped,
        reason: reason.into(),
        network_enabled: false,
    }
}

pub(crate) fn blocked(reason: impl Into<String>) -> LiveEvalGuardDecision {
    LiveEvalGuardDecision {
        status: LiveEvalStatus::Blocked,
        reason: reason.into(),
        network_enabled: false,
    }
}

pub(crate) fn env_is_one(values: &BTreeMap<String, String>, key: &str) -> bool {
    values
        .get(key)
        .map(|value| value.trim() == "1")
        .unwrap_or(false)
}

pub(crate) fn env_trimmed(values: &BTreeMap<String, String>, key: &str) -> Option<String> {
    values
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn parse_optional_u32(values: &BTreeMap<String, String>, key: &str) -> Result<Option<u32>, String> {
    let Some(raw) = env_trimmed(values, key) else {
        return Ok(None);
    };
    raw.parse::<u32>()
        .map(Some)
        .map_err(|_| format!("{key} must be a positive integer"))
}

pub(crate) fn parse_optional_u64(values: &BTreeMap<String, String>, key: &str) -> Result<Option<u64>, String> {
    let Some(raw) = env_trimmed(values, key) else {
        return Ok(None);
    };
    raw.parse::<u64>()
        .map(Some)
        .map_err(|_| format!("{key} must be a positive integer"))
}

pub(crate) fn parse_optional_usd_micros(
    values: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<u64>, String> {
    let Some(raw) = env_trimmed(values, key) else {
        return Ok(None);
    };
    parse_usd_micros(&raw)
        .map(Some)
        .ok_or_else(|| format!("{key} must be a non-negative decimal USD amount"))
}

pub(crate) fn parse_usd_micros(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('-') {
        return None;
    }
    let (dollars, fraction) = trimmed.split_once('.').unwrap_or((trimmed, ""));
    let dollar_micros = dollars.parse::<u64>().ok()?.checked_mul(1_000_000)?;
    let mut fraction_digits = fraction
        .chars()
        .take(6)
        .filter(|character| character.is_ascii_digit())
        .collect::<String>();
    if fraction_digits.len() != fraction.chars().take(6).count() {
        return None;
    }
    while fraction_digits.len() < 6 {
        fraction_digits.push('0');
    }
    let fraction_micros = if fraction_digits.is_empty() {
        0
    } else {
        fraction_digits.parse::<u64>().ok()?
    };
    dollar_micros.checked_add(fraction_micros)
}

pub(crate) fn contains_sensitive_value(value: &Value, private_terms: &[String]) -> bool {
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

pub(crate) fn text_contains_sensitive_value(text: &str, private_terms: &[String]) -> bool {
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

pub(crate) fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
}

pub(crate) fn looks_like_phone(value: &str) -> bool {
    let digit_count = value
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count();
    digit_count >= 10
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || "()+-. ".contains(character))
}

pub(crate) fn looks_like_secret(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("sk-")
        || lower.starts_with("api_")
        || lower.starts_with("pat_")
        || lower.starts_with("ghp_")
        || lower == "bearer"
        || lower.starts_with("bearer_")
        || lower.starts_with("bearer-")
}

