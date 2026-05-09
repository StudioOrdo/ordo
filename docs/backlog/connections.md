# Connections MVP

Status: backend foundation merged; UI not built

## Why It Matters

Connections are the trust-and-work relationship surface for clients, affiliates,
support, workers, services, and future peer Ordos.

## MVP Scope

- Create a durable `Connection` record with type, display identity, status, and
  scope.
- Store grants and revocations for connection-specific access.
- Record connection events and receipts.
- Support Studio Ordo Support and affiliate connections as first concrete
  examples.
- Show Connections in a restrained System or owner surface.

## Backend Foundation

- SQLite schema version 13 stores `connections`, `connection_grants`,
  `connection_events`, and `connection_receipts`.
- Connection grants mirror into shared `resource_grants` with
  `subject_kind = 'connection'` so policy can authorize scoped connection
  actions through the existing durable access spine.
- Protected local daemon routes exist for listing, creating, updating, grant
  creation/revocation, and event inspection.
- Connection lifecycle and grant mutations record durable connection events,
  local receipts, persisted realtime events, and protected route policy audit
  decisions.
- Broad implicit grants are rejected; grant creation requires explicit resource
  ids and stable actions.

## Durable Product Nouns

- Connection
- Connection Grant
- Connection Event
- Receipt
- Revocation

## Acceptance Criteria

- A connection can be created, scoped, suspended, and revoked.
- Connection grants are consulted by policy decisions.
- Connection history is inspectable.
- No connection grants access without explicit scope.

## Non-Goals

- Social graph.
- Public profiles for every connection.
- Full contact manager.
- Availability, handoff inbox, support packet egress, RAG, external
  integrations, affiliate payouts, analytics dashboards, mediated chat UI,
  payments, public portals, or external egress.

## Validation

- Schema, policy, event, receipt, and protected route tests.
- UI smoke once surface exists.
