# Operator Simulator Contract

Status: Implemented contract for 0.1.4 Phase 8

The operator simulator creates realistic staff/operator pressure for workflow
evals. It drives handoff, delegation, tool-approval, review, and follow-up
paths without replacing deterministic authorization and evidence checks.

## Prompt Slot Purpose

Generate one redacted operator turn or action intent for a specific workflow
step:

- act like a business staff member operating Ordo;
- preserve client-safe surfaces;
- request evidence when unsure;
- delegate only with explicit scope;
- avoid hidden persuasion, fake proof, or unsupported publication;
- keep output redacted and bounded to the scenario.

## Required Coverage

Operator simulator scenarios should cover:

- accept handoff;
- reply publicly;
- delegate to Ordo;
- go idle;
- return to agent;
- decline handoff;
- approve or reject a tool request;
- correct assistant behavior;
- request evidence;
- publish or decline a review;
- manage offer/ask follow-up.

## Output Requirements

Operator output must use `ordo.eval_simulator_output.v1` from
`schema.md` with:

- `simulatorRole`: `operator`;
- `actorKind`: a staff-facing actor such as `staff_operator`,
  `manager_admin`, or `owner_system_admin`;
- `messageHash`: hash of the redacted operator message or action note;
- `redactedExcerpt`: safe operator excerpt;
- `expectedPressureSubsystem`: the subsystem under test;
- `evidenceRefs`: handoff, tool, review, offer, ask, or policy evidence where
  available;
- `deterministicAssertionRefs`: assertion ids that remain responsible for
  pass/fail.

`reviewerFindingCategories` must be empty for operator outputs.

## Non-Authority Boundary

Operator simulator output can request or attempt an action. It cannot grant
itself authority. The backend still decides:

- capability and policy authorization;
- queue and handoff visibility;
- delegation scope;
- tool approval state;
- review consent and approval;
- offer/ask outcome attribution;
- public versus staff-only visibility.
