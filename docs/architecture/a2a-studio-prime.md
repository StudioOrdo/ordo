# A2A Studio Ordo Prime

Status: launch wedge direction, future implementation

Studio Ordo Prime is the first governed agent-to-agent relationship every
hosted Ordo should know before launch. It is not a raw backdoor into customer
data. It is a bounded support, feedback, directory, and premium-job projection.

## Launch Purpose

Before broad public launch, hosted Ordos should be able to submit structured
feedback and support evidence back to Studio Ordo. This primes the future
network while solving immediate launch needs:

- collect QA feedback;
- receive support packets;
- request trial extension review;
- report hosted health summaries;
- submit backup-ready receipts;
- request premium production jobs;
- opt into directory candidacy later.

## Relationship Shape

```text
hosted Ordo
-> scoped A2A connection grant
-> Studio Ordo Prime
-> receipt, response artifact, or support outcome
```

The local hosted Ordo still owns its truth. Studio Ordo Prime receives only the
approved projection or artifact packet.

## Initial Capabilities

- `feedback.submit`
- `support_packet.submit`
- `trial_extension.request`
- `hosted_health.summarize`
- `backup_ready.receipt`
- `premium_job.request`
- `directory_candidate.submit`

Each capability needs schema validation, policy, visibility rules, receipt
records, and replayable events.

## Trust Rules

- No hidden egress.
- No remote mutation of canonical local truth.
- No full database access.
- No raw private transcript upload without explicit artifact scope.
- No directory listing without opt-in.
- No premium job without owner approval and credit/token policy.

## Relation To The A2A Roadmap

The general A2A roadmap starts with support packet exchange. Studio Ordo Prime
narrows that into the hosted launch wedge: feedback, support, lifecycle
receipts, and premium job requests between a hosted trial Ordo and Studio Ordo.