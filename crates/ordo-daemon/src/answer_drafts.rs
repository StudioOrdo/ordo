use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

use crate::corpus::{retrieve_corpus, CorpusRetrievalQuery, CorpusRetrievalResponse, CorpusViewer};
use crate::policy::{
    provenance_metadata, ActorContext, ActorKind, PolicyAction, ResourceClassification,
    ResourceKind, ResourceRef,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerDraftRequest {
    pub question: String,
    pub viewer: Option<CorpusViewer>,
    pub actor_id: Option<String>,
    pub limit: Option<usize>,
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerDraftListResponse {
    pub drafts: Vec<AnswerDraftView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerDraftResponse {
    pub draft: AnswerDraftView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerDraftView {
    pub id: String,
    pub status: String,
    pub question: String,
    pub prompt_input: Value,
    pub retrieval_query: Value,
    pub retrieval_evidence: Value,
    pub cited_item_ids: Vec<String>,
    pub draft_markdown: String,
    pub limitations: Vec<String>,
    pub provenance: Value,
    pub citations: Vec<AnswerDraftCitationView>,
    pub created_by_actor_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerDraftCitationView {
    pub id: String,
    pub draft_id: String,
    pub corpus_item_id: String,
    pub corpus_source_id: String,
    pub content_hash: String,
    pub rank: f64,
    pub snippet: String,
    pub evidence: Value,
    pub created_at: String,
}

struct AnswerDraftRecord {
    id: String,
    status: String,
    question: String,
    prompt_input: Value,
    retrieval_query: Value,
    retrieval_evidence: Value,
    cited_item_ids: Vec<String>,
    draft_markdown: String,
    limitations: Vec<String>,
    provenance: Value,
    created_by_actor_id: Option<String>,
    created_at: String,
    updated_at: String,
}

pub fn list_answer_drafts(db_path: &Path) -> Result<AnswerDraftListResponse> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, status, question, prompt_input_json, retrieval_query_json,
                retrieval_evidence_json, cited_item_ids_json, draft_markdown, limitations_json,
                provenance_json, created_by_actor_id, created_at, updated_at
         FROM answer_drafts ORDER BY updated_at DESC, created_at DESC",
    )?;
    let rows = statement.query_map([], answer_draft_from_row)?;
    let mut drafts = Vec::new();
    for row in rows {
        let record = row?;
        let citations = load_answer_draft_citations(&connection, &record.id)?;
        drafts.push(record.into_view(citations));
    }
    Ok(AnswerDraftListResponse { drafts })
}

pub fn read_answer_draft(db_path: &Path, draft_id: &str) -> Result<AnswerDraftResponse> {
    let connection = Connection::open(db_path)?;
    let record = require_answer_draft(&connection, draft_id)?;
    let citations = load_answer_draft_citations(&connection, draft_id)?;
    Ok(AnswerDraftResponse {
        draft: record.into_view(citations),
    })
}

pub fn prepare_answer_draft(
    db_path: &Path,
    request: AnswerDraftRequest,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<AnswerDraftResponse> {
    let question = require_non_empty(&request.question, "Question")?;
    let safe_question = redact_prompt_text(&question);
    let safe_instructions = request.instructions.as_deref().map(redact_prompt_text);
    let retrieval = retrieve_corpus(
        db_path,
        CorpusRetrievalQuery {
            query: question.clone(),
            viewer: request.viewer,
            actor_id: request
                .actor_id
                .clone()
                .or_else(|| actor_id.map(ToString::to_string)),
            limit: request.limit,
        },
    )?;
    let mut connection = Connection::open(db_path)?;
    let transaction = connection.transaction()?;
    let draft_id = format!("answer_draft_{}", Uuid::new_v4());
    let now = Utc::now().to_rfc3339();
    let cited_item_ids = retrieval
        .results
        .iter()
        .map(|result| result.item.id.clone())
        .collect::<Vec<_>>();
    let status = if retrieval.results.is_empty() {
        "needs_evidence"
    } else {
        "drafted_with_evidence"
    };
    let limitations = answer_limitations(&retrieval);
    let draft_markdown = render_answer_draft_markdown(&retrieval, &limitations);
    let retrieval_query = json!({
        "query": safe_question,
        "viewer": retrieval.viewer,
        "limit": request.limit.unwrap_or(10).clamp(1, 25),
        "providerCall": "not_performed",
    });
    let prompt_input = json!({
        "question": redact_prompt_text(&question),
        "instructions": safe_instructions,
        "providerCall": "not_performed",
        "generationMode": "local_evidence_scaffold",
    });
    let retrieval_evidence = redacted_retrieval_evidence(&retrieval)?;
    let provenance = answer_draft_provenance(&draft_id, origin, actor_id);
    transaction.execute(
        "INSERT INTO answer_drafts (
            id, status, question, prompt_input_json, retrieval_query_json, retrieval_evidence_json,
            cited_item_ids_json, draft_markdown, limitations_json, provenance_json,
            created_by_actor_id, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
        params![
            draft_id,
            status,
            redact_prompt_text(&question),
            prompt_input.to_string(),
            retrieval_query.to_string(),
            retrieval_evidence.to_string(),
            serde_json::to_string(&cited_item_ids)?,
            draft_markdown,
            serde_json::to_string(&limitations)?,
            provenance.to_string(),
            actor_id,
            now,
        ],
    )?;
    for result in &retrieval.results {
        transaction.execute(
            "INSERT INTO answer_draft_citations (
                id, draft_id, corpus_item_id, corpus_source_id, content_hash, rank, snippet,
                evidence_json, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                format!("answer_draft_citation_{}", Uuid::new_v4()),
                draft_id,
                result.item.id,
                result.source.id,
                result.item.content_hash,
                result.rank,
                redact_prompt_text(&result.snippet),
                redact_value_strings(result.evidence.clone()).to_string(),
                now,
            ],
        )?;
    }
    transaction.commit()?;
    read_answer_draft(db_path, &draft_id)
}

fn require_answer_draft(connection: &Connection, draft_id: &str) -> Result<AnswerDraftRecord> {
    connection
        .query_row(
            "SELECT id, status, question, prompt_input_json, retrieval_query_json,
                    retrieval_evidence_json, cited_item_ids_json, draft_markdown, limitations_json,
                    provenance_json, created_by_actor_id, created_at, updated_at
             FROM answer_drafts WHERE id = ?1",
            [draft_id],
            answer_draft_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("Answer draft was not found: {draft_id}"))
}

fn answer_draft_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AnswerDraftRecord> {
    let prompt_input_json: String = row.get(3)?;
    let retrieval_query_json: String = row.get(4)?;
    let retrieval_evidence_json: String = row.get(5)?;
    let cited_item_ids_json: String = row.get(6)?;
    let limitations_json: String = row.get(8)?;
    let provenance_json: String = row.get(9)?;
    Ok(AnswerDraftRecord {
        id: row.get(0)?,
        status: row.get(1)?,
        question: row.get(2)?,
        prompt_input: serde_json::from_str(&prompt_input_json).unwrap_or_else(|_| json!({})),
        retrieval_query: serde_json::from_str(&retrieval_query_json).unwrap_or_else(|_| json!({})),
        retrieval_evidence: serde_json::from_str(&retrieval_evidence_json)
            .unwrap_or_else(|_| json!({})),
        cited_item_ids: serde_json::from_str(&cited_item_ids_json).unwrap_or_default(),
        draft_markdown: row.get(7)?,
        limitations: serde_json::from_str(&limitations_json).unwrap_or_default(),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        created_by_actor_id: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn load_answer_draft_citations(
    connection: &Connection,
    draft_id: &str,
) -> Result<Vec<AnswerDraftCitationView>> {
    let mut statement = connection.prepare(
        "SELECT id, draft_id, corpus_item_id, corpus_source_id, content_hash, rank, snippet,
                evidence_json, created_at
         FROM answer_draft_citations WHERE draft_id = ?1 ORDER BY rank, created_at",
    )?;
    let rows = statement.query_map([draft_id], |row| {
        let evidence_json: String = row.get(7)?;
        Ok(AnswerDraftCitationView {
            id: row.get(0)?,
            draft_id: row.get(1)?,
            corpus_item_id: row.get(2)?,
            corpus_source_id: row.get(3)?,
            content_hash: row.get(4)?,
            rank: row.get(5)?,
            snippet: row.get(6)?,
            evidence: serde_json::from_str(&evidence_json).unwrap_or_else(|_| json!({})),
            created_at: row.get(8)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

impl AnswerDraftRecord {
    fn into_view(self, citations: Vec<AnswerDraftCitationView>) -> AnswerDraftView {
        AnswerDraftView {
            id: self.id,
            status: self.status,
            question: self.question,
            prompt_input: self.prompt_input,
            retrieval_query: self.retrieval_query,
            retrieval_evidence: self.retrieval_evidence,
            cited_item_ids: self.cited_item_ids,
            draft_markdown: self.draft_markdown,
            limitations: self.limitations,
            provenance: self.provenance,
            citations,
            created_by_actor_id: self.created_by_actor_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn render_answer_draft_markdown(
    retrieval: &CorpusRetrievalResponse,
    limitations: &[String],
) -> String {
    if retrieval.results.is_empty() {
        return format!(
            "## Draft Status\n\nNeeds evidence. No answer draft was generated because no approved visible corpus evidence matched the request.\n\n## Limitations\n\n{}\n",
            limitations
                .iter()
                .map(|limitation| format!("- {limitation}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    let mut body = String::from("## Evidence-Backed Draft\n\n");
    body.push_str(
        "The following draft is a local evidence scaffold. It only summarizes retrieved source snippets and should be reviewed before use.\n\n",
    );
    body.push_str("## Supported Points\n\n");
    for (index, result) in retrieval.results.iter().enumerate() {
        body.push_str(&format!(
            "{}. {} [source: {}]\n",
            index + 1,
            normalize_snippet(&redact_prompt_text(&result.snippet)),
            result.item.id
        ));
    }
    body.push_str("\n## Citations\n\n");
    for result in &retrieval.results {
        body.push_str(&format!(
            "- {} from {} ({})\n",
            result.item.id, result.source.id, result.item.content_hash
        ));
    }
    body.push_str("\n## Limitations\n\n");
    for limitation in limitations {
        body.push_str(&format!("- {limitation}\n"));
    }
    body
}

fn answer_limitations(retrieval: &CorpusRetrievalResponse) -> Vec<String> {
    let mut limitations = retrieval.limitations.clone();
    limitations.push("No provider or model call was performed in this backend slice.".to_string());
    limitations.push(
        "The draft may only use cited corpus item evidence and must not add unsupported claims."
            .to_string(),
    );
    if retrieval.results.is_empty() {
        limitations.push("Missing evidence prevented answer drafting.".to_string());
    }
    limitations
}

fn answer_draft_provenance(draft_id: &str, origin: &str, actor_id: Option<&str>) -> Value {
    provenance_metadata(
        actor_context_for_origin(origin, actor_id),
        PolicyAction::Prepare,
        ResourceRef::new(ResourceKind::AnswerDraft, draft_id),
        Some("answer.drafts.prepare"),
        ResourceClassification::local_operations_ready_for_review(),
    )
}

fn actor_context_for_origin(origin: &str, actor_id: Option<&str>) -> ActorContext {
    let kind = match origin {
        "mcp" => ActorKind::McpClient,
        "scheduler" => ActorKind::Scheduler,
        "system" => ActorKind::System,
        _ => ActorKind::BrowserOperator,
    };
    ActorContext::new(kind, origin, actor_id.map(ToString::to_string))
}

fn normalize_snippet(snippet: &str) -> String {
    snippet
        .replace(['[', ']'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn redacted_retrieval_evidence(retrieval: &CorpusRetrievalResponse) -> Result<Value> {
    let mut evidence = redact_value_strings(serde_json::to_value(retrieval)?);
    if let Some(object) = evidence.as_object_mut() {
        object.insert(
            "query".to_string(),
            json!(redact_prompt_text(&retrieval.query)),
        );
    }
    Ok(evidence)
}

fn redact_value_strings(value: Value) -> Value {
    match value {
        Value::String(raw) => Value::String(redact_prompt_text(&raw)),
        Value::Array(values) => {
            Value::Array(values.into_iter().map(redact_value_strings).collect())
        }
        Value::Object(entries) => Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key, redact_value_strings(value)))
                .collect(),
        ),
        other => other,
    }
}

fn redact_prompt_text(value: &str) -> String {
    value
        .split_whitespace()
        .map(|token| {
            if contains_secret_indicator(token) {
                "[REDACTED]".to_string()
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_secret_indicator(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "api_key", "apikey", "token", "password", "secret", "bearer", "sk-",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn require_non_empty(value: &str, label: &str) -> Result<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        bail!("{label} is required");
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::{
        create_corpus_item, create_corpus_source, CorpusItemWriteRequest, CorpusSourceWriteRequest,
        CorpusStatus, CorpusVisibility,
    };
    use crate::policy::LOCAL_OWNER_ACTOR_ID;
    use crate::schema::init_database;
    use tempfile::TempDir;

    fn setup_db() -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        (temp_dir, db_path)
    }

    fn seed_public_evidence(db_path: &Path, body_text: &str) -> String {
        let (source, _) = create_corpus_source(
            db_path,
            CorpusSourceWriteRequest {
                source_kind: Some("operator_text".to_string()),
                label: "Answer source".to_string(),
                uri: None,
                resource_kind: None,
                resource_id: None,
                status: Some(CorpusStatus::Approved),
                visibility: Some(CorpusVisibility::Public),
                provenance: Some(json!({ "source": "test" })),
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        let (item, _) = create_corpus_item(
            db_path,
            CorpusItemWriteRequest {
                source_id: source.id,
                item_kind: Some("chunk".to_string()),
                ordinal: Some(1),
                title: "Answer item".to_string(),
                body_text: body_text.to_string(),
                resource_kind: None,
                resource_id: None,
                status: Some(CorpusStatus::Approved),
                visibility: Some(CorpusVisibility::Public),
                provenance: Some(json!({ "lineage": "test" })),
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        item.id
    }

    #[test]
    fn answer_draft_requires_retrieval_evidence_and_records_citations() {
        let (_temp_dir, db_path) = setup_db();
        let item_id = seed_public_evidence(
            &db_path,
            "Studio Ordo offers local-first scheduling help for small studios.",
        );

        let response = prepare_answer_draft(
            &db_path,
            AnswerDraftRequest {
                question: "What scheduling help does Studio Ordo offer?".to_string(),
                viewer: Some(CorpusViewer::Public),
                actor_id: None,
                limit: Some(5),
                instructions: Some("Use evidence only".to_string()),
            },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(response.draft.status, "drafted_with_evidence");
        assert_eq!(response.draft.cited_item_ids, vec![item_id.clone()]);
        assert_eq!(response.draft.citations.len(), 1);
        assert_eq!(response.draft.citations[0].corpus_item_id, item_id);
        assert!(response.draft.draft_markdown.contains("[source:"));
        assert_eq!(
            response.draft.provenance["resource"]["kind"],
            "answer_draft"
        );
        assert_eq!(response.draft.prompt_input["providerCall"], "not_performed");
    }

    #[test]
    fn missing_evidence_marks_draft_without_generating_claims() {
        let (_temp_dir, db_path) = setup_db();
        let response = prepare_answer_draft(
            &db_path,
            AnswerDraftRequest {
                question: "Claim that Ordo supports teleportation".to_string(),
                viewer: Some(CorpusViewer::Public),
                actor_id: None,
                limit: Some(3),
                instructions: None,
            },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(response.draft.status, "needs_evidence");
        assert!(response.draft.cited_item_ids.is_empty());
        assert!(response.draft.draft_markdown.contains("Needs evidence"));
        assert!(!response.draft.draft_markdown.contains("teleportation"));
        assert_eq!(
            response.draft.retrieval_evidence["evidenceState"],
            "missing_evidence"
        );
    }

    #[test]
    fn prompt_secret_material_is_redacted() {
        let (_temp_dir, db_path) = setup_db();
        seed_public_evidence(&db_path, "Ordo keeps operations local and evidence-backed.");
        let response = prepare_answer_draft(
            &db_path,
            AnswerDraftRequest {
                question: "Use api_key sk-live-secret to answer local operations".to_string(),
                viewer: Some(CorpusViewer::Public),
                actor_id: None,
                limit: Some(3),
                instructions: Some("bearer token should not persist".to_string()),
            },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert!(!response.draft.question.contains("sk-live-secret"));
        assert!(!response
            .draft
            .prompt_input
            .to_string()
            .contains("sk-live-secret"));
        assert!(!response
            .draft
            .retrieval_evidence
            .to_string()
            .contains("sk-live-secret"));
        assert!(!response
            .draft
            .citations
            .iter()
            .any(|citation| citation.snippet.contains("sk-live-secret")));
        assert!(!response.draft.prompt_input.to_string().contains("bearer"));
        assert!(response
            .draft
            .prompt_input
            .to_string()
            .contains("[REDACTED]"));
    }
}
