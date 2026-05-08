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
- Mutating daemon routes and MCP now have a first trust-boundary guard: requests
  must come from loopback-to-daemon access or provide the configured daemon
  access token.
- Protected daemon actions now pass through a shared pre-RBAC policy decision
  spine that names actor, action, resource, capability, and outcome while
  preserving the current local trust boundary.
- SQLite stores a durable local access foundation with actors, roles,
  actor-role memberships, and resource grants. Fresh and upgraded databases seed
  deterministic system and local owner baselines.
- Policy decisions can consult durable resource grants for public,
  owner/system, and per-actor private resources, while the current System shell
  remains a local owner/operator surface.
- The capability catalog distinguishes MCP export policy tiers, side effects,
  and approval requirements for read-only, local mutation, operator-confirmed,
  and non-exported dangerous operations.
- The local MCP projection validates JSON-RPC 2.0 request shape and tool
  arguments against catalog input schemas before dispatch, and tool-call results
  include Ordo policy decision metadata.
- Job events and system lifecycle events are replayable from SQLite through a
  global event cursor, and the Events surface reads persisted event history.
- Structured diagnostic logs are persisted locally with bounded retention,
  redaction of secret-like payload keys, query filters, and visible inspection in
  the Logs surface.
- Local issue reports can be prepared through the shared job/task kernel and
  stored as SQLite artifacts. Reports include health, readiness, recent events,
  recent jobs, and structured diagnostic logs as evidence envelopes, then render
  a local markdown draft for operator review, copy, or export.
- Local issue report job artifacts include provenance metadata that identifies
  actor, action, resource, producing capability, producing job, process template,
  and high-trust classification.
- SQLite initialization now runs ordered schema migrations tracked by
  `PRAGMA user_version`; fresh databases and 0.1.0 databases use the same path.
- Backup manifests now record SHA-256 checksum evidence with an algorithm
  version, and restore preflight rejects malformed manifests, checksum
  mismatches, and paths that escape the local backups boundary.
- The System shell now has Playwright browser smoke coverage for healthy and
  degraded daemon states, System Brief evidence/provenance, Backup And Restore,
  Logs, and Reports operator paths across desktop and mobile Chromium viewports.
- Diagnostics And Reports 1.0 has container runtime proof through Docker Compose:
  real daemon endpoints, real System shell Logs/Reports pages, and one
  browser-prepared local report artifact were verified with disposable state.
- The 0.1.0 release evidence dossier is recorded in
  [release-0.1.0.md](process/release-0.1.0.md).

## What Is Not Built Yet

- Full product-depth surfaces such as Studio, People, Offers, Today, and
  Conversations are not built yet.
- Authentication UI, hosted identity, OAuth/email login, public portals, and
  multi-user product surfaces are not implemented yet.
- RAG/vector memory and external integrations are not implemented yet.
- Report submission transports to external systems are not implemented yet;
  Reports 1.0 prepares local evidence packages only.
- MCP is currently a local JSON-RPC daemon projection with first policy tiers,
  not a third-party plugin surface.
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
- [Product Shape](business/product-shape.md) describes the planned Chat, About,
  Offers, and Feed surfaces without claiming they are built.
- [Scaling With Worker Ordos](architecture/scaling-worker-ordos.md) describes
  future Home Ordo and Worker Ordo scaling.
- [Diagnostics And Reports Runtime Proof 1.0](process/diagnostics-reports-runtime-proof-1.0.md)
  records the container proof for the Logs and Reports slice.

## How To Read Claims

If a doc describes product behavior that is not present in code yet, treat it as
direction, not shipped functionality.
