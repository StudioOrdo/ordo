# Backup And Restore

Status: Draft contract for Ordo 0.1.0

Backup and restore are the first concrete safety jobs on the operation kernel.
They must not become bespoke flows outside the job/task/event model.

## Backup Job

`backup.create` should include tasks similar to:

1. Check data boundary.
2. Acquire backup lock.
3. Snapshot SQLite.
4. Scan files.
5. Write archive.
6. Write manifest.
7. Verify integrity.
8. Record backup.

Some tasks may run independently when safe, but the job must preserve explicit
dependencies.

## Restore Job

`restore.execute` should include tasks similar to:

1. Validate restore request.
2. Verify backup archive.
3. Require confirmation.
4. Create safety backup.
5. Acquire restore lock.
6. Restore SQLite.
7. Restore files.
8. Verify restored state.
9. Restart app if needed.
10. Record restore.

Restore must be confirmation-gated and must create a safety backup before
changing live data.

## UI Contract

The Backup And Restore page should show jobs in a table with operation, kind,
status, task progress, current task, elapsed time, started time, completed time,
artifact, and actions.

Progress is task-based. Elapsed time may be displayed. ETA is not shown in
0.1.0.