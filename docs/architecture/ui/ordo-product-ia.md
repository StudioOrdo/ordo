# Ordo Product IA Contract

This document defines the product information architecture that current UI work
should implement against. It distills the MVP brief, mock data seed, and design
discussion into an implementation-facing contract for shells, rooms, work
items, and proof.

Use this as the source of truth when deciding what belongs in the member
experience, what belongs in operator surfaces, and what evidence a UI claim must
cite.

## Product Thesis

Ordo is a business operating surface for solopreneurs. It turns QR scans,
relationship conversations, offers, accepted capabilities, requests, feedback,
referrals, jobs, and system events into visible business process.

The member does not manage a SaaS dashboard. The member talks with Ordo and
responds to business events. Internal users use Support, Growth, and System
surfaces to operate the business behind that member-safe experience.

## IA Rules

1. Role is not navigation.
   Roles control access, permissions, and projection safety. Shells control
   where work happens.

2. One object can appear in many shells.
   A QR scan can appear as a member-safe path, a Support handoff, a Growth
   attribution event, and a System entry-point event. Each shell sees a
   projection of the same object, not an unrelated duplicate.

3. Member help starts in Ordo.
   A member asks for support, clarification, changes, or help through the Ordo
   conversation. The Support shell receives the operational side later.

4. Offers and requests are different.
   Offers are what the business invites a member to accept, buy, join, or
   unlock. Requests are what the business needs the member to do, approve,
   confirm, review, provide, or schedule.

5. Access is the member label for capabilities.
   The product may call the member room `Access` in UI copy. The internal model
   can continue to use `Capability`, `CapabilityGrant`, or `AccessGrant`.

6. Activity is a projection, not a separate object store.
   Activity is the cross-room feed of actionable projections from offers,
   requests, referrals, access, conversations, and system-safe events.

7. Proof is typed evidence.
   Proof means durable evidence references, candidate evidence references, or
   explicit missing-evidence states. Proof is not decorative UI.

8. The member Ordo room is chat-first.
   The member Ordo room uses `[rail][room drawer][chat stage]`. It does not need
   a worklist because there is one primary relationship conversation.

9. Other member rooms use the same work-item contract.
   Activity, Offers, Access, Requests, and Referrals use
   `[rail][room drawer][worklist][selected detail stage]`.

## Core Shells

| Shell | Audience | Job | Room navigation | Work item examples | Proof shown | Not for |
| --- | --- | --- | --- | --- | --- | --- |
| Public/Site | Guest, returning visitor, signed-in member viewing public story | Explain the business, start chat, route QR visitors, offer login/register | Home, About or story, public chat entry as configured | Public story slide, QR entry CTA, offer teaser | Public copy, public content artifacts, safe QR campaign refs | Member operations, internal evidence, account controls |
| Member/Ordo | Customer, student, affiliate, trial user | Let the member talk to Ordo, accept offers, use access, fulfill requests, track referrals | Ordo, Activity, Offers, Access, Requests, Referrals | Offer path, proof approval, feedback request, affiliate terms, trial access, handoff status | Member-safe evidence refs, accepted offer refs, request refs, candidate artifact refs, safe timeline | Staff routing, provider details, policy internals, system operations |
| Support | Staff, owner acting as staff | Handle relationship work, handoffs, conversations, customer requests, reviews, QA | Handoffs, Conversations, Requests, Reviews, Members | Ava requested Keith handoff, Jordan submitted QA, feedback needs follow-up, consultation scheduling | Conversation events, consent, request state, review evidence, member-safe artifact refs, staff notes where allowed | Growth dashboards, infrastructure control, provider secrets |
| Growth | Owner, growth operator | Show business value, source attribution, offer performance, referrals, content outcomes | Overview, QR/Word of mouth, Offers, Affiliates, Content, Rewards, Reports | Founder Meetup QR converted, Sam referral needs outcome, offer fit high, content produced trials | Attribution events, QR refs, value events, offer events, reward status, content performance refs | Customer support queue, system health, raw private member content |
| System | Owner, admin, system operator | Operate infrastructure, hosted instances, providers, backups, job runs, audit | Health, Events, Hosted instances, Jobs, Backups, Providers, Access, Settings | Hosted trial provisioned, weekly reset due, backup downloaded, restore requested, provider disabled | System events, job runs, backup refs, provider status, audit events, policy decisions | Member-facing persuasion, growth storytelling, unsupported public claims |

## Member Rooms

