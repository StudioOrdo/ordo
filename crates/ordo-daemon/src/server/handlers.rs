use anyhow::Result;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path as AxumPath, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use tokio::sync::broadcast;

use crate::answer_drafts::{
    list_answer_drafts, prepare_answer_draft, read_answer_draft, AnswerDraftListResponse,
    AnswerDraftRequest, AnswerDraftResponse,
};
use crate::artifact_patches::{
    apply_artifact_patch_review_proposal, list_artifact_patch_review_proposals,
    load_artifact_patch_review_proposal, ApplyArtifactPatchProposalInput,
    ArtifactPatchApplyResponse, ArtifactPatchReviewListResponse, ArtifactPatchReviewResponse,
};
use crate::availability::{
    create_handoff_inbox_item, evaluate_handoff_eligibility, list_handoff_inbox_with_query,
    list_handoff_receipts, read_availability_state, read_handoff_inbox_item,
    read_strategy_session_status, request_public_relationship_handoff,
    request_strategy_session_handoff, resolve_handoff_inbox_item, update_availability_schedule,
    update_handoff_inbox_item, update_operator_presence, AvailabilityScheduleView,
    AvailabilityScheduleWriteRequest, AvailabilityStateResponse, HandoffEligibilityRequest,
    HandoffEligibilityView, HandoffInboxCreateRequest, HandoffInboxItemView, HandoffInboxListQuery,
    HandoffInboxListResponse, HandoffInboxResolveRequest, HandoffInboxUpdateRequest,
    HandoffReceiptListResponse, OperatorPresenceView, OperatorPresenceWriteRequest,
    PublicRelationshipHandoffRequest, PublicRelationshipHandoffResponse,
    StrategySessionHandoffRequest, StrategySessionHandoffResponse, StrategySessionStatusView,
};
use crate::backups::{
    create_backup, list_backup_restore_jobs, run_restore_preflight, BackupRestoreResponse,
    RestorePreflightRequest,
};
use crate::briefs::{generate_system_brief, latest_system_brief, LatestBriefResponse};
use crate::business::{
    create_business_fact, list_business_facts, update_business_fact, BusinessFactListResponse,
    BusinessFactQuery, BusinessFactView, BusinessFactWriteRequest,
};
use crate::capabilities::{list_capabilities, CapabilityCatalogResponse};
use crate::chat_bootstrap::{bootstrap_local_chat, ChatBootstrapRequest, ChatBootstrapResponse};
use crate::connections::{
    create_connection, create_connection_grant, list_connection_events, list_connection_grants,
    list_connections, revoke_connection_grant, update_connection, ConnectionEventListResponse,
    ConnectionGrantCreateRequest, ConnectionGrantListResponse, ConnectionGrantRevokeRequest,
    ConnectionGrantView, ConnectionListResponse, ConnectionView, ConnectionWriteRequest,
};
use crate::content_analytics::{
    record_public_story_content_analytics, PublicStoryContentAnalyticsRequest,
    PublicStoryContentAnalyticsResponse,
};
use crate::conversation_gateway::handle_conversation_socket;
use crate::corpus::{
    create_corpus_item, create_corpus_source, list_corpus_items, list_corpus_sources,
    read_corpus_item, read_corpus_source, retrieve_corpus, update_corpus_item,
    update_corpus_source, CorpusItemListResponse, CorpusItemView, CorpusItemWriteRequest,
    CorpusRetrievalQuery, CorpusRetrievalResponse, CorpusSourceListResponse, CorpusSourceView,
    CorpusSourceWriteRequest, CorpusViewer,
};
use crate::diagnostics::{
    diagnostic_log, list_diagnostic_logs, record_diagnostic_log, DiagnosticLogQuery,
    DiagnosticLogsResponse, NewDiagnosticLogEntry,
};
use crate::entry_points::{
    create_entry_point, create_visitor_session, list_entry_points, list_visitor_sessions,
    resolve_entry_point, update_entry_point, EntryPointListResponse, EntryPointWriteRequest,
    PublicEntryPointView, PublicVisitorSessionView, TrackedEntryPointView,
    VisitorSessionCreateRequest, VisitorSessionListResponse,
};
use crate::errors::{DaemonErrorCode, ErrorResponse};
use crate::events::{
    append_system_event, replay_events, system_event, EventReplayResponse, RealtimeEvent,
};
use crate::feedback::{
    create_feedback_request, list_feedback_requests, respond_to_feedback_request,
    review_feedback_request, FeedbackRequestCreateRequest, FeedbackRequestListResponse,
    FeedbackRequestQuery, FeedbackRequestRespondRequest, FeedbackRequestReviewRequest,
    FeedbackRequestView,
};
use crate::generated_content_memory::{
    generated_content_memory_review_packet_for_artifact, record_generated_content_memory_decision,
    GeneratedContentMemoryCandidateView, GeneratedContentMemoryDecisionInput,
    GeneratedContentMemoryReviewAudience, GeneratedContentMemoryReviewPacket,
};
use crate::growth_report::{growth_pilot_report, GrowthPilotReportResponse};
use crate::health::{build_health_report, build_readiness_report, HealthReport, ReadinessReport};
use crate::install::{
    complete_local_install, list_provider_configs, read_install_state, update_provider_config,
    CompleteInstallRequest, InstallStateResponse, ProviderConfigView, ProviderListResponse,
    ProviderUpdateRequest,
};
use crate::local_sessions::{
    create_or_restore_local_session, LocalSessionCreateRequest, LocalSessionResponse,
};
use crate::mcp::{handle_mcp_json, McpResponse};
use crate::mcp_packs::{
    disable_mcp_pack, install_mcp_pack, list_mcp_packs, read_mcp_pack, McpPackInstallRequest,
    McpPackListResponse, McpPackResponse,
};
use crate::offers::{
    accept_public_offer, create_offer, inspect_offer_builder, list_hosted_trial_capacity,
    list_offer_acceptances, list_offers, list_public_available_offers, list_trials,
    request_hosted_trial_reset, save_offer_builder_offer, transition_trial, update_offer,
    HostedTrialCapacityResponse, HostedTrialResetPlanView, HostedTrialResetRequest,
    OfferAcceptanceCreateRequest, OfferAcceptanceListResponse, OfferAcceptanceResponse,
    OfferBuilderResponse, OfferBuilderSaveRequest, OfferBuilderSaveResponse, OfferListResponse,
    OfferView, OfferWriteRequest, PublicOfferListResponse, TrialListResponse,
    TrialTransitionRequest, TrialView,
};
use crate::policy::{
    authorize_protected_daemon_action, record_policy_decision, ActorContext, PolicyAction,
    PolicyDecision, PolicyDecisionCorrelation, PolicyOutcome, ProtectedAccessEvidence,
    ResourceKind, ResourceRef,
};
use crate::policy_audit::{
    list_policy_decisions, PolicyDecisionAuditQuery, PolicyDecisionAuditResponse,
};
use crate::product_packs::{
    disable_product_pack, install_product_pack, list_product_packs, read_product_pack,
    ProductPackInstallRequest, ProductPackListResponse, ProductPackResponse,
};
use crate::public_surfaces::{
    homepage_story_deck, public_about, public_asks, public_feed, public_offers, public_surfaces,
    AboutReadModel, AsksReadModel, FeedReadModel, HomepageStoryDeckResponse, OffersReadModel,
    PublicSurfacesResponse,
};
use crate::reports::{
    approve_support_packet, draft_support_packet, export_issue_report, list_issue_reports,
    list_support_packet_receipts, list_support_packets, prepare_issue_report, read_issue_report,
    update_issue_report_status, IssueReportDetailResponse, IssueReportExportRequest,
    IssueReportExportResponse, IssueReportPrepareRequest, IssueReportStatusUpdateRequest,
    IssueReportsResponse, SupportPacketApprovalRequest, SupportPacketDraftRequest,
    SupportPacketListResponse, SupportPacketReceiptListResponse, SupportPacketView,
};
use crate::rewards::{
    list_rewards, qualify_feedback_reward, qualify_referral_reward, transition_reward_event,
    RewardEventTransitionRequest, RewardQualificationRequest, RewardQualificationResponse,
    RewardQuery, RewardSummaryResponse,
};
use crate::scheduler::{read_scheduler_operations, SchedulerOperationsResponse};
use crate::secrets::{constant_time_secret_eq, OrdoSecretString};
use crate::story_intake_artifacts::{
    record_story_founder_intake_packet, StoryFounderIntakeInput, StoryFounderIntakePacket,
    StoryWorkflowApprovalGateEvidence, StoryWorkflowCompilationEvidence,
    StoryWorkflowFanoutEvidence, StoryWorkflowProviderRequirementEvidence,
    StoryWorkflowResolvedVariable, StoryWorkflowTaskBindingEvidence,
};
use crate::story_production_review::{
    story_production_review_packet, StoryProductionReviewAudience, StoryProductionReviewPacket,
    StoryProductionReviewPacketRequest,
};
use crate::story_publish_learning::{
    story_publish_learning_brief, StoryPublishLearningAudience, StoryPublishLearningBrief,
    StoryPublishLearningBriefRequest,
};
use crate::studio_promos::{
    create_promo_video_package, review_promo_video_package, PromoVideoPackageRequest,
    PromoVideoPackageResponse, PromoVideoPackageReviewRequest, PromoVideoPackageReviewResponse,
};
use crate::surface_work_items::{
    list_surface_work_items, SurfaceWorkItemListResponse, SurfaceWorkItemQuery,
};
use crate::workflow_templates::{
    compile_story_homepage_refresh_workflow, StoryHomepageRefreshCompileOutcome,
    StoryHomepageRefreshCompileRequest, STORY_HOMEPAGE_REFRESH_TEMPLATE_ID,
};

const DAEMON_ACCESS_TOKEN_HEADER: &str = "x-ordo-daemon-token";

use super::state::*;
use super::supervisor::*;
pub(crate) async fn health_handler() -> Json<HealthReport> {
    Json(build_health_report())
}

pub(crate) async fn ready_handler(
    State(state): State<AppState>,
) -> (StatusCode, Json<ReadinessReport>) {
    let mut report = build_readiness_report(&state.db_path);
    if let Some(next_status) = &state.next_supervisor_status {
        let next_check = next_supervisor_readiness_check(next_status);
        if next_check.status != "ok" {
            report.status = "not_ready".to_string();
        }
        report.checks.push(next_check);
    }
    let status = if report.status == "ready" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(report))
}

pub(crate) async fn capabilities_handler(
    State(state): State<AppState>,
) -> Result<Json<CapabilityCatalogResponse>, (StatusCode, Json<ErrorResponse>)> {
    list_capabilities(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn install_state_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<InstallStateResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/install/state"),
        Some("install.state.read"),
    )?;
    read_install_state(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn install_complete_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<CompleteInstallRequest>,
) -> Result<Json<InstallStateResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/install/complete"),
        Some("install.complete"),
    )?;
    let (state_response, event) =
        complete_local_install(&state.db_path, request).map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(state_response))
}

pub(crate) async fn local_session_login_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(mut request): Json<LocalSessionCreateRequest>,
) -> Result<Json<LocalSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/local-sessions/login"),
        Some("local_session.login"),
    )?;
    request.mode = "login".to_string();
    let (response, event) =
        create_or_restore_local_session(&state.db_path, request).map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(response))
}

pub(crate) async fn local_session_register_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(mut request): Json<LocalSessionCreateRequest>,
) -> Result<Json<LocalSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/local-sessions/register"),
        Some("local_session.register"),
    )?;
    request.mode = "register".to_string();
    let (response, event) =
        create_or_restore_local_session(&state.db_path, request).map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(response))
}

pub(crate) async fn chat_bootstrap_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<ChatBootstrapRequest>,
) -> Result<Json<ChatBootstrapResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/chat/bootstrap"),
        Some("conversation.bootstrap"),
    )?;
    let (response, event) =
        bootstrap_local_chat(&state.db_path, request).map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(response))
}

pub(crate) async fn providers_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ProviderListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/providers"),
        Some("providers.list"),
    )?;
    list_provider_configs(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn provider_update_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(provider_id): AxumPath<String>,
    Json(request): Json<ProviderUpdateRequest>,
) -> Result<Json<ProviderConfigView>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/providers/{provider_id}"),
        ),
        Some("providers.update"),
    )?;
    let (provider, event) = update_provider_config(&state.db_path, &provider_id, request)
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(provider))
}

pub(crate) async fn business_facts_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<BusinessFactQuery>,
) -> Result<Json<BusinessFactListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/business/facts"),
        Some("business.facts.list"),
    )?;
    list_business_facts(&state.db_path, query)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn business_fact_create_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<BusinessFactWriteRequest>,
) -> Result<Json<BusinessFactView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/business/facts"),
        Some("business.facts.write"),
    )?;
    let (fact, event) = create_business_fact(&state.db_path, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(fact))
}

pub(crate) async fn business_fact_update_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(fact_id): AxumPath<String>,
    Json(request): Json<BusinessFactWriteRequest>,
) -> Result<Json<BusinessFactView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/business/facts/{fact_id}"),
        ),
        Some("business.facts.write"),
    )?;
    let (fact, event) =
        update_business_fact(&state.db_path, &fact_id, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(fact))
}

pub(crate) async fn public_surfaces_handler(
    State(state): State<AppState>,
) -> Result<Json<PublicSurfacesResponse>, (StatusCode, Json<ErrorResponse>)> {
    public_surfaces(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn public_about_handler(
    State(state): State<AppState>,
) -> Result<Json<AboutReadModel>, (StatusCode, Json<ErrorResponse>)> {
    public_about(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn public_offers_handler(
    State(state): State<AppState>,
) -> Result<Json<OffersReadModel>, (StatusCode, Json<ErrorResponse>)> {
    public_offers(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn public_asks_handler(
    State(state): State<AppState>,
) -> Result<Json<AsksReadModel>, (StatusCode, Json<ErrorResponse>)> {
    public_asks(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn public_feed_handler(
    State(state): State<AppState>,
) -> Result<Json<FeedReadModel>, (StatusCode, Json<ErrorResponse>)> {
    public_feed(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn public_homepage_story_handler(
    State(state): State<AppState>,
) -> Result<Json<HomepageStoryDeckResponse>, (StatusCode, Json<ErrorResponse>)> {
    homepage_story_deck(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn public_story_analytics_handler(
    State(state): State<AppState>,
    Json(request): Json<PublicStoryContentAnalyticsRequest>,
) -> Result<Json<PublicStoryContentAnalyticsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (response, event) = record_public_story_content_analytics(&state.db_path, request)
        .map_err(invalid_request_error)?;
    if let Some(event) = event {
        let _ = state.event_sender.send(event);
    }
    Ok(Json(response))
}

pub(crate) async fn entry_points_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<EntryPointListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/entry-points"),
        Some("entry_points.list"),
    )?;
    list_entry_points(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn entry_point_create_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<EntryPointWriteRequest>,
) -> Result<Json<TrackedEntryPointView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/entry-points"),
        Some("entry_points.write"),
    )?;
    let (entry_point, event) = create_entry_point(&state.db_path, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(entry_point))
}

pub(crate) async fn entry_point_update_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(entry_point_id): AxumPath<String>,
    Json(request): Json<EntryPointWriteRequest>,
) -> Result<Json<TrackedEntryPointView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/entry-points/{entry_point_id}"),
        ),
        Some("entry_points.write"),
    )?;
    let (entry_point, event) = update_entry_point(
        &state.db_path,
        &entry_point_id,
        request,
        actor_id(&decision),
    )
    .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(entry_point))
}

pub(crate) async fn visitor_sessions_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<VisitorSessionListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/visitor-sessions"),
        Some("visitor_sessions.list"),
    )?;
    list_visitor_sessions(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn public_entry_point_handler(
    State(state): State<AppState>,
    AxumPath(slug): AxumPath<String>,
) -> Result<Json<PublicEntryPointView>, (StatusCode, Json<ErrorResponse>)> {
    resolve_entry_point(&state.db_path, &slug)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn public_session_create_handler(
    State(state): State<AppState>,
    Json(request): Json<VisitorSessionCreateRequest>,
) -> Result<Json<PublicVisitorSessionView>, (StatusCode, Json<ErrorResponse>)> {
    let (session, event) =
        create_visitor_session(&state.db_path, request).map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(session.into_public_view()))
}

pub(crate) async fn public_relationship_handoff_handler(
    State(state): State<AppState>,
    AxumPath(slug): AxumPath<String>,
    Json(request): Json<PublicRelationshipHandoffRequest>,
) -> Result<Json<PublicRelationshipHandoffResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (response, event) = request_public_relationship_handoff(&state.db_path, &slug, request)
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(response))
}

pub(crate) async fn offers_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<OfferListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/offers"),
        Some("offers.list"),
    )?;
    list_offers(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn offer_builder_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<OfferBuilderResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/offer-builder"),
        Some("offer_builder.inspect"),
    )?;
    inspect_offer_builder(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn offer_builder_save_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<OfferBuilderSaveRequest>,
) -> Result<Json<OfferBuilderSaveResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/offer-builder"),
        Some("offer_builder.write"),
    )?;
    let (response, event) = save_offer_builder_offer(&state.db_path, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(response))
}

