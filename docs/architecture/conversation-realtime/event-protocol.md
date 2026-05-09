# Conversation Event Protocol

Status: `conversation.gateway.v1` protocol types and the protected `/chat/ws`
transport are implemented for the first durable gateway slice.

The conversation protocol should evolve Ordo from a simple outbound WebSocket
projection into a bidirectional, resumable conversation gateway while preserving
the existing durable event rule: SQLite is truth, WebSocket is projection.

## Current Protocol Baseline

The daemon keeps two realtime paths. The current `/ws` path sends one
`websocket.connected` system event,
then forwards events received from a Tokio broadcast channel. The browser status
component parses received JSON and displays the latest `eventType`.

Conversation clients use `/chat/ws`, a separate protected local daemon route.
This keeps system-shell realtime consumers on the simple stream while chat
clients use the richer bidirectional protocol.

## Envelope

Every bidirectional frame should use a versioned envelope:

```json
{
  "schemaVersion": "conversation.gateway.v1",
  "op": "dispatch",
  "type": "message.created",
  "clientId": "client_generated_correlation_id",
  "serverId": "daemon_generated_event_id",
  "conversationId": "conv_...",
  "segmentId": "seg_...",
  "sequence": 42,
  "cursor": 1884,
  "durability": "durable",
  "scope": "conversation",
  "payload": {},
  "occurredAt": "2026-05-09T00:00:00Z"
}
```

Fields:

| Field | Meaning |
| --- | --- |
| `schemaVersion` | Protocol schema version. |
| `op` | Transport operation, such as hello, command, dispatch, ack, error, heartbeat, resume, or replay. |
| `type` | Event or command type. |
| `clientId` | Client-generated correlation id for optimistic reconciliation and retries. |
| `serverId` | Daemon-generated durable or ephemeral event id. |
| `conversationId` | Conversation scope. |
| `segmentId` | Optional session, handoff, or LLM run segment scope. |
| `sequence` | Per-conversation monotonically increasing sequence for ordered UI application. |
| `cursor` | Global durable realtime cursor when persisted into `realtime_events`. |
| `durability` | `durable`, `ephemeral`, or `read_model`. |
| `scope` | `connection`, `user`, `conversation`, `system`, or `run`. |
| `payload` | Command or event data. |
| `occurredAt` | Daemon timestamp for events; client timestamp may be included inside payload. |

## Operations

| Operation | Direction | Purpose |
| --- | --- | --- |
| `hello` | daemon to client | Establish protocol version, heartbeat interval, session id, and resume support. |
| `identify` | client to daemon | Bind browser session to actor, participant, and requested scopes. |
| `subscribe` | client to daemon | Start watching a conversation or user notification stream. |
| `unsubscribe` | client to daemon | Stop watching a conversation. |
| `command` | client to daemon | Ask the daemon to perform a state change. |
| `dispatch` | daemon to client | Deliver a durable, ephemeral, or read-model event. |
| `ack` | daemon to client | Confirm command acceptance or rejection by `clientId`. |
| `heartbeat` | both directions | Keep connection alive and measure latency. |
| `resume` | client to daemon | Resume from last durable cursor and ephemeral subscription state. |
| `replay` | daemon to client | Deliver missed durable events after cursor. |
| `error` | daemon to client | Report command, protocol, auth, policy, or rate-limit errors. |

## Client Commands

Commands are not events. A command requests work; the daemon decides whether it
is allowed and emits canonical events after persistence or ephemeral fanout.

Initial command catalog:

