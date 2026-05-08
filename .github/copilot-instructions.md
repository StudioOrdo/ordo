# Copilot Instructions For Ordo

These instructions apply to work in the Studio Ordo repository. Keep changes
small, evidence-backed, and aligned with the appliance architecture.

## Operating Process

- Treat GitHub issues and pull requests as the public manufacturing record.
- For an issue-driven request, inspect the issue first, confirm the working tree
  is clean, branch from current `main`, implement the accepted scope, validate,
  open a PR that closes the issue, check merge readiness, then merge and sync
  `main` when the user has asked for autonomous completion.
- Do not commit, push, create PRs, or merge unless the user request clearly
  calls for that workflow.
- Preserve unrelated user changes. Never reset, checkout, or revert files you
  did not intentionally change unless the user explicitly asks.
- Use `apply_patch` for source edits. Use `execution_subagent` for most command
  execution. Use `rg`/`rg --files` for repository search.
- Before Docker or Compose commands, call the container tooling config and use
  the configured `docker` / `docker compose` base commands.
- For Next.js work, initialize Next.js DevTools MCP for the project and consult
  official Next.js docs through the provided docs tooling before relying on
  framework assumptions.

## Architecture Spine

- Ordo is a local-first AI appliance for one-person businesses.
- Core spine: `Capability Catalog -> Process Template -> Job -> Task DAG -> Event -> Artifact -> Brief`.
- Rust daemon owns appliance supervision, SQLite initialization/migrations,
  scheduler, job/task kernel, backup/restore, durable event replay, health,
  readiness, WebSocket projection, capability catalog, MCP projection, and
  Docker-time Next.js supervision.
- Next.js owns product UI, routes, read models, policy checks, brief rendering,
  and shell navigation.
- SQLite is the source of truth. WebSocket is only a live projection.
- Docker is one appliance image with `.data` as the durable boundary.
- MCP is a governed projection over daemon/catalog capabilities, not a second
  execution spine and not arbitrary code execution.

## Current Product Surface

- The default UI is System Brief.
- Health, Backup And Restore, Schedules, Preferences, and Events live in the
  System shell.
- Backup, restore preflight, and brief generation are concrete process
  templates running through the shared job/task kernel.
- Restore remains confirmation-gated and non-destructive at the current
  preflight boundary.
- Product-depth surfaces such as Studio, People, Offers, Today, Conversations,
  RAG/vector memory, external integrations, deep RBAC, and full visual
  regression coverage are not built yet.

## Validation

Run validation proportional to the change. For issue completion or shared
behavior, use the full matrix:

```bash
npm run typecheck
npm run build
npm run smoke:ui
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

For Docker/runtime evidence, build and run with a unique Compose project name,
verify `/health`, `/ready`, affected UI/API behavior, and clean up with
`docker compose -p <project> down -v` when the proof uses disposable state.

## Documentation And Evidence

- Keep docs synchronized with shipped behavior when code changes user-visible
  architecture, workflow, runtime, or validation commands.
- Start with `docs/state-of-the-project.md`, `docs/architecture/`, and
  `docs/process/` for current truth.
- `docs/_codex/` is private/reference thinking. Read it when useful, but do not
  edit or commit changes there unless the user explicitly asks.
- The repo is AGPL-3.0-only; do not add copyright/license headers unless asked.

## Implementation Preferences

- Prefer existing Rust daemon, Next.js App Router, SQLite, and local helper
  patterns over new abstractions.
- Keep SQLite schema changes in ordered daemon migrations tracked by
  `PRAGMA user_version`.
- Mutating daemon/MCP routes must respect the local access boundary and catalog
  policy tiers.
- Keep frontend work quiet, operational, and evidence-oriented. Avoid marketing
  pages for system surfaces.
- Playwright UI smoke coverage uses the lightweight mock daemon pattern in
  `tests/ui/` and covers both desktop and mobile Chromium where practical.
