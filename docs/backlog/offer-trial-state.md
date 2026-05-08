# Offer Acceptance And Trial State MVP

Status: not built

## Why It Matters

Offers and trials turn public interest into trackable business outcomes without
losing attribution or follow-up context.

## MVP Scope

- Store offers as durable records.
- Record offer acceptance from a visitor session or connection.
- Add 30-day Ordo trial state as a concrete commercial proof.
- Record trial start, expiration, conversion, void, and follow-up state.
- Link acceptance and trial state to tracked entry and attribution context.

## Durable Product Nouns

- Offer
- Offer Acceptance
- Trial
- Conversion State
- Follow-Up Task

## Acceptance Criteria

- Offer acceptance is persisted with source and visitor/session context.
- Trial state can be listed and summarized for the owner.
- Conversion/void decisions have evidence.
- No payment or credit is inferred without recorded state.

## Non-Goals

- Billing processor integration.
- Legal terms management.
- Affiliate dashboard.

## Validation

- Schema and state transition tests.
- Event tests.
- UI smoke once owner trial view exists.