pub(crate) async fn offer_create_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<OfferWriteRequest>,
) -> Result<Json<OfferView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/offers"),
        Some("offers.write"),
    )?;
    let (offer, event) = create_offer(&state.db_path, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(offer))
}

pub(crate) async fn offer_update_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(offer_id): AxumPath<String>,
    Json(request): Json<OfferWriteRequest>,
) -> Result<Json<OfferView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, format!("/offers/{offer_id}")),
        Some("offers.write"),
    )?;
    let (offer, event) = update_offer(&state.db_path, &offer_id, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(offer))
}

pub(crate) async fn offer_acceptances_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<OfferAcceptanceListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/offer-acceptances"),
        Some("offer_acceptances.list"),
    )?;
    list_offer_acceptances(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn trials_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<TrialListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/trials"),
        Some("trials.list"),
    )?;
    list_trials(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn trial_transition_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(trial_id): AxumPath<String>,
    Json(request): Json<TrialTransitionRequest>,
) -> Result<Json<TrialView>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/trials/{trial_id}/status"),
        ),
        Some("trials.transition"),
    )?;
    let (trial, event) =
        transition_trial(&state.db_path, &trial_id, request).map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(trial))
}

pub(crate) async fn hosted_trial_capacity_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<HostedTrialCapacityResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/hosted-trials/capacity"),
        Some("hosted_trials.capacity.inspect"),
    )?;
    list_hosted_trial_capacity(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn hosted_trial_reset_ready_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(trial_id): AxumPath<String>,
    Json(request): Json<HostedTrialResetRequest>,
) -> Result<Json<HostedTrialResetPlanView>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Validate,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/hosted-trials/{trial_id}/reset-ready"),
        ),
        Some("hosted_trials.reset_ready.validate"),
    )?;
    let (plan, event) = request_hosted_trial_reset(&state.db_path, &trial_id, request)
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(plan))
}

pub(crate) async fn connections_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ConnectionListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/connections"),
        Some("connections.list"),
    )?;
    list_connections(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn connection_create_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<ConnectionWriteRequest>,
) -> Result<Json<ConnectionView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/connections"),
        Some("connections.write"),
    )?;
    let (connection, event) = create_connection(&state.db_path, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(connection))
}

pub(crate) async fn connection_update_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(connection_id): AxumPath<String>,
    Json(request): Json<ConnectionWriteRequest>,
) -> Result<Json<ConnectionView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/connections/{connection_id}"),
        ),
        Some("connections.write"),
    )?;
    let (connection, event) =
        update_connection(&state.db_path, &connection_id, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(connection))
}

pub(crate) async fn connection_grants_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(connection_id): AxumPath<String>,
) -> Result<Json<ConnectionGrantListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/connections/{connection_id}/grants"),
        ),
        Some("connection_grants.list"),
    )?;
    list_connection_grants(&state.db_path, &connection_id)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn connection_grant_create_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(connection_id): AxumPath<String>,
    Json(request): Json<ConnectionGrantCreateRequest>,
) -> Result<Json<ConnectionGrantView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/connections/{connection_id}/grants"),
        ),
        Some("connection_grants.write"),
    )?;
    let (grant, event) =
        create_connection_grant(&state.db_path, &connection_id, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(grant))
}

pub(crate) async fn connection_grant_revoke_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath((connection_id, grant_id)): AxumPath<(String, String)>,
    Json(request): Json<ConnectionGrantRevokeRequest>,
) -> Result<Json<ConnectionGrantView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/connections/{connection_id}/grants/{grant_id}/revoke"),
        ),
        Some("connection_grants.write"),
    )?;
    let (grant, event) =
        revoke_connection_grant(&state.db_path, &grant_id, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(grant))
}

pub(crate) async fn connection_events_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(connection_id): AxumPath<String>,
) -> Result<Json<ConnectionEventListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/connections/{connection_id}/events"),
        ),
        Some("connection_events.list"),
    )?;
    list_connection_events(&state.db_path, &connection_id)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn availability_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<AvailabilityStateResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/availability"),
        Some("availability.read"),
    )?;
    read_availability_state(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn availability_schedule_update_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<AvailabilityScheduleWriteRequest>,
) -> Result<Json<AvailabilityScheduleView>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/availability/schedule"),
        Some("availability.write"),
    )?;
    let (schedule, event) =
        update_availability_schedule(&state.db_path, request).map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(schedule))
}

pub(crate) async fn operator_presence_update_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<OperatorPresenceWriteRequest>,
) -> Result<Json<OperatorPresenceView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/availability/presence"),
        Some("availability.write"),
    )?;
    let (presence, event) = update_operator_presence(&state.db_path, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(presence))
}

pub(crate) async fn handoff_eligibility_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<HandoffEligibilityRequest>,
) -> Result<Json<HandoffEligibilityView>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/handoff/eligibility"),
        Some("handoff.eligibility.evaluate"),
    )?;
    evaluate_handoff_eligibility(&state.db_path, request)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn handoff_inbox_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<HandoffInboxListQuery>,
) -> Result<Json<HandoffInboxListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/handoff/inbox"),
        Some("handoff.inbox.list"),
    )?;
    list_handoff_inbox_with_query(&state.db_path, query)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn handoff_inbox_read_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(item_id): AxumPath<String>,
) -> Result<Json<HandoffInboxItemView>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/handoff/inbox/{item_id}"),
        ),
        Some("handoff.inbox.list"),
    )?;
    read_handoff_inbox_item(&state.db_path, &item_id)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn handoff_inbox_create_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<HandoffInboxCreateRequest>,
) -> Result<Json<HandoffInboxItemView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/handoff/inbox"),
        Some("handoff.inbox.write"),
    )?;
    let (item, event) = create_handoff_inbox_item(&state.db_path, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(item))
}

pub(crate) async fn handoff_inbox_resolve_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(item_id): AxumPath<String>,
    Json(request): Json<HandoffInboxResolveRequest>,
) -> Result<Json<HandoffInboxItemView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/handoff/inbox/{item_id}/resolve"),
        ),
        Some("handoff.inbox.write"),
    )?;
    let (item, event) =
        resolve_handoff_inbox_item(&state.db_path, &item_id, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(item))
}

pub(crate) async fn handoff_inbox_update_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(item_id): AxumPath<String>,
    Json(request): Json<HandoffInboxUpdateRequest>,
) -> Result<Json<HandoffInboxItemView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Update,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/handoff/inbox/{item_id}"),
        ),
        Some("handoff.inbox.write"),
    )?;
    let (item, event) =
        update_handoff_inbox_item(&state.db_path, &item_id, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(item))
}

pub(crate) async fn handoff_receipts_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(item_id): AxumPath<String>,
) -> Result<Json<HandoffReceiptListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/handoff/inbox/{item_id}/receipts"),
        ),
        Some("handoff.receipts.list"),
    )?;
    list_handoff_receipts(&state.db_path, &item_id)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn strategy_session_request_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<StrategySessionHandoffRequest>,
) -> Result<Json<StrategySessionHandoffResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/strategy-sessions/request"),
        Some("strategy_sessions.request"),
    )?;
    let (response, event) =
        request_strategy_session_handoff(&state.db_path, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(response))
}

pub(crate) async fn strategy_session_status_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(item_id): AxumPath<String>,
) -> Result<Json<StrategySessionStatusView>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/strategy-sessions/{item_id}/status"),
        ),
        Some("strategy_sessions.status.read"),
    )?;
    read_strategy_session_status(&state.db_path, &item_id)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn feedback_requests_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<FeedbackRequestQuery>,
) -> Result<Json<FeedbackRequestListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/feedback/requests"),
        Some("feedback.requests.list"),
    )?;
    list_feedback_requests(&state.db_path, query)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn feedback_request_create_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<FeedbackRequestCreateRequest>,
) -> Result<Json<FeedbackRequestView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/feedback/requests"),
        Some("feedback.requests.write"),
    )?;
    let (request, event) = create_feedback_request(&state.db_path, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(request))
}

pub(crate) async fn feedback_request_respond_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(request_id): AxumPath<String>,
    Json(request): Json<FeedbackRequestRespondRequest>,
) -> Result<Json<FeedbackRequestView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/feedback/requests/{request_id}/respond"),
        ),
        Some("feedback.requests.respond"),
    )?;
    let (request, event) =
        respond_to_feedback_request(&state.db_path, &request_id, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(request))
}

pub(crate) async fn feedback_request_review_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(request_id): AxumPath<String>,
    Json(request): Json<FeedbackRequestReviewRequest>,
) -> Result<Json<FeedbackRequestView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Approve,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/feedback/requests/{request_id}/review"),
        ),
        Some("feedback.requests.review"),
    )?;
    let (request, event) =
        review_feedback_request(&state.db_path, &request_id, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(request))
}

pub(crate) async fn rewards_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<RewardQuery>,
) -> Result<Json<RewardSummaryResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/rewards"),
        Some("rewards.list"),
    )?;
    list_rewards(&state.db_path, query)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn growth_pilot_report_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<GrowthPilotReportResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/growth/pilot-report"),
        Some("growth.pilot_report.read"),
    )?;
    growth_pilot_report(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn reward_referral_qualify_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(referral_id): AxumPath<String>,
    Json(request): Json<RewardQualificationRequest>,
) -> Result<Json<RewardQualificationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Approve,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/rewards/referrals/{referral_id}/qualify"),
        ),
        Some("rewards.qualify"),
    )?;
    let (response, event) =
        qualify_referral_reward(&state.db_path, &referral_id, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(response))
}

pub(crate) async fn reward_feedback_qualify_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(eligibility_id): AxumPath<String>,
    Json(request): Json<RewardQualificationRequest>,
) -> Result<Json<RewardQualificationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Approve,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/rewards/feedback/{eligibility_id}/qualify"),
        ),
        Some("rewards.qualify"),
    )?;
    let (response, event) = qualify_feedback_reward(
        &state.db_path,
        &eligibility_id,
        request,
        actor_id(&decision),
    )
    .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(response))
}

pub(crate) async fn reward_event_transition_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(event_id): AxumPath<String>,
    Json(request): Json<RewardEventTransitionRequest>,
) -> Result<Json<RewardQualificationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Update,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/rewards/events/{event_id}/status"),
        ),
        Some("rewards.update"),
    )?;
    let (response, event) =
        transition_reward_event(&state.db_path, &event_id, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(response))
}

pub(crate) async fn public_available_offers_handler(
    State(state): State<AppState>,
) -> Result<Json<PublicOfferListResponse>, (StatusCode, Json<ErrorResponse>)> {
    list_public_available_offers(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn public_offer_accept_handler(
    State(state): State<AppState>,
    AxumPath(offer_slug): AxumPath<String>,
    Json(request): Json<OfferAcceptanceCreateRequest>,
) -> Result<Json<OfferAcceptanceResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (acceptance, trial, access_grant, receipt, event) =
        accept_public_offer(&state.db_path, &offer_slug, request).map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(OfferAcceptanceResponse {
        acceptance,
        trial,
        access_grant,
        receipt,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EventReplayQuery {
    after: Option<i64>,
    limit: Option<usize>,
}

pub(crate) async fn latest_system_brief_handler(
    State(state): State<AppState>,
) -> Result<Json<LatestBriefResponse>, (StatusCode, Json<ErrorResponse>)> {
    latest_system_brief(&state.db_path)
        .map(|brief| Json(LatestBriefResponse { brief }))
        .map_err(internal_error)
}

pub(crate) async fn logs_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<DiagnosticLogQuery>,
) -> Result<Json<DiagnosticLogsResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/logs"),
        Some("diagnostic.logs.list"),
    )?;
    list_diagnostic_logs(&state.db_path, query)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn policy_decisions_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<PolicyDecisionAuditQuery>,
) -> Result<Json<PolicyDecisionAuditResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/policy-decisions"),
        Some("policy.decisions.list"),
    )?;
    list_policy_decisions(&state.db_path, query)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn generate_system_brief_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<LatestBriefResponse>, (StatusCode, Json<ErrorResponse>)> {
    let policy_decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Generate,
        ResourceRef::new(ResourceKind::DaemonRoute, "/briefs/system/generate"),
        Some("brief.system.generate"),
    )?;
    let brief = generate_system_brief(&state.db_path, "http", actor_id(&policy_decision))
        .map_err(internal_error)?;
    record_log(
        &state.db_path,
        NewDiagnosticLogEntry {
            job_id: brief.job_id.clone(),
            capability_id: Some("brief.system.generate".to_string()),
            event_type: Some("brief.system.generated".to_string()),
            ..diagnostic_log(
                "info",
                "brief",
                "System Brief generated.",
                json!({ "briefId": brief.id }),
            )
        },
    );
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "brief.system.generated",
        json!({ "briefId": brief.id, "jobId": brief.job_id, "version": brief.version }),
    );
    Ok(Json(LatestBriefResponse { brief: Some(brief) }))
}

