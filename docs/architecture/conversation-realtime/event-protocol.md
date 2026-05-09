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
| `llm.run.request` | durable `llm.run.started` plus stream events | `llm.invoke` |
| `llm.run.cancel` | durable or ephemeral cancel event | `llm.cancel` |
| `tool.approve` | durable approval event | `llm.tool.approve` |
| `tool.reject` | durable rejection event | `llm.tool.reject` |
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
- `llm.provider.started`
- `llm.tool.requested`
- `llm.tool.approved`
- `llm.tool.rejected`
- `llm.tool.completed`
- `llm.text.completed`
- `llm.usage.recorded`
- `llm.run.completed`
- `llm.run.failed`
- `llm.run.cancelled`

Analysis and briefs:

- `conversation.analysis.queued`
- `conversation.analysis.completed`
- `knowledge.candidate.created`
- `brief.candidate.created`
- `surface.brief.generated`
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

Business outcomes:

- `offer.outcome.recorded`
- `ask.outcome.recorded`
- `referral.captured`
- `referral.qualified`
- `referral.converted`
- `referral.lost`
- `artifact.usage.recorded`

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
2. Conversation replay through `/conversations/:id/events?after=<sequence>` or
   the WebSocket `resume` operation for per-conversation ordered state.

The gateway should send current ephemeral snapshots after resume:

- current typing participants;
- current online/viewing participants allowed by policy;
- current LLM run activity;
- current connection health.

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

- invalid envelope;
- unsupported protocol version;
- auth required;
- policy denied;
- review required;
- rate limited;
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
