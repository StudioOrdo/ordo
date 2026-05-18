# Product Operating Model

Status: canonical direction, implementation varies by surface

This document records the product shape that should guide Ordo work after the
NYC product-shell batch. Current code remains the source of truth for shipped
behavior. This model explains how the shipped pieces should converge.

This is an architecture document. Member-facing UI must translate these terms
into plain language using [Product Language](product-language.md).

## Product Thesis

Ordo is a local-first operating system for governed work.

It is not primarily a chatbot, dashboard, CRM clone, marketplace, or generic
workflow builder. The interface may be conversational, but durable state,
policy, evidence, approvals, and work execution belong to deterministic Ordo
code.

The product loop is:

```text
Attention -> Decision -> Governed Action -> Evidence -> Receipt
```

The runtime rule is:

```text
Daemon notices.
Requests route.
Capabilities authorize.
Offers grant.
Packs constrain.
Jobs execute.
Artifacts prove.
Graph explains.
Knowledge promotes.
Humans decide.
System protects.
```

## Core Surfaces

### My Ordo

My Ordo is the personal cockpit for every member: owner, staff, customer, trial
user, affiliate, support person, or network member.

It answers:

```text
What needs me now?
What is safe to click?
What is only draft or readiness?
What has evidence?
What happens next?
```

My Ordo owns member-facing "For you" activity, Requests, Offers, what Ordo can
do, receipts, and the relationship conversation with Ordo. Chat belongs here as
one way to act, not as a separate product.

### Support

Support is the global handoff surface. Support-capable members can see eligible
handoff requests, claim open work according to policy, and work realtime
relationship conversations without exposing staff-only context to public or
member views.

Product rule:

```text
Any member with support.accept_handoff can claim eligible open handoff requests.
First valid claim wins.
```

### Studio

Studio is the production and construction surface. It owns offers, workflow
templates, production DAGs, generated artifacts, previews, approvals, broken job
repair, and publication prep.

Studio should show workflow state from daemon records. It must not imply task
execution, provider success, publication, graph truth, or memory promotion when
only compilation, readiness, or review state exists.

### Knowledge

Knowledge is the evidence and truth surface. It owns packs, sources, ingestion
DAGs, graph candidates, memory candidates, entity/claim/edge review, promotion
readiness, and retrieval trust.

Knowledge does not make generated analysis truth by default. Generated output
can create candidates or readiness packets. Durable truth requires policy,
evidence, and explicit promotion.

### Growth

Growth is the owner/admin briefing surface. It should become the Presidential
Daily Briefing for analytics, retention, referrals, content performance, offer
performance, and recommended next requests.

A Growth report is not a vague LLM summary. It should expose question, scope,
data used, methods, assumptions, limitations, findings, evidence, and proposed
next actions.

### System

System protects the appliance. It owns runtime health, providers, permissions,
audit, backup, restore, policy, local install state, egress controls, secrets,
and migration safety.

System should be quiet for normal members and explicit for operators.

## Canonical Objects

The object names below are internal product architecture. They are useful for
schemas, code, issues, and implementation planning. They are not automatically
good UI labels. For example, a member should usually see "People waiting for
help" instead of "handoff queue projection" and "Ready for review" instead of
"memory promotion readiness packet."

### Offer

An Offer grants access to capabilities, packs, support, reports, services,
workflows, trials, or network membership.

Offers are governed commercial and relationship entry points. Accepting an
offer may create access grants, capability grants, enabled pack state, request
templates, limits, expiration, and policy constraints.

### Capability

A Capability determines what a member can do and which requests they can
receive.

Capabilities are product-shaped permissions, not UI labels. A surface may show
simple copy, but the daemon should authorize against explicit capability ids.

### Request

A Request is the public/member-friendly object for an ask, approval, review,
handoff, repair, or decision.

Before the canonical Request kernel exists, Request may remain a projection
over source-specific mechanisms such as handoffs, artifact reviews, memory
readiness, workflow approvals, feedback asks, and system issues.

### WorkItem / DecisionQueueItem

