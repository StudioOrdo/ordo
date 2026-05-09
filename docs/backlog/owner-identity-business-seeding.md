# Owner Identity And Business Seeding MVP

Status: backend foundation in progress

## Why It Matters

Public surfaces and business workflows need approved local truth about who the
Ordo represents before it can speak for the business.

## MVP Scope

- Store owner identity basics and business profile basics.
- Add editable business facts with provenance, visibility, and publication
	state.
- Separate private owner facts from public business facts.
- Show a compact System view of seeded truth and missing required fields.
- Emit events when business truth changes.

## Durable Product Nouns

- Owner Identity
- Business Profile
- Business Fact
- Provenance Record
- Visibility Classification

## Acceptance Criteria

- Ordo can distinguish owner-private and public-approved facts.
- Business facts include source, timestamp, visibility, and publication state.
- Public answer surfaces cannot use unapproved or private facts.
- Changes are visible in events or briefs.

## Non-Goals

- Full CRM.
- Multi-user identity.
- Public website editing experience.

## Validation

- Schema/migration tests.
- Policy tests for visibility boundaries.
- UI tests once a truth-seeding surface exists.
