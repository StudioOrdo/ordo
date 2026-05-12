# Developer Guide

Status: canonical public guide for local development

This guide describes how to run, validate, and extend the current Ordo
repository without confusing implemented behavior with future direction.

## Requirements

- Node.js compatible with Next.js 16.
- npm.
- Rust toolchain compatible with the workspace.
- Docker and Docker Compose for appliance runtime proof.
- Playwright browser dependencies for `npm run smoke:ui`.

## First Run

Install JavaScript dependencies:

```bash
npm install
```

Initialize local daemon state:

```bash
cargo run -p ordo-daemon -- init-db --db-path .data/local.db
cargo run -p ordo-daemon -- ready-json --db-path .data/local.db
```

Run the Next.js development server:

```bash
npm run dev
```

Run the daemon separately when testing daemon APIs:

```bash
cargo run -p ordo-daemon -- serve --db-path .data/local.db
```

## Docker Appliance

Create local runtime env only when needed:

```bash
cp .env.example .env.local
```

Do not commit `.env.local`. Compose reads it at runtime through `env_file`, and
the Docker build does not bake it into image layers.

Build and run:

```bash
docker compose build
docker compose up
```

Open `http://localhost:3000` for the UI. The daemon listens on
`http://localhost:17760`.

Useful runtime checks:

```bash
curl http://localhost:17760/health
curl http://localhost:17760/ready
curl http://localhost:17760/capabilities
curl 'http://localhost:17760/events?after=0&limit=100'
```

Persistence lives in the named Compose volume `ordo-data`, mounted at
`/app/.data`. Use `docker compose down -v` only when you intentionally want to
delete appliance state.

## Common Commands

Rust daemon:

```bash
cargo run -p ordo-daemon -- health-json
cargo run -p ordo-daemon -- init-db --db-path .data/local.db
cargo run -p ordo-daemon -- ready-json --db-path .data/local.db
cargo run -p ordo-daemon -- list-capabilities-json --db-path .data/local.db
cargo run -p ordo-daemon -- latest-system-brief-json --db-path .data/local.db
cargo run -p ordo-daemon -- generate-system-brief-json --db-path .data/local.db
cargo run -p ordo-daemon -- create-backup-json --db-path .data/local.db
cargo run -p ordo-daemon -- list-backups-json --db-path .data/local.db
```

MCP projection:

```bash
cargo run -p ordo-daemon -- mcp-json --db-path .data/local.db --method tools/list
cargo run -p ordo-daemon -- mcp-json --db-path .data/local.db --method tools/call --params-json '{"name":"system.status.read","arguments":{}}'
```

Project context export:

```bash
npm run export
```

`npm run export` writes ignored `project-export.txt` for external review tools.
Do not treat it as source of truth when source files are available.

## Validation Matrix

Use validation proportional to the change. For shared behavior, run the full
matrix:

```bash
npm run typecheck
npm run build
npm run smoke:ui
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

For small doc-only changes, `git diff --check` is usually enough.

For focused Rust changes, run the smallest relevant Rust tests first, then widen
when touching shared schema, policy, route, conversation, or eval behavior.

## Live LLM Evals

Default tests are deterministic and network-free. Live provider calls are
manual and guarded.

The daemon currently exposes a guarded OpenAI-compatible smoke runner:

```bash
ORDO_LIVE_LLM_EVALS=1 \
ORDO_LIVE_LLM_ALLOW_NETWORK=1 \
ORDO_LIVE_LLM_PROVIDER=openai \
ORDO_LIVE_LLM_MODEL=<model> \
ORDO_LIVE_LLM_MAX_CASES=1 \
ORDO_LIVE_LLM_BUDGET_USD=0.01 \
OPENAI_API_KEY=<redacted> \
cargo run -p ordo-daemon -- run-live-llm-eval-json --db-path .data/local.db
```

Use `.env.local` for local secrets instead of shell history when possible.
Never print provider key values in logs, docs, reports, or test artifacts.

## Editing Rules

- Keep SQLite schema changes in ordered Rust migrations and update required
  table checks when needed.
- Keep protected routes aligned with route contracts and capability policy.
- Keep docs synchronized when behavior changes user-visible architecture,
  runtime, workflows, validation commands, or security boundaries.
- Prefer deterministic evals before live provider tests.
- Preserve unrelated user changes in the worktree.
