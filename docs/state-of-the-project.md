# State Of The Project

Date: 2026-05-08

Ordo has completed the 0.1.0 Appliance Specimen implementation pass.

The current work is release closeout and the next trust-boundary stabilization
slice before deeper product surfaces land.

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
  Schedules, Preferences, and Events surfaces.
- Docker packages the Rust daemon and Next.js management UI as one appliance
  image with `.data` as the durable state boundary.
- The daemon supervises the required Next.js child process with a bounded
  restart policy when the appliance runtime configures `--next-command`.
- Mutating daemon routes and MCP now have a first trust-boundary guard: requests
  must come from loopback-to-daemon access or provide the configured daemon
  access token.
- The 0.1.0 release evidence dossier is recorded in
  [release-0.1.0.md](process/release-0.1.0.md).

## What Is Not Built Yet

- Full product-depth surfaces such as Studio, People, Offers, Today, and
  Conversations are not built yet.
- Authentication, RBAC enforcement depth, and multi-user policy surfaces are not
  implemented yet.
- RAG/vector memory and external integrations are not implemented yet.
- MCP is currently a local JSON-RPC daemon projection, not a fully hardened
  public transport boundary.
- Durable event replay, schema migrations, backup integrity, MCP policy depth,
  and UI smoke coverage remain the next stabilization concerns.

## Current Goal

Continue `0.1.1 Appliance Trust Boundary` after the runtime supervision and
first network posture slices, then harden MCP policy tiers, event replay, schema
migrations, backup integrity, and UI smoke coverage.

## How To Read Claims

If a doc describes product behavior that is not present in code yet, treat it as
direction, not shipped functionality.
