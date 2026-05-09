use anyhow::{ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::conversations::{append_conversation_event, ConversationMessageView};
use crate::events::RealtimeEvent;

pub const CONVERSATION_ANALYSIS_SCHEMA_VERSION: &str = "conversation.analysis.v1";
pub const DEFAULT_ANALYSIS_KIND: &str = "continuous_message_analysis";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl AnalysisJobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationAnalysisJobView {
    pub id: String,
    pub conversation_id: String,
    pub segment_id: Option<String>,
    pub analysis_kind: String,
    pub status: String,
    pub source_message_id: Option<String>,
    pub source_event_cursor_start: Option<i64>,
    pub source_event_cursor_end: Option<i64>,
    pub input_refs: Vec<String>,
    pub output: Value,
    pub policy_decision_id: Option<String>,
    pub llm_run_id: Option<String>,
    pub error_message_hash: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationAnalysisCandidateView {
    pub id: String,
    pub job_id: String,
    pub conversation_id: String,
    pub segment_id: Option<String>,
    pub candidate_kind: String,
    pub candidate_state: String,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub prompt_slot_ids: Vec<String>,
    pub llm_run_id: Option<String>,
    pub content_hash: String,
    pub summary_text: String,
    pub body: Value,
    pub visibility: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationBriefCandidateView {
    pub id: String,
    pub job_id: String,
    pub conversation_id: String,
    pub segment_id: Option<String>,
    pub candidate_state: String,
    pub title: String,
    pub brief_markdown: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub provenance: Value,
    pub content_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMemoryCandidateView {
    pub id: String,
    pub job_id: String,
    pub conversation_id: String,
    pub segment_id: Option<String>,
    pub memory_kind: String,
    pub candidate_state: String,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub content_hash: String,
    pub summary_text: String,
    pub body: Value,
    pub visibility: String,
    pub approval_status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationAnalysisRunResult {
    pub job: ConversationAnalysisJobView,
    pub candidates: Vec<ConversationAnalysisCandidateView>,
    pub brief_candidate: Option<ConversationBriefCandidateView>,
    pub memory_candidate: Option<ConversationMemoryCandidateView>,
    pub events: Vec<RealtimeEvent>,
}

pub fn queue_analysis_for_message(
    connection: &Connection,
    message: &ConversationMessageView,
) -> Result<Option<ConversationAnalysisJobView>> {
    if !is_message_eligible(message) {
        return Ok(None);
    }
    if let Some(existing) = load_job_for_message(connection, &message.conversation_id, &message.id)?
    {
        return Ok(Some(existing));
    }

    let now = Utc::now().to_rfc3339();
    let job_id = format!("conversation_analysis_job_{}", Uuid::new_v4());
    let input_refs = vec![evidence_ref(message)];
    connection.execute(
        "INSERT INTO conversation_analysis_jobs (
            id,
            conversation_id,
            segment_id,
            analysis_kind,
            status,
            source_message_id,
            source_event_cursor_start,
            source_event_cursor_end,
            input_refs_json,
            created_at,
            updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8, ?9, ?9)",
        params![
            job_id,
            message.conversation_id,
            message.segment_id,
            DEFAULT_ANALYSIS_KIND,
            AnalysisJobStatus::Queued.as_str(),
            message.id,
            message.event_cursor,
            json!(input_refs).to_string(),
            now
        ],
    )?;
    append_conversation_event(
        connection,
        &message.conversation_id,
        message.segment_id.as_deref(),
        None,
        "conversation.analysis.queued",
        json!({
            "jobId": job_id,
            "analysisKind": DEFAULT_ANALYSIS_KIND,
            "sourceMessageId": message.id,
            "sourceEventCursor": message.event_cursor,
            "inputRefs": input_refs,
        }),
        None,
    )?;
    load_job(connection, &job_id).map(Some)
}

pub fn run_deterministic_analysis_for_job(
    connection: &Connection,
    job_id: &str,
) -> Result<ConversationAnalysisRunResult> {
    let job = load_job(connection, job_id)?;
    if job.status == AnalysisJobStatus::Completed.as_str() {
        return load_completed_result(connection, &job);
    }
    ensure!(
        job.status == AnalysisJobStatus::Queued.as_str(),
        "analysis job must be queued before deterministic analysis"
    );
    let message_id = job
        .source_message_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("analysis job source message is required"))?;
    let message = load_message_for_analysis(connection, message_id)?;
    ensure!(
        is_message_eligible(&message),
        "analysis source message is no longer eligible"
    );

    let mut events = Vec::new();
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE conversation_analysis_jobs
         SET status = ?2, started_at = ?3, updated_at = ?3
         WHERE id = ?1",
        params![job.id, AnalysisJobStatus::Running.as_str(), now],
    )?;
    events.push(append_conversation_event(
        connection,
        &job.conversation_id,
        job.segment_id.as_deref(),
        None,
        "conversation.analysis.started",
        json!({
            "jobId": job.id,
            "analysisKind": job.analysis_kind,
            "sourceMessageId": message.id,
        }),
        None,
    )?);

    let evidence_refs = vec![evidence_ref(&message)];
    let provenance = json!({
        "schemaVersion": CONVERSATION_ANALYSIS_SCHEMA_VERSION,
        "generator": "deterministic.local",
        "sourceMessageId": message.id,
        "sourceEventCursor": message.event_cursor,
    });
    let mut candidates = Vec::new();
    candidates.push(insert_candidate(
        connection,
        &job,
        CandidateInsert {
            candidate_kind: "conversation_summary_signal",
            confidence: 0.72,
            evidence_refs: &evidence_refs,
            provenance: provenance.clone(),
            prompt_slot_ids: vec!["recent_conversation_window".to_string()],
            visibility: "participants",
            summary_text: format!("Recent message: {}", safe_excerpt(&message.body_markdown)),
            body: json!({
                "sourceMessageId": message.id,
                "messageKind": message.message_kind,
            }),
        },
    )?);
    if message.body_markdown.contains('?') {
        candidates.push(insert_candidate(
            connection,
            &job,
            CandidateInsert {
                candidate_kind: "open_question",
                confidence: 0.78,
                evidence_refs: &evidence_refs,
                provenance: provenance.clone(),
                prompt_slot_ids: vec!["recent_conversation_window".to_string()],
                visibility: "staff_private",
                summary_text: format!("Open question: {}", safe_excerpt(&message.body_markdown)),
                body: json!({ "questionDetected": true }),
            },
        )?);
    }
    if looks_action_needed(&message.body_markdown) {
        candidates.push(insert_candidate(
            connection,
            &job,
            CandidateInsert {
                candidate_kind: "action_needed",
                confidence: 0.8,
                evidence_refs: &evidence_refs,
                provenance: provenance.clone(),
                prompt_slot_ids: vec!["recent_conversation_window".to_string()],
                visibility: "staff_private",
                summary_text: format!(
                    "Potential action needed: {}",
                    safe_excerpt(&message.body_markdown)
                ),
                body: json!({ "actionSignal": "request_or_commitment" }),
            },
        )?);
        increment_action_count(connection, &job.conversation_id)?;
    }
    if looks_handoff_needed(&message.body_markdown) {
        candidates.push(insert_candidate(
            connection,
            &job,
            CandidateInsert {
                candidate_kind: "handoff_signal",
                confidence: 0.83,
                evidence_refs: &evidence_refs,
                provenance: provenance.clone(),
                prompt_slot_ids: vec!["recent_conversation_window".to_string()],
                visibility: "staff_private",
                summary_text: format!(
                    "Potential handoff signal: {}",
                    safe_excerpt(&message.body_markdown)
                ),
                body: json!({ "handoffSignal": "human_or_sensitive_request" }),
            },
        )?);
    }

    let brief = insert_brief_candidate(
        connection,
        &job,
        &message,
        &evidence_refs,
        provenance.clone(),
    )?;
    let memory = insert_memory_candidate(connection, &job, &message, &evidence_refs, provenance)?;
    let output = json!({
        "candidateCount": candidates.len(),
        "briefCandidateId": brief.id,
        "memoryCandidateId": memory.id,
        "actionCountIncremented": candidates.iter().any(|candidate| candidate.candidate_kind == "action_needed"),
        "status": "completed",
    });
    let completed_at = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE conversation_analysis_jobs
         SET status = ?2, output_json = ?3, completed_at = ?4, updated_at = ?4
         WHERE id = ?1",
        params![
            job.id,
            AnalysisJobStatus::Completed.as_str(),
            output.to_string(),
            completed_at
        ],
    )?;
    update_conversation_summary(connection, &job.conversation_id, &brief, &message)?;
    events.push(append_conversation_event(
        connection,
        &job.conversation_id,
        job.segment_id.as_deref(),
        None,
        "brief.candidate.created",
        json!({
            "jobId": job.id,
            "briefCandidateId": brief.id,
            "candidateState": brief.candidate_state,
            "evidenceRefs": brief.evidence_refs,
            "contentHash": brief.content_hash,
        }),
        None,
    )?);
    events.push(append_conversation_event(
        connection,
        &job.conversation_id,
        job.segment_id.as_deref(),
        None,
        "memory.candidate.created",
        json!({
            "jobId": job.id,
            "memoryCandidateId": memory.id,
            "candidateState": memory.candidate_state,
            "approvalStatus": memory.approval_status,
            "evidenceRefs": memory.evidence_refs,
            "contentHash": memory.content_hash,
        }),
        None,
    )?);
    events.push(append_conversation_event(
        connection,
        &job.conversation_id,
        job.segment_id.as_deref(),
        None,
        "conversation.tags.updated",
        json!({
            "jobId": job.id,
            "candidateState": "proposed",
            "candidateIds": candidates.iter().map(|candidate| candidate.id.as_str()).collect::<Vec<_>>(),
            "evidenceRefs": evidence_refs,
        }),
        None,
    )?);
    events.push(append_conversation_event(
        connection,
        &job.conversation_id,
        job.segment_id.as_deref(),
        None,
        "conversation.analysis.completed",
        json!({
            "jobId": job.id,
            "analysisKind": job.analysis_kind,
            "candidateCount": candidates.len(),
            "briefCandidateId": brief.id,
            "memoryCandidateId": memory.id,
        }),
        None,
    )?);

    Ok(ConversationAnalysisRunResult {
        job: load_job(connection, &job.id)?,
        candidates,
        brief_candidate: Some(brief),
        memory_candidate: Some(memory),
        events,
    })
}

pub fn fail_analysis_job(connection: &Connection, job_id: &str, message: &str) -> Result<()> {
    ensure!(
        !message.trim().is_empty(),
        "analysis failure message is required"
    );
    let job = load_job(connection, job_id)?;
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE conversation_analysis_jobs
         SET status = ?2, error_message_hash = ?3, completed_at = ?4, updated_at = ?4
         WHERE id = ?1",
        params![
            job_id,
            AnalysisJobStatus::Failed.as_str(),
            stable_hash(message),
            now
        ],
    )?;
    append_conversation_event(
        connection,
        &job.conversation_id,
        job.segment_id.as_deref(),
        None,
        "conversation.analysis.failed",
        json!({
            "jobId": job_id,
            "analysisKind": job.analysis_kind,
            "errorHash": stable_hash(message),
        }),
        None,
    )?;
    Ok(())
}

