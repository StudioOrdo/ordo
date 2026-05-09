# Tracked Entry Points And Visitor Sessions MVP

Status: not built

## Why It Matters

QR codes, affiliate links, offer links, and campaign links need durable source
context so Ordo can connect visits, conversations, offers, and attribution.

## MVP Scope

- Create tracked entry point records.
- Generate stable links and QR payloads.
- Create visitor sessions with entry context.
- Carry attribution into future public Ordo conversation and offer acceptance.
- Record visit/session events.

## Durable Product Nouns

- Tracked Entry Point
- Visitor Session
- Attribution Context
- Visit Event

## Acceptance Criteria

- Entry point source is preserved across a visitor session.
- QR/link targets cannot expose non-public destinations.
- Visitor session data has retention and privacy boundaries.
- Session events are queryable for later briefs and attribution.

## Non-Goals

- Full analytics dashboard.
- Cookie-heavy ad tracking.
- Affiliate payouts.

## Validation

- Routing/session tests.
- Policy tests for destination visibility.
- Event replay tests.
