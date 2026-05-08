use anyhow::{Context, Result};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration as StdDuration;
use tokio::sync::broadcast;
use tokio::time::{self, Duration};

use crate::backups::{
    create_backup, list_backup_restore_jobs, run_restore_preflight, BackupRestoreResponse,
    RestorePreflightRequest,
};
use crate::briefs::{
    generate_system_brief, latest_system_brief, run_due_system_brief_schedules, LatestBriefResponse,
};
use crate::capabilities::{list_capabilities, CapabilityCatalogResponse};
use crate::events::{
    append_system_event, replay_events, system_event, EventReplayResponse, RealtimeEvent,
};
use crate::health::{
    build_health_report, build_readiness_report, HealthCheck, HealthReport, ReadinessReport,
};
use crate::mcp::{handle_mcp_json, McpResponse};
use crate::schema::init_database;

const NEXT_SUPERVISOR_MAX_RESTARTS: u32 = 3;
const NEXT_SUPERVISOR_RESTART_DELAY: StdDuration = StdDuration::from_secs(1);
const DAEMON_ACCESS_TOKEN_HEADER: &str = "x-ordo-daemon-token";

type SharedNextSupervisorStatus = Arc<Mutex<NextSupervisorStatus>>;

#[derive(Clone)]
struct AppState {
    db_path: Arc<PathBuf>,
    event_sender: broadcast::Sender<RealtimeEvent>,
    next_supervisor_status: Option<SharedNextSupervisorStatus>,
    access_policy: DaemonAccessPolicy,
}

#[derive(Debug, Clone)]
struct DaemonAccessPolicy {
    access_token: Option<String>,
}

