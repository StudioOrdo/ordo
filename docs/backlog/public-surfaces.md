# Public Surfaces MVP

Status: backend read models implemented; UI not built

## Why It Matters

About, Offers, Asks, and Feed are how the business meets the world. They must
be built from approved public truth, not ad hoc page copy.

## MVP Scope

- Add read models for About, Offers, Asks, and Feed. Implemented in the daemon.
- Only include published public resources. Implemented through the business fact
  visibility and publication gate.
- Provide basic public routes or previews. Implemented as daemon JSON routes.
- Show provenance or source summary where useful. Implemented through public
  field evidence.
- Let System/owner surfaces identify missing public readiness. Implemented as
  explicit readiness metadata; UI rendering is not built.

## Durable Product Nouns

- About Profile
- Offer
- Want
- Ask
- Feed Item
- Public Surface Read Model

## Acceptance Criteria

- Public routes cannot include owner/private material.
- Offer and Ask data are durable business facts, not only page strings.
- Feed items carry provenance metadata.
- System shell can show whether public surfaces are ready once UI work consumes
  the daemon readiness contract.

## Non-Goals

- Full site builder.
- Rich design editor.
- Payments.
- Public account system.

## Validation

- Visibility tests.
- Public read model tests.
- Browser smoke for public routes once UI surfaces are built.
