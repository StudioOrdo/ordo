# State Of The Project

Date: 2026-05-08

Ordo has completed the 0.1.0 Appliance Specimen implementation pass and is
working through the 0.1.1 appliance stabilization slices before deeper product
surfaces land.

## What Is Real Now

- The repository exists under `StudioOrdo/ordo`.
- The project is licensed as AGPL-3.0-only.
- The public README, architecture docs, process docs, and release evidence are
  established through GitHub issue and pull request workflow.
- The process is evidence-backed software manufacturing in public.
- The Rust daemon initializes SQLite, reports health/readiness, owns the job
  kernel, runs scheduled System Brief generation, creates backups, validates
  restore preflight, exposes WebSocket events, and serves a capability/MCP
  projection.
- The Next.js System shell renders Brief, Health, Backup And Restore,
  Schedules, Preferences, Events, Logs, and Reports surfaces.
- Docker packages the Rust daemon and Next.js management UI as one appliance
  image with `.data` as the durable state boundary.
- The daemon supervises the required Next.js child process with a bounded
  restart policy when the appliance runtime configures `--next-command`.
- SQLite stores local install state, local owner/business identity basics,
  provider configuration metadata, and encrypted local appliance vault items.
  The daemon exposes protected local install and provider endpoints with
  redacted provider read models so API keys remain write-only through HTTP
  surfaces.
- SQLite stores durable business facts with provenance, visibility, and
  publication state. The daemon exposes protected local business fact endpoints
  so public surfaces and future retrieval can depend on explicit truth and
  publication boundaries.
- The daemon exposes read-only public surface read models for About, Offers,
  Asks, and Feed. These JSON contracts only derive from published public
  business facts and include explicit readiness and provenance evidence.
- SQLite stores tracked entry points, visitor sessions, and visitor session
  events. The daemon exposes protected management routes and public-safe
  resolution/session creation routes that only point at published public surface
  destinations.
- SQLite stores durable offers, offer acceptances, trials, and trial events. The
  daemon exposes protected owner/operator routes plus public-safe offer listing
  and acceptance routes that only work for public published available offers or
  published public Offers read-model items.
- SQLite stores durable connection records, explicit connection grants,
  connection events, and local receipts. The daemon exposes protected local
  routes for connection records, grant creation/revocation, grant inspection,
  and event inspection; connection grants are scoped through the shared resource
  grant policy spine.
- SQLite stores local availability schedules, operator presence, interruption
  threshold, handoff eligibility decisions, handoff inbox items, handoff events,
  and local handoff receipts. The daemon exposes protected local routes for
  availability management, eligibility evaluation, inbox state, owner decisions,
  and receipt inspection without external delivery.
- Mutating daemon routes and MCP now have a first trust-boundary guard: requests
  must come from loopback-to-daemon access or provide the configured daemon
  access token.
- Protected daemon actions now pass through a shared policy decision
  spine that names actor, action, resource, capability, and outcome while
  preserving the current local trust boundary.
- SQLite stores a durable local access foundation with actors, roles,
  actor-role memberships, and resource grants. Fresh and upgraded databases seed
  deterministic system and local owner baselines.
- Policy decisions can consult durable resource grants for public,
  owner/system, and per-actor private resources, while the current System shell
  remains a local owner/operator surface.
- SQLite stores an access-aware knowledge corpus with source and item records
  that carry resource identity, classification metadata, provenance metadata,
  status, timestamps, item content hashes, and a local SQLite FTS index. The
  daemon exposes protected local corpus source/item management and governed
  retrieval routes that filter by approval status, visibility, viewer context,
  and durable resource access before returning evidence.
- SQLite stores durable local answer draft and answer draft citation records.
  The daemon exposes protected local answer draft preparation/list/read routes;
  preparation first runs governed corpus retrieval, then persists redacted
  prompt metadata, retrieval evidence, cited corpus item IDs, limitations,
  status, provenance, and source citation rows. The current draft output is a
  local evidence scaffold and records that no provider or model call occurred.
