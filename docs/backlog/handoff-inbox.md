# Handoff Inbox MVP

Status: backend foundation merged; UI not built

## Why It Matters

The owner should receive evidence-backed attention items instead of raw noise.
Handoffs are how Ordo crosses an attention, trust, or execution boundary.

## MVP Scope

- Create handoff envelope records.
- Create inbox items for owner review.
- Attach source, destination, request, included evidence, approval requirement,
  delivery state, and receipt/outcome.
- Let the owner accept, decline, queue, or ask Ordo to continue screening.
- Emit events and briefs for attention-worthy handoffs.

## Backend Foundation

- SQLite schema version 14 stores handoff inbox items, handoff events, and local
    handoff receipts.
- Protected local daemon routes list, create, resolve, and inspect receipt
    evidence for inbox items.
- Inbox items preserve source, destination, request, evidence, approval
    requirement, delivery state, owner decision, and resolution timestamps.
- Owner decisions are local-only state transitions; accepting an item records
    `approved_local_only` and receipt evidence with `externalDelivery: false`.

## Durable Product Nouns

- Handoff Envelope
- Attention Inbox Item
- Approval Requirement
- Delivery State
- Receipt

## Acceptance Criteria

- Every handoff has source, destination, evidence, and state.
- No external delivery occurs without required approval.
- Owner decisions are persisted and auditable.
- Inbox items can be listed and resolved.

## Non-Goals

- Full mediated chat.
- External support egress.
- Voice handoff.
- Support packet egress, public portals, external delivery, mediated chat UI,
  payments, affiliate payouts, analytics dashboards, or external integrations.

## Validation

- Schema and state transition tests.
- Policy tests for approval requirements and protected route audit.
- UI smoke once inbox exists.