| Command | Durability outcome | Capability |
| --- | --- | --- |
| `conversation.subscribe` | ephemeral subscription plus optional replay | `conversation.read` |
| `conversation.replay_after_cursor` | replay events | `conversation.read` |
| `message.submit` | `message.created` | `conversation.message.create` |
| `message.edit` | `message.edited` | `conversation.message.edit` |
| `message.delete` | `message.tombstoned` | `conversation.message.delete` |
| `message.undo` | `message.undo.cancelled` | `conversation.message.delete` |
| `message.react` | `message.reaction.added` or `message.reaction.removed` | `conversation.reaction.write` |
| `message.mark_read` | `message.read` and unread rollup events | `conversation.receipt.write` |
| `message.mark_unread` | `message.marked_unread` and unread rollup events | `conversation.receipt.write` |
| `typing.start` | ephemeral `typing.started` | `conversation.presence.write` |
| `typing.stop` | ephemeral `typing.stopped` | `conversation.presence.write` |
| `presence.update` | ephemeral and optional durable presence snapshot | `conversation.presence.write` |
| `llm.run.request` | durable `llm.run.requested`, prompt/provider events, ephemeral deltas, durable completion/failure | `llm.invoke` |
| `llm.run.cancel` | durable `llm.run.cancelled` | `llm.cancel` |
| `tool.approve` | durable approval event | `llm.tool.approve` |
| `tool.reject` | durable rejection event | `llm.tool.reject` |
| `tool.execute` | durable executing/completed or failed event after approval | `llm.tool.execute` |
| `handoff.accept` | `handoff.item.accepted` | `conversation.handoff.manage` |
| `handoff.decline` | `handoff.item.declined` | `conversation.handoff.manage` |
| `handoff.assign` | `handoff.item.assigned` | `conversation.handoff.manage` |
| `handoff.return_to_agent` | `handoff.item.returned_to_agent` | `conversation.handoff.manage` |
| `agent.delegate` | `agent.delegation.changed` | `conversation.agent.delegate` |
| `agent.takeover` | `conversation.mode.changed` | `conversation.agent.delegate` |

All commands should include `clientId`. Mutating commands should be idempotent
for a bounded retry window using `clientId`, actor id, and conversation id.
The first gateway implementation accepts `identify`, `subscribe`,
`unsubscribe`, `resume`/`replay`, `heartbeat`, `message.submit`,
`message.edit`, `message.delete`, `message.undo`, `message.mark_read`,
`message.mark_unread`, `message.react`, `presence.update`, `typing.start`, and
`typing.stop`. Unsupported commands return structured `command.rejected`
errors instead of pretending success.

## Durable Event Catalog

Durable events should persist to domain tables and mirror into
`realtime_events` for global replay when relevant.

Conversation lifecycle:

- `conversation.created`
- `conversation.opened`
- `conversation.paused`
- `conversation.resumed`
- `conversation.closed`
- `conversation.archived`
- `conversation.segment.started`
- `conversation.segment.ended`
- `conversation.episode.created`
- `conversation.episode.updated`
- `conversation.tags.updated`
- `conversation.mode.changed`
- `conversation.summary.updated`

Participants:

- `participant.joined`
- `participant.left`
- `participant.role.changed`
- `participant.grant.added`
- `participant.grant.revoked`
- `participant.muted`
- `participant.unmuted`

Messages:

- `message.created`
- `message.persisted`
- `message.edited`
- `message.deleted`
- `message.tombstoned`
- `message.pinned`
- `message.unpinned`
- `message.attachment.added`
- `message.attachment.removed`

Reactions:

- `message.reaction.added`
- `message.reaction.removed`
- `message.reaction.cleared`

Receipts and unread:

- `message.sent`
- `message.delivered`
- `message.displayed`
- `message.read`
- `message.marked_unread`
- `conversation.unread_count.changed`
- `conversation.mention_count.changed`
- `conversation.action_count.changed`

LLM gateway:

- `llm.run.requested`
- `llm.prompt.compiled`
- `llm.prompt.slot.included`
- `privacy.egress.transformed`
- `privacy.egress.blocked`
- `privacy.egress.reconstructed`
- `llm.provider.started`
- `llm.prompt.slot.accounted`
- `llm.ledger.entry.recorded`
- `llm.tool.requested`
- `llm.tool.approved`
- `llm.tool.rejected`
- `llm.tool.executing`
- `llm.tool.completed`
- `llm.tool.failed`
- `llm.tool.cancelled`
- `llm.text.completed`
- `llm.usage.recorded`
- `llm.run.completed`
- `llm.run.failed`
- `llm.run.cancelled`