pub(crate) async fn list_backup_restore_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<BackupRestoreResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/backups"),
        Some("backup.restore_jobs.list"),
    )?;
    list_backup_restore_jobs(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn create_backup_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<BackupRestoreResponse>, (StatusCode, Json<ErrorResponse>)> {
    let policy_decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/backups/create"),
        Some("backup.create"),
    )?;
    let job = create_backup(&state.db_path, "http", actor_id(&policy_decision))
        .map_err(internal_error)?;
    record_log(
        &state.db_path,
        NewDiagnosticLogEntry {
            job_id: Some(job.id.clone()),
            capability_id: Some("backup.create".to_string()),
            event_type: Some("backup.create.completed".to_string()),
            ..diagnostic_log(
                "info",
                "backup",
                "Backup creation completed.",
                json!({ "status": job.status }),
            )
        },
    );
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "backup.create.completed",
        json!({ "jobId": job.id, "artifactId": job.artifact.as_ref().map(|artifact| artifact.id.clone()) }),
    );
    list_backup_restore_jobs(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn validate_restore_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<RestorePreflightRequest>,
) -> Result<Json<BackupRestoreResponse>, (StatusCode, Json<ErrorResponse>)> {
    let policy_decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Validate,
        ResourceRef::new(ResourceKind::DaemonRoute, "/restore/validate"),
        Some("restore.preflight.validate"),
    )?;
    let job = run_restore_preflight(&state.db_path, request, "http", actor_id(&policy_decision))
        .map_err(internal_error)?;
    record_log(
        &state.db_path,
        NewDiagnosticLogEntry {
            job_id: Some(job.id.clone()),
            capability_id: Some("restore.preflight.validate".to_string()),
            event_type: Some("restore.preflight.completed".to_string()),
            ..diagnostic_log(
                "info",
                "restore",
                "Restore preflight completed.",
                json!({ "status": job.status }),
            )
        },
    );
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "restore.preflight.completed",
        json!({ "jobId": job.id, "status": job.status }),
    );
    list_backup_restore_jobs(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn events_handler(
    State(state): State<AppState>,
    Query(query): Query<EventReplayQuery>,
) -> Result<Json<EventReplayResponse>, (StatusCode, Json<ErrorResponse>)> {
    replay_events(&state.db_path, query.after, query.limit)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn schedules_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<SchedulerOperationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/schedules"),
        Some("schedules.operations.read"),
    )?;
    read_scheduler_operations(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn surface_work_items_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<SurfaceWorkItemQuery>,
) -> Result<Json<SurfaceWorkItemListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/surface/work-items"),
        Some("surface.work_items.list"),
    )?;
    list_surface_work_items(&state.db_path, query)
        .map(Json)
        .map_err(internal_error)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoryProductionReviewQuery {
    #[serde(default)]
    audience: Option<StoryProductionReviewAudience>,
    #[serde(default, alias = "artifact_ids")]
    artifact_ids: Option<String>,
    #[serde(default, alias = "artifactId", alias = "artifact_id")]
    artifact_id: Option<String>,
    #[serde(default, alias = "deck_id")]
    deck_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoryPublishLearningQuery {
    #[serde(default)]
    audience: Option<StoryPublishLearningAudience>,
    #[serde(default, alias = "artifact_ids")]
    artifact_ids: Option<String>,
    #[serde(default, alias = "artifactId", alias = "artifact_id")]
    artifact_id: Option<String>,
    #[serde(default, alias = "deck_id")]
    deck_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeneratedContentMemoryReviewQuery {
    #[serde(default)]
    audience: Option<GeneratedContentMemoryReviewAudience>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeneratedContentMemoryDecisionResponse {
    candidate: GeneratedContentMemoryCandidateView,
    event: RealtimeEvent,
}

impl StoryProductionReviewQuery {
    fn into_request(self) -> StoryProductionReviewPacketRequest {
        let mut artifact_ids = Vec::new();
        if let Some(artifact_id) = self.artifact_id {
            push_artifact_ids(&mut artifact_ids, &artifact_id);
        }
        if let Some(artifact_ids_value) = self.artifact_ids {
            push_artifact_ids(&mut artifact_ids, &artifact_ids_value);
        }
        artifact_ids.sort();
        artifact_ids.dedup();

        StoryProductionReviewPacketRequest {
            audience: self
                .audience
                .unwrap_or(StoryProductionReviewAudience::Staff),
            artifact_ids,
            deck_id: self.deck_id,
        }
    }
}

impl StoryPublishLearningQuery {
    fn into_request(self) -> StoryPublishLearningBriefRequest {
        let mut artifact_ids = Vec::new();
        if let Some(artifact_id) = self.artifact_id {
            push_artifact_ids(&mut artifact_ids, &artifact_id);
        }
        if let Some(artifact_ids_value) = self.artifact_ids {
            push_artifact_ids(&mut artifact_ids, &artifact_ids_value);
        }
        artifact_ids.sort();
        artifact_ids.dedup();

        StoryPublishLearningBriefRequest {
            audience: self.audience.unwrap_or(StoryPublishLearningAudience::Staff),
            deck_id: self
                .deck_id
                .unwrap_or_else(|| "homepage.story.v1".to_string()),
            artifact_ids,
        }
    }
}

fn push_artifact_ids(artifact_ids: &mut Vec<String>, value: &str) {
    artifact_ids.extend(
        value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
    );
}

pub(crate) async fn studio_story_production_review_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<StoryProductionReviewQuery>,
) -> Result<Json<StoryProductionReviewPacket>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/studio/story-production-review"),
        Some("studio.story.production_review.read"),
    )?;
    let connection = rusqlite::Connection::open(state.db_path.as_ref())
        .map_err(|error| internal_error(error.into()))?;
    story_production_review_packet(&connection, query.into_request())
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn studio_story_founder_intake_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<StoryFounderIntakeInput>,
) -> Result<Json<StoryFounderIntakePacket>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/studio/story-founder-intake"),
        Some("studio.story.founder_intake.write"),
    )?;
    let mut connection = rusqlite::Connection::open(state.db_path.as_ref())
        .map_err(|error| internal_error(error.into()))?;
    let mut packet =
        record_story_founder_intake_packet(&connection, request).map_err(invalid_request_error)?;
    let idempotency_key = story_intake_workflow_idempotency_key(&packet);
    let workflow_outcome = compile_story_homepage_refresh_workflow(
        &mut connection,
        StoryHomepageRefreshCompileRequest {
            founder_intake_artifact_id: packet.artifact.id.clone(),
            publish_mode: "manual".to_string(),
            idempotency_key: idempotency_key.clone(),
        },
    )
    .map_err(invalid_request_error)?;
    packet.workflow_compilation = Some(story_intake_workflow_compilation_evidence(
        idempotency_key,
        workflow_outcome,
    ));
    if let Some(event) = packet.event.clone() {
        let _ = state.event_sender.send(event);
    }
    Ok(Json(packet))
}

fn story_intake_workflow_idempotency_key(packet: &StoryFounderIntakePacket) -> String {
    format!(
        "story-founder-intake:{}:{}:v1",
        packet.intake_id, STORY_HOMEPAGE_REFRESH_TEMPLATE_ID
    )
}

fn story_intake_workflow_compilation_evidence(
    idempotency_key: String,
    outcome: StoryHomepageRefreshCompileOutcome,
) -> StoryWorkflowCompilationEvidence {
    if let Some(compilation) = outcome.compilation {
        let plan = &compilation.safe_compiled_plan;
        let compilation_ref = format!("workflow_compilation:{}", compilation.id);
        let mut evidence_refs =
            safe_workflow_refs(plan["variables"]["storyEvidenceRefs"]["value"].as_array())
                .into_iter()
                .chain([compilation_ref.clone()])
                .collect::<Vec<_>>();
        evidence_refs.sort();
        evidence_refs.dedup();
        return StoryWorkflowCompilationEvidence {
            status: "compiled".to_string(),
            template_id: compilation.template_id,
            template_version: compilation.template_version,
            idempotency_key,
            compilation_ref: Some(compilation_ref),
            input_hash: Some(compilation.input_hash),
            evidence_refs,
            missing_inputs: Vec::new(),
            limitations: vec![
                "Workflow compilation evidence is a stored plan snapshot, not task execution."
                    .to_string(),
                "Provider calls, publication, analytics truth, graph promotion, and memory promotion remain gated."
                    .to_string(),
            ],
            safe_next_actions: vec![
                "Review compiled Story workflow state in Studio Preview.".to_string(),
                "Request owner approval before publish or provider execution.".to_string(),
            ],
            resolved_variables: workflow_resolved_variables(plan),
            task_bindings: workflow_task_bindings(plan),
            fanout_groups: workflow_fanout_groups(plan),
            approval_gates: workflow_approval_gates(plan),
            provider_requirements: workflow_provider_requirements(plan),
            live_provider_required: false,
            task_execution_performed: false,
            external_publishing_claimed: false,
            memory_promotion_performed: false,
            confirmed_graph_promotion: false,
        };
    }

    let blocker = outcome.blocker;
    let evidence_refs = blocker
        .as_ref()
        .map(|blocker| blocker.evidence_refs.clone())
        .unwrap_or_default();
    let missing_inputs = blocker
        .as_ref()
        .map(|blocker| blocker.missing.clone())
        .unwrap_or_else(|| vec!["workflow compilation prerequisites".to_string()]);
    let limitations = blocker
        .as_ref()
        .map(|blocker| blocker.limitations.clone())
        .unwrap_or_else(|| {
            vec![
                "No workflow compilation was stored while required inputs were missing."
                    .to_string(),
            ]
        });

    StoryWorkflowCompilationEvidence {
        status: "blocked".to_string(),
        template_id: STORY_HOMEPAGE_REFRESH_TEMPLATE_ID.to_string(),
        template_version: 1,
        idempotency_key,
        compilation_ref: None,
        input_hash: None,
        evidence_refs,
        missing_inputs,
        limitations,
        safe_next_actions: vec![
            "Resolve missing public-safe workflow inputs.".to_string(),
            "Keep Story workflow compilation blocked until evidence is complete.".to_string(),
        ],
        resolved_variables: Vec::new(),
        task_bindings: Vec::new(),
        fanout_groups: Vec::new(),
        approval_gates: Vec::new(),
        provider_requirements: Vec::new(),
        live_provider_required: false,
        task_execution_performed: false,
        external_publishing_claimed: false,
        memory_promotion_performed: false,
        confirmed_graph_promotion: false,
    }
}

fn safe_workflow_refs(values: Option<&Vec<serde_json::Value>>) -> Vec<String> {
    values
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .filter(|value| {
            value.starts_with("artifact:")
                || value.starts_with("business_fact:")
                || value.starts_with("offer:")
                || value.starts_with("tracked_entry_point:")
                || value.starts_with("workflow_compilation:")
        })
        .map(ToString::to_string)
        .collect()
}

fn workflow_resolved_variables(plan: &serde_json::Value) -> Vec<StoryWorkflowResolvedVariable> {
    plan["variables"]
        .as_object()
        .into_iter()
        .flat_map(|variables| variables.iter())
        .map(|(key, value)| StoryWorkflowResolvedVariable {
            key: key.clone(),
            source_kind: value["sourceKind"].as_str().unwrap_or("input").to_string(),
            visibility: value["visibility"].as_str().unwrap_or("staff").to_string(),
            evidence_ref_count: value["evidenceRefs"]
                .as_array()
                .map(|refs| refs.len())
                .unwrap_or(0),
            value_exposed: value.get("value").is_some(),
        })
        .collect()
}

fn workflow_task_bindings(plan: &serde_json::Value) -> Vec<StoryWorkflowTaskBindingEvidence> {
    plan["tasks"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|task| StoryWorkflowTaskBindingEvidence {
            key: task["key"].as_str().unwrap_or("task").to_string(),
            method: task["method"]
                .as_str()
                .unwrap_or("unknown.method")
                .to_string(),
            depends_on: task["dependsOn"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|value| value.as_str().map(ToString::to_string))
                .collect(),
            visibility: task["visibility"].as_str().unwrap_or("staff").to_string(),
            fanout: task["fanout"].as_str().map(ToString::to_string),
            provider_requirement: task["providerRequirement"]
                .as_str()
                .map(ToString::to_string),
            output_artifact_kind: task["outputArtifactKind"].as_str().map(ToString::to_string),
        })
        .collect()
}

fn workflow_fanout_groups(plan: &serde_json::Value) -> Vec<StoryWorkflowFanoutEvidence> {
    plan["fanoutGroups"]
        .as_object()
        .into_iter()
        .flat_map(|fanouts| fanouts.iter())
        .map(|(key, value)| StoryWorkflowFanoutEvidence {
            key: key.clone(),
            item_count: value["items"]
                .as_array()
                .map(|items| items.len())
                .unwrap_or(0),
            max_items: value["maxItems"].as_i64().unwrap_or(0),
        })
        .collect()
}

fn workflow_approval_gates(plan: &serde_json::Value) -> Vec<StoryWorkflowApprovalGateEvidence> {
    plan["approvalGates"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|gate| StoryWorkflowApprovalGateEvidence {
            key: gate["key"].as_str().unwrap_or("approval").to_string(),
            action: gate["action"].as_str().unwrap_or("approval").to_string(),
            required: gate["required"].as_bool().unwrap_or(true),
        })
        .collect()
}

fn workflow_provider_requirements(
    plan: &serde_json::Value,
) -> Vec<StoryWorkflowProviderRequirementEvidence> {
    plan["providerRequirements"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|provider| StoryWorkflowProviderRequirementEvidence {
            key: provider["key"].as_str().unwrap_or("provider").to_string(),
            capability: provider["capability"]
                .as_str()
                .unwrap_or("unknown.capability")
                .to_string(),
            mode: provider["mode"]
                .as_str()
                .unwrap_or("deterministic_mock")
                .to_string(),
            egress: provider["egress"].as_str().unwrap_or("none").to_string(),
            visibility: provider["visibility"]
                .as_str()
                .unwrap_or("staff")
                .to_string(),
        })
        .collect()
}

pub(crate) async fn studio_story_publish_learning_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<StoryPublishLearningQuery>,
) -> Result<Json<StoryPublishLearningBrief>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/studio/story-publish-learning"),
        Some("studio.story.publish_learning.read"),
    )?;
    let connection = rusqlite::Connection::open(state.db_path.as_ref())
        .map_err(|error| internal_error(error.into()))?;
    story_publish_learning_brief(&connection, query.into_request())
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn generated_content_memory_review_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(artifact_id): AxumPath<String>,
    Query(query): Query<GeneratedContentMemoryReviewQuery>,
) -> Result<Json<GeneratedContentMemoryReviewPacket>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/studio/generated-content-memory/{artifact_id}/review"),
        ),
        Some("memory.candidates.review"),
    )?;
    let connection = rusqlite::Connection::open(state.db_path.as_ref())
        .map_err(|error| internal_error(error.into()))?;
    generated_content_memory_review_packet_for_artifact(
        &connection,
        &artifact_id,
        query
            .audience
            .unwrap_or(GeneratedContentMemoryReviewAudience::Staff),
    )
    .map(Json)
    .map_err(internal_error)
}

pub(crate) async fn generated_content_memory_decision_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(candidate_id): AxumPath<String>,
    Json(request): Json<GeneratedContentMemoryDecisionInput>,
) -> Result<Json<GeneratedContentMemoryDecisionResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Approve,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/studio/generated-content-memory/candidates/{candidate_id}/decision"),
        ),
        Some("memory.candidates.decide"),
    )?;
    let connection = rusqlite::Connection::open(state.db_path.as_ref())
        .map_err(|error| internal_error(error.into()))?;
    let (candidate, event) =
        record_generated_content_memory_decision(&connection, &candidate_id, request)
            .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event.clone());
    Ok(Json(GeneratedContentMemoryDecisionResponse {
        candidate,
        event,
    }))
}

pub(crate) async fn studio_promo_video_package_create_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<PromoVideoPackageRequest>,
) -> Result<Json<PromoVideoPackageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/studio/promo-video-packages"),
        Some("studio.promo_video.package"),
    )?;
    let response = create_promo_video_package(&state.db_path, request, "http", actor_id(&decision))
        .map_err(invalid_request_error)?;
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "studio.promo_video.package.created",
        json!({
            "artifactId": response.package.artifact_id,
            "jobId": response.package.job_id,
            "publicationState": response.package.publication_state,
            "externalPublishing": "not_performed",
        }),
    );
    Ok(Json(response))
}

