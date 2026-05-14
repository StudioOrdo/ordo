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

Run the local appliance development runtime:

```bash
npm run dev
```

`npm run dev` starts the Rust daemon and lets the daemon supervise Next.js. It
preflights the Rust and npm toolchains, checks that the daemon and Next ports
are free, loads `.env.local` without printing secret values, verifies local
Ollama by default, initializes SQLite, and prints the daemon and UI URLs. Use
`ORDO_DEV_REQUIRE_OLLAMA=0 npm run dev` only when intentionally developing
without Local chat.

Run the raw Next.js development server only when deliberately bypassing daemon
integration:

```bash
npm run dev:next
```

Run the daemon separately only when testing daemon APIs without Next.js:

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

Compose starts the same appliance shape as local `npm run dev`: the Rust daemon
is the top-level process and supervises the Next.js standalone server. The
container maps Local chat to host Ollama at
`http://host.docker.internal:11434/api` with model `qwen2.5-coder:7b` by
default; override `ORDO_OLLAMA_BASE_URL` or `ORDO_OLLAMA_MODEL` in `.env.local`
when needed.

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

## Local Appliance Runtime

The default development command is the appliance runner. It starts the Rust
daemon, and the daemon supervises Next.js in the same process tree:

```bash
npm run dev
```

The runner loads `.env.local` into the daemon environment without printing any
values, then lets the daemon supervise Next.js. Docker Compose also reads
`.env.local` through `env_file`, so containerized appliance runs use the same
local provider configuration path.

Useful overrides:

```bash
ORDO_DEV_NEXT_PORT=3001 npm run dev
ORDO_DEV_DAEMON_PORT=17761 npm run dev
ORDO_DEV_DATA_DIR=.data/another-dev npm run dev
ORDO_DEV_REQUIRE_OLLAMA=0 npm run dev
```

Default app chat stays deterministic even when `.env.local` contains provider
keys or live smoke guards. Provider/model choices in the browser are loaded
from the daemon provider read model, then sent back through `/chat/ws` as
`llm.run.request`; the browser must not hard-code live model ids or call a
provider directly. To intentionally run app chat through a configured live
provider, an owner/developer must add the app-live guard in addition to the
existing live-provider guards:

```bash
ORDO_APP_LIVE_LLM=1 \
ORDO_LIVE_LLM_EVALS=1 \
ORDO_LIVE_LLM_ALLOW_NETWORK=1 \
ORDO_LIVE_LLM_PROVIDER=<openai|deepseek|anthropic> \
ORDO_LIVE_LLM_MODEL=<model-from-provider-catalog> \
ORDO_LIVE_LLM_MAX_CASES=1 \
ORDO_LIVE_LLM_BUDGET_USD=0.01 \
npm run dev:appliance
```

Provider secrets should come from `.env.local`, secret-file variables, or the
local vault. Do not place API key values directly in the command line. This
mode can use network and provider budget, so do not use it for default
development, validation, or CI.
DeepSeek keys may be supplied as `DEEPSEEK_API_KEY`, `API__DEEPSEEK_API_KEY`,
or the local lowercase `deepseek` variable used by some appliance env files.

The member chat browser UI sends assistant reply requests through the daemon
conversation gateway over `/chat/ws` with `llm.run.request`. The daemon projects
provider output as `llm.text.delta`, `llm.text.completed`, `llm.run.completed`,
and redacted `llm.performance.measured` frames so the UI can stream quickly and
operator tooling can compare time-to-first-token and total latency. The direct
Next.js `/api/chat/stream` route is intentionally fail-closed and must not read
provider keys or call a live provider.

To compare configured providers from a running local appliance, use the
redacted benchmark runner:

```bash
npm run benchmark:chat:providers
```

The benchmark prints a JSON summary ranked by median time-to-first-token. Local
deterministic chat can run without network. Live providers are skipped unless
`ORDO_PROVIDER_BENCHMARK_LIVE=1`, `ORDO_LIVE_LLM_ALLOW_NETWORK=1`, and a budget
guard such as `ORDO_LIVE_LLM_BUDGET_USD=0.01` are present. The benchmark records
provider ids, model ids, latency, delta counts, and failure codes only; it must
not print raw prompts, provider keys, or authorization headers.

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

When intentionally running the guarded smoke locally, prefer the package script
so provider keys come from `.env.local` without being placed in shell history:

```bash
npm run live:llm:smoke
```

The script supplies the explicit non-secret guards for the smoke runner and
defaults to one OpenAI case with a small budget. To override non-secret values,
set environment variables such as `ORDO_LIVE_LLM_MODEL` or
`ORDO_LIVE_LLM_BUDGET_USD` before running it.

For a fully manual shell run, load `.env.local` without echoing it, then provide
only non-secret guard values on the command line:

```bash
set -a
source .env.local
set +a

ORDO_LIVE_LLM_EVALS=1 \
ORDO_LIVE_LLM_ALLOW_NETWORK=1 \
ORDO_LIVE_LLM_PROVIDER=openai \
ORDO_LIVE_LLM_MODEL=<model-from-provider-catalog> \
ORDO_LIVE_LLM_MAX_CASES=1 \
ORDO_LIVE_LLM_BUDGET_USD=0.01 \
cargo run -p ordo-daemon -- run-live-llm-eval-json --db-path .data/local.db
```

Do not run the guarded smoke as part of default validation or CI; it is a manual
owner/developer action because it can use network and spend provider budget.

## Editing Rules

- Keep SQLite schema changes in ordered Rust migrations and update required
  table checks when needed.
- Keep protected routes aligned with route contracts and capability policy.
- Keep docs synchronized when behavior changes user-visible architecture,
  runtime, workflows, validation commands, or security boundaries.
- Prefer deterministic evals before live provider tests.
- Preserve unrelated user changes in the worktree.