Implemented foundation behavior: `llm.text.delta` is an ephemeral gateway
dispatch and is not persisted in `conversation_events`; `llm.text.completed`,
`llm.usage.recorded`, terminal run state, prompt compilation, prompt slot
inclusion, and provider start evidence are durable conversation events. Final
assistant text is persisted as a normal `conversation_messages` row only after
provider completion. Provider keys are not part of the command, event, prompt
slot, or UI contract.

Implemented tool governance behavior: LLM tool requests are durable
conversation events with `toolRequestId`, `runId`, requested capability, reason,
evidence refs, redacted input summary, visibility ceiling, status, and policy
decision id. Unknown capabilities and non-exported/dangerous capabilities are
rejected before request persistence. Execution requires an approved tool request
and a registered exported capability, then emits `llm.tool.executing` followed
by `llm.tool.completed` or `llm.tool.failed`.

Implemented privacy egress behavior: provider-bound user and prompt-slot content
is transformed before `llm.provider.started`. `privacy.egress.transformed`
contains transform run id, scope, detector/transform versions, payload hashes,
placeholder count, detector kinds, placeholders, and content hashes. It does not
contain raw sensitive spans or transformed prompt text. Untransformable payloads
emit `privacy.egress.blocked` and `llm.run.failed` with
`privacy_transform_failed`; the provider adapter is not invoked.

Implemented token ledger behavior: every allowed LLM run creates an
`llm_invocations` row and every included prompt slot creates
`llm_prompt_slot_usage` plus durable `llm.prompt.slot.accounted` evidence.
Provider-reported usage creates append-only `llm_token_ledger_entries` and
durable `llm.ledger.entry.recorded` events. Accounting events carry ids,
hashes, visibility, usage kind, token counts, and pricing snapshot metadata; raw
prompt, user, provider, and sensitive values are not part of the ledger or
UI-facing protocol payloads.

Analysis and briefs:

- `conversation.analysis.queued`
- `conversation.analysis.started`
- `conversation.analysis.completed`
- `conversation.analysis.failed`
- `knowledge_graph.node_candidate.created`
- `knowledge_graph.edge_candidate.created`
- `knowledge_graph.candidate.confirmed`
- `knowledge_graph.candidate.rejected`
- `knowledge_graph.candidate.superseded`
- `brief.candidate.created`
- `memory.candidate.created`
- `surface.brief.refresh.requested`
- `surface.brief.completed`
- `surface.brief.failed`
- `surface.brief.superseded`
- `ethical.recommendation.candidate.created`
- `handoff.eligibility.recorded`
- `handoff.item.created`
- `handoff.item.accepted`
- `handoff.item.declined`
- `handoff.item.assigned`
- `handoff.item.returned_to_agent`
- `handoff.item.closed`
- `handoff.brief.generated`
- `agent.delegation.changed`

Implemented analysis foundation behavior: eligible visible durable message
creation queues a deterministic local `conversation_analysis_jobs` row and
emits `conversation.analysis.queued`. Running the local analyzer emits
`conversation.analysis.started`, proposed operational candidates,
`brief.candidate.created`, `memory.candidate.created`,
`conversation.tags.updated`, and `conversation.analysis.completed`. Failed jobs
store an error hash and emit `conversation.analysis.failed`. Candidates cite
durable message evidence and provenance and remain proposed until a governed
confirmation path promotes, rejects, or supersedes them.

Implemented knowledge graph candidate behavior: deterministic extraction from a
completed analysis job can create staff-private
`knowledge_graph.node_candidate.created` and
`knowledge_graph.edge_candidate.created` events. Node and edge candidates cite
the source message/event evidence, carry provenance/generating job ids, and
default to `proposed`. Lifecycle transitions emit
`knowledge_graph.candidate.confirmed`, `knowledge_graph.candidate.rejected`, or
`knowledge_graph.candidate.superseded`. These events are not product truth and
do not create a graph database or public graph UI. Surface brief jobs now emit
their own refresh/completion/superseded events. The ethical persuasion v1
foundation does not add a new public recommendation event; it records the
evidence-backed prompt slot through `llm.prompt.slot.included` and
`llm.prompt.slot.accounted` while keeping staff/client presentation separated.