| Room | Member-facing purpose | Layout | Work item source | Example items | Proof | Do not put here |
| --- | --- | --- | --- | --- | --- | --- |
| Ordo | One relationship conversation with Ordo. Staff can take over without creating a separate member-facing channel. | Chat stage only | Conversation and safe handoff status | "Ava asked which path fits"; "Keith handoff requested" | Conversation events, safe handoff status, entry refs | Separate support inbox, staff routing, worklist clutter |
| Activity | Cross-room feed of what needs attention now. | Worklist plus selected detail | Actionable projections from all member rooms | Offer question, QR proof approval, feedback request, affiliate terms | Source object refs, status, timeline, action refs | Long reports, completed history by default |
| Offers | Paths the business invites the member to accept, buy, join, or unlock. | Worklist plus selected detail | Offer, offer path, recommendation, fit check | Strategic consultation, hosted 30-day trial, training access, affiliate path | Offer refs, entry refs, conversation refs, terms refs | Requests for member approval, delivered access |
| Access | Accepted offers, granted tools, content, services, hosted trial access, and enabled capabilities. | Worklist plus selected detail | Accepted offer, grant, capability, artifact, hosted instance | Hosted 30-day trial, training resources, consultation prep, QR proof candidate | Grant refs, accepted offer refs, artifact refs, hosted instance refs | Offers not accepted, support requests |
| Requests | Things the business needs the member to do, approve, confirm, review, provide, schedule, or resolve. | Worklist plus selected detail | Request, approval, feedback, scheduling, QA, consent | Approve QR proof, complete private feedback, pick consultation time, confirm backup downloaded | Request refs, consent refs, artifact refs, timeline refs | A member asking for help as a separate support surface |
| Referrals | Affiliate/referral participation, terms, links, QR codes, outcomes, rewards. | Worklist plus selected detail | Referral path, affiliate terms, tracked link, QR asset, outcome, reward | Affiliate terms needed, referral link ready, QR kit candidate, reward pending outcome | Affiliate grant refs, terms refs, attribution refs, outcome refs | General marketing dashboard, unrelated customer data |

## Work Item Contract

Every worklist item should use one shape, regardless of room.

```ts
interface OrdoWorkItem {
  id: string;
  shell: "member" | "support" | "growth" | "system";
  room: string;
  objectType:
    | "conversation"
    | "offer"
    | "access"
    | "request"
    | "referral"
    | "handoff"
    | "review"
    | "qa_issue"
    | "hosted_instance"
    | "job"
    | "artifact"
    | "value_event"
    | "system_event";
  title: string;
  summary: string;
  status: string;
  readState: "unread" | "read";
  sourceLabel: string;
  occurredAt: string;
  primaryAction: OrdoAction;
  secondaryActions: OrdoAction[];
  evidenceRefs: OrdoProofRef[];
  relatedObjectRefs: string[];
  stage: OrdoStageDetail;
}
```

The visual row should show only what is needed:

- title;
- one-line summary;
- source and time;
- read or status state;
- one compact primary action.

## Stage Detail Contract

The selected detail stage is an evidence reader and decision surface. It is not a
floating card and not a generic room summary.

```ts
interface OrdoStageDetail {
  eyebrow: string;
  title: string;
  summary: string;
  whyItMatters?: string;
  recommendedAction?: string;
  actions: OrdoAction[];
  timeline: OrdoTimelineEvent[];
  proof: OrdoProofRef[];
  relatedObjects?: string[];
}
```

Required stage order:

1. Compact eyebrow and status row.
2. Restrained title.
3. One concise summary.
4. Why it matters, only when it changes the decision.
5. Action row.
6. Timeline.
7. Proof and provenance.

Do not add an extra metadata strip, ornamental top rule, duplicated badges, or
generic room summary when a selected item exists.

## Proof Reference Contract

Proof is a typed evidence reference. The UI can render it, filter it, and
summarize it, but proof identity comes from the object or daemon layer.

```ts
interface OrdoProofRef {
  id: string;
  kind:
    | "entry_point"
    | "visitor_session"
    | "conversation_event"
    | "offer_event"
    | "offer_acceptance"
    | "access_grant"
    | "request"
    | "feedback"
    | "handoff"
    | "review"
    | "job_run"
    | "artifact"
    | "hosted_instance_event"
    | "backup"
    | "restore"
    | "attribution_event"
    | "value_event"
    | "audit_event"
    | "policy_decision"
    | "consent"
    | "terms";
  durability: "candidate" | "durable" | "missing";
  visibility: "public" | "member" | "support" | "growth" | "system";
  summary: string;
}
```

## Scenario Maps

These tables define how the same business flow appears across shells. They are
not separate workflows; they are role-safe projections of shared objects.

### Scenario 1: Meetup QR Visitor Starts A Conversation

