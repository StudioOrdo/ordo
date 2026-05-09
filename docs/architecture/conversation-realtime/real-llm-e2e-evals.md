# Product Workflow Evals And Real LLM Readiness

Status: 0.1.4 eval contract and current-code assessment

This document defines how Ordo should validate product functionality before the
owner relies on it in real use. The arc starts with deterministic backend
workflow evals and transcript artifacts, then adds replay-provider fixtures, and
only later runs guarded live LLM provider evals. Real-provider evals remain part
of the plan, but they are not the first proof loop.

Real-provider evals must be opt-in. They spend money, touch live external
systems, and exercise the privacy egress boundary. They should never run as part
of the default local test matrix unless an explicit environment flag enables
them.

The product shape under test is a briefing-first relationship and business
intelligence system for solopreneurs and small teams. Evals should validate the
role lifecycle for anonymous visitors, authenticated clients/members,
affiliates, staff, managers/admins, owner/system admins, the Ordo agent, and
LLM/tool/provider boundaries. They should also validate Customer Feedback,
reviews, Home/About billboards, Offers/Asks as business intent objects, and
artifact/review/outcome evidence loops.

The next eval arc is documented in
`docs/architecture/conversation-realtime/live-product-journey-evals.md`.
0.1.5 should use the 0.1.4 harness, packet, live-runner, simulator, and
artifact-review foundations to validate persona-driven QR/event journeys,
30-day trials, review-return loops, affiliate referrals, staff handoffs, and
cross-persona analyzed reports.
The 0.1.5 Phase 1 persona library and validator are implemented under
`docs/evals/personas/` and `crates/ordo-daemon/src/eval_personas.rs`.
The 0.1.5 Phase 2 multi-case runner foundation is implemented in
`crates/ordo-daemon/src/live_eval_runner.rs`; it plans persona-backed live
journey cases and writes a guarded manifest without executing a journey by
itself.
The 0.1.5 Phase 3 QR-to-trial journey eval is implemented in
`crates/ordo-daemon/src/live_eval_runner.rs`; default tests use the
deterministic provider path to create a public 30-day trial offer, event QR
entry point, visitor session, relationship conversation, privacy/accounted LLM
response, offer acceptance, started trial, business outcome, and attribution
evidence. Live provider execution remains opt-in through the existing guards.
The 0.1.5 Phase 4 review-return journey eval is implemented in
`crates/ordo-daemon/src/live_eval_runner.rs`; default tests reuse the
QR-to-trial setup, create a redacted simulated review-request email/link
artifact, resume the relationship conversation through a return visitor
session, capture private feedback, create a review candidate, prove publication
is blocked before consent and approval, and exercise publish, feature, and
retire visibility boundaries without real email delivery or network calls.
The 0.1.5 Phase 5 affiliate-referral journey eval is implemented in
`crates/ordo-daemon/src/live_eval_runner.rs`; default tests create an active
affiliate connection, scoped conversation grant, referral entry point, referred
visitor session, deterministic conversation/LLM response, offer acceptance,
started trial, referral record, referral-linked business outcome, and
referral/affiliate attribution while proving unrelated conversation access is
denied.
The 0.1.5 Phase 6 admin/staff handoff and moderation journey eval is
implemented in `crates/ordo-daemon/src/live_eval_runner.rs`; default tests
create a relationship conversation, governed handoff, staff and manager queue
evidence, human-led/delegated/returned mode boundaries, moderated review
publication, affiliate grant revocation, and redacted packet/manifest artifacts
without provider keys or network calls.

## Current Assessment

The current codebase can already validate much of the business spine without a
live model:

- `crates/ordo-daemon/src/conversation_protocol.rs` defines the
  `conversation.gateway.v1` envelope and `/chat/ws` route contract.
- `crates/ordo-daemon/src/conversation_gateway.rs` handles bidirectional
  gateway frames, identify, subscribe, replay, message submit/edit/delete/undo,
  mark read/unread, reactions, presence, and typing.
- `crates/ordo-daemon/src/conversations.rs` owns conversation, participant,
  message, receipt, read-state, handoff, mode, queue, and mutation behavior.
- `crates/ordo-daemon/src/conversation_analysis.rs` queues and runs
  deterministic local analysis for summaries, open questions, action-needed
  signals, handoff signals, brief candidates, and memory candidates.
- `crates/ordo-daemon/src/llm_gateway.rs` owns prompt slots, LLM policy
  decisions, privacy egress, deterministic provider streaming, replay-provider
  fixtures, OpenAI-compatible non-streaming provider normalization, final
  assistant message persistence, tool request lifecycle, and token accounting
  hooks.
- `crates/ordo-daemon/src/privacy_egress.rs` transforms emails, phone numbers,
  API-key-shaped values, bearer tokens, and configured private terms into
  placeholders, then reconstructs only in scope.
- `crates/ordo-daemon/src/llm_accounting.rs` records invocation starts,
  prompt-slot estimates, privacy transform ids, provider-reported usage, and
  token ledger rollups.
- `crates/ordo-daemon/src/install.rs` recognizes provider secret env keys for
  Anthropic, OpenAI, and DeepSeek and exposes redacted provider configuration.

The current codebase now has the first real-provider adapter foundation:

- `LlmProviderAdapter` supports deterministic, replay fixture, and
  OpenAI-compatible non-streaming provider paths.
- `answer_drafts.rs` still records local scaffold behavior for answer drafts
  and does not call providers.
- The Rust workspace uses `reqwest` for the OpenAI-compatible HTTPS transport.
  SSE streaming and provider SDK crates remain out of scope.
- There is no opt-in live-provider eval runner or spend guard yet.

That means Ordo can run high-value deterministic E2E tests today, but real LLM
E2E still requires the #134 implementation slice: an opt-in runner with network
and spend guards.

## Environment Readiness

The local `.env.local` file now contains real provider-related keys. Do not
print or commit secret values.

Current code-recognized provider key names:

- `ANTHROPIC_API_KEY`
- `API__ANTHROPIC_API_KEY`
- `OPENAI_API_KEY`
- `API__OPENAI_API_KEY`
- `DEEPSEEK_API_KEY`

Observed local key names include Anthropic and OpenAI keys plus a lowercase
DeepSeek-shaped key. The provider catalog expects `DEEPSEEK_API_KEY`, so a
lowercase `deepseek` key will not be recognized by current Rust provider config
resolution unless normalized by shell tooling or renamed.

Recommended live-eval guard variables:

- `ORDO_LIVE_LLM_EVALS=1` enables live-provider evals.
- `ORDO_LIVE_LLM_ALLOW_NETWORK=1` is required by tests that would otherwise be
  deterministic-only.
- `ORDO_LIVE_LLM_PROVIDER=openai` selects the implemented live provider path.
- `ORDO_LIVE_LLM_MODEL=<model>` selects the OpenAI-compatible model.
- `OPENAI_API_KEY` or `API__OPENAI_API_KEY` supplies the provider secret.
- `ORDO_LIVE_LLM_MAX_CASES=<n>` caps spend per run.
- `ORDO_LIVE_LLM_BUDGET_USD=<amount>` sets a hard budget guard.
- `ORDO_LIVE_LLM_BASE_URL=<url>` optionally overrides the OpenAI-compatible
  base URL.
- `ORDO_LIVE_LLM_TIMEOUT_MS=<milliseconds>` optionally overrides the transport
  timeout.
- `ORDO_LIVE_LLM_ARTIFACT_DIR=<path>` optionally chooses the artifact output
  directory for the CLI runner.
- `ORDO_LIVE_LLM_RECORD_REPLAY=1` stores redacted replay fixtures.

