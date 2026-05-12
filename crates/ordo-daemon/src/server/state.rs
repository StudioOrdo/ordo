use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;
use tokio::sync::broadcast;

use crate::conversation_protocol::ConversationGatewayEnvelope;
use crate::events::RealtimeEvent;

pub(crate) const NEXT_SUPERVISOR_MAX_RESTARTS: u32 = 3;
pub(crate) const NEXT_SUPERVISOR_RESTART_DELAY: StdDuration = StdDuration::from_secs(1);
pub(crate) const DAEMON_ACCESS_TOKEN_HEADER: &str = "x-ordo-daemon-token";

pub(crate) type SharedNextSupervisorStatus = Arc<Mutex<NextSupervisorStatus>>;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db_path: Arc<PathBuf>,
    pub(crate) event_sender: broadcast::Sender<RealtimeEvent>,
    pub(crate) conversation_sender: broadcast::Sender<ConversationGatewayEnvelope>,
    pub(crate) next_supervisor_status: Option<SharedNextSupervisorStatus>,
    pub(crate) access_policy: DaemonAccessPolicy,
}

#[derive(Debug, Clone)]
pub(crate) struct DaemonAccessPolicy {
    pub(crate) access_token: Option<String>,
}

impl DaemonAccessPolicy {
    pub(crate) fn new(access_token: Option<String>) -> Self {
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
pub(crate) enum NextSupervisorPhase {
    Starting,
    Running,
    Restarting,
    Failed,
}

#[derive(Debug, Clone)]
pub(crate) struct NextSupervisorStatus {
    pub(crate) phase: NextSupervisorPhase,
    pub(crate) pid: Option<u32>,
    pub(crate) restart_count: u32,
    pub(crate) detail: String,
}

impl NextSupervisorStatus {
    pub(crate) fn starting() -> Self {
        Self {
            phase: NextSupervisorPhase::Starting,
            pid: None,
            restart_count: 0,
            detail: "Next.js child process is starting.".to_string(),
        }
    }
}
