# Job Kernel V2 MVP

Status: not built

## Why It Matters

Long-running jobs, external handoffs, worker Ordos, and future AI workflows need
ownership, retry, cancel, and resume semantics instead of one-shot handlers.

## MVP Scope

- Add job/task leases and worker ownership.
- Add heartbeat and stale lease handling.
- Add cancel, retry, and resume state transitions.
- Add bounded failure reason and retry evidence.
- Keep process templates and task DAGs as the source of workflow shape.

## Durable Product Nouns

- Lease
- Worker Claim
- Heartbeat
- Retry Attempt
- Cancellation
- Resume Point

## Acceptance Criteria

- A worker can claim and release eligible work.
- Stale claims can be detected and recovered safely.
- Cancel/retry/resume actions are persisted and auditable.
- Existing backup/report/brief jobs still pass.

## Non-Goals

- Distributed worker fleet.
- Queue service dependency.
- Arbitrary code execution.

## Validation

- Kernel unit tests.
- Migration tests.
- Existing full Rust test suite.
