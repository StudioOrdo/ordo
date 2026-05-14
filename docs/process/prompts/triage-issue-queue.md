# Prompt: Triage Issue Queue

Use when the milestone has stale, duplicate, overbroad, or architecture-drifted
issues.

```text
Triage the GitHub issue queue for the target milestone against current source,
tests, docs, PR state, and product architecture.

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
2. Read open PRs and landing blockers.
3. Read open milestone issues, labels, comments, and linked Test Plan issues.
4. Read the current Batch Execution Manifest.
5. Read relevant product, architecture, process, and eval docs.
6. Inspect current code/tests for claimed behavior.
7. Classify each issue:
   - ready;
   - blocked;
   - missing test plan;
   - duplicate;
   - stale but salvageable;
   - obsolete;
   - too broad and needs split;
   - already satisfied by code.
8. Update issue bodies/comments with evidence.
9. Create missing test-plan issues only when an implementation issue remains
   valid and executable.
10. Recommend the next small batch.

Rules:
- Prefer updating existing issues over creating duplicates.
- Do not close issues unless clearly obsolete or already satisfied, and only
  after commenting with evidence.
- Do not invent implementation work not grounded in current docs/code.
- Keep each executable issue to one focused coding session.

Final response:
- number of open implementation issues remaining;
- number of test-plan issues remaining;
- issues updated/created/closed;
- stale/obsolete issue handling;
- blockers;
- recommended next batch and first issue.
```
