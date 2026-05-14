//! Governed patch proposals for artifact text.
//!
//! This first slice intentionally supports text artifacts only. It records
//! unified diffs, hashes, provenance, review state, and artifact version
//! evidence; it does not write files or apply multi-file project patches.

use anyhow::{ensure, Context, Result};
use chrono::Utc;
use diffy::{apply, create_patch, Patch};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::artifacts::{add_artifact_version, ArtifactVersionView};
use crate::events::{append_realtime_event, system_event, RealtimeEvent};

const REVIEW_STATE_PROPOSED: &str = "proposed";
const REVIEW_STATE_NO_OP: &str = "no_op";
const REVIEW_STATE_ACCEPTED: &str = "accepted";
const PATCH_REVIEW_PREVIEW_LIMIT: usize = 800;

#[derive(Debug, Clone)]
pub struct CreateArtifactPatchProposalInput {
    pub source_artifact_id: String,
    pub source_version_id: String,
    pub base_text: String,
    pub proposed_text: String,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub proposed_by_actor_id: String,
}

#[derive(Debug, Clone)]
pub struct ApplyArtifactPatchProposalInput {
    pub proposal_id: String,
    pub current_text: String,
    pub applied_by_actor_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchPreview {
    pub changed: bool,
    pub added_lines: usize,
    pub removed_lines: usize,
    pub hunks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactPatchProposalView {
    pub id: String,
    pub source_artifact_id: String,
    pub source_version_id: String,
    pub base_hash: String,
    pub proposed_hash: String,
    pub patch_text: String,
    pub preview: PatchPreview,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub review_state: String,
    pub accepted_version_id: Option<String>,
    pub proposed_by_actor_id: String,
    pub applied_by_actor_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub applied_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppliedArtifactPatch {
    pub proposal: ArtifactPatchProposalView,
    pub artifact_version: ArtifactVersionView,
    pub event: RealtimeEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactPatchReviewProposal {
    pub id: String,
    pub source_artifact_id: String,
    pub source_artifact_kind: String,
    pub source_artifact_title: String,
    pub source_artifact_status: String,
    pub source_artifact_visibility: String,
    pub source_version_id: String,
    pub base_hash: String,
    pub proposed_hash: String,
    pub preview: PatchPreview,
    pub bounded_patch_preview: String,
    pub preview_truncated: bool,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub review_state: String,
    pub accepted_version_id: Option<String>,
    pub proposed_by_actor_id: String,
    pub applied_by_actor_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub applied_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactPatchReviewListResponse {
    pub proposals: Vec<ArtifactPatchReviewProposal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactPatchReviewResponse {
    pub proposal: ArtifactPatchReviewProposal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactPatchApplyResponse {
    pub proposal: ArtifactPatchReviewProposal,
    pub artifact_version: ArtifactVersionView,
}

#[derive(Debug, Clone)]
struct SourceArtifact {
    id: String,
    artifact_kind: String,
    content_hash: String,
    storage_uri: Option<String>,
}

#[derive(Debug, Clone)]
struct SourceVersion {
    id: String,
    artifact_id: String,
    content_hash: String,
}

pub fn stable_text_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

pub fn create_artifact_patch_proposal(
    connection: &Connection,
    input: CreateArtifactPatchProposalInput,
) -> Result<(ArtifactPatchProposalView, RealtimeEvent)> {
    ensure_actor(&input.proposed_by_actor_id)?;
    ensure!(
        !input.evidence_refs.is_empty(),
        "artifact patch proposal requires evidence refs"
    );
    ensure!(
        input
            .provenance
            .as_object()
            .is_some_and(|object| !object.is_empty()),
        "artifact patch proposal requires provenance"
    );

    let artifact = load_source_artifact(connection, &input.source_artifact_id)?;
    ensure_text_artifact(&artifact)?;
    ensure_safe_storage_uri(artifact.storage_uri.as_deref())?;

    let source_version = load_source_version(connection, &input.source_version_id)?;
    ensure!(
        source_version.artifact_id == artifact.id,
        "source version does not belong to source artifact"
    );

    let latest =
        load_latest_source_version(connection, &artifact.id)?.unwrap_or(source_version.clone());
    ensure!(
        latest.id == source_version.id,
        "source version is stale; create a proposal from the latest artifact version"
    );

    let base_hash = stable_text_hash(&input.base_text);
    let proposed_hash = stable_text_hash(&input.proposed_text);
    ensure!(
        source_version.content_hash == base_hash,
        "base hash does not match source artifact version"
    );
    ensure!(
        current_artifact_hash(connection, &artifact)? == base_hash,
        "current artifact base hash differs from proposal base hash"
    );

    let patch = create_patch(&input.base_text, &input.proposed_text);
    let patch_text = patch.to_string();
    let preview = preview_from_patch(&patch, base_hash != proposed_hash);
    let review_state = if preview.changed {
        REVIEW_STATE_PROPOSED
    } else {
        REVIEW_STATE_NO_OP
    };

    let now = Utc::now().to_rfc3339();
    let id = format!("artifact_patch_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO artifact_patch_proposals (
            id, source_artifact_id, source_version_id, base_hash, proposed_hash,
            patch_text, preview_json, evidence_refs_json, provenance_json, review_state,
            accepted_version_id, proposed_by_actor_id, applied_by_actor_id,
            created_at, updated_at, applied_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, ?11, NULL, ?12, ?12, NULL)",
        params![
            id,
            artifact.id,
            source_version.id,
            base_hash,
            proposed_hash,
            patch_text,
            json!(preview).to_string(),
            json!(input.evidence_refs).to_string(),
            input.provenance.to_string(),
            review_state,
            input.proposed_by_actor_id,
            now,
        ],
    )?;

    let proposal = load_artifact_patch_proposal(connection, &id)?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "artifact.patch.proposed",
            json!({
                "artifactPatchProposalId": proposal.id,
                "artifactId": proposal.source_artifact_id,
                "sourceVersionId": proposal.source_version_id,
                "baseHash": proposal.base_hash,
                "proposedHash": proposal.proposed_hash,
                "reviewState": proposal.review_state,
                "preview": proposal.preview,
                "evidenceRefs": proposal.evidence_refs,
            }),
        ),
    )?;
    Ok((proposal, event))
}

pub fn apply_artifact_patch_proposal(
    connection: &Connection,
    input: ApplyArtifactPatchProposalInput,
) -> Result<AppliedArtifactPatch> {
    ensure_actor(&input.applied_by_actor_id)?;

    let proposal = load_artifact_patch_proposal(connection, &input.proposal_id)?;
    ensure!(
        proposal.review_state == REVIEW_STATE_PROPOSED,
        "{} patch proposals cannot be applied",
        proposal.review_state.replace('_', "-")
    );

    let artifact = load_source_artifact(connection, &proposal.source_artifact_id)?;
    ensure_text_artifact(&artifact)?;
    ensure_safe_storage_uri(artifact.storage_uri.as_deref())?;

    let current_hash = current_artifact_hash(connection, &artifact)?;
    ensure!(
        current_hash == proposal.base_hash,
        "current artifact base hash differs from proposal base hash"
    );
    ensure!(
        stable_text_hash(&input.current_text) == proposal.base_hash,
        "current text hash does not match proposal base hash"
    );

    let patched_text = validate_unified_patch(&input.current_text, &proposal.patch_text)?;
    ensure!(
        stable_text_hash(&patched_text) == proposal.proposed_hash,
        "patch result hash does not match proposed hash"
    );

    let artifact_version = add_artifact_version(
        connection,
        &artifact.id,
        &proposal.proposed_hash,
        None,
        json!({
            "source": "artifact_patch_proposal",
            "artifactPatchProposalId": proposal.id,
            "sourceVersionId": proposal.source_version_id,
            "baseHash": proposal.base_hash,
            "proposedHash": proposal.proposed_hash,
        }),
    )?;
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE artifact_patch_proposals
         SET review_state = ?1,
             accepted_version_id = ?2,
             applied_by_actor_id = ?3,
             updated_at = ?4,
             applied_at = ?4
         WHERE id = ?5",
        params![
            REVIEW_STATE_ACCEPTED,
            artifact_version.id,
            input.applied_by_actor_id,
            now,
            proposal.id,
        ],
    )?;

    let proposal = load_artifact_patch_proposal(connection, &input.proposal_id)?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "artifact.patch.accepted",
            json!({
                "artifactPatchProposalId": proposal.id,
                "artifactId": proposal.source_artifact_id,
                "sourceVersionId": proposal.source_version_id,
                "acceptedVersionId": artifact_version.id,
                "baseHash": proposal.base_hash,
                "proposedHash": proposal.proposed_hash,
                "reviewState": proposal.review_state,
                "evidenceRefs": proposal.evidence_refs,
            }),
        ),
    )?;
    Ok(AppliedArtifactPatch {
        proposal,
        artifact_version,
        event,
    })
}

pub fn validate_unified_patch(base_text: &str, patch_text: &str) -> Result<String> {
    let patch = Patch::from_str(patch_text).context("malformed patch text")?;
    ensure!(!patch.hunks().is_empty(), "malformed patch text");
    apply(base_text, &patch).context("patch does not apply cleanly")
}

pub fn load_artifact_patch_proposal(
    connection: &Connection,
    proposal_id: &str,
) -> Result<ArtifactPatchProposalView> {
    connection
        .query_row(
            "SELECT id, source_artifact_id, source_version_id, base_hash, proposed_hash,
                    patch_text, preview_json, evidence_refs_json, provenance_json, review_state,
                    accepted_version_id, proposed_by_actor_id, applied_by_actor_id,
                    created_at, updated_at, applied_at
             FROM artifact_patch_proposals WHERE id = ?1",
            [proposal_id],
            proposal_from_row,
        )
        .map_err(Into::into)
}

pub fn list_artifact_patch_review_proposals(
    connection: &Connection,
    review_state: Option<&str>,
    limit: usize,
) -> Result<ArtifactPatchReviewListResponse> {
    let limit = limit.clamp(1, 100);
    let mut statement = connection.prepare(
        "SELECT p.id, p.source_artifact_id, a.artifact_kind, a.title, a.status,
                a.visibility_ceiling, p.source_version_id, p.base_hash, p.proposed_hash,
                p.patch_text, p.preview_json, p.evidence_refs_json, p.provenance_json,
                p.review_state, p.accepted_version_id, p.proposed_by_actor_id,
                p.applied_by_actor_id, p.created_at, p.updated_at, p.applied_at
         FROM artifact_patch_proposals p
         JOIN artifacts a ON a.id = p.source_artifact_id
         WHERE (?1 IS NULL OR p.review_state = ?1)
         ORDER BY p.updated_at DESC, p.id ASC
         LIMIT ?2",
    )?;
    let proposals = statement
        .query_map(
            params![review_state, limit as i64],
            review_proposal_from_row,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ArtifactPatchReviewListResponse { proposals })
}

pub fn load_artifact_patch_review_proposal(
    connection: &Connection,
    proposal_id: &str,
) -> Result<ArtifactPatchReviewResponse> {
    let proposal = connection.query_row(
        "SELECT p.id, p.source_artifact_id, a.artifact_kind, a.title, a.status,
                a.visibility_ceiling, p.source_version_id, p.base_hash, p.proposed_hash,
                p.patch_text, p.preview_json, p.evidence_refs_json, p.provenance_json,
                p.review_state, p.accepted_version_id, p.proposed_by_actor_id,
                p.applied_by_actor_id, p.created_at, p.updated_at, p.applied_at
         FROM artifact_patch_proposals p
         JOIN artifacts a ON a.id = p.source_artifact_id
         WHERE p.id = ?1",
        [proposal_id],
        review_proposal_from_row,
    )?;
    Ok(ArtifactPatchReviewResponse { proposal })
}

pub fn apply_artifact_patch_review_proposal(
    connection: &Connection,
    input: ApplyArtifactPatchProposalInput,
) -> Result<ArtifactPatchApplyResponse> {
    let applied = apply_artifact_patch_proposal(connection, input)?;
    let proposal = load_artifact_patch_review_proposal(connection, &applied.proposal.id)?.proposal;
    Ok(ArtifactPatchApplyResponse {
        proposal,
        artifact_version: applied.artifact_version,
    })
}

fn preview_from_patch(patch: &Patch<'_, str>, changed: bool) -> PatchPreview {
    let patch_text = patch.to_string();
    let added_lines = patch_text
        .lines()
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .count();
    let removed_lines = patch_text
        .lines()
        .filter(|line| line.starts_with('-') && !line.starts_with("---"))
        .count();
    PatchPreview {
        changed,
        added_lines,
        removed_lines,
        hunks: patch.hunks().len(),
    }
}

fn ensure_actor(actor_id: &str) -> Result<()> {
    ensure!(
        !actor_id.trim().is_empty(),
        "artifact patch operation requires an authorized actor"
    );
    ensure!(
        actor_id.starts_with("owner:") || actor_id.starts_with("staff:") || actor_id == "system",
        "artifact patch operation requires an authorized actor"
    );
    Ok(())
}

fn ensure_text_artifact(artifact: &SourceArtifact) -> Result<()> {
    let kind = artifact.artifact_kind.as_str();
    let is_text = matches!(
        kind,
        "text"
            | "markdown"
            | "report"
            | "brief"
            | "document"
            | "prompt"
            | "script"
            | "caption"
            | "qa_report"
            | "promo_script"
            | "strategy_brief"
    ) || kind.ends_with("_text")
        || kind.ends_with("_markdown")
        || kind.ends_with("_brief")
        || kind.ends_with("_report")
        || kind.ends_with("_script");
    ensure!(
        is_text,
        "artifact patch proposals support text artifacts only"
    );
    Ok(())
}

fn ensure_safe_storage_uri(storage_uri: Option<&str>) -> Result<()> {
    let Some(storage_uri) = storage_uri else {
        return Ok(());
    };
    ensure!(
        !storage_uri.trim().is_empty(),
        "artifact storage URI cannot be blank"
    );
    ensure!(
        !storage_uri.starts_with("file:")
            && !storage_uri.starts_with('/')
            && !storage_uri.starts_with('~')
            && !storage_uri.contains(".."),
        "artifact patch proposal refuses unsafe storage/path input"
    );
    Ok(())
}

fn current_artifact_hash(connection: &Connection, artifact: &SourceArtifact) -> Result<String> {
    Ok(load_latest_source_version(connection, &artifact.id)?
        .map(|version| version.content_hash)
        .unwrap_or_else(|| artifact.content_hash.clone()))
}

fn load_source_artifact(connection: &Connection, artifact_id: &str) -> Result<SourceArtifact> {
    connection
        .query_row(
            "SELECT id, artifact_kind, content_hash, storage_uri
             FROM artifacts WHERE id = ?1",
            [artifact_id],
            |row| {
                Ok(SourceArtifact {
                    id: row.get(0)?,
                    artifact_kind: row.get(1)?,
                    content_hash: row.get(2)?,
                    storage_uri: row.get(3)?,
                })
            },
        )
        .map_err(Into::into)
}

fn load_source_version(connection: &Connection, version_id: &str) -> Result<SourceVersion> {
    connection
        .query_row(
            "SELECT id, artifact_id, content_hash
             FROM artifact_versions WHERE id = ?1",
            [version_id],
            source_version_from_row,
        )
        .map_err(Into::into)
}

fn load_latest_source_version(
    connection: &Connection,
    artifact_id: &str,
) -> Result<Option<SourceVersion>> {
    connection
        .query_row(
            "SELECT id, artifact_id, content_hash
             FROM artifact_versions
             WHERE artifact_id = ?1
             ORDER BY version DESC
             LIMIT 1",
            [artifact_id],
            source_version_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn source_version_from_row(row: &Row<'_>) -> rusqlite::Result<SourceVersion> {
    Ok(SourceVersion {
        id: row.get(0)?,
        artifact_id: row.get(1)?,
        content_hash: row.get(2)?,
    })
}

fn proposal_from_row(row: &Row<'_>) -> rusqlite::Result<ArtifactPatchProposalView> {
    let preview_json: String = row.get(6)?;
    let evidence_refs_json: String = row.get(7)?;
    let provenance_json: String = row.get(8)?;
    Ok(ArtifactPatchProposalView {
        id: row.get(0)?,
        source_artifact_id: row.get(1)?,
        source_version_id: row.get(2)?,
        base_hash: row.get(3)?,
        proposed_hash: row.get(4)?,
        patch_text: row.get(5)?,
        preview: serde_json::from_str(&preview_json).unwrap_or(PatchPreview {
            changed: false,
            added_lines: 0,
            removed_lines: 0,
            hunks: 0,
        }),
        evidence_refs: serde_json::from_str(&evidence_refs_json).unwrap_or_default(),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        review_state: row.get(9)?,
        accepted_version_id: row.get(10)?,
        proposed_by_actor_id: row.get(11)?,
        applied_by_actor_id: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        applied_at: row.get(15)?,
    })
}

fn review_proposal_from_row(row: &Row<'_>) -> rusqlite::Result<ArtifactPatchReviewProposal> {
    let patch_text: String = row.get(9)?;
    let preview_json: String = row.get(10)?;
    let evidence_refs_json: String = row.get(11)?;
    let provenance_json: String = row.get(12)?;
    let (bounded_patch_preview, preview_truncated) = bounded_patch_preview(&patch_text);
    Ok(ArtifactPatchReviewProposal {
        id: row.get(0)?,
        source_artifact_id: row.get(1)?,
        source_artifact_kind: row.get(2)?,
        source_artifact_title: row.get(3)?,
        source_artifact_status: row.get(4)?,
        source_artifact_visibility: row.get(5)?,
        source_version_id: row.get(6)?,
        base_hash: row.get(7)?,
        proposed_hash: row.get(8)?,
        preview: serde_json::from_str(&preview_json).unwrap_or(PatchPreview {
            changed: false,
            added_lines: 0,
            removed_lines: 0,
            hunks: 0,
        }),
        bounded_patch_preview,
        preview_truncated,
        evidence_refs: serde_json::from_str(&evidence_refs_json).unwrap_or_default(),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        review_state: row.get(13)?,
        accepted_version_id: row.get(14)?,
        proposed_by_actor_id: row.get(15)?,
        applied_by_actor_id: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
        applied_at: row.get(19)?,
    })
}

fn bounded_patch_preview(patch_text: &str) -> (String, bool) {
    let truncated = patch_text.chars().count() > PATCH_REVIEW_PREVIEW_LIMIT;
    let preview = patch_text
        .chars()
        .take(PATCH_REVIEW_PREVIEW_LIMIT)
        .collect::<String>();
    (preview, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::{add_artifact_version, record_artifact, ArtifactInput};
    use crate::schema::init_schema;
    use rusqlite::Connection;
    use serde_json::json;

    fn setup_text_artifact(
        connection: &Connection,
        artifact_kind: &str,
        text: &str,
    ) -> (String, String) {
        let base_hash = stable_text_hash(text);
        let (artifact, _) = record_artifact(
            connection,
            ArtifactInput {
                artifact_kind: artifact_kind.to_string(),
                title: "Landing page copy".to_string(),
                status: "draft".to_string(),
                visibility_ceiling: "owner".to_string(),
                summary: "Patchable text artifact".to_string(),
                source_kind: Some("job".to_string()),
                source_id: Some("job_patch_source".to_string()),
                evidence_refs: vec!["job:job_patch_source".to_string()],
                provenance: json!({"source": "artifact_patch_test"}),
                content_hash: base_hash.clone(),
                storage_uri: None,
                health_status: None,
                created_by_job_id: None,
            },
        )
        .unwrap();
        let version = add_artifact_version(
            connection,
            &artifact.id,
            &base_hash,
            None,
            json!({"source": "artifact_patch_test"}),
        )
        .unwrap();
        (artifact.id, version.id)
    }

    fn proposal_input(
        artifact_id: &str,
        version_id: &str,
        base: &str,
        proposed: &str,
    ) -> CreateArtifactPatchProposalInput {
        CreateArtifactPatchProposalInput {
            source_artifact_id: artifact_id.to_string(),
            source_version_id: version_id.to_string(),
            base_text: base.to_string(),
            proposed_text: proposed.to_string(),
            evidence_refs: vec!["job:job_patch_source".to_string()],
            provenance: json!({"source": "artifact_patch_test"}),
            proposed_by_actor_id: "owner:ordo".to_string(),
        }
    }

    #[test]
    fn creates_and_applies_text_artifact_patch_without_overwriting_history() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let base = "# Offer\n\nBook a visit.\n";
        let proposed = "# Offer\n\nBook a strategy visit.\n";
        let (artifact_id, version_id) = setup_text_artifact(&connection, "markdown", base);

        let (proposal, proposed_event) = create_artifact_patch_proposal(
            &connection,
            proposal_input(&artifact_id, &version_id, base, proposed),
        )
        .unwrap();

        assert_eq!(proposal.source_artifact_id, artifact_id);
        assert_eq!(proposal.source_version_id, version_id);
        assert_eq!(proposal.base_hash, stable_text_hash(base));
        assert_eq!(proposal.proposed_hash, stable_text_hash(proposed));
        assert_eq!(proposal.review_state, "proposed");
        assert!(proposal.patch_text.contains("strategy visit"));
        assert!(proposal.preview.changed);
        assert_eq!(proposed_event.event_type, "artifact.patch.proposed");

        let applied = apply_artifact_patch_proposal(
            &connection,
            ApplyArtifactPatchProposalInput {
                proposal_id: proposal.id.clone(),
                current_text: base.to_string(),
                applied_by_actor_id: "owner:ordo".to_string(),
            },
        )
        .unwrap();

        assert_eq!(applied.proposal.review_state, "accepted");
        assert_eq!(
            applied.proposal.accepted_version_id.as_deref(),
            Some(applied.artifact_version.id.as_str())
        );
        assert_eq!(applied.artifact_version.version, 2);
        assert_eq!(
            applied.artifact_version.content_hash,
            stable_text_hash(proposed)
        );
        assert_eq!(applied.event.event_type, "artifact.patch.accepted");
    }

    #[test]
    fn rejects_stale_base_hash_without_mutation() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let base = "one\n";
        let proposed = "two\n";
        let (artifact_id, version_id) = setup_text_artifact(&connection, "text", base);
        let (proposal, _) = create_artifact_patch_proposal(
            &connection,
            proposal_input(&artifact_id, &version_id, base, proposed),
        )
        .unwrap();

        add_artifact_version(
            &connection,
            &artifact_id,
            &stable_text_hash("three\n"),
            None,
            json!({"source": "concurrent_update"}),
        )
        .unwrap();

        let error = apply_artifact_patch_proposal(
            &connection,
            ApplyArtifactPatchProposalInput {
                proposal_id: proposal.id,
                current_text: base.to_string(),
                applied_by_actor_id: "owner:ordo".to_string(),
            },
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("base hash"));
    }

    #[test]
    fn rejects_malformed_patch_text() {
        let error = validate_unified_patch("alpha\n", "not a unified patch")
            .unwrap_err()
            .to_string();

        assert!(error.contains("malformed patch"));
    }

    #[test]
    fn rejects_malformed_stored_patch_without_creating_version() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let base = "alpha\n";
        let (artifact_id, version_id) = setup_text_artifact(&connection, "markdown", base);
        let (proposal, _) = create_artifact_patch_proposal(
            &connection,
            proposal_input(&artifact_id, &version_id, base, "beta\n"),
        )
        .unwrap();
        connection
            .execute(
                "UPDATE artifact_patch_proposals SET patch_text = ?1 WHERE id = ?2",
                rusqlite::params!["not a unified patch", proposal.id],
            )
            .unwrap();

        let error = apply_artifact_patch_proposal(
            &connection,
            ApplyArtifactPatchProposalInput {
                proposal_id: proposal.id,
                current_text: base.to_string(),
                applied_by_actor_id: "owner:ordo".to_string(),
            },
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("malformed patch"));
        let versions: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifact_versions WHERE artifact_id = ?1",
                [artifact_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(versions, 1);
    }

    #[test]
    fn records_no_op_patch_as_review_only_and_refuses_apply() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let base = "same\n";
        let (artifact_id, version_id) = setup_text_artifact(&connection, "markdown", base);

        let (proposal, _) = create_artifact_patch_proposal(
            &connection,
            proposal_input(&artifact_id, &version_id, base, base),
        )
        .unwrap();

        assert_eq!(proposal.review_state, "no_op");
        assert!(!proposal.preview.changed);

        let error = apply_artifact_patch_proposal(
            &connection,
            ApplyArtifactPatchProposalInput {
                proposal_id: proposal.id,
                current_text: base.to_string(),
                applied_by_actor_id: "owner:ordo".to_string(),
            },
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("no-op"));
    }

    #[test]
    fn rejects_unsupported_artifact_kind_and_missing_apply_actor() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let base = "binary placeholder";
        let (artifact_id, version_id) = setup_text_artifact(&connection, "video", base);

        let error = create_artifact_patch_proposal(
            &connection,
            proposal_input(&artifact_id, &version_id, base, "changed"),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("text artifacts only"));

        let (artifact_id, version_id) = setup_text_artifact(&connection, "markdown", base);
        let (proposal, _) = create_artifact_patch_proposal(
            &connection,
            proposal_input(&artifact_id, &version_id, base, "changed"),
        )
        .unwrap();

        let error = apply_artifact_patch_proposal(
            &connection,
            ApplyArtifactPatchProposalInput {
                proposal_id: proposal.id,
                current_text: base.to_string(),
                applied_by_actor_id: " ".to_string(),
            },
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("authorized actor"));
    }