struct CandidateInsert<'a> {
    candidate_kind: &'a str,
    confidence: f64,
    evidence_refs: &'a [String],
    provenance: Value,
    prompt_slot_ids: Vec<String>,
    visibility: &'a str,
    summary_text: String,
    body: Value,
}

fn insert_candidate(
    connection: &Connection,
    job: &ConversationAnalysisJobView,
    candidate: CandidateInsert<'_>,
) -> Result<ConversationAnalysisCandidateView> {
    ensure!(
        !candidate.evidence_refs.is_empty(),
        "candidate evidence refs are required"
    );
    ensure!(
        !candidate
            .provenance
            .as_object()
            .map(|object| object.is_empty())
            .unwrap_or(true),
        "candidate provenance is required"
    );
    let now = Utc::now().to_rfc3339();
    let id = format!("conversation_analysis_candidate_{}", Uuid::new_v4());
    let safe_summary = sanitize_public_text(&candidate.summary_text);
    let body = sanitize_json(candidate.body);
    let content_hash = stable_hash(&format!(
        "{}|{safe_summary}|{body}",
        candidate.candidate_kind
    ));
    connection.execute(
        "INSERT INTO conversation_analysis_candidates (
            id, job_id, conversation_id, segment_id, candidate_kind, candidate_state,
            confidence, evidence_refs_json, provenance_json, prompt_slot_ids_json,
            llm_run_id, content_hash, summary_text, body_json, visibility, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, 'proposed', ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
        params![
            id,
            job.id,
            job.conversation_id,
            job.segment_id,
            candidate.candidate_kind,
            candidate.confidence,
            json!(candidate.evidence_refs).to_string(),
            candidate.provenance.to_string(),
            json!(candidate.prompt_slot_ids).to_string(),
            job.llm_run_id,
            content_hash,
            safe_summary,
            body.to_string(),
            candidate.visibility,
            now
        ],
    )?;
    load_candidate(connection, &id)
}

