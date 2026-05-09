# Backend Handoff Package 0.1.2

Status: backend handoff foundation implemented for the current 0.1.2 backend
contracts

This package is the handoff map for a future UI implementation agent. It records
which backend contracts are ready to consume, which routes are protected, which
scenarios should be seeded for smoke work, and which product boundaries remain
intentionally unbuilt.

## Handoff Posture

The backend is ready for UI planning against the 0.1.2 contracts when the UI
agent treats these facts as fixed:

- SQLite is the source of truth.
- WebSocket is a live projection only.
- MCP is a governed projection over the capability catalog, not an execution
  spine.
- Docker packages one appliance image and `.data` is the durable boundary.
- Protected local daemon routes require loopback-to-daemon access or the
  configured `x-ordo-daemon-token` header.
- Public routes must derive from explicitly public/published/read-model state.
- Support packets, answer drafts, MCP packs, handoff, and reports remain local
  unless a future approved transport slice adds egress.

## Contract Families

### System And Runtime

| Method | Route | Protection | Response Shape | UI Use |
| --- | --- | --- | --- | --- |
| `GET` | `/health` | public local | `HealthReport` | Daemon liveness and degraded state banners. |
| `GET` | `/ready` | public local | `ReadinessReport` | Appliance readiness checks. |
| `GET` | `/capabilities` | public local | `CapabilityCatalogResponse` | Inspect available governed actions. |
| `GET` | `/events` | public local | `EventReplayResponse` | Paginated persisted event history. |
| `GET` | `/logs` | protected | `DiagnosticLogsResponse` | Local diagnostic log inspection. |
| `GET` | `/policy-decisions` | protected | `PolicyDecisionAuditResponse` | Local policy/audit inspection. |
| `GET` | `/ws` | public local | WebSocket `RealtimeEvent` stream | Live projection for system activity. |
| `POST` | `/mcp` | local MCP JSON-RPC | `McpResponse` | Governed MCP projection only. |

Core response signals:

```json
{
  "status": "ok",
  "checks": [],
  "generatedAt": "2026-05-08T00:00:00Z"
}
```

```json
{
  "events": [{ "id": 1, "eventType": "daemon.started", "payload": {} }],
  "nextCursor": 2
}
```

### Install, Providers, And Local Vault

| Method | Route | Protection | Response Shape | UI Use |
| --- | --- | --- | --- | --- |
| `GET` | `/install/state` | protected | `InstallStateResponse` | First-run and configured-state screens. |
| `POST` | `/install/complete` | protected | `InstallStateResponse` | Persist local owner/business identity. |
| `GET` | `/providers` | protected | `ProviderListResponse` | Show redacted provider readiness. |
| `PUT` | `/providers/:provider_id` | protected | `ProviderConfigView` | Write provider metadata and secret refs. |

Secret boundary:

```json
{
  "providers": [{
    "id": "openai",
    "status": "configured",
    "secretSource": "local_vault",
    "secretPreview": "[REDACTED]"
  }]
}
```

Provider secrets are write-only through HTTP surfaces. UI must not expect API key
round-tripping.

### Business Truth And Public Surfaces

| Method | Route | Protection | Response Shape | UI Use |
| --- | --- | --- | --- | --- |
| `GET` | `/business/facts` | protected | `BusinessFactListResponse` | Owner/operator truth management. |
| `POST` | `/business/facts` | protected | `BusinessFactView` | Create fact with visibility/publication state. |
| `PUT` | `/business/facts/:fact_id` | protected | `BusinessFactView` | Update fact state. |
| `GET` | `/public/surfaces` | public | `PublicSurfacesResponse` | Aggregate public readiness. |
| `GET` | `/public/about` | public | `AboutReadModel` | Public About data. |
| `GET` | `/public/offers` | public | `OffersReadModel` | Public Offers data. |
| `GET` | `/public/asks` | public | `AsksReadModel` | Public Asks data. |
| `GET` | `/public/feed` | public | `FeedReadModel` | Public Feed data. |

Public read-model shape:

```json
{
  "readiness": "ready",
  "items": [{
    "id": "fact_1",
    "title": "Studio hours",
    "body": "By appointment",
    "provenance": { "resource": { "kind": "business_fact" } }
  }]
}
```

Public routes must never be backed by draft, private, staff, or owner-only facts.

### Entry Points, Visitors, Offers, And Trials

