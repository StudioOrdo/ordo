use anyhow::{Context, Result};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::Duration as StdDuration;
use tokio::sync::broadcast;
use tokio::time::{self, Duration};

use crate::briefs::run_due_system_brief_schedules;
use crate::events::RealtimeEvent;
use crate::health::HealthCheck;

const NEXT_SUPERVISOR_MAX_RESTARTS: u32 = 3;
const NEXT_SUPERVISOR_RESTART_DELAY: StdDuration = StdDuration::from_secs(1);
const DAEMON_ACCESS_TOKEN_HEADER: &str = "x-ordo-daemon-token";

use super::handlers::*;
use super::state::*;
pub(crate) fn spawn_next_supervisor(
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

pub(crate) fn should_restart_next_child(restart_count: u32, max_restarts: u32) -> bool {
    restart_count < max_restarts
}

fn child_exit_message(child_id: u32, exit_status: &ExitStatus) -> String {
    format!(
        "Next.js child process {child_id} exited with success={} and code={:?}.",
        exit_status.success(),
        exit_status.code()
    )
}

pub(crate) fn update_next_supervisor_status(
    next_status: &SharedNextSupervisorStatus,
    update: impl FnOnce(&mut NextSupervisorStatus),
) {
    if let Ok(mut status) = next_status.lock() {
        update(&mut status);
    }
}

pub(crate) fn next_supervisor_readiness_check(
    next_status: &SharedNextSupervisorStatus,
) -> HealthCheck {
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

pub(crate) fn spawn_system_brief_scheduler(
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
