# Product Roadmap

Status: Roadmap direction, not fully implemented

This roadmap records the product shape Ordo is marching toward. It is not a
release commitment and does not claim current implementation.

The roadmap should be workflow-driven. A slice is valuable when it adds a
durable product noun, completes a step in a north-star workflow, and provides
evidence through tests, docs, and bounded runtime behavior.

## North Star Loop

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
-> record attribution, receipts, and follow-up
```

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
9. Affiliate dashboard updates with scoped funnel evidence.
10. Owner reviews credit state and approves, pays, or voids credit according to
    evidence.

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
- Attribution ledger and affiliate credit state.
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
11. Affiliate Connection And Attribution Ledger.
12. Minimal Affiliate Dashboard.
13. Meetup QR To Live Handoff Prototype.
14. Mediated Chat Through Ordo.
15. Domain MCP Tool And Capability Pack Hardening.

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
