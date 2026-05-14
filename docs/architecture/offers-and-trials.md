# Offers And Trial Lifecycle

Status: backend foundation plus Offer Builder and reward-reference baseline implemented

This slice turns public interest into durable commercial state without adding UI,
payments, affiliate payouts, analytics dashboards, external egress, mediated
chat, or RAG.

Product direction: offers should eventually bind into Access, hosted trial
capacity, backup-before-wipe expectations, reward programs, and benefit grants.
The first OrdoStudio pilot offer is a 30-day hosted trial with 10 active spots,
reset/wipe behavior after expiration, backup/export before wipe, and reward
extensions for qualified referrals or accepted feedback.

## What Is Implemented

SQLite stores:

- `offers`: durable offer records with slug, title, summary, status,
  visibility, publication state, 30-day default trial duration, terms metadata,
  source metadata, and timestamps.
- `offer_acceptances`: public-safe acceptance records linked to the accepted
  offer plus visitor session, entry point, and attribution context when present.
- `trials`: 30-day trial lifecycle state linked to an acceptance and offer.
- `trial_events`: durable event rows for trial starts and lifecycle decisions.
- hosted trial capacity policy, hosted trial slots, hosted trial waitlist
  entries, and reset guard state for the pilot hosted trial path.
- scoped resource grants for hosted trial use when a public offer acceptance
  receives an active slot.

Protected owner/operator routes:

- `GET /offer-builder`
- `POST /offer-builder`
- `GET /offers`
- `POST /offers`
- `PUT /offers/:offer_id`
- `GET /offer-acceptances`
- `GET /trials`
- `PUT /trials/:trial_id/status`
- hosted trial capacity and reset owner/operator routes for capacity inspection
  and reset evidence.

Public-safe routes:

- `GET /public/available-offers`
- `POST /public/offers/:offer_slug/accept`

Protected routes pass through the local daemon access boundary and record policy
decision evidence through non-MCP-exported capability ids.

## Offer Builder Baseline

The Offer Builder is daemon-owned validation over durable offer records. It is
not a generic page builder, pack executor, payment adapter, or publishing
adapter.

`GET /offer-builder` returns owner/admin readiness state for current offers:

- durable offer config;
- safe public preview for published public offers;
- supported references backed by current primitives: accepted-offer Access
  grants, hosted-trial capacity/waitlist lifecycle, tracked entry points,
  active reward programs, and policy-gated Support handoff CTA state;
- explicit deferrals for product/workforce pack offer bindings, external
  publishing, payments, and OAuth.

`POST /offer-builder` creates or updates the pilot offer through the same
durable `offers` table and blocks publication when the request tries to save
unsupported reward, pack, payment, OAuth, provider, prompt, staff-internal, or
secret-bearing claims as active offer behavior.

The baseline can publish the 30-day OrdoStudio pilot offer only when terms are
public-safe and disclose experimental hosting, human review, and backup/export
before reset or wipe. Feedback/referral hosted-time rewards can be referenced
only through an active reward program backed by reward ledger and benefit-grant
state.

## Public Offer Boundary

Public offer availability is intentionally narrow. An offer can be accepted only
when it is available through one of these public-safe sources:

- an explicit durable offer with `status = available`, `visibility = public`,
  and `publication_state = published`; or
- a public Offers read-model item derived from published public business facts.

Private, authenticated, staff, owner, draft, archived, revoked, paused, and
unpublished material cannot enter public acceptance.

Public offer responses expose only public-safe offer fields and sanitized terms.
Protected metadata remains owner/admin only. Acceptance receipts keep a terms
snapshot so later edits to a published offer do not rewrite historical accepted
terms evidence.

## Attribution And Trial State

When public acceptance includes a visitor session id, the backend copies the
visitor session's entry point id, entry point slug, and attribution JSON into the
acceptance. Additional public acceptance attribution can be merged into that
record. Raw user agent text is not exposed in offer or trial responses.

Accepting an offer starts a trial immediately. Trial state supports:

- `started`
- `converted`
- `voided`
- `expired`
- `follow_up_needed`

Each lifecycle transition records decision evidence and emits persisted realtime
events.

## Hosted Capacity Implemented Now

The backend now has part of the hosted trial capacity spine:

- default active hosted trial slot limit for the pilot path;
- hosted trial capacity policy records;
- hosted trial slot allocation on public offer acceptance when capacity exists;
- waitlist entry creation when active capacity is full;
- `hosted_trial/use` resource grants for accepted hosted trials;
- reset guard state that requires backup evidence and owner decision before
  reuse/reset proceeds.

This is not a complete hosted control plane. It does not start containers,
assign Traefik routes, send email, run reminder delivery, or decommission a
hosted appliance.

## Future Rewards And Hosted Orchestration

The hosted trial product still needs durable state and orchestration beyond the
current foundation:

- hosted Ordo instance records;
- Docker/Traefik provisioning and route state;
- per-trial data and media volume manifests;
- lifecycle reminders and transactional email receipts;
- scheduled conversation rollup artifacts;
- final backup email and return invitation;
- full decommissioning receipt;
- reward extensions, such as seven extra hosted days for a qualified referral;
- accepted-feedback extensions with a policy-defined amount;
- extension caps and reversal rules;
- experimental hosted-service expectations shown before acceptance.

Rewards should be recorded in the reward ledger and granted through Access.
Trial duration should not be silently changed without ledger evidence.

## Non-Goals

- No payment processing.
- No affiliate payout automation.
- No reward or hosted-time extension ledger.
- No Docker/Traefik hosted appliance commissioning.
- No transactional email or reminder delivery.
- No analytics dashboard.
- No cookie-heavy tracking.
- No RAG or mediated chat.
- No external notifications or egress.
- No reward grant without reward ledger and benefit-grant evidence.
- No product/workforce pack binding or arbitrary pack execution.