fn insert_brief_candidate(
    connection: &Connection,
    job: &ConversationAnalysisJobView,
    message: &ConversationMessageView,
    evidence_refs: &[String],
    provenance: Value,
) -> Result<ConversationBriefCandidateView> {
    ensure!(
        !evidence_refs.is_empty(),
        "brief evidence refs are required"
    );
    let now = Utc::now().to_rfc3339();
    let id = format!("conversation_brief_candidate_{}", Uuid::new_v4());
    let excerpt = safe_excerpt(&message.body_markdown);
    let title = "Conversation brief candidate";
    let brief_markdown = format!(
        "What changed: {excerpt}\n\nEvidence: {}",
        evidence_refs.join(", ")
    );
    let content_hash = stable_hash(&brief_markdown);
    connection.execute(
        "INSERT INTO conversation_brief_candidates (
            id, job_id, conversation_id, segment_id, candidate_state, title, brief_markdown,
            evidence_refs_json, limitations_json, provenance_json, content_hash, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, 'proposed', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            id,
            job.id,
            job.conversation_id,
            job.segment_id,
            title,
            sanitize_public_text(&brief_markdown),
            json!(evidence_refs).to_string(),
            json!(["Deterministic local analysis; candidate requires review before product truth."]).to_string(),
            provenance.to_string(),
            content_hash,
            now
        ],
    )?;
    load_brief_candidate(connection, &id)
}

