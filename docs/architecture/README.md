# Architecture

These docs turn the product thesis into system shape.

- [Current System Overview](../system-overview.md)
- [System Architecture](system-architecture.md)
- [Operation Kernel](operation-kernel.md)
- [Capability Catalog](capability-catalog.md)
- [Appliance Runtime](appliance-runtime.md)
- [Briefs](briefs.md)
- [Scheduler](scheduler.md)
- [Realtime Events](realtime-events.md)
- [Conversation Realtime Architecture](conversation-realtime/README.md)
  includes the product doctrine for one client relationship conversation,
  staff handoff queues, role-aware navigation, brief-first surfaces, ethical
  persuasion prompt slots, and realtime protocol/schema/UI implementation.
- [Interactive Account And LLM Chat](conversation-realtime/interactive-account-llm-chat.md)
  defines the active 0.1.8 contract for local account entry, daemon chat
  bootstrap, browser `/chat/ws`, deterministic LLM chat, guarded live-provider
  testing, UI run states, and smoke evidence.
- [Eval System](../evals/README.md) maps deterministic evals, persona-backed
  product journeys, artifact review, and guarded live-provider smoke tests.
- [System Shell](system-shell.md)
- [Backup And Restore](backup-restore.md)
- [Diagnostics And Reports](diagnostics-and-reports.md)
- [Local Install And Providers](local-install-and-providers.md)
- [Business Truth, Visibility, And Publication](business-truth-visibility.md)
- [Public Surface Read Models](public-surfaces.md)
- [Tracked Entry Points And Visitor Sessions](tracked-entry-points.md)
- [Offers And Trial Lifecycle](offers-and-trials.md)
- [Scrollytelling Runtime And Tracked QR Architecture](scrollytelling-runtime/README.md)
- [Connections Foundation](connections.md)
- [Availability And Handoff Inbox](availability-and-handoff.md)
- [Resource, Provenance, And Policy Spine](resource-provenance-policy.md)
- [Access And Local RBAC](access-rbac.md)
- [Knowledge Corpus And Governed Retrieval](knowledge-corpus.md)
- [Scaling With Worker Ordos](scaling-worker-ordos.md)
- [Sovereign Appliance](sovereign-appliance.md)

Architecture docs describe direction until code and tests prove the behavior.

## Status Guide

- Implemented docs describe behavior present in code and tests.
- Draft contracts describe intended architecture for the current appliance
  track.
- Future direction docs describe planned architecture and must not be read as
  shipped behavior.
