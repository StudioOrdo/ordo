# Rewards And Incentives

Status: planning spec, not built

Related docs:

- [Rewards And Incentives](../architecture/rewards-and-incentives.md)
- [OrdoStudio NYC Pilot](../business/ordostudio-nyc-pilot.md)
- [Product Roadmap](../business/product-roadmap.md)

## Why It Matters

Ordo should let owners reward useful behavior with governed benefits. The first
pilot needs hosted-time rewards for qualified referrals and accepted feedback.
Future packs should reuse the same machinery for QA credits, render minutes,
pack unlocks, affiliate credit, community leaderboards, and prizes.

The system must not grant rewards by silently mutating counters. Rewards need
evidence, policy, ledger entries, caps, and reversals.

## MVP Scope

- Add reward program records.
- Add reward rule records.
- Add reward event records.
- Add append-only reward ledger entries.
- Add benefit grants that can extend hosted trial access.
- Add qualification review state for feedback and referrals.
- Support pilot rules:
  - qualified referral grants seven hosted days;
  - accepted feedback grants policy-defined hosted days;
  - extension cap is policy-defined.
- Add protected owner/operator read paths for reward programs, pending reviews,
  ledger entries, and benefit grants.
- Add member-safe read path for the user's own reward status and hosted-time
  balance.

## Durable Product Nouns

- Reward Program
- Reward Rule
- Reward Event
- Reward Ledger Entry
- Benefit Grant
- Benefit Balance
- Qualification Review

## Acceptance Criteria

- A tracked referral can create a pending reward event.
- A trial activation can qualify the pending referral reward.
- A qualified referral creates a ledger entry and a seven-day benefit grant.
- An accepted feedback review can create a hosted-time benefit grant.
- A rejected feedback review records why no reward was granted.
- Benefit grants can be active, expired, revoked, or reversed.
- Hosted trial duration changes cite a benefit grant or owner decision.
- Member view can explain current reward status without exposing private
  referral or staff review details.
- Growth view can show reward evidence and pending reviews.
- Duplicate/self-referral cases can be rejected or sent to review.

## Non-Goals

- No cash payout automation.
- No public leaderboard in the MVP.
- No prize fulfillment.
- No reward for scans alone.
- No reward benefit that bypasses Access.

## Validation

- Migration tests for reward tables and indexes.
- Unit tests for referral qualification and feedback qualification.
- Policy tests for member-safe reward read models.
- Reversal tests that prove balances update from ledger/projection state.
- End-to-end pilot smoke: QR referral -> trial activation -> reward -> hosted
  access extension.
