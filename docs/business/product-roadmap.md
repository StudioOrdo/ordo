# Product Roadmap

Status: Roadmap direction, not fully implemented

Current canon: [Current Product Canon](current-product-canon.md)

This roadmap records the product shape Ordo is marching toward. It is not a
release commitment and does not claim current implementation.

The roadmap should be workflow-driven. A slice is valuable when it adds a
durable product noun, completes a step in a north-star workflow, and provides
evidence through tests, docs, and bounded runtime behavior.

Architecture doctrine: use
[Appliance Operating Discipline](../architecture/appliance-operating-discipline.md).
Borrow enterprise operating patterns where they make Ordo safer and more
reliable, then implement them as local appliance machinery. Use the
[Target Architecture Plan](../architecture/target-architecture-plan.md) for the
Clean/CQRS-lite implementation shape and slice sequence. Use
[Rewards And Incentives](../architecture/rewards-and-incentives.md) for
referral, feedback, credit, benefit-grant, and leaderboard architecture.

## North Star Loop

The canonical product surfaces are:

```text
Member View
Studio
Support
Knowledge
Growth
Systems
```

The product loop is:

```text
Install Ordo
-> seed approved business truth
-> define visibility and availability
-> publish About, Offers, Asks, and Feed
-> share tracked entry points
-> let visitors talk to Ordo
-> qualify intent
-> offer trial, ask, handoff, or connection
-> brief the owner instead of creating noise
-> run governed jobs and produce artifacts
-> reward useful contribution and referrals when qualified
-> record attribution, receipts, and follow-up
```

The operating assumption is that users conversationally configure and reuse
work. Ordo turns intent and variables into governed plans, tasks, requests,
artifacts, approvals, and measurement.

## Active Landing Target: Studio Ordo Hosted Appliance MVP

The current landing target narrows the roadmap to hosted trial appliance
management. Studio Ordo remains the public business and control plane. A trial
user's hosted Ordo can begin as an under-construction appliance while Ordo asks
onboarding questions, generates artifacts, and prepares the public surface.

The MVP loop is:

```text
QR scan
-> Ordo trial request
-> capacity check or waitlist
-> hosted Ordo commissioning
-> Traefik route and per-trial volume
-> onboarding reminders
-> scheduled conversation rollups
-> Growth brief
-> feedback/referral asks
-> final backup and return invitation
-> decommission or convert
```

Use [Studio Ordo Hosted Appliance MVP](studio-ordo-mvp.md),
[Hosted Ordo Control Plane](../architecture/hosted-ordo-control-plane.md), and
[Hosted Ordo Lifecycle](../architecture/hosted-ordo-lifecycle.md) as the active
MVP spine.

## Workflow: OrdoStudio NYC Pilot

1. Owner goes to NYC tech meetups and shows a QR code on the homepage.
2. Visitor scans the QR and enters a tracked visitor session.
3. Ordo presents a 30-day hosted Ordo trial offer with 10 active pilot spots.
4. Visitor accepts the offer and receives Access to a hosted trial Ordo.
5. Systems tracks trial capacity, waitlist, expiration, reset/wipe policy, and
   backup/restore readiness.
6. Studio Ordo commissions the hosted appliance, assigns a hostname, starts the
   container, and routes it through Traefik.
7. The user can export a backup before expiration and restore when allowed.
8. The user can request a strategic consulting session.
9. Ordo creates a Support handoff to Keith with conversation evidence.
10. Studio produces 10-30 second promo videos and publication metadata.
11. Growth tracks scans, chats, acceptances, trial activations, handoffs,
    feedback, referrals, videos produced, and performance evidence.
12. Qualified referrals can grant seven extra hosted days.
13. Accepted feedback can grant policy-defined hosted days.
14. Closeout creates a final backup, emails a return invitation, and blocks
    decommissioning until export evidence exists.

This is the first wedge because it proves Growth, Offers, Access, Systems,
Support, Studio, Member View, and rewards in one bounded loop.

## Workflow: Conversational Production Run

1. Operator asks Ordo for a production outcome, such as a 12-part short video
   sequence, a QA pass on markdown, a report, or a course artifact.
2. Ordo identifies available Access, content scopes, variables, and required
   approvals.
3. Ordo compiles a governed plan/DAG from pack templates and capability
   bindings.
4. Independent tasks run in parallel when policy and dependencies allow.
5. Requests collect feedback, missing information, QA, consent, or approval.
6. Artifacts return to Studio and any member-safe surfaces.
7. Growth records attribution, content performance, and outcome evidence.
8. The run can be copied with new variables and repeated.

## Workflow: Meetup QR To Live Handoff

1. Owner attends a meetup and shares a QR code.
2. Visitor scans the QR code and lands on the public Ordo edge.
3. Ordo answers from approved public About, Offers, Asks, and Feed material.
4. Visitor asks to talk to the owner.
5. Ordo checks visibility, consent, availability, operator status, threshold,
   and request intent.
