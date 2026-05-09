# Backlog

Status: planning backlog, not implementation proof

This folder tracks high-level MVP specs for product and platform features that
are called out in the current roadmap, architecture docs, and code review notes.
Backlog specs are not release commitments and do not claim behavior is shipped.

Use these specs to open GitHub issues. Keep issue scopes smaller than the full
feature whenever possible.

For the current backend-to-UI path, use [Backend MVP Execution Path](backend-mvp-execution-path.md)
as the working checklist.

## How To Use This Backlog

Each feature spec records:

- why the feature matters;
- the MVP scope;
- durable product nouns introduced or completed;
- acceptance criteria;
- non-goals;
- validation evidence expected before merging.

If implementation changes the contract, update the relevant backlog spec and
architecture doc together.

## MVP Specs

| Tracking Doc | Purpose | Status |
| --- | --- | --- |
| [Backend MVP Execution Path](backend-mvp-execution-path.md) | Ordered backend phases and handoff criteria before UI implementation. | active planning checklist |

## Feature Specs

| Spec | Purpose | Status |
| --- | --- | --- |
| [Install And Provider Setup](install-provider-setup.md) | Turn backend install/provider/vault state into a minimal operator setup path. | backend foundation merged; UI not built |
| [Owner Identity And Business Seeding](owner-identity-business-seeding.md) | Capture local owner and business truth for future public surfaces. | backend foundation merged |
| [Content Visibility And Publication](content-visibility-publication.md) | Define public/authenticated/staff/owner visibility and publication state for business truth. | backend foundation merged |
| [Connections](connections.md) | Model trusted relationships with scope, grants, history, and revocation. | not built |
| [Availability And Presence](availability-presence.md) | Model handoff hours, operator status, and interruption thresholds. | not built |
| [Handoff Inbox](handoff-inbox.md) | Create handoff envelopes and owner attention items with evidence and receipts. | not built |
| [Approved Support Packet Handoff](approved-support-packet-handoff.md) | Send reviewed diagnostic/support packets with explicit approval and receipt. | reports foundation exists; egress not built |
| [Reports And QA Loop](reports-qa-loop.md) | Expand local reports into a practical developer/support feedback loop. | local reports exist |
| [Public Surfaces](public-surfaces.md) | Build public About, Offers, Asks, and Feed read models without private leakage. | backend read models implemented; UI not built |
| [Tracked Entry Points And Visitor Sessions](tracked-entry-points-visitor-sessions.md) | Track QR/link/campaign entry and visitor conversations. | not built |
| [Offer Acceptance And Trial State](offer-trial-state.md) | Record offer acceptance, 30-day trial state, and follow-up evidence. | not built |
| [Affiliate Attribution](affiliate-attribution.md) | Connect affiliates, referral assets, attribution, and credit review. | not built |
| [Mediated Chat](mediated-chat.md) | Let owners and visitors communicate through Ordo with context and policy intact. | not built |
| [Job Kernel V2](job-kernel-v2.md) | Add leases, ownership, cancel, retry, and resume semantics for longer work. | not built |
| [Knowledge Corpus And RAG](knowledge-corpus-rag.md) | Move from corpus skeleton to governed retrieval and answer evidence. | skeleton exists |
| [MCP Packs And Tool Hardening](mcp-packs-tool-hardening.md) | Let domain tools and packs customize work without bypassing the trust boundary. | local MCP foundation exists |
| [Worker Ordos And A2A](worker-ordos-a2a.md) | Define worker Ordos, artifact return, and peer support envelopes. | future direction |

## Quality Bar

No backlog item should ship with:

- public answers from private truth;
- hidden egress;
- unscoped connections;
- unsupported product claims;
- provider secrets readable through UI, events, logs, reports, policy metadata,
  or manifests;
- external packets without explicit approval and receipt;
- custom tools outside capability, policy, artifact, brief, and audit spines.
