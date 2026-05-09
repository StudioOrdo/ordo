# Conversation Product Doctrine

Status: Draft product contract for the 0.1.3 conversation realtime spine

This document translates the UX/product letter in
`docs/_letters/ux_product.md` into implementation doctrine for the conversation
architecture packet. It is intentionally product-level: future code should use
this as a contract before choosing data shapes, routes, read models, or UI
defaults.

## Core Principle

Ordo is not a generic CRM, support inbox, SaaS dashboard, or admin console.

The next product/eval arc should treat Ordo as:

```text
A local-first AI business appliance that turns conversations, relationships,
offers, asks, jobs, artifacts, feedback, reviews, referrals, and outcomes into
evidence-backed intelligence briefs for a small business owner.
```

The deeper product is a briefing-first relationship and business intelligence
system. Chat, CRM-like records, public pages, analytics, and content management
are components. The primary value proposition is that Ordo reads what is
happening in the business, connects it to evidence, and tells the owner what
matters and what to do next.

The conversation product should preserve this model:

```text
Clients participate in one relationship conversation.
Staff operate handoffs and business work.
Admins operate the appliance.
LLM jobs create episodes, tags, graph candidates, and briefs from evidence.
Offers and asks are measurable business instruments.
Artifacts are durable knowledge and business objects.
The UI is brief-first, evidence-backed, mobile-first, and narrative-first.
```

Greenfield implementation should prefer clean current contracts over backwards
compatibility with awkward draft names, UI habits, or accidental response
shapes.

## Navigation And Information Architecture

The primary flow is:

```text
Main area menu -> Evidence/record list -> Narrative brief/detail
```

On mobile, this compresses to:

```text
Menu -> Area evidence list -> Selected record brief
```

On desktop, this expands to:

```text
Top rail + staff/admin rail + evidence list + narrative brief/detail
```

The second column is an evidence or record list, not another menu. The detail
surface should default to a narrative brief that answers:

- what is going on;
- what changed;
- what to do next;
- why it matters;
- evidence;
- limitations and provenance.

### Top Rail

The top rail is for public, member, client, affiliate, and staff participation.

Canonical top rail:

```text
Studio Ordo | Chat | Home | Offers | Asks | Latest | Account
```

Anonymous users, authenticated clients, members, affiliates, staff, and owners
may all see the top rail. Non-staff users must not see the staff/admin rail or
appliance internals.

### Staff And Admin Rails

Business staff rail:

```text
Today | Conversations | Connections | Offers | Asks | Customer Feedback | Affiliates | Artifacts | Jobs | Reports
```

Owner/admin system additions:

```text
System | Knowledge | Events | Logs | Backup | Settings
```

Business staff work business movement. Admins and owners operate the appliance.
Health, logs, backup, readiness, events, low-level policy decisions, and other
system internals should not appear as ordinary staff navigation.

Conversations sit above Connections because active interaction comes before
durable relationship memory.

## Role-Aware Chat Presentation

The same conversation substrate presents differently depending on actor,
account role, resource grants, participant identity, conversation mode, handoff
assignment, and allowed scope.

- Anonymous visitors may start from Home/About, Offers, Asks, Latest, QR/link
  entry points, or Chat. They should get a visitor-session-backed relationship
  conversation without seeing staff internals.
- Authenticated clients and members should see one relationship conversation,
  available offers/asks, latest/member-visible activity, account tools, and
  client-safe deliverables.
- Affiliates use the client/member surface plus role-specific account tools
  such as referral links, QR card, referred leads, outcome/commission status,
  approved materials, and settings. They must not see unrelated customer
  conversations or owner-only business details.
- Business staff work from `Conversations -> My Handoffs` and business rails.
  Their detail view starts with the handoff or surface brief before raw
  transcript.
- Managers can see `Team Queue`; owner/admin users can inspect `All
  Conversations`, mode, grants, events, candidates, and provenance when
  authorized.
- System admins may operate appliance truth. System capability does not imply
  ordinary staff should see logs, backup, readiness, prompt internals, privacy
  placeholder maps, or token/cost internals by default.

## Client Conversation Model

Clients and members should not experience threads, tickets, or fragmented
inboxes. They should experience one persistent relationship conversation with
the business.

