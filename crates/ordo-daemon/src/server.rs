use anyhow::{Context, Result};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use serde_json::json;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
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
use crate::events::{system_event, RealtimeEvent};
use crate::health::{build_health_report, build_readiness_report, HealthReport, ReadinessReport};
use crate::mcp::{handle_mcp_request, McpRequest, McpResponse};
use crate::schema::init_database;

#[derive(Clone)]
struct AppState {
    db_path: Arc<PathBuf>,
    event_sender: broadcast::Sender<RealtimeEvent>,
}

#[derive(Debug, Clone)]
pub struct NextSupervisorConfig {
    pub command: String,
    pub args: Vec<String>,
}

pub async fn serve(
    host: String,
    port: u16,
    db_path: PathBuf,
    next_supervisor: Option<NextSupervisorConfig>,
) -> Result<()> {
    init_database(&db_path)?;
    let _generated_briefs = run_due_system_brief_schedules(&db_path)?;

    let (event_sender, _) = broadcast::channel(128);
    let state = AppState {
        db_path: Arc::new(db_path),
        event_sender,
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
        .route("/mcp", post(mcp_handler))
        .route("/ws", get(ws_handler))
        .with_state(state.clone());

    let _ = state.event_sender.send(system_event(
        "daemon.started",
        json!({ "host": host, "port": port }),
    ));
    if let Some(config) = next_supervisor {
        spawn_next_supervisor(config, state.event_sender.clone())?;
    }
    spawn_system_brief_scheduler(state.db_path.clone(), state.event_sender.clone());

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn spawn_next_supervisor(
    config: NextSupervisorConfig,
    event_sender: broadcast::Sender<RealtimeEvent>,
) -> Result<()> {
    let mut child = Command::new(&config.command)
        .args(&config.args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("Failed to start Next.js child process: {}", config.command))?;
    let child_id = child.id();
    let _ = event_sender.send(system_event(
        "next.supervisor.started",
        json!({ "command": config.command, "args": config.args, "pid": child_id }),
    ));

    thread::Builder::new()
        .name("ordo-next-supervisor".to_string())
        .spawn(move || match child.wait() {
            Ok(status) => {
                let _ = event_sender.send(system_event(
                    "next.supervisor.exited",
                    json!({ "pid": child_id, "success": status.success(), "code": status.code() }),
                ));
            }
            Err(error) => {
                let _ = event_sender.send(system_event(
                    "next.supervisor.failed",
                    json!({ "pid": child_id, "message": error.to_string() }),
                ));
            }
        })
        .context("Failed to spawn Next.js supervisor thread")?;
    Ok(())
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
                        let _ = event_sender.send(system_event(
                            "brief.system.generated",
                            json!({
                                "briefId": brief.id,
                                "jobId": brief.job_id,
                                "version": brief.version,
                                "origin": "scheduler",
                            }),
                        ));
                    }
                }
                Err(error) => {
                    let _ = event_sender.send(system_event(
                        "brief.system.schedule_failed",
                        json!({ "message": error.to_string() }),
                    ));
                }
            }
        }
    });
}
async fn health_handler() -> Json<HealthReport> {
    Json(build_health_report())
}

async fn ready_handler(State(state): State<AppState>) -> (StatusCode, Json<ReadinessReport>) {
    let report = build_readiness_report(&state.db_path);
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

async fn latest_system_brief_handler(
    State(state): State<AppState>,
) -> Result<Json<LatestBriefResponse>, (StatusCode, Json<ErrorResponse>)> {
    latest_system_brief(&state.db_path)
        .map(|brief| Json(LatestBriefResponse { brief }))
        .map_err(internal_error)
}

async fn generate_system_brief_handler(
    State(state): State<AppState>,
) -> Result<Json<LatestBriefResponse>, (StatusCode, Json<ErrorResponse>)> {
    let brief = generate_system_brief(&state.db_path, "http", None).map_err(internal_error)?;
    let _ = state.event_sender.send(system_event(
        "brief.system.generated",
        json!({ "briefId": brief.id, "jobId": brief.job_id, "version": brief.version }),
    ));
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
    State(state): State<AppState>,
) -> Result<Json<BackupRestoreResponse>, (StatusCode, Json<ErrorResponse>)> {
    let job = create_backup(&state.db_path, "http", None).map_err(internal_error)?;
    let _ = state.event_sender.send(system_event(
        "backup.create.completed",
        json!({ "jobId": job.id, "artifactId": job.artifact.as_ref().map(|artifact| artifact.id.clone()) }),
    ));
    list_backup_restore_jobs(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

async fn validate_restore_handler(
    State(state): State<AppState>,
    Json(request): Json<RestorePreflightRequest>,
) -> Result<Json<BackupRestoreResponse>, (StatusCode, Json<ErrorResponse>)> {
    let job =
        run_restore_preflight(&state.db_path, request, "http", None).map_err(internal_error)?;
    let _ = state.event_sender.send(system_event(
        "restore.preflight.completed",
        json!({ "jobId": job.id, "status": job.status }),
    ));
    list_backup_restore_jobs(&state.db_path)
        .map(Json)
        .map_err(internal_error)
}

async fn mcp_handler(
    State(state): State<AppState>,
    Json(request): Json<McpRequest>,
) -> Json<McpResponse> {
    Json(handle_mcp_request(&state.db_path, request))
}

fn internal_error(error: anyhow::Error) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: error.to_string(),
        }),
    )
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
