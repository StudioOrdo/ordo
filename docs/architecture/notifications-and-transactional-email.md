# Notifications And Transactional Email

Status: MVP architecture direction

Transactional email is the first external delivery system the hosted MVP needs.
It should be event-driven, idempotent, and evidence-backed. The current repo has
simulated email/link artifacts in evals, but real outbound email is not shipped.

## Shape

```text
domain event
-> notification policy
-> notification schedule
-> delivery attempt
-> provider response
-> receipt artifact/event
```

Email is not the source of truth. It is a delivery projection of state already
recorded in SQLite.

## Required Emails

- waitlist confirmation;
- trial accepted;
- hosted Ordo provisioned;
- onboarding not started;
- onboarding incomplete;
- midpoint feedback ask;
- referral or feedback extension decision;
- trial expiring soon;
- trial expired and backup pending;
- final backup ready;
- return invitation;
- support or owner handoff acknowledgement.

## Notification Records

The durable model should separate policy, schedule, attempt, and receipt:

- notification policy: what should be sent and when;
- notification schedule: the specific due instance for a subject;
- delivery attempt: provider, template, idempotency key, status, error;
- receipt: provider message id, accepted/rejected/bounced/opened data when
  available and policy-safe.

## Idempotency

Every outbound message needs an idempotency key. Example:

```text
trial:<trial_id>:expiring_7_days:<yyyy-mm-dd>
```

Retrying a failed attempt should not create a second logical reminder. It should
append attempt evidence under the same scheduled notification.

## Content Rules

- Do not put secrets or raw private transcripts in email.
- Link to authenticated or capability-scoped surfaces when private data is
  involved.
- Backup-ready email may include a download link only when the link is scoped,
  expiring, and recorded.
- Waitlist and reminder emails should be plain, useful, and easy to understand.
- Every email should correspond to an Ordo event or artifact.

## Current Gap

Real outbound email, provider configuration, suppression lists, bounce handling,
unsubscribe/preferences, and delivery webhooks remain future work. The MVP can
start with one transactional provider and a narrow template set.