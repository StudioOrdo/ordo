# Conversation Realtime Implementation Plan

Status: Staged delivery plan with protocol, schema, core service,
bidirectional gateway, receipts/presence, first premium UI core, UI
recovery/accessibility hardening, LLM foundations, continuous analysis,
knowledge graph candidates, offer/ask attribution foundation, and realtime
release hardening implemented. The final 0.1.3 slice implements the
`ethical_business_persuasion` v1 prompt slot contract.

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

Implementation status: implemented for `/chat/ws` as a protected local daemon
route. The first slice supports hello, identify, subscribe/unsubscribe,
heartbeat, resume/replay, message submit/edit/delete/undo, structured rejection
for unsupported commands, per-conversation local fanout, bounded broadcast
channels, message command rate limiting, and ephemeral typing start/stop events.
Read/unread receipt rollups remain Phase 3.

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

Implementation status: implemented for the backend/protocol slice. The daemon
now supports read/unread receipts, participant read-state rollups, unread count
recalculation after create/delete/read/unread, idempotent reactions,
policy-filtered presence snapshots, and `/chat/ws` commands for
`message.mark_read`, `message.mark_unread`, `message.react`, and
`presence.update`. Delivered/displayed viewport receipts, mention/action-needed
classification, and premium UI rendering remain later slices.

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

Implementation status: implemented for the first local UI core. `/chat` and
`/conversations` now render a brief-first conversation workspace with role-aware
client/staff surfaces, queue rows, narrative brief, timeline, composer,
optimistic send/retry, edit, undo tombstones, reactions, read/unread controls,
typing/presence, recovery/replay states, first-unread/latest anchors,
safe-area composer behavior, explicit action labels, reduced-motion handling,
and browser smoke coverage. The UI uses deterministic `conversation.gateway.v1`
fixture behavior for smoke tests; live browser WebSocket binding, provider
streaming, artifacts, and full multi-device receipt precision remain later
slices.

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
- Pending optimistic messages reconcile after recovery by `clientId` without
  duplicate timeline rows.

## Phase 5: LLM Gateway Streaming

Status: foundation implemented in PR #117, governed tool approval foundation
implemented in the #92 slice, and the ethical persuasion v1 prompt slot
implemented in the #107 slice. The implemented slices add the
Rust-owned provider adapter contract, deterministic local provider test path,
prompt slot assembly, `llm.invoke` and `llm.cancel` capabilities, policy
decision evidence, normalized ephemeral/durable LLM events, cancellation, and
durable final assistant messages through the conversation event stream. Tool
requests now record durable approval/rejection/execution evidence and require
registered exported capabilities before execution. The ethical persuasion
builder requires evidence/source refs for each allowed principle, blocks
unsupported persuasion claims and coercive language, exposes staff reasoning,
keeps client language agency-preserving, and records the slot through prompt
slot accounting. External provider calls, arbitrary MCP execution, the privacy
egress firewall, and full token ledger tables remain deferred to their owning
phases.

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

Status: foundation implemented in the #93 slice. Provider-bound user and prompt
slot payloads now pass through a daemon-owned privacy egress firewall before the
provider adapter sees them. The foundation detects obvious API keys, bearer
tokens, emails, phone numbers, and configured private terms; replaces them with
scoped placeholders; stores mappings through the local encrypted vault boundary;
emits metadata-only durable privacy events; and reconstructs placeholders only
on the local return path for matching transform scope.

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

Status: foundation implemented in the #94 slice. Schema version 21 adds
`llm_invocations`, `llm_prompt_slot_usage`, and
`llm_token_ledger_entries`. The Rust-owned LLM gateway records allowed
invocations, prompt-slot accounting, provider-reported input/output token usage,
safe terminal states, and query-backed rollups by conversation, provider/model,
capability, and prompt slot. Cost data is explicitly conservative and
not-priced until real provider pricing evidence is introduced.

Deliverables:

- Add invocation, prompt slot usage, and ledger entry tables.
- Defer pricing snapshot tables until provider pricing evidence exists.
- Record estimated prompt slot tokens before provider calls.
- Record provider-reported usage after provider calls.
- Add rollup read models for conversation, provider, model, capability, and slot
  analysis.

