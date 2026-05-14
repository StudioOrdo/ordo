# Ordo Core

Status: Product doctrine, not fully implemented

Ordo Core is the durable product. It is the local-first business appliance that
protects trust boundaries, records evidence, and gives a one-person business a
governed presence that can speak, act, qualify, hand off, and remember.

The core should remain timeless across business types. Customization should come
from MCP tools, process templates, adapters, prompts, and domain packs that plug
into the same governed runtime instead of forking the product.

In product terms, Ordo Core is the rock-solid ground for packaged workforces.
Community and commercial packs should add domain labor, not bypass the kernel.

Core should borrow serious operating patterns from enterprise SaaS and express
them through a local-first appliance: CQRS-lite, event trails, RBAC/scoped
grants, workflow execution, retries, leases, approval gates, artifact
provenance, observability, analytics ledgers, and adapter contracts.

## Core Thesis

Ordo Core is a trusted business boundary with an AI interface.

The owner should be able to say:

```text
Ordo can meet the world without giving the world direct access to me.
```

The system should preserve this loop:

```text
Local truth -> governed work -> artifact -> brief -> approved action or exchange
```

## Durable Product Nouns

The core product vocabulary is:

- Identity: the owner, business, Ordo, visitor, actor, or connection being
  represented.
- Content Visibility: the access level for business truth and generated
  material.
- Public Surfaces: approved About, Offers, Asks, and Feed material exposed to
  the world.
- Visitor Session: a public interaction with Ordo before the visitor is fully
  known.
- Tracked Entry Point: a QR code, affiliate link, offer link, content link, or
  campaign link that carries source context.
- Offer: something a visitor can buy, join, book, request, accept, or trial.
- Want: an owner-expressed need, opportunity, request, or desired connection.
- Ask: a Want that has been approved for a visible audience.
- Connection: a trusted relationship with scope, grants, history, and
  revocability.
- Availability: normal rules for when live handoff may be possible.
- Operator Presence: the current operator status and interruption threshold.
- Handoff: a structured transfer from Ordo, a job, an agent, or a visitor to a
  human, connection, or peer Ordo.
- Mediated Chat: a conversation that continues through Ordo with policy and
  context intact.
- Attribution Ledger: the evidence chain from entry point through conversation,
  offer acceptance, trial, conversion, and credit.
- Reward Program: a reusable incentive program that turns qualified actions
  into ledgered benefits.
- Benefit Grant: hosted days, credits, pack access, render minutes,
  leaderboard points, or other earned benefits enforced through Access.
- Job, Task, Event, Artifact, Brief, And Receipt: the existing appliance spine
  for governed work and evidence.

The smaller kernel boundary is:

```text
Actor
Connection
Tracked Entry Point
Offer
Access
Request
Reward
Capability
Plan / Compiled Plan
Job / Task
Artifact
Event
Outcome
Read Model
```

The kernel rule is:

```text
Canonical tables own truth.
Events own audit and replay.
Read models own surface experience.
Adapters own external systems.
```

## Visibility Model

Content should use a small common visibility vocabulary:

```text
public
authenticated
staff
owner
```

- Public content can be shown to anyone and can be used by the public Ordo
  conversation surface.
- Authenticated content can be shown to known signed-in users, such as trial
  users or customers.
- Staff content can be shown to internal operators or support staff.
- Owner content is private to the appliance owner unless explicitly included in
  an approved handoff or packet.

Visibility, publication state, and connection grants are separate controls:

```text
visibility = who can see it by default
publication_state = whether it is live
connection_grant = scoped exception for a specific connection
```

No generated answer should use content above the viewer's clearance. No tracked
entry point should expose a destination above the visitor's clearance.

## Connections And Handoffs

Connections are not a social graph. They are trust-and-work relationships.

A Connection may represent a person, another Ordo, Studio Ordo Support, an
operator, a client, an affiliate, a service provider, a worker, a device, or a
future peer node.

The plain-language rule is:

```text
Show the person when there is a person.
Trust the Ordo or connection boundary.
Call the relationship a Connection.
```

Handoffs are how Ordo crosses an attention, trust, or execution boundary. A
handoff should record:

- the source actor, job, task, agent, visitor, or connection;
- the destination connection or operator;
- included artifacts and evidence;
- requested action;
- required approval;
- delivery state;
- receipt or outcome.

Support packet transfer, agent-to-operator escalation, visitor-to-owner live
handoff, affiliate introduction, and peer-Ordo exchange are all specialized
forms of the same pattern:

```text
Connection + Handoff + Receipt
```

## Availability And Presence

Availability is not only display text. It is handoff policy.

The handoff decision should combine:

```text
availability schedule
operator presence
interruption threshold
request intent
connection trust
visibility and policy checks
```

Operator status should support pausing or conditioning live handoff. Thresholds
should allow modes such as full availability, selective availability, money-only,
urgent-only, and paused.

The product rule is:

```text
Ordo can talk anytime.
Humans are handed off only when availability and policy allow it.
```

## Affiliate And Sales Loop

Affiliate behavior should be modeled as a Connection capability, not a separate
marketing island.

An affiliate connection may receive:

- a default affiliate offer;
- a referral link and QR code;
- permission to promote selected offers;
- an attribution ledger;
- an affiliate dashboard scoped to its own introductions and credit state.

The sales loop is:

```text
Connection -> tracked entry point -> visitor -> Ordo conversation -> offer -> trial -> conversion -> credit
```

The default commercial proof can be:

- Become an affiliate.
- Start a 30-day Ordo trial.
- Track who arrived, talked to Ordo, accepted the offer, started the trial, and
  converted.

No credit should be granted without evidence. No affiliate dashboard should
expose private visitor or owner data beyond its scoped attribution view.

## Rewards And Incentives

Rewards are a reusable Growth capability.

The default reward rule is:

```text
No reward without evidence.
Growth records qualification.
Access grants the benefit.
```

Reward programs can support referral credits, feedback credits, community QA,
leaderboards, prizes, hosted-time extensions, render minutes, or pack unlocks.
The first OrdoStudio pilot can grant seven hosted days for a qualified referral
and policy-defined hosted days for accepted feedback.

Rewards should be ledgered, capped, reviewable, and reversible. Leaderboards
should be opt-in and pseudonymous by default. Prize programs should require
explicit terms and owner approval before fulfillment.

## Customization Through Tools And Packs

Customization should change the work Ordo can perform, not the trust model.

Domain-specific MCP tools and packs should register into the same capability,
policy, artifact, brief, visibility, and audit spines. A custom tool should
declare its identity, capability, input schema, output schema, side effects,
policy tier, visibility classification, required grants, and expected artifacts.

A product pack is a packaged workforce. It can include tools, content scopes,
prompt templates, variables, job plans, request templates, artifact contracts,
limits, approval gates, and growth metrics, but it should still execute through
the same governed runtime.

Examples:

- a yoga teacher Ordo may add scheduling, waiver, and membership tools;
- a bookkeeper Ordo may add receipt ingestion and close checklist tools;
- a consultant Ordo may add proposal, discovery, and onboarding tools;
- a creator Ordo may add content calendar, sponsorship, and guest-ask tools.

The core rule is:

```text
Customize the work, not the trust boundary.
```

## Quality Bar

Every future slice should preserve these rules:

- no public answer from private truth;
- no live interruption without availability, threshold, and policy checks;
- no hidden egress;
- no unscoped connection;
- no unsupported claim;
- no untracked conversion;
- no reward or leaderboard credit without durable qualification evidence;
- no external packet without approval and receipt handling;
- no custom tool outside the capability and audit model;
- no product surface that pretends unbuilt behavior exists.
