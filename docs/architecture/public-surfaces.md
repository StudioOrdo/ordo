# Public Surface Read Models

Status: backend read models implemented

This slice gives Ordo stable daemon-owned contracts for public About, Offers,
Asks, and Feed data without building public UI. These read models are derived
from approved public business truth, not ad hoc page copy.

## What Is Implemented

The daemon exposes read-only public surface endpoints:

- `GET /public/surfaces`
- `GET /public/about`
- `GET /public/offers`
- `GET /public/asks`
- `GET /public/feed`

The read models are schema-free in this slice. They are built from existing
`business_facts` records where both conditions are true:

- `visibility = public`
- `publication_state = published`

Facts with `owner`, `staff`, or `authenticated` visibility are excluded. Facts
in `draft`, `archived`, or `revoked` publication states are excluded.

## Fact Key Mapping

Public surfaces use stable fact-key prefixes:

- `about.*` becomes About fields.
- `offers.*` and `offer.*` become Offer items.
- `asks.*`, `ask.*`, `wants.*`, and `want.*` become Ask items.
- `feed.*` becomes Feed items.

Item surfaces group keys by the first segment after the prefix. For example,
`offers.consulting.title` becomes the `title` field on the `consulting` offer.

## Response Shape

Every response includes explicit readiness metadata. Empty surfaces are not
treated as errors; they return `ready = false`, `factCount = 0`, and a missing
reason that System and future UI surfaces can render honestly.

Public fields include provenance evidence:

- fact id;
- fact key;
- source kind, label, and URI;
- provenance JSON;
- publish and update timestamps.

The read models do not expose internal visibility or publication state because
the route contract only returns records that passed the public publication gate.

## Non-Goals

- No public website UI.
- No rich site builder.
- No visitor session tracking or attribution.
- No offer acceptance or trial lifecycle.
- No RAG retrieval or answer generation.