impl DaemonAccessPolicy {
    fn new(access_token: Option<String>) -> Self {
        Self {
            access_token: access_token.and_then(|token| {
                let trimmed = token.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NextSupervisorConfig {
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NextSupervisorPhase {
    Starting,
    Running,
    Restarting,
    Failed,
}

#[derive(Debug, Clone)]
struct NextSupervisorStatus {
    phase: NextSupervisorPhase,
    pid: Option<u32>,
    restart_count: u32,
    detail: String,
}

impl NextSupervisorStatus {
    fn starting() -> Self {
        Self {
            phase: NextSupervisorPhase::Starting,
            pid: None,
            restart_count: 0,
            detail: "Next.js child process is starting.".to_string(),
        }
    }
}

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
    let next_supervisor_status = next_supervisor
        .as_ref()
        .map(|_| Arc::new(Mutex::new(NextSupervisorStatus::starting())));
    let state = AppState {
        db_path: Arc::new(db_path),
        event_sender,
        next_supervisor_status,
        access_policy: DaemonAccessPolicy::new(access_token),
    };
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/capabilities", get(capabilities_handler))
        .route("/briefs/system/latest", get(latest_system_brief_handler))
        .route(
            "/briefs/system/generate",
            post(generate_system_brief_handler),
        )
        .route("/backups", get(list_backup_restore_handler))
        .route("/backups/create", post(create_backup_handler))
        .route("/restore/validate", post(validate_restore_handler))
        .route("/events", get(events_handler))
        .route("/mcp", post(mcp_handler))
        .route("/ws", get(ws_handler))
        .with_state(state.clone());

    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "daemon.started",
        json!({ "host": host, "port": port }),
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

fn spawn_next_supervisor(
    config: NextSupervisorConfig,
    db_path: Arc<PathBuf>,
    event_sender: broadcast::Sender<RealtimeEvent>,
    next_status: SharedNextSupervisorStatus,
) -> Result<()> {
    thread::Builder::new()
        .name("ordo-next-supervisor".to_string())
        .spawn(move || supervise_next_child(config, db_path, event_sender, next_status))
        .context("Failed to spawn Next.js supervisor thread")?;
    Ok(())
}

fn supervise_next_child(
    config: NextSupervisorConfig,
    db_path: Arc<PathBuf>,
    event_sender: broadcast::Sender<RealtimeEvent>,
    next_status: SharedNextSupervisorStatus,
) {
    let mut restart_count = 0;

    loop {
        match start_next_child(
            &config,
            &db_path,
            &event_sender,
            &next_status,
            restart_count,
        ) {
            Ok(mut child) => {
                let child_id = child.id();
                match child.wait() {
                    Ok(exit_status) => {
                        emit_system_event(
                            &db_path,
                            &event_sender,
                            "next.supervisor.exited",
                            json!({
                                "pid": child_id,
                                "success": exit_status.success(),
                                "code": exit_status.code(),
                                "restartCount": restart_count,
                            }),
                        );
                        if schedule_next_restart(
                            &db_path,
                            &event_sender,
                            &next_status,
                            child_exit_message(child_id, &exit_status),
                            &mut restart_count,
                        ) {
                            continue;
                        }
                        return;
                    }
                    Err(error) => {
                        emit_system_event(
                            &db_path,
                            &event_sender,
                            "next.supervisor.wait_failed",
                            json!({ "pid": child_id, "message": error.to_string(), "restartCount": restart_count }),
                        );
                        if schedule_next_restart(
                            &db_path,
                            &event_sender,
                            &next_status,
                            format!("Next.js child wait failed: {error}"),
                            &mut restart_count,
                        ) {
                            continue;
                        }
                        return;
                    }
                }
            }
            Err(error) => {
                let message = format!("Failed to start Next.js child process: {error}");
                emit_system_event(
                    &db_path,
                    &event_sender,
                    "next.supervisor.start_failed",
                    json!({ "command": config.command, "args": config.args, "message": message.clone(), "restartCount": restart_count }),
                );
                if schedule_next_restart(
                    &db_path,
                    &event_sender,
                    &next_status,
                    message,
                    &mut restart_count,
                ) {
                    continue;
                }
                return;
            }
        }
    }
}

fn start_next_child(
    config: &NextSupervisorConfig,
    db_path: &Path,
    event_sender: &broadcast::Sender<RealtimeEvent>,
    next_status: &SharedNextSupervisorStatus,
    restart_count: u32,
) -> Result<Child> {
    let child = Command::new(&config.command)
        .args(&config.args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| config.command.clone())?;
    let child_id = child.id();
    update_next_supervisor_status(next_status, |status| {
        status.phase = NextSupervisorPhase::Running;
        status.pid = Some(child_id);
        status.restart_count = restart_count;
        status.detail = format!("Next.js child process is running with pid {child_id}.");
    });
    emit_system_event(
        db_path,
        event_sender,
        "next.supervisor.started",
        json!({
            "command": config.command,
            "args": config.args,
            "pid": child_id,
            "restartCount": restart_count,
        }),
    );
    if restart_count > 0 {
        emit_system_event(
            db_path,
            event_sender,
            "next.supervisor.recovered",
            json!({ "pid": child_id, "restartCount": restart_count }),
        );
    }
    Ok(child)
}

fn schedule_next_restart(
    db_path: &Path,
    event_sender: &broadcast::Sender<RealtimeEvent>,
    next_status: &SharedNextSupervisorStatus,
    message: String,
    restart_count: &mut u32,
) -> bool {
    if should_restart_next_child(*restart_count, NEXT_SUPERVISOR_MAX_RESTARTS) {
        *restart_count += 1;
        update_next_supervisor_status(next_status, |status| {
            status.phase = NextSupervisorPhase::Restarting;
            status.pid = None;
            status.restart_count = *restart_count;
            status.detail = format!(
                "{message} Restart attempt {} of {NEXT_SUPERVISOR_MAX_RESTARTS} is scheduled.",
                *restart_count
            );
        });
        emit_system_event(
            db_path,
            event_sender,
            "next.supervisor.restart_attempt",
            json!({
                "restartCount": *restart_count,
                "maxRestarts": NEXT_SUPERVISOR_MAX_RESTARTS,
                "delayMs": NEXT_SUPERVISOR_RESTART_DELAY.as_millis(),
                "message": message,
            }),
        );
        thread::sleep(NEXT_SUPERVISOR_RESTART_DELAY);
        true
    } else {
        update_next_supervisor_status(next_status, |status| {
            status.phase = NextSupervisorPhase::Failed;
            status.pid = None;
            status.restart_count = *restart_count;
            status.detail = format!(
                "{message} Restart budget exhausted after {} attempts.",
                *restart_count
            );
        });
        emit_system_event(
            db_path,
            event_sender,
            "next.supervisor.final_failure",
            json!({
                "restartCount": *restart_count,
                "maxRestarts": NEXT_SUPERVISOR_MAX_RESTARTS,
                "message": message,
            }),
        );
        false
    }
}

fn should_restart_next_child(restart_count: u32, max_restarts: u32) -> bool {
    restart_count < max_restarts
}

fn child_exit_message(child_id: u32, exit_status: &ExitStatus) -> String {
    format!(
        "Next.js child process {child_id} exited with success={} and code={:?}.",
        exit_status.success(),
        exit_status.code()
    )
}

fn update_next_supervisor_status(
    next_status: &SharedNextSupervisorStatus,
    update: impl FnOnce(&mut NextSupervisorStatus),
) {
    if let Ok(mut status) = next_status.lock() {
        update(&mut status);
    }
}

fn next_supervisor_readiness_check(next_status: &SharedNextSupervisorStatus) -> HealthCheck {
    let status = match next_status.lock() {
        Ok(status) => status.clone(),
        Err(error) => {
            return HealthCheck {
                name: "next".to_string(),
                status: "error".to_string(),
                detail: format!("Next.js supervisor status lock failed: {error}."),
            }
        }
    };

    match status.phase {
        NextSupervisorPhase::Running => HealthCheck {
            name: "next".to_string(),
            status: "ok".to_string(),
            detail: status.detail,
        },
        NextSupervisorPhase::Starting | NextSupervisorPhase::Restarting => HealthCheck {
            name: "next".to_string(),
            status: "error".to_string(),
            detail: status.detail,
        },
        NextSupervisorPhase::Failed => HealthCheck {
            name: "next".to_string(),
            status: "error".to_string(),
            detail: status.detail,
        },
    }
}

fn spawn_system_brief_scheduler(
    db_path: Arc<PathBuf>,
    event_sender: broadcast::Sender<RealtimeEvent>,
) {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            match run_due_system_brief_schedules(&db_path) {
                Ok(briefs) => {
                    for brief in briefs {
                        emit_system_event(
                            &db_path,
                            &event_sender,
                            "brief.system.generated",
                            json!({
                                "briefId": brief.id,
                                "jobId": brief.job_id,
                                "version": brief.version,
                                "origin": "scheduler",
                            }),
                        );
                    }
                }
                Err(error) => {
                    emit_system_event(
                        &db_path,
                        &event_sender,
                        "brief.system.schedule_failed",
                        json!({ "message": error.to_string() }),
                    );
                }
            }
        }
    });
}
async fn health_handler() -> Json<HealthReport> {
    Json(build_health_report())
}

async fn ready_handler(State(state): State<AppState>) -> (StatusCode, Json<ReadinessReport>) {
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

async fn capabilities_handler(
    State(state): State<AppState>,
) -> Result<Json<CapabilityCatalogResponse>, (StatusCode, Json<ErrorResponse>)> {
    list_capabilities(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventReplayQuery {
    after: Option<i64>,
    limit: Option<usize>,
}

async fn latest_system_brief_handler(
    State(state): State<AppState>,
) -> Result<Json<LatestBriefResponse>, (StatusCode, Json<ErrorResponse>)> {
    latest_system_brief(&state.db_path)
        .map(|brief| Json(LatestBriefResponse { brief }))
        .map_err(internal_error)
}

async fn generate_system_brief_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<LatestBriefResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(&state.access_policy, &headers, remote_addr)?;
    let brief = generate_system_brief(&state.db_path, "http", None).map_err(internal_error)?;
    emit_system_event(
        &state.db_path,
        &state.event_sender,
        "brief.system.generated",
        json!({ "briefId": brief.id, "jobId": brief.job_id, "version": brief.version }),
    );
    Ok(Json(LatestBriefResponse { brief: Some(brief) }))
}

async fn list_backup_restore_handler(
    State(state): State<AppState>,
) -> Result<Json<BackupRestoreResponse>, (StatusCode, Json<ErrorResponse>)> {
    list_backup_restore_jobs(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

async fn create_backup_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<BackupRestoreResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(&state.access_policy, &headers, remote_addr)?;
    let job = create_backup(&state.db_path, "http", None).map_err(internal_error)?;
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

async fn validate_restore_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<RestorePreflightRequest>,
) -> Result<Json<BackupRestoreResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(&state.access_policy, &headers, remote_addr)?;
    let job =
        run_restore_preflight(&state.db_path, request, "http", None).map_err(internal_error)?;
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

async fn events_handler(
    State(state): State<AppState>,
    Query(query): Query<EventReplayQuery>,
) -> Result<Json<EventReplayResponse>, (StatusCode, Json<ErrorResponse>)> {
    replay_events(&state.db_path, query.after, query.limit)
        .map(Json)
        .map_err(internal_error)
}

async fn mcp_handler(
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    request_body: String,
) -> Result<Json<McpResponse>, (StatusCode, Json<ErrorResponse>)> {
    authorize_protected_daemon_route(&state.access_policy, &headers, remote_addr)?;
    Ok(Json(handle_mcp_json(&state.db_path, &request_body)))
}

fn authorize_protected_daemon_route(
    policy: &DaemonAccessPolicy,
    headers: &HeaderMap,
    remote_addr: SocketAddr,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if protected_daemon_route_allowed(policy, headers, remote_addr) {
        Ok(())
    } else {
        Err(forbidden_error(
            "Protected daemon route requires loopback access or a valid daemon access token.",
        ))
    }
}

fn protected_daemon_route_allowed(
    policy: &DaemonAccessPolicy,
    headers: &HeaderMap,
    remote_addr: SocketAddr,
) -> bool {
    remote_addr.ip().is_loopback()
        || policy
            .access_token
            .as_deref()
            .is_some_and(|token| request_has_access_token(headers, token))
}

fn request_has_access_token(headers: &HeaderMap, expected_token: &str) -> bool {
    headers
        .get(DAEMON_ACCESS_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|token| token == expected_token)
        || headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .is_some_and(|token| token == expected_token)
}

fn internal_error(error: anyhow::Error) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
}

fn forbidden_error(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: message.to_string(),
        }),
    )
}

fn emit_system_event(
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

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.event_sender.subscribe()))
}

async fn handle_socket(
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

async fn send_event(socket: &mut WebSocket, event: &RealtimeEvent) -> Result<(), axum::Error> {
    socket
        .send(Message::Text(
            serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string()),
        ))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
