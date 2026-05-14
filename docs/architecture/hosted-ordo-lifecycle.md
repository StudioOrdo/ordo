# Hosted Ordo Lifecycle

Status: MVP architecture direction

Hosted trial lifecycle should run through Ordo's job/task/event/artifact spine.
Commissioning and decommissioning are not ad hoc shell actions. They are
process templates that produce durable evidence.

## Lifecycle Jobs

### `hosted_ordo.commission`

Purpose: turn an accepted trial offer into a reachable hosted appliance.

```text
validate offer acceptance and hosted slot
-> allocate hostname
-> create data/media volume
-> start container from shared image
-> attach Traefik route
-> poll /health and /ready
-> create provisioning receipt artifact
-> send provisioned email
```

### `conversation.rollup`

Purpose: turn conversations into Growth-ready artifacts on a schedule.

```text
select eligible conversation activity
-> create transcript or bounded summary artifact
-> extract questions, offers, asks, blockers, and next actions
-> update Growth brief inputs
-> emit rollup event
```

Messages stay in conversation tables. Rollup artifacts summarize a time window
or milestone so Growth can reason without treating every message as a separate
business artifact.

### `trial.reminder.send`

Purpose: send lifecycle reminders with idempotent attempts and receipts.

Reminder moments include:

- welcome and provisioned;
- onboarding not started;
- onboarding incomplete;
- midpoint feedback ask;
- expiring in seven days;
- expiring in two days;
- expired and backup pending;
- backup ready and return invitation.

### `trial.closeout`

Purpose: prepare a humane exit before decommissioning.

```text
freeze final conversation rollups
-> run final backup
-> verify manifest and checksum evidence
-> create return invitation
-> send backup-ready email
-> mark decommission eligible after grace policy
```

### `hosted_ordo.decommission`

Purpose: remove the hosted runtime only after closeout is safe.

```text
confirm backup/export evidence
-> confirm grace period or owner override
-> stop container
-> detach route
-> preserve or archive volume according to policy
-> record decommission receipt
```

## Invariants

- A trial is not decommissioned before backup/export evidence exists.
- Email attempts are recorded separately from business events.
- Reminders are idempotent by trial, policy key, and scheduled window.
- Provisioning and decommissioning produce artifacts or receipts.
- Owner overrides are explicit events, not silent state changes.
- Trial extension must eventually flow through reward or benefit evidence, not
  direct timestamp edits.

## MVP Inputs

The current code already provides offers, trials, hosted slots, waitlist, reset
guards, resource grants, jobs, artifacts, backups, scheduler foundations, and
conversation data. The lifecycle MVP adds the missing orchestration records,
process templates, notification records, and control-plane UI.

## Non-Goals

- production uptime SLA;
- multi-region hosting;
- payment processing;
- cash affiliate payout;
- arbitrary custom Docker images per trial;
- automatic destructive wipe without closeout evidence.