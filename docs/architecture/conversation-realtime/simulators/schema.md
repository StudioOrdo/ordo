# Eval Simulator Output Schema

Status: Implemented contract for 0.1.4 Phase 8

Schema version: `ordo.eval_simulator_output.v1`

Simulator outputs are test-driver artifacts for replay and live workflow evals.
They are not product truth and they are not pass/fail authority. Deterministic
backend assertions, durable rows, events, ledgers, policy decisions, privacy
transforms, token accounting, and redacted artifact packets remain the source of
truth.

## Roles

Allowed simulator roles:

- `customer`
- `operator`
- `reviewer`

Unknown roles are rejected.

## Output Shape

Every simulator output must serialize as JSON with:

- `schemaVersion`: must be `ordo.eval_simulator_output.v1`.
- `simulatorRole`: one of the allowed roles.
- `scenarioId`: stable workflow or eval scenario id.
- `turnId`: stable turn id within the scenario.
- `actorKind`: role-specific actor kind such as `anonymous_visitor`,
  `staff_operator`, or `redacted_artifact_reviewer`.
- `intentLabel`: short scenario intent label.
- `messageHash`: `sha256:` prefixed hash of the redacted message or reviewer
  note.
- `redactedExcerpt`: safe excerpt that contains no raw secrets, private terms,
  emails, phone numbers, provider payloads, or staff-only internals.
- `expectedPressureSubsystem`: one allowed pressure subsystem.
- `safetyConstraints`: non-empty constraints the simulator must obey.
- `evidenceRefs`: durable evidence refs when available.
- `artifactRefs`: redacted artifact refs when available.
- `deterministicAssertionRefs`: deterministic assertion ids that own pass/fail
  expectations.
- `reviewerFindingCategories`: known artifact-review categories for reviewer
  outputs only.
- `generatedAt`: timestamp for the simulator output.
- `source`: source label such as `deterministic_fixture`, `replay_fixture`, or
  `guarded_live_provider`.

The schema intentionally has no `passed`, `failed`, `score`, or authority flag.
Unknown fields are rejected by the Rust validator.

## Pressure Subsystems

Allowed `expectedPressureSubsystem` values:

- `privacy`
- `policy`
- `handoff`
- `delegation`
- `feedback_review`
- `home_about`
- `offer_ask`
- `accounting_budget`
- `provider`
- `artifact_review`
- `simulator_fixture`

## Reviewer Categories

Reviewer outputs must use the same categories as
`ordo.eval_artifact_review.v1`:

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

Reviewer categories are candidates for investigation. They do not file issues
automatically and they do not approve code changes.

## Safety Rules

Simulator inputs and outputs must be redacted. They must not contain:

- provider API keys or bearer tokens;
- raw private prompt text;
- raw customer email addresses or phone numbers;
- configured private business terms;
- unredacted staff-only notes;
- raw provider request or response bodies.

The validator rejects obvious secrets, email addresses, phone numbers, and
configured private terms. Future simulator runners must keep the same rejection
boundary before packets, scorecards, logs, or review artifacts are written.
