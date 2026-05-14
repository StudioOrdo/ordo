# Current Product Canon

Status: canonical product stance as of 2026-05-13

This document is the product source of truth when older docs, UI fixtures, or
implementation details disagree. Ordo is using a surface-first product model.
Roles control permissions and projections; roles are not the primary
navigation model.

## Product Sentence

Ordo is a local-first AI operating appliance for solopreneurs and small
operators. It turns conversations, offers, access, requests, jobs, artifacts,
feedback, referrals, and outcomes into evidence-backed business motion.

The product is not only chat, a CRM, a dashboard, a workflow builder, or a
content manager. Those are parts of the system. The deeper product is a
governed workforce substrate: Ordo packages useful workforces for the owner,
not a pile of disconnected tools.

The user-facing standard is:

```text
Ruthlessly simple for the user.
Rigorously disciplined in execution.
```

The operator should be able to talk to Ordo, review what it produced, give
feedback, and approve or redirect work without needing to wire a form-first
workflow.

The architecture standard is enterprise operating discipline inside a local
appliance: CQRS-style read models, event/audit trails, policy decisions,
scoped grants, workflow execution, retries, approval gates, artifact
provenance, analytics ledgers, and extension contracts, without exposing
enterprise admin complexity to the user.

Implementation direction is captured in
[Appliance Operating Discipline](../architecture/appliance-operating-discipline.md)
and [Target Architecture Plan](../architecture/target-architecture-plan.md).

## Canonical Surfaces

The current product IA is:

```text
Member View
Studio
Support
Knowledge
Growth
Systems
```

### Member View

The customer-safe participation surface.

Member rooms:

```text
Ordo
Activity
Offers
Access
Requests
Referrals
```

The member talks with Ordo, accepts offers, uses granted access, responds to
requests, gives feedback, reviews artifacts when asked, and sees safe status.
The member does not see staff routing, policy internals, provider details,
raw system logs, or unrelated customer data.

`Access` is the member-facing label for accepted offers and granted
capabilities. Internal code may still use capability, entitlement, grant, or
capability pack where appropriate.

### Studio

The production surface.

Studio owns repeatable work: job templates, DAGs, media projects, generated
artifacts, content production, publication prep, and review loops. Studio is
where an operator turns knowledge and intent into finished or candidate
artifacts.

Studio is conversational first. The operator expresses intent, reviews
results, provides feedback, approves, rejects, or redirects. Structured fields
can exist for inspection, accessibility, repair, and power-user editing, but
they should not be required for the happy path.

### Support

The relationship and human-attention surface.

Support owns conversations, handoffs, customer requests, QA, feedback, review
triage, customer follow-up, and staff-visible decision work. Support starts
from briefs and work queues, not from raw transcripts by default.

### Knowledge

The grounded memory surface.

Knowledge owns corpus sources, content packs, provenance, source licensing,
retrieval readiness, generated knowledge artifacts, and access-aware RAG
boundaries. Knowledge feeds chat, Studio jobs, offers, requests, and briefs.

### Growth

The business-value surface.

Growth owns offers, asks, QR paths, campaigns, attribution, referrals, content
performance, value events, rewards, and business learning. Growth asks whether
the work produced relationships, conversions, feedback, reach, or useful
evidence.

Tracked entry points are a Growth primitive. QR codes and links can attach to
offers, requests, artifacts, scrollytelling frames, campaigns, events,
referrals, and support paths. The code should carry an opaque tracked entry
point; Ordo records allowed context such as creator, offer, request, artifact,
campaign, created time, scan time, and coarse location when supplied or
permitted.

Rewards are a Growth primitive. Feedback, referrals, QA, community
contribution, publishing milestones, and other tracked actions can earn
benefits such as hosted days, usage credits, render minutes, pack access, or
leaderboard points. Growth records qualification and reward evidence. Access
grants and enforces the benefit.

### Systems

The appliance operation surface.

Systems owns runtime health, providers, local install state, hosted instances,
backups, restores, logs, events, policy, access/RBAC, diagnostics, audit, and
low-level job/runtime operations. Systems is not the ordinary customer or
business-staff experience.

## Core Loop

The product loop is:

```text
Approved truth
-> governed capability
-> job/DAG execution
-> artifact
-> request, approval, or publication
-> brief
-> attribution and learning
```