| Method | Route | Protection | Response Shape | UI Use |
| --- | --- | --- | --- | --- |
| `GET` | `/entry-points` | protected | `EntryPointListResponse` | Manage tracked QR/link/campaign entries. |
| `POST` | `/entry-points` | protected | `TrackedEntryPointView` | Create tracked entry point. |
| `PUT` | `/entry-points/:entry_point_id` | protected | `TrackedEntryPointView` | Update tracked entry point. |
| `GET` | `/public/e/:slug` | public | `PublicEntryPointView` | Resolve public-safe destination. |
| `POST` | `/public/visitor-sessions` | public | `VisitorSessionView` | Start attributed visitor session. |
| `GET` | `/visitor-sessions` | protected | `VisitorSessionListResponse` | Inspect session attribution. |
| `GET` | `/offers` | protected | `OfferListResponse` | Manage offers. |
| `POST` | `/offers` | protected | `OfferView` | Create offer. |
| `PUT` | `/offers/:offer_id` | protected | `OfferView` | Update offer. |
| `GET` | `/public/available-offers` | public | `PublicOfferListResponse` | Public-safe offer listing. |
| `POST` | `/public/offers/:offer_slug/accept` | public | `OfferAcceptanceResponse` | Accept public offer and start trial state. |
| `GET` | `/offer-acceptances` | protected | `OfferAcceptanceListResponse` | Inspect accepted offers. |
| `GET` | `/trials` | protected | `TrialListResponse` | Inspect trial lifecycle. |
| `PUT` | `/trials/:trial_id/status` | protected | `TrialView` | Convert, void, expire, or follow up. |

Offer/trial response cues:

```json
{
  "offer": { "id": "offer_1", "status": "available", "visibility": "public" },
  "trial": { "id": "trial_1", "status": "active", "startedAt": "..." },
  "attribution": { "entryPointId": "entry_1", "visitorSessionId": "session_1" }
}
```

Payments, affiliate payout automation, and external follow-up are not present.

### Connections, Availability, And Handoff

| Method | Route | Protection | Response Shape | UI Use |
| --- | --- | --- | --- | --- |
| `GET` | `/connections` | protected | `ConnectionListResponse` | Inspect trusted relationships. |
| `POST` | `/connections` | protected | `ConnectionView` | Create scoped connection. |
| `PUT` | `/connections/:connection_id` | protected | `ConnectionView` | Update connection status/scope. |
| `GET` | `/connections/:connection_id/grants` | protected | `ConnectionGrantListResponse` | Inspect resource grants. |
| `POST` | `/connections/:connection_id/grants` | protected | `ConnectionGrantView` | Add explicit grant. |
| `PUT` | `/connections/:connection_id/grants/:grant_id/revoke` | protected | `ConnectionGrantView` | Revoke grant. |
| `GET` | `/connections/:connection_id/events` | protected | `ConnectionEventListResponse` | Inspect connection history. |
| `GET` | `/availability` | protected | `AvailabilityStateResponse` | Read schedule/presence/threshold. |
| `PUT` | `/availability/schedule` | protected | `AvailabilityScheduleView` | Update local schedule. |
| `PUT` | `/availability/presence` | protected | `OperatorPresenceView` | Update operator presence. |
| `POST` | `/handoff/eligibility` | protected | `HandoffEligibilityView` | Evaluate owner attention boundary. |
| `GET` | `/handoff/inbox` | protected | `HandoffInboxListResponse` | Inspect local handoff queue. |
| `POST` | `/handoff/inbox` | protected | `HandoffInboxItemView` | Create local handoff item. |
| `PUT` | `/handoff/inbox/:item_id/resolve` | protected | `HandoffInboxItemView` | Approve/decline local-only item. |
| `GET` | `/handoff/inbox/:item_id/receipts` | protected | `HandoffReceiptListResponse` | Inspect local receipts. |

Handoff receipts are local evidence records. No route performs external handoff,
push notification, calendar sync, voice handoff, or mediated chat transport.

### Briefs, Backups, Restore, Reports, And Support Packets

