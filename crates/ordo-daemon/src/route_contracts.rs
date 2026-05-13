use crate::policy::PolicyAction;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
}

impl HttpMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteProtection {
    Protected {
        action: PolicyAction,
        capability_id: &'static str,
    },
    PublicLocal,
    Public,
    LocalMcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonRouteContract {
    pub method: HttpMethod,
    pub pattern: &'static str,
    pub sample_route: &'static str,
    pub protection: RouteProtection,
}

pub const DAEMON_ROUTE_CONTRACTS: &[DaemonRouteContract] = &[
    public_local(HttpMethod::Get, "/health", "/health"),
    public_local(HttpMethod::Get, "/ready", "/ready"),
    public_local(HttpMethod::Get, "/capabilities", "/capabilities"),
    public_local(HttpMethod::Get, "/events", "/events"),
    protected(
        HttpMethod::Get,
        "/logs",
        "/logs",
        PolicyAction::Inspect,
        "diagnostic.logs.list",
    ),
    protected(
        HttpMethod::Get,
        "/policy-decisions",
        "/policy-decisions",
        PolicyAction::Inspect,
        "policy.decisions.list",
    ),
    public_local(HttpMethod::Get, "/ws", "/ws"),
    protected(
        HttpMethod::Get,
        "/chat/ws",
        "/chat/ws",
        PolicyAction::Read,
        "conversation.read",
    ),
    protected(
        HttpMethod::Post,
        "/chat/bootstrap",
        "/chat/bootstrap",
        PolicyAction::Create,
        "conversation.bootstrap",
    ),
    local_mcp(HttpMethod::Post, "/mcp", "/mcp"),
    protected(
        HttpMethod::Get,
        "/install/state",
        "/install/state",
        PolicyAction::Inspect,
        "install.state.read",
    ),
    protected(
        HttpMethod::Post,
        "/install/complete",
        "/install/complete",
        PolicyAction::Create,
        "install.complete",
    ),
    protected(
        HttpMethod::Post,
        "/local-sessions/login",
        "/local-sessions/login",
        PolicyAction::Create,
        "local_session.login",
    ),
    protected(
        HttpMethod::Post,
        "/local-sessions/register",
        "/local-sessions/register",
        PolicyAction::Create,
        "local_session.register",
    ),
    protected(
        HttpMethod::Get,
        "/providers",
        "/providers",
        PolicyAction::Inspect,
        "providers.list",
    ),
    protected(
        HttpMethod::Put,
        "/providers/:provider_id",
        "/providers/openai",
        PolicyAction::Update,
        "providers.update",
    ),
    protected(
        HttpMethod::Get,
        "/business/facts",
        "/business/facts",
        PolicyAction::Inspect,
        "business.facts.list",
    ),
    protected(
        HttpMethod::Post,
        "/business/facts",
        "/business/facts",
        PolicyAction::Create,
        "business.facts.write",
    ),
    protected(
        HttpMethod::Put,
        "/business/facts/:fact_id",
        "/business/facts/fact_1",
        PolicyAction::Update,
        "business.facts.write",
    ),
    public(HttpMethod::Get, "/public/surfaces", "/public/surfaces"),
    public(HttpMethod::Get, "/public/about", "/public/about"),
    public(HttpMethod::Get, "/public/offers", "/public/offers"),
    public(HttpMethod::Get, "/public/asks", "/public/asks"),
    public(HttpMethod::Get, "/public/feed", "/public/feed"),
    protected(
        HttpMethod::Get,
        "/entry-points",
        "/entry-points",
        PolicyAction::Inspect,
        "entry_points.list",
    ),
    protected(
        HttpMethod::Post,
        "/entry-points",
        "/entry-points",
        PolicyAction::Create,
        "entry_points.write",
    ),
    protected(
        HttpMethod::Put,
        "/entry-points/:entry_point_id",
        "/entry-points/entry_1",
        PolicyAction::Update,
        "entry_points.write",
    ),
    public(HttpMethod::Get, "/public/e/:slug", "/public/e/smoke"),
    public(
        HttpMethod::Post,
        "/public/visitor-sessions",
        "/public/visitor-sessions",
    ),
    protected(
        HttpMethod::Get,
        "/visitor-sessions",
        "/visitor-sessions",
        PolicyAction::Inspect,
        "visitor_sessions.list",
    ),
    protected(
        HttpMethod::Get,
        "/offers",
        "/offers",
        PolicyAction::Inspect,
        "offers.list",
    ),
    protected(
        HttpMethod::Post,
        "/offers",
        "/offers",
        PolicyAction::Create,
        "offers.write",
    ),
    protected(
        HttpMethod::Put,
        "/offers/:offer_id",
        "/offers/offer_1",
        PolicyAction::Update,
        "offers.write",
    ),
    public(
        HttpMethod::Get,
        "/public/available-offers",
        "/public/available-offers",
    ),
    public(
        HttpMethod::Post,
        "/public/offers/:offer_slug/accept",
        "/public/offers/offer-smoke/accept",
    ),
    protected(
        HttpMethod::Get,
        "/offer-acceptances",
        "/offer-acceptances",
        PolicyAction::Inspect,
        "offer_acceptances.list",
    ),
    protected(
        HttpMethod::Get,
        "/trials",
        "/trials",
        PolicyAction::Inspect,
        "trials.list",
    ),
    protected(
        HttpMethod::Put,
        "/trials/:trial_id/status",
        "/trials/trial_1/status",
        PolicyAction::Create,
        "trials.transition",
    ),
    protected(
        HttpMethod::Get,
        "/connections",
        "/connections",
        PolicyAction::Inspect,
        "connections.list",
    ),
    protected(
        HttpMethod::Post,
        "/connections",
        "/connections",
        PolicyAction::Create,
        "connections.write",
    ),
    protected(
        HttpMethod::Put,
        "/connections/:connection_id",
        "/connections/connection_1",
        PolicyAction::Update,
        "connections.write",
    ),
    protected(
        HttpMethod::Get,
        "/connections/:connection_id/grants",
        "/connections/connection_1/grants",
        PolicyAction::Inspect,
        "connection_grants.list",
    ),
    protected(
        HttpMethod::Post,
        "/connections/:connection_id/grants",
        "/connections/connection_1/grants",
        PolicyAction::Create,
        "connection_grants.write",
    ),
    protected(
        HttpMethod::Put,
        "/connections/:connection_id/grants/:grant_id/revoke",
        "/connections/connection_1/grants/grant_1/revoke",
        PolicyAction::Update,
        "connection_grants.write",
    ),
    protected(
        HttpMethod::Get,
        "/connections/:connection_id/events",
        "/connections/connection_1/events",
        PolicyAction::Inspect,
        "connection_events.list",
    ),
    protected(
        HttpMethod::Get,
        "/availability",
        "/availability",
        PolicyAction::Inspect,
        "availability.read",
    ),
    protected(
        HttpMethod::Put,
        "/availability/schedule",
        "/availability/schedule",
        PolicyAction::Create,
        "availability.write",
    ),
    protected(
        HttpMethod::Put,
        "/availability/presence",
        "/availability/presence",
        PolicyAction::Create,
        "availability.write",
    ),
    protected(
        HttpMethod::Post,
        "/handoff/eligibility",
        "/handoff/eligibility",
        PolicyAction::Inspect,
        "handoff.eligibility.evaluate",
    ),
    protected(
        HttpMethod::Get,
        "/handoff/inbox",
        "/handoff/inbox",
        PolicyAction::Inspect,
        "handoff.inbox.list",
    ),
    protected(
        HttpMethod::Post,
        "/handoff/inbox",
        "/handoff/inbox",
        PolicyAction::Create,
        "handoff.inbox.write",
    ),
    protected(
        HttpMethod::Get,
        "/handoff/inbox/:item_id",
        "/handoff/inbox/handoff_item_1",
        PolicyAction::Inspect,
        "handoff.inbox.list",
    ),
    protected(
        HttpMethod::Put,
        "/handoff/inbox/:item_id",
        "/handoff/inbox/handoff_item_1",
        PolicyAction::Update,
        "handoff.inbox.write",
    ),
    protected(
        HttpMethod::Put,
        "/handoff/inbox/:item_id/resolve",
        "/handoff/inbox/handoff_item_1/resolve",
        PolicyAction::Create,
        "handoff.inbox.write",
    ),
    protected(
        HttpMethod::Get,
        "/handoff/inbox/:item_id/receipts",
        "/handoff/inbox/handoff_item_1/receipts",
        PolicyAction::Inspect,
        "handoff.receipts.list",
    ),
    public_local(
        HttpMethod::Get,
        "/briefs/system/latest",
        "/briefs/system/latest",
    ),
    protected(
        HttpMethod::Post,
        "/briefs/system/generate",
        "/briefs/system/generate",
        PolicyAction::Generate,
        "brief.system.generate",
    ),
    protected(
        HttpMethod::Get,
        "/backups",
        "/backups",
        PolicyAction::Inspect,
        "backup.restore_jobs.list",
    ),
    protected(
        HttpMethod::Post,
        "/backups/create",
        "/backups/create",
        PolicyAction::Create,
        "backup.create",
    ),
    protected(
        HttpMethod::Post,
        "/restore/validate",
        "/restore/validate",
        PolicyAction::Validate,
        "restore.preflight.validate",
    ),
    protected(
        HttpMethod::Get,
        "/surface/work-items",
        "/surface/work-items",
        PolicyAction::Inspect,
        "surface.work_items.list",
    ),
    protected(
        HttpMethod::Get,
        "/reports/issues",
        "/reports/issues",
        PolicyAction::Inspect,
        "issue.report.list",
    ),
    protected(
        HttpMethod::Post,
        "/reports/issues/prepare",
        "/reports/issues/prepare",
        PolicyAction::Prepare,
        "issue.report.prepare",
    ),
    protected(
        HttpMethod::Get,
        "/reports/issues/:report_id",
        "/reports/issues/report_1",
        PolicyAction::Inspect,
        "issue.report.detail",
    ),
    protected(
        HttpMethod::Put,
        "/reports/issues/:report_id/status",
        "/reports/issues/report_1/status",
        PolicyAction::Update,
        "issue.report.status.update",
    ),
    protected(
        HttpMethod::Post,
        "/reports/issues/:report_id/exports",
        "/reports/issues/report_1/exports",
        PolicyAction::Export,
        "issue.report.export",
    ),
    protected(
        HttpMethod::Get,
        "/support-packets",
        "/support-packets",
        PolicyAction::Inspect,
        "support.packets.list",
    ),
    protected(
        HttpMethod::Post,
        "/support-packets",
        "/support-packets",
        PolicyAction::Prepare,
        "support.packets.draft",
    ),
    protected(
        HttpMethod::Put,
        "/support-packets/:packet_id/approve",
        "/support-packets/packet_1/approve",
        PolicyAction::Approve,
        "support.packets.approve",
    ),
    protected(
        HttpMethod::Get,
        "/support-packets/:packet_id/receipts",
        "/support-packets/packet_1/receipts",
        PolicyAction::Inspect,
        "support.packet.receipts.list",
    ),
    protected(
        HttpMethod::Get,
        "/corpus/sources",
        "/corpus/sources",
        PolicyAction::Inspect,
        "corpus.sources.list",
    ),
    protected(
        HttpMethod::Post,
        "/corpus/sources",
        "/corpus/sources",
        PolicyAction::Create,
        "corpus.sources.write",
    ),
    protected(
        HttpMethod::Get,
        "/corpus/sources/:source_id",
        "/corpus/sources/source_1",
        PolicyAction::Inspect,
        "corpus.sources.list",
    ),
    protected(
        HttpMethod::Put,
        "/corpus/sources/:source_id",
        "/corpus/sources/source_1",
        PolicyAction::Update,
        "corpus.sources.write",
    ),
    protected(
        HttpMethod::Get,
        "/corpus/items",
        "/corpus/items",
        PolicyAction::Inspect,
        "corpus.items.list",
    ),
    protected(
        HttpMethod::Post,
        "/corpus/items",
        "/corpus/items",
        PolicyAction::Create,
        "corpus.items.write",
    ),
    protected(
        HttpMethod::Get,
        "/corpus/items/:item_id",
        "/corpus/items/item_1",
        PolicyAction::Inspect,
        "corpus.items.list",
    ),
    protected(
        HttpMethod::Put,
        "/corpus/items/:item_id",
        "/corpus/items/item_1",
        PolicyAction::Update,
        "corpus.items.write",
    ),
    protected(
        HttpMethod::Post,
        "/corpus/retrieve",
        "/corpus/retrieve",
        PolicyAction::Read,
        "corpus.retrieve",
    ),
    protected(
        HttpMethod::Get,
        "/answer-drafts",
        "/answer-drafts",
        PolicyAction::Inspect,
        "answer.drafts.list",
    ),
    protected(
        HttpMethod::Post,
        "/answer-drafts",
        "/answer-drafts",
        PolicyAction::Prepare,
        "answer.drafts.prepare",
    ),
    protected(
        HttpMethod::Get,
        "/answer-drafts/:draft_id",
        "/answer-drafts/answer_draft_1",
        PolicyAction::Inspect,
        "answer.drafts.list",
    ),
    protected(
        HttpMethod::Get,
        "/mcp/packs",
        "/mcp/packs",
        PolicyAction::Inspect,
        "mcp.packs.list",
    ),
    protected(
        HttpMethod::Post,
        "/mcp/packs",
        "/mcp/packs",
        PolicyAction::Validate,
        "mcp.packs.write",
    ),
    protected(
        HttpMethod::Get,
        "/mcp/packs/:pack_id",
        "/mcp/packs/pack.local.status",
        PolicyAction::Inspect,
        "mcp.packs.list",
    ),
    protected(
        HttpMethod::Put,
        "/mcp/packs/:pack_id/disable",
        "/mcp/packs/pack.local.status/disable",
        PolicyAction::Update,
        "mcp.packs.write",
    ),
];

pub fn protected_route_contracts() -> impl Iterator<Item = &'static DaemonRouteContract> {
    DAEMON_ROUTE_CONTRACTS
        .iter()
        .filter(|contract| matches!(contract.protection, RouteProtection::Protected { .. }))
}

