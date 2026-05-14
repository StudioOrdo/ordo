# LLM Agent Guide

Status: canonical public guide for AI coding agents working in this repository

This guide gives LLM agents a compact operating map for Ordo. It is not a
replacement for reading the files being changed.

## Start Here

Read in this order:

1. [System Overview](system-overview.md)
2. [State Of The Project](state-of-the-project.md)
3. [Developer Guide](developer-guide.md)
4. [Current Product Canon](business/current-product-canon.md)
5. [Workforce Substrate](business/workforce-substrate.md)
6. [Appliance Operating Discipline](architecture/appliance-operating-discipline.md)
7. [Target Architecture Plan](architecture/target-architecture-plan.md)
8. [Rewards And Incentives](architecture/rewards-and-incentives.md)
9. [OrdoStudio NYC Pilot](business/ordostudio-nyc-pilot.md)
10. [Architecture Index](architecture/README.md)
11. [Eval System](evals/README.md)
12. [Issue History](process/issue-history.md)

Then read the specific source files and tests around the task.

## Source Of Truth Order

1. Current source code and tests.
2. Current schema migrations and route contracts.
3. [Current Product Canon](business/current-product-canon.md) for product IA,
   UX stance, and surface vocabulary.
4. [Workforce Substrate](business/workforce-substrate.md) for pack, Studio,
   and user-experience stance.
5. [Appliance Operating Discipline](architecture/appliance-operating-discipline.md)
   and [Target Architecture Plan](architecture/target-architecture-plan.md)
   for backend discipline and implementation shape.
6. [Rewards And Incentives](architecture/rewards-and-incentives.md) for
   Growth rewards, referral, feedback, benefit grants, and leaderboard rules.
7. [State Of The Project](state-of-the-project.md).
8. Current architecture/process/business docs.
9. Backlog docs.
10. Private or archived drafts, when explicitly requested.

Backlog and draft docs are intent, not proof. Do not claim behavior is shipped
unless source, tests, or state docs say it is real now.

## Core Architecture Assumptions

- SQLite is the source of truth.
- Canonical tables own truth, events own audit/replay, and projections own
  surface experience.
- WebSocket is a live projection and command transport, not the record.
- Rust owns durable appliance behavior, policy, provider boundaries, local
  execution, and SQLite migrations.
- Next.js owns product UI, route composition, read-model display, and user
  interaction states.
- Next.js should display daemon/application read models. It should not
  reconstruct product meaning from raw operational tables.
- Product commands should validate access and policy, mutate canonical state,
  append events, and refresh or schedule projections.
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
- reward qualification, hosted-time benefit grants, leaderboard projections,
  and anti-abuse reversals;
- Docker `.data` persistence.

Never print or commit secret values. Redacted key names and configured/missing
status are acceptable; raw values are not.

## Current Product Posture

Ordo is a strong local appliance and eval foundation. It is not production
business automation yet.

The current product canon is surface-first:

```text
Member View
Studio
Support
Knowledge
Growth
Systems
```

Chat is the control surface where users direct, review, give feedback, and
approve work. Jobs, tasks, requests, artifacts, events, read models, access,
and outcomes are the product spine.

Implemented foundations include system shell, process kernel, diagnostics,
reports, backups, provider config, policy, MCP projection, public read models,
entry points, offers/trials, connections, availability/handoffs, knowledge
corpus, conversation realtime, LLM gateway, deterministic/replay/live-smoke
evals, journey personas, artifact review, and cross-journey reports.

Future work includes production public portals, auth UI, broad live-provider
orchestration, embeddings/vector search, provider-backed RAG answers, real
outbound email, payments, hosted trial capacity/reset orchestration, reward
programs, benefit grants, external transports, Worker Ordos, and A2A.

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
