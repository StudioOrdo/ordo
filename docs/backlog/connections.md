# Connections MVP

Status: not built

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

## Validation

- Schema and policy tests.
- Event/replay tests for connection events.
- UI smoke once surface exists.
