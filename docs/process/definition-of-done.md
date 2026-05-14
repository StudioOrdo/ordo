# Definition Of Done

Status: process contract

This document defines completion language for Ordo implementation work. It is
designed to prevent over-eager completion claims and to keep every slice tied to
evidence, tests, GitHub state, and architecture boundaries.

## Completion States

### Local-Only Complete

Local-only complete means code or docs have been changed and validated locally,
but the work has not passed independent QA and has not landed on `main`.

Local-only complete is not done. Do not close implementation or test-plan issues
from this state.

Required evidence:

- branch name;
- commit hash when code was committed;
- files changed;
- focused validation run;
- broader validation run when relevant;
- implementation issue comment;
- linked test-plan issue comment.

### QA-Passed

QA-passed means a separate QA pass inspected the committed diff, verified the
linked test-plan issue, ran relevant validation, and found no blocking defect,
or committed a narrow QA fix and revalidated it.

QA-passed is ready to land, but it is not landed. Do not close issues from
QA-passed state unless the issue is docs-only and explicitly does not require a
merge-backed closeout.

Required evidence:

- reviewed branch and commit;
- implementation issue number;
- linked test-plan issue number;
- files reviewed;
- findings ordered by severity;
- commands run and results;
- residual risk;
- explicit "ready to land" or "not ready to land" statement.

### Landed

Landed means the QA-passed branch has been merged to `main` on GitHub and local
`main` has been refreshed to include the merge commit.

Required evidence:

- PR URL;
- merge commit;
- validation evidence from implementation and QA;
- implementation issue closeout comment;
- test-plan issue closeout comment;
- batch manifest update when the issue belongs to a batch.

### Closable

An implementation issue is closable when:

- acceptance criteria are met;
- linked test-plan coverage is complete or explicitly deferred with rationale;
- QA passed after the latest implementation or QA-fix commit;
- PR is merged;
- merge-backed evidence is posted to the issue.

A test-plan issue is closable when:

- positive, negative, edge, and privacy/security scenarios are covered or
  explicitly deferred;
- required unit, integration, and E2E/smoke coverage exists or has a documented
  deferral;
- validation evidence is posted after merge;
- no known rollout-only test gap remains.

### Not Closable

Do not close issues when:

- work is only local;
- QA has not reviewed the latest commit;
- PR is open, blocked, pending, conflicted, or unmerged;
- linked test-plan issue is missing;
- acceptance criteria changed without updating the issue;
- validation was skipped without rationale;
- behavior is only mocked while the issue claimed durable behavior;
- the implementation relies on live providers without a guarded test mode;
- evidence comments are missing.

## Required Validation

Every implementation slice must run:

- focused tests for touched code;
- typecheck/check command for the touched project when available;
- formatting/lint check for touched files when available;
- `git diff --check`;
- broader tests when touching shared schema, policy, auth, providers, rewards,
  access, jobs, artifacts, graph, projections, routes, navigation, or public
  surfaces.

Docs-only slices must run at least `git diff --check` and link-check or grep
validation when references are changed.

## Required Evidence Comments

Implementation issue comment:

```text
Implementation evidence:
- Branch:
- Commit:
- Files changed:
- Acceptance criteria covered:
- Validation:
- Residual risk:
- Ready for QA:
```

Test-plan issue comment:

```text
Coverage evidence:
- Positive:
- Negative:
- Edge:
- Privacy/security:
- Unit:
- Integration:
- E2E/smoke or deferral:
- Validation:
- Remaining rollout gaps:
```

Landing comment:

```text
Landed evidence:
- PR:
- Merge commit:
- Validation:
- QA:
- Issues closed or left open:
- Next issue:
```

## Stop And Report A Blocker

Stop instead of improvising when:

- source-of-truth docs conflict with current code in a way that changes scope;
- linked test-plan issue is missing;
- issue is blocked or marked waiting;
- GitHub has an unmerged QA-passed PR that should land first;
- worktree changes unrelated to the task would be overwritten;
- validation requires secrets or live providers not available in guarded mode;
- architecture would require bypassing policy, access, artifact, visibility,
  audit, graph, job/DAG, or projection boundaries;
- the implementation cannot be completed in one focused coding session.

## Non-Negotiable Rule

Do not claim completion without evidence. A slice is not done because it looks
right, compiles once, or satisfies a happy path. It is done only when the state
above matches the evidence posted in GitHub and the merged code.
