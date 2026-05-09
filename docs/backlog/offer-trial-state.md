# Offer Acceptance And Trial State MVP

Status: backend foundation implemented

## Why It Matters

Offers and trials turn public interest into trackable business outcomes without
losing attribution or follow-up context.

## MVP Scope

- Store offers as durable records. Implemented in SQLite and protected daemon
	routes.
- Record offer acceptance from a visitor session or connection. Implemented for
	visitor sessions; connections are a later phase.
- Add 30-day Ordo trial state as a concrete commercial proof. Implemented as
	durable trial rows with a 30-day default.
- Record trial start, expiration, conversion, void, and follow-up state.
	Implemented through lifecycle transition state and events.
- Link acceptance and trial state to tracked entry and attribution context.
	Implemented for visitor-session-backed acceptances.

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

## Current Backend Contract

Protected owner/operator routes:

- `GET /offers`
- `POST /offers`
- `PUT /offers/:offer_id`
- `GET /offer-acceptances`
- `GET /trials`
- `PUT /trials/:trial_id/status`

Public-safe routes:

- `GET /public/available-offers`
- `POST /public/offers/:offer_slug/accept`

Public acceptance only works for explicit public/published/available offers or
Offers read-model items derived from published public business facts.
