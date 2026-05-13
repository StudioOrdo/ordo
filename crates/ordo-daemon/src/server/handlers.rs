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
use std::time::Duration as StdDuration;
use tokio::sync::broadcast;

use crate::answer_drafts::{
    list_answer_drafts, prepare_answer_draft, read_answer_draft, AnswerDraftListResponse,
    AnswerDraftRequest, AnswerDraftResponse,
};
use crate::availability::{
    create_handoff_inbox_item, evaluate_handoff_eligibility, list_handoff_inbox,
    list_handoff_receipts, read_availability_state, resolve_handoff_inbox_item,
    update_availability_schedule, update_operator_presence, AvailabilityScheduleView,
    AvailabilityScheduleWriteRequest, AvailabilityStateResponse, HandoffEligibilityRequest,
    HandoffEligibilityView, HandoffInboxCreateRequest, HandoffInboxItemView,
    HandoffInboxListResponse, HandoffInboxResolveRequest, HandoffReceiptListResponse,
    OperatorPresenceView, OperatorPresenceWriteRequest,
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
    PublicEntryPointView, TrackedEntryPointView, VisitorSessionCreateRequest,
    VisitorSessionListResponse, VisitorSessionView,
};
use crate::errors::{DaemonErrorCode, ErrorResponse};
use crate::events::{
    append_system_event, replay_events, system_event, EventReplayResponse, RealtimeEvent,
};
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
    accept_public_offer, create_offer, list_offer_acceptances, list_offers,
    list_public_available_offers, list_trials, transition_trial, update_offer,
    OfferAcceptanceCreateRequest, OfferAcceptanceListResponse, OfferAcceptanceResponse,
    OfferListResponse, OfferView, OfferWriteRequest, PublicOfferListResponse, TrialListResponse,
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
use crate::public_surfaces::{
    public_about, public_asks, public_feed, public_offers, public_surfaces, AboutReadModel,
    AsksReadModel, FeedReadModel, OffersReadModel, PublicSurfacesResponse,
};
use crate::reports::{
    approve_support_packet, draft_support_packet, export_issue_report, list_issue_reports,
    list_support_packet_receipts, list_support_packets, prepare_issue_report, read_issue_report,
    update_issue_report_status, IssueReportDetailResponse, IssueReportExportRequest,
    IssueReportExportResponse, IssueReportPrepareRequest, IssueReportStatusUpdateRequest,
    IssueReportsResponse, SupportPacketApprovalRequest, SupportPacketDraftRequest,
    SupportPacketListResponse, SupportPacketReceiptListResponse, SupportPacketView,
};
use crate::secrets::{constant_time_secret_eq, OrdoSecretString};

const NEXT_SUPERVISOR_MAX_RESTARTS: u32 = 3;
const NEXT_SUPERVISOR_RESTART_DELAY: StdDuration = StdDuration::from_secs(1);
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
) -> Result<Json<VisitorSessionView>, (StatusCode, Json<ErrorResponse>)> {
    let (session, event) =
        create_visitor_session(&state.db_path, request).map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(session))
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
    list_handoff_inbox(&state.db_path)
        .map(Json)
        .map_err(internal_error)
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
    let (acceptance, trial, event) =
        accept_public_offer(&state.db_path, &offer_slug, request).map_err(invalid_request_error)?;
    let _ = state.event_sender.send(event);
    Ok(Json(OfferAcceptanceResponse { acceptance, trial }))
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
    let loopback = remote_addr.ip().is_loopback();
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
    use crate::route_contracts::{RouteProtection, DAEMON_ROUTE_CONTRACTS};
    use crate::schema::init_database;
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
            (
                "/handoff/inbox/handoff_item_1/receipts",
                "handoff.receipts.list",
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
            "/handoff/inbox/handoff_item_1/resolve",
        ] {
            let allowed = authorize_protected_daemon_route(
                &policy,
                &db_path,
                &headers,
                socket_addr("127.0.0.1:4000"),
                PolicyAction::Create,
                ResourceRef::new(ResourceKind::DaemonRoute, route),
                Some(if route.starts_with("/availability") {
                    "availability.write"
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
                    'handoff.eligibility.evaluate', 'handoff.inbox.list', 'handoff.receipts.list'
                 )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(audit_count, 8);
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
}