| Shell | Room | Work item | Member or operator action | Proof |
| --- | --- | --- | --- | --- |
| Member | Ordo | Primary relationship conversation starts from Founder Meetup Intro QR | Ask Ordo which path fits | entry_point, visitor_session, conversation_event |
| Member | Activity | "Ava asked which path fits" | Reply or compare paths | conversation_event, offer_event |
| Member | Offers | "Choose the right Studio Ordo path" | View consultation, trial, training, affiliate path | entry_point, offer_event |
| Support | Conversations | Ava conversation is active | Reply or start handoff if requested | conversation_event, handoff if created |
| Support | Handoffs | Only appears if Ava asks to talk to Keith | Accept handoff, assign, return to Ordo | handoff, conversation_event |
| Growth | QR/Word of mouth | Founder Meetup Intro QR produced a conversation | Inspect source and conversion path | attribution_event, value_event |
| System | Events | Entry session and conversation events created | Inspect event replay | entry_point, visitor_session, audit_event |

### Scenario 2: Hosted 30-Day Ordo Trial

| Shell | Room | Work item | Member or operator action | Proof |
| --- | --- | --- | --- | --- |
| Member | Offers | "Try Ordo for 30 days" | Accept trial or ask a question | offer_event, conversation_event |
| Member | Access | "Hosted 30-day trial" | Open trial access, review reset policy | offer_acceptance, access_grant, hosted_instance_event |
| Member | Requests | "Approve QR card proof" or "Confirm backup policy" | Approve, request changes, confirm | request, artifact, consent |
| Support | Requests | Trial setup or proof approval needs follow-up | Prepare answer, nudge, resolve | request, conversation_event |
| Growth | Offers | Hosted trial conversion attributed to QR | Measure offer fit and source quality | attribution_event, offer_acceptance, value_event |
| System | Hosted instances | Trial instance provisioned or waiting | Reserve, reset, restore, audit | hosted_instance_event, job_run, backup |

### Scenario 3: Feedback And QA During Trial

| Shell | Room | Work item | Member or operator action | Proof |
| --- | --- | --- | --- | --- |
| Member | Requests | "Complete private feedback request" | Respond to feedback request | request, feedback, consent |
| Member | Ordo | Member asks for help or reports a problem in chat | Describe issue, attach safe evidence | conversation_event, feedback |
| Support | Requests | Feedback needs review | Read, respond, ask follow-up, mark resolved | feedback, conversation_event |
| Support | Reviews | Only public review candidates after consent and approval | Approve or decline public use | review, consent, policy_decision |
| Growth | Reports | Feedback indicates product learning or conversion risk | Track learning value and retention signal | value_event, feedback |
| System | Jobs/Events | QA issue links to runtime, browser, backup, or job evidence | Inspect logs or job state | job_run, hosted_instance_event, audit_event |

Feedback and QA stay in Member Requests. Do not create a separate member QA
room.

### Scenario 4: Member Requests A Keith Handoff

| Shell | Room | Work item | Member or operator action | Proof |
| --- | --- | --- | --- | --- |
| Member | Ordo | Safe status: "Keith handoff requested" | Continue conversation | conversation_event, handoff status |
| Member | Activity | Handoff status may appear if it needs member attention | Wait or add context | handoff status, conversation_event |
| Support | Handoffs | "Ava requested Keith while online" | Accept, assign, take over, return to Ordo | handoff, staff_action, conversation_event |
| Support | Conversations | Same member conversation, staff-safe projection | Reply as staff without exposing internals | conversation_event, staff_note where allowed |
| Growth | Overview | Handoff may count as high-intent conversion signal | Inspect only aggregate/source-safe signal | value_event, attribution_event |
| System | Events | Mode and handoff events are durable | Replay, audit, debug | audit_event, policy_decision |

The member must not see staff routing, provider internals, confidence, or private
moderation mechanics.

### Scenario 5: Strategic Consultation Offer And Scheduling

| Shell | Room | Work item | Member or operator action | Proof |
| --- | --- | --- | --- | --- |
| Member | Offers | "Strategic consultation" | Accept, ask fit question, compare with trial | offer_event, conversation_event |
| Member | Requests | "Pick a consultation time" | Select time, confirm prep questions | request, consent |
| Member | Access | "Consultation prep" after acceptance | Review prep brief or meeting details | offer_acceptance, access_grant, artifact |
| Support | Requests | Consultation scheduling needs action | Schedule, ask follow-up, send prep | request, conversation_event |
| Growth | Offers | Consultation recommended from QR or content | Track offer recommendation and conversion | offer_event, attribution_event, value_event |
| System | Events | Schedule request and artifact generation | Audit or replay | audit_event, job_run |

### Scenario 6: Training Or Student Access

| Shell | Room | Work item | Member or operator action | Proof |
| --- | --- | --- | --- | --- |
| Member | Offers | "Training access" | Accept training path | offer_event |
| Member | Access | "Student tools and resources" | Open lesson, tutoring, assignment feedback | access_grant, artifact |
| Member | Requests | "Submit assignment for feedback" or "Confirm session goal" | Upload, answer, approve | request, artifact, feedback |
| Support | Conversations | Student asks for guidance through Ordo | Reply or route to instructor | conversation_event, handoff |
| Growth | Revenue/Offers | Training conversion and retention | Track value, source, outcomes | offer_acceptance, value_event |
| System | Jobs | Content/job artifacts and submissions | Process, store, validate | job_run, artifact, audit_event |

