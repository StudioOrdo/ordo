# Operation Kernel

Status: Draft contract for Ordo 0.1.0

The operation kernel is the reusable execution model for Ordo work.

Current product direction: this kernel should evolve into a small durable job
runtime with enterprise-grade discipline inside the appliance. It should not
become a generic workflow-builder UI. Users direct work conversationally; the
kernel enforces plans, events, leases, retries, requests, artifacts, and
outcomes.

## Vocabulary

| Term | Meaning |
| --- | --- |
| Process Template | Reusable plan made of registered task kinds and dependencies. |
| Job | One run of a process template. |
| Task | One node in the job DAG. |
| Event | Durable fact emitted by a job or task. |
| Artifact | Durable output or evidence created by a task. |

User-facing language should use Process, Job, and Task. Engineering docs may
refer to the task graph as a DAG.

## Deterministic Work Contract

The task DAG is the work contract. It should make execution inspectable without
requiring an LLM to remember the plan.

The DAG answers:

- what work is allowed;
- what task can run now;
- what is blocked;
- what requires a person;
- what evidence was produced;
- what can be retried, skipped, paused, canceled, or resumed;
- what artifacts or requests should surface next.

LLMs may help parse intent, draft task content, review outputs, or summarize
state. The operation kernel owns task state, dependencies, leases, retries,
requests, artifacts, and events.

## Job Shape

Jobs store the exact task plan copied from the template at run start. Later
template edits must not rewrite history for existing jobs.

Minimum job fields:

- id;
- template id and version;
- kind;
- status;
- actor and origin, such as user, chat, scheduler, or system;
- started, completed, and updated timestamps;
- elapsed time;
- current task key;
- completed required task count;
- total required task count;
- failure summary when present.

## Task DAG

Each task has a key, label, kind, required flag, dependencies, input payload,
status, timestamps, attempts, and output or error metadata.

Supported task statuses for 0.1.0:

- pending;
- ready;
- running;
- waiting_for_input;
- blocked;
- succeeded;
- failed;
- skipped;
- canceled.

Future V2 task execution should add:

- lease owner and lease expiration;
- idempotency key;
- executor target;
- retry policy snapshot;
- cancellation reason;
- structured result envelope;
- artifact refs;
- evidence refs;
- metrics and limitations.

A task is ready when all required dependencies have succeeded or been skipped
by an explicit policy.

Human requests are DAG gates, not ad hoc interruptions. A workflow should create
a request when it needs approval, consent, missing information, QA, rights
verification, entity resolution, artifact review, or publication judgment. The
request decision should emit events and unblock, redirect, skip, or cancel
downstream tasks according to policy.

## Progress

Progress is derived from task completion, not wall-clock time.

```text
completed required tasks / total required tasks
```

The UI may show elapsed time as evidence. It must not promise an ETA in 0.1.0.
Future estimates may use historical runs, but estimates are advisory and never
the source of progress truth.

## Concrete 0.1.0 Jobs

- `brief.system.generate` writes the durable System Brief.
- `backup.create` creates a backup artifact and manifest.
- `restore.execute` restores from a backup produced by this appliance.
- `system.health.check` records appliance health evidence.

These jobs prove the kernel before broader product workflows are added.

## CQRS-Lite Direction

The kernel should preserve a clean split:

```text
Command -> canonical mutation -> event -> projection
```

Jobs and tasks are canonical execution records. Surface worklists, Studio run
views, Support queues, Growth dashboards, and Systems briefs are projections.
They should be rebuildable from canonical state and events.