- The capability catalog distinguishes MCP export policy tiers, side effects,
  and approval requirements for read-only, local mutation, operator-confirmed,
  and non-exported dangerous operations.
- Capability role metadata is bound to durable local role membership, so seeded
  owner/system actors can use owner/system capabilities while unknown actors
  without role membership are denied.
- SQLite stores a durable policy decision audit trail for important protected
  daemon and MCP tool-call decisions, separate from diagnostic logs. The daemon
  exposes a protected local read path for recent policy decision audit evidence
  with narrow filters and bounded limits.
- The local MCP projection validates JSON-RPC 2.0 request shape and tool
  arguments against catalog input schemas before dispatch, and tool-call results
  include Ordo policy decision metadata.
- SQLite stores durable local MCP pack and pack tool metadata. The daemon
  exposes protected local pack list/read/install/disable routes; pack manifests
  validate against existing catalog capabilities, schemas, side effects,
  approval requirements, artifact metadata, and MCP export policy. MCP
  `tools/list` and `tools/call` hide or block disabled pack tools, and dangerous
  non-exported capabilities remain blocked.
- Job events and system lifecycle events are replayable from SQLite through a
  global event cursor, and the Events surface reads persisted event history.
- Structured diagnostic logs are persisted locally with bounded retention,
  redaction of secret-like payload keys, query filters, and visible inspection in
  the Logs surface.
- Local issue reports can be prepared through the shared job/task kernel and
  stored as SQLite artifacts. Reports include health, readiness, recent events,
  recent jobs, and structured diagnostic logs as evidence envelopes, then render
  a local markdown draft for operator review, copy, or export.
- SQLite stores durable report export records, report status events, support
  packet drafts, and support packet receipts. The daemon exposes protected local
  routes for report detail, status changes, local markdown export, support
  packet preview, local-only approval, and receipt inspection.
- Support packet drafts are bounded derivatives of local reports and approval
  records `approved_local_only` with `externalDelivery: false`; no support
  packet route performs network delivery.
- Local issue report job artifacts include provenance metadata that identifies
  actor, action, resource, producing capability, producing job, process template,
  and high-trust classification.
- SQLite initialization now runs ordered schema migrations tracked by
  `PRAGMA user_version`; fresh databases and 0.1.0 databases use the same path.
- Backup manifests now record SHA-256 checksum evidence with an algorithm
  version, include selected data-boundary sidecar files such as the local vault
  key for restore usability, and restore preflight rejects malformed manifests,
  checksum mismatches, and paths that escape the local backups boundary.
- The System shell now has Playwright browser smoke coverage for healthy and
  degraded daemon states, System Brief evidence/provenance, Backup And Restore,
  Logs, and Reports operator paths across desktop and mobile Chromium viewports.
- Diagnostics And Reports 1.0 has container runtime proof through Docker Compose:
  real daemon endpoints, real System shell Logs/Reports pages, and one
  browser-prepared local report artifact were verified with disposable state.
- The 0.1.0 release evidence dossier is recorded in
  [release-0.1.0.md](process/release-0.1.0.md).

## What Is Not Built Yet

- Full frontend product-depth surfaces such as Studio, Connections, Offers,
  About, Asks, Feed, Today, and Conversations are not built yet.
- Authentication UI, hosted identity, OAuth/email login, public portals, and
  multi-user product surfaces are not implemented yet.
- Frontend install wizard UI and provider network validation are not implemented
  yet; current install/provider support is daemon-owned backend state and
  protected local routes only.
- Public About, Offers, Asks, and Feed frontend UI routes are not implemented
  yet; current support is daemon-owned JSON read models only.
- Visitor-facing UI, analytics dashboards, and offer/trial attribution consumers
  are not implemented yet; current visitor session support is backend state and
  event evidence only.
- Offers and trial lifecycle UI, payments, affiliate payout automation, and
  external trial follow-up are not implemented yet; current support is backend
  state, policy audit, and event evidence only.
