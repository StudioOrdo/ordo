# Knowledge Corpus And Governed Retrieval

Status: Implemented governed retrieval backend foundation

Ordo has a durable SQLite corpus and a governed local retrieval path for future
knowledge and RAG work. This is not answer generation. It is the access-aware
retrieval substrate future drafts must use before any provider call is allowed.

## Current Shape

The daemon schema stores:

- corpus sources;
- corpus items, including chunk-shaped records;
- a local SQLite FTS index over corpus item title and body text.

Each source and item can carry:

- stable ID;
- source identity;
- resource kind and resource ID;
- classification metadata;
- provenance metadata;
- status;
- timestamps;
- item content hash evidence;
- general metadata for future retrieval plumbing.

Corpus sources and items are ordinary SQLite records. SQLite remains the source
of truth.

The daemon exposes protected local routes for:

- `GET /corpus/sources`
- `POST /corpus/sources`
- `GET /corpus/sources/:source_id`
- `PUT /corpus/sources/:source_id`
- `GET /corpus/items`
- `POST /corpus/items`
- `GET /corpus/items/:item_id`
- `PUT /corpus/items/:item_id`
- `POST /corpus/retrieve`

Corpus mutations maintain source/item records and the local FTS index. They do
not call providers, create embeddings, or send data outside the appliance.

## Retrieval Contract

Retrieval accepts a local query, viewer context, optional actor id, and bounded
limit. It searches SQLite FTS, then filters every candidate before returning it.

Returned results include:

- corpus source and item records;
- FTS rank;
- snippet;
- source and item provenance;
- item content hash;
- classification metadata;
- explicit `generatedAnswer: false` evidence.

Responses include `evidence_found` or `missing_evidence`, plus limitations that
name the local-only FTS boundary.

## Access Boundary

The retrieval foundation supports the same access shapes used by the local RBAC
spine:

- public resources;
- owner/system resources;
- per-actor private resources.

Resource grants remain the durable access path. Retrieval only returns approved
items from approved sources and checks visibility plus resource access before a
candidate becomes evidence.

## What This Enables

This slice prepares for future work such as:

- source-grounded retrieval;
- content packs;
- actor-private memory;
- access-aware briefs and chat answers.

## Non-Goals

- No embeddings.
- No vector store or vector search.
- No RAG answer generation.
- No provider or model calls.
- No chat retrieval UI.
- No public customer, student, or client portal.
- No external integrations.
- No hosted identity provider.
- No legal, medical, finance, or tax product mode.
- No Job Kernel V2.
