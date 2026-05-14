# Hosted Trial Capacity And Reset

Status: partially implemented backend foundation; control-plane orchestration not built

Related docs:

- [Offers And Trial Lifecycle](../architecture/offers-and-trials.md)
- [OrdoStudio NYC Pilot](../business/ordostudio-nyc-pilot.md)
- [Rewards And Incentives](../architecture/rewards-and-incentives.md)

## Why It Matters

The OrdoStudio pilot offer depends on real capacity and lifecycle behavior:
30-day hosted trials, 10 active pilot spots, backup/export before wipe,
expiration/reset, reward extensions, and waitlist behavior when capacity is
full.

This is the Systems side of the first wedge. It proves Ordo can offer hosted
appliances without pretending the pilot is production-critical infrastructure.

## Implemented Foundation

- Hosted trial capacity policy foundation.
- Active hosted trial slot allocation.
- Waitlist entry creation when capacity is full.
- Trial expiration timestamp from offer acceptance.
- Scoped `hosted_trial/use` resource grant on accepted hosted trials.
- Reset guard state requiring backup evidence and owner decision before reset.

## Remaining MVP Scope

- Add hosted Ordo instance records.
- Add Docker/Traefik commissioning and route verification.
- Add per-trial volume/media manifest.
- Add complete reset/wipe schedule.
- Add backup/export readiness before expiration.
- Add explicit pre-wipe reminder state.
- Add reward extension support through benefit grants.
- Add owner override for extension, expiration, and reset decisions.
- Add experimental hosting expectation metadata on the offer/trial.
- Add final backup email, return invitation, and decommission receipt.

## Durable Product Nouns

- Hosted Trial Capacity Policy
- Hosted Trial Slot
- Hosted Trial Waitlist Entry
- Hosted Ordo Instance
- Hosted Ordo Route
- Trial Expiration
- Trial Reset/Wipe Plan
- Backup Before Wipe Requirement
- Trial Extension

## Acceptance Criteria

- The pilot offer can cap active trials at 10.
- The 11th qualified acceptance enters waitlist or receives a capacity response.
- A started trial has a computed expiration date.
- A reward benefit can extend expiration within policy caps.
- A trial cannot be wiped without recorded expiration/reset state.
- Backup/export readiness is visible before wipe.
- Owner can mark a trial converted, expired, voided, or extended.
- Member-safe UI can explain trial status, backup action, and expiration.
- Systems can show capacity, waitlist, expiration, and reset evidence.

## Non-Goals

- No payment processing.
- No production uptime SLA.
- No multi-region hosting automation.
- No cash reward payout.
- No external notification delivery unless a separate notification slice exists.

## Validation

- Migration tests for capacity, waitlist, expiration, and reset records.
- Unit tests for 10-slot capacity and waitlist behavior.
- Unit tests for extension caps and reward-based extension evidence.
- Policy tests for member-safe trial status.
- End-to-end pilot smoke: accept offer -> allocate slot -> extend with reward
  -> export backup -> expire/reset state recorded.
