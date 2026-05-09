# Conversation Realtime Test Plan

Status: Draft validation contract

Conversation realtime touches durable state, policy, WebSocket transport,
privacy, provider egress, token accounting, analysis, and UI. Validation should
scale with that risk.

## Product Contract Tests

Coverage:

- clients see one relationship conversation even when internal episodes,
  segments, handoffs, and provider runs exist;
- business staff default to `My Handoffs`, not an all-conversation feed;
- manager/admin roles can access `Team Queue` and authorized `All
  Conversations`;
- non-staff users never see the staff/admin rail;
- ordinary staff do not see Logs, Backup, readiness, low-level Events, or other
  appliance internals as primary navigation;
- the selected detail opens to a narrative brief before transcript/admin detail;
- Conversations appears above Connections in staff navigation;
- mobile follows menu -> evidence list -> detail brief;
- desktop follows top rail + staff/admin rail + evidence list + detail brief.

Evidence:

- frontend reducer and route tests for role-gated navigation;
- UI smoke tests for staff, admin, and client shells;
- route/read-model tests for conversation queue defaults.

## Baseline Validation Matrix

For shared behavior or issue completion, run the repository matrix:

```bash
npm run typecheck
npm run build
npm run smoke:ui
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

For Docker/runtime evidence, use a unique Compose project name, verify health,
readiness, affected UI/API behavior, and clean up disposable state.

## Schema Tests

Coverage:

- fresh database initializes every required conversation table;
- old database migrates to new schema version;
- required indexes exist;
- one active canonical conversation per user/surface is enforced;
- message sequence uniqueness per conversation is enforced;
- `clientMessageId` idempotency prevents duplicate sends;
- read state primary keys prevent duplicate participant state;
- handoff records require reason, urgency, status, assignment/ownership fields,
  allowed context, and evidence summary;
- conversation modes represent agent-led, human-led active, human-led idle,
  assistive, needs-handoff, and returned-to-agent states;
- episode/tag/graph candidates require candidate state, confidence where
  available, evidence refs, and generating job/provenance where available;
- surface briefs require evidence refs and limitations;
- foreign keys preserve message, receipt, participant, and conversation
  integrity;
- deleting or archiving records follows intended retention policy.

Evidence:

- Rust unit tests around schema initialization;
- migration tests for representative older `PRAGMA user_version` states;
- `REQUIRED_TABLES` updated when tables become implemented.

## Domain Tests

Conversation creation:

- creates conversation, segment, owner/visitor/assistant participants as needed;
- links visitor session or connection when supplied;
- rejects duplicate active canonical conversation when not intended;
- emits `conversation.created` in the same transaction.

Message submission:

- validates actor/participant access;
- creates message and event atomically;
- increments sequence monotonically;
- handles idempotent retry;
- rejects closed or paused conversations as policy/config requires;
- never writes blocked local-only drafts into durable message history.

Reactions:

- adds reaction once per participant/key;
- removes reaction idempotently;
- handles rapid duplicate toggles;
- emits canonical events.

Receipts and unread:

- persisted receipt follows message creation;
- delivered/displayed/read receipts update read state;
- mark unread moves manual unread boundary;
- deleting unread message recalculates counts;
- mention and action-needed counts update independently from unread count.

## Policy Tests

Coverage:

- message create requires conversation access;
- connection participants only see granted resources;
- visitor participants cannot inspect owner-only metadata;
- handoff-only actions require handoff eligibility or owner approval;
- handoff management requires the proper role/capability;
- staff default queue scope does not leak all conversations;
- client views cannot access internal handoff, policy, confidence, prompt, or
  LLM orchestration state;
- LLM invocation requires provider and capability authorization;
- tool approval requires authorized owner/operator actor;
- policy decisions are persisted for protected mutations;
- MCP cannot call unexported or dangerous conversation capabilities.

Evidence:

- Rust tests for policy allow, deny, and review-required paths;
- audit trail assertions for actor, action, resource, capability, outcome,
  reason, and correlation ids.

## WebSocket Protocol Tests

Unit tests:

- envelope parse rejects malformed JSON;
- oversized frames are rejected before JSON parsing;
- unsupported protocol version returns structured error;
- unknown command returns structured error;
- command ack includes original `clientId`;
- durable dispatch includes sequence and cursor when persisted;
- ephemeral dispatch excludes durable cursor unless snapshot state is persisted.

Integration tests:

- connect receives `hello`;
- identify binds actor and participant;
- subscribe receives initial snapshot;
- submit message receives ack and canonical dispatch;
- reconnect with cursor replays missed durable events;
- resume restores subscriptions and sends ephemeral snapshot;
- slow client receives backpressure/degraded behavior instead of unbounded memory
  growth;
- lagged broadcast emits a recoverable replay instruction;
- duplicate message-submit retry reconciles to the canonical message without
  duplicate durable rows or duplicate `message.created` events;
- high-frequency message command flood returns a structured rejection.
- handoff lifecycle commands emit accepted, declined, assigned,
  returned-to-agent, and closed events;
- conversation mode changes are durable and replayable;
- agent delegation changes are correlated to actor, policy decision, and
  evidence.

## Handoff And Agent Etiquette Tests

Coverage:

- good handoff triggers create governed handoff candidates with reason and
  brief evidence;
- routine greetings and successful agent-led replies do not create handoffs;
- staff sees handoff brief before transcript;
- agent remains silent publicly during `human_led_active` unless tagged,
  delegated, or policy requires intervention;
- `@Ordo` delegation enables the requested assistive behavior only in scope;
- human-led idle recovery privately reminds staff before any public holding
  message or return-to-agent behavior;
- client-facing language hides internal routing, confidence, and policy state.

## Typing And Presence Tests

Typing:

- `typing.start` broadcasts to authorized participants;
- rate limit suppresses excessive starts;
- `typing.stop` clears state;
- expiry clears state without stop;
- privacy setting disables outgoing typing;
- raw draft text is never present in events or logs.

Presence:

- presence heartbeat updates ephemeral state;
- disconnect clears or expires presence;
- public participants receive policy-filtered presence labels;
- operator presence snapshots continue to use the existing availability boundary.

## Replay And Recovery Tests

Coverage:

- replay after global cursor returns missed persisted events;
- conversation replay after sequence returns ordered conversation events;
- duplicate replay does not duplicate UI state when client applies event ids;
- pending optimistic messages reconcile by `clientId`;
- failed sends remain retryable;
- reconnect after message persisted but ack lost resolves to canonical message;
- reconnect after command rejected clears pending local state with reason.

## LLM Gateway Tests

Provider abstraction:

- adapter emits normalized start, delta, usage, completion, and failure events;
- Anthropic-style SSE maps to Ordo stream events;
- OpenAI-compatible non-streaming responses map to Ordo completion, usage, and
  failure events through a no-network mocked transport in default tests;
- DeepSeek/OpenAI-compatible SSE maps to Ordo stream events;
- cancellation stops downstream work and emits canonical cancel state.
- OpenAI-compatible request building receives privacy-transformed prompt/user
  payloads, records usage through the token ledger, and omits raw secrets from
  debug/error output, events, and eval artifacts.

Prompt builder:

- slots are included only when policy allows;
- slot token estimates are recorded;
- truncation is deterministic and recorded;
- raw provider prompts are not persisted unless an explicit debug mode is later
  designed and approved.
- the `ethical_business_persuasion` slot records source refs, inclusion reason,
  visibility ceiling, policy decision, transform run, slot version, content
  hash, and token estimates through `llm_prompt_slot_usage`;
- persuasion guidance cannot invent scarcity, social proof, authority,
  relationship context, urgency, or evidence.
- staff-facing persuasion output exposes reasoning and evidence, while
  client-facing output hides internal prompt mechanics and remains
  agency-preserving.

Tool mediation:

- model tool requests map to catalog capabilities;
- unknown tools are rejected;
- review-required tools pause stream until owner approval;
- tool results are returned through the gateway, not directly from client to
  provider.

## Privacy Egress Tests

Coverage:

- detector finds fixture emails, phone numbers, API keys, and configured private
  names;
- transformed provider payload contains placeholders, not raw values;
- placeholder mappings are encrypted or stored through the vault boundary;
- reconstruction only replaces known placeholders;
- placeholder from another invocation or scope is not reconstructed;
- realtime event payloads, diagnostic logs, token ledger rows, and policy audit
  rows do not contain raw sensitive fixtures.

Regression fixtures should include realistic conversation text with names,
emails, phone numbers, client details, API-key-like strings, and false positives.

## Token Ledger Tests

Coverage:

- invocation row created for every provider call;
- prompt slot rows created for every compiled slot;
- estimated slot tokens sum to estimated prompt total;
- provider usage creates append-only ledger entries;
- costs use the pricing snapshot active at invocation time;
- rollups by conversation, provider, model, capability, and slot match ledger
  entries;
- failed or cancelled calls record appropriate partial usage when provider
  supplies it.

## Analysis Tests

Coverage:

- analysis queues after durable message creation;
- analysis does not run for blocked or non-visible messages;
- episode candidates are evidence-backed and idempotent;
- tag candidates can be confirmed, rejected, or superseded;
- rolling summary updates without losing previous summary evidence;
- action-needed detection updates unread action count;
- knowledge graph node candidates require evidence refs and provenance;
- knowledge graph edge candidates require source/target node candidates,
  evidence refs, and provenance;
- knowledge graph candidate extraction is idempotent for the same analysis job;
- knowledge graph lifecycle transitions are durable for confirmed, rejected,
  and superseded states;
- knowledge graph candidates reference source message and event ids without
  becoming automatic business truth;
- offer/ask/referral/outcome attribution candidates reference source entry
  points, conversations, artifacts, and events where available;
- outcomes require evidence refs and provenance;
- attribution records require outcome id, source kind/id, influence role,
  evidence refs, provenance, and proposed candidate state;
- public offer acceptance records an offer outcome and only attributes sources
  that have concrete offer, visitor-session, or entry-point ids;
- attribution lifecycle transitions are durable for confirmed, rejected, and
  superseded states;
- artifacts require evidence refs, provenance, content hash, and visibility
  ceiling;
- artifact links require concrete source ids and do not invent influence;
- deliverables project from artifacts with client-safe label and summary;
- artifact detail briefs answer value, use, next action, producing job,
  provenance, and storage/health where available;
- surface brief records require evidence refs and limitations;
- deterministic surface brief jobs create completed briefs linked to generated
  artifacts;
- latest completed surface brief remains readable while a newer refresh is
  queued, running, or failed;
- newer completed surface briefs supersede older completed briefs;
- brief candidates cite durable conversation evidence;
- ethical recommendation candidates cite durable evidence and limitations;
- provider-backed analysis uses the same privacy and token ledger path.

## Frontend Unit Tests

Coverage:

- message reducer applies durable events idempotently;
- ephemeral typing state expires;
- optimistic send reconciles by `clientId`;
- failed send presents retry state;
- read/unread divider moves correctly;
- receipt display collapses to latest read point;
- streaming assistant bubble converts to durable final message;
- connection recovery applies replay without duplicating messages.
- client role sees one relationship conversation and no internal queue state;
- staff role defaults to `My Handoffs`;
- admin role can navigate to appliance internals;
- recommendation explanations are visible only in staff/admin context.
- product surfaces render latest completed brief and refresh state before raw
  surface detail.

## UI Smoke Tests

Use the existing lightweight mock daemon pattern in `tests/ui/`.

Desktop scenarios:

- verify top rail, staff rail, admin rail, evidence list, and narrative brief
  behavior by role;
- open conversation list and timeline;
- send message with optimistic pending state;
- receive canonical message event;
- show typing indicator and expiry;
- mark conversation read;
- mark unread from a message;
- receive assistant streaming events and final message;
- disconnect and recover with replay.

Mobile scenarios:

- navigate menu -> evidence list -> selected detail brief;
- composer remains usable with keyboard viewport;
- message text and buttons do not overflow;
- unread divider and jump-to-latest controls remain reachable;
- reaction/menu controls use touch-safe targets;
- typing and receipt rows do not shift layout incoherently.

Visual assertions:

- no overlapping text;
- stable timeline dimensions during streaming;
- no layout jump when typing indicators appear/disappear;
- readable badges and receipts in light/dark or current theme.

## Performance Tests

Local performance targets should be measured before being written as hard SLOs.
Initial test scenarios:

- thousands of persisted messages in one conversation;
- many idle WebSocket clients;
- burst of typing events with rate limiting;
- assistant stream with high-frequency deltas coalesced by UI;
- replay window at maximum limit;
- unread rollup recalculation for large conversation.

Assertions:

- no unbounded channel growth;
- bounded memory per connection;
- durable message insert remains transactional;
- high-volume ephemeral deltas do not create durable row explosion.

Current release evidence for #96:

- `/chat/ws` uses Tokio broadcast fanout with bounded receiver lag behavior and a
  retryable `client_lagged` recovery frame;
- inbound text frames over 64 KiB return `frame_too_large` before parsing;
- replay windows are clamped to 500 durable conversation events;
- message command rate limiting is covered at the gateway boundary;
- duplicate message retries are idempotent through `clientMessageId`.

Deferred performance evidence:

- many-client load measurement and hard SLOs remain future release work;
- explicit heartbeat timeout eviction remains future transport hardening;
- cross-process fanout is out of scope while the daemon remains a local
  appliance.

## Security And Abuse Tests

Coverage:

- malformed frames;
- oversized frames;
- command flood;
- replay beyond allowed scope;
- participant id spoofing;
- conversation id enumeration;
- unauthorized receipt write;
- unauthorized read of private conversation;
- model prompt injection attempting to call unapproved tools;
- malicious custom event type ignored by client;
- sensitive content in logs or event payloads.

## Product Workflow Eval Coverage

0.1.4 evals should prove product lifecycle behavior through deterministic
backend workflows before live providers are involved.

Phase 1 harness coverage now includes:

- isolated in-memory SQLite eval store initialized through the current daemon
  schema and built-in capability/template seeds;
- eval case types for case id, fixture hash, actor roles, ordered scenario
  steps, expected evidence channels, assertion results, scorecard summaries,
  and placeholder artifact paths;
- deterministic clock behavior for stable scorecard timestamps;
- explicit deterministic-only provider mode and `network_enabled = false`;
- evidence snapshots for SQLite rows, conversation events, realtime replay,
  policy decisions, prompt-slot accounting, privacy transforms, token ledger,
  analysis candidates, handoff state, artifacts, and surface briefs;
- tests proving missing optional evidence channels are represented as zero
  counts, not ignored;
- tests proving repeated harness runs are stable apart from durable ids owned by
  the underlying domain services.

Phase 2 artifact packet coverage now includes:

- JSON packet, JSON scorecard, and manifest writing under a caller-provided
  output directory;
- packet schema version, case metadata, actor roles, ordered steps, evidence
  snapshots, assertion results, and scorecard summary;
- transcript, timeline, conversation event, realtime replay, policy, prompt
  slot, privacy transform, token ledger, analysis candidate, handoff, artifact,
  and surface brief ledgers;
- empty optional ledgers represented as explicit arrays/counts;
- redaction of obvious emails, phone numbers, API-key-shaped values, bearer-like
  tokens, and configured private terms before artifact serialization;
- deterministic normalized packet generation for repeated runs;
- no provider keys, browser timing, or network access.

Phase 3 first deterministic workflow coverage now includes:

- `relationship_conversation_message`, which creates the canonical visitor
  relationship conversation, creates the visitor participant, submits a
  sensitive fixture message through the backend conversation service, and
  asserts conversation-event plus realtime-replay evidence;
- `privacy_gateway_roundtrip`, which runs the Rust-owned LLM gateway against
  the deterministic local provider and asserts policy decisions, prompt-slot
  accounting, privacy transform metadata, token ledger entries, durable
  conversation events, and realtime replay evidence;
- packet, scorecard, and manifest writing for both workflow cases through the
  same `EvalArtifactWriter` used by Phase 2;
- redaction evidence for the relationship transcript packet and non-leakage
  evidence for provider-bound sensitive fixture content in the privacy gateway
  packet;
- provider-free, network-free execution suitable for local and CI runs.

Phase 3A role lifecycle workflow coverage now includes:

- `role_lifecycle_anonymous_to_client`, which validates anonymous visitor
  relationship conversation creation, authenticated client/member relationship
  conversation continuity, affiliate denial for unrelated customer conversation
  access, and absence of handoff/prompt/privacy/token internals in the
  client-facing lifecycle packet;
- `role_lifecycle_staff_manager_owner_boundaries`, which creates a governed
  handoff and asserts staff `My Handoffs`, manager/admin `Team Queue`,
  owner/system admin `All Conversations`, ordinary staff denial for all
  conversations, and protected-route policy evidence for system internals;
- `role_lifecycle_agent_silence_boundary`, which sets human-led active mode and
  asserts Ordo remains publicly silent without tag, delegation, or
  policy-required intervention;
- packet, scorecard, and manifest writing for all role lifecycle cases through
  `EvalArtifactWriter`;
- provider-free, network-free role boundary coverage with explicit policy,
  handoff, conversation event, and realtime replay evidence where current
  backend helpers support it.

Phase 3B Customer Feedback and Review workflow coverage now includes:

- `feedback_capture_private_business_intelligence`, which captures private
  feedback from durable conversation/message evidence, stars it as staff
  intelligence rather than a rating, proposes a feedback tag candidate, and
  confirms no public review/testimonial is created;
- `review_candidate_consent_publication_boundary`, which creates a review
  candidate from private feedback, verifies publication fails closed before
  consent and approval, transitions through requested, received, consent
  confirmed, approved, published, featured, and retired states, and confirms
  public visibility is removed after retirement;
- schema/domain tests for feedback evidence/provenance requirements and review
  consent/approval publication guards;
- packet, scorecard, and manifest writing for feedback/review cases, including
  feedback and review ledger sections and redaction of sensitive source-message
  fixture text.

Phase 3C Home/About and Offer/Ask product surface workflow coverage now
includes:

- `home_about_public_narrative_brief`, which seeds public Home/About billboard
  facts, draft/private excluded facts, a surface brief, and a linked artifact,
  then verifies the public contract includes evidence refs, allowed state,
  reduced-motion fallback, and Chat/source links;
- `offer_ask_machine_readable_intent`, which seeds public Offer and Ask intent
  facts, verifies human-readable and machine-readable intent metadata, preserves
  the future-A2A boundary, and rejects unsupported public persuasion proof such
  as fake scarcity;
- packet, scorecard, and manifest writing for both cases, including a
  product-surface ledger for durable business facts, offers, and outcomes;
- provider-free, network-free validation that uses public business facts as the
  first durable surface contract instead of adding dedicated billboard/intent
  tables before eval evidence requires them.

Phase 3D handoff, mode, and delegation gateway command coverage now includes:

- gateway command tests for `conversation.handoff.create` and the durable
  handoff lifecycle transitions accepted, assigned, returned to agent, and
  closed, including ack client-id preservation and replay order;
- support for current short handoff aliases such as `handoff.assign` while the
  explicit `conversation.handoff.*` command names become the preferred product
  contract;
- gateway command tests for `conversation.mode.human_led_active`,
  `conversation.mode.return_to_agent`, `conversation.agent.delegate`, and
  `conversation.agent.delegation_revoke`;
- scoped delegation validation that rejects delegation without an explicit
  `delegationScope`;
- non-leakage assertions that mode/delegation dispatch payloads do not expose
  provider, privacy-transform, or policy-decision internals;
- continued provider-free and network-free execution, with SQLite
  `conversation_events` remaining the replay source of truth.

Phase 4 replay-provider fixture coverage now includes:

- `ReplayLlmProvider` tests that load redacted fixtures behind
  `LlmProviderAdapter` and replay deterministic deltas, completions, and usage
  metadata through the normal `LlmGateway`;
- stable request fingerprint matching from provider id, model id, prompt hash,
  and transformed user-message hash;
- canonical provider failure when no approved replay fixture matches;
- fixture validation that rejects obvious raw emails, phone numbers,
  API-key-shaped strings, bearer-token-shaped strings, and configured private
  fixture terms before the fixture can be used;
- a committed tiny success fixture at
  `crates/ordo-daemon/fixtures/llm-replay/tiny-success.json`;
- `replay_provider_fixture`, which writes packet, scorecard, and manifest
  artifacts while preserving redaction and provider usage evidence;
- continued network-free execution with token ledger entries populated from
  replayed provider usage metadata.

Phase 6 opt-in live eval runner coverage now includes:

- guard parsing tests that skip or block without `ORDO_LIVE_LLM_EVALS=1`,
  `ORDO_LIVE_LLM_ALLOW_NETWORK=1`, provider model, or provider key evidence;
- max-case and budget guard tests that prevent provider construction before a
  configured overrun;
- mocked OpenAI-compatible transport tests that run the allowed path without
  provider keys or network;
- `live_openai_compatible_smoke`, which writes packet, scorecard, and manifest
  artifacts through `EvalArtifactWriter`;
- assertions that the live smoke path records policy decisions, prompt slots,
  privacy transforms, token ledger entries, conversation events, and final
  assistant candidate evidence;
- redaction assertions that raw provider secrets and private fixture values do
  not appear in packet artifacts or run summaries.

Phase 7 artifact review classifier coverage now includes:

- passing packet reviews with no failure/blocker findings;
- failed assertion classification into the smallest responsible category;
- missing token, handoff, and analysis evidence classification into
  `accounting_gap`, `handoff_gap`, and `analysis_gap`;
- provider failure evidence classification into `provider_gap` with safe
  metadata only;
- privacy blocker classification for raw email, phone, API-key-shaped, and
  private-term fixture values;
- deterministic review JSON and `artifact-review.md` output;
- proof that review generation is local, provider-free, network-free, and does
  not auto-file GitHub issues.

Phase 8 simulator contract coverage now includes:

- `ordo.eval_simulator_output.v1` schema validation for customer, operator, and
  reviewer simulator outputs;
- customer and operator outputs rejected when they attempt to emit artifact
  review findings;
- reviewer outputs restricted to known artifact-review finding categories;
- missing message hashes, redacted excerpts, deterministic assertion refs, or
  evidence/artifact refs rejected;
- unknown simulator roles and unknown authority fields such as `passed`
  rejected;
- raw email, phone, API-key-shaped, and configured private-term values rejected
  from simulator output;
- deterministic output serialization tests;
- proof that default simulator contract tests require no provider keys and no
  network.

0.1.5 live product journey eval coverage now includes:

- persona markdown parsing and validation with no real private data;
- ten committed synthetic persona fixtures for event QR, trial, review,
  referral, privacy, budget, handoff, and dissatisfaction pressure;
- deterministic persona id ordering and library content hashing;
- validator rejection for missing required fields, unknown pressure subsystems,
  duplicate ids, raw emails, phone numbers, API-key-shaped strings, bearer
  tokens, and configured private terms.
- multi-case live journey planning that loads personas deterministically;
- missing live/network guards produce skipped planning summaries without
  network or provider construction;
- explicit persona id selection rejects unknown ids and preserves requested
  order;
- max-case and budget caps are enforced before journey execution;
- run manifests include persona ids, content hashes, planned case ids, guard
  decisions, and budget summaries without raw secrets or private persona
  narrative.
- QR-to-trial journey execution with no provider keys and no network by
  default;
- tracked event QR entry point, visitor session, relationship conversation,
  visitor message, and deterministic assistant response evidence;
- daemon LLM gateway evidence for `ethical_business_persuasion`, privacy egress
  transform events, prompt-slot accounting, and token ledger rows;
- public OrdoStudio offer acceptance, started 30-day trial, business outcome,
  and offer/session/entry-point attribution evidence;
- packet, scorecard, harness manifest, and QR-to-trial journey manifest output
  with raw provider secrets, raw persona narrative, emails, phone numbers,
  configured private terms, fake urgency, fake scarcity, and unsupported social
  proof absent.
- review-return journey execution with no provider keys, no network, and no
  real outbound email by default;
- QR-to-trial setup evidence reused as the source for review-return;
- simulated review-request email/link artifact that is durable, redacted,
  non-delivered, and marked as simulated;
- return entry point, return visitor session, relationship conversation
  continuity, return visitor message, and deterministic assistant response
  evidence;
- private feedback captured from durable conversation/message evidence and a
  review candidate created from that feedback;
- blocked review publication before consent and approval, followed by durable
  requested, received, consent-confirmed, approved, published, featured, and
  retired lifecycle transitions;
- public review visibility appears after publish/feature and is removed after
  retire;
- review-return packet, scorecard, harness manifest, QR setup, and journey
  manifest output with raw provider secrets, raw persona narrative, emails,
  phone numbers, configured private terms, fake reviews, fake metrics, fake
  urgency, fake scarcity, and unsupported social proof absent.
- affiliate-referral journey execution with no provider keys and no network by
  default;
- affiliate/referrer persona selection from the committed persona library;
- active affiliate connection and scoped connection grant evidence;
- referral entry point, referred visitor session, relationship conversation,
  referred visitor message, and deterministic assistant response evidence;
- public OrdoStudio offer acceptance and started 30-day trial for the referred
  visitor;
- referral record and referral-linked business outcome citing concrete
  affiliate, entry point, visitor session, offer acceptance, trial, and
  conversation/message evidence;
- referral and affiliate-connection attribution proposed only after concrete
  referral/source ids exist;
- affiliate visibility checks that allow the scoped referred conversation and
  deny unrelated conversation access;
- affiliate-referral packet, scorecard, harness manifest, and journey manifest
  output with raw provider secrets, raw persona narrative, emails, phone
  numbers, configured private terms, fake referrals, fake metrics, fake
  urgency, fake scarcity, fake reviews, and unsupported social proof absent.
- admin/staff handoff and moderation journey execution with no provider keys
  and no network by default;
- governed handoff creation from durable conversation/message evidence;
- staff `My Handoffs` and manager `Team Queue` evidence;
- durable handoff transitions through accepted, assigned, in progress,
  returned-to-agent, and closed;
- human-led active mode blocking untagged public agent posts, scoped delegation
  allowing agent posting, and returned-to-agent mode allowing agent resumption;
- review moderation that keeps reviews private before consent and approval and
  publishes only after approval evidence exists;
- affiliate management evidence that creates a scoped grant, revokes it, and
  denies access after revocation;
- admin/staff packet, scorecard, harness manifest, and journey manifest output
  with staff/admin internals, policy/provider mechanics, raw provider secrets,
  raw persona narrative, emails, phone numbers, and configured private terms
  absent from client/public-safe artifacts.
- cross-persona live journey report generation with no provider keys, no
  network, and no GitHub calls by default;
- report input readers for QR-to-trial, review-return, affiliate-referral, and
  admin/staff journey manifests plus packet and artifact-review JSON;
- deterministic JSON and Markdown report output under fixed source commit and
  generated timestamp;
- aggregate conversion/trial, review, referral, handoff/moderation,
  privacy/redaction, prompt-slot, token/accounting, persuasion-boundary, and
  artifact-review finding summaries;
- explicit missing/not-run gaps when a journey type is absent;
- local follow-up issue drafts generated only from missing artifact evidence or
  failure/blocker artifact-review findings, with no automatic filing;
- report output with raw provider secrets, raw persona narrative, emails, phone
  numbers, staff-only internals, and configured private terms absent.
- governed email path decision keeps review-request email simulated for 0.1.5;
- simulated email artifacts remain durable, redacted, non-delivered, and
  evidence-backed;
- reports treat simulated email as local eval evidence, not real delivery;
- real outbound email requires future tests for owner approval, recipient
  consent or lawful basis, suppression/unsubscribe, deliverability controls,
  provider secret handling, audit trail, rate/spend caps, redaction, no raw
  fixture emails, and explicit opt-in live/email guards before any delivery
  adapter can be accepted.

0.1.5 live product journey eval coverage is complete. Future milestones may
add real outbound email only after the governed delivery gates above are
accepted and validated.

Role lifecycle:

- anonymous visitor starts from Home/About, Offer, Ask, Latest, QR/link entry,
  or Chat and receives a visitor-session-backed relationship conversation;
- authenticated client/member sees one relationship conversation and client-safe
  account tools;
- affiliate sees own referral/account tools and cannot see unrelated customer
  conversations;
- business staff defaults to `My Handoffs`;
- manager/admin can access `Team Queue` and authorized `All Conversations`;
- owner/system admin can inspect system surfaces while ordinary staff cannot see
  Logs, Backup, readiness, policy internals, prompt internals, token/cost
  internals, or privacy placeholder maps by default;
- Ordo does not post publicly during human-led active mode unless tagged,
  delegated, or policy requires intervention.

Customer Feedback and Reviews:

- feedback records require durable source evidence;
- feedback tags default to proposed and can be confirmed/rejected/superseded;
- starred feedback affects feedback briefs but is not treated as a customer
  rating;
- review candidates cannot publish without consent and approval;
- published/featured/retired review states are durable and reversible;
- feedback and reviews can link to conversation, segment, message, connection,
  offer, ask, artifact, referral, outcome, and brief where ids exist.

Home/About and Offer/Ask surfaces:

- Home/About billboard claims require linked evidence or explicit aspirational
  labeling;
- generated billboards support pinned, dynamic, draft, published, and retired
  states;
- reduced-motion fallback preserves billboard content and actions;
- fake scarcity, fake reviews, fake metrics, unsupported authority, and
  unsupported social proof are rejected;
- Offers and Asks expose human-readable pages and machine-readable intent
  metadata without implementing external A2A;
- offer/ask outcome and feedback briefs cite referrals, conversations,
  artifacts, entry points, and outcomes only when source ids exist.

Artifact packet review:

- eval packets include redacted transcript, timeline, event ledger, DB ledger,
  prompt-slot ledger, privacy ledger, token ledger, analysis candidates, handoff
  ledger, replay check, scorecard, and artifact review when the corresponding
  data exists;
- finding categories include `schema_gap`, `event_gap`, `policy_gap`,
  `privacy_gap`, `prompt_gap`, `handoff_gap`, `analysis_gap`,
  `accounting_gap`, `ux_contract_gap`, and `provider_gap`.

## Manual Acceptance Checklist

- Message send feels instant.
- Reconnect feels safe and recovers missed messages.
- Typing indicator appears and disappears naturally.
- Read/unread counts match user expectation.
- AI activity explains what Ordo is doing without leaking internals.
- Provider calls are visible in token ledger.
- Sensitive fixtures do not leave the daemon untransformed.
- Conversation summaries and brief candidates cite durable evidence.
- Handoff briefs cite durable messages/events before transcripts.
- Ethical persuasion guidance stays evidence-backed and agency-preserving.
- Staff-only ethical persuasion guidance renders evidence/source refs, and
  client/public chat omits the internal prompt slot panel.
- UI remains polished on desktop and mobile.
