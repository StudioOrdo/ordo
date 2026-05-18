# My Ordo Attention Model

Status: target architecture for member-facing clarity

My Ordo is the member operating surface. It should make Ordo understandable to
a busy non-technical person who needs to know what is safe, what needs a human,
and what happens next.

## Product Role

My Ordo is not a generic inbox and not a chat-only page. It is the personal
cockpit for any member in the system:

- owner;
- staff;
- customer;
- trial user;
- affiliate;
- support person;
- network member.

The default question is:

```text
What needs me now?
```

## Recommended Rooms

Use one term per concept in member UI.

```text
Activity
Offers
Requests
Capabilities
Chat
```

Migration aliases may exist, but member-facing copy should converge:

- `asks` -> `requests`;
- `access` and `packs` -> capability/access facets;
- `handoffs`, `reviews`, `approvals`, and `repairs` -> Request types.

## Activity

Activity is the first-screen summary of what matters. It should combine:

- requests needing action;
- conversations waiting on the member;
- offer opportunities;
- workflow state changes;
- support handoff state visible to that member;
- readiness or review items;
- receipts for completed decisions.

Activity should not be a raw event log.

## Requests

Request is the member-friendly word for work or decisions routed to a person.

Examples:

- approve a draft;
- review a memory readiness packet;
- claim a support handoff;
- answer a missing workflow input;
- accept or decline a relationship request;
- inspect a system issue.

The member sees safe state and allowed actions. Internal WorkItem,
DecisionQueueItem, policy, provider, prompt, or staff routing details stay
behind protected surfaces.

## Offers

Offers are governed entry points. Accepting an offer may grant access to:

- capabilities;
- packs;
- support;
- reports;
- services;
- workflows;
- trials;
- network membership.

Offer acceptance should create auditable state and route follow-up requests
rather than behaving like a plain message.

## Capabilities

Capabilities explain what the member can currently do.

The UI should use plain labels. The daemon should still authorize against
stable capability ids such as `support.accept_handoff`.

## Chat

Chat is a relationship interface, not the product spine. Chat can help the
member ask questions, respond to requests, or understand receipts, but durable
work still moves through Requests, Offers, Capabilities, Jobs, Artifacts,
Events, and policy.

## Attention Classification

The daemon should classify events before delivery. Initial taxonomy:

```text
critical_interrupt
human_decision_required
time_sensitive_opportunity
conversation_waiting_on_you
workflow_state_change
digest_candidate
autonomous_resolution_log
noise_drop
```

Delivery guidance:

- immediate attention for critical interrupts and human decisions;
- digest for non-urgent opportunities and summaries;
- silent timeline for routine workflow changes;
- drop noisy duplicates.

Notifications are delivery. They must link back to canonical My Ordo items and
must not become a separate workflow engine.

## Action Button Policy

Allowed compact actions:

```text
review_now
approve
request_changes
accept_offer
reply_now
snooze
escalate
view_receipt
```

Rules:

- max two compact actions in notification-like contexts;
- one primary action;
- every action maps to a governed endpoint;
- actions are idempotent;
- stale or unauthorized actions route to item detail with rationale;
- risky actions require in-app review.

## Minimum Item Shape

Future My Ordo read models should expose:

```text
item_id
room
title
summary
status
priority
source
occurred_at
timeline[]
evidence_refs[]
actions[]
visibility
```

This shape can initially be a projection over existing tables. It does not
require a canonical Request storage rewrite before the product is ready.

## Plain-Language Requirements

For non-technical members, every item should answer:

- What is this?
- Why am I seeing it?
- Is this safe?
- What happens if I click?
- What is still waiting on a human?
- What evidence or receipt exists?

Avoid member-facing words such as:

- DAG;
- provider internals;
- prompt;
- policy object;
- graph certainty;
- candidate body;
- promotion unless promotion is actually happening;
- publication unless publication is actually happening.

## Safety Invariants

- My Ordo must not expose staff-only routing or owner-only evidence.
- Public/member surfaces must not expose provider keys, prompt internals, raw
  policy, private intake text, private candidate text, or unsupported claims.
- Readiness must not sound like promotion.
- Review must not sound like publication.
- Generated analysis must not appear as source truth.
- Every human decision must be auditable.
