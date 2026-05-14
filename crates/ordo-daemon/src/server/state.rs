use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

use crate::conversation_protocol::ConversationGatewayEnvelope;
use crate::events::RealtimeEvent;
use crate::secrets::{normalize_secret, OrdoSecretString};

const PROTECTED_ROUTE_RATE_LIMIT_MAX_ATTEMPTS: u32 = 30;
const PROTECTED_ROUTE_RATE_LIMIT_WINDOW_SECONDS: i64 = 60;

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
    pub(crate) access_token: Option<OrdoSecretString>,
    pub(crate) rate_limiter: ProtectedRouteRateLimiter,
}

impl DaemonAccessPolicy {
    pub(crate) fn new(access_token: Option<String>) -> Self {
        Self {
            access_token: access_token.and_then(normalize_secret),
            rate_limiter: ProtectedRouteRateLimiter::default(),
        }
    }
}

// First abuse guard: local, in-memory throttling for repeated failed protected
// daemon route attempts. It intentionally does not throttle valid loopback or
// token-authorized appliance traffic, and can be replaced by a keyed provider
// limiter once live-provider cost controls need per-actor/provider budgets.
#[derive(Debug, Clone)]
pub(crate) struct ProtectedRouteRateLimiter {
    state: Arc<Mutex<HashMap<String, ProtectedRouteRateLimitBucket>>>,
    max_attempts: u32,
    window_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProtectedRouteRateLimitDecision {
    pub(crate) allowed: bool,
    pub(crate) retry_after_seconds: Option<i64>,
    pub(crate) remaining_attempts: u32,
}

#[derive(Debug, Clone)]
struct ProtectedRouteRateLimitBucket {
    window_started_at: i64,
    attempt_count: u32,
}

impl Default for ProtectedRouteRateLimiter {
    fn default() -> Self {
        Self::new(
            PROTECTED_ROUTE_RATE_LIMIT_MAX_ATTEMPTS,
            PROTECTED_ROUTE_RATE_LIMIT_WINDOW_SECONDS,
        )
    }
}

impl ProtectedRouteRateLimiter {
    pub(crate) fn new(max_attempts: u32, window_seconds: i64) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_attempts: max_attempts.max(1),
            window_seconds: window_seconds.max(1),
        }
    }

    pub(crate) fn check(&self, key: &str) -> ProtectedRouteRateLimitDecision {
        self.check_at(key, Utc::now().timestamp())
    }

    pub(crate) fn check_at(
        &self,
        key: &str,
        now_seconds: i64,
    ) -> ProtectedRouteRateLimitDecision {
        let mut state = self
            .state
            .lock()
            .expect("protected route rate limiter mutex poisoned");
        state.retain(|_, bucket| now_seconds - bucket.window_started_at < self.window_seconds);
        let bucket = state
            .entry(key.to_string())
            .or_insert(ProtectedRouteRateLimitBucket {
                window_started_at: now_seconds,
                attempt_count: 0,
            });
        if now_seconds < bucket.window_started_at
            || now_seconds - bucket.window_started_at >= self.window_seconds
        {
            bucket.window_started_at = now_seconds;
            bucket.attempt_count = 0;
        }
        if bucket.attempt_count >= self.max_attempts {
            let retry_after = (bucket.window_started_at + self.window_seconds - now_seconds).max(1);
            return ProtectedRouteRateLimitDecision {
                allowed: false,
                retry_after_seconds: Some(retry_after),
                remaining_attempts: 0,
            };
        }
        bucket.attempt_count += 1;
        ProtectedRouteRateLimitDecision {
            allowed: true,
            retry_after_seconds: None,
            remaining_attempts: self.max_attempts.saturating_sub(bucket.attempt_count),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::expose_secret;

    #[test]
    fn daemon_access_policy_wraps_token_as_redacted_secret() {
        let policy = DaemonAccessPolicy::new(Some("  daemon-token-secret  ".to_string()));

        let token = policy.access_token.as_ref().unwrap();
        assert_eq!(expose_secret(token), "daemon-token-secret");
        assert!(!format!("{policy:?}").contains("daemon-token-secret"));
    }

    #[test]
    fn daemon_access_policy_ignores_empty_token() {
        let policy = DaemonAccessPolicy::new(Some("  ".to_string()));

        assert!(policy.access_token.is_none());
    }

    #[test]
    fn protected_route_rate_limiter_blocks_after_configured_attempts() {
        let limiter = ProtectedRouteRateLimiter::new(2, 60);

        assert!(limiter.check_at("192.168.1.10|/providers", 1_000).allowed);
        assert!(limiter.check_at("192.168.1.10|/providers", 1_010).allowed);
        let blocked = limiter.check_at("192.168.1.10|/providers", 1_020);

        assert!(!blocked.allowed);
        assert_eq!(blocked.remaining_attempts, 0);
        assert_eq!(blocked.retry_after_seconds, Some(40));
    }

    #[test]
    fn protected_route_rate_limiter_resets_after_window() {
        let limiter = ProtectedRouteRateLimiter::new(1, 60);

        assert!(limiter.check_at("192.168.1.10|/providers", 1_000).allowed);
        assert!(!limiter.check_at("192.168.1.10|/providers", 1_001).allowed);
        assert!(limiter.check_at("192.168.1.10|/providers", 1_060).allowed);
    }
}
