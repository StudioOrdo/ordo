use anyhow::{ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::events::{append_realtime_event, system_event, RealtimeEvent};
use crate::schema::db::ConnectionExt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactView {
    pub id: String,
    pub artifact_kind: String,
    pub title: String,
    pub status: String,
    pub visibility_ceiling: String,
    pub summary: String,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub content_hash: String,
    pub storage_uri: Option<String>,
    pub health_status: Option<String>,
    pub created_by_job_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactVersionView {
    pub id: String,
    pub artifact_id: String,
    pub version: i64,
    pub content_hash: String,
    pub storage_uri: Option<String>,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactLinkView {
    pub id: String,
    pub artifact_id: String,
    pub link_kind: String,
    pub source_kind: String,
    pub source_id: String,
    pub relation: String,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliverableView {
    pub id: String,
    pub artifact_id: String,
    pub client_label: String,
    pub status: String,
    pub visibility: String,
    pub summary: String,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDetailBrief {
    pub artifact_id: String,
    pub title: String,
    pub value: String,
    pub use_context: String,
    pub next_action: String,
    pub producing_job: Option<String>,
    pub provenance: Value,
    pub evidence_refs: Vec<String>,
    pub storage_health: String,
}

#[derive(Debug, Clone)]
pub struct ArtifactInput {
    pub artifact_kind: String,
    pub title: String,
    pub status: String,
    pub visibility_ceiling: String,
    pub summary: String,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub content_hash: String,
    pub storage_uri: Option<String>,
    pub health_status: Option<String>,
    pub created_by_job_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ArtifactLinkInput {
    pub link_kind: String,
    pub source_kind: String,
    pub source_id: String,
    pub relation: String,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
}

#[derive(Debug, Clone)]
pub struct DeliverableInput {
    pub client_label: String,
    pub status: String,
    pub visibility: String,
    pub summary: String,
}

pub fn record_artifact(
    connection: &Connection,
    input: ArtifactInput,
) -> Result<(ArtifactView, RealtimeEvent)> {
    validate_artifact_input(&input)?;
    let now = Utc::now().to_rfc3339();
    let id = format!("artifact_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO artifacts (
            id, artifact_kind, title, status, visibility_ceiling, summary,
            source_kind, source_id, evidence_refs_json, provenance_json, content_hash,
            storage_uri, health_status, created_by_job_id, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
        params![
            id,
            input.artifact_kind,
            input.title,
            input.status,
            input.visibility_ceiling,
            input.summary,
            input.source_kind,
            input.source_id,
            json!(input.evidence_refs).to_string(),
            input.provenance.to_string(),
            input.content_hash,
            input.storage_uri,
            input.health_status,
            input.created_by_job_id,
            now,
        ],
    )?;
    let artifact = load_artifact(connection, &id)?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "artifact.recorded",
            json!({
                "artifactId": artifact.id,
                "artifactKind": artifact.artifact_kind,
                "title": artifact.title,
                "evidenceRefs": artifact.evidence_refs,
            }),
        ),
    )?;
    Ok((artifact, event))
}

pub fn add_artifact_version(
    connection: &Connection,
    artifact_id: &str,
    content_hash: &str,
    storage_uri: Option<&str>,
    metadata: Value,
) -> Result<ArtifactVersionView> {
    ensure!(
        artifact_exists(connection, artifact_id)?,
        "artifact version requires a known artifact"
    );
    ensure!(!content_hash.trim().is_empty(), "content hash is required");
    let now = Utc::now().to_rfc3339();
    let version = next_artifact_version(connection, artifact_id)?;
    let id = format!("artifact_version_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO artifact_versions (
            id, artifact_id, version, content_hash, storage_uri, metadata_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            artifact_id,
            version,
            content_hash,
            storage_uri,
            metadata.to_string(),
            now,
        ],
    )?;
    load_artifact_version(connection, &id)
}

pub fn link_artifact(
    connection: &Connection,
    artifact_id: &str,
    input: ArtifactLinkInput,
) -> Result<(ArtifactLinkView, RealtimeEvent)> {
    ensure!(
        artifact_exists(connection, artifact_id)?,
        "artifact link requires a known artifact"
    );
    validate_link_input(&input)?;
    let now = Utc::now().to_rfc3339();
    let id = format!("artifact_link_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO artifact_links (
            id, artifact_id, link_kind, source_kind, source_id, relation,
            evidence_refs_json, provenance_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(artifact_id, link_kind, source_kind, source_id, relation)
         DO UPDATE SET evidence_refs_json = excluded.evidence_refs_json,
                       provenance_json = excluded.provenance_json,
                       created_at = excluded.created_at",
        params![
            id,
            artifact_id,
            input.link_kind,
            input.source_kind,
            input.source_id,
            input.relation,
            json!(input.evidence_refs).to_string(),
            input.provenance.to_string(),
            now,
        ],
    )?;
    let link = load_artifact_link_by_identity(
        connection,
        artifact_id,
        &input.link_kind,
        &input.source_kind,
        &input.source_id,
        &input.relation,
    )?;
    let event = append_realtime_event(
        connection,
        &system_event(
            "artifact.linked",
            json!({
                "artifactId": link.artifact_id,
                "linkKind": link.link_kind,
                "sourceKind": link.source_kind,
                "sourceId": link.source_id,
                "relation": link.relation,
                "evidenceRefs": link.evidence_refs,
            }),
        ),
    )?;
    Ok((link, event))
}

pub fn publish_deliverable(
    connection: &Connection,
    artifact_id: &str,
    input: DeliverableInput,
) -> Result<(DeliverableView, RealtimeEvent)> {
    record_deliverable(
        connection,
        artifact_id,
        input,
        "deliverable.published",
        |status, now| {
            if status == "published" {
                Some(now.to_string())
            } else {
                None
            }
        },
    )
}

pub fn stage_deliverable(
    connection: &Connection,
    artifact_id: &str,
    input: DeliverableInput,
) -> Result<(DeliverableView, RealtimeEvent)> {
    record_deliverable(
        connection,
        artifact_id,
        input,
        "deliverable.staged",
        |_status, _now| None,
    )
}

fn record_deliverable<F>(
    connection: &Connection,
    artifact_id: &str,
    input: DeliverableInput,
    event_type: &str,
    published_at_for: F,
) -> Result<(DeliverableView, RealtimeEvent)>
where
    F: FnOnce(&str, &str) -> Option<String>,
{
    ensure!(
        artifact_exists(connection, artifact_id)?,
        "deliverable requires a known artifact"
    );
    ensure!(
        !input.client_label.trim().is_empty(),
        "client label is required"
    );
    ensure!(!input.summary.trim().is_empty(), "summary is required");
    let now = Utc::now().to_rfc3339();
    let published_at = published_at_for(&input.status, &now);
    let id = format!("deliverable_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO artifact_deliverables (
            id, artifact_id, client_label, status, visibility, summary,
            created_at, updated_at, published_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8)",
        params![
            id,
            artifact_id,
            input.client_label,
            input.status,
            input.visibility,
            input.summary,
            now,
            published_at,
        ],
    )?;
    let deliverable = load_deliverable(connection, &id)?;
    let event = append_realtime_event(
        connection,
        &system_event(
            event_type,
            json!({
                "deliverableId": deliverable.id,
                "artifactId": deliverable.artifact_id,
                "clientLabel": deliverable.client_label,
                "status": deliverable.status,
            }),
        ),
    )?;
    Ok((deliverable, event))
}

pub fn artifact_detail_brief(
    connection: &Connection,
    artifact_id: &str,
) -> Result<ArtifactDetailBrief> {
    let artifact = load_artifact(connection, artifact_id)?;
    let link_count = count_artifact_links(connection, artifact_id)?;
    let use_context = match (&artifact.source_kind, &artifact.source_id) {
        (Some(kind), Some(id)) => {
            format!("Used from {kind} {id} with {link_count} linked evidence source(s).")
        }
        _ => {
            format!("Available as a durable artifact with {link_count} linked evidence source(s).")
        }
    };
    let storage_health = match (&artifact.storage_uri, &artifact.health_status) {
        (Some(uri), Some(health)) => format!("{health} at {uri}"),
        (Some(uri), None) => format!("stored at {uri}"),
        (None, Some(health)) => health.clone(),
        (None, None) => "storage not attached".to_string(),
    };
    Ok(ArtifactDetailBrief {
        artifact_id: artifact.id,
        title: artifact.title,
        value: artifact.summary,
        use_context,
        next_action: next_action_for_status(&artifact.status),
        producing_job: artifact.created_by_job_id,
        provenance: artifact.provenance,
        evidence_refs: artifact.evidence_refs,
        storage_health,
    })
}

pub fn list_deliverables_for_artifact(
    connection: &Connection,
    artifact_id: &str,
) -> Result<Vec<DeliverableView>> {
    connection.query_many(
        "SELECT id, artifact_id, client_label, status, visibility, summary, created_at,
                updated_at, published_at
         FROM artifact_deliverables
         WHERE artifact_id = ?1
         ORDER BY updated_at DESC",
        [artifact_id],
        deliverable_from_row,
    )
}

pub fn list_artifact_links(
    connection: &Connection,
    artifact_id: &str,
) -> Result<Vec<ArtifactLinkView>> {
    connection.query_many(
        "SELECT id, artifact_id, link_kind, source_kind, source_id, relation,
                evidence_refs_json, provenance_json, created_at
         FROM artifact_links
         WHERE artifact_id = ?1
         ORDER BY created_at DESC",
        [artifact_id],
        artifact_link_from_row,
    )
}

fn validate_artifact_input(input: &ArtifactInput) -> Result<()> {
    ensure!(
        !input.artifact_kind.trim().is_empty(),
        "artifact kind is required"
    );
    ensure!(!input.title.trim().is_empty(), "artifact title is required");
    ensure!(
        !input.status.trim().is_empty(),
        "artifact status is required"
    );
    ensure!(
        !input.visibility_ceiling.trim().is_empty(),
        "artifact visibility ceiling is required"
    );
    ensure!(
        !input.summary.trim().is_empty(),
        "artifact summary is required"
    );
    ensure!(
        !input.evidence_refs.is_empty(),
        "artifact evidence refs are required"
    );
    ensure!(
        input.provenance.is_object(),
        "artifact provenance is required"
    );
    ensure!(
        !input.content_hash.trim().is_empty(),
        "artifact content hash is required"
    );
    Ok(())
}

fn validate_link_input(input: &ArtifactLinkInput) -> Result<()> {
    ensure!(!input.link_kind.trim().is_empty(), "link kind is required");
    ensure!(
        !input.source_kind.trim().is_empty(),
        "source kind is required"
    );
    ensure!(!input.source_id.trim().is_empty(), "source id is required");
    ensure!(!input.relation.trim().is_empty(), "relation is required");
    ensure!(
        !input.evidence_refs.is_empty(),
        "link evidence refs are required"
    );
    ensure!(input.provenance.is_object(), "link provenance is required");
    Ok(())
}

fn artifact_exists(connection: &Connection, artifact_id: &str) -> Result<bool> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM artifacts WHERE id = ?1",
        [artifact_id],
        |row| row.get(0),
    )?;
    Ok(count == 1)
}

