# Appliance Operating Discipline

Status: architecture doctrine as of 2026-05-13

Current canon:

- [Current Product Canon](../business/current-product-canon.md)
- [Workforce Substrate](../business/workforce-substrate.md)
- [Product Canon Gap Map](product-canon-gap-map.md)
- [Target Architecture Plan](target-architecture-plan.md)
- [Rewards And Incentives](rewards-and-incentives.md)

Ordo should borrow the operating discipline of serious enterprise SaaS systems
and compress it into a local-first appliance. The goal is not enterprise
software complexity. The goal is enterprise-grade execution machinery hidden
behind conversational, text-first, owner-controlled product surfaces.

The formula is:

```text
Enterprise operating discipline
+ local appliance ownership
+ agentic execution
+ conversational UX
+ governed community packs
```

For implementation layering, command/query flow, job-kernel direction, and
slice sequence, use the [Target Architecture Plan](target-architecture-plan.md).

## What To Borrow

Borrow patterns that protect correctness, trust, and repeatability:

- CQRS-style read models;
- append-only events and audit trails;
- RBAC, scoped grants, and policy decisions;
- workflow/DAG execution;
- leases, retries, idempotency, cancellation, and pause/resume;
- approval gates and human requests;
- artifact provenance and versioning;
- observability, traces, and diagnostic packets;
- data retention and visibility boundaries;
- analytics and attribution ledgers;
- extension contracts and adapter boundaries.

These are not "enterprise features." They are operating machinery for serious
AI work.

## What To Reject

Do not import enterprise SaaS bloat:

- centralized multi-tenant assumptions as the default architecture;
- dashboard sprawl;
- configuration-first UX;
- form-first workflow authoring;
- hidden vendor lock-in;
- opaque workflow engines users cannot inspect;
- generic plugin marketplaces that bypass policy;
- read paths that force the frontend to reconstruct product meaning from raw
  tables.

The product should expose a trusted workforce, not an enterprise admin console.

## CQRS-Lite Rule

Use CQRS as a practical read-model discipline, not as ceremony.

```text
Command handler
-> validate access, policy, and invariant
-> mutate canonical table
-> append event
-> update or schedule projection refresh
```

Canonical tables own truth. Events own audit and replay. Projection tables own
surface experience.

The first read-model target should be a shared `surface_work_items`
projection. It can feed Member View, Studio, Support, Knowledge, Growth, and
Systems without asking the frontend to understand raw backend internals.

## Event Discipline

Every meaningful mutation should leave an event with enough information to
answer:

- who acted;
- what changed;
- what policy allowed it;
- what evidence supports it;
- what artifact, request, job, or outcome was affected;
- what should be replayable or projectable later.

Events should be boring, explicit, and stable. Use them for audit,
diagnostics, read-model projection, A2A packet construction, and owner-visible
explanations.

## Result Envelopes

Every task executor should return a structured result envelope:

```text
task id
capability id
executor target
status
output summary
artifact refs
evidence refs
policy decision refs
metrics
limitations
error code/message when failed
```

The envelope is more important than the executor implementation. Rust, browser
WASM, MCP, native Mac tools, AVFoundation, LLM providers, and future A2A
workers should all return through the same shape.

## Clean Architecture Boundary

Ordo's internal direction should be:

```text
Domain -> Application -> Ports -> Infrastructure -> Interfaces
```

Domain objects include Actor, Connection, Tracked Entry Point, Offer, Access,
Request, Reward Program, Benefit Grant, Capability, Pack, Plan, Job, Task,
Artifact, Event, Outcome, and Read Model.

Application services include AcceptOffer, GrantAccess, CompilePlan, StartJob,
RunReadyTasks, CreateRequest, ApproveArtifact, PublishArtifact, and
RecordOutcome. Growth-oriented services include QualifyReward and GrantBenefit.

Ports include CapabilityExecutor, LlmProvider, RetrievalProvider,
ArtifactStore, EventBus, PolicyEngine, PackRegistry, SurfaceProjector, and
GrowthLedger.

Infrastructure adapts SQLite, Rust executors, browser/WASM workers, MCP,
provider clients, native Mac/AVFoundation tools, artifact storage, and future
A2A transports to those ports.

Interfaces include daemon HTTP, chat WebSocket, MCP, frontend read models, CLI,
and future A2A adapters.

## SOLID Rules

- Access logic does not live in the job executor.
- Plan compilation does not execute tasks.
- Surface projections do not mutate canonical truth.
- Artifact storage does not decide publication policy.
- LLM gateway does not invent capability authority.
- MCP, A2A, browser, and native execution are adapters, not the product spine.
- Product packs change what work is available; they do not change trust
  boundaries.

## Knuth Rule

Prefer simple, inspectable data structures first.

Do not build a clever orchestration engine before the invariants are true:

- every action is authorized;
- every job is explainable;
- every artifact has provenance;
- every request knows who must decide;
- every public claim has evidence;
- every outcome can be traced.

Measure only after the shape is honest. Then optimize hot paths such as chat
latency, plan compilation, task claiming, retrieval, artifact writes, and
projection refresh.

## Appliance Advantage

Rust, SQLite, and Next.js give Ordo an unusual opportunity:

- Rust can own durable execution, native integration, provider safety, and
  local media tooling.
- SQLite can be the inspectable local truth boundary with projection tables and
  event replay.
- Next.js can provide an excellent conversational and review-oriented product
  surface.
- Browser/WASM can execute safe client-side candidate work.
- Native Mac capabilities can become governed media and production executors.

Use that advantage to make Ordo an inspectable AI appliance, not another opaque
hosted workflow SaaS.
