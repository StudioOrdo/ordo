# Ordo 0.1.0 Appliance Specimen

Status: Release evidence collected on 2026-05-08

Milestone: [0.1.0 Appliance Specimen](https://github.com/StudioOrdo/ordo/milestone/1)

0.1.0 is the first coherent Ordo appliance specimen. It proves the core
architecture before product depth.

## Goal

Demonstrate that Ordo can run as a local-first AI appliance with a reusable
process/job/task kernel, scheduled briefs, durable events, WebSocket progress,
backup/restore safety, and an Ordo-style System shell.

## Required Capabilities

- Rust daemon supervises the appliance and Next.js process.
- SQLite stores process templates, jobs, task DAGs, events, artifacts,
  schedules, briefs, preferences, and backup records.
- Scheduler creates jobs from process templates.
- WebSocket broadcasts persisted events.
- System Brief is the default UI and shows when it was created.
- System menu includes Brief, Health, Backup And Restore, Schedules,
  Preferences, and Events.
- Backup, restore, and brief generation are concrete job templates on the same
  reusable kernel.
- Docker runs the system as one appliance image.
- Capability catalog defines what Ordo can do; MCP is one governed projection.

## Non-Goals

- Full Studio, People, Offers, Today, and Conversations product depth.
- Arbitrary user code execution.
- Hosted infrastructure as a core dependency.
- Time-based progress or unsupported ETA promises.

## Release Evidence

The release cannot be called complete without:

- passing tests for DAG readiness, task-based progress, event persistence,
  scheduler due work, and WebSocket event shape;
- Docker run evidence;
- browser evidence for the System Brief and Backup And Restore pages;
- backup/restore evidence using `.data`;
- documentation of known limitations.

## Evidence Dossier

### Release Summary

Ordo 0.1.0 Appliance Specimen proves the first coherent local appliance: a Rust
daemon, Next.js System shell, SQLite durable state, reusable process/job/task DAG
kernel, scheduler, WebSocket projection, System Brief artifact, backup/restore
kernel, Docker appliance runtime, and capability catalog with MCP projection.

The release remains intentionally narrow. It proves the operating spine before
Studio, People, Offers, Today, Conversations, hosted services, or arbitrary
user-authored execution.

### Milestone State

- Issue #3, architecture contract: closed.
- Issues #5 through #10, implementation phases 1 through 6: closed.
- PR #4 and PRs #11 through #16: merged on 2026-05-08.
- Current `main` release commit: `e79f204971e997ba12666419ba478e064a96385e`.

### Architecture Proven

- Rust daemon owns health, readiness, scheduling, WebSocket fanout,
  backup/restore, System Brief generation, capability catalog, MCP projection,
  and Docker-time Next.js supervision.
- SQLite stores capabilities, process templates, jobs, tasks, dependencies,
  events, artifacts, schedules, scheduled runs, brief artifacts, preferences,
  and backup records.
- Process templates create jobs through a reusable task DAG kernel; progress is
  task-count based, not time-estimated.
- The System Brief is durable evidence, not only page text.
- Backup creation writes a manifest and persists job/artifact evidence through
  container restart.
- Docker runs the appliance as one image with `.data` as the durable boundary.
- MCP is a JSON-RPC projection over governed daemon capabilities, not a second
  execution spine.

### Validation Evidence

- `npm run typecheck`: passed.
- `npm run build`: passed; compiled successfully in 3.7 seconds, TypeScript
  finished in 1627 ms, and 9 routes were generated.
- `cargo fmt --all -- --check`: passed.
- `cargo test --workspace`: passed; 20 tests passed, 0 failed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `git diff --check`: passed with no whitespace errors.
- `docker compose -p ordo_release_010 build`: passed.
- `docker compose -p ordo_release_010 up -d --wait`: passed; container reported
  healthy.
- Daemon `/health`: returned `status: ok`.
- Daemon `/ready`: returned `status: ready` with required SQLite tables present.
- UI `/`: browser rendered the System Brief with daemon health, SQLite
  readiness, deterministic generator evidence, limitations, and provenance.
- UI `/backup-restore`: browser rendered Backup & Restore with a succeeded
  backup job, 8/8 tasks complete, and a manifest path under `/app/.data/backups`.
- Backup persistence: after container restart, `/backups` still returned one
  succeeded backup job with 8/8 required tasks complete and artifact
  `artifact_c041d6b6-ed7f-4263-ad8b-5b6862c46a8c`.
- `/capabilities`: returned 37 capabilities including `system.status.read`,
  `backup.create`, and `restore.preflight.validate`.
- `/mcp` `tools/list`: returned 8 MCP-exported tools including
  `system.status.read`, `backup.create`, and `brief.system.generate`.
- `/mcp` `tools/call` for `system.status.read`: returned health `ok` and
  readiness `ready`.

### Known Limitations And Residual Risks

- No LLM adapter is configured; the System Brief uses deterministic local
  evidence.
- Authentication, RBAC enforcement depth, and multi-user policy surfaces are not
  included in 0.1.0.
- MCP exposes only the initial safe system projection and is not yet a full MCP
  transport/server implementation beyond the JSON-RPC daemon route.
- Restore remains preflight/safety-gated; broad destructive restore automation is
  intentionally limited.
- Product-depth surfaces such as Studio, People, Offers, Today, and
  Conversations are not included.
- Hosted deployment, external integrations, and marketplace/plugin workflows are
  outside this release.
- Known dependency audit residual: a moderate Next/PostCSS advisory remains; the
  available force fix would downgrade Next and was not applied for this release.

### User-Facing Release Notes

Ordo 0.1.0 is the first appliance specimen. It runs locally as one Docker image,
keeps its durable state in SQLite and `.data`, shows a System Brief as the
default UI, records work through process/job/task evidence, creates backups with
manifests, and exposes a small governed capability/MCP system surface.

This release is not production-complete business automation. It is the working
foundation: local-first state, visible system evidence, governed jobs, backup
safety, live projection, and an inspectable AGPL codebase.

### Tag Recommendation

Create annotated tag `v0.1.0` at commit
`e79f204971e997ba12666419ba478e064a96385e`.

Suggested command:

```bash
git tag -a v0.1.0 e79f204971e997ba12666419ba478e064a96385e -m "Release Ordo 0.1.0 Appliance Specimen"
```