- Connections UI, public portals, availability/handoff behavior, support packet
  egress, mediated chat, external connection integrations, and affiliate payout
  automation are not implemented yet; current support is backend state, scoped
  policy, local events, and receipt evidence only.
- Availability and handoff inbox UI, push notifications, calendar sync, voice
  handoff, mediated handoff chat, and external handoff delivery are not
  implemented yet; current support is backend state, policy audit, eligibility
  evidence, local inbox events, and receipt evidence only.
- Embeddings, vector search, provider-backed RAG answer generation, chat
  retrieval UI, provider calls for answers, and external integrations are not
  implemented yet.
- Report submission and support packet transport to external systems are not
  implemented yet; Reports 1.0 and support packet approval remain local evidence
  and approval records only.
- MCP is currently a local JSON-RPC daemon projection with governed local pack
  metadata, not a third-party plugin surface, marketplace, or arbitrary code
  execution runtime.
- Full visual regression coverage is not implemented yet.

## Current Goal

Continue the 0.1.1 stabilization track after the runtime supervision, network
posture, MCP policy tier, MCP request strictness, durable event replay, schema
migration, backup integrity, UI smoke coverage, and local diagnostics/reporting
slices.

## Useful Current References

- [Diagnostics And Reports](architecture/diagnostics-and-reports.md) describes
  the implemented local Logs and Reports surfaces.
- [Resource, Provenance, And Policy Spine](architecture/resource-provenance-policy.md)
  describes the implemented policy/provenance foundation.
- [Access And Local RBAC](architecture/access-rbac.md) describes the implemented
  durable local access foundation.
- [Local Install And Providers](architecture/local-install-and-providers.md)
  describes the implemented backend install state and redacted provider
  configuration foundation.
- [Business Truth, Visibility, And Publication](architecture/business-truth-visibility.md)
  describes the backend foundation for durable business facts and publication
  boundaries.
- [Public Surface Read Models](architecture/public-surfaces.md) describes the
  implemented daemon contracts for public About, Offers, Asks, and Feed data.
- [Tracked Entry Points And Visitor Sessions](architecture/tracked-entry-points.md)
  describes the implemented backend foundation for QR/link/campaign entry
  context and visitor session evidence.
- [Offers And Trial Lifecycle](architecture/offers-and-trials.md) describes the
  implemented backend foundation for offer acceptance, 30-day trials, and
  lifecycle event evidence.
- [Connections Foundation](architecture/connections.md) describes the
  implemented backend foundation for scoped connection records, grants,
  revocations, events, and receipts.
- [Availability And Handoff Inbox](architecture/availability-and-handoff.md)
  describes the implemented backend foundation for owner attention boundaries,
  eligibility decisions, inbox state, and local receipts.
- [Knowledge Corpus And Governed Retrieval](architecture/knowledge-corpus.md)
  describes the implemented local FTS retrieval and answer draft foundation for
  future knowledge/RAG work.
- [Capability Catalog](architecture/capability-catalog.md) describes the
  implemented catalog, MCP projection, and local pack metadata boundary.
- [Product Shape](business/product-shape.md) describes the planned Chat, About,
  Offers, Asks, Feed, Connections, availability, handoff, affiliate, and sales
  loop direction without claiming they are built.
- [Ordo Core](business/ordo-core.md) describes the durable product doctrine and
  how future MCP tools and packs should customize the work without bypassing the
  trust boundary.
- [Product Roadmap](business/product-roadmap.md) records the north-star
  workflows and slice quality bar for future product development.
- [Scaling With Worker Ordos](architecture/scaling-worker-ordos.md) describes
  future Home Ordo and Worker Ordo scaling.
- [Diagnostics And Reports Runtime Proof 1.0](process/diagnostics-reports-runtime-proof-1.0.md)
  records the container proof for the Logs and Reports slice.

## How To Read Claims

If a doc describes product behavior that is not present in code yet, treat it as
direction, not shipped functionality.
