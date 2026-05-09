use anyhow::{ensure, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use uuid::Uuid;

use crate::vault::{decrypt_secret, store_secret};

pub const PRIVACY_EGRESS_DETECTOR_VERSION: &str = "privacy-egress.detectors.v1";
pub const PRIVACY_EGRESS_TRANSFORM_VERSION: &str = "privacy-egress.transform.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyEgressScope {
    pub scope_kind: String,
    pub scope_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyEgressFinding {
    pub detector_kind: String,
    pub placeholder: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyEgressTransform {
    pub transform_run_id: String,
    pub scope: PrivacyEgressScope,
    pub source_payload_hash: String,
    pub transformed_payload: String,
    pub findings: Vec<PrivacyEgressFinding>,
    pub detector_version: String,
    pub transform_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyEgressReconstruction {
    pub transform_run_id: String,
    pub scope: PrivacyEgressScope,
    pub reconstructed_payload: String,
    pub placeholder_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PrivacyEgressFirewall {
    private_terms: Vec<String>,
}

impl PrivacyEgressFirewall {
    pub fn new(private_terms: Vec<String>) -> Self {
        Self {
            private_terms: private_terms
                .into_iter()
                .filter(|term| !term.trim().is_empty())
                .collect(),
        }
    }

    pub fn transform_payload(
        &self,
        db_path: &Path,
        connection: &Connection,
        scope: PrivacyEgressScope,
        payload: &str,
    ) -> Result<PrivacyEgressTransform> {
        ensure!(
            !contains_ordo_placeholder(payload),
            "provider-bound payload contains an existing privacy placeholder"
        );
        let transform_run_id = format!("privacy_transform_{}", Uuid::new_v4());
        let source_payload_hash = stable_hash(payload);
        let mut transformed_payload = payload.to_string();
        let mut findings = detect_sensitive_values(payload, &self.private_terms);
        findings.sort_by(|left, right| {
            right
                .raw_value
                .len()
                .cmp(&left.raw_value.len())
                .then(left.raw_value.cmp(&right.raw_value))
        });
        findings.dedup_by(|left, right| left.raw_value == right.raw_value);

        let mut public_findings = Vec::new();
        for (index, finding) in findings.iter().enumerate() {
            let placeholder = format!(
                "__ORDO_PRIVATE_{}_{}__",
                sanitize_placeholder_kind(&finding.detector_kind),
                index + 1
            );
            transformed_payload = transformed_payload.replace(&finding.raw_value, &placeholder);
            let content_hash = stable_hash(&finding.raw_value);
            store_secret(
                db_path,
                connection,
                "privacy_placeholder",
                &placeholder,
                &finding.raw_value,
                None,
                json!({
                    "transformRunId": transform_run_id,
                    "placeholder": placeholder,
                    "detectorKind": finding.detector_kind,
                    "scopeKind": scope.scope_kind,
                    "scopeId": scope.scope_id,
                    "contentHash": content_hash,
                }),
            )?;
            public_findings.push(PrivacyEgressFinding {
                detector_kind: finding.detector_kind.clone(),
                placeholder,
                content_hash,
            });
        }

        Ok(PrivacyEgressTransform {
            transform_run_id,
            scope,
            source_payload_hash,
            transformed_payload,
            findings: public_findings,
            detector_version: PRIVACY_EGRESS_DETECTOR_VERSION.to_string(),
            transform_version: PRIVACY_EGRESS_TRANSFORM_VERSION.to_string(),
        })
    }

    pub fn reconstruct_payload(
        db_path: &Path,
        connection: &Connection,
        transform_run_id: &str,
        scope: PrivacyEgressScope,
        payload: &str,
    ) -> Result<PrivacyEgressReconstruction> {
        ensure!(
            !transform_run_id.trim().is_empty(),
            "transform_run_id is required"
        );
        ensure!(
            !scope.scope_kind.trim().is_empty(),
            "scope_kind is required"
        );
        ensure!(!scope.scope_id.trim().is_empty(), "scope_id is required");
        let mut statement = connection.prepare(
            "SELECT id, metadata_json FROM vault_items
             WHERE kind = 'privacy_placeholder'",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut replacements = BTreeMap::new();
        for row in rows {
            let (vault_item_id, metadata_json) = row?;
            let metadata: Value =
                serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({}));
            if metadata["transformRunId"].as_str() == Some(transform_run_id)
                && metadata["scopeKind"].as_str() == Some(scope.scope_kind.as_str())
                && metadata["scopeId"].as_str() == Some(scope.scope_id.as_str())
            {
                let placeholder = metadata["placeholder"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("privacy placeholder metadata is missing"))?;
                replacements.insert(
                    placeholder.to_string(),
                    decrypt_secret(db_path, connection, &vault_item_id)?,
                );
            }
        }

        let mut reconstructed_payload = payload.to_string();
        let mut placeholder_count = 0;
        for (placeholder, plaintext) in replacements {
            if reconstructed_payload.contains(&placeholder) {
                reconstructed_payload = reconstructed_payload.replace(&placeholder, &plaintext);
                placeholder_count += 1;
            }
        }

        ensure!(
            !contains_ordo_placeholder(&reconstructed_payload),
            "payload contains unknown or wrong-scope privacy placeholders"
        );
        Ok(PrivacyEgressReconstruction {
            transform_run_id: transform_run_id.to_string(),
            scope,
            reconstructed_payload,
            placeholder_count,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SensitiveFinding {
    detector_kind: String,
    raw_value: String,
}

fn detect_sensitive_values(payload: &str, private_terms: &[String]) -> Vec<SensitiveFinding> {
    let mut findings = Vec::new();
    for token in payload.split_whitespace() {
        let candidate = trim_token(token);
        if candidate.is_empty() {
            continue;
        }
        if looks_like_api_key(candidate) {
            findings.push(finding("api_key", candidate));
        } else if looks_like_email(candidate) {
            findings.push(finding("email", candidate));
        } else if looks_like_phone(candidate) {
            findings.push(finding("phone", candidate));
        }
    }
    for bearer in bearer_tokens(payload) {
        findings.push(finding("bearer_token", &bearer));
    }
    for term in private_terms {
        if payload.contains(term) {
            findings.push(finding("private_term", term));
        }
    }
    findings
}

fn bearer_tokens(payload: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let parts = payload.split_whitespace().collect::<Vec<_>>();
    for window in parts.windows(2) {
        if window[0].eq_ignore_ascii_case("bearer") {
            tokens.push(format!("{} {}", window[0], trim_token(window[1])));
        }
    }
    tokens
}

fn finding(detector_kind: &str, raw_value: &str) -> SensitiveFinding {
    SensitiveFinding {
        detector_kind: detector_kind.to_string(),
        raw_value: raw_value.to_string(),
    }
}

fn trim_token(token: &str) -> &str {
    token.trim_matches(|character: char| {
        matches!(
            character,
            ',' | ';' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}'
        )
    })
}

fn looks_like_api_key(candidate: &str) -> bool {
    let lower = candidate.to_ascii_lowercase();
    (lower.starts_with("sk-") || lower.starts_with("sk_") || lower.starts_with("ordo_"))
        && candidate.len() >= 12
}

fn looks_like_email(candidate: &str) -> bool {
    let mut parts = candidate.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();
    parts.next().is_none()
        && !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

fn looks_like_phone(candidate: &str) -> bool {
    let digits = candidate
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count();
    digits >= 10
        && candidate
            .chars()
            .all(|character| character.is_ascii_digit() || "()+-. ".contains(character))
}

fn contains_ordo_placeholder(payload: &str) -> bool {
    payload.contains("__ORDO_PRIVATE_")
}

fn sanitize_placeholder_kind(detector_kind: &str) -> String {
    detector_kind
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

pub fn stable_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;
    use rusqlite::Connection;

    fn test_connection() -> (tempfile::TempDir, std::path::PathBuf, Connection) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        let connection = Connection::open(&db_path).unwrap();
        init_schema(&connection).unwrap();
        (temp_dir, db_path, connection)
    }

    #[test]
    fn transforms_sensitive_values_and_reconstructs_in_scope_only() {
        let (_temp_dir, db_path, connection) = test_connection();
        let firewall = PrivacyEgressFirewall::new(vec!["Project Orchid".to_string()]);
        let scope = PrivacyEgressScope {
            scope_kind: "llm_run".to_string(),
            scope_id: "run_1".to_string(),
        };

        let transform = firewall
            .transform_payload(
                &db_path,
                &connection,
                scope.clone(),
                "Email ada@example.com, call +1-212-555-0101, key sk-test-123456, Bearer tok_abcdef123456, Project Orchid.",
            )
            .unwrap();

        assert!(!transform.transformed_payload.contains("ada@example.com"));
        assert!(!transform.transformed_payload.contains("+1-212-555-0101"));
        assert!(!transform.transformed_payload.contains("sk-test-123456"));
        assert!(!transform.transformed_payload.contains("tok_abcdef123456"));
        assert!(!transform.transformed_payload.contains("Project Orchid"));
        assert_eq!(transform.findings.len(), 5);

        let reconstructed = PrivacyEgressFirewall::reconstruct_payload(
            &db_path,
            &connection,
            &transform.transform_run_id,
            scope.clone(),
            &transform.transformed_payload,
        )
        .unwrap();
        assert!(reconstructed
            .reconstructed_payload
            .contains("ada@example.com"));
        assert_eq!(reconstructed.placeholder_count, 5);

        let wrong_scope = PrivacyEgressFirewall::reconstruct_payload(
            &db_path,
            &connection,
            &transform.transform_run_id,
            PrivacyEgressScope {
                scope_kind: "llm_run".to_string(),
                scope_id: "run_2".to_string(),
            },
            &transform.transformed_payload,
        );
        assert!(wrong_scope.is_err());
    }

    #[test]
    fn metadata_does_not_store_raw_sensitive_values() {
        let (_temp_dir, db_path, connection) = test_connection();
        let firewall = PrivacyEgressFirewall::default();
        let transform = firewall
            .transform_payload(
                &db_path,
                &connection,
                PrivacyEgressScope {
                    scope_kind: "llm_run".to_string(),
                    scope_id: "run_1".to_string(),
                },
                "Use sk-test-secret-value for ada@example.com",
            )
            .unwrap();

        let leaked_metadata: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM vault_items
                 WHERE metadata_json LIKE '%sk-test-secret-value%'
                    OR metadata_json LIKE '%ada@example.com%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(leaked_metadata, 0);
        assert!(transform
            .findings
            .iter()
            .all(|finding| finding.content_hash.starts_with("sha256:")));
    }
}