pub(crate) async fn studio_promo_video_package_review_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(artifact_id): AxumPath<String>,
    Json(request): Json<PromoVideoPackageReviewRequest>,
) -> Result<Json<PromoVideoPackageReviewResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Approve,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/studio/promo-video-packages/{artifact_id}/review"),
        ),
        Some("studio.promo_video.review"),
    )?;
    let response =
        review_promo_video_package(&state.db_path, &artifact_id, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "studio.promo_video.review.route_completed",
        json!({
            "artifactId": response.artifact_id,
            "status": response.status,
            "externalPublishing": "not_performed",
        }),
    );
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtifactPatchReviewQuery {
    review_state: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtifactPatchAcceptRequest {
    current_text: String,
}

pub(crate) async fn studio_artifact_patch_review_list_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<ArtifactPatchReviewQuery>,
) -> Result<Json<ArtifactPatchReviewListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/studio/artifact-patches"),
        Some("studio.artifact_patch.review"),
    )?;
    let connection = rusqlite::Connection::open(state.db_path.as_ref())
        .map_err(|error| internal_error(error.into()))?;
    list_artifact_patch_review_proposals(
        &connection,
        query.review_state.as_deref(),
        query.limit.unwrap_or(50),
    )
    .map(Json)
    .map_err(internal_error)
}

pub(crate) async fn studio_artifact_patch_review_read_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(proposal_id): AxumPath<String>,
) -> Result<Json<ArtifactPatchReviewResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/studio/artifact-patches/{proposal_id}"),
        ),
        Some("studio.artifact_patch.review"),
    )?;
    let connection = rusqlite::Connection::open(state.db_path.as_ref())
        .map_err(|error| internal_error(error.into()))?;
    load_artifact_patch_review_proposal(&connection, &proposal_id)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn studio_artifact_patch_accept_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(proposal_id): AxumPath<String>,
    Json(request): Json<ArtifactPatchAcceptRequest>,
) -> Result<Json<ArtifactPatchApplyResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Approve,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/studio/artifact-patches/{proposal_id}/accept"),
        ),
        Some("studio.artifact_patch.accept"),
    )?;
    let connection = rusqlite::Connection::open(state.db_path.as_ref())
        .map_err(|error| internal_error(error.into()))?;
    let response = apply_artifact_patch_review_proposal(
        &connection,
        ApplyArtifactPatchProposalInput {
            proposal_id,
            current_text: request.current_text,
            applied_by_actor_id: artifact_patch_actor_id(&decision).to_string(),
        },
    )
    .map_err(invalid_request_error)?;
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "studio.artifact_patch.accept.route_completed",
        json!({
            "artifactPatchProposalId": response.proposal.id,
            "artifactId": response.proposal.source_artifact_id,
            "acceptedVersionId": response.artifact_version.id,
        }),
    );
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CorpusReadQuery {
    viewer: Option<CorpusViewer>,
    source_id: Option<String>,
}

pub(crate) async fn corpus_sources_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<CorpusReadQuery>,
) -> Result<Json<CorpusSourceListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/corpus/sources"),
        Some("corpus.sources.list"),
    )?;
    list_corpus_sources(&state.db_path, query.viewer)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn corpus_source_read_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(source_id): AxumPath<String>,
    Query(query): Query<CorpusReadQuery>,
) -> Result<Json<CorpusSourceView>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/corpus/sources/{source_id}"),
        ),
        Some("corpus.sources.list"),
    )?;
    read_corpus_source(&state.db_path, &source_id, query.viewer)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn corpus_source_create_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<CorpusSourceWriteRequest>,
) -> Result<Json<CorpusSourceView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/corpus/sources"),
        Some("corpus.sources.write"),
    )?;
    let (source, event) = create_corpus_source(&state.db_path, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(source))
}

pub(crate) async fn corpus_source_update_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(source_id): AxumPath<String>,
    Json(request): Json<CorpusSourceWriteRequest>,
) -> Result<Json<CorpusSourceView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Update,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/corpus/sources/{source_id}"),
        ),
        Some("corpus.sources.write"),
    )?;
    let (source, event) =
        update_corpus_source(&state.db_path, &source_id, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(source))
}

pub(crate) async fn corpus_items_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<CorpusReadQuery>,
) -> Result<Json<CorpusItemListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/corpus/items"),
        Some("corpus.items.list"),
    )?;
    list_corpus_items(&state.db_path, query.source_id.as_deref(), query.viewer)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn corpus_item_read_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(item_id): AxumPath<String>,
    Query(query): Query<CorpusReadQuery>,
) -> Result<Json<CorpusItemView>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/corpus/items/{item_id}"),
        ),
        Some("corpus.items.list"),
    )?;
    read_corpus_item(&state.db_path, &item_id, query.viewer)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn corpus_item_create_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<CorpusItemWriteRequest>,
) -> Result<Json<CorpusItemView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Create,
        ResourceRef::new(ResourceKind::DaemonRoute, "/corpus/items"),
        Some("corpus.items.write"),
    )?;
    let (item, event) = create_corpus_item(&state.db_path, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(item))
}

pub(crate) async fn corpus_item_update_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(item_id): AxumPath<String>,
    Json(request): Json<CorpusItemWriteRequest>,
) -> Result<Json<CorpusItemView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Update,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/corpus/items/{item_id}"),
        ),
        Some("corpus.items.write"),
    )?;
    let (item, event) = update_corpus_item(&state.db_path, &item_id, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(item))
}

pub(crate) async fn corpus_retrieve_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<CorpusRetrievalQuery>,
) -> Result<Json<CorpusRetrievalResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Read,
        ResourceRef::new(ResourceKind::DaemonRoute, "/corpus/retrieve"),
        Some("corpus.retrieve"),
    )?;
    retrieve_corpus(&state.db_path, request)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn answer_drafts_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<AnswerDraftListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/answer-drafts"),
        Some("answer.drafts.list"),
    )?;
    list_answer_drafts(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn answer_draft_read_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(draft_id): AxumPath<String>,
) -> Result<Json<AnswerDraftResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/answer-drafts/{draft_id}"),
        ),
        Some("answer.drafts.list"),
    )?;
    read_answer_draft(&state.db_path, &draft_id)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn answer_draft_prepare_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<AnswerDraftRequest>,
) -> Result<Json<AnswerDraftResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Prepare,
        ResourceRef::new(ResourceKind::DaemonRoute, "/answer-drafts"),
        Some("answer.drafts.prepare"),
    )?;
    let response = prepare_answer_draft(&state.db_path, request, "http", actor_id(&decision))
        .map_err(invalid_request_error)?;
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "answer.draft.prepared",
        json!({
            "draftId": response.draft.id,
            "status": response.draft.status,
            "citedItemIds": response.draft.cited_item_ids,
            "providerCall": "not_performed",
        }),
    );
    Ok(Json(response))
}

pub(crate) async fn mcp_packs_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<McpPackListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/mcp/packs"),
        Some("mcp.packs.list"),
    )?;
    list_mcp_packs(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn mcp_pack_read_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(pack_id): AxumPath<String>,
) -> Result<Json<McpPackResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, format!("/mcp/packs/{pack_id}")),
        Some("mcp.packs.list"),
    )?;
    read_mcp_pack(&state.db_path, &pack_id)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn mcp_pack_install_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<McpPackInstallRequest>,
) -> Result<Json<McpPackResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Validate,
        ResourceRef::new(ResourceKind::DaemonRoute, "/mcp/packs"),
        Some("mcp.packs.write"),
    )?;
    let response = install_mcp_pack(&state.db_path, request, "http", actor_id(&decision))
        .map_err(invalid_request_error)?;
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "mcp.pack.installed",
        json!({
            "packId": response.pack.id,
            "status": response.pack.status,
            "toolCount": response.pack.tools.len(),
        }),
    );
    Ok(Json(response))
}

pub(crate) async fn mcp_pack_disable_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(pack_id): AxumPath<String>,
) -> Result<Json<McpPackResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Update,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/mcp/packs/{pack_id}/disable"),
        ),
        Some("mcp.packs.write"),
    )?;
    let response = disable_mcp_pack(&state.db_path, &pack_id, "http", actor_id(&decision))
        .map_err(invalid_request_error)?;
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "mcp.pack.disabled",
        json!({
            "packId": response.pack.id,
            "status": response.pack.status,
            "toolCount": response.pack.tools.len(),
        }),
    );
    Ok(Json(response))
}

pub(crate) async fn product_packs_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<ProductPackListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/product-packs"),
        Some("product_packs.list"),
    )?;
    list_product_packs(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn product_pack_read_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(pack_id): AxumPath<String>,
) -> Result<Json<ProductPackResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/product-packs/{pack_id}"),
        ),
        Some("product_packs.list"),
    )?;
    read_product_pack(&state.db_path, &pack_id)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn product_pack_install_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<ProductPackInstallRequest>,
) -> Result<Json<ProductPackResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Validate,
        ResourceRef::new(ResourceKind::DaemonRoute, "/product-packs"),
        Some("product_packs.write"),
    )?;
    install_product_pack(&state.db_path, request, "http", actor_id(&decision))
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn product_pack_disable_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(pack_id): AxumPath<String>,
) -> Result<Json<ProductPackResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Update,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/product-packs/{pack_id}/disable"),
        ),
        Some("product_packs.write"),
    )?;
    disable_product_pack(&state.db_path, &pack_id, "http", actor_id(&decision))
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn list_issue_reports_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<IssueReportsResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/reports/issues"),
        Some("issue.report.list"),
    )?;
    list_issue_reports(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn read_issue_report_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(report_id): AxumPath<String>,
) -> Result<Json<IssueReportDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/reports/issues/{report_id}"),
        ),
        Some("issue.report.detail"),
    )?;
    read_issue_report(&state.db_path, &report_id)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn update_issue_report_status_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(report_id): AxumPath<String>,
    Json(request): Json<IssueReportStatusUpdateRequest>,
) -> Result<Json<IssueReportDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Update,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/reports/issues/{report_id}/status"),
        ),
        Some("issue.report.status.update"),
    )?;
    let detail =
        update_issue_report_status(&state.db_path, &report_id, request, actor_id(&decision))
            .map_err(invalid_request_error)?;
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "issue.report.status.updated",
        json!({ "reportId": detail.report.id, "status": detail.report.status }),
    );
    Ok(Json(detail))
}

pub(crate) async fn export_issue_report_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(report_id): AxumPath<String>,
    Json(request): Json<IssueReportExportRequest>,
) -> Result<Json<IssueReportExportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Export,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/reports/issues/{report_id}/exports"),
        ),
        Some("issue.report.export"),
    )?;
    let exported = export_issue_report(&state.db_path, &report_id, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "issue.report.exported",
        json!({
            "reportId": exported.report.id,
            "exportId": exported.export.id,
            "contentHash": exported.export.content_hash,
        }),
    );
    Ok(Json(exported))
}

pub(crate) async fn prepare_issue_report_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<IssueReportPrepareRequest>,
) -> Result<Json<IssueReportsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let policy_decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Prepare,
        ResourceRef::new(ResourceKind::DaemonRoute, "/reports/issues/prepare"),
        Some("issue.report.prepare"),
    )?;
    let report = prepare_issue_report(&state.db_path, request, "http", actor_id(&policy_decision))
        .map_err(internal_error)?;
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "issue.report.prepared",
        json!({
            "reportId": report.id,
            "jobId": report.job_id,
            "severity": report.severity,
            "status": report.status,
        }),
    );
    list_issue_reports(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn support_packets_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<SupportPacketListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(ResourceKind::DaemonRoute, "/support-packets"),
        Some("support.packets.list"),
    )?;
    list_support_packets(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

pub(crate) async fn draft_support_packet_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<SupportPacketDraftRequest>,
) -> Result<Json<SupportPacketView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Prepare,
        ResourceRef::new(ResourceKind::DaemonRoute, "/support-packets"),
        Some("support.packets.draft"),
    )?;
    let packet = draft_support_packet(&state.db_path, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "support.packet.drafted",
        json!({
            "packetId": packet.id,
            "reportId": packet.report_id,
            "externalDelivery": false,
            "approvalRequired": packet.approval_required,
        }),
    );
    Ok(Json(packet))
}

pub(crate) async fn approve_support_packet_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(packet_id): AxumPath<String>,
    Json(request): Json<SupportPacketApprovalRequest>,
) -> Result<Json<SupportPacketView>, (StatusCode, Json<ErrorResponse>)> {
    let decision = authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Approve,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/support-packets/{packet_id}/approve"),
        ),
        Some("support.packets.approve"),
    )?;
    let packet = approve_support_packet(&state.db_path, &packet_id, request, actor_id(&decision))
        .map_err(invalid_request_error)?;
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "support.packet.approved.local_only",
        json!({
            "packetId": packet.id,
            "reportId": packet.report_id,
            "externalDelivery": false,
            "deliveryState": "not_sent",
        }),
    );
    Ok(Json(packet))
}

pub(crate) async fn support_packet_receipts_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AxumPath(packet_id): AxumPath<String>,
) -> Result<Json<SupportPacketReceiptListResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Inspect,
        ResourceRef::new(
            ResourceKind::DaemonRoute,
            format!("/support-packets/{packet_id}/receipts"),
        ),
        Some("support.packet.receipts.list"),
    )?;
    list_support_packet_receipts(&state.db_path, &packet_id)
        .map(Json)
        .map_err(invalid_request_error)
}

pub(crate) async fn mcp_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    request_body: String,
) -> Result<Json<McpResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::CallTool,
        ResourceRef::new(ResourceKind::DaemonRoute, "/mcp"),
        None,
    )?;
    Ok(Json(handle_mcp_json(&state.db_path, &request_body)))
}

pub(crate) fn authorize_protected_daemon_route(
    policy: &DaemonAccessPolicy,
    db_path: &Path,
    headers: &HeaderMap,
    remote_addr: SocketAddr,
    action: PolicyAction,
    resource: ResourceRef,
    capability_id: Option<&str>,
) -> Result<PolicyDecision, (StatusCode, Json<ErrorResponse>)> {
    let decision = protected_daemon_route_decision(
        policy,
        headers,
        remote_addr,
        action,
        resource,
        capability_id,
    );
    record_protected_policy_decision(db_path, &decision);
    if decision.allowed() {
        Ok(decision)
    } else {
        Err(forbidden_error(&decision.reason))
    }
}

fn record_protected_policy_decision(db_path: &Path, decision: &PolicyDecision) {
    if let Ok(connection) = rusqlite::Connection::open(db_path) {
        let _ = record_policy_decision(&connection, decision, PolicyDecisionCorrelation::default());
    }
}

pub(crate) fn protected_daemon_route_decision(
    policy: &DaemonAccessPolicy,
    headers: &HeaderMap,
    remote_addr: SocketAddr,
    action: PolicyAction,
    resource: ResourceRef,
    capability_id: Option<&str>,
) -> PolicyDecision {
    let loopback = protected_route_loopback_or_trusted_docker_host(remote_addr.ip());
    let token = policy
        .access_token
        .as_ref()
        .is_some_and(|token| request_has_access_token(headers, token));
    if !loopback && !token {
        let rate_limit_key = protected_route_rate_limit_key(remote_addr.ip(), &resource);
        let rate_limit = policy.rate_limiter.check(&rate_limit_key);
        if !rate_limit.allowed {
            return PolicyDecision {
                outcome: PolicyOutcome::Denied,
                actor: ActorContext::local_owner("http"),
                action,
                resource,
                capability_id: capability_id.map(ToString::to_string),
                reason: format!(
                    "Protected daemon route rate limit exceeded. Retry after {} second(s).",
                    rate_limit.retry_after_seconds.unwrap_or(1)
                ),
            };
        }
    }
    authorize_protected_daemon_action(
        ActorContext::local_owner("http"),
        action,
        resource,
        capability_id,
        ProtectedAccessEvidence { loopback, token },
    )
}

