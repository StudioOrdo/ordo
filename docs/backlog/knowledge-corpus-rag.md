# Knowledge Corpus And RAG MVP

Status: answer draft backend ready for PR; provider-backed generation not built

## Why It Matters

Ordo needs governed retrieval before it can answer from business truth, public
content, support docs, or domain packs.

## MVP Scope

- Ingest approved text artifacts into corpus sources/items.
- Add SQLite FTS before vector search unless vector need is proven.
- Enforce visibility and provenance on every retrieval candidate.
- Return answer evidence with cited source items and limitations.
- Keep generated answers separate from source truth.

## Backend Foundation

- Corpus source and item routes support protected local create, update, list,
	and read contracts.
- Corpus items maintain SHA-256 content hashes and local SQLite FTS entries for
	title/body search.
- Retrieval returns source/item evidence, rank, snippet, provenance,
	classification, content hash, limitations, and explicit missing-evidence
	states.
- Retrieval filters candidates by approved status, visibility, viewer context,
	and durable resource access before results are returned.
- Answer draft preparation creates durable local draft and citation records only
	after governed retrieval runs.
- Draft records persist prompt/input metadata, retrieval query/evidence, cited
	corpus item IDs, limitations, status, and provenance.
- Missing evidence creates an explicit `needs_evidence` draft state and no
	source claims.
- The current backend does not call providers or models, use embeddings, or
	leave the appliance.

## Durable Product Nouns

- Corpus Source
- Corpus Item
- Retrieval Query
- Retrieval Result
- Answer Evidence
- Answer Draft
- Answer Draft Citation

## Acceptance Criteria

- Public retrieval only returns public approved material.
- Every answer can identify source items or state that evidence is missing.
- Retrieval respects owner/authenticated/staff/public boundaries.
- Corpus ingestion is repeatable and inspectable.
- Missing evidence is explicit when no approved visible source item matches.
- Answer drafts cite corpus item IDs and record provenance.
- Draft output does not add unsupported facts from the prompt.
- Secret-like prompt and instruction material is redacted before persistence.

## Non-Goals

- Vector database in the first MVP unless required.
- Open-ended web crawling.
- Answers without evidence.
- Provider/model calls in this backend foundation.
- Embeddings or vector search in this backend foundation.
- External answer transport or chat UI.

## Validation

- Access-aware retrieval tests.
- FTS/query tests.
- Evidence-first answer draft tests.
- Missing-evidence and redaction tests.
- Report/brief evidence tests.
