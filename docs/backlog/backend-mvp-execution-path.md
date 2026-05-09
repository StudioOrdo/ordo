# Backend MVP Execution Path

Status: active planning checklist

This is the trackable path from the current local install/provider/vault slice to
a backend-complete MVP surface that can be handed to a UI implementation agent.
It is ordered to finish source-of-truth, policy, retrieval, and workflow
contracts before UI work depends on them.

Use this as the working checklist for GitHub issues and implementation slices.
Each phase should end with updated docs, tests, and stable route/read-model
contracts.

## Handoff Goal

The backend is ready for UI handoff when the daemon exposes stable, documented,
tested contracts for:

- install state and provider configuration;
- owner and business truth;
- visibility and publication policy;
- public surface read models;
- tracked entry points and visitor sessions;
- offers, trial state, and attribution hooks;
- connections, availability, and handoff inbox;
- corpus ingestion and governed retrieval;
- RAG answer drafts with evidence and redaction guarantees.

## Phase Checklist

GitHub milestone: `0.1.2 Backend MVP Readiness`

| Order | Phase | GitHub Issue | Backlog Spec | Backend Exit Criteria | Status |
| --- | --- | --- | --- | --- | --- |
| 0 | Local install, providers, and vault | #49 | [Install And Provider Setup](install-provider-setup.md) | Daemon routes, schema, vault encryption, secret redaction, backup key archival, protected policy decisions, and docs exist. | complete |
| 1 | Business truth, visibility, and publication spine | #50 | [Owner Identity And Business Seeding](owner-identity-business-seeding.md), [Content Visibility And Publication](content-visibility-publication.md) | Durable business facts, provenance, visibility, publication state, and policy helpers exist with tests proving public/private boundaries. | complete |
| 2 | Public surface read models | #51 | [Public Surfaces](public-surfaces.md) | Backend read models for About, Offers, Asks, and Feed return only published public resources. | complete |
| 3 | Tracked entry points and visitor sessions | #52 | [Tracked Entry Points And Visitor Sessions](tracked-entry-points-visitor-sessions.md) | Entry point records, QR/link payloads, visitor sessions, attribution context, and visit/session events exist. | next |
| 4 | Offers and trial lifecycle | #53 | [Offer Acceptance And Trial State](offer-trial-state.md) | Offers, offer acceptance, 30-day trial state, conversion/void/follow-up state, and attribution links exist. | not started |
| 5 | Connections foundation | #54 | [Connections](connections.md) | Connections, grants, revocations, scoped access policy, connection events, and support/affiliate-ready types exist. | not started |
| 6 | Availability and handoff inbox | #55 | [Availability And Presence](availability-presence.md), [Handoff Inbox](handoff-inbox.md) | Availability schedule, operator presence, interruption threshold, handoff eligibility, inbox items, approval state, and receipts exist. | not started |
| 7 | Reports and approved support packet backend | #56 | [Reports And QA Loop](reports-qa-loop.md), [Approved Support Packet Handoff](approved-support-packet-handoff.md) | Report detail/export/status contracts exist, and support packet egress is approval-gated with receipt tracking. | not started |
| 8 | Knowledge corpus and governed retrieval | #57 | [Knowledge Corpus And RAG](knowledge-corpus-rag.md) | Corpus ingestion, source/item provenance, SQLite FTS retrieval, visibility filtering, and retrieval evidence exist. | not started |
| 9 | RAG answer draft spine | #58 | [Knowledge Corpus And RAG](knowledge-corpus-rag.md) | Provider-backed answer draft job uses governed retrieval, emits evidence, avoids unsupported claims, and preserves redaction guarantees. | not started |
| 10 | MCP pack and tool hardening | #59 | [MCP Packs And Tool Hardening](mcp-packs-tool-hardening.md) | Pack manifest validation, tool schemas, side effect declarations, capability policy mapping, and disable behavior exist. | not started |
| 11 | Backend handoff package | #60 | this document | UI-ready route contracts, state docs, smoke seeds, validation matrix, and known non-goals are collected for the UI agent. | not started |

## Phase Detail

### 0. Local Install, Providers, And Vault

Current local implementation work should be finished, validated, committed, and
represented by a GitHub issue or PR before moving deeper into MVP backend work.

GitHub issue: #49. Pull request: #61, merged.

Done means:

- schema version and migrations are stable;
- provider secrets are encrypted and write-only;
- backup/restore preserves vault usability;
- protected route capability ids are cataloged;
- docs describe the honest security boundary;
- full Rust validation passes.

Current validation evidence:

- `cargo fmt --all -- --check`: passed;
- `cargo test --workspace`: passed, 83 tests;
- `cargo clippy --workspace --all-targets -- -D warnings`: passed;
- `git diff --check`: passed.