### Scenario 7: Affiliate Or Referral Path

| Shell | Room | Work item | Member or operator action | Proof |
| --- | --- | --- | --- | --- |
| Member | Referrals | "Affiliate terms are needed" | Review terms | terms, policy_decision |
| Member | Referrals | "Referral link is ready" | Copy link or QR after approval | access_grant, attribution_event |
| Member | Activity | Reward or outcome needs attention | View evidence or respond | value_event, attribution_event |
| Support | Requests | Affiliate question needs answer | Reply or escalate | conversation_event, request |
| Growth | Affiliates | Sam or Ava produced trial/conversation outcome | Attribute, review reward readiness | attribution_event, value_event, reward |
| System | Access | Affiliate grant and revocation state | Validate, revoke, audit | access_grant, audit_event |

### Scenario 8: Content Or Artifact Production

Studio and Knowledge are not the current member focus, but the business process
must leave room for them.

| Shell | Room | Work item | Member or operator action | Proof |
| --- | --- | --- | --- | --- |
| Member | Access | "Training short" or "lesson artifact" is available | View or download | access_grant, artifact |
| Member | Requests | "Review generated short" if member approval is needed | Approve or request changes | request, artifact |
| Support | Requests | Customer content approval needs follow-up | Nudge or resolve | request, conversation_event |
| Growth | Content | Short drove QR scans or trials | Measure performance and value | attribution_event, value_event |
| System | Jobs | Render job completed or failed | Inspect job, retry, audit | job_run, artifact, audit_event |

## Offer, Request, Access, And Activity Semantics

### Offer

An offer is a business invitation.

Examples:

- hosted 30-day Ordo trial;
- strategic AI consultation;
- training/student access;
- affiliate/referral path.

Required data:

- offered_to;
- source;
- terms or price state;
- status;
- accepted_at when accepted;
- evidence refs.

### Request

A request is something waiting on a person.

Examples:

- approve QR card proof;
- complete private feedback;
- pick a consultation time;
- confirm backup downloaded;
- provide assignment material;
- review affiliate terms.

Required data:

- requested_by;
- requested_of;
- due or urgency state;
- status;
- allowed actions;
- evidence refs.

### Access

Access is an accepted offer or granted capability.

Examples:

- hosted trial instance;
- consultation prep;
- training resources;
- affiliate kit;
- generated artifact.

Required data:

- source offer or grant;
- available actions;
- expiry/reset state if any;
- evidence refs.

### Activity

Activity is the cross-room attention feed.

Examples:

- Ava asked which path fits;
- QR proof needs approval;
- feedback request waiting;
- affiliate terms needed;
- hosted trial resets soon.

Required data:

- source object;
- reason it is in Activity;
- current action;
- evidence refs.

## Layout Rules

### Member Ordo Room

```text
[ rail ][ room drawer ][ chat stage ]
```

The member has one primary relationship conversation with Ordo. Staff can take
over in Support, but the member does not manage multiple staff channels.

### Other Member Rooms

```text
[ rail ][ room drawer ][ worklist ][ selected detail stage ]
```

The room drawer answers "where am I?" The worklist answers "what needs
attention?" The selected stage answers "what am I deciding or reviewing?"

### Operator Shells

Support, Growth, and System should use the same pattern:

```text
[ rail ][ shell drawer ][ worklist ][ selected detail stage ]
```

Different shells change room navigation and projections, not the primitive
grammar.

## Copy Rules

Use concrete business process language.

Prefer:

- "Approve QR card proof"
- "Complete private feedback"
- "Try Ordo for 30 days"
- "Review affiliate terms"
- "Founder Meetup Intro QR"
- "Hosted trial resets Sunday"

Avoid:

- "review your state"
- "mocked content"
- "current room context"
- "generic dashboard"
- "CRM object"
- "QA room" in the member shell

## Acceptance Gates

Future member and operator UI should not be accepted unless these gates hold.

- Member Ordo is chat-first and does not show a redundant worklist.
- Member requests include feedback and QA asks.
- Offers are invitations to accept, buy, join, or unlock.
- Access items are backed by accepted offers or explicit grants.
- Activity items point to a source object and evidence.
- Every selected detail has actions, timeline, and proof.
- Support handoff details are staff-safe and never leak into member views.
- Growth metrics cite attribution or value-event evidence.
- System operational claims cite system events, job runs, backups, provider
  state, or audit events.
- Candidate artifacts remain candidate until daemon validation gives durable
  identity.
- Role-safe projections decide visibility before rendering.

