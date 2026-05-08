# Operation Kernel

Status: Draft contract for Ordo 0.1.0

The operation kernel is the reusable execution model for Ordo work.

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

A task is ready when all required dependencies have succeeded or been skipped
by an explicit policy.

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