6. Ordo asks screening questions when required.
7. Ordo creates an inbound connection request and operator handoff brief.
8. Owner accepts, declines, asks Ordo to continue screening, or queues the
   request for later.
9. If accepted, Ordo opens mediated chat.
10. The relationship can become a Connection with history, commitments,
    receipts, and follow-up tasks.

## Workflow: Affiliate Referral To Trial Conversion

1. Owner enables an affiliate connection.
2. Ordo issues referral assets: link, QR code, and share copy.
3. Affiliate promotes the default trial offer or another approved offer.
4. Visitor lands through the tracked entry point.
5. Ordo records the visit and carries attribution into chat.
6. Ordo answers from approved public truth and presents the offer.
7. Visitor accepts a 30-day Ordo trial.
8. Trial state is recorded and linked to the attribution ledger.
9. Qualified conversion creates reward or credit evidence according to the
   active reward program.
10. Affiliate dashboard updates with scoped funnel evidence.
11. Owner reviews credit state and approves, pays, grants benefit, or voids
    credit according to evidence.

## Workflow: Approved Support Packet Handoff

1. Local Ordo prepares a support or diagnostic report.
2. Owner reviews the packet contents locally.
3. Owner explicitly approves egress to Studio Ordo Support.
4. Ordo sends only the approved bounded packet.
5. Studio Ordo Support receives the packet and returns a receipt.
6. Local Ordo records the sent packet, receipt, connection event, and outcome.

This is an A2A-shaped support handoff, not full agent networking.

## Control Planes To Build

The product needs several small control planes before broad UI depth:

- Install and provider configuration.
- Owner identity and first-run business seeding.
- Content visibility and publication state.
- Connections and scoped grants.
- Availability, operator presence, and interruption thresholds.
- Handoff envelopes, inbox items, and receipts.
- Visitor sessions and tracked entry points.
- Offer acceptance and trial state.
- Offer-to-access grants and entitlements.
- Hosted trial capacity, expiration, backup-before-wipe, and reset policy.
- Hosted Ordo instance records, Docker/Traefik route orchestration, and
   per-trial volume manifests.
- Notification policy, transactional email attempts, receipts, and lifecycle
   reminders.
- Conversation rollup artifacts for Growth briefs.
- Trial closeout, final backup email, return invitation, and decommissioning
   evidence.
- Request, feedback, consent, and approval state.
- Reward programs, reward rules, reward ledger, and benefit grants.
- Product/workforce pack manifests.
- Compiled plans, variables, schemas, and DAG run records.
- CQRS-lite surface projections.
- Studio job/DAG template authoring and execution.
- Knowledge corpus, retrieval, and content-pack provenance.
- Attribution ledger and affiliate credit state.
- Referral and feedback reward qualification.
- Growth value events and content-performance evidence.
- Consent, preferences, retention, and expiration.
- Notification policy for live handoff and queued attention.

## Suggested Slice Order

1. Local Install And Provider Configuration Spine.
2. Owner Identity And First-Run Business Seeding.
3. Content Visibility And Publication Spine.
4. Connections Data Spine.
5. Availability And Operator Presence Spine.
6. Handoff Envelope And Attention Inbox.
7. Approved Support Packet Handoff.
8. Public Surface Read Models for About, Offers, Asks, and Feed.
9. Tracked Entry Point And Visitor Session Spine.
10. Offer Acceptance And Trial State.
11. Offer-To-Access Grant Spine.
12. Hosted Trial Capacity, Expiration, Backup, And Reset Spine.
13. Hosted Ordo Instance And Route Spine.
14. Commissioning And Decommissioning Job Templates.
15. Notification Policy, Email Attempts, And Receipts.
16. Conversation Rollup Artifacts For Growth.
17. Trial Closeout Backup And Return Invitation.
18. Request, Feedback, Consent, And Approval Spine.
19. Rewards, Referral Credit, And Benefit Grant Spine.
20. CQRS-Lite Surface Work Item Projection.
21. Product/Workforce Pack Manifest Spine.
22. Compiled Plan, Variable, And DAG Run Spine.
23. Job Kernel V2 Leases, Retries, And Result Envelopes.
24. Knowledge Corpus And Retrieval Spine.
25. Studio Job Template And DAG Execution Spine.
26. Affiliate Connection And Attribution Ledger.
27. Growth Value Events And Content Performance.
28. Minimal Affiliate Dashboard.
29. Meetup QR To Live Handoff Prototype.
30. Mediated Chat Through Ordo.
31. Domain Tool And Capability Pack Hardening.

Each slice should be small enough to validate but real enough to move a north
star workflow forward.

## Slice Quality Template

Each implementation issue should state:

```text
Slice:
Why it matters:
Workflow step advanced:
Durable product noun added or completed:
User-visible proof:
Backend proof:
Non-goals:
Acceptance criteria:
Validation:
```

No slice should ship with a UI claim that is not backed by durable appliance
state, policy, and evidence.