fn next_artifact_version(connection: &Connection, artifact_id: &str) -> Result<i64> {
    let current: Option<i64> = connection.query_row(
        "SELECT MAX(version) FROM artifact_versions WHERE artifact_id = ?1",
        [artifact_id],
        |row| row.get(0),
    )?;
    Ok(current.unwrap_or(0) + 1)
}

fn count_artifact_links(connection: &Connection, artifact_id: &str) -> Result<i64> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM artifact_links WHERE artifact_id = ?1",
            [artifact_id],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn next_action_for_status(status: &str) -> String {
    match status {
        "draft" => "review evidence before publishing".to_string(),
        "ready" => "decide whether this should become a client deliverable".to_string(),
        "published" => "monitor use and outcome influence".to_string(),
        "archived" => "retain for provenance only".to_string(),
        _ => "inspect evidence and decide the next business action".to_string(),
    }
}

fn load_artifact(connection: &Connection, artifact_id: &str) -> Result<ArtifactView> {
    connection
        .query_row(
            "SELECT id, artifact_kind, title, status, visibility_ceiling, summary,
                    source_kind, source_id, evidence_refs_json, provenance_json, content_hash,
                    storage_uri, health_status, created_by_job_id, created_at, updated_at
             FROM artifacts WHERE id = ?1",
            [artifact_id],
            artifact_from_row,
        )
        .map_err(Into::into)
}

