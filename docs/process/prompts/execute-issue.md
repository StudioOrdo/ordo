# Prompt: Execute Issue

Use to implement exactly one scoped issue.

```text
Execute the next highest-priority open implementation issue from the current
Batch Execution Manifest.

Milestone:
<milestone title>

Primary goal:
Implement exactly one scoped issue using TDD, validation, GitHub comments, and
a local commit. Do not broaden scope.

Process docs to follow:
- docs/process/agent-execution-protocol.md
- docs/process/definition-of-done.md
- docs/process/implementation-issue-template.md
- docs/process/test-plan-template.md
- docs/process/nyc-demo-product-shell-lane.md when working in milestone
  `0.1.9 OrdoStudio NYC Pilot Foundations`

Before selecting work:
1. Refresh git and GitHub state.
2. Read the latest Batch Execution Manifest.
3. Check for failed QA, unresolved blocker, unmerged PR, or local-only
   completed issue.
4. Resolve any QA/landing gate before selecting new work.
5. Otherwise select the next eligible open implementation issue.

Issue selection:
- work only on open implementation issues in the milestone;
- do not select issues labeled type:test as primary;
- require a linked Test Plan issue;
- skip and comment if blocked, waiting, missing test plan, stale, duplicate, or
  already satisfied.

Required reading:
- selected implementation issue;
- linked Test Plan issue;
- Batch Execution Manifest;
- process docs listed above;
- architecture docs named by the issue;
- current source and tests.

Architecture gates:
- preserve canonical/event/graph/projection/access/artifact/job/DAG/pack/workflow
  boundaries;
- stay in the StudioOrdo/ordo product shell lane for NYC demo work;
- do not fake providers, analytics, rewards, publishing, graph certainty, task
  execution, AI capability, hosted capacity, or pack execution;
- do not merge or import Executor, add a new database/vector store, build full
  pack assurance, or rewrite the Request kernel during NYC demo slices;
- do not leak staff routing, provider internals, prompt internals, secrets, raw
  policy internals, owner-only data, private artifact text, compiled-plan
  inputs, task private payloads, graph certainty, or unsupported claims.

Git workflow:
- start from updated main unless manifest requires a stack branch;
- create/reuse branch: codex/issue-<number>-<short-slug>;
- commit with: Issue #<number>: <short summary>.

Required completion:
- TDD where practical;
- focused tests;
- relevant broader validation;
- formatting/typecheck/check as appropriate;
- git diff --check;
- implementation issue evidence comment;
- test-plan issue coverage comment.

Do not create a PR or close issues unless resolving an explicit landing gate.

Final response:
- issue implemented;
- branch and commit;
- files changed;
- validation run;
- GitHub comments posted;
- residual risks;
- ready for QA.
```