Business outcomes:

- `business.outcome.recorded`
- `business.attribution.proposed`
- `business.attribution.confirmed`
- `business.attribution.rejected`
- `business.attribution.superseded`
- `referral.captured`
- `referral.qualified`
- `referral.converted`
- `referral.lost`
- `ask.outcome.recorded`
- `artifact.recorded`
- `artifact.linked`
- `artifact.usage.recorded`
- `deliverable.published`

Implemented outcome and attribution behavior: public offer acceptance records a
durable `business_outcomes` row and emits `business.outcome.recorded` with
offer, entry point, visitor session, and evidence ids only. Attribution rows
default to `proposed`; accepted offers can propose direct offer influence plus
visitor-session and entry-point influence when those source ids exist. The
model intentionally does not infer campaigns, referrals, artifacts, asks, or
payments without source evidence.

Implemented artifact and deliverable behavior: normalized artifact events carry
artifact ids, kind, title, and evidence refs for system surfaces. Deliverable
events carry client-safe deliverable ids, artifact ids, label, and status
without exposing internal provenance, storage, job, or policy details.

Implemented Customer Feedback and Review events:

- `feedback.item.created`
- `feedback.item.tagged`
- `feedback.item.starred`
- `feedback.item.unstarred`
- `feedback.item.review_candidate.marked`
- `review.requested`
- `review.received`
- `review.consent.confirmed`
- `review.approved`
- `review.published`
- `review.featured`
- `review.retired`

Deferred product intelligence events for later surface work:

- `feedback.item.untagged`
- `feedback.item.linked`
- `feedback.item.review_candidate.dismissed`
- `feedback.brief.requested`
- `feedback.brief.generated`
- `home.about.refresh.requested`
- `home.about.billboard.generated`
- `home.about.billboard.pinned`
- `home.about.billboard.published`
- `home.about.billboard.retired`
- `conversation.mode.changed`
- `conversation.agent.delegated`
- `conversation.agent.delegation.revoked`

These are planned event contracts, not current route claims. They should follow
the existing persistence rule: validate, mutate domain state, insert ordered
domain/conversation event, mirror to global realtime only when relevant, commit,
then broadcast. Feedback/review/Home events must cite evidence refs and must not
publish private feedback without consent and approval.

## Ephemeral Event Catalog

Ephemeral events are routed through in-memory conversation rooms. They may be
coalesced and are not replayed after reconnect except as current snapshot state.

Typing:

- `typing.started`
- `typing.stopped`
- `typing.expired`

Presence:

- `presence.changed`
- `presence.heartbeat`
- `conversation.viewing.started`
- `conversation.viewing.stopped`

Draft and focus:

- `draft.activity.changed`
- `composer.focused`
- `composer.blurred`

AI activity:

- `llm.thinking`
- `llm.retrieving`
- `llm.tool.streaming`
- `llm.text.delta`
- `llm.reconstruction.delta`

Agent assist:

- `agent.private_assist.available`
- `agent.idle_reminder.pending`
- `agent.holding_message.pending`

Transport:

- `connection.latency.sampled`
- `connection.recovered`
- `connection.degraded`
- `connection.resumed`

## Typing Rules

Typing indicators should be privacy-aware and cheap.

- A client may send `typing.start` on keypress only if it has not sent one for
  the same conversation in the last two to three seconds.
- The daemon broadcasts `typing.started` with an expiry timestamp.
- The UI removes typing state when `typing.stopped` arrives or the expiry passes.
- The daemon periodically prunes expired typing state.
- The client sends `typing.stop` on message submit, composer clear, blur, route
  leave, or idle timeout.
- Typing events never include draft text.
- User privacy settings may disable outgoing typing indicators.

## Receipt Rules

Receipt progression should support both simple UI and audit-grade accounting.

