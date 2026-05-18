# NYC Meetup Current Batch Handoff

Status: handoff note for work after the current batch

This note records the end-state direction for the NYC meetup lane so the next
agent or developer can continue without re-deriving the product shape.

## Current Product-Shell Chain

The NYC product-shell implementation chain has landed:

1. Story Intake creates workflow compilation evidence.
2. Studio Preview consumes workflow state.
3. Tracked public entry creates an idempotent relationship handoff.
4. Studio Publications exposes memory promotion readiness without promotion.
5. NYC validation baseline was restored.
6. Real E2E demo rehearsal produced UI fixes.
7. Non-technical member/operator UX hardening landed.

Current follow-up work is focused on demo credibility, not new architecture.

## Active Batch

The active support credibility slice is:

```text
#435 - Support: Wire staff handoff surface to daemon-backed queue projection
```

The intended end state for this slice:

- `/staff/handoffs` reads the daemon-backed support queue projection.
- Staff see open, claimed, and resolved handoff state safely.
- Claim actions are governed by `support.accept_handoff`.
- Public/member views do not expose staff routing or internal context.
- The support surface is credible for the NYC meetup demo.

Do not broaden this slice into canonical Request storage, support CRM, external
delivery, provider calls, graph mutation, memory promotion, pack assurance, or
Executor import.

## Product North Star

Use this operating model for future work:

```text
Attention -> Decision -> Governed Action -> Evidence -> Receipt
```

And this runtime rule:

```text
Daemon notices.
Requests route.
Capabilities authorize.
Offers grant.
Packs constrain.
Jobs execute.
Artifacts prove.
Graph explains.
Knowledge promotes.
Humans decide.
System protects.
```

The canonical product surfaces are:

- My Ordo: personal attention and action cockpit.
- Support: global handoff and realtime relationship work.
- Studio: production DAGs, artifacts, offers, workflows, previews, approvals.
- Knowledge: sources, packs, graph/memory candidates, review, promotion.
- Growth: owner/admin briefings, metrics, referrals, next actions.
- System: appliance safety, providers, permissions, audit, backup, policy.

## Next Work After #435

Recommended issue-sized follow-ups:

1. Architecture: Adopt Ordo product operating model.
   - Link new model docs from product canon and architecture index.
   - Reconcile old issue-history language that predates the NYC product-shell
     lane.

2. UX: Make My Ordo the member attention cockpit.
   - `/my` should answer what needs attention now.
   - Requests, Offers, Capabilities, and Chat should use plain language.
   - Existing source-specific work should project into member-safe items.

3. Support: Complete handoff detail and realtime conversation readiness.
   - Claim is not enough for full support credibility.
   - The next step is the safe detail view and conversation state.

4. Knowledge: Seed a governed demo memory readiness path.
   - The Publications readiness UI exists.
   - The demo needs an approved candidate path without memory promotion.

5. Live LLM: Fix standalone live LLM eval FK readiness failure.
   - Governed chat provider smoke has worked.
   - The standalone eval path still needs database readiness cleanup.

6. Architecture: Define Executor bridge contract.
   - Keep `ordo_executor` as donor/foundry.
   - Define import/export contracts before any code movement.

## What Not To Touch Yet

- Do not merge `ordo_executor`.
- Do not switch databases.
- Do not add a vector or graph database.
- Do not rewrite canonical Request storage as part of support UI.
- Do not build full pack assurance, consensus, or operational reviewer
  personas.
- Do not auto-promote graph or memory objects.
- Do not publish externally.
- Do not call providers from UI polish or support queue work.

## Meetup Readiness Definition

The NYC meetup path is credible when a demo attendee can see:

```text
public story path
-> Studio workflow state
-> public tracked-entry handoff
-> staff support queue
-> memory readiness without promotion
-> evidence-safe copy
-> local appliance trust boundary
```

The demo should make clear:

- what Ordo is doing;
- what is safe to click;
- what still needs a human;
- what is only readiness or draft;
- what evidence exists;
- what is intentionally not automatic.
