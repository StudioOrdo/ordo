# Interactive Account And LLM Chat

Status: 0.1.8 implementation contract

Issue #214 is the contract-setting phase for the 0.1.8 arc. It replaces the
stale Product Onboarding Surfaces direction as the active implementation plan.
Issues #215 through #221 carry the executable slices.

This arc connects three foundations that already exist but are not yet wired
into one interactive product path:

- local account entry through Login and Register surfaces;
- daemon-owned conversation identity, replay, and `/chat/ws` transport;
- daemon-owned LLM invocation with deterministic default behavior and guarded
  OpenAI-compatible live testing.

The goal is a local-first appliance loop, not hosted auth, production
multi-user identity, broad provider orchestration, external delivery, payments,
or a public chatbot. SQLite remains the source of truth. WebSocket remains a
live command and projection path. External model calls remain daemon-mediated
through policy, prompt slots, privacy egress, provider boundaries, and token
accounting.

## Why This Replaces Product Onboarding Surfaces

The previous Product Onboarding Surfaces issues (#205 through #213) were closed
as stale after the May 12, 2026 direction reset. That arc tried to turn journey
eval evidence into public, offer, referral, review-return, and staff/admin
surfaces. The current need is narrower and more enabling: make the existing
relationship conversation real enough for interactive local testing.

The 0.1.8 arc should prove the smallest useful product loop:

```text
local account session -> chat bootstrap -> browser /chat/ws -> durable user
message -> daemon LLM request -> assistant message -> replay/smoke evidence
```

Once this loop works, future public onboarding, trial, referral, review-return,
and staff/admin surfaces can depend on a real chat/session substrate instead of
mock conversation state.

## Current Code Evidence

Current source shows the integration gap clearly:

- `app/login/page.tsx` and `app/register/page.tsx` render placeholder forms and
  navigate with links into `/my/chat?role=client`; they do not create or read a
  local session.
- `docs/architecture/local-install-and-providers.md` documents install/provider
  state and explicitly says hosted identity, login UI, password reset, OAuth,
  and frontend install wizard behavior are non-goals of that completed slice.
- `crates/ordo-daemon/src/policy.rs` defines seeded local owner/system actors
  and `ActorContext`, while `crates/ordo-daemon/src/schema/migrations.rs` seeds
  the durable access baseline.
- `components/conversation-foundation.tsx` renders a strong conversation
  surface, but `useGatewayConversation` currently simulates send, failure,
  replay, read state, and assistant recovery locally from fixture data.
- `lib/conversation-protocol.ts` already defines `conversation.gateway.v1`,
  `/chat/ws`, message commands, and LLM/tool command names such as
  `llm.run.request`, `llm.run.cancel`, `tool.approve`, `tool.reject`, and
  `tool.execute`.
- `lib/ordoos-realtime.ts` contains reducer-style realtime primitives, but its
  command kind list currently focuses on message and conversation commands, not
  LLM run commands.
- `crates/ordo-daemon/src/server/mod.rs` exposes `/chat/ws` beside `/ws`.
- `crates/ordo-daemon/src/conversation_gateway/handlers.rs` implements
  identify, subscribe, replay, message lifecycle, receipts, reactions,
  presence, handoffs, modes, and delegation. Its command router does not yet
  handle `llm.run.request`.
- `crates/ordo-daemon/src/llm_gateway/core.rs` already owns `LlmGateway`,
  `run_completion`, policy decisions, prompt slot accounting, privacy egress,
  provider dispatch, assistant-message creation, token ledger recording, and
  LLM run events.
- `crates/ordo-daemon/src/llm_gateway/types.rs` already provides
  `DeterministicLlmProvider`, `OpenAiCompatibleProvider`,
  `OpenAiCompatibleConfig`, and safe provider error normalization.
- `docs/developer-guide.md` documents guarded live LLM evals and the default
  deterministic, network-free validation posture.

## Phase Boundaries

### #215 Phase 1: Local Account Session Scaffold

Add the smallest appliance-local login/register session path needed for a
browser to enter the member chat surface. The session should bind to the local
appliance context without claiming hosted auth.

Required boundaries:

- local-first session read model;
- safe form submission from Login and Register;
- no raw passwords or secret-like values in logs, events, reports, or policy
  metadata;
- no OAuth, hosted identity, email verification, password reset, or multi-tenant
  RBAC claims.

### #216 Phase 2: Daemon Chat Bootstrap

Add a protected/local bootstrap path that gives the frontend the identities it
needs to use `/chat/ws` safely:

- `actorId`;
- `conversationId`;
- `participantId`;
- `assistantParticipantId`;
- websocket URL or enough route metadata for the frontend to derive it.

The bootstrap service should be idempotent for the same local actor and should
create missing conversation participants without duplicating the canonical
relationship conversation.

### #217 Phase 3: Browser `/chat/ws` Adapter

Replace the member chat send path with a small browser websocket adapter over
the existing `conversation.gateway.v1` envelope:

- connect;
- identify;
- subscribe;
- submit message;
- reconcile ack and dispatch frames;
- replay after cursor;
- surface recoverable errors and degraded daemon state.

The transport adapter, frame mapping, and React presentation should stay
separate so later LLM states do not tangle the component tree with protocol
mechanics.

### #218 Phase 4: Deterministic `llm.run.request`

Bridge `llm.run.request` through the conversation gateway using deterministic
provider mode by default. The handler should call the daemon-owned LLM gateway
instead of adding frontend/provider logic.

The default path must persist and broadcast evidence for:

- LLM run requested;
- prompt compiled;
- prompt slots included/accounted;
- privacy egress transformed or blocked;
- provider started;
- text delta/completion where available;
- usage recorded;
- run completed or failed;
- final assistant message when completion succeeds.

### #219 Phase 5: Guarded OpenAI-Compatible Chat Mode

Allow interactive local chat to use the daemon-owned local provider path by
default while cloud providers remain behind explicit live-provider guards. The
deterministic provider remains available as an explicit fixture path for CI and
evals.

Live chat mode must fail closed when network, model, provider, budget, timeout,
or API-key requirements are missing. Provider keys remain write-only and must
not appear in UI, HTTP responses, WebSocket frames, durable events, diagnostic
logs, reports, or test artifacts.

Current implementation status: member chat submits through `/chat/ws` with a
provider/model selector. The browser sends `providerId: "local"` for Local
Ollama and never calls providers directly. The conversation gateway allows the
local Ollama adapter without cloud live-provider guards, while non-local
providers still require the safe `ORDO_APP_LIVE_LLM`, network, budget, catalog,
model, and credential checks before provider dispatch.

Live-provider mode remains disabled until a later guarded validation slice adds
all of the following as explicit preconditions:

- configured provider and model allowlist;
- write-only API key presence through the provider boundary;
- budget and timeout limits;
- network/live-call opt-in outside CI defaults;
- safe failure evidence that never exposes provider keys, raw prompts, prompt
  slot content, policy internals, staff/system-only details, or credentials.

### #220 Phase 6: LLM Run States In Chat UI

Expose client-safe LLM run states in the member chat surface:

- ready/degraded;
- request queued or pending;
- provider/request in progress;
- completed assistant reply;
- failed run with safe retry/degraded copy;
- replayed/recovered state.

Member surfaces must not expose raw prompts, provider keys, policy internals,
token ledgers, privacy placeholder maps, or staff-only reasoning. Staff/admin
evidence can be richer only where existing role-safe projections authorize it.

### #221 Phase 7: End-To-End Smoke Evidence

Prove the full deterministic path without default external network calls:

```text
login/register -> local session -> chat bootstrap -> /chat/ws identify and
subscribe -> message.submit -> llm.run.request -> deterministic assistant
message -> replay-safe UI
```

The smoke plan should include daemon-unavailable and reconnect/replay cases,
plus manual instructions for guarded live-provider proof.

Repeatable local evidence for the real daemon path lives in:

```bash
npm run smoke:chat:real
```

that command starts the Rust daemon with disposable SQLite state, starts Next
pointed at the daemon, registers a local member session through the browser
API, opens the member chat, sends one message through `/chat/ws`, and verifies
the daemon-backed assistant path without exposing raw credentials, prompt
content, provider keys, or internal policy details.

### #228-#231 Phase 8: Provider-Agnostic Readiness

Add provider-specific readiness slices while keeping member chat deterministic
by default and live calls disabled until a later explicit guarded validation
path exists. This phase is about the daemon's provider boundary, not browser
provider selection.

2026 provider research baseline:

- OpenAI's public API reference now lives under `developers.openai.com` and
  emphasizes server-side Bearer API keys, request IDs, rate-limit headers,
  backwards-compatible REST evolution, and Responses-era model behavior. Our
  existing OpenAI-compatible adapter remains useful, but GPT-5-class behavior
  needs readiness metadata for endpoint family, reasoning settings, timeouts,
  and budget before any member-chat live path is allowed.
- Anthropic's current primary API is the native Messages API at
  `POST /v1/messages`, with token counting at `POST /v1/messages/count_tokens`.
  Its content blocks, tools, thinking, usage, stop reasons, and stream events
  are not OpenAI-compatible by default, so Anthropic needs a native adapter
  contract rather than being forced through the OpenAI-compatible path.
- DeepSeek documents both OpenAI-compatible and Anthropic-compatible API
  formats. The OpenAI-compatible base URL is `https://api.deepseek.com`; the
  Anthropic-compatible base URL is `https://api.deepseek.com/anthropic`. Current
  documented models include `deepseek-v4-flash` and `deepseek-v4-pro`, while
  `deepseek-chat` and `deepseek-reasoner` are documented as deprecated after
  2026-07-24.
- Ollama's local API is served by default at `http://localhost:11434/api`, with
  `POST /api/chat`, `POST /api/generate`, `GET /api/tags`, `GET /api/ps`, and
  `GET /api/version`. It supports streaming by default and `stream:false` for a
  single response, plus local timing/token-ish metadata. Default CI must not
  require Ollama to be installed.
- Provider-agnostic gateway patterns such as LiteLLM normalize many providers
  into an OpenAI-like shape, but Ordo should keep its daemon-owned policy,
  prompt, privacy, accounting, and vault boundaries rather than outsourcing the
  trust boundary to a proxy by default.

Phase 8 provider issues:

- #228 OpenAI provider readiness resolver: resolve configured provider/model,
  endpoint family, key source, timeout, budget, and live-call opt-in without
  making default network calls.
- #229 Anthropic Messages provider readiness resolver: safely inspect config and
  report native-adapter readiness or `unsupported_adapter` until a Messages
  adapter exists.
- #230 DeepSeek provider readiness resolver: normalize OpenAI-compatible versus
  Anthropic-compatible base URLs, reject deprecated models, and prove endpoint
  shape with mocked transport only.
- #231 Local Ollama provider readiness resolver: `local` now maps to the
  daemon-owned Ollama adapter at the local `/api/chat` endpoint, with
  `local_fake/fake-chat` reserved for explicit deterministic fixtures. Next
  depth is safe localhost readiness probing and mocked adapter normalization.

Readiness decisions should be structured and safe, for example `disabled`,
`missing_key`, `missing_model`, `missing_budget`, `missing_timeout`,
`live_network_disabled`, `unsupported_adapter`, `provider_unreachable`, or
`ready_but_live_disabled`. These decisions may be exposed to owner/admin
surfaces later, but member chat must continue to show only safe run states and
must never expose provider keys, raw prompts, prompt slot content, policy
internals, staff/system-only details, or credentials.

## Validation Contract

Default validation remains deterministic, local, and network-free.

Positive cases:

- registration/login creates or restores a local appliance session;
- chat bootstrap returns stable conversation and participant identities;
- websocket identify, subscribe, message submit, ack, dispatch, and replay work
  from the browser;
- deterministic `llm.run.request` records policy, prompt, privacy, provider,
  usage, completion/failure, and assistant-message evidence;
- the member chat UI renders a safe assistant reply and recovery state.

Negative cases:

- invalid account input is rejected without creating misleading session state;
- protected daemon routes reject unauthorized non-loopback requests without a
  valid daemon token;
- unsupported or malformed websocket frames return structured `command.rejected`
  errors;
- live provider mode is blocked when required guards, model, budget, timeout, or
  API key are missing;
- provider errors do not leak raw keys, prompts, private fixture values, or
  internal policy details to member surfaces.

Edge cases:

- repeated registration/login and repeated bootstrap calls are idempotent where
  the contract says they should be;
- missing assistant participant can be recreated without duplicating the
  conversation;
- reconnect and replay do not duplicate optimistic or durable messages;
- rapid double submit or double LLM trigger is blocked or deduplicated;
- privacy egress failure records safe failure evidence and does not create a
  final assistant message.

## Design Constraints

- Use small modules with single responsibilities.
- Treat the session, bootstrap, websocket transport, frame mapper, and visual UI
  state as separate contracts.
- Use the existing provider adapter strategy instead of introducing provider
  branching into UI code.
- Keep SQLite as the durable source of truth.
- Keep WebSocket as live command/projection transport, not the record.
- Keep policy, privacy egress, prompt slots, provider dispatch, and token
  accounting in the Rust daemon.
- Prefer deterministic tests before guarded live-provider checks.
- Update public docs when behavior changes trust boundaries, runtime setup, or
  validation commands.

## Explicit Non-Goals

- Hosted identity.
- OAuth or email login.
- Password reset.
- Production multi-user authentication.
- Broad RBAC redesign.
- Public chatbot behavior.
- Arbitrary tool execution from chat.
- Broad provider orchestration beyond the existing OpenAI-compatible adapter.
- Real outbound email.
- Payments or billing automation.
- External delivery or integration transports.
- Default CI live-provider calls.

## Closeout Evidence For Each Phase

Each implementation issue should close with:

- files changed;
- tests and commands run;
- policy, privacy, provider, and UI evidence for changed boundaries;
- screenshots or browser traces for visible frontend behavior;
- explicit remaining risks;
- a note that default validation stayed deterministic and network-free unless a
  manually guarded live-provider proof was intentionally run.