Exit criteria:

- Owner can inspect token usage like a storage breakdown.
- Tests prove slot totals reconcile to invocation totals where provider data is
  available.
- Tests prove accounting rows/events do not contain raw prompt text, user text,
  provider text, or privacy-sensitive values.
- The `ethical_business_persuasion` prompt slot is accounted like every other
  slot while keeping evidence/source refs, version, visibility, content hash,
  and deterministic token estimates.

## Phase 8: Continuous Analysis And Briefs

Status: foundation implemented in the #95, #102, #103, #104, and #105 slices. Schema
version 22 adds `conversation_analysis_jobs`,
`conversation_analysis_candidates`, `conversation_brief_candidates`, and
`conversation_memory_candidates`. Schema version 23 adds
`knowledge_graph_node_candidates` and `knowledge_graph_edge_candidates`. Schema
version 24 adds `referral_records`, `business_outcomes`, and
`business_outcome_attributions`. Schema version 25 adds normalized `artifacts`,
`artifact_versions`, `artifact_links`, and `artifact_deliverables`.
Schema version 26 adds `surface_briefs`.
Eligible visible durable messages queue idempotent local analysis jobs. The
deterministic analyzer creates proposed operational candidates, a narrative
brief candidate, and a relationship-memory candidate with evidence refs,
provenance, safe summaries, content hashes, and no automatic truth promotion.
Deterministic graph extraction can create proposed staff-private node and edge
candidates from completed analysis jobs. Offer acceptance now records an
evidence-backed outcome and attribution candidates for offer, visitor session,
and entry point influence when those ids exist. Artifact cards now use Artifact
as the staff/system noun and Deliverable as the client-facing projection noun
where intentionally exposed. Surface brief refresh jobs now produce
evidence-backed deterministic briefs linked to generated artifacts while the UI
continues to load previous completed briefs during refresh. The
`ethical_business_persuasion` v1 slot is implemented as an evidence-backed
prompt slot and staff-only UI guidance contract. Provider-backed analysis
remains deferred.

Deliverables:

- Queue analysis after eligible durable message creation.
- Update rolling summary and action counts from deterministic local signals.
- Create proposed open question, action-needed, handoff signal, brief, and
  memory candidates.
- Create proposed knowledge graph node and edge candidates from source message
  evidence.
- Record offer/ask/referral/outcome attribution foundation with proposed
  evidence-backed influence rows.
- Record normalized artifacts, artifact links, version hashes, and client-safe
  deliverable projections.
- Record deterministic surface brief jobs/read models for initial surfaces, with
  latest-completed-first loading and artifact linkage.
- Record ethical persuasion prompt slot guidance with evidence/source refs,
  staff-only reasoning, client-safe language, and prompt-slot accounting.
- Keep memory/corpus promotion behind a later governed approval path.

Exit criteria:

- Analysis is bounded, policy-aware, and resilient to provider unavailability.
- Brief candidates cite durable conversation evidence.
- Memory candidates require approval and do not auto-promote to corpus or
  business truth.
- Graph, persuasion, and attribution outputs remain candidates until confirmed
  through governed paths.

## Phase 9: Hardening

Status: implemented for the 0.1.3 local realtime release slice. The gateway
keeps SQLite as truth and treats WebSocket fanout as a bounded projection and
command transport. The release hardening adds structured oversized-frame
rejection, retryable lagged-client replay instructions, replay gap/idempotency
tests, message command flood tests, and explicit docs evidence for the remaining
transport and load risks.

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

Deferred release-hardening work:

- broad many-client load tests and measured SLOs;
- heartbeat timeout eviction for abandoned browser sockets;
- distributed fanout or cross-process gateway coordination;
- provider-backed continuous analysis and measured production load envelopes.

## Phase 10: Product Workflow Evals And Real LLM Readiness

Status: accepted 0.1.4 milestone. Phase 0 product/eval alignment is complete,
and the Phase 1 deterministic backend eval harness foundation is implemented.
This is an eval/manufacturing arc, not a feature-expansion sprint. The purpose
is to prove the product shape through deterministic workflow evidence before
relying on live providers or owner usage.

Delivery order:

