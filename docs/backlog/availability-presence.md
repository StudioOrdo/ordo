# Availability And Presence MVP

Status: not built

## Why It Matters

Ordo should be able to talk anytime, but humans should only be interrupted when
availability and policy allow it.

## MVP Scope

- Store business handoff hours.
- Store operator presence state.
- Store interruption threshold: open, selective, money-only, urgent-only,
  paused.
- Add a handoff eligibility function that considers schedule, presence,
  threshold, intent, and connection trust.
- Show current handoff status to the owner.

## Durable Product Nouns

- Availability Schedule
- Operator Presence
- Interruption Threshold
- Handoff Eligibility Decision

## Acceptance Criteria

- Ordo can decide whether live handoff is currently allowed.
- Decisions produce evidence explaining the result.
- Paused status blocks live handoff.
- Public UI never promises live owner access when policy denies it.

## Non-Goals

- Calendar sync.
- Push notifications.
- Voice calls.

## Validation

- Unit tests for handoff eligibility cases.
- Policy decision tests.
- UI smoke once status is surfaced.