| Method | Route | Protection | Response Shape | UI Use |
| --- | --- | --- | --- | --- |
| `GET` | `/briefs/system/latest` | public local | `LatestBriefResponse` | System Brief surface. |
| `POST` | `/briefs/system/generate` | protected | `LatestBriefResponse` | Generate local System Brief job/artifact. |
| `GET` | `/backups` | protected | `BackupRestoreResponse` | List backup/restore jobs. |
| `POST` | `/backups/create` | protected | `BackupRestoreResponse` | Create backup. |
| `POST` | `/restore/validate` | protected | `BackupRestoreResponse` | Restore preflight only. |
| `GET` | `/reports/issues` | protected | `IssueReportsResponse` | List local issue reports. |
| `POST` | `/reports/issues/prepare` | protected | `IssueReportDetailResponse` | Prepare report artifact. |
| `GET` | `/reports/issues/:report_id` | protected | `IssueReportDetailResponse` | Read report detail. |
| `PUT` | `/reports/issues/:report_id/status` | protected | `IssueReportDetailResponse` | Update local report status. |
| `POST` | `/reports/issues/:report_id/exports` | protected | `IssueReportExportResponse` | Save local markdown export. |
| `GET` | `/support-packets` | protected | `SupportPacketListResponse` | List local support packets. |
| `POST` | `/support-packets` | protected | `SupportPacketView` | Draft local packet preview. |
| `PUT` | `/support-packets/:packet_id/approve` | protected | `SupportPacketView` | Local-only approval record. |
| `GET` | `/support-packets/:packet_id/receipts` | protected | `SupportPacketReceiptListResponse` | Inspect receipt evidence. |

Restore validation is a preflight boundary. Support packet approval records
`approved_local_only` and `externalDelivery: false`.

### Corpus, Retrieval, And Answer Drafts

| Method | Route | Protection | Response Shape | UI Use |
| --- | --- | --- | --- | --- |
| `GET` | `/corpus/sources` | protected | `CorpusSourceListResponse` | Manage source records. |
| `POST` | `/corpus/sources` | protected | `CorpusSourceView` | Create source with provenance/classification. |
| `GET` | `/corpus/sources/:source_id` | protected | `CorpusSourceView` | Read source detail. |
| `PUT` | `/corpus/sources/:source_id` | protected | `CorpusSourceView` | Update source. |
| `GET` | `/corpus/items` | protected | `CorpusItemListResponse` | Manage source chunks/items. |
| `POST` | `/corpus/items` | protected | `CorpusItemView` | Create item and FTS row. |
| `GET` | `/corpus/items/:item_id` | protected | `CorpusItemView` | Read item detail. |
| `PUT` | `/corpus/items/:item_id` | protected | `CorpusItemView` | Update item and FTS row. |
| `POST` | `/corpus/retrieve` | protected | `CorpusRetrievalResponse` | Governed local FTS retrieval. |
| `GET` | `/answer-drafts` | protected | `AnswerDraftListResponse` | List local draft records. |
| `POST` | `/answer-drafts` | protected | `AnswerDraftResponse` | Prepare local evidence scaffold. |
| `GET` | `/answer-drafts/:draft_id` | protected | `AnswerDraftResponse` | Read draft/citations. |

Retrieval and answer draft response cues:

```json
{
  "query": "handoff hours",
  "evidenceState": "evidence_found",
  "results": [{
    "item": { "id": "corpus_item_1", "contentHash": "sha256:..." },
    "source": { "id": "corpus_source_1" },
    "snippet": "...",
    "evidence": { "generatedAnswer": false }
  }],
  "limitations": ["Local SQLite FTS retrieval only."]
}
```

```json
{
  "draft": {
    "status": "drafted_with_evidence",
    "citedItemIds": ["corpus_item_1"],
    "draftMarkdown": "## Evidence-Backed Draft\n...",
    "limitations": ["No provider or model call was performed in this backend slice."],
    "citations": [{ "corpusItemId": "corpus_item_1", "contentHash": "sha256:..." }]
  }
}
```

If evidence is missing, answer drafts use `needs_evidence` and do not generate
source claims. Provider-backed answer generation, embeddings, and vector search
are future work.

### MCP Pack Metadata

| Method | Route | Protection | Response Shape | UI Use |
| --- | --- | --- | --- | --- |
| `GET` | `/mcp/packs` | protected | `McpPackListResponse` | Inspect local pack metadata. |
| `POST` | `/mcp/packs` | protected | `McpPackResponse` | Validate/install/update pack metadata. |
| `GET` | `/mcp/packs/:pack_id` | protected | `McpPackResponse` | Read pack detail/tool export state. |
| `PUT` | `/mcp/packs/:pack_id/disable` | protected | `McpPackResponse` | Disable local pack tools. |
| `POST` | `/mcp` with `tools/list` | local MCP | `McpResponse` | List exported governed tools. |
| `POST` | `/mcp` with `tools/call` | local MCP | `McpResponse` | Call hard-coded governed projections. |

