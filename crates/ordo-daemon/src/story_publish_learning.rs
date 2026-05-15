use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::artifacts::{load_artifact, ArtifactView};
use crate::content_analytics::{
    list_content_analytics_events_for_content, summarize_content_analytics_for_content,
    ContentAnalyticsEventView, ContentAnalyticsSummary,
};
use crate::generated_content_memory::{
    generated_content_memory_review_packet_for_artifact, GeneratedContentMemoryReviewAudience,
    GeneratedContentMemoryReviewPacket,
};
use crate::security::redaction;
use crate::story_publish_approvals::STORY_HOMEPAGE_PUBLISH_APPROVAL_PACKAGE_ARTIFACT_KIND;

pub const STORY_PUBLISH_LEARNING_BRIEF_SCHEMA_VERSION: &str =
    "ordo.story_publish_learning_brief.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoryPublishLearningAudience {
    Staff,
    Owner,
    Member,
    Public,
}

impl StoryPublishLearningAudience {
    fn as_str(self) -> &'static str {
        match self {
            Self::Staff => "staff",
            Self::Owner => "owner",
            Self::Member => "member",
            Self::Public => "public",
        }
    }

    fn can_read_learning_evidence(self) -> bool {
        matches!(self, Self::Staff | Self::Owner)
    }

    fn memory_audience(self) -> GeneratedContentMemoryReviewAudience {
        match self {
            Self::Staff => GeneratedContentMemoryReviewAudience::Staff,
            Self::Owner => GeneratedContentMemoryReviewAudience::Owner,
            Self::Member => GeneratedContentMemoryReviewAudience::Member,
            Self::Public => GeneratedContentMemoryReviewAudience::Public,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StoryPublishLearningBriefRequest {
    pub audience: StoryPublishLearningAudience,
    pub deck_id: String,
    pub artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryPublishLearningBrief {
    pub schema_version: String,
    pub status: String,
    pub audience: String,
    pub deck_id: String,
    pub read_only: bool,
    pub mutation_performed: bool,
    pub confirmed_graph_promotion: bool,
    pub memory_promotion_performed: bool,
    pub live_provider_called: bool,
    pub external_publishing_claimed: bool,
    pub source_status: Vec<StoryPublishLearningMetric>,
    pub content_metrics: Vec<StoryPublishLearningMetric>,
    pub publish_evidence: Vec<StoryPublishLearningSource>,
    pub memory_summary: StoryPublishMemoryLearningSummary,
    pub outcome_summary: StoryPublishOutcomeLearningSummary,
    pub reward_summary: StoryPublishRewardLearningSummary,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub recommended_next_actions: Vec<String>,
    pub analytics_summary: Option<ContentAnalyticsSummary>,
    pub memory_review_packets: Vec<GeneratedContentMemoryReviewPacket>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryPublishLearningMetric {
    pub key: String,
    pub label: String,
    pub value: i64,
    pub source_status: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryPublishLearningSource {
    pub source_kind: String,
    pub source_id: String,
    pub status: String,
    pub source_status: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryPublishMemoryLearningSummary {
    pub candidate_count: usize,
    pub state_counts: Vec<StoryPublishLearningMetric>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub confirmed_graph_promotion: bool,
    pub memory_promotion_performed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryPublishOutcomeLearningSummary {
    pub outcome_count: usize,
    pub attribution_state: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryPublishRewardLearningSummary {
    pub reward_event_count: usize,
    pub granted_count: usize,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
}

pub fn story_publish_learning_brief(
    connection: &Connection,
    request: StoryPublishLearningBriefRequest,
) -> Result<StoryPublishLearningBrief> {
    let deck_id = safe_identifier(&request.deck_id);
    let private_audience = request.audience.can_read_learning_evidence();
    let analytics_summary =
        summarize_content_analytics_for_content(connection, "homepage_story_deck", &deck_id)?;
    let analytics_events =
        list_content_analytics_events_for_content(connection, "homepage_story_deck", &deck_id)?;

    let publish_evidence =
        publish_evidence_for_artifacts(connection, &request.artifact_ids, request.audience)?;
    let mut memory_review_packets = Vec::new();
    for artifact_id in &request.artifact_ids {
        if load_artifact(connection, artifact_id).is_ok() {
            let packet = generated_content_memory_review_packet_for_artifact(
                connection,
                artifact_id,
                request.audience.memory_audience(),
            )?;
            if packet.candidate_count > 0 {
                memory_review_packets.push(packet);
            }
        }
    }

    let memory_summary = memory_summary(&memory_review_packets, request.audience);
    let outcome_summary = outcome_summary(connection, &analytics_events, request.audience)?;
    let reward_summary = reward_summary(connection, &analytics_events, request.audience)?;
    let content_metrics = content_metrics(&analytics_summary, request.audience);
    let source_status = source_status_metrics(&analytics_summary, request.audience);

    let mut evidence_refs = Vec::new();
    if private_audience {
        append_unique(&mut evidence_refs, &analytics_summary.evidence_refs);
        for source in &publish_evidence {
            append_unique(&mut evidence_refs, &source.evidence_refs);
        }
        append_unique(&mut evidence_refs, &memory_summary.evidence_refs);
        append_unique(&mut evidence_refs, &outcome_summary.evidence_refs);
        append_unique(&mut evidence_refs, &reward_summary.evidence_refs);
    }

    let limitations = limitations(
        &analytics_summary,
        &analytics_events,
        &publish_evidence,
        &memory_summary,
        &outcome_summary,
        &reward_summary,
        request.audience,
    );
    let status = if analytics_events.is_empty() && publish_evidence.is_empty() {
        "missing"
    } else if limitations
        .iter()
        .any(|value| value == "external_analytics_missing" || value == "outcome_evidence_missing")
    {
        "partial"
    } else {
        "complete"
    };
    let recommended_next_actions = recommended_next_actions(status, &limitations);

    Ok(StoryPublishLearningBrief {
        schema_version: STORY_PUBLISH_LEARNING_BRIEF_SCHEMA_VERSION.to_string(),
        status: status.to_string(),
        audience: request.audience.as_str().to_string(),
        deck_id,
        read_only: true,
        mutation_performed: false,
        confirmed_graph_promotion: false,
        memory_promotion_performed: false,
        live_provider_called: false,
        external_publishing_claimed: false,
        source_status,
        content_metrics,
        publish_evidence,
        memory_summary,
        outcome_summary,
        reward_summary,
        evidence_refs: sorted_unique(evidence_refs),
        limitations,
        recommended_next_actions,
        analytics_summary: private_audience.then_some(analytics_summary),
        memory_review_packets: if private_audience {
            memory_review_packets
        } else {
            Vec::new()
        },
    })
}

fn publish_evidence_for_artifacts(
    connection: &Connection,
    artifact_ids: &[String],
    audience: StoryPublishLearningAudience,
) -> Result<Vec<StoryPublishLearningSource>> {
    let mut sources = Vec::new();
    for artifact_id in artifact_ids {
        let Ok(artifact) = load_artifact(connection, artifact_id) else {
            let private_audience = audience.can_read_learning_evidence();
            sources.push(StoryPublishLearningSource {
                source_kind: if private_audience {
                    "artifact".to_string()
                } else {
                    "story_publish_evidence".to_string()
                },
                source_id: if private_audience {
                    safe_identifier(artifact_id)
                } else {
                    "story_publish_evidence_unavailable".to_string()
                },
                status: "missing".to_string(),
                source_status: "missing".to_string(),
                evidence_refs: Vec::new(),
                limitations: vec!["publish_artifact_missing".to_string()],
            });
            continue;
        };
        if artifact.artifact_kind != STORY_HOMEPAGE_PUBLISH_APPROVAL_PACKAGE_ARTIFACT_KIND {
            continue;
        }
        sources.push(source_for_publish_artifact(&artifact, audience));
    }
    Ok(sources)
}

fn source_for_publish_artifact(
    artifact: &ArtifactView,
    audience: StoryPublishLearningAudience,
) -> StoryPublishLearningSource {
    let private_audience = audience.can_read_learning_evidence();
    let mut limitations = limitations_from_artifact(artifact);
    if !private_audience {
        limitations.push("member_public_publish_learning_is_summary_only".to_string());
    }
    StoryPublishLearningSource {
        source_kind: if private_audience {
            artifact.artifact_kind.clone()
        } else {
            "story_publish_evidence".to_string()
        },
        source_id: if private_audience {
            artifact.id.clone()
        } else {
            "story_publish_evidence_available".to_string()
        },
        status: artifact.status.clone(),
        source_status: "manual".to_string(),
        evidence_refs: audience_refs(audience, &artifact.evidence_refs),
        limitations: sorted_unique(limitations),
    }
}

fn content_metrics(
    summary: &ContentAnalyticsSummary,
    audience: StoryPublishLearningAudience,
) -> Vec<StoryPublishLearningMetric> {
    let wanted = [
        "generated",
        "approved",
        "published",
        "viewed",
        "clicked",
        "requested",
        "trial_started",
        "referred",
        "feedback_submitted",
        "outcome_linked",
    ];
    wanted
        .iter()
        .map(|key| {
            let source = summary
                .event_counts
                .iter()
                .find(|metric| metric.key == *key);
            StoryPublishLearningMetric {
                key: (*key).to_string(),
                label: learning_label(key),
                value: source.map(|metric| metric.value).unwrap_or(0),
                source_status: source
                    .map(|metric| metric.source_status.clone())
                    .unwrap_or_else(|| "missing".to_string()),
                evidence_refs: source
                    .map(|metric| audience_refs(audience, &metric.evidence_refs))
                    .unwrap_or_default(),
            }
        })
        .collect()
}

fn source_status_metrics(
    summary: &ContentAnalyticsSummary,
    audience: StoryPublishLearningAudience,
) -> Vec<StoryPublishLearningMetric> {
    ["measured", "manual", "missing"]
        .iter()
        .map(|key| {
            let source = summary
                .source_status_counts
                .iter()
                .find(|metric| metric.key == *key);
            StoryPublishLearningMetric {
                key: (*key).to_string(),
                label: format!("{} evidence", key.replace('_', " ")),
                value: source.map(|metric| metric.value).unwrap_or(0),
                source_status: (*key).to_string(),
                evidence_refs: source
                    .map(|metric| audience_refs(audience, &metric.evidence_refs))
                    .unwrap_or_default(),
            }
        })
        .collect()
}

fn memory_summary(
    packets: &[GeneratedContentMemoryReviewPacket],
    audience: StoryPublishLearningAudience,
) -> StoryPublishMemoryLearningSummary {
    let mut state_counts = std::collections::BTreeMap::<String, i64>::new();
    let mut evidence_refs = Vec::new();
    let mut limitations = Vec::new();
    for packet in packets {
        append_unique(
            &mut evidence_refs,
            &audience_refs(audience, &packet.evidence_refs),
        );
        append_unique(&mut limitations, &packet.limitations);
        for item in &packet.items {
            *state_counts
                .entry(item.candidate_state.clone())
                .or_insert(0) += 1;
        }
    }
    let state_counts = state_counts
        .into_iter()
        .map(|(key, value)| StoryPublishLearningMetric {
            label: format!("{} memory candidates", key.replace('_', " ")),
            key,
            value,
            source_status: "measured".to_string(),
            evidence_refs: Vec::new(),
        })
        .collect::<Vec<_>>();
    StoryPublishMemoryLearningSummary {
        candidate_count: packets.iter().map(|packet| packet.candidate_count).sum(),
        state_counts,
        evidence_refs: sorted_unique(evidence_refs),
        limitations: sorted_unique(limitations),
        confirmed_graph_promotion: false,
        memory_promotion_performed: false,
    }
}

fn outcome_summary(
    connection: &Connection,
    events: &[ContentAnalyticsEventView],
    audience: StoryPublishLearningAudience,
) -> Result<StoryPublishOutcomeLearningSummary> {
    let private_audience = audience.can_read_learning_evidence();
    let outcome_ids = sorted_unique(
        events
            .iter()
            .filter_map(|event| event.outcome_id.clone())
            .collect(),
    );
    let mut evidence_refs = Vec::new();
    let mut found = 0;
    for outcome_id in &outcome_ids {
        let row = connection.query_row(
            "SELECT evidence_refs_json FROM business_outcomes WHERE id = ?1",
            [outcome_id],
            |row| row.get::<_, String>(0),
        );
        if let Ok(refs_json) = row {
            found += 1;
            if private_audience {
                append_unique(
                    &mut evidence_refs,
                    &serde_json::from_str::<Vec<String>>(&refs_json).unwrap_or_default(),
                );
                evidence_refs.push(format!("business_outcome:{outcome_id}"));
            }
        }
    }
    let mut limitations = Vec::new();
    if outcome_ids.is_empty() {
        limitations.push("outcome_evidence_missing".to_string());
    } else if found < outcome_ids.len() {
        limitations.push("partial_outcome_records_missing".to_string());
    }
    if has_outcome_without_supporting_path(events) {
        limitations.push("partial_attribution_is_open_question_not_causal_proof".to_string());
    }
    Ok(StoryPublishOutcomeLearningSummary {
        outcome_count: found,
        attribution_state: if outcome_ids.is_empty() {
            "missing".to_string()
        } else if limitations.is_empty() {
            "evidence_linked_not_causal_proof".to_string()
        } else {
            "partial_or_conflicting".to_string()
        },
        evidence_refs: sorted_unique(evidence_refs),
        limitations: sorted_unique(limitations),
    })
}

fn reward_summary(
    connection: &Connection,
    events: &[ContentAnalyticsEventView],
    audience: StoryPublishLearningAudience,
) -> Result<StoryPublishRewardLearningSummary> {
    let private_audience = audience.can_read_learning_evidence();
    let referral_ids = sorted_unique(
        events
            .iter()
            .filter_map(|event| event.referral_id.clone())
            .collect(),
    );
    let mut event_count = 0;
    let mut granted_count = 0;
    let mut evidence_refs = Vec::new();
    for referral_id in &referral_ids {
        let mut statement = connection.prepare(
            "SELECT id, state, evidence_refs_json
             FROM reward_events
             WHERE source_kind = 'referral_record' AND source_id = ?1
             ORDER BY updated_at DESC, id ASC",
        )?;
        let rows = statement.query_map([referral_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let (event_id, state, refs_json) = row?;
            event_count += 1;
            if state == "granted" {
                granted_count += 1;
            }
            if private_audience {
                evidence_refs.push(format!("reward_event:{event_id}"));
                append_unique(
                    &mut evidence_refs,
                    &serde_json::from_str::<Vec<String>>(&refs_json).unwrap_or_default(),
                );
            }
        }
    }
    let mut limitations = Vec::new();
    if referral_ids.is_empty() {
        limitations.push("referral_evidence_missing".to_string());
    } else if event_count == 0 {
        limitations.push("reward_event_evidence_missing".to_string());
    }
    Ok(StoryPublishRewardLearningSummary {
        reward_event_count: event_count,
        granted_count,
        evidence_refs: sorted_unique(evidence_refs),
        limitations: sorted_unique(limitations),
    })
}

fn limitations(
    summary: &ContentAnalyticsSummary,
    events: &[ContentAnalyticsEventView],
    publish_evidence: &[StoryPublishLearningSource],
    memory_summary: &StoryPublishMemoryLearningSummary,
    outcome_summary: &StoryPublishOutcomeLearningSummary,
    reward_summary: &StoryPublishRewardLearningSummary,
    audience: StoryPublishLearningAudience,
) -> Vec<String> {
    let mut limitations = vec![
        "story_publish_learning_brief_is_read_only".to_string(),
        "canonical_tables_remain_truth".to_string(),
        "events_remain_audit_replay".to_string(),
        "analytics_are_event_first_and_local".to_string(),
        "no_external_analytics_or_publishing_claimed".to_string(),
        "no_memory_or_graph_truth_promotion_performed".to_string(),
        "attribution_is_evidence_linked_not_causal_proof".to_string(),
    ];
    if events.is_empty() {
        limitations.push("content_analytics_missing".to_string());
    }
    if publish_evidence.is_empty() {
        limitations.push("manual_publish_evidence_missing".to_string());
    }
    for limitation in &summary.limitations {
        limitations.push(limitation.key.clone());
    }
    append_unique(&mut limitations, &memory_summary.limitations);
    append_unique(&mut limitations, &outcome_summary.limitations);
    append_unique(&mut limitations, &reward_summary.limitations);
    if !audience.can_read_learning_evidence() {
        limitations.push("member_public_learning_brief_is_summary_only".to_string());
        limitations.push("generated_content_candidate_text_is_not_returned".to_string());
        limitations.push("private_artifact_text_is_not_returned".to_string());
        limitations.push("internal_evidence_refs_are_not_returned".to_string());
    }
    sorted_unique(limitations)
}

fn recommended_next_actions(status: &str, limitations: &[String]) -> Vec<String> {
    let mut actions = Vec::new();
    if status == "missing" {
        actions.push("record_manual_publish_or_content_analytics_evidence".to_string());
    }
    if limitations
        .iter()
        .any(|value| value == "external_analytics_missing")
    {
        actions.push("keep_platform_metrics_marked_missing_until_imported".to_string());
    }
    if limitations
        .iter()
        .any(|value| value == "outcome_evidence_missing")
    {
        actions.push("wait_for_offer_trial_feedback_or_outcome_evidence".to_string());
    }
    if limitations
        .iter()
        .any(|value| value == "reward_event_evidence_missing")
    {
        actions.push("do_not_claim_reward_credit_until_reward_event_exists".to_string());
    }
    if actions.is_empty() {
        actions.push("review_learning_and_choose_next_story_iteration".to_string());
    }
    sorted_unique(actions)
}

fn learning_label(key: &str) -> String {
    match key {
        "trial_started" => "trial started".to_string(),
        "feedback_submitted" => "feedback submitted".to_string(),
        "outcome_linked" => "outcome linked".to_string(),
        other => other.replace('_', " "),
    }
}

fn has_outcome_without_supporting_path(events: &[ContentAnalyticsEventView]) -> bool {
    let has_outcome = events
        .iter()
        .any(|event| event.event_kind == "outcome_linked");
    let has_supporting_path = events.iter().any(|event| {
        matches!(
            event.event_kind.as_str(),
            "clicked" | "requested" | "trial_started" | "referred" | "feedback_submitted"
        )
    });
    has_outcome && !has_supporting_path
}

fn limitations_from_artifact(artifact: &ArtifactView) -> Vec<String> {
    let mut limitations = Vec::new();
    if let Some(contract_limitations) = artifact
        .provenance
        .pointer("/contract/limitations")
        .and_then(Value::as_array)
    {
        for value in contract_limitations {
            if let Some(text) = value.as_str() {
                limitations.push(redaction::redact_public_text(text));
            }
        }
    }
    if artifact.status != "published" {
        limitations.push("publish_package_not_published".to_string());
    }
    sorted_unique(limitations)
}

fn audience_refs(audience: StoryPublishLearningAudience, refs: &[String]) -> Vec<String> {
    if !audience.can_read_learning_evidence() {
        return Vec::new();
    }
    sorted_unique(refs.iter().map(|value| safe_identifier(value)).collect())
}

fn append_unique(values: &mut Vec<String>, additions: &[String]) {
    for value in additions {
        let value = safe_identifier(value);
        if !value.is_empty() && !values.contains(&value) {
            values.push(value);
        }
    }
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    values.sort();
    values.dedup();
    values
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::{record_artifact, ArtifactInput};
    use crate::content_analytics::{
        record_content_analytics_event, ContentAnalyticsEventInput, ContentAnalyticsEventKind,
        ContentAnalyticsSourceStatus,
    };
    use crate::generated_content_memory::{
        ingest_generated_content_memory_candidates, GeneratedContentMemoryIngestionInput,
        GeneratedContentMemoryItemInput, GeneratedContentMemoryKind, GeneratedContentMemoryState,
    };
    use crate::public_surfaces::{
        HomepageNarrativeDeck, HomepageNarrativeSlide, HomepageStoryCopySlot, HomepageStoryCta,
        HomepageStoryDeckResponse, HomepageStoryProfile, HomepageStoryRefreshContract,
        PublicSurfaceReadiness,
    };
    use crate::schema::init_schema;
    use crate::story_publish_approvals::{
        record_homepage_publish_approval_package, HomepagePublishApprovalInput,
    };
    use serde_json::json;

    #[test]
    fn staff_learning_brief_links_publish_analytics_memory_outcomes_and_rewards() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let source = source_artifact(&connection, "source", "Public-safe story source.");
        seed_memory(&connection, &source.id);
        let publish = record_homepage_publish_approval_package(
            &connection,
            publish_input(vec![source.id.clone()], "homepage-publish-v1"),
        )
        .unwrap();
        seed_growth_records(&connection);
        seed_complete_analytics(&connection);

        let brief = story_publish_learning_brief(
            &connection,
            StoryPublishLearningBriefRequest {
                audience: StoryPublishLearningAudience::Staff,
                deck_id: "homepage.story.v1".to_string(),
                artifact_ids: vec![source.id, publish.package_artifact.id],
            },
        )
        .unwrap();

        assert_eq!(
            brief.schema_version,
            STORY_PUBLISH_LEARNING_BRIEF_SCHEMA_VERSION
        );
        assert_eq!(brief.status, "partial");
        assert_eq!(brief.audience, "staff");
        assert_eq!(metric(&brief.content_metrics, "generated"), 1);
        assert_eq!(metric(&brief.content_metrics, "approved"), 1);
        assert_eq!(metric(&brief.content_metrics, "published"), 1);
        assert_eq!(metric(&brief.content_metrics, "viewed"), 2);
        assert_eq!(metric(&brief.content_metrics, "clicked"), 1);
        assert_eq!(metric(&brief.content_metrics, "requested"), 1);
        assert_eq!(metric(&brief.content_metrics, "trial_started"), 1);
        assert_eq!(metric(&brief.content_metrics, "referred"), 1);
        assert_eq!(metric(&brief.content_metrics, "feedback_submitted"), 1);
        assert_eq!(metric(&brief.content_metrics, "outcome_linked"), 1);
        assert_eq!(brief.memory_summary.candidate_count, 2);
        assert_eq!(brief.outcome_summary.outcome_count, 1);
        assert_eq!(
            brief.outcome_summary.attribution_state,
            "evidence_linked_not_causal_proof"
        );
        assert_eq!(brief.reward_summary.reward_event_count, 1);
        assert_eq!(brief.reward_summary.granted_count, 1);
        assert!(!brief.publish_evidence.is_empty());
        assert!(brief.analytics_summary.is_some());
        assert_eq!(brief.memory_review_packets.len(), 1);
        assert!(brief
            .evidence_refs
            .contains(&"business_outcome:outcome_trial_1".to_string()));
        assert!(brief
            .evidence_refs
            .contains(&"reward_event:reward_event_referral_1".to_string()));
        assert!(brief
            .limitations
            .contains(&"external_analytics_missing".to_string()));
        assert!(!brief.confirmed_graph_promotion);
        assert!(!brief.memory_promotion_performed);
        assert!(!brief.live_provider_called);
        assert!(!brief.external_publishing_claimed);

        let encoded = serde_json::to_string(&brief).unwrap();
        assert!(!encoded.contains("caused conversion"));
        assert!(!encoded.contains("external publishing succeeded"));
        assert!(!encoded.contains("provider internal"));
    }

    #[test]
    fn missing_analytics_returns_explicit_limitations_not_inferred_performance() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        let brief = story_publish_learning_brief(
            &connection,
            StoryPublishLearningBriefRequest {
                audience: StoryPublishLearningAudience::Owner,
                deck_id: "missing.deck".to_string(),
                artifact_ids: Vec::new(),
            },
        )
        .unwrap();

        assert_eq!(brief.status, "missing");
        assert!(brief
            .limitations
            .contains(&"content_analytics_missing".to_string()));
        assert!(brief
            .limitations
            .contains(&"manual_publish_evidence_missing".to_string()));
        assert_eq!(metric(&brief.content_metrics, "viewed"), 0);
        assert_eq!(brief.outcome_summary.outcome_count, 0);
        assert_eq!(brief.reward_summary.reward_event_count, 0);
        assert!(brief
            .recommended_next_actions
            .contains(&"record_manual_publish_or_content_analytics_evidence".to_string()));
    }

    #[test]
    fn partial_outcome_path_is_an_open_question_not_causal_proof() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_business_outcome(&connection);
        record_content_analytics_event(
            &connection,
            ContentAnalyticsEventInput {
                event_kind: ContentAnalyticsEventKind::OutcomeLinked,
                outcome_id: Some("outcome_trial_1".to_string()),
                idempotency_key: "outcome-only".to_string(),
                evidence_refs: vec!["business_outcome:outcome_trial_1".to_string()],
                payload: json!({"outcomeKind": "trial_started"}),
                ..analytics_input(ContentAnalyticsEventKind::OutcomeLinked, "outcome-only")
            },
        )
        .unwrap();

        let brief = story_publish_learning_brief(
            &connection,
            StoryPublishLearningBriefRequest {
                audience: StoryPublishLearningAudience::Staff,
                deck_id: "homepage.story.v1".to_string(),
                artifact_ids: Vec::new(),
            },
        )
        .unwrap();

        assert_eq!(brief.outcome_summary.outcome_count, 1);
        assert_eq!(
            brief.outcome_summary.attribution_state,
            "partial_or_conflicting"
        );
        assert!(brief
            .outcome_summary
            .limitations
            .contains(&"partial_attribution_is_open_question_not_causal_proof".to_string()));
        assert!(brief
            .limitations
            .contains(&"partial_attribution_is_open_question_not_causal_proof".to_string()));
    }

    #[test]
    fn member_learning_brief_hides_candidate_text_private_refs_and_internal_details() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        let source = source_artifact(&connection, "private-source", "Public-safe source.");
        seed_memory(&connection, &source.id);
        let publish = record_homepage_publish_approval_package(
            &connection,
            publish_input(vec![source.id.clone()], "homepage-publish-member"),
        )
        .unwrap();
        seed_complete_analytics(&connection);

        let brief = story_publish_learning_brief(
            &connection,
            StoryPublishLearningBriefRequest {
                audience: StoryPublishLearningAudience::Member,
                deck_id: "homepage.story.v1".to_string(),
                artifact_ids: vec![
                    source.id,
                    publish.package_artifact.id,
                    "internal-missing-artifact-id".to_string(),
                ],
            },
        )
        .unwrap();

        assert_eq!(brief.audience, "member");
        assert!(brief.analytics_summary.is_none());
        assert!(brief.memory_review_packets.is_empty());
        assert!(brief.evidence_refs.is_empty());
        assert_eq!(brief.memory_summary.candidate_count, 2);
        assert!(brief
            .content_metrics
            .iter()
            .all(|metric| metric.evidence_refs.is_empty()));
        assert!(brief
            .publish_evidence
            .iter()
            .all(|source| source.evidence_refs.is_empty()));
        assert!(brief
            .limitations
            .contains(&"generated_content_candidate_text_is_not_returned".to_string()));

        let encoded = serde_json::to_string(&brief).unwrap();
        assert!(!encoded.contains("Published story content can inform candidate memory"));
        assert!(!encoded.contains("privateReviewerNote"));
        assert!(!encoded.contains("manual_publish:homepage_v1"));
        assert!(!encoded.contains("approval:owner_1"));
        assert!(!encoded.contains("business_outcome:outcome_trial_1"));
        assert!(!encoded.contains("reward_event:reward_event_referral_1"));
        assert!(!encoded.contains("story.homepage_publish_approval_package"));
        assert!(!encoded.contains("internal-missing-artifact-id"));
        assert!(!encoded.contains("provider internal"));
        assert!(!encoded.contains("prompt internal"));
        assert!(!encoded.contains("private artifact text"));
    }

    #[test]
    fn learning_brief_is_deterministic_and_read_only() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_complete_analytics(&connection);
        let before_analytics: i64 = connection
            .query_row("SELECT COUNT(*) FROM content_analytics_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        let before_memory: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM generated_content_memory_candidates",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let request = StoryPublishLearningBriefRequest {
            audience: StoryPublishLearningAudience::Staff,
            deck_id: "homepage.story.v1".to_string(),
            artifact_ids: Vec::new(),
        };
        let first = story_publish_learning_brief(&connection, request.clone()).unwrap();
        let second = story_publish_learning_brief(&connection, request).unwrap();

        assert_eq!(first, second);
        assert!(first.read_only);
        assert!(!first.mutation_performed);
        assert!(!first.memory_promotion_performed);
        let after_analytics: i64 = connection
            .query_row("SELECT COUNT(*) FROM content_analytics_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        let after_memory: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM generated_content_memory_candidates",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(before_analytics, after_analytics);
        assert_eq!(before_memory, after_memory);
    }

    fn metric(metrics: &[StoryPublishLearningMetric], key: &str) -> i64 {
        metrics
            .iter()
            .find(|metric| metric.key == key)
            .map(|metric| metric.value)
            .unwrap_or(0)
    }

    fn source_artifact(connection: &Connection, source_id: &str, summary: &str) -> ArtifactView {
        record_artifact(
            connection,
            ArtifactInput {
                artifact_kind: "story.test_source".to_string(),
                title: format!("Source {source_id}"),
                status: "ready".to_string(),
                visibility_ceiling: "staff".to_string(),
                summary: summary.to_string(),
                source_kind: Some("test".to_string()),
                source_id: Some(source_id.to_string()),
                evidence_refs: vec![format!("artifact:{source_id}")],
                provenance: json!({"fixture": source_id}),
                content_hash: format!("sha256:{source_id}"),
                storage_uri: None,
                health_status: Some("available".to_string()),
                created_by_job_id: None,
            },
        )
        .unwrap()
        .0
    }

    fn seed_memory(connection: &Connection, artifact_id: &str) {
        let published = GeneratedContentMemoryItemInput {
            memory_kind: GeneratedContentMemoryKind::CandidateClaim,
            candidate_state: Some(GeneratedContentMemoryState::Published),
            summary_text: "Published story content can inform candidate memory.".to_string(),
            body: json!({
                "claim": "Published story content can inform candidate memory.",
                "privateReviewerNote": "Internal review note"
            }),
            confidence: 0.84,
            evidence_refs: vec!["artifact:story_source".to_string()],
            limitations: vec!["candidate_requires_owner_review".to_string()],
            visibility: "staff".to_string(),
            approval_evidence_refs: vec!["approval:owner_1".to_string()],
            publication_evidence_refs: vec!["manual_publish:homepage_v1".to_string()],
            feedback_evidence_refs: vec!["feedback:feedback_1".to_string()],
            outcome_evidence_refs: vec!["business_outcome:outcome_trial_1".to_string()],
            rejection_evidence_refs: Vec::new(),
        };
        let rejected = GeneratedContentMemoryItemInput {
            memory_kind: GeneratedContentMemoryKind::NegativeMemory,
            candidate_state: Some(GeneratedContentMemoryState::Rejected),
            summary_text: "Do not reuse the rejected story direction.".to_string(),
            body: json!({"preference": "avoid rejected story direction"}),
            confidence: 0.8,
            evidence_refs: vec!["artifact_review:review_1".to_string()],
            limitations: vec!["negative_memory_only".to_string()],
            visibility: "staff".to_string(),
            approval_evidence_refs: Vec::new(),
            publication_evidence_refs: Vec::new(),
            feedback_evidence_refs: Vec::new(),
            outcome_evidence_refs: Vec::new(),
            rejection_evidence_refs: vec!["artifact_review:review_1".to_string()],
        };
        ingest_generated_content_memory_candidates(
            connection,
            GeneratedContentMemoryIngestionInput {
                artifact_id: artifact_id.to_string(),
                artifact_version_id: None,
                workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
                workflow_compilation_id: Some("workflow_compilation_story_v1".to_string()),
                job_id: Some("job_story_v1".to_string()),
                extraction_fixture_id: "fixture.story.learning.v1".to_string(),
                items: vec![published, rejected],
            },
        )
        .unwrap();
    }

    fn seed_complete_analytics(connection: &Connection) {
        for (kind, key, refs, payload) in [
            (
                ContentAnalyticsEventKind::Generated,
                "generated-1",
                vec!["artifact:story_source".to_string()],
                json!({"artifact": "story_source"}),
            ),
            (
                ContentAnalyticsEventKind::Approved,
                "approved-1",
                vec!["approval:owner_1".to_string()],
                json!({"approval": "owner"}),
            ),
            (
                ContentAnalyticsEventKind::Viewed,
                "viewed-1",
                vec!["visitor_session:visitor_1".to_string()],
                json!({"localEvent": true}),
            ),
            (
                ContentAnalyticsEventKind::Clicked,
                "clicked-1",
                vec!["visitor_session:visitor_1".to_string()],
                json!({"target": "start_trial"}),
            ),
            (
                ContentAnalyticsEventKind::Requested,
                "requested-1",
                vec!["request:onboarding_1".to_string()],
                json!({"request": "handoff"}),
            ),
            (
                ContentAnalyticsEventKind::TrialStarted,
                "trial-1",
                vec!["trial:trial_1".to_string()],
                json!({"trial": "trial_1"}),
            ),
            (
                ContentAnalyticsEventKind::Referred,
                "referral-1",
                vec!["referral:referral_1".to_string()],
                json!({"referral": "referral_1"}),
            ),
            (
                ContentAnalyticsEventKind::FeedbackSubmitted,
                "feedback-1",
                vec!["feedback:feedback_1".to_string()],
                json!({"feedback": "feedback_1"}),
            ),
            (
                ContentAnalyticsEventKind::OutcomeLinked,
                "outcome-1",
                vec!["business_outcome:outcome_trial_1".to_string()],
                json!({"outcomeKind": "trial_started"}),
            ),
        ] {
            let mut input = analytics_input(kind, key);
            input.evidence_refs = refs;
            input.payload = payload;
            if matches!(
                kind,
                ContentAnalyticsEventKind::Clicked | ContentAnalyticsEventKind::Requested
            ) {
                input.cta_id = Some("start_trial".to_string());
            }
            if kind == ContentAnalyticsEventKind::Referred {
                input.referral_id = Some("referral_1".to_string());
            }
            if kind == ContentAnalyticsEventKind::OutcomeLinked {
                input.outcome_id = Some("outcome_trial_1".to_string());
            }
            record_content_analytics_event(connection, input).unwrap();
        }
        let mut missing = analytics_input(ContentAnalyticsEventKind::Viewed, "missing-platform");
        missing.source_kind = "external_platform".to_string();
        missing.source_id = "platform_missing".to_string();
        missing.source_status = ContentAnalyticsSourceStatus::Missing;
        missing.evidence_refs = vec!["limitation:external_analytics_missing".to_string()];
        missing.limitation_labels = vec!["external_analytics_missing".to_string()];
        missing.payload = json!({"metricStatus": "missing"});
        record_content_analytics_event(connection, missing).unwrap();
    }

    fn analytics_input(
        kind: ContentAnalyticsEventKind,
        idempotency_key: &str,
    ) -> ContentAnalyticsEventInput {
        ContentAnalyticsEventInput {
            event_kind: kind,
            content_ref_kind: "homepage_story_deck".to_string(),
            content_ref_id: "homepage.story.v1".to_string(),
            content_version_id: Some("homepage.story.v1".to_string()),
            artifact_id: Some("artifact_story_v1".to_string()),
            artifact_version_id: None,
            surface: "public_story".to_string(),
            section_id: Some("identity".to_string()),
            cta_id: None,
            workflow_template_id: Some("studio.story.scrollytelling_homepage".to_string()),
            workflow_compilation_id: Some("workflow_compilation_story_v1".to_string()),
            job_id: Some("job_story_v1".to_string()),
            tracked_entry_point_id: Some("entry_point_1".to_string()),
            visitor_session_id: Some("visitor_1".to_string()),
            referral_id: None,
            outcome_id: None,
            source_kind: "test".to_string(),
            source_id: "learning".to_string(),
            idempotency_key: idempotency_key.to_string(),
            source_status: ContentAnalyticsSourceStatus::Measured,
            visibility: "staff".to_string(),
            evidence_refs: vec!["artifact:story_source".to_string()],
            limitation_labels: Vec::new(),
            payload: json!({"localEvent": true}),
            occurred_at: Some("2026-05-15T00:00:00Z".to_string()),
        }
    }

    fn seed_growth_records(connection: &Connection) {
        seed_business_outcome(connection);
        connection
            .execute_batch(
                r#"
                INSERT INTO referral_records (
                    id, status, referrer_connection_id, referred_connection_id, conversation_id,
                    entry_point_id, visitor_session_id, evidence_refs_json, provenance_json,
                    created_at, updated_at, closed_at
                ) VALUES (
                    'referral_1', 'qualified', NULL, NULL,
                    NULL, NULL, NULL, '["referral:referral_1"]', '{}',
                    '2026-05-15T00:00:00Z', '2026-05-15T00:00:00Z', NULL
                );
                INSERT INTO reward_programs (
                    id, slug, name, status, visibility, terms_json, policy_json,
                    starts_at, ends_at, created_at, updated_at
                ) VALUES (
                    'reward_program_story', 'story-referral', 'Story referral',
                    'active', 'staff', '{}', '{}', NULL, NULL,
                    '2026-05-15T00:00:00Z', '2026-05-15T00:00:00Z'
                );
                INSERT INTO reward_rules (
                    id, program_id, trigger_kind, status, benefit_kind, benefit_quantity,
                    benefit_unit, max_quantity_per_actor, qualification_policy_json,
                    created_at, updated_at
                ) VALUES (
                    'reward_rule_story', 'reward_program_story', 'referral_trial',
                    'active', 'hosted_trial_time', 7, 'day', 30, '{}',
                    '2026-05-15T00:00:00Z', '2026-05-15T00:00:00Z'
                );
                INSERT INTO reward_events (
                    id, program_id, rule_id, connection_id, source_kind, source_id, state,
                    idempotency_key, reason, evidence_refs_json, provenance_json,
                    qualified_at, granted_at, created_at, updated_at
                ) VALUES (
                    'reward_event_referral_1', 'reward_program_story', 'reward_rule_story',
                    'connection_referrer', 'referral_record', 'referral_1', 'granted',
                    'reward-referral-1', 'Referral trial activated.',
                    '["referral:referral_1","trial:trial_1"]', '{}',
                    '2026-05-15T00:00:00Z', '2026-05-15T00:00:00Z',
                    '2026-05-15T00:00:00Z', '2026-05-15T00:00:00Z'
                );
                "#,
            )
            .unwrap();
    }

    fn seed_business_outcome(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO business_outcomes (
                    id, outcome_kind, status, connection_id, conversation_id, segment_id,
                    offer_id, ask_id, artifact_id, entry_point_id, visitor_session_id,
                    referral_id, value_micros, currency, evidence_refs_json, provenance_json,
                    occurred_at, created_at, updated_at
                 ) VALUES (
                    'outcome_trial_1', 'trial_started', 'recorded', NULL,
                    NULL, NULL, NULL, NULL, NULL, NULL, NULL,
                    NULL, NULL, NULL, '[\"trial:trial_1\",\"visitor_session:visitor_1\"]',
                    '{}', '2026-05-15T00:00:00Z', '2026-05-15T00:00:00Z',
                    '2026-05-15T00:00:00Z'
                 )",
                [],
            )
            .unwrap();
    }

    fn publish_input(
        source_artifact_ids: Vec<String>,
        idempotency_key: &str,
    ) -> HomepagePublishApprovalInput {
        HomepagePublishApprovalInput {
            package_id: "homepage-v1".to_string(),
            idempotency_key: idempotency_key.to_string(),
            deck: deck(),
            source_artifact_ids,
            image_artifact_ids: Vec::new(),
            approval_state: "approved".to_string(),
            approval_actor_id: "owner_1".to_string(),
            approval_evidence_refs: vec!["approval:owner_1".to_string()],
            manual_publish_evidence_refs: vec!["manual_publish:homepage_v1".to_string()],
            limitations: vec!["Manual local publish only.".to_string()],
            workflow_compilation_id: Some("workflow_compilation_story_v1".to_string()),
            job_id: Some("job_story_v1".to_string()),
            occurred_at: Some("2026-05-15T00:00:00Z".to_string()),
        }
    }

    fn deck() -> HomepageStoryDeckResponse {
        HomepageStoryDeckResponse {
            profile: HomepageStoryProfile {
                positioning: "Studio Ordo is a local-first operating appliance.".to_string(),
                audience: Some("business owners".to_string()),
                primary_cta: Some(HomepageStoryCta {
                    label: "Start trial".to_string(),
                    href: "/offers/nyc-pilot".to_string(),
                    evidence_refs: vec!["business_fact:cta".to_string()],
                }),
                evidence_refs: vec!["business_fact:profile".to_string()],
                limitations: Vec::new(),
            },
            deck: HomepageNarrativeDeck {
                deck_id: "homepage.story.v1".to_string(),
                version: 1,
                surface: "homepage".to_string(),
                slides: vec![HomepageNarrativeSlide {
                    slide_id: "identity".to_string(),
                    section_id: "identity".to_string(),
                    order: 1,
                    title: "A practical answer to enshittification".to_string(),
                    body: "Ordo keeps the owner in control of evidence-backed work.".to_string(),
                    copy_slots: vec![HomepageStoryCopySlot {
                        slot: "sourceLine".to_string(),
                        value: json!("Published public homepage profile"),
                    }],
                    cta_refs: Vec::new(),
                    evidence_refs: vec!["business_fact:slide.identity".to_string()],
                    limitations: Vec::new(),
                    motion_profile: "cinematic".to_string(),
                    reduced_motion_fallback: "Owner-controlled local-first work.".to_string(),
                    image_brief_method: Some("homepage.prepare_image_briefs".to_string()),
                }],
                evidence_refs: vec!["business_fact:deck".to_string()],
                limitations: Vec::new(),
            },
            readiness: PublicSurfaceReadiness {
                surface: "homepage.story".to_string(),
                ready: true,
                fact_count: 3,
                missing: Vec::new(),
            },
            refresh: HomepageStoryRefreshContract {
                manual_refresh_supported: true,
                scheduled_refresh_supported: false,
                image_brief_method: "homepage.prepare_image_briefs".to_string(),
                live_provider_required: false,
                limitations: Vec::new(),
            },
        }
    }
}
