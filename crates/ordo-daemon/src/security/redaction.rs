use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RedactedText {
    pub text: String,
    pub redacted_count: usize,
    pub detectors: BTreeSet<&'static str>,
}

pub(crate) fn redact_eval_text(text: &str, private_terms: &[String]) -> RedactedText {
    redact_tokenized(text, private_terms, RedactionStyle::Eval)
}

pub(crate) fn redact_artifact_review_text(text: &str) -> RedactedText {
    let private_terms = vec![
        "Project Orchid".to_string(),
        "Project".to_string(),
        "Orchid".to_string(),
    ];
    redact_tokenized(text, &private_terms, RedactionStyle::Eval)
}

pub(crate) fn redact_public_text(text: &str) -> String {
    redact_tokenized(text, &[], RedactionStyle::Public).text
}

pub(crate) fn sanitize_json_strings(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(redact_public_text(&text)),
        Value::Array(values) => {
            Value::Array(values.into_iter().map(sanitize_json_strings).collect())
        }
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (key, sanitize_json_strings(value)))
                .collect(),
        ),
        other => other,
    }
}

pub(crate) fn contains_sensitive_text(content: &str, private_terms: &[String]) -> bool {
    let lower = content.to_ascii_lowercase();
    private_terms
        .iter()
        .filter(|term| !term.trim().is_empty())
        .any(|term| lower.contains(&term.to_ascii_lowercase()))
        || lower.contains("project orchid")
        || lower.contains("bearer ")
        || lower.contains("sk-live")
        || content.split_whitespace().any(|token| {
            let trimmed = trimmed_token(token);
            looks_like_email(trimmed) || looks_like_phone(trimmed) || looks_like_secret(trimmed)
        })
}

