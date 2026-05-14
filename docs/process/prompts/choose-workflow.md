# Prompt: Choose Workflow

Use when you are not sure whether to research, execute, QA, land, or triage.
This is a routing prompt, not an implementation prompt.

```text
Choose the correct Ordo development workflow for the current request.

Do not edit files, commit, push, merge, close issues, or create issues unless
the chosen workflow explicitly requires it and the user asks to proceed.

Read:
- docs/process/agent-execution-protocol.md
- docs/process/definition-of-done.md
- docs/process/prompts/README.md
- latest git status
- current GitHub PR/issue state if the request mentions issues, PRs, milestone,
  QA, landing, stale work, or blockers.

Classify the request as one of:
- Research Batch
- Execute Issue
- QA Issue
- Land Issue
- Triage Issue Queue
- Docs/Architecture Planning
- Product Discussion
- Blocked/Needs Human Decision

Decision rules:
- If there is a QA-passed unmerged branch, recommend Land Issue before new work.
- If there is failed QA, recommend QA/fix before new work.
- If the manifest or issue queue is stale, recommend Research Batch or Triage
  Issue Queue.
- If the user asks to implement one issue and gates are clear, recommend
  Execute Issue.
- If the user asks to review latest work, recommend QA Issue.
- If the user asks about product direction or architecture before code,
  recommend Docs/Architecture Planning.
- If source-of-truth docs conflict or the worktree is unsafe, report the
  blocker.

Final response:
- recommended workflow;
- why;
- exact prompt file to use;
- required docs to read first;
- blockers or gates;
- one-sentence next action.
```