Pack response cue:

```json
{
  "pack": {
    "id": "pack.local.status",
    "status": "enabled",
    "tools": [{
      "toolName": "system.status.read",
      "capabilityId": "system.status.read",
      "mcpExportPolicy": "read_only",
      "exportStatus": "exported"
    }]
  }
}
```

Packs are metadata over existing capabilities. They cannot introduce shell
commands, native plugins, hosted registry tools, provider/model transport, or
external egress.

## Smoke Seed Scenarios

Use these as fixture goals for the first UI agent. They can be produced by HTTP
requests, direct test helpers, or future seed scripts, but each scenario must
respect the same daemon policy boundaries.

| Scenario | Minimum Seed State | Expected UI Proof |
| --- | --- | --- |
| First run | Fresh SQLite database after daemon init. | `/ready` is ready, install state shows incomplete owner/business setup, public surfaces show explicit missing readiness. |
| Configured provider | Completed install, provider config row with write-only secret ref. | Providers page can show configured/redacted state without revealing a secret. |
| Public surface | Published public business facts for About, Offers, Asks, and Feed. | Public read-model routes return ready state and only public records. |
| Visitor journey | Public entry point, visitor session, public offer, offer acceptance, active trial. | Attribution is visible from entry point through trial without payments. |
| Availability and handoff | Availability schedule, operator presence, eligibility request, inbox item, local receipt. | Owner attention UI can show eligible/not eligible evidence and local-only resolution. |
| Reports/support packets | Prepared issue report, export, support packet draft, local approval receipt. | Reports UI can preview/export local markdown and show `externalDelivery: false`. |
| Corpus retrieval | Approved public corpus source and item. | Retrieval returns cited source/item evidence and limitations. |
| Answer draft | Corpus evidence plus answer draft prepare request. | Draft records cite item IDs; missing evidence produces `needs_evidence`. |
| MCP pack metadata | Local pack manifest over an existing capability, then disabled. | Pack UI can show enabled/disabled/blocked states; MCP hides disabled tools. |

## Validation Commands

For backend handoff documentation-only changes, the minimum proof is:

```bash
git diff --check
```

For source, schema, seed helper, or route changes, run the full matrix:

```bash
npm run typecheck
npm run build
npm run smoke:ui
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Expected proof points for the full matrix:

- frontend typecheck and production build pass;
- UI smoke tests pass across the existing desktop/mobile Chromium coverage;
- Rust formatting is clean;
- all workspace tests pass;
- Clippy passes with warnings denied;
- diff whitespace is clean.

## Known Non-Goals At Handoff

The UI agent must not infer these as available:

- product-depth frontend surfaces such as Studio, Today, Conversations, People,
  public About/Offers/Asks/Feed pages, or install wizard UI;
- hosted identity, OAuth/email login, multi-user public portals, or external
  connection integrations;
- external egress for support packets, handoff, reports, provider validation,
  or pack registries;
- payments, affiliate payouts, analytics dashboards, mediated chat UI, push
  notifications, calendar sync, or voice handoff;
- embeddings, vector search, provider-backed RAG answer generation, or provider
  calls for answers;
- arbitrary plugin execution, shell/native command execution, third-party MCP
  marketplaces, or Worker Ordo/A2A networking;
- broad visual regression coverage beyond the current smoke suite.

## UI Agent Starting Points

A future UI implementation agent should begin with:

1. Read [State Of The Project](../state-of-the-project.md) for shipped versus
   planned behavior.
2. Read [Backend MVP Execution Path](../backlog/backend-mvp-execution-path.md)
   for phase status and backend handoff criteria.
3. Read the architecture docs linked from this package for the contract family
   being implemented.
4. Build UI against the response shapes above and the concrete Rust structs in
   `crates/ordo-daemon/src/`.
5. Add UI smoke coverage only for user-visible behavior introduced by the UI
   slice.

This package is a map, not a generated client. If code and docs ever disagree,
code plus tests are the immediate truth and docs should be corrected in the same
PR.
