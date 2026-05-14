# MVP Landing Plan

Status: planning guide

The active landing target is Studio Ordo hosted appliance management. Keep code
changes small and evidence-backed, but aim every slice at the trial lifecycle.

## Landing Sequence

1. Document the MVP and current/future split.
2. Add hosted instance records and read models.
3. Add commissioning/decommissioning process templates without real Docker side
   effects.
4. Add Docker/Traefik adapter behind a local-only guarded capability.
5. Add notification policy, schedules, attempts, and receipts.
6. Add trial reminder schedules.
7. Add conversation rollup artifacts for Growth briefs.
8. Add closeout backup and return invitation flow.
9. Add owner/staff UI for capacity, slots, waitlist, instances, reminders, and
   closeout evidence.
10. Add A2A Studio Ordo Prime feedback/support wedge.

## Slice Rule

Every slice should answer four questions:

```text
What durable product noun did this add or complete?
Which hosted trial lifecycle step moved forward?
What evidence proves it happened?
What remains explicitly future?
```

## Validation Bias

Use deterministic tests for state machines, policy, idempotency, and artifacts.
Use Docker/Traefik runtime proof only when the slice actually touches the
container boundary.