# Product Shape

Status: Product direction, not fully implemented

Ordo is a local-first operating system for one-person businesses. The current
repo proves the appliance spine. The full product shape is broader: Ordo should
become a trusted business boundary that can speak, qualify, protect attention,
run governed work, and exchange evidence without surrendering owner control.

The product should feel like a browser-like runtime for AI work and business
presence: one place to interact, inspect, approve, remember, publish, hand off,
and move artifacts across governed capabilities.

The durable core is described in [Ordo Core](ordo-core.md). The workflow-driven
roadmap is described in [Product Roadmap](product-roadmap.md).

## Core Loop

The product loop is:

```text
Local truth -> governed work -> artifact -> brief -> approved action or exchange
```

The world-facing loop is:

```text
Tracked entry point -> public Ordo conversation -> offer, ask, handoff, or connection -> evidence and follow-up
```

## Implemented Today

The current product surface is the System appliance shell:

- Brief
- Health
- Backup And Restore
- Schedules
- Preferences
- Events
- Logs
- Reports

These surfaces help the operator inspect the appliance, understand what
happened, prepare local diagnostic reports, and verify that work is grounded in
evidence.

## Planned Product Surfaces

The main business product should organize around owner, public, and connection
surfaces. These are planned surfaces, not implemented in the current repo.

### Chat

Chat is the primary operating interface. The owner should be able to ask Ordo to
inspect, plan, create, revise, schedule, and explain work from one place.

Chat should not bypass governance. It should route intent through capabilities,
jobs, evidence, and approvals.

### About

About is the public business story. It should explain who the business serves,
what it believes, what evidence supports its claims, and where visitors should
go next.

This should become a scrollytelling business narrative, not a generic page
builder.

### Offers

Offers describe what can be bought, joined, booked, requested, accepted, or
trialed.

An offer should be durable business data, not only marketing copy. It should
connect to audience, visibility, fulfillment, evidence, follow-up, tracked entry
points, attribution, and measurement.

### Asks And Wants

A Want is an owner-expressed need, opportunity, request, or desired connection.
An Ask is a Want that has been approved for a visible audience.

Asks let the business say what it is looking for: clients, collaborators,
guests, referrals, sponsors, contractors, venues, testers, support, or evidence.

### Feed

Feed is the public stream of composite artifacts: articles, podcast outputs,
images, clips, briefs, updates, and syndicated machine-readable content.

The feed should support short-form content marketing while preserving provenance
and source grounding.

### Connections

Connections are trusted relationships with scope, grants, history, and
revocability. A Connection may represent a person, another Ordo, Studio Ordo
Support, an affiliate, a client, a worker, a service provider, a device, or a
future peer node.

Connections should not feel like a social network. They are the trust-and-work
surface for handoffs, packets, receipts, affiliate introductions, support, and
future peer exchange.

## Content Visibility

Public surfaces need a shared visibility vocabulary:

```text
public
authenticated
staff
owner
```

This model should gate About, Offers, Asks, Feed items, Ordo answer sources,
handoff packets, affiliate materials, and trial onboarding. Public Ordo answers
must only use public approved truth. Authenticated, staff, and owner contexts
may use deeper material according to policy.

Visibility, publication state, and connection grants are separate controls.

## Owner Operating Room

Briefs are how Ordo explains what matters. The owner should not have to inspect
raw tables, logs, or workflow internals first.

The product should bring back:

- the current state;
- what changed;
- what needs attention;
- evidence;
- limitations;
- recommended next action.

The owner should receive briefs and inbox items instead of raw noise. Handoffs,
connection requests, support receipts, affiliate credit reviews, and trial
opportunities should land in an attention surface with evidence and clear
actions.

## Availability And Handoff

Availability is part of policy, not only page copy. Ordo should distinguish
normal business hours, live handoff hours, operator presence, and interruption
thresholds such as open, selective, money-only, urgent-only, and paused.

The product rule is:

```text
Ordo can talk anytime.
Humans are handed off only when availability and policy allow it.
```

Handoffs should carry source, destination, intent, evidence, required approval,
delivery state, and receipt or outcome.

## Affiliate And Sales Loop

Affiliate behavior should be a Connection capability. An affiliate connection
may receive referral assets, promote approved offers, and view an affiliate
dashboard scoped to its own attribution evidence.

The closed sales loop is:

```text
Connection -> tracked entry point -> visitor -> Ordo conversation -> offer -> trial -> conversion -> credit
```

Default commercial proofs should include a become-affiliate offer and a 30-day
Ordo trial offer. Attribution should track who arrived, talked to Ordo, accepted
the offer, started a trial, and converted.

## Build-Measure-Learn Loop

Ordo should help a solopreneur experiment faster:

1. define an offer, story, or content hypothesis;
2. publish or send it;
3. observe response;
4. summarize evidence;
5. recommend revisions;
6. preserve what worked as reusable process.

The goal is not just content generation. The goal is lower business overhead
and faster learning.

## Knowledge And Packs

Future Ordo should support curated knowledge and capability packs with
provenance. A user should be able to build approved content packs from source
material and generated artifacts, then load those packs into another Ordo.

This is not implemented yet. The near-term prerequisite is access-aware
knowledge/RAG with clear corpus provenance.

Customization should come from MCP tools, process templates, adapters, prompts,
and domain packs that register into the same capability, policy, artifact,
brief, visibility, and audit spines.

The customization rule is:

```text
Customize the work, not the trust boundary.
```

## Network Direction

Ordo should be useful alone. Later, Ordos may exchange governed artifacts and
requests.

The first likely network use case is support: a local Ordo prepares a local
issue report, the operator approves sending it, and Studio Ordo Support receives
a provenanced support packet and returns a receipt.

Worker Ordos, A2A networking, service discovery, and content-pack exchange are
future direction. They are not current product behavior.