1. Align the product workflow eval canon with `product_ux2`.
2. Add deterministic backend eval harness foundation. Implemented as
   `crates/ordo-daemon/src/eval_harness.rs` with deterministic case, step,
   actor-role, evidence snapshot, assertion, and scorecard summary types.
3. Add transcript artifact packets and scorecards. Implemented with
   `EvalArtifactWriter`, JSON packet/scorecard/manifest output, durable ledger
   sections, redaction summaries, and artifact-review placeholders for #140.
4. Implement first deterministic backend workflow evals. The initial slice is
   implemented with `relationship_conversation_message` and
   `privacy_gateway_roundtrip`, both provider-free and packet-backed.
5. Implement role lifecycle workflow evals. The initial slice is implemented
   with `role_lifecycle_anonymous_to_client`,
   `role_lifecycle_staff_manager_owner_boundaries`, and
   `role_lifecycle_agent_silence_boundary`, all deterministic and packet-backed.
6. Implement Customer Feedback and Review workflow evals. The initial slice is
   implemented with `feedback_capture_private_business_intelligence` and
   `review_candidate_consent_publication_boundary`, backed by minimal durable
   feedback, tag, and review tables.
7. Implement Home/About and Offer/Ask product surface workflow evals. The
   initial slice is implemented with `home_about_public_narrative_brief` and
   `offer_ask_machine_readable_intent`, backed by public business facts,
   surface-brief/artifact evidence, product-surface packet ledgers, and
   unsupported-proof guardrails. Dedicated Home/About billboard and offer/ask
   intent tables remain deferred until later eval evidence requires them.
8. Wire handoff, mode, and delegation gateway command coverage. Implemented
   with `/chat/ws` commands for handoff create/lifecycle, conversation mode
   set/human-led/return-to-agent, and scoped agent delegate/revoke, plus durable
   replay and trust-boundary tests.
9. Add replay-provider fixture support. Implemented with
   `ReplayLlmProvider`, `ordo.llm_replay_fixture.v1`, a redacted tiny fixture,
   request fingerprint matching, usage replay through the token ledger, and a
   packet-backed `replay_provider_fixture` eval.
10. Add real provider adapter behind the Rust-owned LLM gateway. Implemented
    for OpenAI-compatible non-streaming Chat Completions-style responses with a
    `reqwest` transport, mocked-transport default tests, privacy-transformed
    request input, safe provider failure normalization, and usage recording
    through the existing token ledger.
11. Add opt-in live eval runner with network and spend guards. Implemented with
    `live_eval_runner`, `ordo-daemon run-live-llm-eval-json`, required
    live/network env guards, OpenAI provider/model/key checks, conservative
    max-case and budget caps, mocked no-network default tests, and a packet-
    backed `live_openai_compatible_smoke` path through the existing LLM gateway.
12. Add artifact review automation that classifies findings and drives
    follow-on issues. Implemented as `eval_artifact_review`, a deterministic
    local classifier for eval packets that writes review JSON and
    `artifact-review.md`, maps findings to subsystem categories/severities,
    emits local redacted issue drafts where appropriate, and does not call
    GitHub, providers, or the network.
13. Define customer, operator, and reviewer simulator prompt contracts.
    Implemented with `docs/architecture/conversation-realtime/simulators/` and
    `eval_simulators`, a deterministic schema validator for
    `ordo.eval_simulator_output.v1` outputs that keeps simulators as
    non-authoritative pressure signals, requires evidence/assertion refs, and
    rejects unsafe raw values or pass/fail authority fields.

Phase boundaries:

- Deterministic evals run by default and never need provider keys or network.
- Replay fixtures run in CI only after redacted fixtures are approved.
- Live provider evals are opt-in and require explicit network and spend guards.
- Generated findings become GitHub issues only after they have transcript,
  ledger, scorecard, and smallest-responsible-subsystem evidence.
- Simulator outputs create customer/operator/reviewer pressure for future
  replay/live runs, but deterministic assertions and durable evidence remain
  authoritative.
- Product additions such as Customer Feedback, Reviews, Home/About billboards,
  and Offer/Ask intent metadata should be implemented only after eval evidence
  clarifies the smallest useful schema and UI contract.
