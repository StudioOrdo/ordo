# LLM Agent Guide

Status: canonical public guide for AI coding agents working in this repository

This guide gives LLM agents a compact operating map for Ordo. It is not a
replacement for reading the files being changed.

## Start Here

Read in this order:

1. [System Overview](system-overview.md)
2. [State Of The Project](state-of-the-project.md)
3. [Developer Guide](developer-guide.md)
4. [Architecture Index](architecture/README.md)
5. [Eval System](evals/README.md)
6. [Issue History](process/issue-history.md)

Then read the specific source files and tests around the task.

## Source Of Truth Order

1. Current source code and tests.
2. Current schema migrations and route contracts.
3. [State Of The Project](state-of-the-project.md).
4. Current architecture/process/business docs.
5. Backlog docs.
6. Private or archived drafts, when explicitly requested.

Backlog and draft docs are intent, not proof. Do not claim behavior is shipped
unless source, tests, or state docs say it is real now.

## Core Architecture Assumptions

- SQLite is the source of truth.
- WebSocket is a live projection and command transport, not the record.
- Rust owns durable appliance behavior, policy, provider boundaries, local
  execution, and SQLite migrations.
- Next.js owns product UI, route composition, read-model display, and user
  interaction states.
- MCP is a governed projection over registered capabilities, not arbitrary code
  execution.
- External LLM calls must go through daemon-owned policy, prompt slots, privacy
  egress, and accounting.

## High-Risk Boundaries

Be extra careful around:

- provider keys and `.env.local` values;
- vault and backup sidecar files;
- policy decisions, actor roles, and resource grants;
- public/private/staff/owner visibility;
- prompt slots and provider-bound payloads;
- privacy egress placeholdering and reconstruction;
- token ledger and eval artifacts;
- support packet/report export boundaries;
- Docker `.data` persistence.

Never print or commit secret values. Redacted key names and configured/missing
status are acceptable; raw values are not.

## Current Product Posture

Ordo is a strong local appliance and eval foundation. It is not production
business automation yet.

Implemented foundations include system shell, process kernel, diagnostics,
reports, backups, provider config, policy, MCP projection, public read models,
entry points, offers/trials, connections, availability/handoffs, knowledge
corpus, conversation realtime, LLM gateway, deterministic/replay/live-smoke
evals, journey personas, artifact review, and cross-journey reports.

Future work includes production public portals, auth UI, broad live-provider
orchestration, embeddings/vector search, provider-backed RAG answers, real
outbound email, payments, external transports, Worker Ordos, and A2A.

## How To Make Changes

1. Inspect current files before editing.
2. Identify whether the change touches schema, route contracts, policy,
   provider egress, UI, evals, or docs.
3. Keep edits small and aligned with existing local patterns.
4. Add or update tests/evals when changing shared behavior.
5. Update public docs when behavior changes the architecture, runtime, or
   validation story.
6. Run focused validation, then widen when risk is high.

## Eval-First Debugging

When a behavior is unclear, prefer building or running deterministic evals
before live provider tests. Ordo's eval artifacts should expose:

- transcript evidence;
- event ledger evidence;
- database row evidence;
- prompt-slot evidence;
- privacy evidence;
- token accounting evidence;
- handoff/mode evidence;
- artifact review findings.

Use live provider calls only behind explicit guards and spend caps.

## Useful Implementation Anchors

| Area | Primary files |
| --- | --- |
| SQLite schema | `crates/ordo-daemon/src/schema.rs` |
| Server routes | `crates/ordo-daemon/src/server.rs` |
| Route contracts | `crates/ordo-daemon/src/route_contracts.rs` |
| Process kernel | `crates/ordo-daemon/src/kernel.rs`, `templates.rs` |
| Events/WebSocket | `events.rs`, `server.rs`, `conversation_gateway.rs` |
| Policy/access | `policy.rs`, `access.rs`, `capabilities.rs` |
| Provider config/vault | `install.rs`, `vault.rs` |
| Conversations | `conversations.rs`, `conversation_protocol.rs`, `conversation_gateway.rs` |
| LLM gateway | `llm_gateway.rs`, `llm_accounting.rs`, `privacy_egress.rs` |
| Evals | `eval_harness.rs`, `live_eval_runner.rs`, `eval_artifact_review.rs`, `eval_personas.rs`, `eval_simulators.rs` |
| UI | `app/`, `components/`, `lib/` |
