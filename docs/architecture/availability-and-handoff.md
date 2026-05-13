# Availability And Handoff Inbox

Status: backend foundation implemented; UI not built

Availability and handoff inbox state form the owner-attention boundary. Ordo can
record when live owner handoff is allowed, why it is blocked, and which evidence
backed attention items need owner review. This slice is local-only: it does not
send messages, submit support packets, trigger notifications, open portals, or
perform external egress.

## Durable Tables

Schema version 14 adds six tables:

- `availability_schedules` stores handoff hours as weekly windows, schedule
  status, timezone label, and metadata.
- `operator_presence` stores the current local operator presence and
  interruption threshold.
- `handoff_eligibility_decisions` records local allow/deny decisions with
  intent, connection trust, reason, and evidence JSON.
- `handoff_inbox_items` stores owner attention items with source, destination,
  request, evidence, approval requirement, delivery state, owner decision, and
  resolution timestamps.
- `handoff_events` stores local state-change events for inbox items.
- `handoff_receipts` stores local receipt evidence for those handoff events.

Fresh databases lazily seed one default availability schedule and one default
operator presence record when the availability API is read or updated.

## Protected Routes

All management routes are protected daemon routes and record policy decisions:

- `GET /availability` reads the current schedule and presence state.
- `PUT /availability/schedule` updates the default handoff schedule.
- `PUT /availability/presence` updates operator presence and interruption
  threshold.
- `POST /handoff/eligibility` records a local handoff eligibility decision.
- `GET /handoff/inbox` lists owner attention items.
- `POST /handoff/inbox` creates an approval-gated attention item.
- `PUT /handoff/inbox/:item_id/resolve` records the owner decision.
- `GET /handoff/inbox/:item_id/receipts` lists local receipt evidence.

The capability catalog marks these as non-MCP-exported protected management
capabilities: `availability.read`, `availability.write`,
`handoff.eligibility.evaluate`, `handoff.inbox.list`, `handoff.inbox.write`, and
`handoff.receipts.list`.

## Eligibility Behavior

Eligibility considers the active schedule, current presence, interruption
threshold, requested intent, and connection trust. A live handoff is allowed only
when:

- the schedule is active and the evaluated time falls inside a configured
  window, or no windows are configured;
- operator presence is `available`;
- the threshold allows the intent.

The threshold values are intentionally conservative:

- `open` allows all intents while schedule and presence allow.
- `selective` allows trusted connections, money, or urgent intents.
- `money_only` allows only money intent.
- `urgent_only` allows only urgent intent.
- `paused` blocks handoff.

Each decision is persisted with evidence so future UI and briefs can explain why
owner access is or is not currently available.

## Support Queue Fields

The handoff inbox is also the first durable Support queue. It is not a CRM or
external ticketing system; it is the local owner-attention spine that Support
can project into briefs and actions.

Each queue item records:

- source object kind and id, such as account, member, visitor session, trial,
  connection, conversation, job, artifact, or request;
- reason and requested action;
- urgency;
- assignee actor id when a staff operator owns the next step;
- due time and next-action hint when supplied;
- evidence refs that can be safely projected into Support work items;
- visibility limited to staff, owner, or system.

Public and member surfaces must not read this table directly. When handoff
status is shown outside Support, it must come through a role-safe projection and
must not reveal staff routing, notes, provider details, policy internals, or
owner-only evidence.

## Inbox And Receipts

Handoff inbox items start in `pending_owner_approval`. Owner decisions transition
them to `approved_local_only`, `declined`, `queued`, or `continue_screening`.
Accepting an item remains local-only and records `externalDelivery: false` in
the event and receipt payload. This gives the backend an auditable decision
spine without adding delivery transports.

Support updates can also assign an item, return it to Ordo for continued
screening, or refresh queue metadata. Terminal decisions are made through the
resolve route so declined and locally approved items remain auditable.

## Non-Goals

This slice does not build UI, availability calendars, handoff chat, push
notifications, voice handoff, support packet egress, RAG, external integrations,
affiliate payouts, analytics dashboards, mediated chat UI, payments, public
portals, or any external egress.
