# Availability And Presence MVP

Status: backend foundation merged; UI not built

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

## Backend Foundation

- SQLite schema version 14 stores availability schedules, operator presence,
    interruption threshold, and handoff eligibility decisions.
- Protected local daemon routes read and update schedule/presence state.
- Handoff eligibility records evidence for allow/deny decisions based on
    schedule, presence, threshold, intent, and connection trust.
- Paused schedule, paused/offline/non-available presence, and restrictive
    thresholds block live handoff decisions.

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
- UI, public promises of live access, push notifications, external calendars,
  external integrations, payments, mediated chat, or any external egress.

## Validation

- Unit tests for handoff eligibility cases.
- Protected route policy audit tests.
- UI smoke once status is surfaced.
