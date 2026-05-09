# Conversation Realtime Implementation Plan

Status: Draft staged delivery plan

This plan keeps the first implementation small enough to validate while
preserving the architecture needed for premium realtime chat, brief-first
product surfaces, role-aware handoff work, LLM streaming, privacy, token
accounting, ethical recommendation slots, and continuous analysis.

## Product Alignment Phase

Before application code begins, publish the product doctrine in this packet and
update the milestone issues so implementation preserves:

- one client-visible relationship conversation with internal episodes;
- staff defaults to `My Handoffs`, `Team Queue`, and authorized `All
  Conversations`;
- top rail, staff rail, and admin/system rail separation;
- governed handoff objects and handoff briefs;
- human-led agent etiquette and idle recovery;
- artifacts as system noun and deliverables as client-facing language where
  useful;
- offers/asks as measurable instruments connected to referrals, outcomes,
  entry points, conversations, and artifacts;
- `ethical_business_persuasion` as an evidence-backed prompt slot, not hidden
  manipulation.

## Phase 0: Protocol And Capability Contract

Deliverables:

- Write the versioned gateway envelope types in Rust and TypeScript.
- Register initial capabilities for conversation read, message create, receipt
  write, presence write, handoff management, agent delegation, LLM invoke, LLM
  cancel, and tool approval.
- Decide whether chat uses `/chat/ws` or versions the existing `/ws` route.
- Define policy resource kinds for conversation, message, participant, handoff,
  episode/segment, prompt slot, LLM run, and receipt.
- Add protocol fixtures for command, ack, dispatch, error, heartbeat, and resume
  frames.
- Add protocol fixtures for conversation mode changes, handoff lifecycle events,
  episode/tag updates, brief candidates, and ethical recommendation candidates.

Exit criteria:

- Protocol types serialize and deserialize in Rust tests.
- TypeScript read model types match protocol fixtures.
- Capability catalog seed validates new capability ids.

## Phase 1: Conversation Core Schema

Deliverables:

- Add migrations for conversations, segments, participants, messages,
  conversation events, receipts, and read states.
- Add schema support or explicit deferred contracts for governed handoffs,
  conversation modes, tags, graph candidates, and surface briefs.
- Add Rust domain module for conversation creation and message submission.
- Add protected HTTP routes for conversation list, conversation read, and message
  submit if useful for non-WebSocket tests.
- Persist durable conversation events and mirror replayable events into
  `realtime_events`.
- Add idempotency by `clientMessageId`.

Exit criteria:

- Schema migration tests pass on fresh and upgraded databases.
- Message submission is atomic: no message without event and no event without
  message.
- Existing `/events` replay can show conversation events where projected.

## Phase 2: Bidirectional Gateway

Deliverables:

- Implement WebSocket read loop and write loop with bounded channels.
- Support `hello`, `identify`, `subscribe`, `command`, `ack`, `error`,
  `heartbeat`, and `resume`.
- Route `message.submit`, `message.mark_read`, `message.mark_unread`,
  `typing.start`, and `typing.stop`.
- Add per-conversation room subscriptions.
- Add rate limits for typing and message commands.

Exit criteria:

- Browser can submit a message over WebSocket and receive canonical dispatch.
- Reconnect after durable cursor replays missed message events.
- Typing events are broadcast live but not persisted as messages.

## Phase 3: Read/Unread And Receipts

Deliverables:

- Implement read state rollups.
- Emit receipt events for persisted, delivered, displayed, read, and marked
  unread where supported.
- Add conversation list read model with unread, mention, and action counts.
- Add role-aware queue read models for `My Handoffs`, `Team Queue`, and
  authorized `All Conversations`.
- Add mark-all-read and mark-unread-from-message behavior.

Exit criteria:

- Counts update correctly after create, delete, read, unread, and grant changes.
- UI can show unread divider and global attention badge from daemon state.

## Phase 4: Premium Chat UI

Deliverables:

- Build role-aware navigation, conversation queues, relationship conversation
  detail, narrative brief area, timeline, composer, typing indicator, receipt
  state, reaction controls, unread divider, and recovery banner.
- Add optimistic send and retry behavior.
- Add handoff brief before transcript and staff-only evidence/reasoning
  inspection for recommendations.
- Add mock daemon support in UI smoke tests.
- Add mobile layout coverage.

Exit criteria:

- UI smoke tests cover desktop and mobile send/read/typing/reconnect flows.
- No layout overlap in timeline, composer, badges, or message action controls.

## Phase 5: LLM Gateway Streaming

Deliverables:

- Add provider adapter abstraction for Anthropic, OpenAI, DeepSeek, and local.
- Normalize provider streams into Ordo events.
- Add prompt builder slots.
- Add `ethical_business_persuasion` prompt slot with evidence/source refs,
  guardrails, and token accounting.
- Add tool request/approval/result flow through the capability catalog.
- Persist final assistant messages and invocation metadata.
- Keep high-volume token deltas ephemeral; persist final usage.

Exit criteria:

- One provider can stream into a conversation through Rust only.
- Cancel stops provider work and emits canonical state.
- Tool use cannot bypass catalog policy.

## Phase 6: Privacy Egress Firewall

Deliverables:

- Add transform run and placeholder tables.
- Add initial detectors for obvious secrets, emails, phone numbers, API keys,
  and configured private terms.
- Replace sensitive spans with scoped placeholders before provider calls.
- Reconstruct placeholders only on local return path.
- Add inspectable metadata without logging raw values.

Exit criteria:

- Provider payload tests prove raw sensitive fixtures do not leave the daemon.
- Reconstruction only happens for known placeholders in the correct scope.

## Phase 7: Token Ledger

Deliverables:

- Add invocation, prompt slot usage, ledger entry, and pricing snapshot tables.
- Record estimated prompt slot tokens before provider calls.
- Record provider-reported usage after provider calls.
- Add rollup read models for conversation, provider, model, capability, and slot
  analysis.

Exit criteria:

- Owner can inspect token usage like a storage breakdown.
- Tests prove slot totals reconcile to invocation totals where provider data is
  available.
- Tests prove the ethical persuasion slot has evidence refs and cannot be used
  to invent urgency, authority, social proof, or relationship context.

## Phase 8: Continuous Analysis And Briefs

Deliverables:

- Queue analysis after durable message creation.
- Update rolling summary, episodes, tags, open questions, action items, handoff
  signals, ethical recommendation candidates, and brief candidates.
- Add knowledge graph candidate records.
- Add offer/ask/referral/outcome attribution candidates where evidence exists.
- Add surface brief jobs for business, conversations, connections, offers,
  asks, artifacts, jobs, affiliates, and customers.
- Add approval path for memory/corpus promotion.

Exit criteria:

- Analysis is bounded, policy-aware, and resilient to provider unavailability.
- Brief candidates cite durable conversation evidence.
- Graph, memory, persuasion, and attribution outputs remain candidates until
  confirmed through governed paths.

## Phase 9: Hardening

Deliverables:

- Backpressure and bounded memory tests.
- WebSocket load tests for many idle and active conversations.
- Failure injection for disconnects, provider errors, malformed frames, slow
  clients, and replay gaps.
- Security review of prompt egress, placeholder storage, receipt visibility, and
  policy decisions.

Exit criteria:

- Full validation matrix passes.
- Runtime behavior is documented in public architecture docs.