Client-facing language should be simple:

```text
Your conversation with Studio Ordo
Keith has joined the conversation.
A Studio Ordo team member is reviewing this.
Ordo Assistant is available.
```

Do not expose internal routing, confidence scores, handoff logic, policy state,
or LLM orchestration to the client.

## Staff Conversation Model

Staff operate many conversations, but the default surface is a work queue, not a
surveillance feed.

Primary staff views:

- `My Handoffs`
- `Team Queue`
- `All Conversations`

Default routing:

- business staff default to `My Handoffs`;
- manager/admin roles default to `Team Queue`;
- owner roles may default to `Today Brief` or `Team Queue`;
- technical admins may access all views, but `All Conversations` is not the
  ordinary staff default.

Conversation rows should answer:

- why this is in the queue;
- whether the viewer needs to act;
- urgency;
- status;
- involved connection/client;
- last meaningful change.

## Governed Handoffs

A handoff is a governed object, not a label on a conversation.

Minimum handoff fields:

- `conversation_id`
- `connection_id`
- `episode_id` or `segment_id`
- `requested_by`
- `assigned_to`
- `reason`
- `urgency`
- `required_capability`
- `evidence_summary`
- `allowed_context`
- `status`
- `receipt`

Lifecycle statuses:

- `suggested`
- `requested`
- `accepted`
- `declined`
- `assigned`
- `in_progress`
- `returned_to_agent`
- `closed`

Good handoff triggers include a direct request for a human, low confidence,
pricing or discount decisions, purchase intent, sell-to-us opportunities,
referrals, sensitive topics, policy boundaries, complaints, high-value
connections, and explicit staff tags.

Bad handoff triggers include every new message, casual greetings, routine offer
questions, and successful agent-led replies.

Every handoff should include a brief before the transcript:

- why this is in the queue;
- what the customer wants;
- what Ordo already said;
- relevant offer, ask, artifact, or prior relationship context;
- suggested reply;
- risk or constraint;
- evidence and provenance;
- full transcript access below the brief.

## Agent Etiquette

When a human staff member is actively leading a conversation, the Ordo agent
does not post publicly unless tagged, delegated, or policy requires
intervention.

Supported modes:

- `agent_led`
- `human_led_active`
- `human_led_idle`
- `assistive_private`
- `needs_handoff`
- `returned_to_agent`

Examples of explicit delegation:

```text
@Ordo summarize this
@Ordo find the offer
@Ordo draft a reply
@Ordo what do we know about Ava?
@Ordo take over routine questions
```

When staff joins or replies, the conversation becomes human-led. The agent can
continue to assist privately, but should not interrupt publicly. If a human-led
conversation goes idle, Ordo should privately remind the staff member first.
After a configured timeout, Ordo may post a soft holding message or resume only
when delegation and policy allow it.

## Episodes Inside One Conversation

The client sees one relationship conversation. Internally, Ordo organizes that
conversation into episodes or segments.

Conceptual model:

```text
Connection = the person or organization
Conversation = the persistent relationship room
Episode/Segment = a topic, matter, session window, handoff, or provider run
Handoff = a staff assignment/request inside an episode
Message = individual chat entry
Brief = synthesized summary of what matters
```

The first schema can map product episodes onto `conversation_segments` if that
keeps the implementation simple. If the semantics diverge, add a later episode
table instead of overloading segment fields.

LLM-created episodes are candidates. They require evidence, confidence,
provenance, and correction paths before being treated as durable business truth.

## Tags, Graph Candidates, And Memory

Conversation analysis may produce:

- episodes;
- tags;
- entities;
- relationships;
- signals;
- brief candidates;
- knowledge graph candidates;
- memory candidates.

Tags are operational, not decorative. They should support routing, briefing,
offer/ask measurement, staff next actions, and relationship memory.

## Customer Feedback And Reviews

Customer Feedback is a first-class business area. It follows the standard Ordo
shape:

```text
Customer Feedback area -> feedback evidence list -> feedback brief/detail
```

Definitions:

- Feedback is anything a customer says or does that teaches the business
  something. It is private business intelligence by default.
- Review is feedback the customer has explicitly allowed Ordo to publish.
- Testimonial is a curated, published review used in Home/About, Offers, or
  Latest.