    #[test]
    fn rejects_unsafe_storage_uri_and_keeps_large_preview_deterministic() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let base = "line\n";
        let (artifact_id, version_id) = setup_text_artifact(&connection, "markdown", base);
        connection
            .execute(
                "UPDATE artifacts SET storage_uri = ?1 WHERE id = ?2",
                rusqlite::params!["../outside.md", artifact_id],
            )
            .unwrap();

        let error = create_artifact_patch_proposal(
            &connection,
            proposal_input(&artifact_id, &version_id, base, "line changed\n"),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("unsafe storage/path"));

        let large_base = (0..300)
            .map(|index| format!("line {index}\n"))
            .collect::<String>();
        let large_proposed = large_base.replace("line 299", "line 299 updated");
        let (artifact_id, version_id) = setup_text_artifact(&connection, "markdown", &large_base);

        let (first, _) = create_artifact_patch_proposal(
            &connection,
            proposal_input(&artifact_id, &version_id, &large_base, &large_proposed),
        )
        .unwrap();
        let (second, _) = create_artifact_patch_proposal(
            &connection,
            proposal_input(&artifact_id, &version_id, &large_base, &large_proposed),
        )
        .unwrap();

        assert_eq!(first.base_hash, second.base_hash);
        assert_eq!(first.proposed_hash, second.proposed_hash);
        assert_eq!(first.preview, second.preview);
        assert_eq!(first.patch_text, second.patch_text);
    }

    #[test]
    fn review_list_shapes_bounded_metadata_without_actor_contexts() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let large_base = (0..400)
            .map(|index| format!("line {index}\n"))
            .collect::<String>();
        let large_proposed = (0..400)
            .map(|index| format!("line {index} updated\n"))
            .collect::<String>();
        let (artifact_id, version_id) = setup_text_artifact(&connection, "markdown", &large_base);

        let (proposal, _) = create_artifact_patch_proposal(
            &connection,
            proposal_input(&artifact_id, &version_id, &large_base, &large_proposed),
        )
        .unwrap();
        let review = list_artifact_patch_review_proposals(&connection, Some("proposed"), 10)
            .unwrap()
            .proposals
            .remove(0);
        let serialized = serde_json::to_string(&review).unwrap();

        assert_eq!(review.id, proposal.id);
        assert_eq!(review.source_artifact_title, "Landing page copy");
        assert_eq!(review.source_artifact_visibility, "owner");
        assert!(review.preview.changed);
        assert!(review.preview_truncated);
        assert!(review.bounded_patch_preview.chars().count() <= PATCH_REVIEW_PREVIEW_LIMIT);
        assert!(!serialized.contains("rawPrompt"));
        assert!(!serialized.contains("provider"));
        assert!(!serialized.contains("policy"));
        assert!(!serialized.contains("sk_live"));
    }

    #[test]
    fn review_apply_uses_governed_apply_path_and_returns_safe_proposal() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let base = "alpha\n";
        let proposed = "beta\n";
        let (artifact_id, version_id) = setup_text_artifact(&connection, "markdown", base);
        let (proposal, _) = create_artifact_patch_proposal(
            &connection,
            proposal_input(&artifact_id, &version_id, base, proposed),
        )
        .unwrap();

        let response = apply_artifact_patch_review_proposal(
            &connection,
            ApplyArtifactPatchProposalInput {
                proposal_id: proposal.id,
                current_text: base.to_string(),
                applied_by_actor_id: "owner:local_owner".to_string(),
            },
        )
        .unwrap();

        assert_eq!(response.proposal.review_state, "accepted");
        assert_eq!(
            response.proposal.accepted_version_id.as_deref(),
            Some(response.artifact_version.id.as_str())
        );
        assert_eq!(response.artifact_version.version, 2);
        assert_eq!(
            response.artifact_version.content_hash,
            stable_text_hash(proposed)
        );
    }
}
