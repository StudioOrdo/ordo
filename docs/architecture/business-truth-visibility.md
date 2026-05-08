# Business Truth, Visibility, And Publication

Status: backend foundation in progress

This slice gives Ordo a durable place to store local business truth before that
truth is used by public surfaces, governed retrieval, or RAG answer drafting.

## What Is Implemented

The daemon stores business facts in SQLite with provenance, visibility, and
publication state.

SQLite stores:

- `business_facts`: subject, stable fact key, JSON value, source metadata,
  provenance JSON, visibility, publication state, timestamps, and creating
  actor.

The daemon exposes protected local endpoints:

- `GET /business/facts`
- `POST /business/facts`
- `PUT /business/facts/:fact_id`

All routes use the protected daemon access boundary. They are owner/operator
backend contracts, not public website routes.

## Visibility And Publication

Visibility answers who a fact is meant for:

- `public`
- `authenticated`
- `staff`
- `owner`

Publication state answers whether the fact may be used outside owner review:

- `draft`
- `published`
- `archived`
- `revoked`

Public and future retrieval surfaces may only use facts that are both
`visibility = public` and `publication_state = published`. Authenticated and
staff viewers may only see published facts within their tier. The owner view can
inspect draft, archived, revoked, and owner-only facts.

## Events

Business fact writes emit durable system events:

```text
business.fact.created
business.fact.updated
```

Event payloads include fact id, fact key, visibility, and publication state.
They do not include fact values, so private or draft business content does not
leak through the event stream.

## Non-Goals

- No public surface rendering.
- No rich content editor.
- No multi-user identity.
- No RAG answer generation.
- No external publication or egress.
