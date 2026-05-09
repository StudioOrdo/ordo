# Knowledge Corpus And RAG MVP

Status: corpus skeleton exists; retrieval not built

## Why It Matters

Ordo needs governed retrieval before it can answer from business truth, public
content, support docs, or domain packs.

## MVP Scope

- Ingest approved text artifacts into corpus sources/items.
- Add SQLite FTS before vector search unless vector need is proven.
- Enforce visibility and provenance on every retrieval candidate.
- Return answer evidence with cited source items and limitations.
- Keep generated answers separate from source truth.

## Durable Product Nouns

- Corpus Source
- Corpus Item
- Retrieval Query
- Retrieval Result
- Answer Evidence

## Acceptance Criteria

- Public retrieval only returns public approved material.
- Every answer can identify source items or state that evidence is missing.
- Retrieval respects owner/authenticated/staff/public boundaries.
- Corpus ingestion is repeatable and inspectable.

## Non-Goals

- Vector database in the first MVP unless required.
- Open-ended web crawling.
- Answers without evidence.

## Validation

- Access-aware retrieval tests.
- FTS/query tests.
- Report/brief evidence tests.