fn insert_memory_candidate(
    connection: &Connection,
    job: &ConversationAnalysisJobView,
    message: &ConversationMessageView,
    evidence_refs: &[String],
    provenance: Value,
) -> Result<ConversationMemoryCandidateView> {
    ensure!(
        !evidence_refs.is_empty(),
        "memory evidence refs are required"
    );
    let now = Utc::now().to_rfc3339();
    let id = format!("conversation_memory_candidate_{}", Uuid::new_v4());
    let summary = format!(
        "Possible relationship memory: {}",
        safe_excerpt(&message.body_markdown)
    );
    let body = sanitize_json(json!({
        "sourceMessageId": message.id,
        "promotion": "requires_approval",
        "text": summary,
    }));
    let safe_summary = sanitize_public_text(&summary);
    let content_hash = stable_hash(&format!("relationship_memory|{safe_summary}|{body}"));
    connection.execute(
        "INSERT INTO conversation_memory_candidates (
            id, job_id, conversation_id, segment_id, memory_kind, candidate_state,
            confidence, evidence_refs_json, provenance_json, content_hash, summary_text,
            body_json, visibility, approval_status, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, 'relationship_memory', 'proposed', 0.7, ?5, ?6, ?7, ?8, ?9, 'staff_private', 'requires_approval', ?10, ?10)",
        params![
            id,
            job.id,
            job.conversation_id,
            job.segment_id,
            json!(evidence_refs).to_string(),
            provenance.to_string(),
            content_hash,
            safe_summary,
            body.to_string(),
            now
        ],
    )?;
    load_memory_candidate(connection, &id)
}

fn load_completed_result(
    connection: &Connection,
    job: &ConversationAnalysisJobView,
) -> Result<ConversationAnalysisRunResult> {
    Ok(ConversationAnalysisRunResult {
        job: job.clone(),
        candidates: load_candidates_for_job(connection, &job.id)?,
        brief_candidate: load_latest_brief_for_job(connection, &job.id)?,
        memory_candidate: load_latest_memory_for_job(connection, &job.id)?,
        events: Vec::new(),
    })
}

