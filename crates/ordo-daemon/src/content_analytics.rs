use anyhow::{ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::events::{append_realtime_event, system_event, RealtimeEvent};
use crate::schema::db::ConnectionExt;
use crate::security::redaction;

pub const CONTENT_ANALYTICS_SCHEMA_VERSION: &str = "content_analytics.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentAnalyticsEventKind {
    Generated,
    Approved,
    Published,
    Viewed,
    Clicked,
    Requested,
    TrialStarted,
    Referred,
    FeedbackSubmitted,
    OutcomeLinked,
}

impl ContentAnalyticsEventKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Generated => "generated",
            Self::Approved => "approved",
            Self::Published => "published",
            Self::Viewed => "viewed",
            Self::Clicked => "clicked",
            Self::Requested => "requested",
            Self::TrialStarted => "trial_started",
            Self::Referred => "referred",
            Self::FeedbackSubmitted => "feedback_submitted",
            Self::OutcomeLinked => "outcome_linked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentAnalyticsSourceStatus {
    Measured,
    Manual,
    Missing,
}

impl ContentAnalyticsSourceStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::Manual => "manual",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContentAnalyticsEventInput {
    pub event_kind: ContentAnalyticsEventKind,
    pub content_ref_kind: String,
    pub content_ref_id: String,
    pub content_version_id: Option<String>,
    pub artifact_id: Option<String>,
    pub artifact_version_id: Option<String>,
    pub surface: String,
    pub section_id: Option<String>,
    pub cta_id: Option<String>,
    pub workflow_template_id: Option<String>,
    pub workflow_compilation_id: Option<String>,
    pub job_id: Option<String>,
    pub tracked_entry_point_id: Option<String>,
    pub visitor_session_id: Option<String>,
    pub referral_id: Option<String>,
    pub outcome_id: Option<String>,
    pub source_kind: String,
    pub source_id: String,
    pub idempotency_key: String,
    pub source_status: ContentAnalyticsSourceStatus,
    pub visibility: String,
    pub evidence_refs: Vec<String>,
    pub limitation_labels: Vec<String>,
    pub payload: Value,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAnalyticsEventView {
    pub id: String,
    pub event_kind: String,
    pub content_ref_kind: String,
    pub content_ref_id: String,
    pub content_version_id: Option<String>,
    pub artifact_id: Option<String>,
    pub artifact_version_id: Option<String>,
    pub surface: String,
    pub section_id: Option<String>,
    pub cta_id: Option<String>,
    pub workflow_template_id: Option<String>,
    pub workflow_compilation_id: Option<String>,
    pub job_id: Option<String>,
    pub tracked_entry_point_id: Option<String>,
    pub visitor_session_id: Option<String>,
    pub referral_id: Option<String>,
    pub outcome_id: Option<String>,
    pub source_kind: String,
    pub source_id: String,
    pub idempotency_key: String,
    pub source_status: String,
    pub visibility: String,
    pub evidence_refs: Vec<String>,
    pub limitation_labels: Vec<String>,
    pub payload_hash: String,
    pub occurred_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAnalyticsSummary {
    pub schema_version: String,
    pub content_ref_kind: String,
    pub content_ref_id: String,
    pub surface: String,
    pub event_counts: Vec<ContentAnalyticsMetric>,
    pub source_status_counts: Vec<ContentAnalyticsMetric>,
    pub recent_events: Vec<ContentAnalyticsSummaryEvent>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<ContentAnalyticsLimitation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAnalyticsMetric {
    pub key: String,
    pub value: i64,
    pub source_status: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAnalyticsSummaryEvent {
    pub event_id: String,
    pub event_kind: String,
    pub source_status: String,
    pub surface: String,
    pub section_id: Option<String>,
    pub cta_id: Option<String>,
    pub occurred_at: String,
    pub evidence_refs: Vec<String>,
    pub limitation_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAnalyticsLimitation {
    pub key: String,
    pub label: String,
    pub source_status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicStoryContentAnalyticsRequest {
    pub event_kind: String,
    pub deck_id: String,
    pub deck_version: Option<i64>,
    pub section_id: Option<String>,
    pub cta_id: Option<String>,
    pub entry_point_slug: Option<String>,
    pub visitor_session_id: Option<String>,
    pub idempotency_key: String,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicStoryContentAnalyticsResponse {
    pub event: ContentAnalyticsEventView,
    pub context_state: String,
    pub limitations: Vec<String>,
}

pub fn record_content_analytics_event(
    connection: &Connection,
    input: ContentAnalyticsEventInput,
) -> Result<(ContentAnalyticsEventView, Option<RealtimeEvent>)> {
    validate_event_input(&input)?;
    let payload = safe_json(input.payload);
    let evidence_refs = safe_vec(input.evidence_refs);
    let limitation_labels = safe_vec(input.limitation_labels);
    let payload_hash = stable_hash(&format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        input.event_kind.as_str(),
        input.content_ref_kind.trim(),
        input.content_ref_id.trim(),
        input.content_version_id.as_deref().unwrap_or(""),
        input.artifact_id.as_deref().unwrap_or(""),
        input.artifact_version_id.as_deref().unwrap_or(""),
        input.surface.trim(),
        input.section_id.as_deref().unwrap_or(""),
        input.cta_id.as_deref().unwrap_or(""),
        input.workflow_template_id.as_deref().unwrap_or(""),
        input.workflow_compilation_id.as_deref().unwrap_or(""),
        input.job_id.as_deref().unwrap_or(""),
        input.tracked_entry_point_id.as_deref().unwrap_or(""),
        input.visitor_session_id.as_deref().unwrap_or(""),
        input.referral_id.as_deref().unwrap_or(""),
        input.outcome_id.as_deref().unwrap_or(""),
        input.source_status.as_str(),
        payload
    ));
    let id = stable_id(
        "content_analytics_event",
        &stable_hash(&format!(
            "{}|{}|{}",
            input.source_kind.trim(),
            input.source_id.trim(),
            input.idempotency_key.trim()
        )),
    );
    if let Some(existing) = load_content_analytics_event(connection, &id)? {
        ensure!(
            existing.payload_hash == payload_hash,
            "content analytics idempotency conflict for source tuple"
        );
        return Ok((existing, None));
    }

    let now = Utc::now().to_rfc3339();
    let occurred_at = input.occurred_at.unwrap_or_else(|| now.clone());
    connection.execute(
        "INSERT INTO content_analytics_events (
            id, event_kind, content_ref_kind, content_ref_id, content_version_id,
            artifact_id, artifact_version_id, surface, section_id, cta_id,
            workflow_template_id, workflow_compilation_id, job_id, tracked_entry_point_id,
            visitor_session_id, referral_id, outcome_id, source_kind, source_id,
            idempotency_key, source_status, visibility, evidence_refs_json,
            limitation_labels_json, payload_json, payload_hash, occurred_at, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                   ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)",
        params![
            id,
            input.event_kind.as_str(),
            safe_identifier(&input.content_ref_kind),
            safe_identifier(&input.content_ref_id),
            input
                .content_version_id
                .map(|value| safe_identifier(&value)),
            input.artifact_id.map(|value| safe_identifier(&value)),
            input
                .artifact_version_id
                .map(|value| safe_identifier(&value)),
            safe_identifier(&input.surface),
            input.section_id.map(|value| safe_identifier(&value)),
            input.cta_id.map(|value| safe_identifier(&value)),
            input
                .workflow_template_id
                .map(|value| safe_identifier(&value)),
            input
                .workflow_compilation_id
                .map(|value| safe_identifier(&value)),
            input.job_id.map(|value| safe_identifier(&value)),
            input
                .tracked_entry_point_id
                .map(|value| safe_identifier(&value)),
            input
                .visitor_session_id
                .map(|value| safe_identifier(&value)),
            input.referral_id.map(|value| safe_identifier(&value)),
            input.outcome_id.map(|value| safe_identifier(&value)),
            safe_identifier(&input.source_kind),
            safe_identifier(&input.source_id),
            safe_identifier(&input.idempotency_key),
            input.source_status.as_str(),
            normalize_visibility(&input.visibility),
            json!(evidence_refs).to_string(),
            json!(limitation_labels).to_string(),
            payload.to_string(),
            payload_hash,
            occurred_at,
            now,
        ],
    )?;
    let event = load_content_analytics_event(connection, &id)?
        .expect("content analytics event was just inserted");
    let realtime_event = append_realtime_event(
        connection,
        &system_event(
            "content_analytics.event_recorded",
            json!({
                "eventId": event.id,
                "eventKind": event.event_kind,
                "contentRefKind": event.content_ref_kind,
                "contentRefId": event.content_ref_id,
                "surface": event.surface,
                "sourceStatus": event.source_status,
                "evidenceRefs": event.evidence_refs,
                "limitationLabels": event.limitation_labels,
            }),
        ),
    )?;
    Ok((event, Some(realtime_event)))
}

pub fn record_public_story_content_analytics(
    db_path: &Path,
    request: PublicStoryContentAnalyticsRequest,
) -> Result<(PublicStoryContentAnalyticsResponse, Option<RealtimeEvent>)> {
    let connection = Connection::open(db_path)?;
    record_public_story_content_analytics_on_connection(&connection, request)
}

fn record_public_story_content_analytics_on_connection(
    connection: &Connection,
    request: PublicStoryContentAnalyticsRequest,
) -> Result<(PublicStoryContentAnalyticsResponse, Option<RealtimeEvent>)> {
    let event_kind = public_story_event_kind(&request.event_kind)?;
    let deck_id = require_public_identifier(&request.deck_id, "public story deck id")?;
    let deck_version = request.deck_version.unwrap_or(1).max(1);
    let section_id = normalize_optional_identifier(request.section_id, "public story section id")?;
    let cta_id = normalize_optional_identifier(request.cta_id, "public story CTA id")?;
    let entry_point_slug =
        normalize_optional_identifier(request.entry_point_slug, "tracked entry slug")?;
    let visitor_session_id =
        normalize_optional_identifier(request.visitor_session_id, "visitor session id")?;
    let idempotency_key =
        require_public_identifier(&request.idempotency_key, "public story idempotency key")?;

    ensure!(
        section_id.is_some() || event_kind != ContentAnalyticsEventKind::Viewed,
        "public story view events require section context"
    );
    ensure!(
        cta_id.is_some() || event_kind != ContentAnalyticsEventKind::Clicked,
        "public story click events require CTA context"
    );

    let has_context = entry_point_slug.is_some() || visitor_session_id.is_some();
    let source_status = if has_context {
        ContentAnalyticsSourceStatus::Measured
    } else {
        ContentAnalyticsSourceStatus::Missing
    };
    let source_id = visitor_session_id
        .as_deref()
        .or(entry_point_slug.as_deref())
        .unwrap_or("anonymous_public_story")
        .to_string();
    let mut evidence_refs = vec![format!("homepage_story_deck:{deck_id}")];
    if let Some(section_id) = &section_id {
        evidence_refs.push(format!("homepage_section:{section_id}"));
    }
    if let Some(cta_id) = &cta_id {
        evidence_refs.push(format!("homepage_cta:{cta_id}"));
    }
    if let Some(entry_point_slug) = &entry_point_slug {
        evidence_refs.push(format!("tracked_entry_point_slug:{entry_point_slug}"));
    }
    if let Some(visitor_session_id) = &visitor_session_id {
        evidence_refs.push(format!("visitor_session:{visitor_session_id}"));
    }
    let limitations = if has_context {
        Vec::new()
    } else {
        vec!["missing_visitor_or_tracked_entry_context".to_string()]
    };
    let context_state = if has_context { "measured" } else { "missing" }.to_string();

    let (event, realtime_event) = record_content_analytics_event(
        connection,
        ContentAnalyticsEventInput {
            event_kind,
            content_ref_kind: "homepage_story_deck".to_string(),
            content_ref_id: deck_id.clone(),
            content_version_id: Some(format!("{deck_id}:v{deck_version}")),
            artifact_id: None,
            artifact_version_id: None,
            surface: "public_story".to_string(),
            section_id,
            cta_id,
            workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
            workflow_compilation_id: None,
            job_id: None,
            tracked_entry_point_id: None,
            visitor_session_id,
            referral_id: None,
            outcome_id: None,
            source_kind: "public_story_runtime".to_string(),
            source_id,
            idempotency_key,
            source_status,
            visibility: "staff".to_string(),
            evidence_refs,
            limitation_labels: limitations.clone(),
            payload: json!({
                "localEvent": true,
                "contextState": context_state,
                "entryPointSlug": entry_point_slug,
                "externalAnalytics": "not_called",
                "cookieTracking": "not_used",
            }),
            occurred_at: request.occurred_at,
        },
    )?;

    Ok((
        PublicStoryContentAnalyticsResponse {
            event,
            context_state,
            limitations,
        },
        realtime_event,
    ))
}

pub fn summarize_content_analytics_for_content(
    connection: &Connection,
    content_ref_kind: &str,
    content_ref_id: &str,
) -> Result<ContentAnalyticsSummary> {
    let events = list_content_analytics_events_for_content(
        connection,
        &safe_identifier(content_ref_kind),
        &safe_identifier(content_ref_id),
    )?;
    let surface = events
        .first()
        .map(|event| event.surface.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let mut event_counts = count_metrics(
        &events,
        |event| event.event_kind.clone(),
        |event| event.source_status.clone(),
    );
    event_counts.sort_by(|left, right| left.key.cmp(&right.key));
    let mut source_status_counts = count_metrics(
        &events,
        |event| event.source_status.clone(),
        |event| event.source_status.clone(),
    );
    source_status_counts.sort_by(|left, right| left.key.cmp(&right.key));

    let evidence_refs = stable_unique(events.iter().flat_map(|event| event.evidence_refs.clone()));
    let mut limitations = stable_unique(
        events
            .iter()
            .flat_map(|event| event.limitation_labels.clone()),
    )
    .into_iter()
    .map(|label| ContentAnalyticsLimitation {
        key: label.clone(),
        label,
        source_status: "missing".to_string(),
    })
    .collect::<Vec<_>>();
    if events
        .iter()
        .any(|event| event.source_status == ContentAnalyticsSourceStatus::Missing.as_str())
        && !limitations
            .iter()
            .any(|limitation| limitation.key == "external_analytics_missing")
    {
        limitations.push(ContentAnalyticsLimitation {
            key: "external_analytics_missing".to_string(),
            label: "External analytics are missing; no platform metrics are inferred.".to_string(),
            source_status: "missing".to_string(),
        });
    }

    let recent_events = events
        .iter()
        .take(10)
        .map(|event| ContentAnalyticsSummaryEvent {
            event_id: event.id.clone(),
            event_kind: event.event_kind.clone(),
            source_status: event.source_status.clone(),
            surface: event.surface.clone(),
            section_id: event.section_id.clone(),
            cta_id: event.cta_id.clone(),
            occurred_at: event.occurred_at.clone(),
            evidence_refs: event.evidence_refs.clone(),
            limitation_labels: event.limitation_labels.clone(),
        })
        .collect();

    Ok(ContentAnalyticsSummary {
        schema_version: CONTENT_ANALYTICS_SCHEMA_VERSION.to_string(),
        content_ref_kind: safe_identifier(content_ref_kind),
        content_ref_id: safe_identifier(content_ref_id),
        surface,
        event_counts,
        source_status_counts,
        recent_events,
        evidence_refs,
        limitations,
    })
}

pub fn list_content_analytics_events_for_content(
    connection: &Connection,
    content_ref_kind: &str,
    content_ref_id: &str,
) -> Result<Vec<ContentAnalyticsEventView>> {
    connection.query_many(
        "SELECT id, event_kind, content_ref_kind, content_ref_id, content_version_id,
                artifact_id, artifact_version_id, surface, section_id, cta_id,
                workflow_template_id, workflow_compilation_id, job_id, tracked_entry_point_id,
                visitor_session_id, referral_id, outcome_id, source_kind, source_id,
                idempotency_key, source_status, visibility, evidence_refs_json,
                limitation_labels_json, payload_hash, occurred_at, created_at
         FROM content_analytics_events
         WHERE content_ref_kind = ?1 AND content_ref_id = ?2
         ORDER BY occurred_at DESC, id ASC",
        params![content_ref_kind, content_ref_id],
        content_analytics_event_from_row,
    )
}

fn load_content_analytics_event(
    connection: &Connection,
    event_id: &str,
) -> Result<Option<ContentAnalyticsEventView>> {
    connection
        .query_row(
            "SELECT id, event_kind, content_ref_kind, content_ref_id, content_version_id,
                    artifact_id, artifact_version_id, surface, section_id, cta_id,
                    workflow_template_id, workflow_compilation_id, job_id, tracked_entry_point_id,
                    visitor_session_id, referral_id, outcome_id, source_kind, source_id,
                    idempotency_key, source_status, visibility, evidence_refs_json,
                    limitation_labels_json, payload_hash, occurred_at, created_at
             FROM content_analytics_events
             WHERE id = ?1",
            [event_id],
            content_analytics_event_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn validate_event_input(input: &ContentAnalyticsEventInput) -> Result<()> {
    ensure!(
        !input.content_ref_kind.trim().is_empty() && !input.content_ref_id.trim().is_empty(),
        "content analytics requires a content reference"
    );
    ensure!(
        !input.surface.trim().is_empty(),
        "content analytics requires a surface"
    );
    ensure!(
        !input.source_kind.trim().is_empty()
            && !input.source_id.trim().is_empty()
            && !input.idempotency_key.trim().is_empty(),
        "content analytics requires a source idempotency tuple"
    );
    let safe_evidence_refs = safe_vec(input.evidence_refs.clone());
    let safe_limitation_labels = safe_vec(input.limitation_labels.clone());
    ensure!(
        !safe_evidence_refs.is_empty(),
        "content analytics requires safe evidence refs"
    );
    if input.source_status != ContentAnalyticsSourceStatus::Measured {
        ensure!(
            !safe_limitation_labels.is_empty(),
            "manual or missing content analytics evidence requires safe limitation labels"
        );
    }
    if matches!(
        input.event_kind,
        ContentAnalyticsEventKind::Clicked | ContentAnalyticsEventKind::Requested
    ) {
        ensure!(
            input.section_id.is_some() || input.cta_id.is_some(),
            "content engagement events require section or CTA context"
        );
    }
    if input.event_kind == ContentAnalyticsEventKind::OutcomeLinked {
        ensure!(
            input.outcome_id.is_some(),
            "outcome-linked content analytics events require an outcome id"
        );
    }
    ensure_safe_text(&input.content_ref_kind)?;
    ensure_safe_text(&input.content_ref_id)?;
    ensure_safe_text(&input.surface)?;
    ensure_safe_text(&input.source_kind)?;
    ensure_safe_text(&input.source_id)?;
    ensure_safe_text(&input.idempotency_key)?;
    for value in [
        &input.content_version_id,
        &input.artifact_id,
        &input.artifact_version_id,
        &input.section_id,
        &input.cta_id,
        &input.workflow_template_id,
        &input.workflow_compilation_id,
        &input.job_id,
        &input.tracked_entry_point_id,
        &input.visitor_session_id,
        &input.referral_id,
        &input.outcome_id,
    ]
    .into_iter()
    .flatten()
    {
        ensure_safe_text(value)?;
    }
    for value in input
        .evidence_refs
        .iter()
        .chain(input.limitation_labels.iter())
    {
        ensure_safe_text(value)?;
    }
    ensure_safe_text(&input.payload.to_string())?;
    Ok(())
}

fn public_story_event_kind(value: &str) -> Result<ContentAnalyticsEventKind> {
    match value.trim() {
        "viewed" => Ok(ContentAnalyticsEventKind::Viewed),
        "clicked" => Ok(ContentAnalyticsEventKind::Clicked),
        _ => anyhow::bail!("unsupported public story analytics event kind"),
    }
}

fn require_public_identifier(value: &str, label: &str) -> Result<String> {
    ensure_safe_text(value)?;
    let value = safe_identifier(value);
    ensure!(!value.is_empty(), "{label} is required");
    ensure_safe_text(&value)?;
    Ok(value)
}

fn normalize_optional_identifier(value: Option<String>, label: &str) -> Result<Option<String>> {
    value
        .map(|value| {
            ensure_safe_text(&value)?;
            let value = safe_identifier(&value);
            ensure!(!value.is_empty(), "{label} cannot be empty");
            ensure_safe_text(&value)?;
            Ok(value)
        })
        .transpose()
}

fn content_analytics_event_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ContentAnalyticsEventView> {
    let evidence_json: String = row.get(22)?;
    let limitation_json: String = row.get(23)?;
    Ok(ContentAnalyticsEventView {
        id: row.get(0)?,
        event_kind: row.get(1)?,
        content_ref_kind: row.get(2)?,
        content_ref_id: row.get(3)?,
        content_version_id: row.get(4)?,
        artifact_id: row.get(5)?,
        artifact_version_id: row.get(6)?,
        surface: row.get(7)?,
        section_id: row.get(8)?,
        cta_id: row.get(9)?,
        workflow_template_id: row.get(10)?,
        workflow_compilation_id: row.get(11)?,
        job_id: row.get(12)?,
        tracked_entry_point_id: row.get(13)?,
        visitor_session_id: row.get(14)?,
        referral_id: row.get(15)?,
        outcome_id: row.get(16)?,
        source_kind: row.get(17)?,
        source_id: row.get(18)?,
        idempotency_key: row.get(19)?,
        source_status: row.get(20)?,
        visibility: row.get(21)?,
        evidence_refs: serde_json::from_str(&evidence_json).unwrap_or_default(),
        limitation_labels: serde_json::from_str(&limitation_json).unwrap_or_default(),
        payload_hash: row.get(24)?,
        occurred_at: row.get(25)?,
        created_at: row.get(26)?,
    })
}

fn count_metrics(
    events: &[ContentAnalyticsEventView],
    key: impl Fn(&ContentAnalyticsEventView) -> String,
    status: impl Fn(&ContentAnalyticsEventView) -> String,
) -> Vec<ContentAnalyticsMetric> {
    let mut metrics = std::collections::BTreeMap::<String, ContentAnalyticsMetric>::new();
    for event in events {
        let key = key(event);
        let entry = metrics
            .entry(key.clone())
            .or_insert(ContentAnalyticsMetric {
                key,
                value: 0,
                source_status: status(event),
                evidence_refs: Vec::new(),
            });
        entry.value += 1;
        append_unique(&mut entry.evidence_refs, &event.evidence_refs);
    }
    metrics.into_values().collect()
}

fn ensure_safe_text(text: &str) -> Result<()> {
    let lower = text.to_ascii_lowercase();
    let blocked = [
        "prompt internal",
        "provider internal",
        "provider payload",
        "raw policy",
        "policy internal",
        "owner-only",
        "private artifact text",
        "task private payload",
        "staff routing",
        "graph certainty",
        "unsupported claim",
        "fake analytics",
        "fake publishing",
        "fake uptime",
        "fake conversion",
        "fake provider",
        "secret",
        "api_key",
        "password",
        "bearer ",
    ];
    ensure!(
        !blocked.iter().any(|needle| lower.contains(needle)),
        "content analytics contains private/internal or unsupported claim text"
    );
    ensure!(
        !redaction::contains_sensitive_text(text, &[]),
        "content analytics contains sensitive text"
    );
    Ok(())
}

fn safe_text(text: &str) -> String {
    redaction::redact_public_text(text.trim())
}

fn safe_json(value: Value) -> Value {
    redaction::sanitize_json_strings(value)
}

fn safe_vec(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| safe_text(&value))
        .filter(|value| !value.trim().is_empty())
        .collect()
}

fn safe_identifier(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | ':' | '.' | '/')
            {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
}

fn normalize_visibility(value: &str) -> String {
    match value.trim() {
        "public" => "public",
        "authenticated" | "member" => "authenticated",
        "staff" | "staff_private" => "staff",
        "owner" | "owner_private" => "owner",
        _ => "staff",
    }
    .to_string()
}

fn append_unique(target: &mut Vec<String>, refs: &[String]) {
    for value in refs {
        let safe = safe_identifier(value);
        if !safe.trim().is_empty() && !target.contains(&safe) {
            target.push(safe);
        }
    }
}

fn stable_unique(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut result = Vec::new();
    for value in values {
        let safe = safe_identifier(&value);
        if !safe.trim().is_empty() && !result.contains(&safe) {
            result.push(safe);
        }
    }
    result
}

fn stable_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn stable_id(prefix: &str, content_hash: &str) -> String {
    let suffix = content_hash
        .strip_prefix("sha256:")
        .unwrap_or(content_hash)
        .chars()
        .take(24)
        .collect::<String>();
    format!("{prefix}_{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;

    fn input(kind: ContentAnalyticsEventKind, idempotency_key: &str) -> ContentAnalyticsEventInput {
        ContentAnalyticsEventInput {
            event_kind: kind,
            content_ref_kind: "homepage_story_deck".to_string(),
            content_ref_id: "homepage_story_deck_v1".to_string(),
            content_version_id: Some("homepage_story_deck_v1".to_string()),
            artifact_id: Some("artifact_story_v1".to_string()),
            artifact_version_id: Some("artifact_version_story_v1".to_string()),
            surface: "public_story".to_string(),
            section_id: Some("scene_1".to_string()),
            cta_id: None,
            workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
            workflow_compilation_id: Some("workflow_compilation_story_v1".to_string()),
            job_id: Some("job_story_v1".to_string()),
            tracked_entry_point_id: Some("entry_point_qr_1".to_string()),
            visitor_session_id: Some("visitor_session_1".to_string()),
            referral_id: None,
            outcome_id: None,
            source_kind: "public_story_runtime".to_string(),
            source_id: "runtime_local_fixture".to_string(),
            idempotency_key: idempotency_key.to_string(),
            source_status: ContentAnalyticsSourceStatus::Measured,
            visibility: "staff".to_string(),
            evidence_refs: vec!["artifact:artifact_story_v1".to_string()],
            limitation_labels: vec![],
            payload: json!({
                "localEvent": true,
                "platformAnalytics": "not_called",
            }),
            occurred_at: Some("2026-05-14T20:00:00Z".to_string()),
        }
    }

    #[test]
    fn records_public_story_events_and_safe_summary_without_fake_metrics() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        let published = record_content_analytics_event(
            &connection,
            ContentAnalyticsEventInput {
                event_kind: ContentAnalyticsEventKind::Published,
                source_kind: "manual_publish".to_string(),
                source_id: "owner_preview".to_string(),
                idempotency_key: "publish-homepage-v1".to_string(),
                evidence_refs: vec!["publication:homepage_v1".to_string()],
                payload: json!({"publicationMode": "manual"}),
                ..input(ContentAnalyticsEventKind::Published, "publish-homepage-v1")
            },
        )
        .unwrap();
        assert!(published.1.is_some());

        let clicked = record_content_analytics_event(
            &connection,
            ContentAnalyticsEventInput {
                event_kind: ContentAnalyticsEventKind::Clicked,
                cta_id: Some("start_trial".to_string()),
                idempotency_key: "click-start-trial-1".to_string(),
                evidence_refs: vec![
                    "visitor_session:visitor_session_1".to_string(),
                    "entry_point:entry_point_qr_1".to_string(),
                ],
                payload: json!({"target": "offer_30_day_trial"}),
                ..input(ContentAnalyticsEventKind::Clicked, "click-start-trial-1")
            },
        )
        .unwrap();
        assert_eq!(clicked.0.event_kind, "clicked");

        record_content_analytics_event(
            &connection,
            ContentAnalyticsEventInput {
                event_kind: ContentAnalyticsEventKind::TrialStarted,
                idempotency_key: "trial-started-1".to_string(),
                evidence_refs: vec!["trial:trial_1".to_string()],
                payload: json!({"trialId": "trial_1"}),
                ..input(ContentAnalyticsEventKind::TrialStarted, "trial-started-1")
            },
        )
        .unwrap();
        record_content_analytics_event(
            &connection,
            ContentAnalyticsEventInput {
                event_kind: ContentAnalyticsEventKind::Referred,
                referral_id: Some("referral_1".to_string()),
                idempotency_key: "referral-1".to_string(),
                evidence_refs: vec!["referral:referral_1".to_string()],
                payload: json!({"referralId": "referral_1"}),
                ..input(ContentAnalyticsEventKind::Referred, "referral-1")
            },
        )
        .unwrap();
        record_content_analytics_event(
            &connection,
            ContentAnalyticsEventInput {
                event_kind: ContentAnalyticsEventKind::OutcomeLinked,
                outcome_id: Some("business_outcome_1".to_string()),
                idempotency_key: "outcome-1".to_string(),
                evidence_refs: vec!["business_outcome:business_outcome_1".to_string()],
                payload: json!({"outcomeKind": "trial_started"}),
                ..input(ContentAnalyticsEventKind::OutcomeLinked, "outcome-1")
            },
        )
        .unwrap();
        record_content_analytics_event(
            &connection,
            ContentAnalyticsEventInput {
                event_kind: ContentAnalyticsEventKind::Viewed,
                source_kind: "external_platform".to_string(),
                source_id: "instagram".to_string(),
                idempotency_key: "missing-instagram-views".to_string(),
                source_status: ContentAnalyticsSourceStatus::Missing,
                evidence_refs: vec!["limitation:external_analytics_missing".to_string()],
                limitation_labels: vec!["external_analytics_missing".to_string()],
                payload: json!({"externalPlatform": "instagram", "metricStatus": "missing"}),
                ..input(ContentAnalyticsEventKind::Viewed, "missing-instagram-views")
            },
        )
        .unwrap();

        let summary = summarize_content_analytics_for_content(
            &connection,
            "homepage_story_deck",
            "homepage_story_deck_v1",
        )
        .unwrap();
        assert_eq!(summary.schema_version, CONTENT_ANALYTICS_SCHEMA_VERSION);
        assert!(summary
            .event_counts
            .iter()
            .any(|metric| metric.key == "published" && metric.value == 1));
        assert!(summary
            .event_counts
            .iter()
            .any(|metric| metric.key == "clicked" && metric.value == 1));
        assert!(summary
            .event_counts
            .iter()
            .any(|metric| metric.key == "trial_started" && metric.value == 1));
        assert!(summary
            .source_status_counts
            .iter()
            .any(|metric| metric.key == "missing" && metric.value == 1));
        assert!(summary
            .limitations
            .iter()
            .any(|limitation| limitation.key == "external_analytics_missing"));
        let summary_json = serde_json::to_string(&summary).unwrap();
        assert!(!summary_json.contains("publicationMode"));
        assert!(!summary_json.contains("trialId"));
        assert!(!summary_json.contains("fake"));
    }

    #[test]
    fn rejects_private_internal_unsupported_and_fake_analytics_text_without_mutation() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let mut unsafe_input = input(ContentAnalyticsEventKind::Viewed, "unsafe");
        unsafe_input.payload = json!({
            "claim": "fake analytics from provider internal prompt internals with sk-live-secret",
        });

        let error = record_content_analytics_event(&connection, unsafe_input).unwrap_err();
        assert!(error
            .to_string()
            .contains("private/internal or unsupported claim"));
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM content_analytics_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn rejects_unsafe_optional_public_refs_and_empty_sanitized_evidence() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        let mut unsafe_optional = input(ContentAnalyticsEventKind::Clicked, "unsafe-optional");
        unsafe_optional.cta_id = Some("staff routing owner-only CTA".to_string());
        let error = record_content_analytics_event(&connection, unsafe_optional).unwrap_err();
        assert!(error
            .to_string()
            .contains("private/internal or unsupported claim"));

        let mut blank_evidence = input(ContentAnalyticsEventKind::Viewed, "blank-evidence");
        blank_evidence.evidence_refs = vec!["   ".to_string()];
        let error = record_content_analytics_event(&connection, blank_evidence).unwrap_err();
        assert!(error.to_string().contains("requires safe evidence refs"));

        let mut blank_limitation = input(ContentAnalyticsEventKind::Viewed, "blank-limitation");
        blank_limitation.source_status = ContentAnalyticsSourceStatus::Missing;
        blank_limitation.evidence_refs = vec!["limitation:external_analytics_missing".to_string()];
        blank_limitation.limitation_labels = vec!["   ".to_string()];
        let error = record_content_analytics_event(&connection, blank_limitation).unwrap_err();
        assert!(error
            .to_string()
            .contains("requires safe limitation labels"));

        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM content_analytics_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn idempotent_retries_are_stable_and_conflicts_reject_without_inflating_metrics() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let first = record_content_analytics_event(
            &connection,
            input(ContentAnalyticsEventKind::Viewed, "view-1"),
        )
        .unwrap();
        let second = record_content_analytics_event(
            &connection,
            input(ContentAnalyticsEventKind::Viewed, "view-1"),
        )
        .unwrap();
        assert_eq!(first.0.id, second.0.id);
        assert!(first.1.is_some());
        assert!(second.1.is_none());

        let mut conflicting = input(ContentAnalyticsEventKind::Viewed, "view-1");
        conflicting.payload = json!({"localEvent": true, "changed": true});
        let error = record_content_analytics_event(&connection, conflicting).unwrap_err();
        assert!(error.to_string().contains("idempotency conflict"));

        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM content_analytics_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn manual_and_missing_events_require_limitation_labels_and_outcome_context() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let mut manual = input(ContentAnalyticsEventKind::OutcomeLinked, "manual-outcome");
        manual.source_status = ContentAnalyticsSourceStatus::Manual;
        manual.outcome_id = Some("business_outcome_manual".to_string());
        manual.evidence_refs = vec!["business_outcome:business_outcome_manual".to_string()];
        manual.limitation_labels = vec!["manual_offline_evidence".to_string()];
        manual.payload = json!({"source": "owner_entered_offline_result"});

        let event = record_content_analytics_event(&connection, manual)
            .unwrap()
            .0;
        assert_eq!(event.source_status, "manual");
        assert!(event
            .limitation_labels
            .contains(&"manual_offline_evidence".to_string()));

        let mut missing = input(ContentAnalyticsEventKind::Viewed, "missing-no-limit");
        missing.source_status = ContentAnalyticsSourceStatus::Missing;
        missing.limitation_labels = vec![];
        let error = record_content_analytics_event(&connection, missing).unwrap_err();
        assert!(error
            .to_string()
            .contains("requires safe limitation labels"));

        let mut outcome_missing =
            input(ContentAnalyticsEventKind::OutcomeLinked, "missing-outcome");
        outcome_missing.event_kind = ContentAnalyticsEventKind::OutcomeLinked;
        outcome_missing.outcome_id = None;
        let error = record_content_analytics_event(&connection, outcome_missing).unwrap_err();
        assert!(
            error.to_string().contains("require an outcome id"),
            "{error}"
        );
    }

    #[test]
    fn public_story_ingestion_records_bounded_view_and_click_events() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        let viewed = record_public_story_content_analytics_on_connection(
            &connection,
            PublicStoryContentAnalyticsRequest {
                event_kind: "viewed".to_string(),
                deck_id: "homepage.story.v1".to_string(),
                deck_version: Some(2),
                section_id: Some("identity".to_string()),
                cta_id: None,
                entry_point_slug: Some("nyc-pilot".to_string()),
                visitor_session_id: Some("visitor_session_1".to_string()),
                idempotency_key: "homepage.story.v1:2:viewed:identity:visitor_session_1"
                    .to_string(),
                occurred_at: Some("2026-05-14T23:40:00Z".to_string()),
            },
        )
        .unwrap();
        assert_eq!(viewed.0.event.event_kind, "viewed");
        assert_eq!(viewed.0.event.source_status, "measured");
        assert_eq!(viewed.0.context_state, "measured");
        assert!(viewed.0.limitations.is_empty());
        assert!(viewed
            .0
            .event
            .evidence_refs
            .contains(&"visitor_session:visitor_session_1".to_string()));

        let repeated = record_public_story_content_analytics_on_connection(
            &connection,
            PublicStoryContentAnalyticsRequest {
                event_kind: "viewed".to_string(),
                deck_id: "homepage.story.v1".to_string(),
                deck_version: Some(2),
                section_id: Some("identity".to_string()),
                cta_id: None,
                entry_point_slug: Some("nyc-pilot".to_string()),
                visitor_session_id: Some("visitor_session_1".to_string()),
                idempotency_key: "homepage.story.v1:2:viewed:identity:visitor_session_1"
                    .to_string(),
                occurred_at: Some("2026-05-14T23:40:00Z".to_string()),
            },
        )
        .unwrap();
        assert_eq!(viewed.0.event.id, repeated.0.event.id);
        assert!(viewed.1.is_some());
        assert!(repeated.1.is_none());

        let clicked = record_public_story_content_analytics_on_connection(
            &connection,
            PublicStoryContentAnalyticsRequest {
                event_kind: "clicked".to_string(),
                deck_id: "homepage.story.v1".to_string(),
                deck_version: Some(2),
                section_id: Some("identity".to_string()),
                cta_id: Some("talk-with-ordo".to_string()),
                entry_point_slug: Some("nyc-pilot".to_string()),
                visitor_session_id: Some("visitor_session_1".to_string()),
                idempotency_key: "homepage.story.v1:2:clicked:talk-with-ordo:visitor_session_1"
                    .to_string(),
                occurred_at: Some("2026-05-14T23:41:00Z".to_string()),
            },
        )
        .unwrap();
        assert_eq!(clicked.0.event.event_kind, "clicked");
        assert_eq!(clicked.0.event.cta_id.as_deref(), Some("talk-with-ordo"));

        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM content_analytics_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn public_story_ingestion_handles_missing_context_and_rejects_unsafe_payloads() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        let missing_context = record_public_story_content_analytics_on_connection(
            &connection,
            PublicStoryContentAnalyticsRequest {
                event_kind: "viewed".to_string(),
                deck_id: "homepage.story.v1".to_string(),
                deck_version: Some(1),
                section_id: Some("identity".to_string()),
                cta_id: None,
                entry_point_slug: None,
                visitor_session_id: None,
                idempotency_key: "homepage.story.v1:1:viewed:identity:anonymous".to_string(),
                occurred_at: None,
            },
        )
        .unwrap();
        assert_eq!(missing_context.0.event.source_status, "missing");
        assert_eq!(missing_context.0.context_state, "missing");
        assert!(missing_context
            .0
            .event
            .limitation_labels
            .contains(&"missing_visitor_or_tracked_entry_context".to_string()));

        let unsupported = record_public_story_content_analytics_on_connection(
            &connection,
            PublicStoryContentAnalyticsRequest {
                event_kind: "requested".to_string(),
                deck_id: "homepage.story.v1".to_string(),
                deck_version: Some(1),
                section_id: Some("identity".to_string()),
                cta_id: None,
                entry_point_slug: None,
                visitor_session_id: None,
                idempotency_key: "unsupported".to_string(),
                occurred_at: None,
            },
        )
        .unwrap_err();
        assert!(unsupported.to_string().contains("unsupported public story"));

        let unsafe_ref = record_public_story_content_analytics_on_connection(
            &connection,
            PublicStoryContentAnalyticsRequest {
                event_kind: "clicked".to_string(),
                deck_id: "homepage.story.v1".to_string(),
                deck_version: Some(1),
                section_id: Some("identity".to_string()),
                cta_id: Some("provider internal prompt internals".to_string()),
                entry_point_slug: None,
                visitor_session_id: None,
                idempotency_key: "unsafe-click".to_string(),
                occurred_at: None,
            },
        )
        .unwrap_err();
        assert!(unsafe_ref
            .to_string()
            .contains("private/internal or unsupported claim"));

        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM content_analytics_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }
}
