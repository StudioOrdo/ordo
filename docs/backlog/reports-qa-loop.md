# Reports And QA Loop MVP

Status: backend foundation merged

## Why It Matters

Reports are the near-term product loop for developer trust, trial QA, and
support handoff.

## MVP Scope

- Add report detail view.
- Add copy/export markdown affordances.
- Add issue package preview.
- Track report status from draft to reviewed/exported/submitted.
- Prepare for optional GitHub/support submission without making it automatic.

## Backend Foundation

- Report detail contracts return the stored local report, export records, status
	events, and any derived support packets.
- Markdown exports are persisted as local export records with content hash and
	exact reviewed content.
- Report status transitions are durable events with actor and reason evidence.
- Support packet preview reuses local issue report content as a bounded,
	redacted, local-only packet draft.

## Durable Product Nouns

- Issue Report
- Evidence Envelope
- Report Status
- Export Record
- Submission Receipt

## Acceptance Criteria

- Operator can inspect complete local report evidence.
- Exported markdown matches reviewed local report content.
- Secrets remain redacted in reports and exports.
- Submission remains explicitly opt-in.
- External submission transports remain absent from the backend contract.

## Non-Goals

- Automatic GitHub issue creation in the MVP.
- Hidden telemetry.
- AI-generated unfalsifiable diagnoses.

## Validation

- Report rendering tests.
- Redaction tests.
- UI smoke for detail/copy/export paths.