Every review is feedback. Not every feedback item is a review. No feedback
becomes public proof without consent and owner/staff approval.

Feedback records should link to the source conversation, segment, message,
connection, offer, ask, artifact, referral, outcome, and generated brief where
evidence exists. Feedback tags are candidates first: `proposed`, `confirmed`,
`rejected`, or `superseded`. Starred feedback is staff-marked high-signal
business intelligence; it is not a customer rating. Rating is customer-provided
score evidence, if any. Featured means approved for public display.

Review lifecycle:

```text
Feedback captured -> Review candidate -> Review requested -> Review received
-> Consent confirmed -> Approved -> Published -> Featured -> Retired
```

## Home/About Narrative Brief

Home/About should be the public narrative brief of the business, not a static
biography page. It may use a scrollytelling sequence of evidence-backed
billboards that summarize and link into Offers, Asks, Latest, Artifacts,
Customer Feedback, Reviews, Outcomes, and Chat.

Suggested billboard sequence:

```text
Identity -> Problem -> Transformation -> Featured Offer -> Current Ask
-> Proof/Reviews -> Latest Activity -> Artifact/Outcome Proof -> Call To Action
```

Each billboard should have one message, one proof point or visual, one action,
and one detail link. Billboard states are `pinned`, `dynamic`, `draft`,
`published`, and `retired` so public narrative changes remain owner-governed.
Motion should clarify state and must have reduced-motion fallbacks.

Brand archetypes may guide copy, but claims must stay evidence-backed. For
Studio Ordo the likely archetype mix is `Sage + Magician + Creator`: clarity,
transformation, and tangible artifacts. Ethical persuasion may guide narrative
structure only when the proof exists. No fake scarcity, fake reviews, fake
metrics, unsupported authority, or invented social proof.

## Offers And Asks As Business Intent

Offers and Asks are human-readable pages and machine-readable business intent
objects. This keeps them ready for future A2A discovery without implementing
external A2A now.

An Offer describes what this Ordo can provide, who it is for, required inputs,
produced artifact/deliverable, terms, approval requirements, and how to start.
An Ask describes what this Ordo wants from its network, what qualifies, how
someone or another agent can respond, and what happens next.

Agents may discover, propose, summarize, match, and prepare. Humans or policy
decide what becomes real.

Example tags:

- `buying-signal`
- `pricing-question`
- `handoff-needed`
- `affiliate-interest`
- `sell-to-us`
- `referral-offered`
- `support-risk`
- `waiting-on-owner`
- `waiting-on-customer`
- `offer-accepted`
- `ask-response`
- `artifact-requested`

Candidate states:

- `proposed`
- `confirmed`
- `rejected`
- `superseded`

Graph candidates must include confidence, source message/event ids, provenance,
and generating job id. SQLite remains durable truth; use graph-shaped tables
before considering a graph database.

The UI should not default to a visual graph. The graph quietly powers narrative
briefs, related items, evidence, and recommended actions.

## Offers, Asks, Referrals, And Outcomes

Offers and Asks are measurable business instruments, not just content pages.

Ordo should eventually answer:

- who came in;
- where they came from;
- which offer or ask attracted them;
- whether they bought from us;
- whether they sold or provided something to us;
- who referred them;
- which artifact, QR code, link, post, or conversation influenced the action.

Core concepts:

- Referral
- Transaction/Outcome
- Offer Performance
- Ask Performance
- Attribution

Outcome types:

- buy from us;
- sell to us;
- refer to us;
- partner with us;
- donate/support;
- trial started;
- trial converted;
- declined/lost.

The strongest measurement line is relationship-to-outcome conversion:

```text
QR scan -> connection created -> conversation started -> offer accepted -> transaction recorded
Ask viewed -> referral submitted -> lead qualified -> trial started -> paid setup
```

Do not fake analytics before the evidence exists.

## Artifacts And Deliverables

`Artifact` is the canonical system noun.

Artifacts include briefs, reports, exports, backups, snapshots, generated media,
imported knowledge, support packets, offer materials, QR/card designs, and
published content.

Client-facing surfaces may use `Deliverable` when that language is clearer.
Deliverables are projections from artifacts, not a separate source of truth.
They should expose client-safe labels, summaries, and actions without leaking
internal job, provenance, policy, or storage mechanics unless that detail is
explicitly useful to the client.

