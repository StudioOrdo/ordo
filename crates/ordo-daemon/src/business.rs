use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

use crate::events::{append_realtime_event_tx, system_event, RealtimeEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusinessFactVisibility {
    Public,
    Authenticated,
    Staff,
    Owner,
}

impl BusinessFactVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Authenticated => "authenticated",
            Self::Staff => "staff",
            Self::Owner => "owner",
        }
    }
}

impl TryFrom<&str> for BusinessFactVisibility {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "public" => Ok(Self::Public),
            "authenticated" => Ok(Self::Authenticated),
            "staff" => Ok(Self::Staff),
            "owner" => Ok(Self::Owner),
            _ => bail!("Unsupported business fact visibility: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationState {
    Draft,
    Published,
    Archived,
    Revoked,
}

impl PublicationState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Archived => "archived",
            Self::Revoked => "revoked",
        }
    }
}

impl TryFrom<&str> for PublicationState {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "draft" => Ok(Self::Draft),
            "published" => Ok(Self::Published),
            "archived" => Ok(Self::Archived),
            "revoked" => Ok(Self::Revoked),
            _ => bail!("Unsupported publication state: {value}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusinessFactViewer {
    Public,
    Authenticated,
    Staff,
    Owner,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessFactQuery {
    pub viewer: Option<BusinessFactViewer>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessFactListResponse {
    pub facts: Vec<BusinessFactView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessFactView {
    pub id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub fact_key: String,
    pub value: Value,
    pub source_kind: String,
    pub source_label: Option<String>,
    pub source_uri: Option<String>,
    pub provenance: Value,
    pub visibility: BusinessFactVisibility,
    pub publication_state: PublicationState,
    pub created_by_actor_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessFactWriteRequest {
    pub subject_type: Option<String>,
    pub subject_id: Option<String>,
    pub fact_key: String,
    pub value: Value,
    pub source_kind: Option<String>,
    pub source_label: Option<String>,
    pub source_uri: Option<String>,
    pub provenance: Option<Value>,
    pub visibility: Option<BusinessFactVisibility>,
    pub publication_state: Option<PublicationState>,
}

#[derive(Debug, Clone)]
struct BusinessFactRecord {
    id: String,
    subject_type: String,
    subject_id: String,
    fact_key: String,
    value: Value,
    source_kind: String,
    source_label: Option<String>,
    source_uri: Option<String>,
    provenance: Value,
    visibility: BusinessFactVisibility,
    publication_state: PublicationState,
    created_by_actor_id: Option<String>,
    created_at: String,
    updated_at: String,
    published_at: Option<String>,
    archived_at: Option<String>,
}

pub fn list_business_facts(
    db_path: &Path,
    query: BusinessFactQuery,
) -> Result<BusinessFactListResponse> {
    let connection = Connection::open(db_path)?;
    list_business_facts_connection(
        &connection,
        query.viewer.unwrap_or(BusinessFactViewer::Owner),
    )
}

pub fn create_business_fact(
    db_path: &Path,
    request: BusinessFactWriteRequest,
    actor_id: Option<&str>,
) -> Result<(BusinessFactView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let id = format!("business_fact_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();
    let fact_key = require_identifier(&request.fact_key, "Fact key")?;
    let visibility = request.visibility.unwrap_or(BusinessFactVisibility::Owner);
    let publication_state = request.publication_state.unwrap_or(PublicationState::Draft);
    let published_at = (publication_state == PublicationState::Published).then(|| now.clone());
    let archived_at = matches!(
        publication_state,
        PublicationState::Archived | PublicationState::Revoked
    )
    .then(|| now.clone());
    transaction.execute(
        "INSERT INTO business_facts (
            id, subject_type, subject_id, fact_key, value_json, source_kind, source_label,
            source_uri, provenance_json, visibility, publication_state, created_by_actor_id,
            created_at, updated_at, published_at, archived_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13, ?14, ?15)",
        params![
            id,
            normalize_optional_string(request.subject_type)
                .unwrap_or_else(|| "business".to_string()),
            normalize_optional_string(request.subject_id)
                .unwrap_or_else(|| "business_local".to_string()),
            fact_key,
            request.value.to_string(),
            normalize_optional_string(request.source_kind)
                .unwrap_or_else(|| "operator".to_string()),
            normalize_optional_string(request.source_label),
            normalize_optional_string(request.source_uri),
            request.provenance.unwrap_or_else(|| json!({})).to_string(),
            visibility.as_str(),
            publication_state.as_str(),
            actor_id,
            now,
            published_at,
            archived_at,
        ],
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "business.fact.created",
            json!({
                "factId": id,
                "factKey": fact_key,
                "visibility": visibility.as_str(),
                "publicationState": publication_state.as_str(),
            }),
        ),
    )?;
    transaction.commit()?;
    let record = find_business_fact(&connection, &id)?.expect("business fact just inserted");
    Ok((record.into_view(), event))
}

pub fn update_business_fact(
    db_path: &Path,
    fact_id: &str,
    request: BusinessFactWriteRequest,
    actor_id: Option<&str>,
) -> Result<(BusinessFactView, RealtimeEvent)> {
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let existing = find_business_fact(&transaction, fact_id)?
        .ok_or_else(|| anyhow::anyhow!("Business fact was not found: {fact_id}"))?;
    let now = Utc::now().to_rfc3339();
    let fact_key = require_identifier(&request.fact_key, "Fact key")?;
    let visibility = request.visibility.unwrap_or(existing.visibility);
    let publication_state = request
        .publication_state
        .unwrap_or(existing.publication_state);
    let published_at = if publication_state == PublicationState::Published
        && existing.publication_state != PublicationState::Published
    {
        Some(now.clone())
    } else {
        existing.published_at
    };
    let archived_at = if matches!(
        publication_state,
        PublicationState::Archived | PublicationState::Revoked
    ) && !matches!(
        existing.publication_state,
        PublicationState::Archived | PublicationState::Revoked
    ) {
        Some(now.clone())
    } else {
        existing.archived_at
    };
    transaction.execute(
        "UPDATE business_facts
         SET subject_type = ?1,
             subject_id = ?2,
             fact_key = ?3,
             value_json = ?4,
             source_kind = ?5,
             source_label = ?6,
             source_uri = ?7,
             provenance_json = ?8,
             visibility = ?9,
             publication_state = ?10,
             created_by_actor_id = COALESCE(created_by_actor_id, ?11),
             updated_at = ?12,
             published_at = ?13,
             archived_at = ?14
         WHERE id = ?15",
        params![
            normalize_optional_string(request.subject_type).unwrap_or(existing.subject_type),
            normalize_optional_string(request.subject_id).unwrap_or(existing.subject_id),
            fact_key,
            request.value.to_string(),
            normalize_optional_string(request.source_kind).unwrap_or(existing.source_kind),
            normalize_optional_string(request.source_label).or(existing.source_label),
            normalize_optional_string(request.source_uri).or(existing.source_uri),
            request
                .provenance
                .unwrap_or(existing.provenance)
                .to_string(),
            visibility.as_str(),
            publication_state.as_str(),
            actor_id,
            now,
            published_at,
            archived_at,
            fact_id,
        ],
    )?;
    let event = append_realtime_event_tx(
        &transaction,
        &system_event(
            "business.fact.updated",
            json!({
                "factId": fact_id,
                "factKey": fact_key,
                "visibility": visibility.as_str(),
                "publicationState": publication_state.as_str(),
            }),
        ),
    )?;
    transaction.commit()?;
    let record = find_business_fact(&connection, fact_id)?.expect("business fact just updated");
    Ok((record.into_view(), event))
}

pub fn list_business_facts_connection(
    connection: &Connection,
    viewer: BusinessFactViewer,
) -> Result<BusinessFactListResponse> {
    let mut statement = connection.prepare(
        "SELECT id, subject_type, subject_id, fact_key, value_json, source_kind, source_label,
                source_uri, provenance_json, visibility, publication_state, created_by_actor_id,
                created_at, updated_at, published_at, archived_at
         FROM business_facts
         ORDER BY updated_at DESC, id DESC",
    )?;
    let rows = statement.query_map([], business_fact_from_row)?;
    let mut facts = Vec::new();
    for row in rows {
        let record = row?;
        if can_view_business_fact(viewer, record.visibility, record.publication_state) {
            facts.push(record.into_view());
        }
    }
    Ok(BusinessFactListResponse { facts })
}

pub fn can_view_business_fact(
    viewer: BusinessFactViewer,
    visibility: BusinessFactVisibility,
    publication_state: PublicationState,
) -> bool {
    if viewer == BusinessFactViewer::Owner {
        return true;
    }
    if publication_state != PublicationState::Published {
        return false;
    }
    match viewer {
        BusinessFactViewer::Public => visibility == BusinessFactVisibility::Public,
        BusinessFactViewer::Authenticated => matches!(
            visibility,
            BusinessFactVisibility::Public | BusinessFactVisibility::Authenticated
        ),
        BusinessFactViewer::Staff => matches!(
            visibility,
            BusinessFactVisibility::Public
                | BusinessFactVisibility::Authenticated
                | BusinessFactVisibility::Staff
        ),
        BusinessFactViewer::Owner => true,
    }
}

fn find_business_fact(
    connection: &Connection,
    fact_id: &str,
) -> rusqlite::Result<Option<BusinessFactRecord>> {
    connection
        .query_row(
            "SELECT id, subject_type, subject_id, fact_key, value_json, source_kind, source_label,
                    source_uri, provenance_json, visibility, publication_state, created_by_actor_id,
                    created_at, updated_at, published_at, archived_at
             FROM business_facts
             WHERE id = ?1",
            [fact_id],
            business_fact_from_row,
        )
        .optional()
}

fn business_fact_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BusinessFactRecord> {
    let value_json: String = row.get(4)?;
    let provenance_json: String = row.get(8)?;
    let visibility: String = row.get(9)?;
    let publication_state: String = row.get(10)?;
    Ok(BusinessFactRecord {
        id: row.get(0)?,
        subject_type: row.get(1)?,
        subject_id: row.get(2)?,
        fact_key: row.get(3)?,
        value: serde_json::from_str(&value_json).unwrap_or(Value::Null),
        source_kind: row.get(5)?,
        source_label: row.get(6)?,
        source_uri: row.get(7)?,
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        visibility: BusinessFactVisibility::try_from(visibility.as_str()).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, error.into())
        })?,
        publication_state: PublicationState::try_from(publication_state.as_str()).map_err(
            |error| {
                rusqlite::Error::FromSqlConversionFailure(
                    10,
                    rusqlite::types::Type::Text,
                    error.into(),
                )
            },
        )?,
        created_by_actor_id: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        published_at: row.get(14)?,
        archived_at: row.get(15)?,
    })
}

impl BusinessFactRecord {
    fn into_view(self) -> BusinessFactView {
        BusinessFactView {
            id: self.id,
            subject_type: self.subject_type,
            subject_id: self.subject_id,
            fact_key: self.fact_key,
            value: self.value,
            source_kind: self.source_kind,
            source_label: self.source_label,
            source_uri: self.source_uri,
            provenance: self.provenance,
            visibility: self.visibility,
            publication_state: self.publication_state,
            created_by_actor_id: self.created_by_actor_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            published_at: self.published_at,
            archived_at: self.archived_at,
        }
    }
}

fn require_identifier(value: &str, label: &str) -> Result<String> {
    let normalized = normalize_optional_string(Some(value.to_string()))
        .ok_or_else(|| anyhow::anyhow!("{label} is required."))?;
    if normalized.len() > 120
        || !normalized.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
    {
        bail!("{label} must be a stable identifier.");
    }
    Ok(normalized)
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().replace(char::is_whitespace, " "))
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::LOCAL_OWNER_ACTOR_ID;
    use crate::schema::init_database;
    use tempfile::TempDir;

