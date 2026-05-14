# Product Shape

Status: Product direction, not fully implemented

Current canon: [Current Product Canon](current-product-canon.md)

Ordo is a local-first operating system for one-person businesses. The current
repo proves the appliance spine. The full product shape is broader: Ordo should
become a trusted business boundary that can speak, qualify, protect attention,
run governed work, and exchange evidence without surrendering owner control.

The product should feel like a governed workforce substrate, not a toolbox.
The owner should be able to direct work conversationally, inspect evidence,
approve outcomes, publish, hand off, remember, and move artifacts across
governed capabilities without becoming a workflow engineer.

The durable core is described in [Ordo Core](ordo-core.md). The workflow-driven
roadmap is described in [Product Roadmap](product-roadmap.md). The workforce
stance is described in [Workforce Substrate](workforce-substrate.md). The
backend discipline is described in
[Appliance Operating Discipline](../architecture/appliance-operating-discipline.md).

The product borrows from enterprise SaaS where enterprise systems are right:
commands, read models, events, audit, RBAC, grants, workflows, retries,
approval gates, provenance, observability, analytics, and extension contracts.
It rejects the enterprise habit of making the user operate the machinery.

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

The implemented surface closest to the current canon is the Systems appliance
shell:

- Brief
- Health
- Backup And Restore
- Schedules
- Preferences
- Events
- Logs
- Reports

These rooms help the operator inspect the appliance, understand what
happened, prepare local diagnostic reports, and verify that work is grounded in
evidence.

## Canonical Product Surfaces

The current product stance is surface-first:

```text
Member View
Studio
Support
Knowledge
Growth
Systems
```

Roles are permission context and projection policy. Roles are not the primary
navigation model.

### Member View

Member View is the customer-safe participation surface. The member talks to
Ordo, sees Activity, accepts Offers, uses Access, responds to Requests, and
participates in Referrals. Members should not see staff routing, provider
mechanics, raw policy state, or unrelated customer data.

### Studio

Studio is the production surface. It owns repeatable jobs, DAGs, templates,
media work, artifact generation, review loops, and publication prep. Studio is
conversational first: the operator talks to Ordo, reviews results, gives
feedback, approves, rejects, or redirects. Structured controls can support
inspection and repair, but they should not become the default happy path.

### Support

Support is the relationship and human-attention surface. It owns conversations,
handoffs, customer requests, QA, feedback, review triage, and staff-visible
decision work.

### Knowledge

Knowledge is the grounded memory surface. It owns corpus sources, provenance,
content packs, retrieval readiness, generated knowledge artifacts, and
access-aware RAG boundaries.

### Growth

Growth is the business-value surface. It owns offers, asks, QR paths,
campaigns, attribution, referrals, content performance, value events, rewards,
benefit grants, and business learning.

### Systems

Systems is the appliance operation surface. It owns runtime health, providers,
local install state, hosted instances, backups, restores, logs, events, policy,
access/RBAC, diagnostics, audit, and low-level runtime operations.

### Public Story

Home/About, public Offers, public Asks, Latest, and public Chat are public
projections of the same underlying surfaces. Public claims must be backed by
published public truth or clearly marked as aspiration.

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

## Briefing Surface

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
the offer, started a trial, converted, and earned qualified credit.

## Rewards And Benefits

Rewards should be a reusable Growth function:

```text
tracked action -> qualification -> reward ledger -> benefit grant -> Access
```

Examples include referral-hosted-time credits, feedback-hosted-time credits,
community QA credits, leaderboard points, render minutes, and pack unlocks.

The important split is that Growth qualifies and records the reward, while
Access enforces the benefit. A reward program should never silently extend a
trial or unlock a capability without ledger evidence.

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

## Governed Creative Tools

Creative tools should be reusable provider capabilities wrapped by
product-shaped methods and workflow templates. Ordo can support image
generation, image review, TTS, transcription, video storyboard drafts, web
search, screenshot QA, QR generation, and content analytics, but those tools
must run through jobs, artifacts, policy, visibility, audit, access, Growth,
and approval gates.

The product rule is:

```text
Generic capability.
Product-shaped method.
Governed workflow.
Evidence-backed artifact.
```

The owner should be able to ask for flexible work, such as twelve zodiac images
or an article plus a matching image about a shared topic, without becoming a
workflow engineer. Internally, Ordo should resolve typed workflow variables,
expand bounded fanout, run approved capabilities, preserve artifacts, and ask
for approval before publishing or external egress.

## Content Learning Loop

Ordo should learn from content operations, but generated content should inform
memory as evidence and candidate claims, not automatically become truth.

The durable learning loop is:

```text
generated artifact
-> extracted claims and preferences
-> approval, rejection, publication, or feedback
-> engagement and outcome events
-> candidate, preference, negative, published, or confirmed memory
-> better next workflow
```

Ordo should remember:

- what was generated;
- what was approved;
- what was rejected;
- what was published;
- what users saw;
- what they clicked, requested, tried, referred, or reviewed;
- what feedback or outcomes followed.

Content analytics belongs to Studio and Growth, not to a generic analytics
dashboard. Studio records what was produced and reviewed. Growth records
attribution, engagement, reward, referral, trial, feedback, and outcome
evidence. Knowledge and graph memory receive only evidence-backed candidate or
confirmed facts according to visibility and approval policy.

## Knowledge, Access, And Packs

Future Ordo should support curated knowledge and workforce packs with
provenance. A user should be able to build approved content packs from source
material and generated artifacts, then load those packs into another Ordo.

This is not implemented yet. The near-term prerequisite is access-aware
knowledge/RAG with clear corpus provenance.

In product language, offers unlock Access. Access may include content,
capabilities, templates, request types, artifact types, usage limits,
visibility rules, and approval gates. Internal implementation can use
capability packs, workforce packs, grants, and entitlements, but the
member-facing surface should remain concrete about what the user can use now.

Customization should come from MCP tools, process templates, adapters, prompts,
and domain packs that register into the same capability, policy, artifact,
brief, visibility, and audit spines.

The customization rule is:

```text
Customize the work, not the trust boundary.
```

## Network Direction

Ordo should be useful alone. Later, Ordos may exchange governed artifacts and
requests. The direction is many sovereign appliances, not one giant SaaS
database. Agent-to-agent networking should project Ordo requests, jobs,
artifacts, receipts, offers, and asks through policy instead of exposing raw
internals.

The first network wedge should be an approved support packet or worker
assignment, because both can be bounded by artifacts, receipts, and explicit
egress approval.

The first likely network use case is support: a local Ordo prepares a local
issue report, the operator approves sending it, and Studio Ordo Support receives
a provenanced support packet and returns a receipt.

Worker Ordos, A2A networking, service discovery, and content-pack exchange are
future direction. They are not current product behavior.
