# Reviewer Simulator Contract

Status: Implemented contract for 0.1.4 Phase 8

The reviewer simulator inspects redacted eval artifacts and proposes candidate
investigation targets. It does not inspect raw transcripts, does not file
issues automatically, and does not decide pass/fail.

## Prompt Slot Purpose

Review redacted artifact packets, scorecards, and artifact-review output:

- classify candidate findings into the Ordo artifact-review taxonomy;
- cite artifact refs and evidence refs;
- suggest smallest responsible subsystem;
- explain investigation targets in safe language;
- avoid direct code edits as authoritative instruction;
- never request raw private transcripts, provider payloads, or secrets.

## Allowed Inputs

Reviewer simulator input is limited to redacted artifacts:

- `ordo.eval_artifact_packet.v1` packet JSON;
- scorecard JSON;
- manifest JSON;
- `ordo.eval_artifact_review.v1` review JSON;
- `artifact-review.md`;
- redacted transcript or timeline excerpts when available.

The reviewer must not receive raw provider prompts, raw provider responses,
privacy placeholder maps, API keys, or unredacted customer/staff transcripts.

## Output Requirements

Reviewer output must use `ordo.eval_simulator_output.v1` from
`schema.md` with:

- `simulatorRole`: `reviewer`;
- `actorKind`: `redacted_artifact_reviewer` or equivalent;
- `messageHash`: hash of the redacted review note;
- `redactedExcerpt`: safe excerpt of the review conclusion;
- `expectedPressureSubsystem`: usually `artifact_review`, or the smallest
  subsystem being examined;
- `artifactRefs`: redacted packet/review artifacts inspected;
- `deterministicAssertionRefs`: assertions that own pass/fail;
- `reviewerFindingCategories`: one or more known
  `ordo.eval_artifact_review.v1` categories.

## Finding Categories

Reviewer findings are limited to:

- `schema_gap`
- `event_gap`
- `policy_gap`
- `privacy_gap`
- `prompt_gap`
- `handoff_gap`
- `analysis_gap`
- `accounting_gap`
- `ux_contract_gap`
- `provider_gap`
- `test_fixture_gap`

Unknown categories are rejected by the Rust schema validator.

## Non-Authority Boundary

Reviewer simulator output is a candidate pressure signal. It cannot:

- mark a deterministic eval passed or failed;
- approve a PR;
- file a GitHub issue without a governed filing path;
- request raw transcripts or secrets;
- turn a finding into accepted work without review.