    #[test]
    fn business_fact_visibility_policy_blocks_public_private_and_draft() {
        assert!(can_view_business_fact(
            BusinessFactViewer::Public,
            BusinessFactVisibility::Public,
            PublicationState::Published
        ));
        assert!(!can_view_business_fact(
            BusinessFactViewer::Public,
            BusinessFactVisibility::Owner,
            PublicationState::Published
        ));
        assert!(!can_view_business_fact(
            BusinessFactViewer::Public,
            BusinessFactVisibility::Public,
            PublicationState::Draft
        ));
        assert!(can_view_business_fact(
            BusinessFactViewer::Owner,
            BusinessFactVisibility::Owner,
            PublicationState::Draft
        ));
    }

    #[test]
    fn create_business_fact_persists_provenance_and_emits_event() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        let (fact, event) = create_business_fact(
            &db_path,
            BusinessFactWriteRequest {
                subject_type: None,
                subject_id: None,
                fact_key: "about.tagline".to_string(),
                value: json!("Local-first operations for one-person businesses."),
                source_kind: Some("operator".to_string()),
                source_label: Some("Setup interview".to_string()),
                source_uri: None,
                provenance: Some(json!({ "note": "seeded by owner" })),
                visibility: Some(BusinessFactVisibility::Public),
                publication_state: Some(PublicationState::Published),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(fact.fact_key, "about.tagline");
        assert_eq!(fact.visibility, BusinessFactVisibility::Public);
        assert_eq!(fact.publication_state, PublicationState::Published);
        assert_eq!(fact.provenance["note"], "seeded by owner");
        assert_eq!(event.event_type, "business.fact.created");
        assert!(!event.payload.to_string().contains("Local-first operations"));
    }

    #[test]
    fn public_query_only_returns_published_public_facts() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        for (fact_key, visibility, publication_state) in [
            (
                "about.public",
                BusinessFactVisibility::Public,
                PublicationState::Published,
            ),
            (
                "about.draft",
                BusinessFactVisibility::Public,
                PublicationState::Draft,
            ),
            (
                "about.owner",
                BusinessFactVisibility::Owner,
                PublicationState::Published,
            ),
        ] {
            create_business_fact(
                &db_path,
                BusinessFactWriteRequest {
                    subject_type: None,
                    subject_id: None,
                    fact_key: fact_key.to_string(),
                    value: json!(fact_key),
                    source_kind: None,
                    source_label: None,
                    source_uri: None,
                    provenance: None,
                    visibility: Some(visibility),
                    publication_state: Some(publication_state),
                },
                Some(LOCAL_OWNER_ACTOR_ID),
            )
            .unwrap();
        }

        let public = list_business_facts(
            &db_path,
            BusinessFactQuery {
                viewer: Some(BusinessFactViewer::Public),
            },
        )
        .unwrap();

        assert_eq!(public.facts.len(), 1);
        assert_eq!(public.facts[0].fact_key, "about.public");
    }
}
