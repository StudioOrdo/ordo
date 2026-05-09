# Connections Foundation

Status: backend foundation implemented; UI not built

Connections are durable relationship records for clients, affiliates, support,
services, and future Worker Ordos. The current backend slice is deliberately
local and scoped: it records relationships, explicit grants, revocations,
connection events, and local receipts without adding portals, chat UI, external
egress, affiliate payouts, or availability/handoff behavior.

## Durable Tables

Schema version 13 adds four connection tables:

- `connections` stores relationship type, display name, lifecycle status,
  identity JSON, scope JSON, metadata JSON, creator, and status timestamps.
- `connection_grants` stores the connection-owned view of an explicit grant,
  including resource kind, resource id, action, status, expiry, reason, creator,
  revocation actor, and revocation reason.
- `connection_events` stores local lifecycle and grant events for inspection.
- `connection_receipts` stores local receipt evidence for each connection event.

Connection grants mirror into the shared `resource_grants` table with
`subject_kind = 'connection'` and the connection id as `subject_id`. This keeps
connection authorization on the same policy spine as actor and role grants.

## Protected Routes

All connection management routes are protected daemon routes and record policy
decisions:

- `GET /connections` lists connection records.
- `POST /connections` creates a connection.
- `PUT /connections/:connection_id` updates connection identity, scope,
  metadata, and status.
- `GET /connections/:connection_id/grants` lists explicit grants.
- `POST /connections/:connection_id/grants` creates an explicit scoped grant.
- `PUT /connections/:connection_id/grants/:grant_id/revoke` revokes a grant.
- `GET /connections/:connection_id/events` lists durable events and receipts.

The capability catalog marks these as non-MCP-exported protected management
capabilities for now: `connections.list`, `connections.write`,
`connection_grants.list`, `connection_grants.write`, and
`connection_events.list`.

## Policy Behavior

`authorize_connection_resource_access` checks the durable grant tables before a
connection can act on a resource. A grant authorizes access only when:

- the connection exists and is `active`;
- the `connection_grants` row is `active`;
- the mirrored `resource_grants` row is an allow grant for
  `subject_kind = 'connection'` and the matching connection id;
- resource kind, resource id, and action match the request;
- neither the connection grant nor resource grant is expired.

Connection grant creation rejects wildcard resource ids and wildcard-style
actions by requiring stable explicit identifiers. Suspending, revoking, or
archiving a connection revokes its active grants.

## Events And Receipts

Connection creation, connection update, grant creation, and grant revocation
write both durable connection events and persisted realtime events. Connection
event payloads intentionally include operational identifiers, type, status,
resource identity, and action; they do not copy provider secrets or private
identity blobs into event history.

Each durable connection event receives a local `connection_receipts` row with a
`local_recorded` receipt. Future approval-gated egress can add other receipt
kinds without changing the foundation contract.

## Non-Goals

This slice does not build Connections UI, availability, handoff inbox, support
packet egress, RAG, external integrations, affiliate payouts, analytics
dashboards, mediated chat UI, payments, public portals, or external egress.
