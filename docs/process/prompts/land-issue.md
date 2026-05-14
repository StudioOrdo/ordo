# Prompt: Land Issue

Use to merge the latest QA-passed branch.

```text
Land the latest QA-passed implementation branch.

Primary goal:
Create or update the PR, merge it properly on GitHub if clean, then update
issues and manifest with merge-backed evidence.

Process docs to follow:
- docs/process/agent-execution-protocol.md
- docs/process/definition-of-done.md

Before landing:
1. Refresh git and GitHub state.
2. Confirm current branch, latest commit, implementation issue, linked Test
   Plan issue, and Batch Execution Manifest.
3. Confirm QA passed after the latest implementation or QA commit.
4. Confirm the worktree state is understood and unrelated dirty files are not
   involved.
5. Confirm whether an open PR already exists for this branch.

Landing rules:
- Do not merge if QA failed or the latest commit has not been validated.
- Do not merge if GitHub reports conflicts or blocked checks.
- If checks are pending, wait or report blocker.
- If no PR exists, push branch and create a focused PR to main.
- If PR exists, update it as needed.
- Merge only when GitHub reports clean/mergeable and no required checks fail.
- Use the project normal merge strategy.
- Delete remote branch after merge if safe.

After merge:
- refresh main;
- confirm merge commit;
- comment on implementation issue with PR URL, merge commit, and validation
  evidence;
- close implementation issue if Definition Of Done allows it;
- comment on linked Test Plan issue with merge-backed coverage evidence;
- close linked Test Plan issue if coverage is complete;
- update Batch Execution Manifest with landed evidence and next eligible
  implementation issue.

Final response:
- PR created/updated;
- merge result and merge commit;
- issues closed/updated;
- manifest updated;
- current open PRs;
- next implementation issue.
```