fn protected_route_loopback_or_trusted_docker_host(ip: IpAddr) -> bool {
    if ip.is_loopback() {
        return true;
    }
    if std::env::var("ORDO_DAEMON_TRUST_DOCKER_HOST")
        .ok()
        .as_deref()
        != Some("1")
    {
        return false;
    }
    match ip {
        IpAddr::V4(ip) => ip.is_private(),
        IpAddr::V6(ip) => (ip.segments()[0] & 0xfe00) == 0xfc00,
    }
}

fn protected_route_rate_limit_key(ip: IpAddr, resource: &ResourceRef) -> String {
    format!("{}|{}|{}", ip, resource.kind.as_str(), resource.id)
}

#[cfg(test)]
fn protected_daemon_route_allowed(
    policy: &DaemonAccessPolicy,
    headers: &HeaderMap,
    remote_addr: SocketAddr,
) -> bool {
    protected_daemon_route_decision(
        policy,
        headers,
        remote_addr,
        PolicyAction::Execute,
        ResourceRef::new(ResourceKind::DaemonRoute, "test"),
        None,
    )
    .allowed()
}

fn actor_id(decision: &PolicyDecision) -> Option<&str> {
    Some(decision.actor.kind.as_str())
}

fn artifact_patch_actor_id(decision: &PolicyDecision) -> &'static str {
    match decision.actor.kind {
        crate::policy::ActorKind::LocalOwner | crate::policy::ActorKind::BrowserOperator => {
            "owner:local_owner"
        }
        crate::policy::ActorKind::System | crate::policy::ActorKind::Scheduler => "system",
        crate::policy::ActorKind::McpClient => "staff:mcp_client",
    }
}

fn request_has_access_token(headers: &HeaderMap, expected_token: &OrdoSecretString) -> bool {
    headers
        .get(DAEMON_ACCESS_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|token| constant_time_secret_eq(token, expected_token))
        || headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .is_some_and(|token| constant_time_secret_eq(token, expected_token))
}

pub(crate) fn internal_error(error: anyhow::Error) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new(
            DaemonErrorCode::Internal,
            error.to_string(),
        )),
    )
}

fn forbidden_error(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse::new(DaemonErrorCode::Forbidden, message)),
    )
}

pub(crate) fn invalid_request_error(error: anyhow::Error) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::new(
            DaemonErrorCode::InvalidRequest,
            error.to_string(),
        )),
    )
}

pub(crate) fn record_log(db_path: &Path, entry: NewDiagnosticLogEntry) {
    let _ = record_diagnostic_log(db_path, entry);
}

pub(crate) fn emit_system_event(
    db_path: &Path,
    event_sender: &broadcast::Sender<RealtimeEvent>,
    event_type: &str,
    payload: serde_json::Value,
) {
    let event = append_system_event(db_path, event_type, payload).unwrap_or_else(|error| {
        system_event(
            "system.event_persist_failed",
            json!({ "eventType": event_type, "message": error.to_string() }),
        )
    });
    let _ = event_sender.send(event);
}

pub(crate) async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.event_sender.subscribe()))
}

pub(crate) async fn chat_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
) -> Response {
    match authorize_protected_daemon_route(
        &state.access_policy,
        &state.db_path,
        &headers,
        remote_addr,
        PolicyAction::Read,
        ResourceRef::new(ResourceKind::DaemonRoute, "/chat/ws"),
        Some("conversation.read"),
    ) {
        Ok(_) => ws
            .on_upgrade(move |socket| {
                handle_conversation_socket(
                    socket,
                    state.db_path.clone(),
                    state.conversation_sender.clone(),
                )
            })
            .into_response(),
        Err(error) => error.into_response(),
    }
}

