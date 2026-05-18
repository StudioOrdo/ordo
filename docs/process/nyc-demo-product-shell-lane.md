# NYC Demo Product Shell Lane

Status: required execution guidance for the 0.1.9 NYC demo chain

This document constrains the next demo delivery work to `StudioOrdo/ordo`.
Executor, knowledge-pack foundry import, vector databases, marketplace work,
full pack assurance, operational reviewer personas, and consensus review are
outside this lane.

## Mission

Demo-proof the Ordo product shell without destabilizing the local appliance
spine:

```text
public story path
Studio workflow state
support handoff path
memory readiness without automatic promotion
evidence-safe UI
local appliance trust boundary
```

Current source of truth:

1. Current `ordo` code and tests.
2. Current GitHub issues and the latest Batch Execution Manifest.
3. This lane document and linked architecture/product docs.
4. Broader future architecture docs.

## Product Vocabulary

Use these terms consistently in issue comments, docs, UI copy, and review
reports:

- `Member`: any person in the system: owner, staff, customer, support person,
  trial user, affiliate, or network member.
- `Offer`: grants access to capabilities, packs, support, reports, services,
  workflows, trials, or network membership.
- `Capability`: determines what a member can do and which requests they can
  receive.
- `Request`: public/member-friendly object for an ask, approval, review,
  handoff, repair, or decision.
- `DecisionQueueItem` / `WorkItem`: internal routing object behind requests.
- `Pack`: declares workflows, knowledge, policy, assurance, graph boundaries,
  and request templates.
- `Studio`: production DAGs, offer/workflow construction, generated artifacts,
  broken job resolution.
- `Knowledge`: packs, graph candidates, memory candidates, entity/claim/edge
  review, ingestion, promotion readiness.
- `Support`: global handoff and realtime conversation view.
- `Growth`: owner/admin PDB and future SUMO report surface.
- `System`: health, permissions, providers, audit, backup, policy, and local
  appliance safety.

Target reconciliation:

```text
Offers grant access.
Capabilities determine what members can do.
Requests route work and decisions.
Packs define workflows, knowledge, policy, and assurance.
Studio runs production DAGs.
Knowledge runs ingestion/review/promotion DAGs.
Support handles handoffs.
Growth creates operational briefs and next-action requests.
System protects the appliance.
Graph explains why things connect.
Evidence decides what can be claimed.
Humans decide when policy requires accountability.
```

## Validation Baseline

Run these before each feature slice, or explicitly report why a known baseline
failure is being isolated:

```bash
git status --short
git branch --show-current
git log --oneline -5

cargo fmt --all -- --check
cargo test --workspace
npm run typecheck
npm run build
npm run smoke:ui
```

Known audit baseline from 2026-05-16:

- `cargo test --workspace`: passed, 572 tests.
- `npm run typecheck`: passed.
- `npm run build`: passed.
- README temp SQLite daemon smoke: passed.
- `cargo fmt --all -- --check`: failed on committed rustfmt drift.
- `npm run smoke:ui`: failed 4 stale/changed UI expectations.

Do not hide these failures. Either fix them as hygiene slices or state clearly
that the selected feature branch is isolating them.

## Primary NYC Issue Order

Work in this order unless the owner changes demo priority:

1. #413 - Wire Story Intake to workflow compilation evidence.
2. #415 - Add Story workflow state to Studio Preview.
3. #419 - Add first-user relationship landing handoff from tracked entry.
4. #417 - Add memory promotion readiness packet for approved generated content.

Rationale:

- #413 creates the evidence bridge needed by #415.
- #415 makes governed workflow state visible in Studio Preview.
- #419 comes before #417 because the public relationship path is demo-critical
  for the NYC narrative.
- #417 is trust-boundary work and must remain readiness-only, not promotion.

## Slice Boundaries

### #413 Story Intake To Workflow Compilation Evidence

Goal: approved or submitted Story Intake creates durable workflow compilation
evidence refs usable by Studio Preview without leaking private intake text or
provider/policy internals.

Do not publish, promote memory, mutate graph truth, call external providers, or
broaden into generic Executor import.

### #415 Studio Preview Workflow State

Goal: Studio Preview shows governed workflow state from daemon evidence.

Required states:

```text
compiled
blocked
missing_input
awaiting_approval
ready
degraded
```

Do not collapse the job DAG and graph memory. The job DAG controls execution
state; the graph explains what the job used, produced, affected, or proved.

### #419 First-User Relationship Landing Handoff

Goal: a tracked public entry can create or return a safe first-user relationship
handoff.

Product rule:

```text
Any member with support.accept_handoff can claim eligible open handoff requests.
First valid claim wins.
```

Existing capabilities may be mapped or aliased, but issue comments and UI copy
should converge on `support.accept_handoff`.

Do not make all staff automatic support agents, expose internal conversation
context, or create duplicate handoffs on refresh/retry.

### #417 Memory Promotion Readiness Packet

Goal: approved generated-content memory candidates produce a read-only
readiness packet.

This is not memory promotion. The packet may include evidence refs, blockers,
actor/job origin, and allowed next action. It must not mutate canonical memory,
graph, vectors, pack state, or hidden provider/prompt internals.

## Request Spine Guidance

Do not introduce a fully canonical stored Request table before NYC unless a demo
issue absolutely requires it.

For NYC, keep Request as a projection over existing source-specific mechanisms:

- support handoffs;
- feedback asks;
- artifact reviews;
- memory decisions;
- workflow approvals;
- system issues.

Future kernel slice:

```text
Request:
  public/member-friendly object for ask, approval, review, handoff,
  repair, or decision.

DecisionQueueItem / WorkItem:
  internal routing object with state, source area, assignment,
  visibility, priority, due date, and allowed actions.
```

## Pack And Assurance Guidance

Only implement pack state semantics before NYC if required by the open demo
issues.

Future-safe pack state vocabulary:

```text
installed
active
inactive
required
quarantined
deprecated
superseded
```

Before NYC, enforce only what is needed:

- inactive packs do not influence active projections;
- required packs cannot be silently skipped.

Do not build the full assurance/persona/consensus kernel before NYC.

Presentation personas may affect tone/style only. They must not authorize
truth, promotion, policy, publication, access, or support routing.

## Non-Goals Before NYC

Do not:

- merge `ordo_executor` into `ordo`;
- build Executor import;
- switch databases;
- add Qdrant, Lance, Kuzu, or another vector/graph database;
- build the SUMO report chain;
- build full consensus review;
- build operational reviewer personas;
- create a marketplace;
- generalize every HITL queue;
- rewrite Request storage;
- allow generated analysis to become truth;
- auto-promote memory or graph objects;
- let inactive packs influence decisions;
- skip required packs silently.

## Slice Report Format

Every patch or PR must report:

- what changed;
- files changed;
- DB/migration impact;
- routes impacted;
- UI surfaces impacted;
- trust-boundary impact;
- tests added or updated;
- commands run;
- known failures;
- what was intentionally not done.