The world-facing loop is:

```text
Entry point
-> Ordo conversation
-> offer, ask, handoff, or connection
-> access, request, or job
-> artifact or outcome
-> reward or benefit when earned
-> attribution and follow-up
```

## Offer, Access, Request, DAG

This is the product spine:

```text
Offer accepted
-> Access granted
-> User asks Ordo for an outcome
-> Ordo validates access and variables
-> Ordo compiles a governed job/DAG
-> Tasks run through capabilities
-> Requests collect human input, feedback, consent, or approval
-> Artifacts return to the right surfaces
-> Growth records attribution and value
```

An offer should be more than marketing copy. It should install or unlock a
governed operating context:

- content/corpus scope;
- tools/capabilities;
- job templates;
- request templates;
- artifact types;
- usage limits;
- visibility and publication rules;
- approval and consent gates;
- attribution rules;
- reward and benefit rules.

A request is something waiting on a person. Requests include approvals,
feedback, consent, scheduling, QA follow-up, artifact review, missing
information, or confirmation.

Product packs should install repeatable workforces, not arbitrary code. A pack
may include capability bindings, content scopes, prompt templates, variables,
schema validation, job plans, request templates, artifact contracts, visibility
rules, limits, approval gates, and growth measurements.

## Rewards And Benefits

Reward programs should be reusable system objects, not one-off marketing logic.

The durable reward loop is:

```text
tracked action
-> qualification rule
-> reward event
-> reward ledger entry
-> benefit grant
-> Access enforcement
```

Pilot examples:

- qualified referral grants seven extra hosted days;
- accepted feedback grants policy-defined hosted days;
- community QA can grant credits;
- leaderboard participation is opt-in and projected from ledger evidence.

No reward should be granted without evidence. A referral should require a
qualified downstream event, not only a QR scan. Feedback should require useful
review, not only submission. Rewards may be pending, granted, capped, rejected,
expired, or reversed.

See [Rewards And Incentives](../architecture/rewards-and-incentives.md).

## Text-First UX

Every important state must have a text explanation:

- what Ordo is doing;
- what it needs from a person;
- what artifact was produced;
- what evidence supports it;
- what limitations remain;
- what action is recommended.

This is required for accessibility and future voice, phone, SMS/Twilio, and
agent-to-agent projections. Voice is not a separate product shape. It is
another interface onto the same request, job, artifact, brief, approval, and
event spine.

## Brief-First UX

Every operator surface should default to a brief or selected detail that answers:

- what is happening;
- what changed;
- what needs attention;
- why it matters;
- what action is recommended;
- what evidence supports it;
- what the limitations are.

The UI rule is:

```text
Brief first.
Evidence second.
Controls after context.
```

## Trust Boundaries

Ordo must preserve these rules:

- no public answer from private truth;
- no customer view of staff routing, provider internals, prompt internals, or
  policy machinery;
- no unsupported public claim;
- no hidden egress;
- no unscoped connection or access grant;
- no reward, credit, leaderboard rank, or trial extension without durable
  qualification evidence;
- no custom tool outside the capability, policy, artifact, visibility, and
  audit model;
- no product UI claim that durable state cannot support.

## AGPL Product Posture

The AGPL repository is Ordo Core: the local-first appliance, job kernel,
capability catalog, policy, artifacts, public/member/operator surfaces, and
extension contracts.

Commercial value can live around the core through hosted convenience, managed
Worker Ordos, premium packs, curated corpora, templates, support, native media
execution, and implementation services. The open core should remain useful and
inspectable on its own.

Community extension should produce governed packs and packaged workforces that
stand on Ordo Core. It should not require users to become workflow engineers.

## Appliance Discipline

Ordo should borrow what enterprise SaaS learned about reliable operations and
put it inside an inspectable appliance:

- command handlers validate policy and mutate canonical truth;
- events preserve audit, replay, and explanation;
- read models serve Member View, Studio, Support, Knowledge, Growth, and
  Systems;
- task executors return structured result envelopes;
- artifacts carry provenance, visibility, versions, and review state;
- growth claims come from attribution, outcome, and reward ledgers;
- adapters connect MCP, browser/WASM, providers, native tools, and future A2A
  without becoming the product spine.

The user sees a trusted workforce. The appliance runs disciplined operating
machinery.