### 1. Business Truth, Visibility, And Publication Spine

This is the next implementation slice. It should add the durable truth and
policy layer that public surfaces and RAG will consume.

GitHub issue: #50. Pull request: #62, merged.

Done means:

- owner/business install basics can seed editable business facts;
- business facts include source, provenance, visibility, and publication state;
- policy helpers decide whether a viewer can inspect or retrieve each fact;
- tests prove draft/private/staff/owner material cannot enter public read
  models or retrieval.

Current validation evidence:

- `cargo fmt --all -- --check`: passed;
- `cargo test --workspace`: passed, 88 tests;
- `cargo clippy --workspace --all-targets -- -D warnings`: passed;
- `git diff --check`: passed.

### 2. Public Surface Read Models

This phase creates backend contracts for UI without designing the UI yet.

GitHub issue: #51. Pull request: #63, merged.

Done means:

- About, Offers, Asks, and Feed have read endpoints or read-model builders;
- every response is derived from published public records;
- missing-readiness states are explicit enough for the System UI.

Current implementation evidence:

- daemon routes exist for `/public/surfaces`, `/public/about`,
  `/public/offers`, `/public/asks`, and `/public/feed`;
- public read models are derived from published public `business_facts` only;
- read-model responses include provenance evidence and explicit readiness.

Current validation evidence:

- `cargo fmt --all -- --check`: passed;
- `cargo test --workspace`: passed, 91 tests;
- `cargo clippy --workspace --all-targets -- -D warnings`: passed;
- `git diff --check`: passed.

### 3. Tracked Entry Points And Visitor Sessions

This phase gives the product a durable path from QR/link/campaign entry into
visitor activity.

Done means:

- tracked entry points can be created and resolved;
- visitor sessions carry entry context;
- session events preserve enough attribution evidence for offers and affiliate
  credit later.

### 4. Offers And Trial Lifecycle

This phase turns public interest into durable commercial state.

Done means:

- offers are records, not page copy;
- offer acceptance records visitor/session/source context;
- trial state supports start, expiration, conversion, void, and follow-up;
- attribution hooks are present but payout automation is not required.

### 5. Connections Foundation

This phase creates scoped relationships for clients, affiliates, support,
services, and future worker Ordos.

Done means:

- connections have type, identity, status, scope, grants, and revocations;
- policy decisions can consult connection grants;
- connection events and receipts are durable.

### 6. Availability And Handoff Inbox

This phase gives Ordo a safe owner-attention boundary.

Done means:

- schedule, presence, and interruption threshold influence handoff eligibility;
- handoff inbox items include source, destination, evidence, required approval,
  delivery state, and receipt;
- no live or external handoff can occur without policy approval.

### 7. Reports And Approved Support Packet Backend

This phase turns local reports into an explicit support handoff path.

Done means:

- reports have detail, export, and status contracts;
- support packet payload is previewable before egress;
- sending is approval-gated and receipt-backed;
- redaction tests cover provider/vault material.

### 8. Knowledge Corpus And Governed Retrieval

This phase makes retrieval safe before generation is added.

Done means:

- corpus sources and items can be created from approved facts/content;
- SQLite FTS retrieval returns candidate items with provenance;
- retrieval filters by visibility, publication state, and viewer context;
- missing evidence is a first-class result.

### 9. RAG Answer Draft Spine

This phase adds provider-backed drafting on top of governed retrieval.

Done means:

- answer draft jobs gather retrieval evidence before provider calls;
- outputs include cited source items and limitations;
- unsupported claims are rejected or clearly marked;
- provider calls use configured provider/vault state without leaking secrets.

### 10. MCP Pack And Tool Hardening

This phase keeps customization inside the appliance trust boundary.

Done means:

- pack manifests describe capabilities, schemas, side effects, approval needs,
  and artifact contracts;
- tools cannot execute without capability policy mapping;
- dangerous tools remain non-exported until approval flows exist.

### 11. Backend Handoff Package

This is the final pre-UI step.

Done means:

- route contracts and response examples are documented;
- seed scenarios exist for first-run, configured provider, public surface,
  visitor session, offer/trial, handoff, and retrieval;
- validation commands are current;
- known non-goals are explicit;
- GitHub issues/PRs are clean enough for the UI agent to start without reverse
  engineering backend intent.

## Validation Bar

For every backend phase:

- migration tests for new tables or columns;
- unit tests for policy boundaries;
- route/read-model tests for stable contracts;
- event/artifact/brief tests when the phase emits durable evidence;
- docs updated in `docs/architecture/`, `docs/state-of-the-project.md`, and the
  relevant backlog spec;
- `cargo fmt --all -- --check`;
- `cargo test --workspace`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `git diff --check`.

Frontend validation begins only when a phase creates or changes UI behavior.
