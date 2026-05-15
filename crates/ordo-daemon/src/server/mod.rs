use anyhow::Result;
use axum::routing::{get, post, put};
use axum::Router;
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

use crate::briefs::run_due_system_brief_schedules;
use crate::diagnostics::diagnostic_log;
use crate::schema::init_database;

pub mod handlers;
pub mod state;
pub mod supervisor;

pub(crate) use handlers::*;
pub use state::*;
pub(crate) use supervisor::*;

pub async fn serve(
    host: String,
    port: u16,
    db_path: PathBuf,
    next_supervisor: Option<NextSupervisorConfig>,
    access_token: Option<String>,
) -> Result<()> {
    init_database(&db_path)?;
    let _generated_briefs = run_due_system_brief_schedules(&db_path)?;

    let (event_sender, _) = broadcast::channel(128);
    let (conversation_sender, _) = broadcast::channel(256);
    let next_supervisor_status = next_supervisor
        .as_ref()
        .map(|_| Arc::new(Mutex::new(NextSupervisorStatus::starting())));
    let state = AppState {
        db_path: Arc::new(db_path),
        event_sender,
        conversation_sender,
        next_supervisor_status,
        access_policy: DaemonAccessPolicy::new(access_token),
    };
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/capabilities", get(capabilities_handler))
        .route("/install/state", get(install_state_handler))
        .route("/install/complete", post(install_complete_handler))
        .route("/local-sessions/login", post(local_session_login_handler))
        .route(
            "/local-sessions/register",
            post(local_session_register_handler),
        )
        .route("/chat/bootstrap", post(chat_bootstrap_handler))
        .route("/providers", get(providers_handler))
        .route("/providers/:provider_id", put(provider_update_handler))
        .route("/business/facts", get(business_facts_handler))
        .route("/business/facts", post(business_fact_create_handler))
        .route(
            "/business/facts/:fact_id",
            put(business_fact_update_handler),
        )
        .route("/public/surfaces", get(public_surfaces_handler))
        .route("/public/about", get(public_about_handler))
        .route("/public/offers", get(public_offers_handler))
        .route("/public/asks", get(public_asks_handler))
        .route("/public/feed", get(public_feed_handler))
        .route("/public/homepage-story", get(public_homepage_story_handler))
        .route(
            "/public/story-analytics",
            post(public_story_analytics_handler),
        )
        .route("/entry-points", get(entry_points_handler))
        .route("/entry-points", post(entry_point_create_handler))
        .route(
            "/entry-points/:entry_point_id",
            put(entry_point_update_handler),
        )
        .route("/visitor-sessions", get(visitor_sessions_handler))
        .route(
            "/offer-builder",
            get(offer_builder_handler).post(offer_builder_save_handler),
        )
        .route("/offers", get(offers_handler))
        .route("/offers", post(offer_create_handler))
        .route("/offers/:offer_id", put(offer_update_handler))
        .route("/offer-acceptances", get(offer_acceptances_handler))
        .route("/trials", get(trials_handler))
        .route("/trials/:trial_id/status", put(trial_transition_handler))
        .route(
            "/hosted-trials/capacity",
            get(hosted_trial_capacity_handler),
        )
        .route(
            "/hosted-trials/:trial_id/reset-ready",
            post(hosted_trial_reset_ready_handler),
        )
        .route("/connections", get(connections_handler))
        .route("/connections", post(connection_create_handler))
        .route(
            "/connections/:connection_id",
            put(connection_update_handler),
        )
        .route(
            "/connections/:connection_id/grants",
            get(connection_grants_handler),
        )
        .route(
            "/connections/:connection_id/grants",
            post(connection_grant_create_handler),
        )
        .route(
            "/connections/:connection_id/grants/:grant_id/revoke",
            put(connection_grant_revoke_handler),
        )
        .route(
            "/connections/:connection_id/events",
            get(connection_events_handler),
        )
        .route("/availability", get(availability_handler))
        .route(
            "/availability/schedule",
            put(availability_schedule_update_handler),
        )
        .route(
            "/availability/presence",
            put(operator_presence_update_handler),
        )
        .route("/handoff/eligibility", post(handoff_eligibility_handler))
        .route("/handoff/inbox", get(handoff_inbox_handler))
        .route("/handoff/inbox", post(handoff_inbox_create_handler))
        .route(
            "/handoff/inbox/:item_id",
            get(handoff_inbox_read_handler).put(handoff_inbox_update_handler),
        )
        .route(
            "/handoff/inbox/:item_id/resolve",
            put(handoff_inbox_resolve_handler),
        )
        .route(
            "/handoff/inbox/:item_id/receipts",
            get(handoff_receipts_handler),
        )
        .route(
            "/strategy-sessions/request",
            post(strategy_session_request_handler),
        )
        .route(
            "/strategy-sessions/:item_id/status",
            get(strategy_session_status_handler),
        )
        .route(
            "/feedback/requests",
            get(feedback_requests_handler).post(feedback_request_create_handler),
        )
        .route(
            "/feedback/requests/:request_id/respond",
            post(feedback_request_respond_handler),
        )
        .route(
            "/feedback/requests/:request_id/review",
            post(feedback_request_review_handler),
        )
        .route("/rewards", get(rewards_handler))
        .route("/growth/pilot-report", get(growth_pilot_report_handler))
        .route(
            "/rewards/referrals/:referral_id/qualify",
            post(reward_referral_qualify_handler),
        )
        .route(
            "/rewards/feedback/:eligibility_id/qualify",
            post(reward_feedback_qualify_handler),
        )
        .route(
            "/rewards/events/:event_id/status",
            put(reward_event_transition_handler),
        )
        .route(
            "/public/available-offers",
            get(public_available_offers_handler),
        )
        .route(
            "/public/offers/:offer_slug/accept",
            post(public_offer_accept_handler),
        )
        .route("/public/e/:slug", get(public_entry_point_handler))
        .route(
            "/public/visitor-sessions",
            post(public_session_create_handler),
        )
        .route("/logs", get(logs_handler))
        .route("/policy-decisions", get(policy_decisions_handler))
        .route("/briefs/system/latest", get(latest_system_brief_handler))
        .route(
            "/briefs/system/generate",
            post(generate_system_brief_handler),
        )
        .route("/backups", get(list_backup_restore_handler))
        .route("/backups/create", post(create_backup_handler))
        .route("/restore/validate", post(validate_restore_handler))
        .route("/events", get(events_handler))
        .route("/schedules", get(schedules_handler))
        .route("/surface/work-items", get(surface_work_items_handler))
        .route(
            "/studio/story-production-review",
            get(studio_story_production_review_handler),
        )
        .route(
            "/studio/story-publish-learning",
            get(studio_story_publish_learning_handler),
        )
        .route(
            "/studio/generated-content-memory/:artifact_id/review",
            get(generated_content_memory_review_handler),
        )
        .route(
            "/studio/generated-content-memory/candidates/:candidate_id/decision",
            post(generated_content_memory_decision_handler),
        )
        .route(
            "/studio/promo-video-packages",
            post(studio_promo_video_package_create_handler),
        )
        .route(
            "/studio/promo-video-packages/:artifact_id/review",
            put(studio_promo_video_package_review_handler),
        )
        .route(
            "/studio/artifact-patches",
            get(studio_artifact_patch_review_list_handler),
        )
        .route(
            "/studio/artifact-patches/:proposal_id",
            get(studio_artifact_patch_review_read_handler),
        )
        .route(
            "/studio/artifact-patches/:proposal_id/accept",
            put(studio_artifact_patch_accept_handler),
        )
        .route("/corpus/sources", get(corpus_sources_handler))
        .route("/corpus/sources", post(corpus_source_create_handler))
        .route(
            "/corpus/sources/:source_id",
            get(corpus_source_read_handler),
        )
        .route(
            "/corpus/sources/:source_id",
            put(corpus_source_update_handler),
        )
        .route("/corpus/items", get(corpus_items_handler))
        .route("/corpus/items", post(corpus_item_create_handler))
        .route("/corpus/items/:item_id", get(corpus_item_read_handler))
        .route("/corpus/items/:item_id", put(corpus_item_update_handler))
        .route("/corpus/retrieve", post(corpus_retrieve_handler))
        .route("/answer-drafts", get(answer_drafts_handler))
        .route("/answer-drafts", post(answer_draft_prepare_handler))
        .route("/answer-drafts/:draft_id", get(answer_draft_read_handler))
        .route("/mcp/packs", get(mcp_packs_handler))
        .route("/mcp/packs", post(mcp_pack_install_handler))
        .route("/mcp/packs/:pack_id", get(mcp_pack_read_handler))
        .route("/mcp/packs/:pack_id/disable", put(mcp_pack_disable_handler))
        .route("/product-packs", get(product_packs_handler))
        .route("/product-packs", post(product_pack_install_handler))
        .route("/product-packs/:pack_id", get(product_pack_read_handler))
        .route(
            "/product-packs/:pack_id/disable",
            put(product_pack_disable_handler),
        )
        .route("/reports/issues", get(list_issue_reports_handler))
        .route("/reports/issues/:report_id", get(read_issue_report_handler))
        .route(
            "/reports/issues/:report_id/status",
            put(update_issue_report_status_handler),
        )
        .route(
            "/reports/issues/:report_id/exports",
            post(export_issue_report_handler),
        )
        .route(
            "/reports/issues/prepare",
            post(prepare_issue_report_handler),
        )
        .route("/support-packets", get(support_packets_handler))
        .route("/support-packets", post(draft_support_packet_handler))
        .route(
            "/support-packets/:packet_id/approve",
            put(approve_support_packet_handler),
        )
        .route(
            "/support-packets/:packet_id/receipts",
            get(support_packet_receipts_handler),
        )
        .route("/mcp", post(mcp_handler))
        .route("/ws", get(ws_handler))
        .route("/chat/ws", get(chat_ws_handler))
        .with_state(state.clone());

    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "daemon.started",
        json!({ "host": host, "port": port }),
    );
    record_log(
        &state.db_path,
        diagnostic_log(
            "info",
            "daemon",
            "Daemon started.",
            json!({ "host": host, "port": port }),
        ),
    );
    if let (Some(config), Some(next_status)) =
        (next_supervisor, state.next_supervisor_status.clone())
    {
        spawn_next_supervisor(
            config,
            state.db_path.clone(),
            state.event_sender.clone(),
            next_status,
        )?;
    }
    spawn_system_brief_scheduler(state.db_path.clone(), state.event_sender.clone());

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