Artifact briefs should answer:

- what this is;
- why it matters;
- where it is used;
- what to do next;
- what job produced it;
- what evidence/provenance supports it;
- storage or health status when available.

## Surface Brief Jobs

The desired loop is:

```text
Raw system data
-> scheduled or triggered job
-> evidence packet
-> deterministic or LLM synthesis
-> brief artifact
-> updated UI surface
-> owner action
-> new events
-> next brief
```

Major surfaces should eventually have generated or refreshed briefs:

- `business.brief.generate`
- `connections.brief.generate`
- `conversations.brief.generate`
- `offers.brief.generate`
- `asks.brief.generate`
- `artifacts.brief.generate`
- `jobs.brief.generate`
- `affiliate.brief.generate`
- `customer.brief.generate`
- `conversation.episode.extract`
- `conversation.tags.update`
- `connection.memory.update`
- `handoff.brief.generate`
- `offer.performance.summarize`
- `ask.performance.summarize`
- `referral.attribution.update`
- `artifact.usage.summarize`

The latest completed brief should load first. Refresh jobs must not block the
primary UI from rendering existing evidence.

## Public, Member, Client, And Affiliate Surfaces

Authenticated non-staff users are not inside the staff/admin appliance. Their
primary navigation is:

```text
Chat | Home | Offers | Asks | Latest | Account
```

Client/member account tools may include:

- My conversations
- My offers
- My deliverables
- My requests
- Settings

Affiliate account tools may include:

- Affiliate dashboard
- Referral links
- QR card
- Referred leads
- Outcome/commission status
- Approved materials
- Settings

Staff/owner account tools may include:

- Open System
- My profile
- Preferences
- Sign out

Recommended landing defaults:

- anonymous visitor -> Home/About first;
- authenticated client/member -> Chat first;
- staff/owner -> Today/System first;
- QR card scan -> campaign/card landing page first;
- returning lead with open conversation -> Chat first.

## Ethical Business Persuasion Prompt Slot

Ordo may use Robert Cialdini's persuasion principles as an ethical business
communication lens, not as a manipulation layer. This belongs in a reusable
prompt slot named `ethical_business_persuasion`.

Use the slot when drafting replies, handoff suggestions, offer or ask
recommendations, briefs, and staff guidance.

The slot may consider:

- reciprocity: value already provided and useful next help;
- commitment/consistency: stated interests, choices, commitments, or prior
  actions;
- social proof: verified examples or outcomes only;
- authority: genuine expertise, artifacts, credentials, or evidence only;
- liking: authentic relationship context and appropriate tone;
- scarcity: real constraints only, never invented urgency;
- unity: genuine shared mission, identity, community, or affiliation.

Guardrails:

- do not invent evidence;
- do not exaggerate urgency;
- do not exploit fear, shame, confusion, dependency, or pressure;
- do not hide material limitations;
- do not present candidates as facts;
- do not override consent, privacy, safety, or policy;
- keep staff-facing reasoning evidence-backed and inspectable;
- keep client-facing language respectful, plain, and agency-preserving.

The prompt slot should record the same metadata as other LLM slots: slot id,
version, evidence/source refs, visibility ceiling, policy decision,
transform run, token estimates, inclusion reason, and truncation reason.

Implementation status: the `ethical_business_persuasion` v1 slot is a
daemon-owned contract. The Rust builder requires explicit evidence/source refs
for every included principle, rejects unsupported social proof, authority,
scarcity, urgency, pressure, shame, fear, confusion, dependency, and internal
mechanics in client-facing copy, and records usage through the shared prompt
slot accounting ledger. Staff views may show reasoning and evidence. Client
surfaces must show only the plain suggestion language, if any, and never the
internal prompt slot mechanics.

## Non-Goals

Do not use this product pass to build:

- a full visual graph UI;
- a full CRM dashboard;
- fake analytics;
- fake scarcity or social proof;
- artifact-performance plugins without evidence;
- a full affiliate commission system;
- external social or YouTube analytics;
- a multi-operator enterprise inbox;
- a new graph database;
- direct provider calls from Next.js.
