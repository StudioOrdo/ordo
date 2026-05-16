# Target Architecture Plan

Status: target implementation plan as of 2026-05-13

Current canon:

- [Current Product Canon](../business/current-product-canon.md)
- [Workforce Substrate](../business/workforce-substrate.md)
- [Appliance Operating Discipline](appliance-operating-discipline.md)
- [Product Canon Gap Map](product-canon-gap-map.md)

This plan translates the product stance into implementation shape. Ordo should
borrow proven enterprise operating ideas, compress them into a local appliance,
and hide the machinery behind conversational, evidence-backed surfaces.

The product promise is:

```text
Enterprise-grade operating machinery for one-person businesses
without making the person operate enterprise software.
```

## Target Flow

```text
conversation intent
-> command
-> policy and access decision
-> plan compilation
-> job / task DAG
-> executor result envelope
-> event
-> artifact / request / outcome
-> surface projection
-> brief, review, approval, or next action
```

Chat is the control surface, not the product spine. Jobs, tasks, requests,
artifacts, events, read models, access, and outcomes are the product spine.

## DAG As Operating Spine

The DAG is the operating spine for work. The graph explains relationships and
memory; the DAG changes the world through bounded tasks, gates, retries, and
artifacts.

```text
LLM parses intent or proposes a next step
-> Ordo validates access and policy
-> Ordo resolves typed variables
-> Ordo compiles a deterministic DAG
-> tasks run through registered capabilities
-> human requests pause or unblock gated work
-> artifacts, events, graph candidates, and projections record what happened
```

LLMs should help with language-shaped work: intent parsing, classification,
drafting, summarization, explanation, review, ambiguity handling, and candidate
proposals. Deterministic Ordo code owns plan compilation, DAG structure,
policy, access, capability dispatch, retries, artifact state, event emission,
publication, rewards, and graph or memory promotion.

The practical rule is:

```text
The LLM may suggest what work this is.
Ordo decides what work is allowed and how it runs.
```

## Layers

Use a Clean Architecture direction:

```text
Domain
-> Application
-> Ports
-> Infrastructure
-> Interfaces
```

### Domain

The domain should stay small and boring:

- Actor
- Connection
- Tracked Entry Point
- Offer
- Access
- Request
- Reward Program / Benefit Grant
- Capability
- Pack
- Plan / Compiled Plan
- Job / Task
- Artifact
- Event
- Outcome
- Read Model

Domain code owns invariants. It should not know about React, HTTP, provider
SDKs, MCP, A2A, browser workers, or AVFoundation.

### Application

Application services own commands and orchestration:

- AcceptOffer
- GrantAccess
- CompilePlan
- StartJob
- ClaimReadyTask
- CompleteTask
- CreateRequest
- ApproveArtifact
- PublishArtifact
- RecordOutcome
- QualifyReward
- GrantBenefit
- ProjectSurfaceWorkItems

Each command handler should validate policy and invariants, mutate canonical
state, append events, and update or schedule read-model projection.

### Ports

Ports describe what Ordo needs without binding to a vendor or runtime:

- PolicyEngine
- CapabilityExecutor
- LlmProvider
- RetrievalProvider
- ArtifactStore
- EventBus
- PackRegistry
- SurfaceProjector
- GrowthLedger
- RewardLedger
- NotificationSink
- A2ATransport

### Infrastructure

Infrastructure adapts real systems to the ports:

- SQLite canonical tables, events, and projections;
- Rust daemon job execution;
- OpenAI-compatible and local deterministic LLM providers;
- MCP projections over registered capabilities;
- browser/WASM workers;
- native Mac and AVFoundation executors;
- artifact storage;
- future A2A transports.

Adapters are replaceable. The product spine is not.

### Interfaces

Interfaces expose Ordo safely:

- Next.js surfaces;
- daemon HTTP routes;
- `/chat/ws`;
- `/ws` event replay;
- CLI commands;
- MCP;
- future A2A.

Interfaces should render or submit commands. They should not invent product
truth, bypass policy, or mutate canonical state directly.

## CQRS-Lite Implementation

Use CQRS as a read-model discipline, not a distributed-systems project.

