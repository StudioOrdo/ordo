# Implementation Issue Template

Status: required template for behavior-changing implementation issues

Use this template when creating or rewriting implementation issues. Keep each
issue small enough for one focused coding session. Link a companion `Test Plan:`
issue before execution starts.

```md
# <imperative title>

Milestone:
Linked test plan: #<number>

## Product Surface

Primary surface:

Related surfaces:

User-visible proof:

## Architecture Docs

Required reading:
- `docs/...`

Architecture invariants:
- Canonical tables own truth.
- Events own audit/replay.
- Graph tables own relationship traversal and explanation where relevant.
- Projections/read models own surface experience.
- Core owns trust. Packs own workflows.
- AI may add color, explanation, and interpretation, but deterministic Ordo code
  owns structure, policy, access, artifacts, jobs, graph relationships,
  approvals, publication, rewards, and audit.

## Durable Product Nouns

Add or complete:
- <noun>

Do not create:
- <non-goal noun>

## Canonical Tables Touched

Tables:
- <table>: <create/read/update/delete behavior>

No canonical table changes:
- <why>

## Events Emitted

Events:
- `<event.type>`: <when emitted, payload limits>

No event changes:
- <why>

## Graph Nodes And Edges

Nodes:
- `<node_kind>` from `<resource_ref>`

Edges:
- `<source>` `<relationship_kind>` `<target>`

Candidate vs confirmed behavior:

No graph changes:
- <why>

## Projections And Read Models

Projection/read model changes:
- <surface>: <role-safe behavior>

Public/member leak checks:
- staff routing:
- provider internals:
- prompt internals:
- policy internals:
- owner-only data:
- private artifact text:
- unsupported claims:

## Access And Policy Boundaries

Viewer roles:

Access grants required:

Policy decisions recorded:

Failure mode:

## Artifact Behavior

Artifacts created or updated:

Artifact provenance:

Visibility ceiling:

Approval state:

No artifact changes:
- <why>

## Pack Boundary

Core-owned behavior:

Pack-owned workflow behavior:

Pack registration or manifest change:

No pack changes:
- <why>

## LLM Method Contracts

LLM-safe method names involved:
- `<family.method_name>`: <read or mutation, visibility, output evidence>

No arbitrary SQL or generic context access is allowed.

No LLM method changes:
- <why>

## Non-Goals

- <explicitly not doing>

## Acceptance Criteria

- [ ] <observable behavior>
- [ ] <data/audit evidence>
- [ ] <role/privacy behavior>
- [ ] <test coverage>

## Validation Commands

Focused:

```sh
<command>
```

Broader:

```sh
<command>
```

Required check:

```sh
git diff --check
```

## Do Not Claim Complete Until

- [ ] focused tests pass;
- [ ] broader validation passes or deferral is posted with rationale;
- [ ] linked test-plan issue scenarios are covered or explicitly deferred;
- [ ] implementation issue has evidence comment;
- [ ] test-plan issue has coverage comment;
- [ ] code is committed locally;
- [ ] no public/member leaks are introduced;
- [ ] no architecture boundary is bypassed.
```