Live evals should refuse to run if either `ORDO_LIVE_LLM_EVALS` or
`ORDO_LIVE_LLM_ALLOW_NETWORK` is missing. The implemented Phase 6 runner also
refuses to run without an OpenAI model and provider key. Missing max-case or
budget env vars use conservative defaults of one case and `$0.01`; malformed
values fail closed.

## Sibling Patterns To Borrow

Nearby projects show useful patterns, but no drop-in Ordo eval runner.

Reusable ideas:

- keep replay fixtures separate from live provider calls;
- write structured review/eval packets to disk;
- validate response contracts and retry or coerce when safe;
- expose runtime provider status before generation;
- support fake provider fallback for deterministic tests;
- capture usage and latency per provider call;
- never log request secrets.

These map directly to Ordo's needs: deterministic CI remains stable, while
opt-in live evals produce redacted artifacts for review and regression replay.

## Eval Delivery Order

Use this order so the system can be autonomously developed from durable
evidence rather than live-model impressions:

1. Deterministic backend workflow evals.
2. Transcript artifact packets and scorecards.
3. Product role lifecycle evals.
4. Customer Feedback and Review workflow evals.
5. Home/About and Offer/Ask product surface evals.
6. Replay provider fixtures.
7. Real provider adapter.
8. Opt-in live eval runner with network and spend guards.
9. Artifact review loop that classifies findings and drives follow-on issues.
10. Customer, operator, and reviewer simulator contracts for future replay/live
    workflow pressure.
11. Live product journey evals that run persona-driven QR-to-trial,
    review-return, affiliate-referral, and staff/admin handoff paths behind
    explicit live-provider guards. Phases 1-6 now provide the committed
    synthetic persona library, deterministic validator, multi-case guard/cap
    planner, QR-to-trial execution, review-return execution,
    affiliate-referral execution, and admin/staff moderation execution.

Every implementation phase should begin by re-reading the issue, current docs,
and current code because earlier eval work may change the best implementation
path.

## Backend-Only Early Proof Evals

Backend-only evals should be the first implementation stage. They prove the
appliance brain before Ordo spends money on live providers or time on frontend
polish.

The goal is not to simulate the UI. The goal is to prove that a business
conversation can move through the Rust daemon spine and leave behind the right
durable evidence:

```text
Seed SQLite -> Conversation Command -> Policy Decision -> Prompt Slots -> Privacy Transform -> Provider Adapter -> Message/Event/Artifact -> Accounting -> Analysis -> Replay
```

These evals should run against an isolated SQLite database, use the existing
deterministic provider by default, and drive the same backend services or
HTTP/WebSocket routes that production uses. They should not depend on
Playwright, screenshots, browser timing, or real provider keys.

Backend-only evals should answer these questions early:

- can Ordo create or resolve the one canonical relationship conversation for a
  user, visitor, or connection;
- can a message submit flow persist the message, receipts, read model, and
  realtime events atomically;
- can the LLM gateway assemble prompt slots from the right evidence and record
  slot hashes and token estimates;
- can the privacy egress firewall placeholder sensitive values before the
  provider boundary;
- can the deterministic provider exercise the same completion path that a live
  provider will use later;
- can the assistant response, invocation status, accounting rows, and durable
  events be replayed into a coherent projection;
- can deterministic conversation analysis create summaries, open questions,
  handoff signals, brief candidates, and memory candidates when expected;
- can scorecards show token and event evidence without raw secrets or private
  fixture text.

Recommended backend eval layout:

```text
tests/evals/backend/
  cases/
    relationship_conversation_message.json
    privacy_gateway_roundtrip.json
    token_slot_rollup.json
    deterministic_analysis_handoff.json
    replay_projection_rebuild.json
  scorecards/
    <timestamped backend eval packets>
```

Each backend scorecard should include:

- seeded fixture ids and hashes;
- command ids and correlation ids;
- policy decision ids and outcomes;
- durable event ids, types, cursors, and conversation sequences;
- prompt slot ids, slot names, hashes, visibility class, and estimated tokens;
- privacy transform id and placeholder counts by detector type;
- invocation id, provider id, model id, status, and usage if available;
- token ledger deltas by conversation, capability, provider, model, and slot;
- analysis job ids and candidate ids created;
- replay cursor used and rebuilt projection checksum;
- pass/fail assertions and redacted failure notes.

Backend-only evals should become the default proof loop for early development:

1. Add or change backend behavior.
2. Run focused backend eval cases with deterministic provider.
3. Review scorecard deltas for events, prompt slots, privacy, accounting, and
   analysis.
4. Promote stable live-provider outputs into replay fixtures only after the
   deterministic path is trustworthy.

This keeps the live model as a later variable instead of the first unknown. It
also matches Ordo's architecture: SQLite is the source of truth, Rust is the
control point, and WebSocket/UI state is a projection that can be validated
after the durable spine is proven.

Implemented Phase 1 harness foundation:

- `crates/ordo-daemon/src/eval_harness.rs` defines the deterministic backend
  eval harness contract.
- Eval cases declare case id, title, fixture hash, actor roles, ordered steps,
  expected evidence channels, and assertion thresholds.
- Actor roles cover anonymous visitor, client/member, affiliate, staff,
  manager/admin, owner/system admin, Ordo agent, and the
  LLM/tool/provider-boundary actor.
- Evidence snapshots explicitly count SQLite rows, conversation events,
  realtime replay rows, policy decisions, prompt slot accounting, privacy
  transforms, token ledger entries, analysis candidates, handoff state,
  artifact records, and surface brief records.
- Missing optional channels are represented with count `0`; they are not
  silently ignored.
- Scorecard summaries are deterministic, provider-free, network-free, and carry
  a placeholder artifact path for Phase 2 transcript packet work.
- The first harness test exercises an anonymous visitor relationship
  conversation and message path through the current schema and durable event
  spine.

The harness does not yet write full transcript artifact packets or scorecard
files. That remains the Phase 2 scope.

Implemented Phase 2 transcript packet and scorecard foundation:

- `EvalArtifactWriter` writes a JSON packet, JSON scorecard, and manifest for a
  deterministic eval case.
- Packet schema version is `ordo.eval_artifact_packet.v1`.
- Packets include case metadata, actor roles, ordered steps, evidence
  snapshots, assertion results, transcript entries, timeline entries,
  conversation event ledger, realtime replay ledger, policy decision ledger,
  prompt-slot ledger, privacy transform ledger, token ledger, analysis
  candidate ledger, handoff ledger, artifact ledger, surface brief ledger, a
  redaction summary, and an artifact-review placeholder.
- Missing optional ledgers are represented as empty arrays rather than omitted.
- Scorecard JSON remains smaller than the packet and keeps the deterministic
  Phase 1 pass/fail summary.
- Manifest JSON includes schema version, run id, case ids, validation status,
  source commit, actor roles, packet path, and scorecard path.
- Redaction runs before packet serialization and replaces obvious emails,
  phone numbers, bearer/API-key-shaped tokens, and configured private terms with
  placeholders while preserving hashes, ids, counts, source refs, and metadata.

