# Realtime Events

Status: Draft contract for Ordo 0.1.0

Realtime is a projection of durable events.

## Source Of Truth

SQLite stores job, task, schedule, brief, and system events. WebSocket
broadcasts those events to connected clients after persistence.

The browser must be able to reconnect, fetch missed events, and reconstruct
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