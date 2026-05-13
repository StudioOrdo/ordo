use anyhow::Result;
use chrono::Utc;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

const SCHEMA_VERSION: &str = "ordo.growth_pilot_report.v1";
const RECENT_ITEM_LIMIT: usize = 5;
const EVIDENCE_REF_LIMIT: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrowthReportSourceStatus {
    Measured,
    Manual,
    Missing,
    Deferred,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrowthPilotReportResponse {
    pub schema_version: String,
    pub generated_at: String,
    pub sections: Vec<GrowthPilotReportSection>,
    pub limitations: Vec<GrowthPilotReportLimitation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrowthPilotReportSection {
    pub key: String,
    pub title: String,
    pub source_status: GrowthReportSourceStatus,
    pub metrics: Vec<GrowthPilotReportMetric>,
    pub recent_items: Vec<GrowthPilotReportItem>,
    pub evidence_refs: Vec<GrowthPilotEvidenceRef>,
    pub limitations: Vec<GrowthPilotReportLimitation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrowthPilotReportMetric {
    pub key: String,
    pub label: String,
    pub value: i64,
    pub unit: String,
    pub source_status: GrowthReportSourceStatus,
    pub evidence_refs: Vec<GrowthPilotEvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrowthPilotReportItem {
    pub source_kind: String,
    pub source_id: String,
    pub label: String,
    pub status: String,
    pub source_status: GrowthReportSourceStatus,
    pub occurred_at: String,
    pub evidence_refs: Vec<GrowthPilotEvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrowthPilotEvidenceRef {
    pub source_kind: String,
    pub source_id: String,
    pub label: String,
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrowthPilotReportLimitation {
    pub key: String,
    pub label: String,
    pub detail: String,
    pub source_status: GrowthReportSourceStatus,
}

pub fn growth_pilot_report(db_path: &Path) -> Result<GrowthPilotReportResponse> {
    let connection = Connection::open(db_path)?;
    let sections = vec![
        tracked_entry_section(&connection)?,
        offer_section(&connection)?,
        hosted_trial_section(&connection)?,
        support_handoff_section(&connection)?,
        feedback_section(&connection)?,
        rewards_section(&connection)?,
        studio_promo_section(&connection)?,
    ];
    Ok(GrowthPilotReportResponse {
        schema_version: SCHEMA_VERSION.to_string(),
        generated_at: Utc::now().to_rfc3339(),
        sections,
        limitations: global_limitations(),
    })
}

fn tracked_entry_section(connection: &Connection) -> Result<GrowthPilotReportSection> {
    let mut recent_items = load_recent_items(
        connection,
        "SELECT id, status, created_at
         FROM visitor_sessions
         ORDER BY created_at DESC, id DESC
         LIMIT 5",
        "visitor_session",
        "Visitor session",
    )?;
    recent_items.extend(load_recent_items(
        connection,
        "SELECT id, status, updated_at
         FROM tracked_entry_points
         ORDER BY updated_at DESC, id DESC
         LIMIT 5",
        "tracked_entry_point",
        "Tracked entry point",
    )?);
    recent_items.truncate(RECENT_ITEM_LIMIT);

    Ok(section(
        "tracked_entry",
        "Tracked Entry And Sessions",
        GrowthReportSourceStatus::Measured,
        vec![
            metric(
                "tracked_entry_points",
                "Tracked entry points",
                count(
                    connection,
                    "SELECT COUNT(*) FROM tracked_entry_points WHERE status != 'archived'",
                )?,
                "entry_points",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM tracked_entry_points ORDER BY updated_at DESC, id DESC LIMIT 5",
                    "tracked_entry_point",
                    "Tracked entry point",
                )?,
            ),
            metric(
                "visitor_sessions",
                "Visitor sessions",
                count(connection, "SELECT COUNT(*) FROM visitor_sessions")?,
                "sessions",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM visitor_sessions ORDER BY created_at DESC, id DESC LIMIT 5",
                    "visitor_session",
                    "Visitor session",
                )?,
            ),
            metric(
                "active_visitor_sessions",
                "Active visitor sessions",
                count(
                    connection,
                    "SELECT COUNT(*) FROM visitor_sessions WHERE status = 'active'",
                )?,
                "sessions",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
        ],
        recent_items,
        Vec::new(),
    ))
}

fn offer_section(connection: &Connection) -> Result<GrowthPilotReportSection> {
    let mut recent_items = load_recent_items(
        connection,
        "SELECT id, status, updated_at
         FROM offers
         ORDER BY updated_at DESC, id DESC
         LIMIT 5",
        "offer",
        "Offer",
    )?;
    recent_items.extend(load_recent_items(
        connection,
        "SELECT id, status, accepted_at
         FROM offer_acceptances
         ORDER BY accepted_at DESC, id DESC
         LIMIT 5",
        "offer_acceptance",
        "Offer acceptance",
    )?);
    recent_items.truncate(RECENT_ITEM_LIMIT);

    Ok(section(
        "offers",
        "Offers And Acceptances",
        GrowthReportSourceStatus::Measured,
        vec![
            metric(
                "offers",
                "Offers",
                count(connection, "SELECT COUNT(*) FROM offers")?,
                "offers",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM offers ORDER BY updated_at DESC, id DESC LIMIT 5",
                    "offer",
                    "Offer",
                )?,
            ),
            metric(
                "published_public_offers",
                "Published public offers",
                count(
                    connection,
                    "SELECT COUNT(*) FROM offers
                     WHERE status = 'available'
                       AND visibility = 'public'
                       AND publication_state = 'published'",
                )?,
                "offers",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
            metric(
                "tracked_offer_destination_sessions",
                "Tracked sessions landing on offers",
                count(
                    connection,
                    "SELECT COUNT(*) FROM visitor_sessions WHERE destination_surface = 'offers'",
                )?,
                "sessions",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
            metric(
                "offer_acceptances",
                "Offer acceptances",
                count(connection, "SELECT COUNT(*) FROM offer_acceptances")?,
                "acceptances",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM offer_acceptances ORDER BY accepted_at DESC, id DESC LIMIT 5",
                    "offer_acceptance",
                    "Offer acceptance",
                )?,
            ),
            metric(
                "waitlisted_acceptances",
                "Waitlisted acceptances",
                count(
                    connection,
                    "SELECT COUNT(*) FROM offer_acceptances WHERE status = 'waitlisted'",
                )?,
                "acceptances",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
            metric(
                "individual_offer_view_events",
                "Individual offer view events",
                0,
                "views",
                GrowthReportSourceStatus::Missing,
                Vec::new(),
            ),
        ],
        recent_items,
        vec![limitation(
            "offer_view_events_missing",
            "Individual offer views are not tracked yet",
            "The report counts tracked sessions that land on the offers surface, but there is no durable per-offer view event table in this slice.",
            GrowthReportSourceStatus::Missing,
        )],
    ))
}

fn hosted_trial_section(connection: &Connection) -> Result<GrowthPilotReportSection> {
    let mut recent_items = load_recent_items(
        connection,
        "SELECT id, status, updated_at
         FROM trials
         ORDER BY updated_at DESC, id DESC
         LIMIT 5",
        "trial",
        "Hosted trial",
    )?;
    recent_items.extend(load_recent_items(
        connection,
        "SELECT id, status, updated_at
         FROM hosted_trial_waitlist_entries
         ORDER BY updated_at DESC, id DESC
         LIMIT 5",
        "hosted_trial_waitlist_entry",
        "Hosted trial waitlist entry",
    )?);
    recent_items.truncate(RECENT_ITEM_LIMIT);

    Ok(section(
        "hosted_trials",
        "Hosted Trials, Capacity, Backup, And Reset",
        GrowthReportSourceStatus::Measured,
        vec![
            metric(
                "trials",
                "Hosted trials",
                count(connection, "SELECT COUNT(*) FROM trials")?,
                "trials",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM trials ORDER BY updated_at DESC, id DESC LIMIT 5",
                    "trial",
                    "Hosted trial",
                )?,
            ),
            metric(
                "started_trials",
                "Started trials",
                count(connection, "SELECT COUNT(*) FROM trials WHERE status = 'started'")?,
                "trials",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
            metric(
                "active_hosted_slots",
                "Active hosted trial slots",
                count(
                    connection,
                    "SELECT COUNT(*) FROM hosted_trial_slots WHERE status = 'active'",
                )?,
                "slots",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
            metric(
                "waitlist_entries",
                "Hosted trial waitlist entries",
                count(connection, "SELECT COUNT(*) FROM hosted_trial_waitlist_entries")?,
                "entries",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM hosted_trial_waitlist_entries ORDER BY updated_at DESC, id DESC LIMIT 5",
                    "hosted_trial_waitlist_entry",
                    "Hosted trial waitlist entry",
                )?,
            ),
            metric(
                "backup_ready_slots",
                "Backup-ready hosted slots",
                count(
                    connection,
                    "SELECT COUNT(*) FROM hosted_trial_slots WHERE backup_status = 'ready'",
                )?,
                "slots",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
            metric(
                "reset_ready_slots",
                "Reset-ready hosted slots",
                count(
                    connection,
                    "SELECT COUNT(*) FROM hosted_trial_slots WHERE reset_state = 'ready_for_owner_review'",
                )?,
                "slots",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
        ],
        recent_items,
        Vec::new(),
    ))
}

fn support_handoff_section(connection: &Connection) -> Result<GrowthPilotReportSection> {
    let recent_items = load_recent_items(
        connection,
        "SELECT id, delivery_state, updated_at
         FROM handoff_inbox_items
         ORDER BY updated_at DESC, id DESC
         LIMIT 5",
        "handoff_inbox_item",
        "Support handoff",
    )?;

    Ok(section(
        "support_handoffs",
        "Support Handoffs And Strategy Sessions",
        GrowthReportSourceStatus::Measured,
        vec![
            metric(
                "handoff_items",
                "Support handoff items",
                count(connection, "SELECT COUNT(*) FROM handoff_inbox_items")?,
                "items",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM handoff_inbox_items ORDER BY updated_at DESC, id DESC LIMIT 5",
                    "handoff_inbox_item",
                    "Support handoff",
                )?,
            ),
            metric(
                "strategy_session_handoffs",
                "Strategy session handoffs",
                count(
                    connection,
                    "SELECT COUNT(*) FROM handoff_inbox_items
                     WHERE destination_kind = 'support'
                       AND destination_id = 'strategy_session'",
                )?,
                "items",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
            metric(
                "open_handoffs",
                "Open handoffs",
                count(
                    connection,
                    "SELECT COUNT(*) FROM handoff_inbox_items WHERE resolved_at IS NULL",
                )?,
                "items",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
            metric(
                "resolved_handoffs",
                "Resolved handoffs",
                count(
                    connection,
                    "SELECT COUNT(*) FROM handoff_inbox_items WHERE resolved_at IS NOT NULL",
                )?,
                "items",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
        ],
        recent_items,
        Vec::new(),
    ))
}

fn feedback_section(connection: &Connection) -> Result<GrowthPilotReportSection> {
    let recent_items = load_recent_items(
        connection,
        "SELECT id, status, updated_at
         FROM feedback_requests
         ORDER BY updated_at DESC, id DESC
         LIMIT 5",
        "feedback_request",
        "Feedback request",
    )?;

    Ok(section(
        "feedback",
        "Feedback Requests And Review",
        GrowthReportSourceStatus::Measured,
        vec![
            metric(
                "feedback_requests",
                "Feedback requests",
                count(connection, "SELECT COUNT(*) FROM feedback_requests")?,
                "requests",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM feedback_requests ORDER BY updated_at DESC, id DESC LIMIT 5",
                    "feedback_request",
                    "Feedback request",
                )?,
            ),
            metric(
                "feedback_responses",
                "Feedback responses",
                count(connection, "SELECT COUNT(*) FROM feedback_request_responses")?,
                "responses",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM feedback_request_responses ORDER BY updated_at DESC, id DESC LIMIT 5",
                    "feedback_response",
                    "Feedback response",
                )?,
            ),
            metric(
                "accepted_feedback_reviews",
                "Accepted feedback reviews",
                count(
                    connection,
                    "SELECT COUNT(*) FROM feedback_request_reviews WHERE decision = 'accepted'",
                )?,
                "reviews",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
            metric(
                "reward_eligibility_records",
                "Reward eligibility records",
                count(connection, "SELECT COUNT(*) FROM feedback_reward_eligibility")?,
                "records",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM feedback_reward_eligibility ORDER BY updated_at DESC, id DESC LIMIT 5",
                    "feedback_reward_eligibility",
                    "Feedback reward eligibility",
                )?,
            ),
        ],
        recent_items,
        Vec::new(),
    ))
}

fn rewards_section(connection: &Connection) -> Result<GrowthPilotReportSection> {
    let recent_items = load_recent_items(
        connection,
        "SELECT id, state, updated_at
         FROM reward_events
         ORDER BY updated_at DESC, id DESC
         LIMIT 5",
        "reward_event",
        "Reward event",
    )?;

    Ok(section(
        "rewards",
        "Rewards, Ledger, Benefits, And Balances",
        GrowthReportSourceStatus::Measured,
        vec![
            metric(
                "reward_events",
                "Reward events",
                count(connection, "SELECT COUNT(*) FROM reward_events")?,
                "events",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM reward_events ORDER BY updated_at DESC, id DESC LIMIT 5",
                    "reward_event",
                    "Reward event",
                )?,
            ),
            metric(
                "qualified_reward_events",
                "Qualified reward events",
                count(connection, "SELECT COUNT(*) FROM reward_events WHERE state = 'qualified'")?,
                "events",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
            metric(
                "granted_reward_events",
                "Granted reward events",
                count(connection, "SELECT COUNT(*) FROM reward_events WHERE state = 'granted'")?,
                "events",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
            metric(
                "capped_reward_events",
                "Capped reward events",
                count(
                    connection,
                    "SELECT COUNT(*) FROM reward_events WHERE capped_at IS NOT NULL",
                )?,
                "events",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
            metric(
                "reversed_ledger_entries",
                "Reversed ledger entries",
                count(
                    connection,
                    "SELECT COUNT(*) FROM reward_ledger_entries WHERE reversed_at IS NOT NULL",
                )?,
                "entries",
                GrowthReportSourceStatus::Measured,
                Vec::new(),
            ),
            metric(
                "benefit_grants",
                "Benefit grants",
                count(connection, "SELECT COUNT(*) FROM benefit_grants")?,
                "grants",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM benefit_grants ORDER BY updated_at DESC, id DESC LIMIT 5",
                    "benefit_grant",
                    "Benefit grant",
                )?,
            ),
            metric(
                "benefit_balances",
                "Benefit balances",
                count(connection, "SELECT COUNT(*) FROM benefit_balances")?,
                "balances",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM benefit_balances ORDER BY updated_at DESC, id DESC LIMIT 5",
                    "benefit_balance",
                    "Benefit balance",
                )?,
            ),
            metric(
                "public_leaderboard_rank",
                "Public leaderboard rank",
                0,
                "ranks",
                GrowthReportSourceStatus::Deferred,
                Vec::new(),
            ),
        ],
        recent_items,
        vec![limitation(
            "leaderboard_deferred",
            "Leaderboard is deferred",
            "Reward evidence is available to Growth, but public or community leaderboard projections are out of scope for this slice.",
            GrowthReportSourceStatus::Deferred,
        )],
    ))
}

fn studio_promo_section(connection: &Connection) -> Result<GrowthPilotReportSection> {
    let recent_items = load_recent_items(
        connection,
        "SELECT id, status, updated_at
         FROM artifacts
         WHERE artifact_kind = 'studio.promo_video.package'
         ORDER BY updated_at DESC, id DESC
         LIMIT 5",
        "artifact",
        "Promo package artifact",
    )?;

    Ok(section(
        "studio_promos",
        "Studio Promo Packages And Publication Evidence",
        GrowthReportSourceStatus::Measured,
        vec![
            metric(
                "promo_video_packages",
                "Promo video packages",
                count(
                    connection,
                    "SELECT COUNT(*) FROM artifacts
                     WHERE artifact_kind = 'studio.promo_video.package'",
                )?,
                "packages",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT id FROM artifacts
                     WHERE artifact_kind = 'studio.promo_video.package'
                     ORDER BY updated_at DESC, id DESC
                     LIMIT 5",
                    "artifact",
                    "Promo package artifact",
                )?,
            ),
            metric(
                "staged_manual_packages",
                "Staged manual promo packages",
                count(
                    connection,
                    "SELECT COUNT(*) FROM artifacts
                     WHERE artifact_kind = 'studio.promo_video.package'
                       AND health_status = 'staged_manual'",
                )?,
                "packages",
                GrowthReportSourceStatus::Manual,
                Vec::new(),
            ),
            metric(
                "promo_deliverables",
                "Promo deliverables",
                count(
                    connection,
                    "SELECT COUNT(*)
                     FROM artifact_deliverables d
                     JOIN artifacts a ON a.id = d.artifact_id
                     WHERE a.artifact_kind = 'studio.promo_video.package'",
                )?,
                "deliverables",
                GrowthReportSourceStatus::Measured,
                latest_refs(
                    connection,
                    "SELECT d.id
                     FROM artifact_deliverables d
                     JOIN artifacts a ON a.id = d.artifact_id
                     WHERE a.artifact_kind = 'studio.promo_video.package'
                     ORDER BY d.updated_at DESC, d.id DESC
                     LIMIT 5",
                    "artifact_deliverable",
                    "Promo deliverable",
                )?,
            ),
            metric(
                "external_publications",
                "External platform publications",
                0,
                "publications",
                GrowthReportSourceStatus::Deferred,
                Vec::new(),
            ),
            metric(
                "platform_performance_metrics",
                "Platform performance metrics",
                0,
                "metrics",
                GrowthReportSourceStatus::Missing,
                Vec::new(),
            ),
        ],
        recent_items,
        vec![
            limitation(
                "external_publication_deferred",
                "External publishing is deferred",
                "The promo package workflow stages local artifacts only; TikTok/YouTube posting and OAuth are not part of this slice.",
                GrowthReportSourceStatus::Deferred,
            ),
            limitation(
                "platform_analytics_missing",
                "Platform analytics are missing",
                "The report does not claim views, watch time, conversions, or platform performance without a future governed integration.",
                GrowthReportSourceStatus::Missing,
            ),
        ],
    ))
}

fn section(
    key: &str,
    title: &str,
    source_status: GrowthReportSourceStatus,
    metrics: Vec<GrowthPilotReportMetric>,
    recent_items: Vec<GrowthPilotReportItem>,
    limitations: Vec<GrowthPilotReportLimitation>,
) -> GrowthPilotReportSection {
    let evidence_refs = collect_evidence_refs(&metrics, &recent_items);
    GrowthPilotReportSection {
        key: key.to_string(),
        title: title.to_string(),
        source_status,
        metrics,
        recent_items,
        evidence_refs,
        limitations,
    }
}

fn metric(
    key: &str,
    label: &str,
    value: i64,
    unit: &str,
    source_status: GrowthReportSourceStatus,
    evidence_refs: Vec<GrowthPilotEvidenceRef>,
) -> GrowthPilotReportMetric {
    GrowthPilotReportMetric {
        key: key.to_string(),
        label: label.to_string(),
        value,
        unit: unit.to_string(),
        source_status,
        evidence_refs,
    }
}

fn limitation(
    key: &str,
    label: &str,
    detail: &str,
    source_status: GrowthReportSourceStatus,
) -> GrowthPilotReportLimitation {
    GrowthPilotReportLimitation {
        key: key.to_string(),
        label: label.to_string(),
        detail: detail.to_string(),
        source_status,
    }
}

fn global_limitations() -> Vec<GrowthPilotReportLimitation> {
    vec![
        limitation(
            "external_publishing_deferred",
            "External publishing is deferred",
            "No TikTok, YouTube, OAuth, or platform publishing API is called by this report.",
            GrowthReportSourceStatus::Deferred,
        ),
        limitation(
            "platform_analytics_missing",
            "Platform analytics are missing",
            "No platform reach, watch-time, or conversion metric is reported unless future durable evidence exists.",
            GrowthReportSourceStatus::Missing,
        ),
        limitation(
            "payments_oauth_deferred",
            "Payments and OAuth are deferred",
            "The NYC pilot report intentionally avoids payment processing and OAuth state.",
            GrowthReportSourceStatus::Deferred,
        ),
        limitation(
            "conversion_rates_not_calculated",
            "Conversion rates are not calculated",
            "The report exposes evidence-backed counts and recent evidence, not inferred conversion rates or unsupported scarcity claims.",
            GrowthReportSourceStatus::Deferred,
        ),
        limitation(
            "live_provider_behavior_not_measured",
            "Live provider behavior is not measured",
            "Provider work, AI capability, uptime, and media rendering are not inferred by this deterministic report.",
            GrowthReportSourceStatus::Deferred,
        ),
    ]
}

fn count(connection: &Connection, sql: &str) -> Result<i64> {
    Ok(connection.query_row(sql, [], |row| row.get(0))?)
}

fn latest_refs(
    connection: &Connection,
    sql: &str,
    source_kind: &str,
    label_prefix: &str,
) -> Result<Vec<GrowthPilotEvidenceRef>> {
    let mut statement = connection.prepare(sql)?;
    let refs = statement
        .query_map([], |row| {
            let id: String = row.get(0)?;
            Ok(evidence_ref(source_kind, &id, label_prefix))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(refs)
}

fn load_recent_items(
    connection: &Connection,
    sql: &str,
    source_kind: &str,
    label_prefix: &str,
) -> Result<Vec<GrowthPilotReportItem>> {
    let mut statement = connection.prepare(sql)?;
    let items = statement
        .query_map([], |row| item_from_row(row, source_kind, label_prefix))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(items)
}

fn item_from_row(
    row: &Row<'_>,
    source_kind: &str,
    label_prefix: &str,
) -> rusqlite::Result<GrowthPilotReportItem> {
    let id: String = row.get(0)?;
    let status: String = row.get(1)?;
    let occurred_at: String = row.get(2)?;
    Ok(GrowthPilotReportItem {
        source_kind: source_kind.to_string(),
        source_id: id.clone(),
        label: format!("{label_prefix} {id}"),
        status,
        source_status: GrowthReportSourceStatus::Measured,
        occurred_at,
        evidence_refs: vec![evidence_ref(source_kind, &id, label_prefix)],
    })
}

fn evidence_ref(source_kind: &str, source_id: &str, label_prefix: &str) -> GrowthPilotEvidenceRef {
    GrowthPilotEvidenceRef {
        source_kind: source_kind.to_string(),
        source_id: source_id.to_string(),
        label: format!("{label_prefix} {source_id}"),
        uri: format!("ordo://{source_kind}/{source_id}"),
    }
}

fn collect_evidence_refs(
    metrics: &[GrowthPilotReportMetric],
    recent_items: &[GrowthPilotReportItem],
) -> Vec<GrowthPilotEvidenceRef> {
    let mut seen = BTreeSet::new();
    let mut refs = Vec::new();
    for reference in metrics
        .iter()
        .flat_map(|metric| metric.evidence_refs.iter())
        .chain(
            recent_items
                .iter()
                .flat_map(|item| item.evidence_refs.iter()),
        )
    {
        if seen.insert(reference.uri.clone()) {
            refs.push(reference.clone());
        }
        if refs.len() >= EVIDENCE_REF_LIMIT {
            break;
        }
    }
    refs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::{BusinessFactVisibility, PublicationState};
    use crate::entry_points::{
        create_entry_point, create_visitor_session, EntryPointWriteRequest,
        PublicDestinationSurface, VisitorSessionCreateRequest,
    };
    use crate::feedback::{
        create_feedback_request, respond_to_feedback_request, review_feedback_request,
        FeedbackRequestCreateRequest, FeedbackRequestRespondRequest, FeedbackRequestReviewDecision,
        FeedbackRequestReviewRequest,
    };
    use crate::offers::{
        accept_public_offer, create_offer, AcceptanceStatus, OfferAcceptanceCreateRequest,
        OfferStatus, OfferWriteRequest,
    };
    use crate::rewards::{qualify_feedback_reward, RewardQualificationRequest};
    use crate::schema::init_database;
    use crate::studio_promos::{create_promo_video_package, PromoVideoPackageRequest};
    use serde_json::json;

    #[test]
    fn empty_growth_pilot_report_is_deterministic_and_explicit_about_missing_sources() {
        let (_temp_dir, db_path) = setup_db();

        let report = growth_pilot_report(&db_path).unwrap();

        assert_eq!(report.schema_version, SCHEMA_VERSION);
        assert_eq!(
            metric_value(&report, "tracked_entry", "visitor_sessions"),
            0
        );
        assert_eq!(
            metric_status(&report, "offers", "individual_offer_view_events"),
            GrowthReportSourceStatus::Missing
        );
        assert_eq!(
            metric_status(&report, "studio_promos", "platform_performance_metrics"),
            GrowthReportSourceStatus::Missing
        );
        assert!(report
            .limitations
            .iter()
            .any(|limitation| limitation.key == "platform_analytics_missing"));
        assert!(report.sections.iter().all(|section| {
            section
                .metrics
                .iter()
                .all(|metric| !matches!(metric.source_status, GrowthReportSourceStatus::Unknown))
        }));
    }

    #[test]
    fn report_aggregates_current_pilot_evidence_without_private_payloads() {
        let (_temp_dir, db_path) = setup_db();
        seed_pilot_evidence(&db_path);

        let report = growth_pilot_report(&db_path).unwrap();

        assert_eq!(
            metric_value(&report, "tracked_entry", "visitor_sessions"),
            1
        );
        assert_eq!(metric_value(&report, "offers", "offer_acceptances"), 1);
        assert_eq!(metric_value(&report, "hosted_trials", "trials"), 1);
        assert_eq!(
            metric_value(&report, "support_handoffs", "strategy_session_handoffs"),
            1
        );
        assert_eq!(metric_value(&report, "feedback", "feedback_requests"), 1);
        assert_eq!(metric_value(&report, "rewards", "benefit_grants"), 1);
        assert_eq!(
            metric_value(&report, "studio_promos", "promo_video_packages"),
            1
        );
        assert_eq!(
            metric_status(&report, "studio_promos", "external_publications"),
            GrowthReportSourceStatus::Deferred
        );

        let serialized = serde_json::to_string(&report).unwrap();
        assert!(!serialized.contains("SECRET_MEMBER_DETAIL"));
        assert!(!serialized.contains("SECRET_STAFF_ROUTE"));
        assert!(!serialized.contains("SECRET_FEEDBACK_BODY"));
        assert!(serialized.contains("ordo://offer_acceptance/"));
        assert!(serialized.contains("ordo://artifact/"));
    }

    #[test]
    fn report_is_read_only_and_does_not_mutate_events_or_policy_decisions() {
        let (_temp_dir, db_path) = setup_db();
        seed_pilot_evidence(&db_path);
        let before_events = table_count(&db_path, "realtime_events");
        let before_policy = table_count(&db_path, "policy_decisions");

        let first = growth_pilot_report(&db_path).unwrap();
        let second = growth_pilot_report(&db_path).unwrap();

        assert_eq!(before_events, table_count(&db_path, "realtime_events"));
        assert_eq!(before_policy, table_count(&db_path, "policy_decisions"));
        assert_eq!(
            metric_value(&first, "studio_promos", "promo_video_packages"),
            metric_value(&second, "studio_promos", "promo_video_packages")
        );
    }

    fn setup_db() -> (tempfile::TempDir, std::path::PathBuf) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        (temp_dir, db_path)
    }

    fn seed_pilot_evidence(db_path: &Path) {
        let (offer, _) = create_offer(
            db_path,
            OfferWriteRequest {
                slug: "nyc-pilot".to_string(),
                title: "OrdoStudio NYC Pilot".to_string(),
                summary: "Thirty day hosted Ordo pilot.".to_string(),
                status: Some(OfferStatus::Available),
                visibility: Some(BusinessFactVisibility::Public),
                publication_state: Some(PublicationState::Published),
                trial_days: Some(30),
                source_kind: Some("growth".to_string()),
                source_ref: Some("nyc-meetup".to_string()),
                terms: Some(json!({ "trialDays": 30, "activeSpots": 10 })),
                metadata: Some(json!({ "campaign": "nyc-pilot" })),
            },
            None,
        )
        .unwrap();
        let (entry_point, _) = create_entry_point(
            db_path,
            EntryPointWriteRequest {
                slug: "nyc-meetup".to_string(),
                label: "NYC meetup QR".to_string(),
                status: None,
                source_kind: "event".to_string(),
                source_label: Some("NYC tech meetup".to_string()),
                destination_surface: PublicDestinationSurface::Offers,
                destination_id: Some(offer.id.clone()),
                attribution: Some(json!({ "campaign": "nyc-pilot" })),
                metadata: Some(json!({ "location": "nyc" })),
            },
            None,
        )
        .unwrap();
        let (session, _) = create_visitor_session(
            db_path,
            VisitorSessionCreateRequest {
                entry_point_slug: entry_point.slug,
                session_id: None,
                user_agent: Some("SECRET_MEMBER_DETAIL browser".to_string()),
                attribution: Some(json!({ "source": "qr" })),
            },
        )
        .unwrap();
        let (acceptance, trial, access_grant, _, _) = accept_public_offer(
            db_path,
            &offer.slug,
            OfferAcceptanceCreateRequest {
                visitor_session_id: Some(session.id.clone()),
                local_session_id: None,
                idempotency_key: Some("accept-once".to_string()),
                attribution: Some(json!({ "entryPointId": session.entry_point_id })),
                acceptance_context: Some(json!({ "note": "accepted at table" })),
            },
        )
        .unwrap();
        assert_eq!(acceptance.status, AcceptanceStatus::Accepted);
        let member_actor_id = access_grant.subject_id.clone();

        crate::availability::request_strategy_session_handoff(
            db_path,
            crate::availability::StrategySessionHandoffRequest {
                conversation_id: None,
                visitor_session_id: Some(session.id.clone()),
                trial_id: Some(trial.id.clone()),
                access_grant_id: Some(access_grant.id.clone()),
                connection_id: Some("connection_1".to_string()),
                member_actor_id: Some(member_actor_id.clone()),
                message_excerpt: Some("SECRET_STAFF_ROUTE please route to Keith".to_string()),
                context_summary: Some("Strategy help requested.".to_string()),
                urgency: Some("normal".to_string()),
                connection_trust: None,
                evaluated_at: None,
                evidence_refs: Some(vec![format!("trial:{}", trial.id)]),
            },
            None,
        )
        .unwrap();

        let (feedback, _) = create_feedback_request(
            db_path,
            FeedbackRequestCreateRequest {
                target_kind: "trial".to_string(),
                target_id: trial.id.clone(),
                member_actor_id: Some(member_actor_id.clone()),
                connection_id: Some("connection_1".to_string()),
                conversation_id: None,
                source_kind: "growth".to_string(),
                source_id: Some("nyc-pilot".to_string()),
                prompt: "Share feedback about the hosted pilot.".to_string(),
                member_context_summary: "Pilot member was invited to give feedback.".to_string(),
                due_at: None,
                priority: Some("normal".to_string()),
                evidence_refs: vec![format!("trial:{}", trial.id)],
                provenance: json!({ "source": "test" }),
                staff_context: json!({ "route": "SECRET_STAFF_ROUTE" }),
            },
            None,
        )
        .unwrap();
        let (responded, _) = respond_to_feedback_request(
            db_path,
            &feedback.id,
            FeedbackRequestRespondRequest {
                response_kind: Some("feedback".to_string()),
                body_summary: "SECRET_FEEDBACK_BODY useful feedback.".to_string(),
                idempotency_key: None,
                evidence_refs: vec![format!("feedback_request:{}", feedback.id)],
                provenance: json!({ "source": "member" }),
            },
            None,
        )
        .unwrap();
        let response_id = responded.responses.first().unwrap().id.clone();
        let (reviewed, _) = review_feedback_request(
            db_path,
            &feedback.id,
            FeedbackRequestReviewRequest {
                decision: FeedbackRequestReviewDecision::Accepted,
                response_id: Some(response_id.clone()),
                tags: vec!["pilot-feedback".to_string()],
                reason: "Useful pilot feedback.".to_string(),
                evidence_refs: vec![format!("feedback_request_response:{response_id}")],
                provenance: json!({ "source": "support" }),
            },
            None,
        )
        .unwrap();
        let eligibility = reviewed.reward_eligibility.unwrap();
        qualify_feedback_reward(
            db_path,
            &eligibility.id,
            RewardQualificationRequest {
                trial_id: Some(trial.id.clone()),
                activation_trial_id: None,
                actor_id: Some(member_actor_id),
                connection_id: Some("connection_1".to_string()),
                evidence_refs: vec![format!("feedback_reward_eligibility:{}", eligibility.id)],
                reason: Some("Accepted feedback earns hosted days.".to_string()),
                amount: None,
                idempotency_key: Some("feedback-reward-once".to_string()),
            },
            None,
        )
        .unwrap();

        create_promo_video_package(
            db_path,
            PromoVideoPackageRequest {
                title: Some("NYC pilot promo".to_string()),
                brief: "Invite builders to try OrdoStudio for thirty days.".to_string(),
                audience: Some("NYC meetup builders".to_string()),
                offer_id: Some(offer.id),
                duration_seconds: Some(20),
                platforms: Some(vec!["tiktok".to_string(), "youtube_shorts".to_string()]),
                aspect_ratio: Some("9:16".to_string()),
                evidence_refs: Some(vec![format!("offer_acceptance:{}", acceptance.id)]),
            },
            "test",
            None,
        )
        .unwrap();
    }

    fn section<'a>(
        report: &'a GrowthPilotReportResponse,
        key: &str,
    ) -> &'a GrowthPilotReportSection {
        report
            .sections
            .iter()
            .find(|section| section.key == key)
            .expect("section")
    }

    fn metric<'a>(
        report: &'a GrowthPilotReportResponse,
        section_key: &str,
        metric_key: &str,
    ) -> &'a GrowthPilotReportMetric {
        section(report, section_key)
            .metrics
            .iter()
            .find(|metric| metric.key == metric_key)
            .expect("metric")
    }

    fn metric_value(
        report: &GrowthPilotReportResponse,
        section_key: &str,
        metric_key: &str,
    ) -> i64 {
        metric(report, section_key, metric_key).value
    }

    fn metric_status(
        report: &GrowthPilotReportResponse,
        section_key: &str,
        metric_key: &str,
    ) -> GrowthReportSourceStatus {
        metric(report, section_key, metric_key).source_status
    }

    fn table_count(db_path: &Path, table: &str) -> i64 {
        let connection = Connection::open(db_path).unwrap();
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }
}