pub fn protected_route_capability_ids() -> BTreeSet<&'static str> {
    protected_route_contracts()
        .filter_map(|contract| match contract.protection {
            RouteProtection::Protected { capability_id, .. } => Some(capability_id),
            _ => None,
        })
        .collect()
}

const fn protected(
    method: HttpMethod,
    pattern: &'static str,
    sample_route: &'static str,
    action: PolicyAction,
    capability_id: &'static str,
) -> DaemonRouteContract {
    DaemonRouteContract {
        method,
        pattern,
        sample_route,
        protection: RouteProtection::Protected {
            action,
            capability_id,
        },
    }
}

const fn public_local(
    method: HttpMethod,
    pattern: &'static str,
    sample_route: &'static str,
) -> DaemonRouteContract {
    DaemonRouteContract {
        method,
        pattern,
        sample_route,
        protection: RouteProtection::PublicLocal,
    }
}

const fn public(
    method: HttpMethod,
    pattern: &'static str,
    sample_route: &'static str,
) -> DaemonRouteContract {
    DaemonRouteContract {
        method,
        pattern,
        sample_route,
        protection: RouteProtection::Public,
    }
}

const fn local_mcp(
    method: HttpMethod,
    pattern: &'static str,
    sample_route: &'static str,
) -> DaemonRouteContract {
    DaemonRouteContract {
        method,
        pattern,
        sample_route,
        protection: RouteProtection::LocalMcp,
    }
}