Receipt kinds:

- `sent`: client accepted the send command locally;
- `persisted`: daemon wrote the message;
- `delivered`: event reached another participant session;
- `displayed`: message was rendered in a visible conversation viewport;
- `read`: participant intentionally or automatically marked through this
  message as read;
- `unread`: participant manually moved unread boundary backward.

Only daemon-confirmed receipt state is canonical. The client may render local
pending state immediately, then reconcile after `ack` and canonical dispatch.
The current backend persists `persisted`, `read`, and `unread` receipts.
`delivered` and `displayed` remain protocol concepts until UI viewport/session
delivery evidence exists.

## Read And Unread Rules

Each participant should have one canonical read boundary per conversation.

Track:

- `last_read_message_id`;
- `last_read_event_cursor`;
- `last_read_at`;
- `manual_unread_from_message_id`;
- unread message count;
- unread mention count;
- unread action-needed count.

Unread counts should update when messages are created, deleted, read, marked
unread, participant grants change, conversation visibility changes, or a message
is classified as requiring action.
The current backend stores participant read boundaries in
`conversation_read_states`, recalculates unread counts from message sequence and
manual unread boundaries, and exposes a conversation list read model with last
message, read state, conversation counts, and policy-filtered presence.

## Reaction Rules

Reactions are durable conversation events and idempotent state transitions.

- `message.react` with `action: "add"` creates one active reaction per
  `(message, participant, reactionKey)` and repeats without duplicate events.
- `action: "remove"` removes an active reaction when present and is otherwise a
  no-op.
- `action: "toggle"` adds when absent and removes when present.
- Canonical events are `message.reaction.added` and
  `message.reaction.removed`.

## Presence Rules

Presence is privacy-filtered projection state. The gateway broadcasts
`presence.changed` as an ephemeral dispatch and updates
`conversation_presence_snapshots` plus participant `last_seen_at`. It does not
create message rows or durable conversation replay events.

Presence visibility:

- `public`: visible to public-capable participants where a future surface allows
  it;
- `participants`: visible to participants in the conversation;
- `private`: visible only to the participant represented by the snapshot.

## Replay And Resume

The daemon should support two recovery paths:

1. Global replay through the existing `/events?after=<cursor>` path for
   persisted appliance events.
2. Conversation replay through `/chat/ws` `subscribe`, `resume`, or `replay`
   frames using per-conversation `afterSequence` and bounded `limit` values.

The gateway should send current ephemeral snapshots after resume:

- current typing participants;
- current online/viewing participants allowed by policy;
- current LLM run activity;
- current connection health.

The implemented `/chat/ws` gateway clamps replay windows to at most 500 durable
conversation events. A replay after a future or already-consumed sequence
returns no dispatch frames rather than duplicating state. If the local broadcast
receiver lags behind the in-memory fanout buffer, the daemon emits retryable
`client_lagged` so the browser can replay from its latest durable cursor.

## Error Events

Errors should be structured and correlate to `clientId`:

```json
{
  "op": "error",
  "type": "command.rejected",
  "clientId": "...",
  "payload": {
    "code": "policy_denied",
    "message": "Message submission is not allowed for this conversation.",
    "policyDecisionId": "pd_...",
    "retryable": false
  }
}
```

Error codes should cover:

- `invalid_envelope`;
- `frame_too_large`;
- `unsupported_protocol_version`;
- auth required;
- policy denied;
- review required;
- rate limited;
- `client_lagged`;
- conversation not found;
- participant not found;
- idempotency conflict;
- provider unavailable;
- privacy transform failed;
- token budget exceeded.

## Rate Limits

Initial local limits should include:

- typing start: one per participant per conversation per two seconds;
- presence update: one per participant per five seconds, excluding disconnect;
- message submit: configurable sustained and burst limits by actor/session;
- LLM run request: capability and provider budget limits;
- replay: bounded count and cursor windows;
- reaction toggles: coalesce duplicate rapid toggles.

Rate limits should emit explicit events or command errors instead of silently
dropping durable commands.
