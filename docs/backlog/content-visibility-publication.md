# Content Visibility And Publication MVP

Status: backend foundation in progress

## Why It Matters

Ordo can only safely answer visitors if content has a clear visibility and
publication contract.

## MVP Scope

- Implement a shared visibility vocabulary: public, authenticated, staff,
  owner.
- Track publication state separately from visibility.
- Attach visibility and publication state to business facts, offers, asks, feed
  items, and future corpus items.
- Add policy helpers that decide whether a viewer can see or use a resource.
- Produce evidence when publication state changes.

## Durable Product Nouns

- Visibility
- Publication State
- Published Resource
- Viewer Context
- Connection Grant

## Acceptance Criteria

- A public visitor cannot retrieve or cause answers from non-public material.
- Owner/staff/authenticated visibility is represented distinctly.
- Publication state can be draft, published, archived, or revoked.
- Tests cover allowed and denied visibility paths.

## Non-Goals

- Rich content editor.
- Public website design.
- Full user accounts.

## Validation

- Policy unit tests.
- Schema migration tests.
- Public read model tests that prove private material is excluded.