Artifact review finding classification remains Phase 7 (#140). The Phase 2
packet only reserves the placeholder and stable categories.

## Workflow Pressure Tests

The eval suite should work like a pressure test for the application. A coding
agent should be able to run realistic workflows, collect transcript artifacts,
review the artifacts, and then work through the surfaced failures one by one.
The point is to make hidden product and architecture problems visible: missing
events, weak handoff boundaries, awkward prompt slots, privacy leakage,
incorrect unread state, incomplete queues, token waste, and places where the
backend cannot yet express the product contract.

Each workflow should bundle several lower-level cases into one real-world
scenario. The suite should still record every assertion separately so a failed
workflow tells the agent exactly which subsystem failed.

Workflow evals should use three kinds of actors:

- deterministic seeded actors for system invariants and policy checks;
- LLM-simulated customer actors for varied customer language, urgency, tone,
  confusion, objections, and follow-up behavior;
- LLM-simulated human operator actors for handoff acceptance, staff replies,
  delegation to Ordo, idle behavior, and return-to-agent behavior.

The customer and operator simulators are test drivers, not authority. Their
messages create realistic pressure, but pass/fail remains grounded in durable
backend evidence: rows, events, policy decisions, prompt slots, accounting,
analysis candidates, handoff state, replay behavior, and redacted transcript
artifacts.

Current implementation grounding:

- `/chat/ws` currently supports identify, subscribe, replay, message submit,
  edit, delete, undo, mark read/unread, reactions, presence, typing, and
  heartbeat.
- `conversations.rs` already supports canonical conversations, participants,
  message idempotency, receipts, read state, reactions, presence snapshots,
  handoff records, handoff transitions, conversation modes, queue scopes,
  agent public-post etiquette decisions, and episode candidates.
- `llm_gateway.rs` already supports deterministic provider runs, prompt slots,
  privacy transform, assistant message persistence, invocation/accounting
  events, cancellation, and governed tool request approval/execution lifecycle.
- `conversation_analysis.rs` already queues message analysis and creates
  summary, open-question, action-needed, handoff-signal, brief, and memory
  candidates.
- Handoff commands are named in the protocol, but the current gateway command
  switch only wires message, read, reaction, presence, typing, subscribe, and
  replay commands. Handoff and agent-mode workflow evals should initially drive
  backend domain functions directly, then move to gateway tests as commands are
  wired.

### Transcript Test Artifacts

Every workflow should produce a transcript artifact packet. The packet is the
object that the coding agent and reviewers inspect after the run.

Recommended layout:

```text
tests/evals/artifacts/<run-id>/
  manifest.json
  transcript.redacted.jsonl
  timeline.md
  scorecard.json
  event-ledger.json
  db-ledger.json
  prompt-slots.json
  privacy-ledger.json
  token-ledger.json
  analysis-candidates.json
  handoff-ledger.json
  replay-check.json
  artifact-review.md
```

Artifact rules:

- `transcript.redacted.jsonl` contains actor labels, message ids, timestamps,
  message hashes, redacted excerpts, and generated simulator intent labels.
- `timeline.md` is human-readable and ordered by durable sequence/cursor, not
  by local test timing.
- `event-ledger.json` records expected and actual event types, sequence order,
  cursor continuity, durability, and broadcast/replay visibility.
- `db-ledger.json` records the rows created or updated by table and id.
- `prompt-slots.json` records slot ids, labels, visibility ceilings, source
  refs, content hashes, estimated tokens, inclusion decisions, and truncation
  reasons.
- `privacy-ledger.json` records detector counts and placeholders, never raw
  sensitive fixture values.
- `token-ledger.json` records usage by run, provider, model, capability,
  conversation, and prompt slot.
- `analysis-candidates.json` records candidate kinds, states, confidence,
  evidence refs, visibility, and content hashes.
- `handoff-ledger.json` records handoff status transitions, actor ids, mode
  changes, assignment, allowed context, and brief evidence.
- `replay-check.json` records the cursor/sequence replay request and a checksum
  of the rebuilt projection.
- `artifact-review.md` is produced after the run by deterministic checks plus,
  optionally, an LLM reviewer over the redacted artifacts.

The artifact packet should be red-team friendly. If a transcript contains a
phone number, email, token-like value, private business term, unsupported tool
request, duplicate command, or staff-only note, the packet should make it easy
to prove whether Ordo contained it correctly.

### Artifact Review Loop

After each workflow run, the coding agent should analyze the artifact packet
before editing code.

Review procedure:

1. Confirm the workflow completed or failed at an expected boundary.
2. Diff expected and actual durable events by event type, sequence, cursor, and
   payload hash.
3. Check database row counts and required foreign-key links.
4. Scan redacted transcript excerpts for customer/operator realism and missing
   product behavior.
5. Verify no raw private fixture value appears outside allowed local storage.
6. Inspect prompt slots for missing evidence, over-broad visibility, stale
   context, or token bloat.
7. Inspect token ledger rollups for the largest cost drivers.
8. Inspect analysis candidates for false positives, false negatives, missing
   evidence refs, and unsafe visibility.
9. Inspect handoff state for correct assignment, mode, queue visibility, and
   agent-silence behavior.
10. File or implement fixes against the smallest responsible subsystem.

The review output should classify findings:

- `schema_gap`: required durable noun or index is missing;
- `event_gap`: durable or ephemeral event is missing, duplicated, unordered, or
  not replayable;
- `policy_gap`: role, visibility, queue, or tool authority is wrong;
- `privacy_gap`: sensitive value left the allowed boundary;
- `prompt_gap`: slot selection, evidence, visibility, or truncation is wrong;
- `handoff_gap`: mode, assignment, allowed context, queue, or agent etiquette is
  wrong;
- `analysis_gap`: candidate, brief, memory, or action count is wrong;
- `accounting_gap`: invocation, usage, slot, rollup, or budget accounting is
  wrong;
- `ux_contract_gap`: backend cannot yet support the desired product experience;
- `provider_gap`: live/replay provider response handling is wrong.

Implemented Phase 7 artifact review classifier behavior:

- `eval_artifact_review` reads `ordo.eval_artifact_packet.v1` packet JSON and
  writes `ordo.eval_artifact_review.v1` review JSON plus `artifact-review.md`.
- Findings are deterministic local candidates with category, severity, status,
  case id, source artifact hash, evidence refs, suggested owner subsystem, and
  optional local GitHub issue draft text.
- The classifier maps failed assertions and missing ledger evidence to the
  smallest responsible subsystem. Examples include token ledger gaps to
  `accounting_gap`, handoff ledger gaps to `handoff_gap`, analysis candidate
  gaps to `analysis_gap`, and provider failure evidence to `provider_gap`.
- Raw emails, phone numbers, API-key-shaped strings, bearer-token-shaped
  strings, and configured private fixture terms become `privacy_gap` blockers.
- Redaction markers are recorded as containment evidence, not automatic
  failures.
- The classifier does not call providers, GitHub, or the network. Issue drafts
  are local redacted text only; filing remains a governed human/agent workflow.

Implemented Phase 8 simulator contract behavior:

- `docs/architecture/conversation-realtime/simulators/` defines versioned
  customer, operator, reviewer, and shared schema contracts for future
  replay/live workflow pressure.
- `crates/ordo-daemon/src/eval_simulators.rs` defines
  `ordo.eval_simulator_output.v1` and validates customer, operator, and
  reviewer simulator outputs.
- Simulator outputs must cite redacted evidence or artifact refs, message
  hashes, expected pressure subsystems, safety constraints, and deterministic
  assertion refs.
- Reviewer outputs are restricted to the same finding taxonomy as
  `ordo.eval_artifact_review.v1`.
- Unknown roles, unknown fields, missing message hashes or excerpts, raw
  secrets, raw emails, raw phone numbers, configured private terms, and
  simulator-owned pass/fail fields are rejected.
- Simulator outputs remain candidate pressure signals. Deterministic backend
  assertions and durable evidence remain the pass/fail source of truth.

### Real-World Workflow Eval Suite

These workflows bundle the 100-plus atomic cases into product journeys. The
initial runner can execute the backend-only portions with deterministic actors;
later runs can swap customer/operator turns to LLM simulators and replay/live
provider adapters.

Role lifecycle workflows are first-class:

- anonymous visitor starts from Home/About, Offer, Ask, Latest, QR/link entry
  point, or Chat and receives a visitor-session-backed relationship
  conversation;
- authenticated client/member sees one relationship conversation and
  client-safe account tools;
- affiliate sees referral/account tools without unrelated customer data;
- business staff defaults to `My Handoffs`;
- manager/admin can inspect `Team Queue` and authorized `All Conversations`;
- owner/system admin can operate appliance surfaces while ordinary staff remain
  shielded from system internals;
- Ordo agent stays silent publicly during human-led active mode unless tagged,
  delegated, or policy requires intervention.

Product surface workflows are also first-class:

- Customer Feedback is captured as private business intelligence;
- feedback tags are proposed candidates with evidence;
- starred feedback influences feedback briefs without becoming a customer
  rating;
- review candidates require consent before publication;
- Home/About billboards require linked evidence or clear aspirational language;
- Offers/Asks remain human-readable and machine-readable business intent
  objects;
- ethical persuasion and brand profile guidance cannot invent scarcity,
  reviews, metrics, authority, or social proof.

1. `workflow_new_visitor_service_intake`
   - Customer simulator asks what the business does, shares a need, budget, and
     timing.
   - Assert canonical conversation, participants, message events, unread count,
     prompt slots, assistant response, analysis summary, intake/action
     candidate, brief candidate, memory candidate, and replay checksum.
2. `workflow_returning_connection_continuity`
   - Seed an existing connection and prior message, then customer returns with a
     follow-up.
   - Assert same relationship conversation or correct linked conversation,
     recent context slot, no duplicate active conversation, memory candidate
     evidence, and continuity in transcript artifact.
3. `workflow_privacy_contact_details_roundtrip`
   - Customer provides email, phone, and a private project codename.
   - Assert local message keeps policy-controlled original, provider payload is
     placeholdered, output reconstructs only in scope, scorecard has no raw
     private values, and privacy ledger records detector counts.
4. `workflow_urgent_human_request_handoff`
   - Customer says they need a human urgently.
   - Assert analysis creates `handoff_signal`, action count increments, a
     handoff record can be created with reason/urgency/allowed context, queue
     row appears for authorized staff, and handoff brief cites message evidence.
5. `workflow_handoff_accept_staff_reply`
   - Staff/operator simulator accepts the handoff and replies publicly.
   - Assert handoff transition to accepted/in progress, mode becomes
     `human_led_active`, read/unread states update, staff message is durable,
     client-visible transcript excludes internal handoff reasoning, and replay
     rebuilds the same state.
6. `workflow_human_led_agent_silence`
   - Customer asks another question while staff owns the conversation.
   - Assert `may_agent_post_publicly` blocks public agent response unless the
     operator tags/delegates Ordo or policy requires intervention.
7. `workflow_operator_delegates_ordo`
   - Operator simulator asks `@Ordo` for a private draft or specific public
     answer.
   - Assert delegation scope is recorded, prompt slots include only allowed
     context, assistant behavior matches scope, and public/private visibility is
     correct.
8. `workflow_staff_idle_private_reminder`
   - Staff accepts handoff, then no staff response occurs after the configured
     idle boundary.
   - Assert human-led idle mode, private reminder event, no public holding
     message before policy allows it, and staff-only visibility.
9. `workflow_return_to_agent`
   - Staff returns the conversation to Ordo with allowed context.
   - Assert handoff transition, `returned_to_agent` mode, allowed context in
     prompt slots, and agent can resume public posting.
10. `workflow_handoff_decline_recovery`
    - Staff declines a handoff with reason.
    - Assert declined terminal state, queue removal, conversation mode recovery,
      client-safe response, and no stale handoff in staff queue.
11. `workflow_multi_staff_queue_authorization`
    - Seed assigned and unassigned handoffs for staff, manager, admin, and
      client roles.
    - Assert staff sees only `My Handoffs`, manager sees `Team Queue`, admin sees
      `All Conversations`, and client cannot access staff queues.
12. `workflow_two_clients_isolation`
    - Two customer simulators converse concurrently with overlapping topics and
      private data.
    - Assert conversation ids, participants, read states, privacy transforms,
      analysis candidates, and prompt slots never cross-contaminate.
13. `workflow_read_unread_receipts_reconnect`
    - Customer sends multiple messages, staff marks read/unread, then reconnects
      from a cursor.
    - Assert unread counts, manual unread boundary, receipt rows, replay order,
      and optimistic/lost-ack reconciliation.
14. `workflow_typing_presence_luxury_signals`
    - Customer and operator simulators send typing and presence updates during a
      live handoff.
    - Assert typing is ephemeral, draft text is absent, presence visibility is
      policy-filtered, and reconnect snapshots do not invent durable events.
15. `workflow_message_edit_delete_undo_analysis_boundary`
    - Customer sends, edits, undoes, and deletes messages around analysis jobs.
    - Assert revisions, tombstones, undo grace behavior, read counts, and future
      prompt slots do not use deleted content.
16. `workflow_reaction_and_micro_acknowledgement`
    - Staff reacts to a customer message and customer reacts to staff.
    - Assert reaction idempotency, remove/toggle behavior, event visibility, and
      no unread inflation from reaction-only changes unless intended.
17. `workflow_offer_recommendation_evidence_backed`
    - Customer describes a need that matches a seeded offer.
    - Assert ethical recommendation slot, visible business facts only,
      offer-interest candidate, evidence refs, agency-preserving language, and
      token attribution.
18. `workflow_missing_evidence_refusal`
    - Customer asks for a claim that no approved corpus/business fact supports.
    - Assert answer states limitation, asks clarifying question or routes to
      handoff, and does not invent facts.
19. `workflow_conflicting_customer_memory`
    - Customer contradicts an earlier preference.
    - Assert memory candidate remains proposed, contradiction/clarification
      signal is recorded when supported, and prompt slots do not promote
      unapproved truth.
20. `workflow_sensitive_topic_safe_handoff`
    - Customer shares sensitive personal or business information that should not
      be handled casually.
    - Assert privacy transform, handoff signal, staff-private candidate
      visibility, client-safe response, and no external leakage.
21. `workflow_tool_request_requires_approval`
    - Assistant needs a governed capability during a conversation.
    - Assert tool request recorded, unknown/dangerous tools rejected,
      review-required execution blocks until approval, approval records policy,
      and result returns through gateway only.
22. `workflow_tool_rejection_customer_recovery`
    - Operator rejects a tool request.
    - Assert assistant receives refusal context, customer response remains
      helpful, no tool side effect occurs, and policy audit explains why.
23. `workflow_provider_failure_no_bad_state`
    - Deterministic/replay provider fails mid-run.
    - Assert invocation failure, no final assistant message, transcript marks
      safe failure, accounting records what it can, and retry/fallback policy is
      explicit.
24. `workflow_cancel_llm_run`
    - Operator cancels an in-flight run.
    - Assert policy decision, provider cancellation call, `llm.run.cancelled`, no
      duplicate assistant message, and clean transcript state.
25. `workflow_token_budget_pressure`
    - Seed a long history and retrieval evidence that exceeds the configured
      budget.
    - Assert deterministic truncation, slot-level token accounting, largest-cost
      scorecard section, and refusal or compacting behavior when budget is
      exceeded.
26. `workflow_replay_after_client_lag`
    - Simulate a lagged client and replay after cursor/sequence.
    - Assert client receives replay guidance, replay returns durable events in
      order, ephemeral typing is not treated as durable, and projection checksum
      matches.
27. `workflow_duplicate_command_idempotency`
    - Submit duplicate `clientMessageId` and lost-ack retry patterns.
    - Assert one message row, one canonical event sequence, stable ack/replay
      reconciliation, and no duplicate analysis jobs.
28. `workflow_public_client_boundary`
    - Client attempts to access staff/admin-visible handoff, policy, prompt, or
      logs state through conversation views.
    - Assert denied or filtered data, no staff rail leakage, and transcript only
      shows client-appropriate language.
29. `workflow_report_generation_after_failure`
    - Force a workflow failure and then prepare diagnostic/report evidence.
    - Assert issue report captures relevant event/accounting/health evidence
      without secrets and gives a useful repair trail.
30. `workflow_backup_restore_preserves_conversation_spine`
    - Run conversation activity, create backup, restore preflight/safe restore
      boundary, then inspect conversation state.
    - Assert durable messages, events, accounting, candidates, and handoff rows
      remain consistent across the backup/restore proof boundary.
31. `workflow_live_provider_smoke_customer_operator_sim`
    - With live guards enabled, use a tiny prompt to simulate customer and
      operator turns against one provider.
    - Assert provider auth, model, usage, latency, redaction, and transcript
      scorecard without judging full business quality yet.
32. `workflow_cross_provider_business_regression`
    - Replay the same redacted workflow against OpenAI, Anthropic, and DeepSeek
      adapters when available.
    - Assert contract equivalence, compare rubric scores, compare token/cost
      profile, and identify provider-specific failures.

These workflows should be implemented gradually, but the inventory should stay
larger than the immediate implementation. The suite is a map of where pressure
belongs, not a promise that every product surface is already wired through one
public route.

Additional product workflow inventory from the product shape:

33. `workflow_role_lifecycle_anonymous_to_client`
    - Visitor enters through Home/About or Offer and later authenticates.
    - Assert one relationship conversation survives identity attachment, staff
      internals remain hidden, and account tools change only after authorization.
34. `workflow_affiliate_referral_surface_boundary`
    - Affiliate views referral tools and chat cards.
    - Assert affiliate can see own referral/outcome evidence and cannot see
      unrelated customer conversations or owner-only internals.
35. `workflow_staff_manager_owner_navigation_boundaries`
    - Staff, manager/admin, and owner/system admin open conversation/product
      surfaces.
    - Assert staff defaults to My Handoffs, manager can see Team Queue, owner can
      inspect All Conversations/system areas, and ordinary staff cannot see
      Logs/Backup/readiness/policy internals.
36. `workflow_feedback_capture_and_tag_candidates`
    - Conversation contains positive feedback, pricing confusion, and a feature
      request.
    - Assert feedback items cite message evidence, tags default to proposed, and
      graph candidates stay private/proposed.
37. `workflow_feedback_star_affects_brief`
    - Staff stars high-signal feedback.
    - Assert the feedback brief includes starred feedback as high-signal
      business intelligence without treating it as a rating or public proof.
38. `workflow_review_consent_publication_boundary`
    - Positive feedback becomes review candidate, review requested, received,
      consented, approved, published, featured, and retired.
    - Assert no public review/testimonial appears before consent and approval.
39. `workflow_home_about_billboard_evidence`
    - Home/About refresh proposes billboards from offers, asks, reviews,
      artifacts, latest activity, outcomes, and chat CTA.
    - Assert each claim links to evidence or is marked aspirational; fake
      scarcity, fake metrics, and fake reviews are rejected.
40. `workflow_offer_ask_intent_matching`
    - Seed offer/ask intent metadata and a conversation need.
    - Assert matching remains proposed, humans/policy decide what becomes real,
      and attribution links only when source ids exist.

### Simulator Design

Customer simulator prompts should be scenario-specific and constrained. They
should produce messages, not assertions. Include traits such as impatience,
budget sensitivity, uncertainty, urgency, objection, typo-prone mobile style,
and privacy-sensitive disclosure.

Human operator simulator prompts should model staff behavior that Ordo must
support: accepting handoffs, sending concise replies, asking Ordo for a draft,
delegating a scoped task, going idle, returning to agent, declining handoff,
and rejecting unsafe tool requests.

Simulator output should be stored as redacted transcript turns with:

- `actorKind`: `customer_simulator`, `operator_simulator`, `ordo_agent`,
  `system`, or `reviewer`;
- `intentLabel`: short label such as `asks_for_human`, `shares_contact`,
  `accepts_handoff`, or `delegates_private_draft`;
- `messageHash` and redacted excerpt;
- `expectedPressure`: subsystem expected to react, such as privacy, handoff,
  accounting, read-state, or replay.

The LLM reviewer should only see redacted artifacts. It should produce finding
labels and suggested investigation targets, not direct code changes.

## Live Provider Adapter Slice

Before true real-LLM E2E can run, add a Rust adapter behind
`LlmProviderAdapter`.

Minimum adapter contract:

1. Accept provider id, model id, base URL, redacted secret/config input, and a
   timeout boundary.
2. Compile Ordo prompt slots through `LlmGateway`, not inside the adapter.
3. Receive privacy-transformed prompt and user message only.
4. Call provider APIs over HTTPS.
5. Normalize provider streaming or non-streaming response into:
   - `TextDelta` events where streaming is available;
   - `Completed { text, usage }`;
   - `Failed { code, message }`.
6. Never log raw prompts, raw secrets, or raw provider response bodies by
   default.
7. Respect cancellation where provider transport allows it.
8. Record provider status and retry/fallback reason in metadata.

Recommended order:

1. Implement OpenAI-compatible non-streaming adapter first because DeepSeek can
   be made OpenAI-compatible and the shape is easiest to validate. Implemented
   in Phase 5 for request/response normalization and mocked-transport gateway
   tests.
2. Add SSE streaming once non-streaming evals pass.
3. Add Anthropic adapter with native event normalization.
4. Add provider fallback and per-provider response contract validation.

Dependencies to evaluate for Rust:

- `reqwest` for HTTPS; implemented for the first OpenAI-compatible transport;
- `reqwest-eventsource` or `eventsource-stream` for SSE;
- provider SDKs only if they simplify usage without hiding important event and
  accounting details.

## Eval Harness Shape

Add a separate harness rather than overloading unit tests.

Proposed layout:

```text
tests/evals/
  README.md
  backend/
    cases/
    scorecards/
  artifacts/
    <run-id>/
  simulators/
    customer.md
    operator.md
    reviewer.md
  cases/
    conversation_intake.json
    retrieval_grounded_answer.json
    privacy_placeholder_roundtrip.json
    handoff_detection.json
    offer_recommendation.json
  replays/
    <redacted provider fixtures>
  scorecards/
    <timestamped eval packets>
scripts/
  run-live-llm-evals.mjs
```

Each eval case should include:

- stable id;
- business purpose;
- setup records to seed into SQLite;
- actor plan and simulator prompts when the workflow uses LLM-simulated turns;
- conversation messages to submit;
- route under test: backend service, gateway frame, replay fixture, or live
  provider;
- provider and model policy;
- prompt-slot expectations;
- privacy expectations;
- transcript artifact expectations;
- output contract;
- deterministic assertions;
- optional LLM-graded rubric;
- token budget;
- expected artifacts/events;
- whether live network is required.

Each eval run should write a scorecard:

```json
{
  "schemaVersion": "ordo.eval.scorecard.v1",
  "runId": "eval_run_...",
  "providerId": "anthropic",
  "modelId": "...",
  "caseCount": 12,
  "passed": 11,
  "failed": 1,
  "totalTokens": 12345,
  "estimatedCostMicros": 0,
  "cases": []
}
```

Scorecards may include hashes, ids, event types, rubric scores, and redacted
excerpts. They must not include API keys, raw sensitive fixtures, or raw
provider prompts.

## Evaluation Layers

Use four layers. Do not jump straight to live providers for every assertion.

### Layer 1: Deterministic Domain E2E

Uses SQLite, conversation gateway handlers, deterministic provider, local
analysis, privacy transform, and token ledger. Runs in normal CI.

Validates:

- conversation state changes;
- event sequencing;
- read/unread;
- privacy transform;
- token ledger writes;
- deterministic analysis outputs;
- brief and memory candidates.

### Layer 2: Replay Provider E2E

Uses recorded redacted provider responses. Runs in normal CI after fixtures are
approved. The implemented Phase 4 replay layer adds
`ordo.llm_replay_fixture.v1` fixtures, a `ReplayLlmProvider` behind
`LlmProviderAdapter`, and a tiny approved fixture at
`crates/ordo-daemon/fixtures/llm-replay/tiny-success.json`.

Validates:

- provider event normalization;
- response parser behavior;
- usage parsing;
- reconstruction from placeholders;
- output contract validation;
- regression stability.

Replay fixtures match provider requests by a stable request fingerprint derived
from provider id, model id, prompt hash, and transformed user-message hash.
Fixtures must include provider/model ids, expected prompt slot ids, ordered
events, usage metadata for completed responses, redaction summary, provenance
refs, and timestamps. Fixture validation fails closed when the fixture contains
obvious raw emails, phone numbers, API-key-shaped strings, bearer-token-shaped
strings, or configured private fixture terms.

### Layer 3: Live Provider Smoke

Uses real keys and very small prompts. Runs only with live eval env guards.
The implemented CLI entrypoint is:

```bash
ORDO_LIVE_LLM_EVALS=1 \
ORDO_LIVE_LLM_ALLOW_NETWORK=1 \
ORDO_LIVE_LLM_PROVIDER=openai \
ORDO_LIVE_LLM_MODEL=<model> \
OPENAI_API_KEY=<redacted> \
ORDO_LIVE_LLM_MAX_CASES=1 \
ORDO_LIVE_LLM_BUDGET_USD=0.01 \
cargo run -p ordo-daemon -- run-live-llm-eval-json \
  --db-path .data/live-eval.db \
  --output-dir .data/evals/live \
  --source-commit <commit>
```

Without the guards, the command returns a structured skipped or blocked JSON
summary and does not construct the network provider.

Validates:

- provider auth works;
- model name works;
- latency and usage are captured;
- privacy transform runs before network;
- minimal output contract can pass.

### Layer 4: Business Scenario Live Evals

Uses realistic seeded business data and conversation flows. Runs manually or in
controlled nightly jobs with spend caps.

Validates:

- business usefulness;
- groundedness;
- handoff judgment;
- offer/ask recommendation quality;
- conversation summary quality;
- token economics.

## Core Assertions

Every real-LLM E2E case should check these invariants:

- provider call goes through `LlmGateway`;
- policy decision exists for the LLM invocation;
- prompt slots are recorded with content hashes and token estimates;
- privacy transform runs before provider call;
- provider-bound payload contains placeholders for sensitive fixtures;
- final assistant message is persisted as a conversation message;
- `llm.run.completed` or `llm.run.failed` is recorded;
- token ledger entries exist when provider usage is available;
- no raw secret fixture appears in realtime event payloads, diagnostic logs,
  policy audit metadata, or token ledger metadata;
- output cites or references durable evidence when the case requires grounding;
- scorecard records pass/fail without leaking secrets.

## Use-Case Matrix

### Provider And Gateway Health

1. OpenAI-compatible minimal answer returns exact short text.
2. Anthropic minimal answer returns exact short text.
3. DeepSeek minimal answer returns exact short text.
4. Provider missing key fails with structured `provider_unavailable`.
5. Invalid model fails with structured provider error and no assistant message.
6. Timeout or cancellation emits `llm.run.cancelled` or `llm.run.failed`.
7. Provider usage maps into token ledger entries.
8. Provider latency and model id are recorded in scorecard metadata.

### Prompt Slots And Accounting

9. System policy slot is included and hashed.
10. Actor context slot is included only for authorized actor.
11. Business truth slot includes approved visible facts only.
12. Retrieval evidence slot is omitted when no evidence is visible.
13. Recent conversation window truncates deterministically under budget.
14. Tool schema slot records estimated tokens.
15. Ethical business persuasion slot is present only for recommendation cases.
16. Slot token rollup approximately reconciles to provider input usage.

### Privacy Egress

17. Email address is placeholdered before provider call.
18. Phone number is placeholdered before provider call.
19. API-key-shaped value is placeholdered before provider call.
20. Bearer token is placeholdered before provider call.
21. Configured private term is placeholdered before provider call.
22. Provider output containing known placeholder reconstructs locally.
23. Wrong-scope placeholder does not reconstruct.
24. Provider output containing unknown placeholder fails safely.
25. Scorecard contains hashes/placeholders, not raw sensitive values.

### Conversation Basics

26. Create canonical visitor conversation from entry point/session.
27. Attach known connection to existing visitor conversation.
28. Submit human message and receive assistant response.
29. Assistant response is persisted with correct participant id.
30. Message edit updates revision history and analysis input boundary.
31. Delete/tombstone prevents deleted content from future prompt slots.
32. Undo send works before undo expiry.
33. Reaction add/remove is reflected in read model and events.
34. Typing events never persist raw draft text.
35. Presence update is policy-filtered for visitor view.

### Read, Unread, And Receipts

36. New visitor message increments staff unread count.
37. Staff read advances last read boundary.
38. Mark unread from message moves unread boundary backward.
39. Mention count updates separately from unread count.
40. Action-needed count updates when analysis finds a request.
41. Delivered/displayed/read receipts do not leak staff-only metadata to client.
42. Reconnect replays missed durable message and read-state events.

### Governed Retrieval And Grounded Answers

43. Answer uses only approved corpus evidence.
44. Private owner-only corpus item is hidden from visitor answer.
45. Missing evidence response states limitation instead of inventing facts.
46. Answer cites source item ids or artifact refs.
47. Retrieval evidence token cost is attributed to retrieval slot.
48. Conflicting evidence produces uncertainty and asks for clarification.
49. Stale evidence is described as stale when metadata marks it stale.

### Business Intake

50. Visitor asks about services; Ordo summarizes need and asks next best
    qualifying question.
51. Visitor gives budget and timing; Ordo extracts structured intake candidate.
52. Visitor asks for unavailable service; Ordo responds honestly and routes to
    alternative offer or handoff.
53. Visitor provides contact details; privacy egress protects details while
    conversation stores them locally under policy.
54. Visitor expresses urgency; handoff signal candidate is created.
55. Visitor asks for human; handoff request is created when eligibility allows.

### Offers, Asks, And Ethical Recommendations

56. Ordo recommends an offer based on stated need and visible business facts.
57. Ordo refuses to recommend an offer when evidence is insufficient.
58. Ordo distinguishes public offer language from staff-private reasoning.
59. Ordo records offer interest candidate with evidence refs.
60. Ordo records accepted offer attribution to entry point/session when present.
61. Ordo suggests an ask only when aligned with user intent.
62. Recommendation uses respectful, agency-preserving language.

### Handoff And Staff Operations

63. Agent detects human-needed request and creates handoff signal.
64. Handoff eligibility respects availability schedule and operator presence.
65. Staff accepts handoff and conversation mode becomes human-led active.
66. Staff declines handoff and conversation returns to agent-led or screening.
67. Staff idle triggers assistive/private reminder behavior.
68. Staff returns conversation to agent with allowed context.
69. Handoff brief includes reason, urgency, allowed context, and evidence refs.
70. Client never sees staff-only handoff reasoning.

### Continuous Analysis And Briefs

71. Message queues analysis job.
72. Analysis creates conversation summary signal.
73. Question creates open-question candidate.
74. Request creates action-needed candidate and count increment.
75. Sensitive or human request creates handoff signal.
76. Brief candidate cites durable message evidence.
77. Memory candidate is proposed, not automatically promoted to truth.
78. Analysis failure records safe hash and does not block message delivery.

### Knowledge Graph Candidates

79. Person node candidate is proposed from conversation evidence.
80. Business/topic node candidate is proposed from conversation evidence.
81. Interest edge candidate links connection to offer/topic.
82. Contradiction candidate is created for conflicting user statements.
83. Candidate remains proposed until approved.
84. Rejected candidate is not used in later prompt slots.

### Tool Use And Approval

85. Model requests supported catalog capability and tool request is recorded.
86. Unknown tool is rejected.
87. Dangerous capability is blocked.
88. Review-required tool pauses until approval.
89. Approved tool execution records result summary.
90. Rejected tool returns refusal context to assistant.
91. Tool result is included in final answer only through gateway.

### Token Economics

92. Conversation token usage rolls up by provider/model.
93. Usage rolls up by capability.
94. Usage rolls up by prompt slot.
95. Usage rolls up by conversation.
96. Failed provider call records invocation failure.
97. Cancelled call records partial usage when provider returns it.
98. Eval run stops when max cases is reached.
99. Eval run stops when budget guard is exceeded.
100. Scorecard highlights largest token consumers.

### Recovery And Reliability

101. WebSocket reconnect resumes durable events after sequence/cursor.
102. Duplicate client command id does not duplicate message.
103. Lost ack reconciles after replay.
104. Slow client receives lag/replay guidance instead of unbounded memory.
105. Provider transient failure retries only within configured policy.
106. Provider fallback is recorded when used.
107. Replay fixture can reproduce a previous live eval deterministically.

### Public/Member/Client Surface Boundaries

108. Public visitor cannot access staff/admin navigation through conversation.
109. Client sees one relationship conversation, not internal episodes.
110. Staff sees My Handoffs first.
111. Manager/admin can inspect Team Queue where authorized.
112. Logs, backup, readiness, and low-level events stay admin/system surfaces.
113. Detail view opens to narrative brief before transcript/admin detail.

## First Eval Cases To Implement

Start small and high-signal:

1. `workflow_new_visitor_service_intake`
2. `workflow_privacy_contact_details_roundtrip`
3. `workflow_urgent_human_request_handoff`
4. `workflow_handoff_accept_staff_reply`
5. `workflow_read_unread_receipts_reconnect`
6. `workflow_duplicate_command_idempotency`
7. `workflow_replay_after_client_lag`
8. `workflow_token_budget_pressure`
9. `workflow_tool_request_requires_approval`
10. `workflow_public_client_boundary`

The first ten workflows should run backend-only where possible. Handoff
creation, handoff lifecycle transitions, conversation mode changes, and scoped
agent delegation now have `/chat/ws` command coverage for the accepted Phase 3D
surface. Remaining eval shortcuts should be treated as finding evidence only
when a workflow needs a command outside that supported set.

Implemented first slice:

1. `relationship_conversation_message`
   - Creates or finds the canonical visitor relationship conversation.
   - Adds a visitor participant and submits a sensitive fixture message through
     the current backend service path.
   - Asserts durable conversation events and realtime replay evidence.
   - Writes packet, scorecard, and manifest artifacts with email, phone,
     API-key-shaped, and configured private-term redaction.
2. `privacy_gateway_roundtrip`
   - Runs the Rust-owned LLM gateway through the deterministic local provider.
   - Exercises prompt-slot accounting, privacy egress metadata, policy evidence,
     token ledger rows, durable conversation events, and realtime replay.
   - Confirms provider-bound sensitive fixture content is not serialized into
     eval packet artifacts.

This slice deliberately stays backend-only and provider-free. It does not claim
full role lifecycle coverage, Customer Feedback/Review coverage, Home/About or
Offer/Ask product-surface coverage, replay-provider fixtures, or live-provider
readiness.

Implemented role lifecycle slice:

1. `role_lifecycle_anonymous_to_client`
   - Exercises anonymous visitor relationship conversation creation, client
     relationship conversation continuity, and affiliate denial for unrelated
     customer conversation access.
   - Records protected-route and resource policy decisions for client/member
     and affiliate trust boundaries.
   - Confirms packet output keeps handoff, prompt-slot, privacy-transform, and
     token-ledger ledgers empty for this client-facing lifecycle case.
2. `role_lifecycle_staff_manager_owner_boundaries`
   - Creates a governed handoff assigned to staff.
   - Asserts staff defaults to `My Handoffs`, manager/admin can inspect `Team
     Queue`, owner/system admin can inspect `All Conversations`, and ordinary
     staff cannot access `All Conversations`.
   - Records protected-route policy evidence for non-owner denial and owner
     loopback allowance on system internals.
3. `role_lifecycle_agent_silence_boundary`
   - Sets a conversation to `human_led_active`.
   - Asserts Ordo is blocked from public posting without tag, delegation, or
     policy-required intervention.
   - Records policy evidence for the blocked public-agent-post boundary while
     keeping prompt/provider internals out of the packet.

The role lifecycle slice still stays backend-only, deterministic, and
provider-free. It does not implement Customer Feedback/Review workflows,
Home/About or Offer/Ask product surface workflows, replay fixtures, live
provider adapters, or the full handoff/mode/delegation gateway command surface.

Implemented Customer Feedback and Review slice:

1. `feedback_capture_private_business_intelligence`
   - Seeds a client conversation message as durable feedback evidence.
   - Captures private customer feedback with evidence refs and provenance.
   - Stars the feedback as a staff signal, not a customer rating.
   - Proposes a feedback tag with `proposed` candidate state.
   - Confirms no public review/testimonial is created from private feedback.
2. `review_candidate_consent_publication_boundary`
   - Creates a review candidate from private feedback evidence.
   - Confirms publication fails closed before consent and approval.
   - Transitions through requested, received, consent confirmed, approved,
     published, featured, and retired states.
   - Confirms public review visibility appears only after consent and approval
     and is removed from the public list after retirement.

This slice adds the smallest durable feedback/review backend foundation needed
for deterministic evals. It does not implement Customer Feedback UI, Home/About
review presentation, broad review solicitation automation, or autonomous
artifact-review finding filing.

Implemented Home/About and Offer/Ask product surface slice:

1. `home_about_public_narrative_brief`
   - Seeds public Home/About billboard facts, draft/private excluded facts, a
     surface brief row, and a linked artifact.
   - Reads the Home/About product surface contract from durable public business
     facts rather than a dedicated billboard table.
   - Confirms the public billboard preserves evidence refs, allowed state,
     reduced-motion fallback, and source links including Chat.
   - Confirms draft and staff/private fixture text do not enter the public
     surface contract.
2. `offer_ask_machine_readable_intent`
   - Seeds public Offer and Ask intent facts with human-readable copy and
     machine-readable metadata.
   - Confirms intent objects preserve the future-A2A contract without claiming
     external A2A implementation.
   - Confirms humans/policy remain the boundary for what becomes real.
   - Confirms unsupported public scarcity/social-proof style claims are rejected
     by the product-surface contract.

This slice extends eval packets with a product-surface ledger for business
facts, offers, and outcomes. It intentionally does not add dedicated Home/About
billboard, offer intent, or ask intent tables yet; existing public business
facts are sufficient for this first deterministic eval proof.

Implemented Phase 3D gateway command coverage:

- `conversation.handoff.create` creates a durable handoff request from
  conversation evidence and broadcasts `conversation.handoff.requested`.
- `conversation.handoff.accept`, `conversation.handoff.decline`,
  `conversation.handoff.assign`, `conversation.handoff.return_to_agent`, and
  `conversation.handoff.close` drive the existing durable handoff lifecycle and
  replay as `conversation.handoff.*` events. Existing short aliases such as
  `handoff.accept` remain accepted for the current protocol fixtures.
- `conversation.mode.set`, `conversation.mode.human_led_active`, and
  `conversation.mode.return_to_agent` update the durable conversation mode and
  replay as `conversation.mode.changed`.
- `conversation.agent.delegate` and `conversation.agent.delegation_revoke`
  update scoped agent delegation through the same durable mode record. The
  delegate command requires a non-empty `delegationScope`.
- Unsupported or invalid command attempts continue to return structured
  gateway errors rather than pretending unsupported workflows succeeded.

The Phase 3D tests prove ack/client-id preservation, durable replay order, and
non-leakage of provider, privacy-transform, and policy internals in mode
dispatch payloads. Product workflow evals can now move covered handoff, mode,
and delegation paths through `/chat/ws` instead of direct-domain shortcuts as
each eval is revisited.

Implemented Phase 4 replay-provider fixture slice:

1. `ReplayLlmProvider`
   - Implements the existing Rust-owned `LlmProviderAdapter` boundary without
     network access or provider keys.
   - Loads redacted `ordo.llm_replay_fixture.v1` fixtures from JSON.
   - Matches requests by stable fingerprint and rejects unknown requests as
     canonical provider failures.
   - Replays ordered text deltas, final completion, provider usage metadata, or
     safe provider failure events through the existing `LlmGateway`.
2. `replay_provider_fixture`
   - Runs the committed tiny replay fixture through the eval harness.
   - Records policy, prompt-slot accounting, privacy transform, token ledger,
     conversation event, realtime replay, packet, scorecard, and manifest
     evidence.
   - Keeps packet output redacted and deterministic while preserving provider
     id/model id and usage metadata.

Implemented Phase 5 OpenAI-compatible provider adapter slice:

1. `OpenAiCompatibleProvider`
   - Implements the existing Rust-owned `LlmProviderAdapter` boundary for
     non-streaming Chat Completions-style responses.
   - Accepts provider id, model id, base URL, API key, and timeout config.
   - Builds provider requests only from `LlmGateway` prompt slots and
     privacy-transformed user content.
   - Uses a transport seam so default tests use a deterministic mock transport
     and never require provider keys or network.
   - Normalizes successful assistant text and provider usage metadata into the
     same completion path used by deterministic and replay providers.
   - Normalizes provider errors, transport failures, missing config, and
     unsupported response shapes into safe provider failures without raw
     request/response persistence.
2. Gateway integration evidence
   - Mocked adapter runs through policy, privacy egress, token ledger,
     conversation events, final assistant message persistence, and durable
     failure behavior.
   - Tests prove sensitive fixture text is transformed before the adapter sees
     it and does not appear in event payloads.

Implemented Phase 6 opt-in live eval runner slice:

1. `live_eval_runner`
   - Defines `ordo.live_eval_runner.v1` config, guard decision, spend/case cap,
     and redacted run summary contracts.
   - Requires `ORDO_LIVE_LLM_EVALS=1` and
     `ORDO_LIVE_LLM_ALLOW_NETWORK=1` before any network-capable provider can be
     constructed.
   - Supports the OpenAI-compatible provider path from Phase 5, one smoke case,
     conservative default caps, and fail-closed parsing for malformed caps.
   - Runs `live_openai_compatible_smoke` through `LlmGateway`, privacy egress,
     prompt slot accounting, token ledger, durable conversation events, and the
     existing `EvalArtifactWriter`.
2. CLI evidence
   - `ordo-daemon run-live-llm-eval-json` prints skipped/blocked/completed JSON
     summaries.
   - Default tests use a mocked OpenAI-compatible transport, require no keys,
     and do not call the network.

After those pass, add the first simulator and provider cases:

1. `workflow_live_provider_smoke_customer_operator_sim`
2. `workflow_returning_connection_continuity`
3. `workflow_operator_delegates_ordo`
4. `workflow_human_led_agent_silence`
5. `workflow_offer_recommendation_evidence_backed`
6. `workflow_missing_evidence_refusal`
7. `workflow_cross_provider_business_regression`

## Scoring Rubric

Each business scenario should receive both deterministic assertions and a rubric
score.

Suggested rubric dimensions:

- `groundedness`: answer only uses visible evidence;
- `policy_fit`: respects role, visibility, and handoff boundaries;
- `privacy_fit`: no leaked sensitive fixture;
- `business_usefulness`: moves the conversation forward;
- `clarity`: concise and understandable;
- `tone`: warm, professional, non-manipulative;
- `actionability`: creates or recommends the right next step;
- `token_efficiency`: avoids wasteful prompt or output size.

Use deterministic checks for pass/fail gates. Use LLM or human rubric scores to
compare quality across providers and prompt revisions.

## What To Build Next

1. Add backend-only eval case schema and isolated SQLite fixture setup.
2. Add transcript artifact packet writer with redacted transcript, timeline,
   ledgers, scorecard, replay check, and artifact review output.
3. Add deterministic backend eval runner and first workflow cases. The initial
   implemented cases are `relationship_conversation_message` and
   `privacy_gateway_roundtrip`; the remaining workflow inventory should be
   added in focused follow-on issues.
4. Add role lifecycle workflow evals. The initial implemented cases are
   `role_lifecycle_anonymous_to_client`,
   `role_lifecycle_staff_manager_owner_boundaries`, and
   `role_lifecycle_agent_silence_boundary`.
5. Add Customer Feedback and Review workflow evals. The initial implemented
   cases are `feedback_capture_private_business_intelligence` and
   `review_candidate_consent_publication_boundary`.
6. Wire currently direct-domain handoff/mode workflows through gateway commands.
   The accepted Phase 3D gateway slice now covers handoff create/lifecycle,
   mode set/human-led/return-to-agent, and scoped agent delegate/revoke
   commands through durable events and replay.
7. Add replay fixture support for provider-shaped responses. Implemented with
   `ReplayLlmProvider`, `ordo.llm_replay_fixture.v1`, a committed tiny success
   fixture, and a packet-backed `replay_provider_fixture` eval.
8. Add artifact reviewer that classifies findings by `schema_gap`, `event_gap`,
   `policy_gap`, `privacy_gap`, `prompt_gap`, `handoff_gap`, `analysis_gap`,
   `accounting_gap`, `ux_contract_gap`, and `provider_gap`.
9. Add customer and operator simulator prompts with redacted transcript turn
   capture.
10. Add real provider adapter behind `LlmProviderAdapter` with
   OpenAI-compatible non-streaming support. Implemented for the adapter and
   mocked gateway integration path.
11. Add opt-in live eval runner with env guards and spend caps. Implemented for
   one guarded OpenAI-compatible smoke case and mocked no-network default tests.
12. Add artifact reviewer that classifies findings by `schema_gap`,
   `event_gap`, `policy_gap`, `privacy_gap`, `prompt_gap`, `handoff_gap`,
   `analysis_gap`, `accounting_gap`, `ux_contract_gap`, `provider_gap`, and
   `test_fixture_gap`. Implemented as a local deterministic packet classifier
   with JSON and Markdown outputs and no automatic GitHub filing.
13. Add live product journey evals. The initial implemented cases are
   QR-to-trial, review-return, and affiliate-referral. QR-to-trial covers event
   QR, visitor session, relationship conversation, privacy/accounted daemon LLM
   response, offer acceptance, trial, outcome, and attribution. Review-return
   covers simulated review-request email/link evidence, return session, private
   feedback, review candidate, consent/approval publication guard, and
   publish/feature/retire visibility. Affiliate-referral covers affiliate
   connection/grant, referral entry/session, referred conversation,
   offer/trial, referral record, referral-linked outcome, attribution, and
   scoped visibility boundaries.
14. Add Anthropic and DeepSeek provider coverage.
15. Add SSE streaming normalization once non-streaming passes.
