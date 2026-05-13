# Ordo System Overview

Status: canonical public map for the current repository

Date: 2026-05-09

This document is the short, current map of Ordo for developers, reviewers, and
LLM agents. It summarizes what the system is, what is implemented, what remains
future work, and where to read next.

When this document and another source disagree, trust current code and tests
first, then [State Of The Project](state-of-the-project.md), then this map.

## Product Thesis

Ordo is a local-first AI appliance for one-person businesses.

The owner works in conversation. Behind the conversation, Ordo stores durable
business truth, routes governed work, preserves evidence, creates artifacts,
tracks outcomes, and produces briefs that explain what happened and what should
happen next.

The core principle is:

```text
Human decides. Assistant operates. Process governs.
```

Ordo is not a generic chatbot, CRM, dashboard bundle, or arbitrary plugin
runtime.

## Runtime Shape

| Layer | Responsibility |
| --- | --- |
| Rust daemon | SQLite migrations, appliance supervision, health/readiness, process kernel, backup/restore, realtime events, protected routes, MCP projection, policy, provider boundaries, conversation and eval foundations. |
| Next.js | Local management UI, system shell, conversation surfaces, read models, route composition, and user-facing product states. |
| SQLite | Source of truth for durable appliance, product, policy, conversation, artifact, eval, and accounting state. |
| WebSocket | Live projection and bidirectional conversation command transport. It is not the source of truth. |
| Docker | One appliance image with `.data` as the durable local boundary. |

The durable spine is:

```text
Capability Catalog -> Process Template -> Job -> Task DAG -> Event -> Artifact -> Brief
```

The conversation and LLM spine extends that into:

```text
Conversation Command -> Policy Decision -> Prompt Slots -> Privacy Egress -> Provider Adapter -> Message/Event/Artifact -> Accounting -> Analysis -> Replay
```

## Implemented Now

- One Docker appliance image runs the Rust daemon and supervised Next.js
  standalone server.
- SQLite initializes through ordered migrations and seeds the local appliance
  baseline.
- System shell surfaces exist for Brief, Health, Backup And Restore, Schedules,
  Preferences, Events, Logs, and Reports.
- The job/task kernel runs concrete brief, backup, restore-preflight, report,
  and support-packet-local workflows.
- Diagnostics, logs, reports, local exports, support packet drafts, and support
  packet receipts are local evidence records. No report or support packet is
  sent to an external service.
- Provider configuration, local owner/business identity, and encrypted local
  vault items exist behind protected daemon routes. Provider keys are write-only
  through HTTP read models.
- Business facts, public surface read models, entry points, visitor sessions,
  offers, offer acceptances, trials, outcomes, attribution, feedback, reviews,
  connections, grants, availability, handoff inbox records, and receipts exist
  as backend foundations.
- The capability catalog, local MCP JSON-RPC projection, MCP pack metadata,
  route contracts, policy decisions, local roles, and resource grants are
  implemented as the local governance boundary.
- The knowledge corpus uses SQLite FTS for access-aware retrieval and answer
  draft scaffolds. Provider-backed RAG answers remain future work.
- Conversation realtime foundations are implemented: conversations,
  participants, messages, edit/undo/delete, receipts/read state, reactions,
  presence, modes, handoffs, segments, `/chat/ws`, replay, surface brief records,
  artifacts/deliverables, and premium conversation UI surfaces.
- LLM foundations are implemented: deterministic provider, replay fixtures,
  OpenAI-compatible non-streaming provider adapter, prompt slots, policy
  decisions, privacy egress, token ledger accounting, tool approval lifecycle,
  analysis candidates, knowledge graph candidates, and ethical business
  persuasion guardrails.
- Eval foundations are implemented: deterministic eval harness, transcript
  artifact packets, artifact review classifier, simulator contracts, persona
  library, guarded live OpenAI-compatible smoke runner, QR-to-trial journey,
  review-return journey, affiliate-referral journey, admin/staff journey, and
  cross-journey reports.

## Not Built Yet

- Production-ready public Chat, About, Offers, Asks, and Feed portals.
- Hosted identity, authentication UI, OAuth/email login, and production RBAC UI.
- Broad live-provider orchestration across all product answers.
- Embeddings, vector search, provider-backed RAG answer generation, chat
  retrieval UI, and content packs.
- Real outbound email, payments, affiliate payout automation, and external
  support/report transports.
- Worker Ordos, A2A networking, hosted trial orchestration, and arbitrary
  third-party MCP execution.
- Full visual regression coverage.

## Source Layout

| Path | Purpose |
| --- | --- |
| `app/` | Next.js app routes for the local UI and API route wrappers. |
| `components/` | Shared React components for system and conversation surfaces. |
| `lib/` | TypeScript clients, protocol types, and UI support code. |
| `crates/ordo-daemon/src/` | Rust daemon, SQLite schema, routes, domain services, policy, evals, and runtime. |
| `docs/` | Public doctrine, architecture, process, decisions, backlog, and eval fixtures. |
| `docs/evals/personas/` | Synthetic persona fixtures for live journey eval planning and pressure tests. |
| `tests/ui/` | Playwright smoke tests with a lightweight mock daemon. |
| `docker/` | Runtime helper scripts used by the Docker image. |

## Canonical Reader Paths

For a new developer:

1. [README](../README.md)
2. [Developer Guide](developer-guide.md)
3. [State Of The Project](state-of-the-project.md)
4. [Architecture](architecture/README.md)
5. [Eval System](evals/README.md)
6. [Issue History](process/issue-history.md)

For an LLM coding agent:

1. [LLM Agent Guide](llm-agent-guide.md)
2. [State Of The Project](state-of-the-project.md)
3. [Backend Handoff Package 0.1.2](process/backend-handoff-package-0.1.2.md)
4. [Interactive Account And LLM Chat](architecture/conversation-realtime/interactive-account-llm-chat.md)
5. [Live Product Journey Evals](architecture/conversation-realtime/live-product-journey-evals.md)
6. Current source and tests for the files being changed.

For product direction:

1. [Business Canon](business/README.md)
2. [Product Shape](business/product-shape.md)
3. [Ordo Core](business/ordo-core.md)
4. [Product Roadmap](business/product-roadmap.md)
5. [Conversation Product Doctrine](architecture/conversation-realtime/product-doctrine.md)