fn load_job_for_message(
    connection: &Connection,
    conversation_id: &str,
    message_id: &str,
) -> Result<Option<ConversationAnalysisJobView>> {
    connection
        .query_row(
            "SELECT id FROM conversation_analysis_jobs
             WHERE conversation_id = ?1 AND analysis_kind = ?2 AND source_message_id = ?3",
            params![conversation_id, DEFAULT_ANALYSIS_KIND, message_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .map(|id| load_job(connection, &id))
        .transpose()
}

pub fn load_job(connection: &Connection, job_id: &str) -> Result<ConversationAnalysisJobView> {
    connection
        .query_row(
            "SELECT id, conversation_id, segment_id, analysis_kind, status, source_message_id,
                    source_event_cursor_start, source_event_cursor_end, input_refs_json, output_json,
                    policy_decision_id, llm_run_id, error_message_hash, created_at, started_at,
                    completed_at, updated_at
             FROM conversation_analysis_jobs
             WHERE id = ?1",
            [job_id],
            job_from_row,
        )
        .map_err(Into::into)
}

fn load_message_for_analysis(
    connection: &Connection,
    message_id: &str,
) -> Result<ConversationMessageView> {
    connection
        .query_row(
            "SELECT id, conversation_id, segment_id, participant_id, message_kind, status,
                    body_markdown, visibility, client_message_id, sequence, event_cursor,
                    undo_expires_at, undo_cancelled_at, created_at, edited_at, deleted_at
             FROM conversation_messages
             WHERE id = ?1",
            [message_id],
            |row| {
                Ok(ConversationMessageView {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    segment_id: row.get(2)?,
                    participant_id: row.get(3)?,
                    message_kind: row.get(4)?,
                    status: row.get(5)?,
                    body_markdown: row.get(6)?,
                    visibility: row.get(7)?,
                    client_message_id: row.get(8)?,
                    sequence: row.get(9)?,
                    event_cursor: row.get(10)?,
                    undo_expires_at: row.get(11)?,
                    undo_cancelled_at: row.get(12)?,
                    created_at: row.get(13)?,
                    edited_at: row.get(14)?,
                    deleted_at: row.get(15)?,
                })
            },
        )
        .map_err(Into::into)
}

fn load_candidate(connection: &Connection, id: &str) -> Result<ConversationAnalysisCandidateView> {
    connection
        .query_row(
            "SELECT id, job_id, conversation_id, segment_id, candidate_kind, candidate_state,
                    confidence, evidence_refs_json, provenance_json, prompt_slot_ids_json,
                    llm_run_id, content_hash, summary_text, body_json, visibility, created_at, updated_at
             FROM conversation_analysis_candidates
             WHERE id = ?1",
            [id],
            candidate_from_row,
        )
        .map_err(Into::into)
}

fn load_candidates_for_job(
    connection: &Connection,
    job_id: &str,
) -> Result<Vec<ConversationAnalysisCandidateView>> {
    let mut statement = connection.prepare(
        "SELECT id, job_id, conversation_id, segment_id, candidate_kind, candidate_state,
                confidence, evidence_refs_json, provenance_json, prompt_slot_ids_json,
                llm_run_id, content_hash, summary_text, body_json, visibility, created_at, updated_at
         FROM conversation_analysis_candidates
         WHERE job_id = ?1
         ORDER BY created_at ASC",
    )?;
    let rows = statement.query_map([job_id], candidate_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn load_brief_candidate(
    connection: &Connection,
    id: &str,
) -> Result<ConversationBriefCandidateView> {
    connection
        .query_row(
            "SELECT id, job_id, conversation_id, segment_id, candidate_state, title, brief_markdown,
                    evidence_refs_json, limitations_json, provenance_json, content_hash, created_at, updated_at
             FROM conversation_brief_candidates
             WHERE id = ?1",
            [id],
            brief_from_row,
        )
        .map_err(Into::into)
}

fn load_latest_brief_for_job(
    connection: &Connection,
    job_id: &str,
) -> Result<Option<ConversationBriefCandidateView>> {
    connection
        .query_row(
            "SELECT id, job_id, conversation_id, segment_id, candidate_state, title, brief_markdown,
                    evidence_refs_json, limitations_json, provenance_json, content_hash, created_at, updated_at
             FROM conversation_brief_candidates
             WHERE job_id = ?1
             ORDER BY created_at DESC
             LIMIT 1",
            [job_id],
            brief_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn load_memory_candidate(
    connection: &Connection,
    id: &str,
) -> Result<ConversationMemoryCandidateView> {
    connection
        .query_row(
            "SELECT id, job_id, conversation_id, segment_id, memory_kind, candidate_state,
                    confidence, evidence_refs_json, provenance_json, content_hash, summary_text,
                    body_json, visibility, approval_status, created_at, updated_at
             FROM conversation_memory_candidates
             WHERE id = ?1",
            [id],
            memory_from_row,
        )
        .map_err(Into::into)
}

fn load_latest_memory_for_job(
    connection: &Connection,
    job_id: &str,
) -> Result<Option<ConversationMemoryCandidateView>> {
    connection
        .query_row(
            "SELECT id, job_id, conversation_id, segment_id, memory_kind, candidate_state,
                    confidence, evidence_refs_json, provenance_json, content_hash, summary_text,
                    body_json, visibility, approval_status, created_at, updated_at
             FROM conversation_memory_candidates
             WHERE job_id = ?1
             ORDER BY created_at DESC
             LIMIT 1",
            [job_id],
            memory_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn update_conversation_summary(
    connection: &Connection,
    conversation_id: &str,
    brief: &ConversationBriefCandidateView,
    message: &ConversationMessageView,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE conversations
         SET summary_json = ?2,
             last_meaningful_change = 'conversation.analysis.completed',
             updated_at = ?3
         WHERE id = ?1",
        params![
            conversation_id,
            json!({
                "schemaVersion": CONVERSATION_ANALYSIS_SCHEMA_VERSION,
                "briefCandidateId": brief.id,
                "candidateState": brief.candidate_state,
                "latestEvidenceRef": evidence_ref(message),
                "summary": brief.brief_markdown,
                "updatedBy": "deterministic.local",
            })
            .to_string(),
            now
        ],
    )?;
    Ok(())
}

fn increment_action_count(connection: &Connection, conversation_id: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE conversations
         SET action_count = action_count + 1,
             last_meaningful_change = 'conversation.analysis.action_needed',
             updated_at = ?1
         WHERE id = ?2",
        params![now, conversation_id],
    )?;
    Ok(())
}

fn job_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConversationAnalysisJobView> {
    Ok(ConversationAnalysisJobView {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        segment_id: row.get(2)?,
        analysis_kind: row.get(3)?,
        status: row.get(4)?,
        source_message_id: row.get(5)?,
        source_event_cursor_start: row.get(6)?,
        source_event_cursor_end: row.get(7)?,
        input_refs: json_string_array(row.get(8)?),
        output: json_object(row.get(9)?),
        policy_decision_id: row.get(10)?,
        llm_run_id: row.get(11)?,
        error_message_hash: row.get(12)?,
        created_at: row.get(13)?,
        started_at: row.get(14)?,
        completed_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

fn candidate_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ConversationAnalysisCandidateView> {
    Ok(ConversationAnalysisCandidateView {
        id: row.get(0)?,
        job_id: row.get(1)?,
        conversation_id: row.get(2)?,
        segment_id: row.get(3)?,
        candidate_kind: row.get(4)?,
        candidate_state: row.get(5)?,
        confidence: row.get(6)?,
        evidence_refs: json_string_array(row.get(7)?),
        provenance: json_object(row.get(8)?),
        prompt_slot_ids: json_string_array(row.get(9)?),
        llm_run_id: row.get(10)?,
        content_hash: row.get(11)?,
        summary_text: row.get(12)?,
        body: json_object(row.get(13)?),
        visibility: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

fn brief_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConversationBriefCandidateView> {
    Ok(ConversationBriefCandidateView {
        id: row.get(0)?,
        job_id: row.get(1)?,
        conversation_id: row.get(2)?,
        segment_id: row.get(3)?,
        candidate_state: row.get(4)?,
        title: row.get(5)?,
        brief_markdown: row.get(6)?,
        evidence_refs: json_string_array(row.get(7)?),
        limitations: json_string_array(row.get(8)?),
        provenance: json_object(row.get(9)?),
        content_hash: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn memory_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ConversationMemoryCandidateView> {
    Ok(ConversationMemoryCandidateView {
        id: row.get(0)?,
        job_id: row.get(1)?,
        conversation_id: row.get(2)?,
        segment_id: row.get(3)?,
        memory_kind: row.get(4)?,
        candidate_state: row.get(5)?,
        confidence: row.get(6)?,
        evidence_refs: json_string_array(row.get(7)?),
        provenance: json_object(row.get(8)?),
        content_hash: row.get(9)?,
        summary_text: row.get(10)?,
        body: json_object(row.get(11)?),
        visibility: row.get(12)?,
        approval_status: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn is_message_eligible(message: &ConversationMessageView) -> bool {
    message.deleted_at.is_none()
        && message.status == "sent"
        && matches!(message.visibility.as_str(), "participants" | "public")
        && !matches!(message.message_kind.as_str(), "system" | "internal")
}

fn looks_action_needed(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("please")
        || lower.contains("need ")
        || lower.contains("can you")
        || lower.contains("could you")
        || lower.contains("follow up")
        || lower.contains("next step")
}

fn looks_handoff_needed(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("human")
        || lower.contains("person")
        || lower.contains("call me")
        || lower.contains("urgent")
        || lower.contains("sensitive")
}

fn evidence_ref(message: &ConversationMessageView) -> String {
    format!("conversation_message:{}", message.id)
}

fn safe_excerpt(text: &str) -> String {
    let sanitized = sanitize_public_text(text);
    let mut excerpt = sanitized.chars().take(180).collect::<String>();
    if sanitized.chars().count() > 180 {
        excerpt.push_str("...");
    }
    excerpt
}

fn sanitize_json(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(sanitize_public_text(&text)),
        Value::Array(values) => Value::Array(values.into_iter().map(sanitize_json).collect()),
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (key, sanitize_json(value)))
                .collect(),
        ),
        other => other,
    }
}

fn sanitize_public_text(text: &str) -> String {
    let mut sanitized = Vec::new();
    let mut skip_next_bearer = false;
    for token in text.split_whitespace() {
        if skip_next_bearer {
            sanitized.push("[REDACTED_TOKEN]".to_string());
            skip_next_bearer = false;
            continue;
        }
        if token.eq_ignore_ascii_case("bearer") {
            sanitized.push("Bearer".to_string());
            skip_next_bearer = true;
            continue;
        }
        let trimmed = token.trim_matches(|ch: char| {
            !ch.is_alphanumeric() && ch != '@' && ch != '-' && ch != '_' && ch != '.'
        });
        if looks_like_email(trimmed) {
            sanitized.push(token.replace(trimmed, "[REDACTED_EMAIL]"));
        } else if looks_like_api_key(trimmed) {
            sanitized.push(token.replace(trimmed, "[REDACTED_SECRET]"));
        } else {
            sanitized.push(token.to_string());
        }
    }
    sanitized.join(" ")
}

fn looks_like_email(token: &str) -> bool {
    token.contains('@') && token.contains('.') && token.len() >= 6
}

fn looks_like_api_key(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    (lower.starts_with("sk-") || lower.starts_with("tok_") || lower.starts_with("key_"))
        && token.len() >= 10
}

fn json_string_array(raw: String) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
}

fn json_object(raw: String) -> Value {
    serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}))
}

fn stable_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::conversations::{
        create_conversation_message, create_conversation_participant,
        find_or_create_canonical_conversation, CanonicalConversationRequest,
        ConversationMessageCreateRequest, ConversationParticipantCreateRequest,
    };
    use crate::schema::init_schema;

    fn test_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
                 VALUES ('actor_staff', 'staff', 'Staff', 'active', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO connections (
                    id, connection_type, display_name, status, identity_json, scope_json, metadata_json, created_at, updated_at
                 ) VALUES ('connection_1', 'client', 'Client', 'active', '{}', '{}', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
    }

    fn conversation_and_participant(connection: &Connection) -> (String, String) {
        let conversation = find_or_create_canonical_conversation(
            connection,
            &CanonicalConversationRequest {
                surface: "client_portal".to_string(),
                subject_kind: "connection".to_string(),
                subject_id: "connection_1".to_string(),
                connection_id: Some("connection_1".to_string()),
                visitor_session_id: None,
                created_by_actor_id: Some("actor_staff".to_string()),
            },
        )
        .unwrap();
        let participant = create_conversation_participant(
            connection,
            &ConversationParticipantCreateRequest {
                conversation_id: conversation.id.clone(),
                participant_kind: "connection".to_string(),
                actor_id: None,
                connection_id: Some("connection_1".to_string()),
                visitor_session_id: None,
                display_name: "Client".to_string(),
                role: "client".to_string(),
            },
        )
        .unwrap();
        (conversation.id, participant.id)
    }

    fn create_message(connection: &Connection, body: &str) -> ConversationMessageView {
        let (conversation_id, participant_id) = conversation_and_participant(connection);
        create_conversation_message(
            connection,
            &ConversationMessageCreateRequest {
                conversation_id,
                segment_id: None,
                participant_id,
                message_kind: "human".to_string(),
                body_markdown: body.to_string(),
                visibility: "participants".to_string(),
                client_message_id: format!("client_msg_{}", Uuid::new_v4()),
                reply_to_message_id: None,
                undo_expires_at: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn message_creation_queues_analysis_once_for_eligible_message() {
        let connection = test_connection();
        let message = create_message(&connection, "Can you please help with next steps?");

        let queued = queue_analysis_for_message(&connection, &message)
            .unwrap()
            .unwrap();
        let duplicate = queue_analysis_for_message(&connection, &message)
            .unwrap()
            .unwrap();

        assert_eq!(queued.id, duplicate.id);
        assert_eq!(queued.status, "queued");
        let job_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_analysis_jobs WHERE source_message_id = ?1",
                [&message.id],
                |row| row.get(0),
            )
            .unwrap();
        let queued_events: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_events WHERE event_type = 'conversation.analysis.queued'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(job_count, 1);
        assert_eq!(queued_events, 1);
    }

    #[test]
    fn analysis_lifecycle_creates_proposed_brief_memory_and_action_candidates() {
        let connection = test_connection();
        let message = create_message(
            &connection,
            "Can you please have a human call me about next steps?",
        );
        let job = queue_analysis_for_message(&connection, &message)
            .unwrap()
            .unwrap();

        let result = run_deterministic_analysis_for_job(&connection, &job.id).unwrap();

        assert_eq!(result.job.status, "completed");
        assert!(result.started_event_seen());
        assert!(result
            .candidates
            .iter()
            .any(|candidate| candidate.candidate_kind == "action_needed"));
        assert!(result
            .candidates
            .iter()
            .any(|candidate| candidate.candidate_kind == "handoff_signal"));
        assert!(result
            .candidates
            .iter()
            .all(|candidate| candidate.candidate_state == "proposed"
                && !candidate.evidence_refs.is_empty()
                && !candidate.provenance.as_object().unwrap().is_empty()
                && candidate.content_hash.starts_with("sha256:")));
        let brief = result.brief_candidate.unwrap();
        assert_eq!(brief.candidate_state, "proposed");
        assert!(brief
            .evidence_refs
            .contains(&format!("conversation_message:{}", message.id)));
        let memory = result.memory_candidate.unwrap();
        assert_eq!(memory.candidate_state, "proposed");
        assert_eq!(memory.approval_status, "requires_approval");
        let action_count: i64 = connection
            .query_row(
                "SELECT action_count FROM conversations WHERE id = ?1",
                [&message.conversation_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(action_count, 1);
    }

    #[test]
    fn repeated_analysis_for_same_job_is_idempotent() {
        let connection = test_connection();
        let message = create_message(&connection, "What happens next?");
        let job = queue_analysis_for_message(&connection, &message)
            .unwrap()
            .unwrap();

        let first = run_deterministic_analysis_for_job(&connection, &job.id).unwrap();
        let second = run_deterministic_analysis_for_job(&connection, &job.id).unwrap();

        assert_eq!(first.job.id, second.job.id);
        assert!(second.events.is_empty());
        let candidate_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM conversation_analysis_candidates WHERE job_id = ?1",
                [&job.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(candidate_count, first.candidates.len() as i64);
    }

    #[test]
    fn private_deleted_and_internal_messages_are_not_queued() {
        let connection = test_connection();
        let (conversation_id, participant_id) = conversation_and_participant(&connection);
        let private = create_conversation_message(
            &connection,
            &ConversationMessageCreateRequest {
                conversation_id,
                segment_id: None,
                participant_id,
                message_kind: "internal".to_string(),
                body_markdown: "Private note".to_string(),
                visibility: "private".to_string(),
                client_message_id: "private_msg".to_string(),
                reply_to_message_id: None,
                undo_expires_at: None,
            },
        )
        .unwrap();

        assert!(queue_analysis_for_message(&connection, &private)
            .unwrap()
            .is_none());
    }

    #[test]
    fn candidates_do_not_store_sensitive_fixture_text() {
        let connection = test_connection();
        let message = create_message(
            &connection,
            "Please remember ada@example.com has Bearer tok_abcdef123456 and key sk-test-123456.",
        );
        let job = queue_analysis_for_message(&connection, &message)
            .unwrap()
            .unwrap();

        run_deterministic_analysis_for_job(&connection, &job.id).unwrap();

        for raw in ["ada@example.com", "tok_abcdef123456", "sk-test-123456"] {
            for (table, columns) in [
                (
                    "conversation_analysis_candidates",
                    "summary_text || body_json || provenance_json",
                ),
                (
                    "conversation_brief_candidates",
                    "brief_markdown || provenance_json",
                ),
                (
                    "conversation_memory_candidates",
                    "summary_text || body_json || provenance_json",
                ),
            ] {
                let leaked_count: i64 = connection
                    .query_row(
                        &format!("SELECT COUNT(*) FROM {table} WHERE {columns} LIKE ?1"),
                        [format!("%{raw}%")],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(leaked_count, 0, "{table} leaked {raw}");
            }
        }
    }

    impl ConversationAnalysisRunResult {
        fn started_event_seen(&self) -> bool {
            self.events
                .iter()
                .any(|event| event.event_type == "conversation.analysis.started")
        }
    }
}
