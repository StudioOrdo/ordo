# Executor Bridge Contract

Status: boundary guidance before integration work

`ordo_executor` is valuable as a donor and foundry lane, but it should not be
merged directly into `StudioOrdo/ordo`.

## Responsibility Split

`StudioOrdo/ordo` owns the product appliance:

- My Ordo;
- Support;
- Studio;
- Knowledge;
- Growth;
- System;
- offers;
- capabilities;
- requests and work projections;
- jobs and DAG state;
- artifacts;
- events;
- graph and memory promotion boundaries;
- reports and briefs;
- provider policy, audit, backup, and restore.

`ordo_executor` owns evidence-processing and foundry experiments:

- source manifests and corpus processing;
- document, transcript, media, and archive extraction;
- entity, claim, vector, and graph candidates;
- HITL review packet experiments;
- pack export and inspection;
- progressive QA manufacturing;
- rendered website and projection experiments.

## Bridge Rule

Use a contract, not a repo merge.

The minimum future bridge is:

```text
ExecutorRunExport
  run manifest
  source artifact records
  extraction artifacts
  candidate graph records
  claim/entity/edge candidates
  review packets
  promotion receipts or readiness receipts
  QA reports
  evidence/source refs
  trust-boundary summary

OrdoImportReceipt
  imported refs
  rejected refs
  required reviews
  policy decisions
  created jobs/artifacts/events
  limitations
```

## Import Boundaries

Ordo import must preserve these rules:

- executor output is not automatically truth;
- generated analysis is not raw evidence;
- candidates remain candidates until Ordo policy or humans promote them;
- imported artifacts need provenance, content hash, visibility ceiling, and
  source/evidence refs;
- pack state changes require Ordo policy;
- graph truth mutation requires Ordo graph promotion paths;
- memory promotion requires Ordo memory promotion paths;
- provider/prompt internals stay private;
- import creates events and audit receipts.

## Donor Candidates

Good candidates to borrow from `ordo_executor`:

- safe task result envelope patterns;
- evidence role and trust-boundary tests;
- pack export/import manifest ideas;
- review packet shapes;
- progressive QA run and quality report contracts;
- website QA manufacturing process;
- operator simulation and promotion preview patterns.

Do not import wholesale:

- the experimental UI as an Ordo surface;
- public-story compatibility code;
- separate homepage graph concepts;
- direct vector database assumptions;
- provider scripts that bypass Ordo policy;
- generated analysis as source truth.

## Integration Sequence

1. Stabilize current Ordo product shell and NYC handoff path.
2. Define a narrow export/import schema in docs and tests.
3. Import one read-only executor run fixture into Ordo as candidates and
   artifacts.
4. Add an Ordo review/projection surface for that fixture.
5. Only then consider shared crates or selected code movement.

## Non-Goals

Before the bridge contract is proven, do not:

- merge repositories;
- switch databases;
- add a vector or graph database;
- make Executor a runtime dependency of the Ordo demo;
- allow Executor output to skip Ordo policy, evidence, review, or promotion.
