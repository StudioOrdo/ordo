# Prompt: QA Issue

Use to review the latest completed implementation slice.

```text
QA the latest completed implementation issue from the current branch and
milestone.

Primary goal:
Verify correctness, security, determinism, architecture alignment, and test
coverage. Treat this as adversarial senior review.

Process docs to follow:
- docs/process/agent-execution-protocol.md
- docs/process/definition-of-done.md
- docs/process/test-plan-template.md

Scope discovery:
1. Refresh git and GitHub state.
2. Identify current branch and latest commit.
3. Determine implementation issue from branch, commit, PR, and issue comments.
4. Determine linked Test Plan issue.
5. Read the latest Batch Execution Manifest.
6. Review only the completed implementation slice unless a defect requires a
   narrow fix.

Required reading:
- implementation issue;
- linked Test Plan issue;
- Batch Execution Manifest;
- architecture docs named by the issue;
- current source and tests touched by the implementation.

QA checklist:
- inspect committed diff against parent;
- verify acceptance criteria;
- verify linked test-plan coverage;
- verify architecture invariants;
- check correctness, security/privacy, idempotency, rollback/retry,
  schema/migration safety, event/projection/graph consistency, UI behavior,
  dependency cost, and test quality where relevant.

Validation:
- focused tests for touched code;
- typecheck/check command for touched project;
- formatting check for touched files;
- git diff --check;
- broader tests for shared schema, policy, auth, providers, rewards, access,
  jobs, artifacts, graph, projections, routes, or navigation.

Defect handling:
- If a real defect exists, make the smallest scoped fix, add/adjust tests where
  practical, rerun validation, comment on both issues, and commit with:
  Issue #<number> QA: <short summary>.
- If no defect exists, do not change code; comment with QA evidence and
  residual risk.

Do not close issues from local-only evidence.

Final response:
- QA result;
- issues reviewed;
- branch and commit reviewed;
- files reviewed;
- commands run and results;
- findings ordered by severity;
- fixes made if any;
- ready to land or blocked;
- whether issues are closable after merge.
```
