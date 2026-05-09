# Tracked Entry Points And Visitor Sessions MVP

Status: backend foundation implemented

## Why It Matters

QR codes, affiliate links, offer links, and campaign links need durable source
context so Ordo can connect visits, conversations, offers, and attribution.

## MVP Scope

- Create tracked entry point records. Implemented in SQLite.
- Generate stable links and QR payloads. Implemented as daemon read models and
	payload JSON.
- Create visitor sessions with entry context. Implemented through public-safe
	session creation.
- Carry attribution into future public Ordo conversation and offer acceptance.
	Implemented as durable session attribution context; future consumers are not
	built.
- Record visit/session events. Implemented as durable session event rows and
	persisted realtime events.

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
