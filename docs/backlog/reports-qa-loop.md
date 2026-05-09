# Reports And QA Loop MVP

Status: local reports exist

## Why It Matters

Reports are the near-term product loop for developer trust, trial QA, and
support handoff.

## MVP Scope

- Add report detail view.
- Add copy/export markdown affordances.
- Add issue package preview.
- Track report status from draft to reviewed/exported/submitted.
- Prepare for optional GitHub/support submission without making it automatic.

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

## Non-Goals

- Automatic GitHub issue creation in the MVP.
- Hidden telemetry.
- AI-generated unfalsifiable diagnoses.

## Validation

- Report rendering tests.
- Redaction tests.
- UI smoke for detail/copy/export paths.
