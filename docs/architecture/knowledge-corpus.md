# Knowledge Corpus Skeleton

Status: Implemented retrieval safety foundation

Ordo now has a durable SQLite skeleton for future knowledge and RAG work. This
is not shipped retrieval. It is the access-aware substrate future retrieval must
use.

## Current Shape

The daemon schema stores:

- corpus sources;
- corpus items, including future chunk-shaped records.

Each record can carry:

- stable ID;
- source identity;
- resource kind and resource ID;
- classification metadata;
- provenance metadata;
- status;
- timestamps;
- general metadata for future retrieval plumbing.

Corpus sources and items are ordinary SQLite records. SQLite remains the source
of truth.

## Access Boundary

Corpus records have resource identity before retrieval exists. That lets policy
tests prove future retrieval cannot ignore access boundaries.

The current foundation supports the same access shapes used by the local RBAC
spine:

- public resources;
- owner/system resources;
- per-actor private resources.

Resource grants remain the durable access path. Future retrieval should first
resolve corpus records through policy-aware resource checks, then retrieve or
rank only records the actor is allowed to read.

## What This Enables

This slice prepares for future work such as:

- approved knowledge corpora;
- source-grounded retrieval;
- content packs;
- actor-private memory;
- access-aware briefs and chat answers.

## Non-Goals

- No embeddings.
- No vector store or vector search.
- No RAG answer generation.
- No chat retrieval UI.
- No public customer, student, or client portal.
- No external integrations.
- No hosted identity provider.
- No legal, medical, finance, or tax product mode.
- No Job Kernel V2.