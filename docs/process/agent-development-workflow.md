# Agent Development Workflow

Status: public operating model for AI-assisted development

Ordo is built with AI assistance, but not with unbounded agent autonomy.

Agents work inside a public manufacturing system:

```text
Research -> Execute -> QA -> Land
```

The goal is to make AI-assisted development visible, bounded, testable,
reviewable, and correctable.

## GitHub Objects

| Object | Role |
| --- | --- |
| Docs | Durable doctrine, architecture, process, and product truth. |
| Issue | Accepted manufacturing unit after triage. |
| Test-plan issue | Coverage and validation contract for an implementation issue. |
| Batch manifest | Priority, dependency, and eligibility contract for a milestone. |
| Branch | One scoped execution lane. |
| Commit | Local implementation checkpoint. |
| Pull request | Implementation evidence packet. |
| QA comment | Adversarial review evidence and residual risk. |
| Merge | Public proof boundary. |
| Issue closeout | Manufacturing record update. |

## Research Mode

Use Research after a landing step, product or docs shift, stale issue cleanup,
or when the manifest appears wrong.

Research should:

- refresh git and GitHub state;
- confirm latest `origin/main`;
- inspect open PRs and landing gates;
- read open milestone implementation issues and linked test plans;
- read the active batch manifest;
- read relevant product and architecture docs;
- inspect current source and tests around likely gaps;
- identify duplicates, stale issues, missing test plans, wrong priorities, and
  issues that no longer match the landed architecture;
- update or propose the next small executable batch.

Research does not implement code.

## Execute Mode

Execute implements exactly one scoped issue.

Before selecting work, the agent must:

- refresh git and GitHub state;
- read the active batch manifest;
- check whether QA or landing reported a failed gate, blocker, unmerged PR, or
  local-only completed issue;
- handle unresolved landing or QA gates before starting new work;
- select the next eligible open implementation issue in the target milestone;
- require a linked `Test Plan:` issue.

Execution should use TDD, focused validation, broader validation when risk
requires it, GitHub evidence comments, and a local commit. It should not broaden
scope.

## QA Mode

QA is adversarial senior review of the latest completed implementation branch.

QA should verify:

- acceptance criteria;
- test-plan coverage;
- positive, negative, and edge scenarios;
- architecture boundaries;
- security and privacy boundaries;
- deterministic validation;
- public/member surface leakage risks;
- formatting and diff hygiene;
- residual risks.

If a real defect exists, QA may make the smallest scoped fix, adjust tests where
practical, rerun validation, comment evidence, and commit with a QA-specific
message.

If no defect exists, QA should not make code changes.

## Land Mode

Landing turns QA-passed local work into merge-backed public evidence.

Before landing, confirm:

- current branch;
- latest commit;
- implementation issue;
- linked test-plan issue;
- active batch manifest;
- QA passed after the latest implementation or QA commit;
- worktree is clean except for intended branch content;
- no unrelated dirty files are involved;
- PR state and mergeability.

Do not merge if QA failed, checks are failing, conflicts exist, or latest work
has not been validated.

After merge, refresh `main`, comment merge-backed evidence on the
implementation and test-plan issues, close issues when appropriate, and update
the batch manifest with the next eligible issue.

## Boundaries

Agents must not:

- create commits, branches, PRs, pushes, issue comments, issue closes, or merges
  unless the user explicitly asks for that workflow;
- bypass capability, policy, visibility, access, artifact, audit, Growth,
  job/DAG, or projection boundaries;
- fake rewards, hosted capacity, providers, publishing, uptime, analytics,
  trial scarcity, AI capability, or pack execution;
- leak staff routing, provider internals, prompt internals, secrets, raw policy
  internals, owner-only data, private artifact text, or unsupported claims;
- use live providers without explicit guards, network intent, and budget caps.

## Why This Matters

The project is solo-developed and heavily AI-assisted. That is not a reason to
relax engineering discipline. It is why the discipline must be public.

The point is not to let AI write faster unchecked. The point is to prove that
independent developers can use AI to build serious software through visible
scope, evidence, tests, review, and merge-backed truth.