# Prompt: Research Batch

Use after a landing step, product/doc shift, stale issue cleanup need, or when
the manifest appears wrong.

```text
Research the current codebase, docs, GitHub milestone, PR state, and issue
state, then update the next executable batch.

Milestone:
<milestone title>

Process docs to follow:
- docs/process/agent-execution-protocol.md
- docs/process/definition-of-done.md
- docs/process/implementation-issue-template.md
- docs/process/test-plan-template.md

Do not implement product code.

Required steps:
1. Refresh git and GitHub state.
2. Confirm latest origin/main.
3. Read open PRs and identify landing gates.
4. Read open milestone implementation issues and linked Test Plan issues.
5. Read the latest Batch Execution Manifest.
6. Read relevant product, architecture, process, and eval docs.
7. Inspect current source/tests for the next likely gaps.
8. Identify duplicates, stale issues, missing test plans, wrong priorities, and
   issues that no longer match the landed architecture.
9. Update or create implementation/test-plan issue pairs using the templates.
10. Post a Batch Execution Manifest on the planning issue.

Manifest must include:
- milestone;
- latest main commit;
- open PRs and landing gates;
- docs read;
- code areas inspected;
- prioritized issue order;
- linked test-plan issue for each implementation issue;
- blockers/dependencies;
- expected branch name per issue;
- validation expectations;
- do-not-start-until warnings;
- next recommended implementation issue.

Rules:
- Prefer updating existing issues over creating duplicates.
- Split broad issues into one-session implementation/test-plan pairs.
- Do not close issues unless clearly obsolete and commented with evidence.
- Keep the batch small.
- Treat local-only completed work and unmerged PRs as blockers.

Final response:
- batch summary;
- issues created/updated;
- recommended stack order;
- blockers;
- first implementation issue for Execute.
```
