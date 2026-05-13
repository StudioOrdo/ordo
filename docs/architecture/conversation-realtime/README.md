# Conversation Realtime Architecture

Status: Draft contract for mediated chat, LLM streaming, presence, receipts,
brief-first product surfaces, and luxury conversation UX

This packet describes the next Ordo conversation architecture. It is grounded in
the current appliance spine and intentionally keeps Rust as the control point
for durable state, policy, privacy, provider egress, token accounting, and
realtime sequencing.

## Current Grounding

The current codebase already provides the spine this work should extend:

- `crates/ordo-daemon/src/events.rs` defines `RealtimeEvent`, persists events
  into `realtime_events`, and supports replay by global cursor.
- `crates/ordo-daemon/src/server.rs` exposes `/ws`, currently as an outbound
  broadcast stream of persisted appliance events.
- `docs/architecture/realtime-events.md` establishes that WebSocket is a live
  projection and SQLite is the source of truth.
- `docs/backlog/mediated-chat.md` defines the planned durable nouns:
  Conversation, Participant, Message, Transcript Summary, and Conversation
  State.
- `docs/architecture/capability-catalog.md` keeps execution governed by
  registered capabilities rather than arbitrary tool or model calls.
- `docs/architecture/resource-provenance-policy.md` defines the actor, action,
  resource, capability, decision, and provenance vocabulary.
- `docs/architecture/knowledge-corpus.md` provides the access-aware retrieval
  substrate future chat answers must use before provider-backed generation.
- `docs/architecture/connections.md` and
  `docs/architecture/availability-and-handoff.md` already include local
  connection events, receipts, operator presence, handoff events, and handoff
  receipts that conversation chat should reuse rather than duplicate.

## Packet Contents

- [Architecture](architecture.md) describes the runtime boundaries, command and
  event flow, durable versus ephemeral event split, LLM gateway integration,
  privacy firewall, token ledger integration, and knowledge graph hooks.
- [Product Doctrine](product-doctrine.md) defines the role-aware navigation,
  one-relationship-conversation model, staff handoff queues, governed handoff
  object, episodes, artifacts, surface briefs, offer/ask measurement, and
  ethical persuasion prompt-slot doctrine that implementation must preserve.
- [Event Protocol](event-protocol.md) defines the bidirectional WebSocket
  gateway envelope, command catalog, durable event catalog, ephemeral event
  catalog, receipt semantics, and replay rules.
- [Data Model](data-model.md) proposes the SQLite tables, migrations, indexes,
  and read models needed for one conversation per user, messages, receipts,
  unread counts, presence, privacy transforms, token usage, and analysis jobs.
- [Frontend Experience](frontend-experience.md) defines the luxury chat surface,
  states, microinteractions, read/unread behavior, typing indicators, AI
  activity, accessibility, and mobile behavior.
- [Implementation Plan](implementation-plan.md) stages the work from protocol
  and schema through chat UI, provider streaming, privacy, token accounting,
  analysis, and briefs.
- [Test Plan](test-plan.md) defines schema, policy, realtime, recovery,
  privacy, token ledger, LLM gateway, UI, and smoke validation.
- [Product Workflow Evals And Real LLM Readiness](real-llm-e2e-evals.md)
  defines the 0.1.4 validation arc: deterministic product workflow evals,
  transcript artifacts, role lifecycle coverage, replay fixtures, guarded live
  provider evals, and artifact-review-driven follow-up work.
- [Live Product Journey Evals](live-product-journey-evals.md) defines the
  completed 0.1.5 persona-backed QR-to-trial, review-return, affiliate,
  admin/staff, and analyzed report eval arc.
- [Product Onboarding Surfaces](product-onboarding-surfaces.md) records the
  superseded 0.1.7 arc for QR/event landing, offer/trial, client conversation,
  review-return, referral, and staff review surfaces.
- [Interactive Account And LLM Chat](interactive-account-llm-chat.md) defines
  the active 0.1.8 contract for local login/register, chat bootstrap, browser
  `/chat/ws`, deterministic LLM chat, guarded OpenAI-compatible testing, UI run
  states, and end-to-end smoke evidence.

## Design Doctrine

Conversation is not a side channel. It is another projection of the Ordo spine:

```text
Capability Catalog -> Conversation Command -> Policy Decision -> Durable Event -> Artifact/Message -> Brief
```

The chat surface should feel as responsive as a top-tier social app while the
product remains a brief-first relationship operating system:

- clients participate in one persistent relationship conversation;
- staff operate handoffs, queues, business work, and briefs;
- admins operate the appliance;
- LLM jobs create episodes, tags, graph candidates, briefs, and recommendations
  from evidence;
- offers and asks are measurable business instruments;
- artifacts are durable knowledge/business objects;
- public/member/client navigation is separate from staff and admin appliance
  navigation.

Every durable action should be explainable. Every external model call should be
mediated. Every private value leaving the appliance should pass through the
egress firewall. Persuasive recommendations should use the
`ethical_business_persuasion` prompt slot and must remain evidence-backed,
inspectable, respectful, and agency-preserving.