pub(crate) fn redact_support_packet_markdown(markdown: &str) -> String {
    markdown
        .lines()
        .map(|line| {
            if contains_secret_indicator(line) {
                "[redacted support packet line]".to_string()
            } else {
                redact_public_text(line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn contains_secret_indicator(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "api_key",
        "apikey",
        "token",
        "password",
        "secret",
        "vault key",
        "vault://",
        "bearer ",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(crate) fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && local.len() <= 128
        && domain.contains('.')
        && !domain.ends_with('.')
        && domain
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-".contains(character))
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
    (lower.starts_with("sk-") && value.len() >= 8)
        || (lower.starts_with("api_") && value.len() >= 12)
        || (lower.starts_with("pat_") && value.len() >= 12)
        || (lower.starts_with("ghp_") && value.len() >= 12)
        || (lower.starts_with("gho_") && value.len() >= 12)
        || (lower.starts_with("key_") && value.len() >= 12)
        || (lower.starts_with("tok_") && value.len() >= 12)
        || lower == "bearer"
        || (lower.starts_with("bearer_") && value.len() >= 14)
        || (lower.starts_with("bearer-") && value.len() >= 14)
        || lower.starts_with("vault://")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RedactionStyle {
    Eval,
    Public,
}

fn redact_tokenized(text: &str, private_terms: &[String], style: RedactionStyle) -> RedactedText {
    let mut output = Vec::new();
    let mut redacted_count = 0;
    let mut detectors = BTreeSet::new();
    let mut skip_next_bearer = false;
    for token in text.split_whitespace() {
        let trimmed = trimmed_token(token);
        if skip_next_bearer {
            redacted_count += 1;
            detectors.insert("bearer_token");
            let replacement = match style {
                RedactionStyle::Eval => "[REDACTED:secret]",
                RedactionStyle::Public => "[REDACTED_TOKEN]",
            };
            output.push(token.replace(trimmed, replacement));
            skip_next_bearer = false;
            continue;
        }
        if trimmed.eq_ignore_ascii_case("bearer") {
            let replacement = match style {
                RedactionStyle::Eval => "[REDACTED:secret]",
                RedactionStyle::Public => "Bearer",
            };
            output.push(token.replace(trimmed, replacement));
            if style == RedactionStyle::Eval {
                redacted_count += 1;
                detectors.insert("bearer_token");
            }
            skip_next_bearer = true;
            continue;
        }

        let replacement = if looks_like_email(trimmed) {
            detectors.insert("email");
            replacement_for(style, "email")
        } else if looks_like_phone(trimmed) {
            detectors.insert("phone");
            replacement_for(style, "phone")
        } else if looks_like_secret(trimmed) {
            detectors.insert("secret");
            replacement_for(style, "secret")
        } else if is_private_term(trimmed, private_terms) {
            detectors.insert("private_term");
            replacement_for(style, "private_term")
        } else {
            None
        };

        if let Some(replacement) = replacement {
            redacted_count += 1;
            output.push(token.replace(trimmed, replacement));
        } else {
            output.push(token.to_string());
        }
    }
    RedactedText {
        text: output.join(" "),
        redacted_count,
        detectors,
    }
}

fn replacement_for(style: RedactionStyle, detector: &str) -> Option<&'static str> {
    Some(match (style, detector) {
        (RedactionStyle::Eval, "email") => "[REDACTED:email]",
        (RedactionStyle::Eval, "phone") => "[REDACTED:phone]",
        (RedactionStyle::Eval, "secret") => "[REDACTED:secret]",
        (RedactionStyle::Eval, "private_term") => "[REDACTED:private_term]",
        (RedactionStyle::Public, "email") => "[REDACTED_EMAIL]",
        (RedactionStyle::Public, "phone") => "[REDACTED_PHONE]",
        (RedactionStyle::Public, "secret") => "[REDACTED_SECRET]",
        (RedactionStyle::Public, "private_term") => "[REDACTED_PRIVATE]",
        _ => return None,
    })
}

fn is_private_term(token: &str, private_terms: &[String]) -> bool {
    private_terms
        .iter()
        .filter(|term| !term.trim().is_empty())
        .any(|term| token.eq_ignore_ascii_case(term.trim()))
}

fn trimmed_token(token: &str) -> &str {
    token.trim_matches(|character: char| {
        matches!(
            character,
            '"' | '\'' | ',' | '.' | ';' | ':' | '{' | '}' | '[' | ']' | '(' | ')' | '<' | '>'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_redaction_covers_expected_secret_shapes() {
        let private_terms = vec!["Project".to_string(), "Orchid".to_string()];
        let result = redact_eval_text(
            "Email alex@example.com or call 555-123-4567 with Bearer sk-live-secret and vault://team/key for Project Orchid.",
            &private_terms,
        );

        assert!(!result.text.contains("alex@example.com"));
        assert!(!result.text.contains("555-123-4567"));
        assert!(!result.text.contains("sk-live-secret"));
        assert!(!result.text.contains("vault://team/key"));
        assert!(!result.text.contains("Project"));
        assert!(!result.text.contains("Orchid"));
        assert!(result.text.contains("[REDACTED:email]"));
        assert!(result.text.contains("[REDACTED:phone]"));
        assert!(result.text.contains("[REDACTED:secret]"));
        assert!(result.text.contains("[REDACTED:private_term]"));
        assert!(result.redacted_count >= 6);
    }

    #[test]
    fn public_redaction_preserves_false_positives() {
        let redacted = redact_public_text(
            "Tokenization is not a token. The api_key detector label is safe. Reach support@example.com with sk-test-value.",
        );

        assert!(redacted.contains("Tokenization"));
        assert!(redacted.contains("not a token"));
        assert!(redacted.contains("api_key detector label"));
        assert!(!redacted.contains("support@example.com"));
        assert!(!redacted.contains("sk-test-value"));
    }

    #[test]
    fn support_packet_redacts_secret_indicator_lines() {
        let redacted = redact_support_packet_markdown(
            "safe line\napi_key: sk-test-value\ncontact alex@example.com",
        );

        assert!(redacted.contains("safe line"));
        assert!(redacted.contains("[redacted support packet line]"));
        assert!(redacted.contains("[REDACTED_EMAIL]"));
        assert!(!redacted.contains("alex@example.com"));
        assert!(!redacted.contains("sk-test-value"));
    }

    #[test]
    fn sensitive_text_detector_catches_private_terms_and_keys() {
        let terms = vec!["Project Orchid".to_string()];

        assert!(contains_sensitive_text("Project Orchid launched", &terms));
        assert!(contains_sensitive_text("Bearer sk-live-secret", &[]));
        assert!(contains_sensitive_text("alex@example.com", &[]));
        assert!(!contains_sensitive_text("ordinary public report", &terms));
    }
}
