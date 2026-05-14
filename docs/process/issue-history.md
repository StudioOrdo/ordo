# Issue History

Status: public manufacturing record map

This document summarizes the completed and active GitHub issue arcs that shaped
the current Ordo repository. Issues and pull requests remain the public
manufacturing record; this file is the reader-friendly index.

Snapshot date: 2026-05-14.

## Milestone Map

| Milestone | Issue range | Meaning |
| --- | --- | --- |
| 0.1.0 Appliance Specimen | #3, #5-#10 | Proved the first local appliance: Rust daemon, Next.js system shell, SQLite kernel, backup/restore, events, briefs, Docker packaging, and release evidence. |
| 0.1.1 Appliance Trust Boundary | #17-#24 | Stabilized diagnostics, report evidence, local support packet boundaries, protected routes, policy audit, and runtime proof. |
| 0.1.2 Backend MVP Readiness | #49-#60, #73-#76, #78-#80 | Built backend foundations for install/providers/vault, business truth, public read models, entry points, offers/trials, connections, availability/handoff, reports, corpus retrieval, answer drafts, MCP packs, and backend handoff docs. |
| 0.1.3 Conversation Realtime Spine | #82-#107 | Built the conversation architecture canon, role-aware IA, durable conversation schema, `/chat/ws`, message lifecycle, read state, reactions, presence, conversation UI, LLM gateway, tool approval, privacy egress, token ledger, continuous analysis, graph candidates, artifacts, surface briefs, realtime hardening, and ethical persuasion slot. |
| 0.1.4 Product Workflow Evals And Real LLM Readiness | #128-#134, #137-#148 | Built deterministic workflow evals, transcript artifact packets, role lifecycle coverage, feedback/review pressure tests, replay fixtures, OpenAI-compatible adapter, guarded live eval runner, simulator contracts, and artifact review classification. |
| 0.1.5 Live Product Journey Evals | #162-#170 | Built persona library, multi-case planning, QR-to-trial journey, review-return journey, affiliate-referral journey, admin/staff journey, and cross-persona analyzed reports. |
| 0.1.6 OrdoOS Frontend Architecture Foundation | #180-#187 | Built the frontend substrate for shell contracts, role-safe projections, screen context, experience settings, realtime read models, browser capability candidates, and hardening gates. |
| 0.1.7 Product Onboarding Surfaces | #205-#213 | Superseded before implementation after the May 12, 2026 direction reset; issues were closed stale/not planned. |
| 0.1.8 Interactive Account And LLM Chat | #214-#221, #228-#231 | Older open arc for local account/session contract, chat bootstrap, browser `/chat/ws`, deterministic LLM chat, guarded OpenAI-compatible local testing, UI run states, provider readiness resolvers, and end-to-end smoke evidence. Some issues remain open and should be reconciled against current code before new work depends on them. |
| 0.1.9 OrdoStudio NYC Pilot Foundations | #272, #305-#314 | Active accepted batch for product-contract backend foundations around job run idempotency, structured task result envelopes, pause/resume/skip DAG primitives, surface object timelines, and product request spine projection. |

Older unmilestoned issues include early project setup, product framing,
architecture drafts, and planning notes. Treat milestone arcs and merged PR
evidence as stronger than unmilestoned exploratory issues.

## How To Read Old Issues

- Closed issues describe accepted slices and evidence at the time they merged.
- Backlog docs turn broad ideas into issue-ready scopes.
- State docs summarize what remained true after later implementation.
- Current source and tests override old issue assumptions.
- If an old issue describes future behavior that is not in code, treat it as
  direction, not shipped functionality.

## Public Evidence Chain

The intended chain for completed work is:

```text
docs/backlog spec -> implementation issue -> test-plan issue -> branch
-> focused tests -> QA -> PR -> checks -> review -> merge -> state/process docs
```

The current agent-assisted workflow is documented in
[Agent Development Workflow](agent-development-workflow.md). It treats GitHub as
the public manufacturing ledger: issues own accepted work, test-plan issues own
coverage contracts, pull requests own implementation evidence, and merge-backed
comments close the loop.

Important evidence documents:

- [Release 0.1.0](release-0.1.0.md)
- [Diagnostics And Reports Runtime Proof 1.0](diagnostics-reports-runtime-proof-1.0.md)
- [Backend Handoff Package 0.1.2](backend-handoff-package-0.1.2.md)
- [Live Product Journey Evals](../architecture/conversation-realtime/live-product-journey-evals.md)
- [State Of The Project](../state-of-the-project.md)

## Current Active Direction

The active product direction is Studio Ordo hosted appliance foundations. The
current accepted 0.1.9 batch is tracked by #272 and pairs each implementation
issue with a linked test-plan issue:

- #305 / #306: job run idempotency keys for DAG starts;
- #307 / #308: structured task result envelopes for job DAG execution;
- #309 / #310: pause, resume, and skip primitives for job DAG execution;
- #311 / #312: surface object timeline projection;
- #313 / #314: product request spine projection for human input.

The latest #272 manifest identifies #305 as the next eligible implementation
issue after the landing gate cleared. Older 0.1.8 issues remain open and should
be cleaned up or reconciled through the Research workflow before being treated
as current executable work.