pub(crate) async fn handle_socket(
    mut socket: WebSocket,
    mut event_receiver: broadcast::Receiver<RealtimeEvent>,
) {
    let connected = system_event("websocket.connected", json!({ "transport": "websocket" }));
    if send_event(&mut socket, &connected).await.is_err() {
        return;
    }

    loop {
        match event_receiver.recv().await {
            Ok(event) => {
                if send_event(&mut socket, &event).await.is_err() {
                    return;
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                let lagged = system_event("websocket.lagged", json!({ "skipped": skipped }));
                if send_event(&mut socket, &lagged).await.is_err() {
                    return;
                }
            }
            Err(broadcast::error::RecvError::Closed) => return,
        }
    }
}

pub(crate) async fn send_event(
    socket: &mut WebSocket,
    event: &RealtimeEvent,
) -> Result<(), axum::Error> {
    socket
        .send(Message::Text(
            serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string()),
        ))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts::{record_artifact, ArtifactInput};
    use crate::capabilities::built_in_capabilities;
    use crate::generated_content_memory::{
        ingest_generated_content_memory_candidates, GeneratedContentMemoryIngestionInput,
        GeneratedContentMemoryItemInput, GeneratedContentMemoryKind, GeneratedContentMemoryState,
    };
    use crate::route_contracts::{HttpMethod, RouteProtection, DAEMON_ROUTE_CONTRACTS};
    use crate::schema::init_database;
    use crate::story_intake_artifacts::{
        StoryFounderIntakeInput, StoryIntakeClaimInput, STORY_FOUNDER_INTAKE_ARTIFACT_KIND,
    };
    use crate::story_publish_approvals::STORY_HOMEPAGE_PUBLISH_APPROVAL_PACKAGE_ARTIFACT_KIND;
    use std::collections::BTreeSet;
    use std::sync::{Arc, Mutex};

    fn socket_addr(value: &str) -> SocketAddr {
        value.parse().unwrap()
    }

    #[test]
    fn next_restart_policy_is_bounded() {
        assert!(should_restart_next_child(0, 3));
        assert!(should_restart_next_child(2, 3));
        assert!(!should_restart_next_child(3, 3));
    }

    #[test]
    fn next_readiness_is_ok_when_child_is_running() {
        let next_status = Arc::new(Mutex::new(NextSupervisorStatus {
            phase: NextSupervisorPhase::Running,
            pid: Some(123),
            restart_count: 1,
            detail: "Next.js child process is running with pid 123.".to_string(),
        }));

        let check = next_supervisor_readiness_check(&next_status);

        assert_eq!(check.name, "next");
        assert_eq!(check.status, "ok");
        assert!(check.detail.contains("pid 123"));
    }

    #[test]
    fn next_readiness_fails_when_restart_budget_is_exhausted() {
        let next_status = Arc::new(Mutex::new(NextSupervisorStatus {
            phase: NextSupervisorPhase::Failed,
            pid: None,
            restart_count: 3,
            detail: "Restart budget exhausted after 3 attempts.".to_string(),
        }));

        let check = next_supervisor_readiness_check(&next_status);

        assert_eq!(check.name, "next");
        assert_eq!(check.status, "error");
        assert!(check.detail.contains("exhausted"));
    }

    #[test]
    fn next_readiness_fails_while_child_is_restarting() {
        let next_status = Arc::new(Mutex::new(NextSupervisorStatus {
            phase: NextSupervisorPhase::Restarting,
            pid: None,
            restart_count: 1,
            detail: "Restart attempt 1 of 3 is scheduled.".to_string(),
        }));

        let check = next_supervisor_readiness_check(&next_status);

        assert_eq!(check.name, "next");
        assert_eq!(check.status, "error");
        assert!(check.detail.contains("Restart attempt"));
    }

    #[test]
    fn daemon_access_policy_ignores_empty_tokens() {
        let policy = DaemonAccessPolicy::new(Some("  ".to_string()));

        assert!(policy.access_token.is_none());
    }

    #[test]
    fn protected_daemon_routes_allow_loopback_without_token() {
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        assert!(protected_daemon_route_allowed(
            &policy,
            &headers,
            socket_addr("127.0.0.1:4000")
        ));
    }

    #[test]
    fn protected_daemon_routes_deny_non_loopback_without_token() {
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        assert!(!protected_daemon_route_allowed(
            &policy,
            &headers,
            socket_addr("192.168.1.10:4000")
        ));
    }

    #[test]
    fn protected_daemon_routes_allow_bearer_token_for_non_loopback() {
        let policy = DaemonAccessPolicy::new(Some("secret".to_string()));
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer secret".parse().unwrap());

        assert!(protected_daemon_route_allowed(
            &policy,
            &headers,
            socket_addr("192.168.1.10:4000")
        ));
    }

    #[test]
    fn protected_daemon_routes_allow_header_token_for_non_loopback() {
        let policy = DaemonAccessPolicy::new(Some("secret".to_string()));
        let mut headers = HeaderMap::new();
        headers.insert(DAEMON_ACCESS_TOKEN_HEADER, "secret".parse().unwrap());

        assert!(protected_daemon_route_allowed(
            &policy,
            &headers,
            socket_addr("192.168.1.10:4000")
        ));
    }

    #[test]
    fn protected_daemon_routes_reject_wrong_token_without_leaking_secret() {
        let policy = DaemonAccessPolicy::new(Some("super-secret-daemon-token".to_string()));
        let mut headers = HeaderMap::new();
        headers.insert(
            DAEMON_ACCESS_TOKEN_HEADER,
            "attacker-supplied-token".parse().unwrap(),
        );

        let decision = protected_daemon_route_decision(
            &policy,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/providers"),
            Some("providers.list"),
        );

        assert!(!decision.allowed());
        let serialized = format!("{policy:?}\n{}", decision.metadata());
        assert!(!serialized.contains("super-secret-daemon-token"));
        assert!(!serialized.contains("attacker-supplied-token"));
    }

    #[test]
    fn protected_daemon_routes_rate_limit_repeated_bad_non_loopback_attempts() {
        let mut policy = DaemonAccessPolicy::new(None);
        policy.rate_limiter = ProtectedRouteRateLimiter::new(2, 60);
        let headers = HeaderMap::new();

        for _ in 0..2 {
            let decision = protected_daemon_route_decision(
                &policy,
                &headers,
                socket_addr("192.168.1.10:4000"),
                PolicyAction::Inspect,
                ResourceRef::new(ResourceKind::DaemonRoute, "/providers"),
                Some("providers.list"),
            );
            assert!(!decision.allowed());
            assert!(decision.reason.contains("requires loopback"));
        }
        let blocked = protected_daemon_route_decision(
            &policy,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/providers"),
            Some("providers.list"),
        );

        assert!(!blocked.allowed());
        assert!(blocked.reason.contains("rate limit exceeded"));
        assert!(!blocked.reason.contains("192.168.1.10"));
    }

    #[test]
    fn protected_daemon_route_limiter_does_not_block_valid_token() {
        let mut policy = DaemonAccessPolicy::new(Some("super-secret-daemon-token".to_string()));
        policy.rate_limiter = ProtectedRouteRateLimiter::new(1, 60);
        let empty_headers = HeaderMap::new();
        let mut token_headers = HeaderMap::new();
        token_headers.insert(
            DAEMON_ACCESS_TOKEN_HEADER,
            "super-secret-daemon-token".parse().unwrap(),
        );

        let denied = protected_daemon_route_decision(
            &policy,
            &empty_headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/providers"),
            Some("providers.list"),
        );
        assert!(!denied.allowed());
        let allowed = protected_daemon_route_decision(
            &policy,
            &token_headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/providers"),
            Some("providers.list"),
        );

        assert!(allowed.allowed());
    }

    #[test]
    fn daemon_route_contracts_cover_every_registered_route_path() {
        let source = include_str!("mod.rs");
        let mut registered_paths = BTreeSet::new();
        let mut cursor = 0;
        while let Some(route_start) = source[cursor..].find(".route(") {
            let after_route = &source[cursor + route_start..];
            let first_quote = after_route
                .find('"')
                .expect("registered daemon routes use string literal paths");
            let after_first_quote = &after_route[first_quote + 1..];
            let second_quote = after_first_quote
                .find('"')
                .expect("registered daemon routes use closed string literal paths");
            let path = &after_first_quote[..second_quote];
            if path.starts_with('/') {
                registered_paths.insert(path.to_string());
            }
            cursor += route_start + first_quote + second_quote + 2;
        }
        let contract_paths = DAEMON_ROUTE_CONTRACTS
            .iter()
            .map(|contract| contract.pattern.to_string())
            .collect::<BTreeSet<_>>();

        assert_eq!(registered_paths, contract_paths);
    }

    #[test]
    fn protected_route_contracts_use_the_daemon_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let empty_headers = HeaderMap::new();
        let token_policy = DaemonAccessPolicy::new(Some("secret".to_string()));
        let mut token_headers = HeaderMap::new();
        token_headers.insert(DAEMON_ACCESS_TOKEN_HEADER, "secret".parse().unwrap());

        let protected_contracts = DAEMON_ROUTE_CONTRACTS
            .iter()
            .filter_map(|contract| match contract.protection {
                RouteProtection::Protected {
                    action,
                    capability_id,
                } => Some((contract, action, capability_id)),
                _ => None,
            })
            .collect::<Vec<_>>();

        for (contract, action, capability_id) in &protected_contracts {
            let denied = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &empty_headers,
                socket_addr("192.168.1.10:4000"),
                *action,
                ResourceRef::new(ResourceKind::DaemonRoute, contract.sample_route),
                Some(*capability_id),
            );
            assert!(
                denied.is_err(),
                "{} {} should deny non-loopback without token",
                contract.method.as_str(),
                contract.pattern
            );

            let loopback_allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &empty_headers,
                socket_addr("127.0.0.1:4000"),
                *action,
                ResourceRef::new(ResourceKind::DaemonRoute, contract.sample_route),
                Some(*capability_id),
            );
            assert!(
                loopback_allowed.is_ok(),
                "{} {} should allow loopback appliance access",
                contract.method.as_str(),
                contract.pattern
            );

            let token_allowed = authorize_protected_daemon_route(
                &token_policy,
                &db_path,
                &token_headers,
                socket_addr("192.168.1.10:4000"),
                *action,
                ResourceRef::new(ResourceKind::DaemonRoute, contract.sample_route),
                Some(*capability_id),
            );
            assert!(
                token_allowed.is_ok(),
                "{} {} should allow configured daemon token access",
                contract.method.as_str(),
                contract.pattern
            );
        }

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM policy_decisions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(audit_count, (protected_contracts.len() * 3) as i64);
    }

    #[test]
    fn policy_decision_query_route_uses_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/policy-decisions"),
            Some("policy.decisions.list"),
        );
        assert!(denied.is_err());

        let allowed = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/policy-decisions"),
            Some("policy.decisions.list"),
        );
        assert!(allowed.is_ok());

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id = 'policy.decisions.list'
                   AND resource_id = '/policy-decisions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 2);
    }

    #[test]
    fn chat_ws_route_uses_protected_conversation_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::DaemonRoute, "/chat/ws"),
            Some("conversation.read"),
        );
        assert!(denied.is_err());

        let loopback_allowed = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::DaemonRoute, "/chat/ws"),
            Some("conversation.read"),
        );
        assert!(loopback_allowed.is_ok());

        let token_policy = DaemonAccessPolicy::new(Some("secret".to_string()));
        let mut token_headers = HeaderMap::new();
        token_headers.insert(DAEMON_ACCESS_TOKEN_HEADER, "secret".parse().unwrap());
        let token_allowed = authorize_protected_daemon_route(
            &token_policy,
            &db_path,
            &token_headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Read,
            ResourceRef::new(ResourceKind::DaemonRoute, "/chat/ws"),
            Some("conversation.read"),
        );
        assert!(token_allowed.is_ok());
    }

    #[test]
    fn diagnostic_log_route_uses_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/logs"),
            Some("diagnostic.logs.list"),
        );
        assert!(denied.is_err());

        let allowed = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/logs"),
            Some("diagnostic.logs.list"),
        );
        assert!(allowed.is_ok());

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id = 'diagnostic.logs.list'
                   AND resource_id = '/logs'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 2);
    }

    #[test]
    fn install_and_provider_mutations_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/install/complete"),
            Some("install.complete"),
        );
        assert!(denied.is_err());

        let allowed = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/providers/anthropic"),
            Some("providers.update"),
        );
        assert!(allowed.is_ok());

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN ('install.complete', 'providers.update')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 2);
    }

    #[test]
    fn business_fact_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/business/facts"),
            Some("business.facts.write"),
        );
        assert!(denied.is_err());

        let allowed = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/business/facts"),
            Some("business.facts.list"),
        );
        assert!(allowed.is_ok());

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN ('business.facts.write', 'business.facts.list')
                   AND resource_id = '/business/facts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 2);
    }

    #[test]
    fn entry_point_management_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/entry-points"),
            Some("entry_points.write"),
        );
        assert!(denied.is_err());

        let allowed_entry_points = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/entry-points"),
            Some("entry_points.list"),
        );
        assert!(allowed_entry_points.is_ok());

        let allowed_sessions = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/visitor-sessions"),
            Some("visitor_sessions.list"),
        );
        assert!(allowed_sessions.is_ok());

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN ('entry_points.write', 'entry_points.list', 'visitor_sessions.list')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 3);
    }

    #[test]
    fn offer_and_trial_management_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/offers"),
            Some("offers.write"),
        );
        assert!(denied.is_err());

        for (route, capability) in [
            ("/offers", "offers.list"),
            ("/offer-acceptances", "offer_acceptances.list"),
            ("/trials", "trials.list"),
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                PolicyAction::Inspect,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(allowed.is_ok());
        }

        let transition_allowed = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/trials/trial_1/status"),
            Some("trials.transition"),
        );
        assert!(transition_allowed.is_ok());

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN (
                    'offers.write', 'offers.list', 'offer_acceptances.list',
                    'trials.list', 'trials.transition'
                 )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 5);
    }

    #[test]
    fn offer_builder_routes_use_protected_access_boundary() {
        let inspect_contract = DAEMON_ROUTE_CONTRACTS
            .iter()
            .find(|contract| contract.pattern == "/offer-builder")
            .expect("offer builder inspect route contract");
        assert_eq!(inspect_contract.sample_route, "/offer-builder");
        assert!(matches!(
            inspect_contract.protection,
            RouteProtection::Protected {
                action: PolicyAction::Inspect,
                capability_id: "offer_builder.inspect",
            }
        ));

        let write_contract = DAEMON_ROUTE_CONTRACTS
            .iter()
            .find(|contract| {
                contract.pattern == "/offer-builder" && contract.method == HttpMethod::Post
            })
            .expect("offer builder write route contract");
        assert!(matches!(
            write_contract.protection,
            RouteProtection::Protected {
                action: PolicyAction::Create,
                capability_id: "offer_builder.write",
            }
        ));
    }

    #[test]
    fn hosted_trial_operations_routes_are_protected_by_system_capabilities() {
        let capacity_contract = DAEMON_ROUTE_CONTRACTS
            .iter()
            .find(|contract| contract.pattern == "/hosted-trials/capacity")
            .expect("hosted trial capacity route contract");
        assert_eq!(capacity_contract.sample_route, "/hosted-trials/capacity");
        assert!(matches!(
            capacity_contract.protection,
            RouteProtection::Protected {
                action: PolicyAction::Inspect,
                capability_id: "hosted_trials.capacity.inspect",
            }
        ));

        let reset_contract = DAEMON_ROUTE_CONTRACTS
            .iter()
            .find(|contract| contract.pattern == "/hosted-trials/:trial_id/reset-ready")
            .expect("hosted trial reset-readiness route contract");
        assert_eq!(
            reset_contract.sample_route,
            "/hosted-trials/trial_1/reset-ready"
        );
        assert!(matches!(
            reset_contract.protection,
            RouteProtection::Protected {
                action: PolicyAction::Validate,
                capability_id: "hosted_trials.reset_ready.validate",
            }
        ));
    }

    #[test]
    fn connection_management_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/connections"),
            Some("connections.write"),
        );
        assert!(denied.is_err());

        for (route, capability) in [
            ("/connections", "connections.list"),
            ("/connections/connection_1/grants", "connection_grants.list"),
            ("/connections/connection_1/events", "connection_events.list"),
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                PolicyAction::Inspect,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(allowed.is_ok());
        }

        let grant_allowed = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Create,
            ResourceRef::new(
                ResourceKind::DaemonRoute,
                "/connections/connection_1/grants",
            ),
            Some("connection_grants.write"),
        );
        assert!(grant_allowed.is_ok());

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN (
                    'connections.write', 'connections.list', 'connection_grants.list',
                    'connection_grants.write', 'connection_events.list'
                 )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 5);
    }

    #[test]
    fn availability_and_handoff_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/handoff/inbox"),
            Some("handoff.inbox.write"),
        );
        assert!(denied.is_err());

        for (route, capability) in [
            ("/availability", "availability.read"),
            ("/handoff/eligibility", "handoff.eligibility.evaluate"),
            ("/handoff/inbox", "handoff.inbox.list"),
            ("/handoff/inbox/handoff_item_1", "handoff.inbox.list"),
            (
                "/handoff/inbox/handoff_item_1/receipts",
                "handoff.receipts.list",
            ),
            (
                "/strategy-sessions/handoff_item_1/status",
                "strategy_sessions.status.read",
            ),
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                PolicyAction::Inspect,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(allowed.is_ok());
        }

        for route in [
            "/availability/schedule",
            "/availability/presence",
            "/handoff/inbox/handoff_item_1",
            "/handoff/inbox/handoff_item_1/resolve",
            "/strategy-sessions/request",
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                if route == "/handoff/inbox/handoff_item_1" {
                    PolicyAction::Update
                } else {
                    PolicyAction::Create
                },
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(if route.starts_with("/availability") {
                    "availability.write"
                } else if route.starts_with("/strategy-sessions") {
                    "strategy_sessions.request"
                } else {
                    "handoff.inbox.write"
                }),
            );
            assert!(allowed.is_ok());
        }

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN (
                    'handoff.inbox.write', 'availability.read', 'availability.write',
                    'handoff.eligibility.evaluate', 'handoff.inbox.list', 'handoff.receipts.list',
                    'strategy_sessions.request', 'strategy_sessions.status.read'
                 )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 12);
    }

    #[test]
    fn feedback_request_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/feedback/requests"),
            Some("feedback.requests.write"),
        );
        assert!(denied.is_err());

        for (route, action, capability) in [
            (
                "/feedback/requests",
                PolicyAction::Inspect,
                "feedback.requests.list",
            ),
            (
                "/feedback/requests",
                PolicyAction::Create,
                "feedback.requests.write",
            ),
            (
                "/feedback/requests/feedback_request_1/respond",
                PolicyAction::Create,
                "feedback.requests.respond",
            ),
            (
                "/feedback/requests/feedback_request_1/review",
                PolicyAction::Approve,
                "feedback.requests.review",
            ),
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                action,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(
                allowed.is_ok(),
                "{route} should be protected but usable locally"
            );
        }

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN (
                    'feedback.requests.write',
                    'feedback.requests.list',
                    'feedback.requests.respond',
                    'feedback.requests.review'
                 )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 5);
    }

    #[test]
    fn reward_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Approve,
            ResourceRef::new(
                ResourceKind::DaemonRoute,
                "/rewards/referrals/referral_1/qualify",
            ),
            Some("rewards.qualify"),
        );
        assert!(denied.is_err());

        for (route, action, capability) in [
            ("/rewards", PolicyAction::Inspect, "rewards.list"),
            (
                "/rewards/referrals/referral_1/qualify",
                PolicyAction::Approve,
                "rewards.qualify",
            ),
            (
                "/rewards/feedback/feedback_reward_eligibility_1/qualify",
                PolicyAction::Approve,
                "rewards.qualify",
            ),
            (
                "/rewards/events/reward_event_1/status",
                PolicyAction::Update,
                "rewards.update",
            ),
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                action,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(
                allowed.is_ok(),
                "{route} should be protected but usable locally"
            );
        }

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN ('rewards.list', 'rewards.qualify', 'rewards.update')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 5);
    }

    #[test]
    fn growth_pilot_report_route_uses_protected_access_boundary() {
        let contract = DAEMON_ROUTE_CONTRACTS
            .iter()
            .find(|contract| contract.pattern == "/growth/pilot-report")
            .expect("growth pilot report route contract");
        assert_eq!(contract.sample_route, "/growth/pilot-report");
        assert!(matches!(
            contract.protection,
            RouteProtection::Protected {
                action: PolicyAction::Inspect,
                capability_id: "growth.pilot_report.read",
            }
        ));

        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/growth/pilot-report"),
            Some("growth.pilot_report.read"),
        );
        assert!(denied.is_err());

        let allowed = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/growth/pilot-report"),
            Some("growth.pilot_report.read"),
        );
        assert!(allowed.is_ok());

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id = 'growth.pilot_report.read'
                   AND resource_id = '/growth/pilot-report'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 2);
    }

    #[test]
    fn surface_work_items_route_uses_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/surface/work-items"),
            Some("surface.work_items.list"),
        );
        assert!(denied.is_err());

        let allowed = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/surface/work-items"),
            Some("surface.work_items.list"),
        );
        assert!(allowed.is_ok());

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id = 'surface.work_items.list'
                   AND resource_id = '/surface/work-items'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 2);
    }

    #[test]
    fn story_production_review_route_uses_protected_access_boundary() {
        let contract = DAEMON_ROUTE_CONTRACTS
            .iter()
            .find(|contract| contract.pattern == "/studio/story-production-review")
            .expect("story production review route contract");
        assert_eq!(contract.sample_route, "/studio/story-production-review");
        assert!(matches!(
            contract.protection,
            RouteProtection::Protected {
                action: PolicyAction::Inspect,
                capability_id: "studio.story.production_review.read",
            }
        ));

        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/studio/story-production-review"),
            Some("studio.story.production_review.read"),
        );
        assert!(denied.is_err());

        let allowed = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/studio/story-production-review"),
            Some("studio.story.production_review.read"),
        );
        assert!(allowed.is_ok());

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id = 'studio.story.production_review.read'
                   AND resource_id = '/studio/story-production-review'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 2);
    }

    #[test]
    fn story_publish_learning_route_uses_protected_access_boundary() {
        let contract = DAEMON_ROUTE_CONTRACTS
            .iter()
            .find(|contract| contract.pattern == "/studio/story-publish-learning")
            .expect("story publish learning route contract");
        assert_eq!(contract.sample_route, "/studio/story-publish-learning");
        assert!(matches!(
            contract.protection,
            RouteProtection::Protected {
                action: PolicyAction::Inspect,
                capability_id: "studio.story.publish_learning.read",
            }
        ));

        let capability = built_in_capabilities()
            .into_iter()
            .find(|capability| capability.id == "studio.story.publish_learning.read")
            .expect("story publish learning capability");
        assert_eq!(capability.family, "studio");

        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/studio/story-publish-learning"),
            Some("studio.story.publish_learning.read"),
        );
        assert!(denied.is_err());

        let allowed = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/studio/story-publish-learning"),
            Some("studio.story.publish_learning.read"),
        );
        assert!(allowed.is_ok());

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id = 'studio.story.publish_learning.read'
                   AND resource_id = '/studio/story-publish-learning'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 2);
    }

    #[test]
    fn studio_story_intake_story_founder_intake_route_uses_protected_access_boundary() {
        let contract = DAEMON_ROUTE_CONTRACTS
            .iter()
            .find(|contract| contract.pattern == "/studio/story-founder-intake")
            .expect("story founder intake route contract");
        assert_eq!(contract.sample_route, "/studio/story-founder-intake");
        assert!(matches!(
            contract.protection,
            RouteProtection::Protected {
                action: PolicyAction::Create,
                capability_id: "studio.story.founder_intake.write",
            }
        ));

        let capability = built_in_capabilities()
            .into_iter()
            .find(|capability| capability.id == "studio.story.founder_intake.write")
            .expect("story founder intake capability");
        assert_eq!(capability.family, "studio");
        assert!(capability
            .artifact_kinds
            .contains(&"story.founder_intake_packet".to_string()));

        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/studio/story-founder-intake"),
            Some("studio.story.founder_intake.write"),
        );
        assert!(denied.is_err());

        let allowed = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("127.0.0.1:4000"),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/studio/story-founder-intake"),
            Some("studio.story.founder_intake.write"),
        );
        assert!(allowed.is_ok());

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id = 'studio.story.founder_intake.write'
                   AND resource_id = '/studio/story-founder-intake'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 2);
    }

    #[test]
    fn story_production_review_query_maps_to_packet_request() {
        let request = StoryProductionReviewQuery {
            audience: Some(StoryProductionReviewAudience::Owner),
            artifact_ids: Some("artifact_b, artifact_a, artifact_b".to_string()),
            artifact_id: Some("artifact_c".to_string()),
            deck_id: Some("homepage.story.v1".to_string()),
        }
        .into_request();

        assert_eq!(request.audience, StoryProductionReviewAudience::Owner);
        assert_eq!(
            request.artifact_ids,
            vec![
                "artifact_a".to_string(),
                "artifact_b".to_string(),
                "artifact_c".to_string()
            ]
        );
        assert_eq!(request.deck_id, Some("homepage.story.v1".to_string()));

        let default_request = StoryProductionReviewQuery {
            audience: None,
            artifact_ids: None,
            artifact_id: None,
            deck_id: None,
        }
        .into_request();

        assert_eq!(
            default_request.audience,
            StoryProductionReviewAudience::Staff
        );
        assert!(default_request.artifact_ids.is_empty());
    }

    #[test]
    fn story_publish_learning_query_maps_to_brief_request() {
        let request = StoryPublishLearningQuery {
            audience: Some(StoryPublishLearningAudience::Owner),
            artifact_ids: Some("artifact_b, artifact_a, artifact_b".to_string()),
            artifact_id: Some("artifact_c".to_string()),
            deck_id: Some("homepage.story.custom".to_string()),
        }
        .into_request();

        assert_eq!(request.audience, StoryPublishLearningAudience::Owner);
        assert_eq!(request.deck_id, "homepage.story.custom");
        assert_eq!(
            request.artifact_ids,
            vec![
                "artifact_a".to_string(),
                "artifact_b".to_string(),
                "artifact_c".to_string()
            ]
        );

        let default_request = StoryPublishLearningQuery {
            audience: None,
            artifact_ids: None,
            artifact_id: None,
            deck_id: None,
        }
        .into_request();

        assert_eq!(
            default_request.audience,
            StoryPublishLearningAudience::Staff
        );
        assert_eq!(default_request.deck_id, "homepage.story.v1");
        assert!(default_request.artifact_ids.is_empty());
    }

    #[tokio::test]
    async fn story_production_review_handler_returns_packet_without_mutation() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let (artifact, _) = record_artifact(
            &connection,
            ArtifactInput {
                artifact_kind: "story.narrative_deck".to_string(),
                title: "Founder Story Deck".to_string(),
                status: "ready".to_string(),
                visibility_ceiling: "staff".to_string(),
                summary: "Evidence-backed founder story deck.".to_string(),
                source_kind: Some("story_pack".to_string()),
                source_id: Some("story_pack_homepage".to_string()),
                evidence_refs: vec!["workflow:story_homepage".to_string()],
                provenance: json!({"generatedBy": "story_pack.test", "contract": {"deckId": "homepage.story.v1"}}),
                content_hash: "sha256:story-production-review-handler".to_string(),
                storage_uri: Some("ordo://artifact/story-production-review-handler".to_string()),
                health_status: Some("available".to_string()),
                created_by_job_id: None,
            },
        )
        .unwrap();
        let artifact_count_before = table_count(&connection, "artifacts");
        let memory_count_before = table_count(&connection, "generated_content_memory_candidates");
        drop(connection);

        let (event_sender, _) = broadcast::channel(8);
        let (conversation_sender, _) = broadcast::channel(8);
        let state = AppState {
            db_path: Arc::new(db_path),
            event_sender,
            conversation_sender,
            next_supervisor_status: None,
            access_policy: DaemonAccessPolicy::new(None),
        };

        let response = studio_story_production_review_handler(
            ConnectInfo(socket_addr("127.0.0.1:4000")),
            HeaderMap::new(),
            State(state),
            Query(StoryProductionReviewQuery {
                audience: Some(StoryProductionReviewAudience::Owner),
                artifact_ids: Some(artifact.id.clone()),
                artifact_id: None,
                deck_id: None,
            }),
        )
        .await
        .expect("loopback protected route returns Story production review packet");
        let packet = response.0;

        assert_eq!(packet.audience, "owner");
        assert_eq!(packet.read_only, true);
        assert_eq!(packet.mutation_performed, false);
        assert_eq!(packet.confirmed_graph_promotion, false);
        assert_eq!(packet.live_provider_called, false);
        assert_eq!(packet.external_publishing_claimed, false);
        assert_eq!(packet.deck_id, Some("homepage.story.v1".to_string()));
        assert!(packet
            .components
            .iter()
            .any(|component| component.artifact_ref == Some(format!("artifact:{}", artifact.id))));

        let connection = rusqlite::Connection::open(packet_db_path(&temp_dir)).unwrap();
        assert_eq!(table_count(&connection, "artifacts"), artifact_count_before);
        assert_eq!(
            table_count(&connection, "generated_content_memory_candidates"),
            memory_count_before
        );
    }

    #[tokio::test]
    async fn studio_story_intake_story_founder_intake_handler_records_packet_and_handles_retries() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        seed_story_workflow_public_homepage(&db_path);
        let state = test_state(db_path.clone());

        let response = studio_story_founder_intake_handler(
            ConnectInfo(socket_addr("127.0.0.1:4000")),
            HeaderMap::new(),
            State(state.clone()),
            Json(valid_story_founder_intake_input()),
        )
        .await
        .expect("loopback protected route records Story founder intake packet");
        let packet = response.0;

        assert_eq!(packet.schema_version, "ordo.story_founder_intake_packet.v1");
        assert_eq!(packet.intake_id, "story-founder-intake-handler");
        assert_eq!(packet.visibility_ceiling, "owner");
        assert_eq!(packet.approval_state, "needs_review");
        assert!(packet.version.is_none());
        assert_eq!(packet.readiness.status, "ready_for_narrative_deck");
        assert!(packet.readiness.narrative_deck_ready);
        assert!(packet.mutation_performed);
        assert!(!packet.live_provider_called);
        assert!(!packet.external_publishing_claimed);
        assert!(!packet.memory_promotion_performed);
        assert!(!packet.confirmed_graph_promotion);
        assert!(!packet.readiness.automatic_memory_promotion);
        assert!(!packet.readiness.confirmed_graph_promotion);
        let workflow = packet
            .workflow_compilation
            .as_ref()
            .expect("Story intake includes workflow compilation evidence");
        assert_eq!(workflow.status, "compiled");
        assert_eq!(
            workflow.template_id,
            STORY_HOMEPAGE_REFRESH_TEMPLATE_ID.to_string()
        );
        assert!(workflow
            .compilation_ref
            .as_deref()
            .unwrap()
            .starts_with("workflow_compilation:"));
        assert!(workflow
            .input_hash
            .as_deref()
            .unwrap()
            .starts_with("sha256:"));
        assert!(workflow
            .task_bindings
            .iter()
            .any(|task| task.method == "homepage.createNarrativeDeck"));
        assert!(workflow
            .task_bindings
            .iter()
            .any(|task| task.method == "publish.requestApproval"));
        assert!(workflow
            .approval_gates
            .iter()
            .any(|gate| gate.action == "publish" && gate.required));
        assert!(workflow
            .provider_requirements
            .iter()
            .all(|provider| provider.mode == "deterministic_mock" && provider.egress == "none"));
        assert!(!workflow.live_provider_required);
        assert!(!workflow.task_execution_performed);
        assert!(!workflow.external_publishing_claimed);
        assert!(!workflow.memory_promotion_performed);
        assert!(!workflow.confirmed_graph_promotion);
        assert!(packet.event.is_some());
        let packet_json = serde_json::to_string(&packet).unwrap();
        assert!(!packet_json.contains("Internal founder note"));
        assert!(!packet_json.contains("privateNotes"));
        for forbidden in [
            "provider internal",
            "prompt internal",
            "compiled plan private input",
            "task private payload",
            "graph certainty",
        ] {
            assert!(
                !packet_json.contains(forbidden),
                "Story intake packet leaked {forbidden}: {packet_json}"
            );
        }

        let retry = studio_story_founder_intake_handler(
            ConnectInfo(socket_addr("127.0.0.1:4000")),
            HeaderMap::new(),
            State(state.clone()),
            Json(valid_story_founder_intake_input()),
        )
        .await
        .expect("same intake payload is idempotent");
        let retry_packet = retry.0;
        assert!(!retry_packet.mutation_performed);
        assert!(retry_packet.event.is_none());
        assert_eq!(
            retry_packet
                .workflow_compilation
                .as_ref()
                .unwrap()
                .compilation_ref,
            packet
                .workflow_compilation
                .as_ref()
                .unwrap()
                .compilation_ref
        );

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let artifact_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE artifact_kind = ?1",
                [STORY_FOUNDER_INTAKE_ARTIFACT_KIND],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(artifact_count, 1);
        let compilation_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM workflow_template_compilations
                 WHERE idempotency_key = ?1",
                [story_intake_workflow_idempotency_key(&packet)],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(compilation_count, 1);
    }

    #[tokio::test]
    async fn studio_story_intake_workflow_compilation_blocks_missing_input_without_fake_row() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let state = test_state(db_path.clone());

        let response = studio_story_founder_intake_handler(
            ConnectInfo(socket_addr("127.0.0.1:4000")),
            HeaderMap::new(),
            State(state),
            Json(valid_story_founder_intake_input()),
        )
        .await
        .expect("loopback protected route records Story founder intake packet");
        let packet = response.0;
        let workflow = packet.workflow_compilation.unwrap();

        assert_eq!(workflow.status, "blocked");
        assert!(workflow.compilation_ref.is_none());
        assert!(workflow.input_hash.is_none());
        assert!(workflow
            .missing_inputs
            .contains(&"published public homepage profile positioning".to_string()));
        assert!(!workflow.live_provider_required);
        assert!(!workflow.task_execution_performed);
        assert!(!workflow.external_publishing_claimed);
        assert!(!workflow.memory_promotion_performed);
        assert!(!workflow.confirmed_graph_promotion);

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let compilation_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM workflow_template_compilations
                 WHERE idempotency_key = ?1",
                [workflow.idempotency_key],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(compilation_count, 0);
    }

    #[tokio::test]
    async fn studio_story_intake_story_founder_intake_handler_rejects_conflicting_retry_fail_closed(
    ) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let state = test_state(db_path.clone());

        let _ = studio_story_founder_intake_handler(
            ConnectInfo(socket_addr("127.0.0.1:4000")),
            HeaderMap::new(),
            State(state.clone()),
            Json(valid_story_founder_intake_input()),
        )
        .await
        .expect("initial intake succeeds");

        let mut conflicting = valid_story_founder_intake_input();
        conflicting.business_stance = "A materially different Story Pack stance.".to_string();
        let error = studio_story_founder_intake_handler(
            ConnectInfo(socket_addr("127.0.0.1:4000")),
            HeaderMap::new(),
            State(state),
            Json(conflicting),
        )
        .await
        .expect_err("same intake id with different payload fails closed");

        assert_eq!(error.0, StatusCode::BAD_REQUEST);
        assert!(error.1 .0.error.contains("idempotency key conflicts"));

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let artifact_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE artifact_kind = ?1",
                [STORY_FOUNDER_INTAKE_ARTIFACT_KIND],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(artifact_count, 1);
    }

    #[tokio::test]
    async fn story_publish_learning_handler_returns_brief_without_mutation() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let (artifact, _) = record_artifact(
            &connection,
            ArtifactInput {
                artifact_kind: STORY_HOMEPAGE_PUBLISH_APPROVAL_PACKAGE_ARTIFACT_KIND.to_string(),
                title: "Story Homepage Publish Package".to_string(),
                status: "published".to_string(),
                visibility_ceiling: "staff".to_string(),
                summary: "Manual publish approval package.".to_string(),
                source_kind: Some("story_pack".to_string()),
                source_id: Some("story_pack_homepage".to_string()),
                evidence_refs: vec!["publish_approval:story_homepage".to_string()],
                provenance: json!({"contract": {"deckId": "homepage.story.v1", "limitations": ["manual external platform metrics not imported"]}}),
                content_hash: "sha256:story-publish-learning-handler".to_string(),
                storage_uri: Some("ordo://artifact/story-publish-learning-handler".to_string()),
                health_status: Some("available".to_string()),
                created_by_job_id: None,
            },
        )
        .unwrap();
        let artifact_count_before = table_count(&connection, "artifacts");
        let memory_count_before = table_count(&connection, "generated_content_memory_candidates");
        let analytics_count_before = table_count(&connection, "content_analytics_events");
        let reward_count_before = table_count(&connection, "reward_events");
        drop(connection);

        let (event_sender, _) = broadcast::channel(8);
        let (conversation_sender, _) = broadcast::channel(8);
        let state = AppState {
            db_path: Arc::new(db_path),
            event_sender,
            conversation_sender,
            next_supervisor_status: None,
            access_policy: DaemonAccessPolicy::new(None),
        };

        let response = studio_story_publish_learning_handler(
            ConnectInfo(socket_addr("127.0.0.1:4000")),
            HeaderMap::new(),
            State(state),
            Query(StoryPublishLearningQuery {
                audience: Some(StoryPublishLearningAudience::Owner),
                artifact_ids: Some(artifact.id.clone()),
                artifact_id: None,
                deck_id: Some("homepage.story.v1".to_string()),
            }),
        )
        .await
        .expect("loopback protected route returns Story publish learning brief");
        let brief = response.0;

        assert_eq!(brief.audience, "owner");
        assert_eq!(brief.deck_id, "homepage.story.v1");
        assert_eq!(brief.read_only, true);
        assert_eq!(brief.mutation_performed, false);
        assert_eq!(brief.confirmed_graph_promotion, false);
        assert_eq!(brief.memory_promotion_performed, false);
        assert_eq!(brief.live_provider_called, false);
        assert_eq!(brief.external_publishing_claimed, false);
        assert!(brief
            .publish_evidence
            .iter()
            .any(|source| source.source_id == artifact.id));
        assert!(brief
            .limitations
            .contains(&"story_publish_learning_brief_is_read_only".to_string()));

        let connection = rusqlite::Connection::open(packet_db_path(&temp_dir)).unwrap();
        assert_eq!(table_count(&connection, "artifacts"), artifact_count_before);
        assert_eq!(
            table_count(&connection, "generated_content_memory_candidates"),
            memory_count_before
        );
        assert_eq!(
            table_count(&connection, "content_analytics_events"),
            analytics_count_before
        );
        assert_eq!(
            table_count(&connection, "reward_events"),
            reward_count_before
        );
    }

    #[test]
    fn generated_content_memory_routes_use_protected_access_boundary() {
        for (pattern, sample_route, action, capability) in [
            (
                "/studio/generated-content-memory/:artifact_id/review",
                "/studio/generated-content-memory/artifact_1/review",
                PolicyAction::Inspect,
                "memory.candidates.review",
            ),
            (
                "/studio/generated-content-memory/candidates/:candidate_id/decision",
                "/studio/generated-content-memory/candidates/generated_content_memory_candidate_1/decision",
                PolicyAction::Approve,
                "memory.candidates.decide",
            ),
        ] {
            let contract = DAEMON_ROUTE_CONTRACTS
                .iter()
                .find(|contract| contract.pattern == pattern)
                .unwrap_or_else(|| panic!("{pattern} route contract"));
            assert_eq!(contract.sample_route, sample_route);
            assert!(matches!(
                contract.protection,
                RouteProtection::Protected {
                    action: contract_action,
                    capability_id: contract_capability,
                } if contract_action == action && contract_capability == capability
            ));
        }

        let review_capability = built_in_capabilities()
            .into_iter()
            .find(|capability| capability.id == "memory.candidates.review")
            .expect("memory candidate review capability");
        assert_eq!(review_capability.family, "memory");
        let decision_capability = built_in_capabilities()
            .into_iter()
            .find(|capability| capability.id == "memory.candidates.decide")
            .expect("memory candidate decision capability");
        assert_eq!(decision_capability.family, "memory");

        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        for (route, action, capability) in [
            (
                "/studio/generated-content-memory/artifact_1/review",
                PolicyAction::Inspect,
                "memory.candidates.review",
            ),
            (
                "/studio/generated-content-memory/candidates/generated_content_memory_candidate_1/decision",
                PolicyAction::Approve,
                "memory.candidates.decide",
            ),
        ] {
            let denied = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("192.168.1.10:4000"),
                action,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(denied.is_err(), "{route} should deny non-loopback access");

            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                action,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(
                allowed.is_ok(),
                "{route} should allow protected loopback access"
            );
        }

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN ('memory.candidates.review', 'memory.candidates.decide')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 4);
    }

    #[tokio::test]
    async fn generated_content_memory_review_route_returns_role_safe_packet_without_mutation() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let (artifact, _) = story_memory_artifact(&connection);
        let private_summary = "The founder story can state that Ordo learns from approved work.";
        let (candidates, _) = ingest_generated_content_memory_candidates(
            &connection,
            story_memory_ingestion_input(&artifact.id, private_summary),
        )
        .unwrap();
        let candidate_id = candidates[0].id.clone();
        let candidate_count_before =
            table_count(&connection, "generated_content_memory_candidates");
        let event_count_before = table_count(&connection, "realtime_events");
        drop(connection);

        let state = test_state(db_path);
        let response = generated_content_memory_review_handler(
            ConnectInfo(socket_addr("127.0.0.1:4000")),
            HeaderMap::new(),
            State(state),
            AxumPath(artifact.id.clone()),
            Query(GeneratedContentMemoryReviewQuery {
                audience: Some(GeneratedContentMemoryReviewAudience::Member),
            }),
        )
        .await
        .expect("loopback protected route returns memory review packet");
        let packet = response.0;

        assert_eq!(packet.artifact_id, artifact.id);
        assert_eq!(packet.audience, "member");
        assert_eq!(packet.candidate_count, 1);
        assert_eq!(packet.confirmed_graph_promotion, false);
        assert_eq!(packet.live_provider_called, false);
        assert_eq!(packet.items[0].candidate_id, candidate_id);
        assert_eq!(packet.items[0].body_redacted, true);
        assert_eq!(
            packet.items[0].summary_text,
            "Generated content memory candidate requires authorized review."
        );
        assert!(!packet.items[0].summary_text.contains("Ordo learns"));
        assert_eq!(packet.items[0].body, json!({}));
        assert!(packet
            .limitations
            .contains(&"member_safe_packet_redacts_candidate_bodies".to_string()));

        let connection = rusqlite::Connection::open(packet_db_path(&temp_dir)).unwrap();
        assert_eq!(
            table_count(&connection, "generated_content_memory_candidates"),
            candidate_count_before
        );
        assert_eq!(
            table_count(&connection, "realtime_events"),
            event_count_before
        );
    }

    #[tokio::test]
    async fn generated_content_memory_decision_route_records_safe_event_without_graph_promotion() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let (artifact, _) = story_memory_artifact(&connection);
        let (candidates, _) = ingest_generated_content_memory_candidates(
            &connection,
            story_memory_ingestion_input(
                &artifact.id,
                "Approved homepage story claims can inform candidate memory.",
            ),
        )
        .unwrap();
        let candidate_id = candidates[0].id.clone();
        drop(connection);

        let state = test_state(db_path);
        let response = generated_content_memory_decision_handler(
            ConnectInfo(socket_addr("127.0.0.1:4000")),
            HeaderMap::new(),
            State(state),
            AxumPath(candidate_id.clone()),
            Json(GeneratedContentMemoryDecisionInput {
                decision: GeneratedContentMemoryState::Approved,
                reason: "Owner approved this as candidate evidence.".to_string(),
                evidence_refs: vec!["owner_review:memory_candidate".to_string()],
            }),
        )
        .await
        .expect("loopback protected route records memory candidate decision");
        let response = response.0;

        assert_eq!(response.candidate.id, candidate_id);
        assert_eq!(response.candidate.candidate_state, "approved");
        assert_eq!(
            response.candidate.memory_effect,
            "candidate_stronger_evidence"
        );
        assert_eq!(
            response.event.event_type,
            "generated_content_memory.decision_recorded"
        );
        assert_eq!(response.event.payload["candidateId"], candidate_id);
        assert_eq!(response.event.payload["candidateState"], "approved");
        assert!(response.event.payload.get("body").is_none());
        assert!(response.event.payload.get("summaryText").is_none());
        assert!(response.event.payload.get("providerPayload").is_none());
        assert!(response
            .event
            .payload
            .get("confirmedGraphPromotion")
            .is_none());

        let connection = rusqlite::Connection::open(packet_db_path(&temp_dir)).unwrap();
        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM realtime_events
                 WHERE event_type = 'generated_content_memory.decision_recorded'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 1);
    }

    fn story_memory_artifact(
        connection: &rusqlite::Connection,
    ) -> (crate::artifacts::ArtifactView, RealtimeEvent) {
        record_artifact(
            connection,
            ArtifactInput {
                artifact_kind: "story.narrative_deck".to_string(),
                title: "Founder Story Deck".to_string(),
                status: "ready".to_string(),
                visibility_ceiling: "staff".to_string(),
                summary: "Evidence-backed founder story deck.".to_string(),
                source_kind: Some("story_pack".to_string()),
                source_id: Some("story_pack_homepage".to_string()),
                evidence_refs: vec!["workflow:story_homepage".to_string()],
                provenance: json!({
                    "generatedBy": "story_pack.test",
                    "contract": {"deckId": "homepage.story.v1"}
                }),
                content_hash: "sha256:generated-content-memory-route-artifact".to_string(),
                storage_uri: Some("ordo://artifact/generated-content-memory-route".to_string()),
                health_status: Some("available".to_string()),
                created_by_job_id: None,
            },
        )
        .unwrap()
    }

    fn story_memory_ingestion_input(
        artifact_id: &str,
        summary_text: &str,
    ) -> GeneratedContentMemoryIngestionInput {
        GeneratedContentMemoryIngestionInput {
            artifact_id: artifact_id.to_string(),
            artifact_version_id: None,
            workflow_template_id: Some("story.homepage.scrollytelling.v1".to_string()),
            workflow_compilation_id: Some("workflow_compilation_story_homepage".to_string()),
            job_id: Some("job_story_homepage_memory".to_string()),
            extraction_fixture_id: "fixture.story.memory.route".to_string(),
            items: vec![GeneratedContentMemoryItemInput {
                memory_kind: GeneratedContentMemoryKind::CandidateClaim,
                candidate_state: None,
                summary_text: summary_text.to_string(),
                body: json!({
                    "claim": summary_text,
                    "source": "generated_story_artifact"
                }),
                confidence: 0.84,
                evidence_refs: vec![
                    format!("artifact:{artifact_id}"),
                    "workflow:story_homepage".to_string(),
                    "private_note:owner_only".to_string(),
                ],
                limitations: vec!["candidate_requires_owner_review".to_string()],
                visibility: "staff".to_string(),
                approval_evidence_refs: vec![],
                publication_evidence_refs: vec![],
                feedback_evidence_refs: vec![],
                outcome_evidence_refs: vec![],
                rejection_evidence_refs: vec![],
            }],
        }
    }

    fn test_state(db_path: std::path::PathBuf) -> AppState {
        let (event_sender, _) = broadcast::channel(8);
        let (conversation_sender, _) = broadcast::channel(8);
        AppState {
            db_path: Arc::new(db_path),
            event_sender,
            conversation_sender,
            next_supervisor_status: None,
            access_policy: DaemonAccessPolicy::new(None),
        }
    }

    fn valid_story_founder_intake_input() -> StoryFounderIntakeInput {
        StoryFounderIntakeInput {
            intake_id: "story-founder-intake-handler".to_string(),
            founder_story: "A local-first studio operator helps founders publish durable public stories from approved evidence.".to_string(),
            business_stance: "Ordo is a practical answer to brittle hosted tooling and extractive content platforms.".to_string(),
            audience: Some("founders evaluating local-first AI operations".to_string()),
            public_claims: vec![StoryIntakeClaimInput {
                claim: "Ordo keeps public Story Pack claims evidence-backed and reviewable.".to_string(),
                evidence_refs: vec!["artifact:approved-founder-note".to_string()],
            }],
            proof_evidence_refs: vec!["artifact:approved-founder-note".to_string()],
            private_notes: vec!["Internal founder note that must remain owner scoped.".to_string()],
            style_preferences: vec!["plainspoken".to_string()],
            offer_refs: vec!["offer:pilot-foundations".to_string()],
            cta_refs: vec!["cta:request-onboarding".to_string()],
            limitations: vec!["Requires owner review before public derivative use.".to_string()],
            source_kind: Some("story_pack_intake".to_string()),
            source_id: Some("story-founder-intake-handler".to_string()),
            created_by_job_id: None,
        }
    }

    fn seed_story_workflow_public_homepage(db_path: &std::path::Path) {
        let connection = rusqlite::Connection::open(db_path).unwrap();
        insert_public_story_fact(
            &connection,
            "homepage.profile.positioning",
            json!("Ordo is a local-first operating appliance for relationship-led businesses."),
        );
        insert_public_story_fact(&connection, "homepage.slides.hero.order", json!(10));
        insert_public_story_fact(&connection, "homepage.slides.hero.sectionId", json!("hero"));
        insert_public_story_fact(
            &connection,
            "homepage.slides.hero.title",
            json!("Studio Ordo"),
        );
        insert_public_story_fact(
            &connection,
            "homepage.slides.hero.body",
            json!("A public story grounded in local evidence."),
        );
        insert_public_story_fact(&connection, "homepage.slides.proof.order", json!(20));
        insert_public_story_fact(
            &connection,
            "homepage.slides.proof.sectionId",
            json!("proof"),
        );
        insert_public_story_fact(
            &connection,
            "homepage.slides.proof.title",
            json!("Proof before polish"),
        );
        insert_public_story_fact(
            &connection,
            "homepage.slides.proof.body",
            json!("The story changes when evidence changes."),
        );
    }

    fn insert_public_story_fact(
        connection: &rusqlite::Connection,
        fact_key: &str,
        value: serde_json::Value,
    ) {
        connection
            .execute(
                "INSERT INTO business_facts (
                    id, subject_type, subject_id, fact_key, value_json, source_kind,
                    source_label, source_uri, provenance_json, visibility, publication_state,
                    created_by_actor_id, created_at, updated_at, published_at, archived_at
                 ) VALUES (
                    ?1, 'business', 'business_local', ?2, ?3, 'operator',
                    'story intake workflow test', NULL, '{\"test\":true}', 'public', 'published',
                    NULL, 'now', 'now', 'now', NULL
                 )",
                rusqlite::params![
                    format!("business_fact_{}", fact_key.replace('.', "_")),
                    fact_key,
                    value.to_string()
                ],
            )
            .unwrap();
    }

    fn packet_db_path(temp_dir: &tempfile::TempDir) -> std::path::PathBuf {
        temp_dir.path().join("local.db")
    }

    fn table_count(connection: &rusqlite::Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    #[test]
    fn studio_promo_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/studio/promo-video-packages"),
            Some("studio.promo_video.package"),
        );
        assert!(denied.is_err());

        for (route, action, capability) in [
            (
                "/studio/promo-video-packages",
                PolicyAction::Create,
                "studio.promo_video.package",
            ),
            (
                "/studio/promo-video-packages/artifact_1/review",
                PolicyAction::Approve,
                "studio.promo_video.review",
            ),
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                action,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(
                allowed.is_ok(),
                "{route} should be protected but usable locally"
            );
        }

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN ('studio.promo_video.package', 'studio.promo_video.review')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 3);
    }

    #[test]
    fn studio_artifact_patch_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/studio/artifact-patches"),
            Some("studio.artifact_patch.review"),
        );
        assert!(denied.is_err());

        for (route, action, capability) in [
            (
                "/studio/artifact-patches",
                PolicyAction::Inspect,
                "studio.artifact_patch.review",
            ),
            (
                "/studio/artifact-patches/patch_1",
                PolicyAction::Inspect,
                "studio.artifact_patch.review",
            ),
            (
                "/studio/artifact-patches/patch_1/accept",
                PolicyAction::Approve,
                "studio.artifact_patch.accept",
            ),
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                action,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(
                allowed.is_ok(),
                "{route} should be protected but usable locally"
            );
        }

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN ('studio.artifact_patch.review', 'studio.artifact_patch.accept')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 4);
    }

    #[test]
    fn report_and_support_packet_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Inspect,
            ResourceRef::new(ResourceKind::DaemonRoute, "/reports/issues"),
            Some("issue.report.list"),
        );
        assert!(denied.is_err());

        for (route, action, capability) in [
            (
                "/reports/issues",
                PolicyAction::Inspect,
                "issue.report.list",
            ),
            (
                "/reports/issues/report_1",
                PolicyAction::Inspect,
                "issue.report.detail",
            ),
            (
                "/reports/issues/report_1/status",
                PolicyAction::Update,
                "issue.report.status.update",
            ),
            (
                "/reports/issues/report_1/exports",
                PolicyAction::Export,
                "issue.report.export",
            ),
            (
                "/support-packets",
                PolicyAction::Inspect,
                "support.packets.list",
            ),
            (
                "/support-packets",
                PolicyAction::Prepare,
                "support.packets.draft",
            ),
            (
                "/support-packets/packet_1/approve",
                PolicyAction::Approve,
                "support.packets.approve",
            ),
            (
                "/support-packets/packet_1/receipts",
                PolicyAction::Inspect,
                "support.packet.receipts.list",
            ),
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                action,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(allowed.is_ok());
        }

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN (
                    'issue.report.list', 'issue.report.detail',
                    'issue.report.status.update', 'issue.report.export',
                    'support.packets.list', 'support.packets.draft',
                    'support.packets.approve', 'support.packet.receipts.list'
                 )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 9);
    }

    #[test]
    fn corpus_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Create,
            ResourceRef::new(ResourceKind::DaemonRoute, "/corpus/items"),
            Some("corpus.items.write"),
        );
        assert!(denied.is_err());

        for (route, action, capability) in [
            (
                "/corpus/sources",
                PolicyAction::Inspect,
                "corpus.sources.list",
            ),
            (
                "/corpus/sources/source_1",
                PolicyAction::Inspect,
                "corpus.sources.list",
            ),
            (
                "/corpus/sources",
                PolicyAction::Create,
                "corpus.sources.write",
            ),
            (
                "/corpus/sources/source_1",
                PolicyAction::Update,
                "corpus.sources.write",
            ),
            ("/corpus/items", PolicyAction::Inspect, "corpus.items.list"),
            (
                "/corpus/items/item_1",
                PolicyAction::Inspect,
                "corpus.items.list",
            ),
            ("/corpus/items", PolicyAction::Create, "corpus.items.write"),
            (
                "/corpus/items/item_1",
                PolicyAction::Update,
                "corpus.items.write",
            ),
            ("/corpus/retrieve", PolicyAction::Read, "corpus.retrieve"),
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                action,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(allowed.is_ok());
        }

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN (
                    'corpus.sources.list', 'corpus.sources.write',
                    'corpus.items.list', 'corpus.items.write', 'corpus.retrieve'
                 )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 10);
    }

    #[test]
    fn answer_draft_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Prepare,
            ResourceRef::new(ResourceKind::DaemonRoute, "/answer-drafts"),
            Some("answer.drafts.prepare"),
        );
        assert!(denied.is_err());

        for (route, action, capability) in [
            (
                "/answer-drafts",
                PolicyAction::Inspect,
                "answer.drafts.list",
            ),
            (
                "/answer-drafts/answer_draft_1",
                PolicyAction::Inspect,
                "answer.drafts.list",
            ),
            (
                "/answer-drafts",
                PolicyAction::Prepare,
                "answer.drafts.prepare",
            ),
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                action,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(allowed.is_ok());
        }

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN ('answer.drafts.list', 'answer.drafts.prepare')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 4);
    }

    #[test]
    fn mcp_pack_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Validate,
            ResourceRef::new(ResourceKind::DaemonRoute, "/mcp/packs"),
            Some("mcp.packs.write"),
        );
        assert!(denied.is_err());

        for (route, action, capability) in [
            ("/mcp/packs", PolicyAction::Inspect, "mcp.packs.list"),
            (
                "/mcp/packs/pack.local.status",
                PolicyAction::Inspect,
                "mcp.packs.list",
            ),
            ("/mcp/packs", PolicyAction::Validate, "mcp.packs.write"),
            (
                "/mcp/packs/pack.local.status/disable",
                PolicyAction::Update,
                "mcp.packs.write",
            ),
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                action,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(allowed.is_ok());
        }

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN ('mcp.packs.list', 'mcp.packs.write')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 5);
    }

    #[test]
    fn product_pack_routes_use_protected_access_boundary() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let policy = DaemonAccessPolicy::new(None);
        let headers = HeaderMap::new();

        let denied = authorize_protected_daemon_route(
            &policy,
            &db_path,
            &headers,
            socket_addr("192.168.1.10:4000"),
            PolicyAction::Validate,
            ResourceRef::new(ResourceKind::DaemonRoute, "/product-packs"),
            Some("product_packs.write"),
        );
        assert!(denied.is_err());

        for (route, action, capability) in [
            (
                "/product-packs",
                PolicyAction::Inspect,
                "product_packs.list",
            ),
            (
                "/product-packs/product_pack.nyc.promo_ops",
                PolicyAction::Inspect,
                "product_packs.list",
            ),
            (
                "/product-packs",
                PolicyAction::Validate,
                "product_packs.write",
            ),
            (
                "/product-packs/product_pack.nyc.promo_ops/disable",
                PolicyAction::Update,
                "product_packs.write",
            ),
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                action,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(capability),
            );
            assert!(allowed.is_ok());
        }

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let audit_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM policy_decisions
                 WHERE capability_id IN ('product_packs.list', 'product_packs.write')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 5);
    }
}