fn load_artifact_version(connection: &Connection, version_id: &str) -> Result<ArtifactVersionView> {
    connection
        .query_row(
            "SELECT id, artifact_id, version, content_hash, storage_uri, metadata_json, created_at
             FROM artifact_versions WHERE id = ?1",
            [version_id],
            artifact_version_from_row,
        )
        .map_err(Into::into)
}

fn load_artifact_link_by_identity(
    connection: &Connection,
    artifact_id: &str,
    link_kind: &str,
    source_kind: &str,
    source_id: &str,
    relation: &str,
) -> Result<ArtifactLinkView> {
    connection
        .query_row(
            "SELECT id, artifact_id, link_kind, source_kind, source_id, relation,
                    evidence_refs_json, provenance_json, created_at
             FROM artifact_links
             WHERE artifact_id = ?1 AND link_kind = ?2 AND source_kind = ?3
               AND source_id = ?4 AND relation = ?5",
            params![artifact_id, link_kind, source_kind, source_id, relation],
            artifact_link_from_row,
        )
        .map_err(Into::into)
}

fn load_deliverable(connection: &Connection, deliverable_id: &str) -> Result<DeliverableView> {
    connection
        .query_row(
            "SELECT id, artifact_id, client_label, status, visibility, summary, created_at,
                    updated_at, published_at
             FROM artifact_deliverables WHERE id = ?1",
            [deliverable_id],
            deliverable_from_row,
        )
        .map_err(Into::into)
}