Keep SQLite. Do not introduce a separate read database, queue service, or event
stream service until the appliance proves it needs one.

```text
Canonical tables own truth.
Events own audit and replay.
Projection tables own surface experience.
```

First projections:

- `surface_work_items`: shared attention/work queue for Member View, Studio,
  Support, Knowledge, Growth, and Systems.
- `surface_object_timeline`: role-safe timeline for selected offers, requests,
  jobs, artifacts, connections, and entry points.
- `surface_briefs`: plain-language summaries with evidence references and
  limitations.

Read models are disposable. If product meaning changes, rebuild projections
from canonical state and events.

## Job Kernel V2

The job runtime should support serious production work without becoming a
workflow-builder product.

Required kernel behavior:

- compiled plan snapshot per run;
- variable schema validation;
- DAG dependency validation;
- task leases and lease expiration;
- idempotency keys;
- retry policy snapshots;
- pause, resume, cancel, and skip decisions;
- structured result envelopes;
- artifact and evidence references;
- human request gates;
- parallel execution where dependencies and policy allow.

Every executor returns through the same envelope whether the work ran in Rust,
TypeScript, browser/WASM, MCP, native Mac tools, AVFoundation, an LLM provider,
or a future peer Ordo.

## Product Pack Boundary

A product pack is a packaged workforce. It can install work, but it must not
install hidden authority.

Pack manifests may include:

- capability bindings;
- content and corpus scopes;
- prompt templates;
- variable schemas;
- compiled-plan templates;
- request templates;
- artifact contracts;
- visibility and publication rules;
- approval gates;
- limits and reset policies;
- growth metrics.

Packs reference registered capabilities. They do not introduce arbitrary code,
new egress paths, provider transports, or trust boundaries.

## Pattern Use

Use patterns where they simplify the system:

- Command: user and system mutations.
- Strategy: executor selection and provider selection.
- Adapter: MCP, A2A, browser/WASM, native Mac, provider, and storage bindings.
- State: job, task, request, offer, trial, and publication lifecycles.
- Observer: event fanout, projection refresh, and diagnostics.
- Specification: policy, visibility, access, and publication predicates.
- Builder: compiled plans, prompt payloads, QR contexts, and support packets.

Do not add pattern vocabulary where a plain function or table is clearer.

## Engineering Discipline

Agentic coding increases throughput, so the architecture has to reduce
ambiguity:

- every slice names the durable product noun it adds or completes;
- every UI claim cites durable state or stays clearly aspirational;
- every external call crosses an adapter boundary and records policy;
- every artifact records provenance and visibility;
- every implementation issue includes validation and non-goals;
- every broad change updates tests or docs that would catch regression.

The work style should be conservative: small slices, explicit invariants,
boring data structures, focused tests, and source-of-truth docs kept current.

## Implementation Sequence

Use this sequence unless a narrower bug fix is blocking current work:

1. Command/event conventions for offer, access, request, job, artifact, and
   outcome mutations.
2. `surface_work_items` projection for all canonical surfaces.
3. Offer-to-access grants and entitlement state.
4. Hosted trial capacity, expiration, backup-before-wipe, and reset policy.
5. Reward programs, referral/feedback qualification, reward ledger, and benefit
   grants through Access.
6. Product/workforce pack manifest spine.
7. Request spine for approval, feedback, consent, QA, missing information, and
   artifact review.
8. Compiled plan, variable schema, and reusable DAG run support.
9. Job Kernel V2 leases, retries, idempotency, cancellation, and result
   envelopes.
10. Studio production execution, including media-capability envelopes for
   browser/WASM, MetaVisKit-style JSON, and native Mac/AVFoundation tools.
11. Growth ledger for QR, referral, offer, content, publish, reward, and
   performance events.
12. A2A support packet and worker assignment adapters after local invariants
    are strong.

## Non-Goals

- A generic visible workflow builder as the default UX.
- A plugin marketplace that bypasses capability policy.
- A separate read-model service before SQLite projections are exhausted.
- A hosted multi-tenant control plane as the default architecture.
- Form-first configuration for ordinary users.
- Public or member UI that exposes raw operational tables.