A WorkItem or DecisionQueueItem is the internal routing object behind a
Request. It carries source area, state, assignment, visibility, priority, due
date, allowed actions, and audit evidence.

Members should not need to understand this internal routing language.

### Pack

A Pack declares workflows, knowledge, policy, assurance, graph boundaries, and
request templates. Packs own workflow shape. Core owns trust.

Packs cannot gain authority from prompt text, UI placement, or hidden defaults.

### Job / DAG

A Job or DAG executes governed production work. The job DAG controls execution
state. Graph memory explains what the job used, produced, affected, or proved.
These must not collapse into one model.

### Artifact

An Artifact is a durable output or evidence-bearing work product. It should
carry provenance, content hash, evidence refs, source refs, visibility ceiling,
status, and approval state when relevant.

### Event

An Event is the audit and replay record for state changes. Human approvals,
policy decisions, egress decisions, promotions, and publication transitions
must leave auditable events.

### Graph

Graph explains relationships and provenance. Canonical tables own truth, events
own audit, graph tables own traversal/explanation, and projections own surface
experience.

Vectors can assist retrieval later. They do not own truth.

### Memory

Memory is reusable context with strict boundaries. Generated content may create
memory candidates or readiness packets. Promotion is explicit, evidence-backed,
and auditable.

### Report / Brief

Reports and briefs turn state into operational understanding. They should show
evidence, limitations, and next actions. They may generate Requests, but should
not silently mutate truth or publish outcomes.

## Daemon-First Flow

The daemon is the always-on local runtime. It watches local state, evaluates
policy, prepares safe work, and asks for human decisions when accountability is
required.

```text
Signal arrives
  public entry, conversation, source, offer, support ask, job result, report

Daemon evaluates
  policy, capability, pack state, confidence, evidence, visibility

Daemon creates or updates work
  Request for people
  WorkItem for routing
  Job/DAG for execution
  Artifact/Event for durable record

Surface shows safe action
  My Ordo, Support, Studio, Knowledge, Growth, or System

Human or policy decides
  approve, reject, request changes, claim, snooze, escalate, view receipt

Ordo records outcome
  event, artifact, audit, evidence refs, state transition
```

## Surface Routing Rule

Use the smallest surface that matches the decision:

- personal attention or relationship action -> My Ordo;
- global handoff work -> Support;
- production workflow or artifact construction -> Studio;
- evidence, graph, memory, or promotion -> Knowledge;
- business intelligence and recommended next action -> Growth;
- appliance safety, provider, policy, audit, backup, restore -> System.

## Safety Invariants

- Generated analysis must not become raw evidence by default.
- LLM output must create candidates, drafts, or readiness, not durable truth.
- Promotion requires policy or explicit review.
- Promoted graph or knowledge objects need evidence/provenance.
- Prompt/provider internals must not become public labels or member memory.
- Packs cannot gain hidden authority from UI placement or prompt text.
- Inactive packs should not influence active retrieval or decisions.
- Required packs cannot be silently skipped.
- Human approvals must create auditable decisions.
- External/provider egress must be policy-checked.
- Secrets and local environment values must not appear in committed artifacts,
  issue comments, screenshots, or member-visible UI.

## Ordo And Executor Boundary

`StudioOrdo/ordo` owns the operating shell and local appliance runtime:

- My Ordo, Support, Studio, Knowledge, Growth, System;
- offers, capabilities, requests, jobs, artifacts, events, graph, memory,
  reports, policy, audit, backup, and restore.

`ordo_executor` is a donor and foundry lane:

- source processing;
- evidence extraction;
- pack building;
- progressive QA manufacturing;
- website/projection experiments;
- export/import contract research.

The bridge should be a contract and selected donor code, not a repo merge.

## Non-Goals For The Current Batch

Do not use this model to justify broad architecture work in the current batch:

- no Executor merge;
- no new vector or graph database;
- no canonical Request storage rewrite unless specifically scoped;
- no full pack assurance/persona consensus kernel;
- no marketplace;
- no hidden provider calls;
- no automatic memory or graph promotion;
- no public publishing without governed approval.
