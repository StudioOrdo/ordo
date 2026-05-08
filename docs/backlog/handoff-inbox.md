# Handoff Inbox MVP

Status: not built

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

## Validation

- Schema and state transition tests.
- Policy tests for approval requirements.
- UI smoke once inbox exists.
