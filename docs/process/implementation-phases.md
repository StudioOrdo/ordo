# Implementation Phases

These phases implement [Ordo 0.1.0 Appliance Specimen](release-0.1.0.md).

## Phase 0: Architecture Contract

Define the system architecture, kernel, runtime, briefs, scheduler, realtime,
system shell, backup/restore, and release criteria.

No app code is scaffolded in this phase.

## Phase 1: Rust Appliance Spine

- Add Rust workspace and `ordo-daemon`.
- Initialize SQLite schema.
- Add job, task, dependency, event, artifact, schedule, brief, and preference
  tables.
- Add health endpoint.
- Add WebSocket endpoint.
- Add scheduler loop stub.
- Test DAG readiness and task-based progress.

## Phase 2: Next System Shell

- Scaffold Next.js app.
- Add System shell using primary rail, second-column evidence index, and main
  pane.
- Render System Brief as the default page.
- Add Health, Backup And Restore, Schedules, Preferences, and Events routes.
- Add WebSocket connection indicator.

## Phase 3: Brief Kernel

- Add `brief.system.generate` process template.
- Add scheduled System Brief generation.
- Add deterministic fallback brief generation.
- Add LLM adapter boundary.
- Save brief artifacts with as-of timestamp and evidence references.

## Phase 4: Backup And Restore Kernel

- Add `backup.create` process template.
- Add `restore.execute` process template.
- Create `.data` backup archive and manifest.
- Require restore confirmation and safety backup.
- Show backup/restore jobs in a table with task-based progress.

## Phase 5: Docker Appliance Runtime

- Build one Docker image.
- Run Rust daemon as the top-level process.
- Supervise Next.js as a child process.
- Mount `.data` as durable state.
- Add compose file and health checks.

## Phase 6: Capability Catalog And MCP Projection

- Add seed capability catalog.
- Route UI, scheduler, and job tasks through cataloged capabilities.
- Export governed system tools through explicit MCP policy tiers.
- Keep MCP as a projection, not the execution spine.