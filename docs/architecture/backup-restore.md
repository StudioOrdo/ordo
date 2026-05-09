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

Backup manifests record SHA-256 checksum evidence as `sha256:v1:<hex>` and
include the checksum algorithm version. The database snapshot checksum is stored
in the manifest and artifact metadata. The manifest checksum is computed from a
normalized manifest payload with the self-referential manifest checksum field
cleared, then stored back in the manifest and artifact evidence.

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

Restore preflight verifies the backup manifest before any destructive restore
task can proceed. The reader rejects malformed JSON, unsupported manifest schema
versions, unsupported checksum algorithms or versions, database checksum
mismatches, manifest checksum mismatches, artifact manifest paths outside the
local backups boundary, and manifest-declared archive paths that escape their
backup archive.

## Local Vault Backup Boundary

The local appliance vault stores provider keys and future sensitive appliance
values encrypted in SQLite, unlocked by a `vault.key` file inside the durable
data boundary.

For the current backup model, local appliance backups include both encrypted
vault data in the SQLite snapshot and selected data-boundary sidecar files such
as `vault.key`. Restore preflight verifies the archived sidecar file checksums
alongside the database and manifest checksums. This preserves restore usability
without user-managed key recovery.

This also means backup archives should be protected like the host and `.data`
volume. The vault protects against casual database inspection and accidental
leakage; it does not protect against someone who has full access to both a
backup archive and the vault key material inside that archive. Future export
modes may add redacted or passphrase-protected backup variants for stronger
sharing boundaries.

## UI Contract

The Backup And Restore page should show jobs in a table with operation, kind,
status, task progress, current task, elapsed time, started time, completed time,
artifact, and actions.

Progress is task-based. Elapsed time may be displayed. ETA is not shown in
0.1.0.