fn artifact_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArtifactView> {
    let evidence_json: String = row.get(8)?;
    let provenance_json: String = row.get(9)?;
    Ok(ArtifactView {
        id: row.get(0)?,
        artifact_kind: row.get(1)?,
        title: row.get(2)?,
        status: row.get(3)?,
        visibility_ceiling: row.get(4)?,
        summary: row.get(5)?,
        source_kind: row.get(6)?,
        source_id: row.get(7)?,
        evidence_refs: serde_json::from_str(&evidence_json).unwrap_or_default(),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        content_hash: row.get(10)?,
        storage_uri: row.get(11)?,
        health_status: row.get(12)?,
        created_by_job_id: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn artifact_version_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArtifactVersionView> {
    let metadata_json: String = row.get(5)?;
    Ok(ArtifactVersionView {
        id: row.get(0)?,
        artifact_id: row.get(1)?,
        version: row.get(2)?,
        content_hash: row.get(3)?,
        storage_uri: row.get(4)?,
        metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(6)?,
    })
}

fn artifact_link_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArtifactLinkView> {
    let evidence_json: String = row.get(6)?;
    let provenance_json: String = row.get(7)?;
    Ok(ArtifactLinkView {
        id: row.get(0)?,
        artifact_id: row.get(1)?,
        link_kind: row.get(2)?,
        source_kind: row.get(3)?,
        source_id: row.get(4)?,
        relation: row.get(5)?,
        evidence_refs: serde_json::from_str(&evidence_json).unwrap_or_default(),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        created_at: row.get(8)?,
    })
}

fn deliverable_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeliverableView> {
    Ok(DeliverableView {
        id: row.get(0)?,
        artifact_id: row.get(1)?,
        client_label: row.get(2)?,
        status: row.get(3)?,
        visibility: row.get(4)?,
        summary: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        published_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;

    #[test]
    fn artifact_records_require_evidence_and_provenance() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        let missing_evidence = record_artifact(
            &connection,
            ArtifactInput {
                artifact_kind: "offer.material".to_string(),
                title: "Starter QR Card".to_string(),
                status: "ready".to_string(),
                visibility_ceiling: "staff".to_string(),
                summary: "QR card proof for the Starter offer.".to_string(),
                source_kind: Some("offer".to_string()),
                source_id: Some("offer_starter".to_string()),
                evidence_refs: vec![],
                provenance: json!({"generatedBy": "artifact.test"}),
                content_hash: "sha256:artifact".to_string(),
                storage_uri: Some("ordo://artifact/starter-qr-card".to_string()),
                health_status: Some("available".to_string()),
                created_by_job_id: None,
            },
        );
        assert!(missing_evidence.is_err());

        let (artifact, event) = record_artifact(&connection, starter_artifact()).unwrap();
        assert_eq!(artifact.artifact_kind, "offer.material");
        assert_eq!(artifact.visibility_ceiling, "staff");
        assert_eq!(event.event_type, "artifact.recorded");
        assert_eq!(event.payload["artifactId"], artifact.id);
    }

    #[test]
    fn deliverable_projects_from_artifact_without_internal_mechanics() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let (artifact, _) = record_artifact(&connection, starter_artifact()).unwrap();

        let (deliverable, event) = publish_deliverable(
            &connection,
            &artifact.id,
            DeliverableInput {
                client_label: "QR card proof".to_string(),
                status: "published".to_string(),
                visibility: "client".to_string(),
                summary: "Client-safe card proof ready for review.".to_string(),
            },
        )
        .unwrap();

        assert_eq!(deliverable.artifact_id, artifact.id);
        assert_eq!(deliverable.client_label, "QR card proof");
        assert_eq!(deliverable.visibility, "client");
        assert!(deliverable.published_at.is_some());
        assert_eq!(event.event_type, "deliverable.published");
        assert!(event.payload.get("provenance").is_none());
        assert!(event.payload.get("storageUri").is_none());
    }

    #[test]
    fn artifact_links_require_concrete_source_ids_and_do_not_invent_attribution() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let (artifact, _) = record_artifact(&connection, starter_artifact()).unwrap();

        let bad_link = link_artifact(
            &connection,
            &artifact.id,
            ArtifactLinkInput {
                link_kind: "influence".to_string(),
                source_kind: "business_outcome".to_string(),
                source_id: "".to_string(),
                relation: "assisted".to_string(),
                evidence_refs: vec!["message_1".to_string()],
                provenance: json!({"generatedBy": "artifact.link.test"}),
            },
        );
        assert!(bad_link.is_err());

        let (link, event) = link_artifact(
            &connection,
            &artifact.id,
            ArtifactLinkInput {
                link_kind: "influence".to_string(),
                source_kind: "business_outcome".to_string(),
                source_id: "outcome_1".to_string(),
                relation: "assisted".to_string(),
                evidence_refs: vec!["message_1".to_string()],
                provenance: json!({"generatedBy": "artifact.link.test"}),
            },
        )
        .unwrap();

        assert_eq!(link.source_kind, "business_outcome");
        assert_eq!(link.source_id, "outcome_1");
        assert_eq!(event.event_type, "artifact.linked");
        assert_eq!(
            list_artifact_links(&connection, &artifact.id)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn artifact_detail_brief_answers_value_use_next_action_and_health() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let (artifact, _) = record_artifact(&connection, starter_artifact()).unwrap();
        add_artifact_version(
            &connection,
            &artifact.id,
            "sha256:artifact-v2",
            Some("ordo://artifact/starter-qr-card/v2"),
            json!({"reviewState": "ready"}),
        )
        .unwrap();

        let brief = artifact_detail_brief(&connection, &artifact.id).unwrap();
        assert_eq!(brief.title, "Starter QR Card");
        assert!(brief.value.contains("QR card proof"));
        assert!(brief.use_context.contains("offer"));
        assert_eq!(
            brief.next_action,
            "decide whether this should become a client deliverable"
        );
        assert!(brief.storage_health.contains("available"));
        assert_eq!(brief.evidence_refs, vec!["offer_view_starter_3"]);
    }

    fn starter_artifact() -> ArtifactInput {
        ArtifactInput {
            artifact_kind: "offer.material".to_string(),
            title: "Starter QR Card".to_string(),
            status: "ready".to_string(),
            visibility_ceiling: "staff".to_string(),
            summary: "QR card proof for the Starter offer.".to_string(),
            source_kind: Some("offer".to_string()),
            source_id: Some("offer_starter".to_string()),
            evidence_refs: vec!["offer_view_starter_3".to_string()],
            provenance: json!({"generatedBy": "artifact.test", "source": "offer"}),
            content_hash: "sha256:artifact".to_string(),
            storage_uri: Some("ordo://artifact/starter-qr-card".to_string()),
            health_status: Some("available".to_string()),
            created_by_job_id: None,
        }
    }
}
