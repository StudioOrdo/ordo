# Diagnostics And Reports Runtime Proof 1.0

Date: 2026-05-08

This note records the container runtime proof for the local Diagnostics And
Reports 1.0 vertical slice.

## Runtime Setup

- Compose project: `ordo_diag_reports_runtime`
- Image/runtime: repository `compose.yaml` and `Dockerfile`
- Disposable state: Compose project volume, removed after proof
- Host UI port: `127.0.0.1:3100`
- Host daemon port: `127.0.0.1:17761`
- Container ports remained the appliance defaults: UI `3000`, daemon `17760`
- A disposable daemon access token was set for direct host verification of
  protected daemon mutation routes.

## Daemon Endpoint Evidence

The containerized daemon was verified through these routes:

- `GET /health` returned `ok` for `ordo-daemon`.
- `GET /ready` returned `ready` with SQLite required tables present and the
  Next.js child process running.
- `GET /logs?limit=20` returned recent structured diagnostic logs.
- `GET /reports/issues` returned locally prepared report artifacts.
- `POST /reports/issues/prepare` created a ready-for-review local report
  artifact through the containerized daemon.

Direct daemon report evidence after POST:

- Report: `report_58472e12-30df-4448-8ed9-02d90fc04fce`
- Job: `job_1b5ca355-2309-45b0-bd70-d9c76ee5a968`
- Status: `ready_for_review`
- Severity: `high`
- Evidence sources present: `health`, `readiness`, `recent_events`,
  `recent_jobs`, `diagnostic_logs`
- Markdown included matching Diagnostics Summary lines for all five evidence
  sources.

## Browser Evidence

The System shell was opened from the containerized UI at
`http://127.0.0.1:3100`.

- `/logs` rendered the real daemon log table with report/job entries, including
  `Issue report prepared.`, `Job event job.succeeded`, and correlated job IDs.
- `/reports` rendered the report queue, latest markdown report preview, and the
  Evidence Checklist.
- A report was prepared through the browser UI, not only by direct API call.

Browser-created report evidence:

- Report: `report_d22913fa-8249-4145-8314-fdc1254f57b6`
- Job: `job_e6a39196-ccc9-4304-bc2b-65f2cde32532`
- Title: `UI runtime proof report`
- Status: `ready_for_review`
- Severity: `high`
- Evidence checklist displayed `health`, `readiness`, `recent_events`,
  `recent_jobs`, and `diagnostic_logs` as succeeded.
- Latest markdown preview included Diagnostics Summary and Evidence sections for
  all five required evidence sources.

## Validation

Final validation for this slice:

```bash
npm run typecheck
npm run build
npm run smoke:ui
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

## Boundary

Reports 1.0 prepares and stores local evidence packages. External submission
transports remain future operator-confirmed actions.
