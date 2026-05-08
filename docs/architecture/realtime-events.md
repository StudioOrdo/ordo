# Realtime Events

Status: Implemented first durable replay slice for Ordo 0.1.1

Realtime is a projection of durable events.

## Source Of Truth

SQLite stores job, task, schedule, brief, and system events. The daemon mirrors
replayable events into a global cursor log and exposes them through `/events`.
WebSocket broadcasts those events to connected clients after persistence.

The browser can reconnect, request events after its last cursor, and reconstruct
state from persisted records.

## Event Families

0.1.0 should support these event families:

- job events;
- task events;
- artifact events;
- schedule events;
- brief events;
- system health and daemon lifecycle events;
- Next supervision and restart events.

## WebSocket Role

The WebSocket stream makes progress feel live. It does not replace the event
store.

The UI should show connection status, but job history and progress must remain
correct after refresh.

## Replay API

`GET /events` returns persisted events ordered by global cursor. Clients may use
`after=<cursor>` to fetch events missed while disconnected, and `limit=<count>`
to bound the replay window.

Job events keep their per-job sequence in `job_events`; the replay log adds a
global cursor that spans job and system lifecycle events.
