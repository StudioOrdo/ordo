# Agent Execution Protocol

Status: required workflow for AI-assisted implementation

This protocol turns Ordo's docs, issues, tests, and GitHub state into a repeatable
development circuit. Raw model capability is useful only when routed through
clear source-of-truth order, evidence gates, and review states.

## Source Of Truth Order

When sources disagree, trust:

1. Current source code and tests.
2. Current GitHub issue, linked test-plan issue, milestone, PR, and latest
   Batch Execution Manifest.
3. Current product and architecture docs.
4. Process docs in this folder.
5. Prior issue comments and test-plan comments.
6. Local drafts or archived notes.

Never implement from memory when current code, docs, or GitHub state can answer
the question.

## Research Before Editing

Before editing code or docs:

- refresh git and GitHub state;
- inspect the current branch and dirty worktree;
- read the selected issue and linked test-plan issue;
- read the latest batch manifest when working from a milestone;
- read architecture docs named by the issue;
- inspect current code and tests for the touched area;
- identify unrelated dirty files and leave them alone.

If research shows the issue is stale, blocked, already satisfied, or missing a
test plan, comment with evidence and stop or select the next eligible issue.

## Issue Selection

For milestone execution:

- select only open implementation issues in the target milestone;
- do not select issues labeled `type:test` as primary;
- prefer the next issue in the Batch Execution Manifest;
- require a linked `Test Plan:` issue;
- skip blocked, waiting, duplicate, stale, or already satisfied issues with a
  GitHub comment explaining evidence.

If a previous QA or landing step left a failed QA, unmerged PR, blocker, or
local-only completed issue, resolve that gate before selecting new work.

## Scope Control

Implement exactly one scoped issue.

Do not broaden scope to nearby refactors, new surfaces, additional issue
cleanup, opportunistic architecture changes, or unrelated formatting.

If the issue cannot be completed in one focused coding session, split it into
smaller implementation and test-plan pairs before coding.

## TDD Expectations

Default order:

1. Add or update focused tests for the intended behavior.
2. Confirm tests fail for the right reason when practical.
3. Implement the smallest behavior needed.
4. Run focused validation.
5. Add broader tests only when the blast radius requires it.
6. Run required validation and `git diff --check`.

Docs-only work should still validate links, terms, and whitespace.

## Architecture Boundaries

Do not bypass:

- capability catalog and policy decisions;
- access grants and visibility ceilings;
- artifact provenance and approval state;
- event audit/replay;
- graph candidate/confirmed boundaries;
- job/DAG lifecycle and idempotency;
- projections/read models for surface experience;
- provider gateways and deterministic test fakes;
- pack permissions and uninstall boundaries.

Public/member surfaces must not leak staff routing, provider internals, prompt
internals, secrets, raw policy internals, owner-only data, private artifact text,
or unsupported claims.

## GitHub Comments

Implementation comment after local completion must include:

- branch;
- commit;
- files changed;
- acceptance criteria covered;
- validation run;
- residual risk;
- ready-for-QA state.

Test-plan comment must include:

- scenarios covered;
- unit/integration/E2E coverage;
- deterministic provider/mocking evidence;
- deferred coverage and rationale.

QA comment must include:

- reviewed commit;
- findings;
- commands run;
- fixes made if any;
- ready-to-land state.

Landing comment must include:

- PR URL;
- merge commit;
- validation and QA evidence;
- issue closeout or reason left open;
- next eligible issue.

## Validation Requirements

At minimum:

- focused tests for touched code;
- project typecheck/check when available;
- formatting/lint check when available;
- `git diff --check`;
- broader tests for shared schema, policy, auth, providers, rewards, access,
  jobs, artifacts, graph, projections, routes, navigation, or public/member UI.

Live providers, hosted infrastructure, real publishing, real image generation,
real TTS, real payments, and real analytics must be guarded and optional. The
default validation path must be deterministic.

## QA Behavior

QA is adversarial review, not confirmation.

QA must inspect the committed diff against its parent, verify acceptance
criteria, verify linked test-plan coverage, run validation, and check
architecture invariants. If a real defect exists, QA should make the smallest
scoped fix, adjust tests where practical, rerun validation, comment on GitHub,
and commit with:

```text
Issue #<number> QA: <short summary>
```

If no defect exists, QA should not change code.

## Landing Behavior

Only land QA-passed branches.

Before merge:

- confirm latest commit was validated;
- confirm worktree state is understood;
- confirm PR exists or create it;
- confirm GitHub reports no conflicts or blocked checks.

After merge:

- refresh `main`;
- confirm merge commit;
- comment on implementation and test-plan issues with merge-backed evidence;
- close issues only when the Definition Of Done allows it;
- update the batch manifest.

## Stale PR Behavior

If open PRs exist:

- list them before selecting new work;
- land QA-passed PRs before new implementation;
- report blocked/conflicted/stale PRs instead of stacking invisible local work;
- do not merge PRs with failed or unreviewed QA.

## Blocker Behavior

Stop and report a blocker when:

- a required linked test-plan issue is missing;
- source-of-truth docs contradict the selected issue;
- current code already satisfies the issue;
- validation requires unavailable secrets or live providers;
- unrelated dirty files would be overwritten;
- architecture requires bypassing a trust boundary;
- the requested work is too broad for one issue.

## No-Close Rule

Do not close implementation or test-plan issues from local-only evidence.
Close only after merge-backed evidence, unless the prompt explicitly asks for a
docs-only administrative close and the issue is clearly obsolete with evidence.
