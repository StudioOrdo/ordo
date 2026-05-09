# Diagnostics And Reports

Status: Implemented local appliance slice

Diagnostics and Reports make Ordo's own operation inspectable. They are the
first concrete QA loop inside the appliance: Ordo records structured local
observations, prepares reviewable issue reports, and keeps evidence close to the
operator before any external submission exists.

## Current Surface

The System shell includes two implemented local surfaces:

- Logs: structured diagnostic observations from the local appliance.
- Reports: local issue reports and diagnostic packages prepared through the job
  kernel.

The daemon exposes:

- `GET /logs`
- `GET /reports/issues`
- `GET /reports/issues/:report_id`
- `PUT /reports/issues/:report_id/status`
- `POST /reports/issues/:report_id/exports`
- `POST /reports/issues/prepare`
- `GET /support-packets`
- `POST /support-packets`
- `PUT /support-packets/:packet_id/approve`
- `GET /support-packets/:packet_id/receipts`

The browser uses the bundled Next.js API route to request report preparation
from the daemon. Report preparation is a protected daemon mutation.

## Diagnostic Logs

Structured diagnostic logs are stored in SQLite. They are local operational
records, not remote telemetry.

Logs can include:

- level;
- source;
- message;
- job id;
- task key;
- capability id;
- event type;
- structured payload;
- timestamp.

Secret-like payload keys are redacted before storage. Query results are bounded
so the System UI can inspect recent observations without exposing the appliance
as an unbounded log export.

Provider API keys and local vault material must not appear in diagnostic log
payloads. API-key-shaped payload keys are redacted before storage.

## Issue Reports

Reports are local evidence packages.

Preparing a report:

1. creates an `issue.report.prepare` job;
2. runs the report task plan through the shared job/task kernel;
3. collects selected evidence from health, readiness, recent events, recent
   jobs, browser context, and diagnostic logs;
4. applies deterministic redaction policy;
5. stores a durable report artifact in SQLite;
6. renders markdown for operator review, copy, or export.

Reports are stored locally as artifacts. They are not automatically submitted to
GitHub, support systems, model providers, or other Ordos.

Report list reads return summary rows for queue rendering and selection. Full
markdown, diagnostics, collected evidence, redactions, exports, status events,
and support packet derivatives remain on the report detail contract.

Report detail reads include the stored artifact, local export records, status
events, and any support packet drafts derived from the report. Markdown exports
are durable local records with the exact reviewed markdown content and content
hash evidence. Status changes are stored as local status events.

Reports must not include plaintext provider keys or local vault key material.
Provider status may be summarized only through redacted presence/source
metadata.

Issue report job artifacts include provenance metadata for the current local
policy spine: actor, action, report resource, producing capability, producing
job, process template, and local high-trust classification.

## Support Packets

Support packets are local, approval-gated derivatives of issue reports. A draft
support packet previews bounded markdown content, destination metadata, payload
hash evidence, and `externalDelivery: false` before any future transport is
introduced.

Approving a packet records `approved_local_only` and a local receipt with
`deliveryState: not_sent`. Approval is evidence for a future egress gate, not a
network send. There is no daemon route in this slice that delivers a support
packet to Studio Ordo Support, GitHub, model providers, or other Ordos.

## Evidence Boundary

Reports are intentionally bounded. The current report path should include enough
evidence to help reproduce and triage a problem without dumping the whole
appliance.

The current implementation is local-first:

- no hidden network egress;
- no automatic external issue creation;
- no automatic A2A support submission;
- no support packet delivery route;
- no raw database upload;
- no unrestricted log export.

External submission is future work and should require operator confirmation.

## Why This Matters

Ordo's software manufacturing loop depends on QA. Diagnostics and Reports turn
user feedback into structured evidence that can become a GitHub issue, support
handoff, or future Ordo-to-Ordo support packet.

The same pattern should later apply to business workflows:

```text
observation -> evidence -> report -> review -> accepted work -> artifact
```

## Relationship To Future Work

Diagnostics and Reports are a foundation for:

- hosted trial QA;
- project health briefs;
- support handoff;
- A2A support issue packets;
- community issue clustering;
- operator-confirmed external submission.

Only the local preparation and inspection loop is implemented today.
