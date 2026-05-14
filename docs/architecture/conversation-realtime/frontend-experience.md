# Conversation Frontend Experience

Status: Product and interaction contract with the first premium UI core,
recovery/accessibility hardening, live journey evals, and OrdoOS frontend
foundation implemented. The active follow-on arc is 0.1.8 Interactive Account
And LLM Chat, which connects local account entry, chat bootstrap, browser
`/chat/ws`, deterministic LLM chat, guarded live-provider testing, UI run
states, and smoke evidence before broader onboarding surfaces resume.

Current canon: [Current Product Canon](../../business/current-product-canon.md)

The conversation UI should feel fast, polished, and emotionally legible while
remaining operational. It should be the primary working surface, not a landing
page and not a support widget pasted into the appliance.

## Experience Principles

- Default to a narrative brief/detail, not a generic dashboard panel.
- Preserve the product split: members participate; Studio, Support, Knowledge,
  Growth, and Systems operators work from their respective surfaces; roles only
  control permissions and projections.
- Local echo immediately; daemon truth reconciles quietly.
- Every state has a visual answer: pending, sent, delivered, read, failed,
  edited, deleted, streaming, waiting for approval, offline, recovered.
- Motion and microinteractions should clarify state, not distract from work.
- The UI should feel dense enough for repeated use and refined enough to feel
  premium.
- AI activity should be visible without exposing private prompts, raw retrieved
  text, or secret policy details.
- Read/unread and action-needed should be separate concepts.
- Mobile should be first-class, not a compressed desktop afterthought.

## Navigation Model

The primary information architecture is:

```text
Main area menu -> Evidence/record list -> Narrative brief/detail
```

Mobile:

```text
Menu -> Area evidence list -> Selected record brief
```

Desktop:

```text
Surface rail + room tabs + evidence list + narrative brief/detail
```

The canonical product surfaces are:

```text
Member View | Studio | Support | Knowledge | Growth | Systems
```

Roles control which surfaces, rooms, records, and actions are visible.
Non-staff users must never see Support queues, Studio production controls,
Knowledge internals, Growth operator evidence, or Systems appliance internals
unless a specific grant creates a safe projection. Ordinary business staff
should not see logs, backup, events, readiness, or low-level appliance internals
as primary navigation.

## Primary Surfaces

### Interactive Account And LLM Chat

0.1.8 should make the relationship conversation usable as a real local
interactive loop before broadening the public/client dashboard surface area. The
active route sequence is:

```text
login/register -> local session -> chat bootstrap -> browser /chat/ws ->
durable user message -> daemon LLM request -> assistant message -> replay-safe
UI
```

The canonical implementation plan lives in
`interactive-account-llm-chat.md`. Each slice should consume real backend
contracts where available and remain client-safe:

- Login and Register create or restore only a local appliance session.
- Chat bootstrap resolves canonical conversation and participant identity before
  the browser sends websocket commands.
- The member chat surface uses `/chat/ws` for identify, subscribe, submit,
  replay, and recoverable error states.
- LLM chat defaults to deterministic provider behavior and runs live provider
  calls only behind explicit guards.
- Member surfaces preserve scoped evidence without exposing
  unrelated customer data.
- Staff/admin review surfaces inspect handoffs, review moderation, affiliate
  state, and journey reports without making ordinary staff operate the
  appliance.

### Client Relationship Conversation

Clients and members should see one persistent relationship conversation with
the business. They should not see tickets, thread IDs, internal handoff states,
confidence scores, policy state, or LLM orchestration.

Client-facing labels should be plain:

- Your conversation with Studio Ordo.
- A Studio Ordo team member is reviewing this.
- Ordo Assistant is available.

### Support Conversation Queues

Support operators should not default to an all-conversation surveillance feed.

Primary views:

- `My Handoffs`
- `Team Queue`
- `All Conversations`

Business staff default to `My Handoffs`. Manager/admin roles may default to
`Team Queue`. Owners may default to a Systems brief or Team Queue. `All
Conversations` is available only to roles with the right scope.

### Affiliate Chat And Account Tools

Affiliates use the client/member conversation surface, not staff queues. Their
account tools may include Affiliate dashboard, Referral links, QR card,
Referred leads, Outcome/commission status, Approved materials, and Settings.
Affiliate chat cards may show referral captured, lead qualified, ask matched,
offer accepted by referred connection, and outcome recorded when those facts are
authorized and evidence-backed.

### Customer Feedback

Customer Feedback is a business staff area, not an admin appliance screen. It
uses the same area pattern:

```text
Customer Feedback -> feedback evidence list -> feedback brief/detail
```

The default detail is a narrative Feedback Brief: what customers are saying,
what is working, what needs attention, review candidates, recommended actions,
and evidence. Feedback rows should answer who gave the feedback, type/signal,
whether it needs action, whether it can become a review, and the source object.

Selected feedback detail should show meaning, suggested action, source,
connection, conversation episode/segment, related offer, ask, artifact, outcome,
status, and actions such as star, tag, request review, attach, create job, open
conversation, approve/reject review candidate.

### Reviews And Public Proof

Feedback is private business intelligence until consent turns it into a review.
A testimonial is a curated/published review used in Home/About, Offers, or
Latest. Review request and publication UI must make consent, display-name
preference, approval, publication, featured, and retired states visible.

### Home/About Narrative Brief

Home/About is the public narrative brief of the business. It should use
scrollytelling billboards only when motion clarifies state and reduced-motion
fallbacks preserve the same information. Billboards may summarize Offers, Asks,
Latest, Artifacts, Customer Feedback, Reviews, Outcomes, and Chat, but every
claim must link to evidence or be clearly aspirational. Owner-governed states
are pinned, dynamic, draft, published, and retired.

### Offers And Asks As Intent Objects

Offer and Ask surfaces should remain human-readable while preserving enough
structure for future machine-readable business intent: what is provided or
wanted, who qualifies, required inputs, produced artifact/deliverable, terms,
approval needs, how to start/respond, and outcome/referral links. Do not expose
external A2A controls until those contracts exist.

### Conversation List And Queue Rows

The list should show:

- participant or conversation label;
- why this is in the queue;
- urgency;
- handoff status and assigned actor where relevant;
- last message preview with privacy-safe redaction;
- unread count;
- mention count;
- action-needed count;
- live presence state;
- handoff waiting indicator;
- AI/tool approval waiting indicator;
- last activity time;
- muted/paused/closed state.

Rows should prioritize last meaningful change over raw activity when a handoff,
approval, offer/ask outcome, or customer action matters more than the latest
chat text.

### Narrative Brief Detail

Each selected record should default to a brief before the raw transcript or
admin detail. The brief should answer:

- what is going on;
- what changed;
- what to do next;
- why it matters;
- evidence;
- limitations and provenance.

Handoff detail should show the handoff brief before the transcript.

### Conversation Timeline

The timeline should show:

- grouped messages by speaker and time;
- date and unread separators;
- stable scroll anchoring during streaming;
- inline receipts where useful, collapsed by default;
- reactions;
- replies or lightweight thread affordances;
- edited and deleted states;
- artifact cards for briefs, offers, support packets, citations, approvals, and
  tool results;
- AI activity chips inside the timeline when relevant.

### Composer

The composer should support:

- optimistic send;
- enter to send and shift-enter for newline on desktop;
- mobile send button with stable size;
- attachment/artifact insertion when available;
- slash or command palette actions for owner workflows;
- visible disabled state when policy blocks sending;
- retry after failed send;
- draft preservation locally, without daemon persistence unless explicitly
  saved by a future feature.

## Message States

| State | UI behavior |
| --- | --- |
| pending | Message appears immediately with subdued opacity and a local spinner/check. |
| persisted | Pending styling clears after daemon ack and canonical id arrives. |
| delivered | Delivery check or subtle receipt appears for sender. |
| displayed | Optional intermediate state for multi-device precision. |
| read | Avatar, check, or compact read receipt appears only at the latest read point. |
| failed | Message stays in place with retry and inspect affordance. |
| edited | Small edited label with revision history available to owner/admin views. |
| deleted | Tombstone remains when needed for timeline integrity. |
| blocked | Policy-rejected message does not enter durable history; local composer shows reason. |

## Typing Indicators

Typing should be calm and high quality:

- show participant avatar or label plus animated dots;
- collapse multiple typers into a compact sentence;
- remove automatically after expiry;
- do not shift layout repeatedly;
- do not show typing for users who disabled typing indicators;
- use separate assistant states for thinking, retrieving, using tools, waiting,
  and responding.

Assistant activity labels should be short:

- `Thinking`
- `Checking evidence`
- `Reviewing policy`
- `Using tool`
- `Waiting for approval`
- `Responding`

These labels should be driven by daemon events rather than guessed by the UI.

## Read And Unread

Unread should be precise and useful.

The UI should support:

- unread divider in the timeline;
- jump to first unread;
- jump to latest;
- mark conversation read;
- mark unread from message;
- unread mention count;
- action-needed count;
- conversation list badges;
- global shell badge for total attention.

Do not treat every unread message equally. Owner attention should distinguish:

- unread;
- mentioned;
- needs reply;
- needs approval;
- handoff waiting;
- tool approval waiting;
- privacy review needed.

## AI Streaming

AI output should stream smoothly without creating hundreds of durable rows.

Recommended UI behavior:

- render a live assistant bubble from ephemeral `llm.text.delta` events;
- pin scroll to bottom only when the user is already at bottom;
- preserve user's scroll position if they are reading earlier messages;
- show evidence/tool chips above or below the streaming bubble;
- convert the live bubble to a durable canonical message only after
  `llm.text.completed` or equivalent final event;
- show partial failure with retry and evidence of what completed;
- show token/cost details behind an inspect affordance, not inline noise.

## Agent Etiquette

When a human staff member is actively leading, Ordo does not post publicly
unless tagged, delegated, or policy requires intervention. The UI should make
the current mode legible to staff without exposing internal state to clients.

Staff affordances should support:

- `@Ordo summarize this`;
- `@Ordo find the offer`;
- `@Ordo draft a reply`;
- `@Ordo what do we know about this connection?`;
- `@Ordo take over routine questions`.

If a human-led conversation goes idle, staff should receive a private reminder
before any public holding message or return-to-agent behavior.

## Rich Ordo Cards

Conversation should render Ordo artifacts as compact cards:

- brief generated;
- support packet drafted;
- offer viewed or accepted;
- handoff requested;
- tool approval requested;
- corpus evidence cited;
- privacy transform applied;
- token usage warning;
- backup or system job status when a system conversation invokes appliance work.
- offer/ask outcome recorded;
- referral captured or qualified;
- episode/tag candidate;
- handoff brief generated;
- surface brief refreshed.

Cards should have stable dimensions and clear actions. Avoid nested cards.
Staff-facing cards use `Artifact` and may show producing job, provenance, and
storage/health. Client-facing cards use `Deliverable` only when an artifact is
intentionally exposed; they should show value, use, and next action without
internal storage or policy mechanics.

## Presence

Presence visible to participants should be policy-filtered.

Owner/operator states:

- Available;
- Here;
- Focused;
- Away;
- Paused;
- Offline.

Visitor-facing language can be softer:

- Available now;
- Replies soon;
- Currently paused;
- Handoff requested.

Do not expose raw device count, private status messages, or exact activity to
public participants unless policy allows it.

## Connection Recovery

The UI should make recovery feel safe:

- show small degraded banner only when needed;
- keep composer usable for local draft input;
- queue eligible commands with local pending state;
- replay missed durable events after reconnect;
- reconcile pending commands by `clientId`;
- show `Recovered` briefly after replay catches up.

## Accessibility

The chat UI should include:

- keyboard navigation through timeline and composer;
- accessible labels for icon buttons;
- aria-live regions for new messages and connection status, with restraint;
- reduced motion support;
- sufficient color contrast for badges and receipts;
- readable focus rings;
- stable touch targets on mobile.

## Mobile Behavior

Mobile should prioritize:

- fast open to latest conversation;
- sticky composer with safe-area handling;
- bottom sheet actions for reactions, message actions, and artifact details;
- predictable keyboard resizing;
- no layout jumps when typing indicators or receipts appear;
- one-handed send and reaction flows.

## Ethical Persuasion In The UI

Persuasion guidance is staff-facing decision support, not hidden pressure. If
Ordo suggests persuasive language for an offer, ask, reply, or handoff, staff
views should be able to inspect the evidence and reasoning. Client-facing text
should remain plain, respectful, and agency-preserving.

The UI should never display fake scarcity, invented social proof, unsupported
authority claims, or urgency that is not backed by real constraints.

Implemented behavior: staff conversation detail can render an `Ethical business
persuasion` guidance panel that shows the `ethical_business_persuasion` slot
version, use case, principles, staff reasoning, evidence refs, source refs, and
client-safe suggestion. The same panel is omitted from client/public chat
surfaces so ordinary users do not see prompt slot mechanics or internal
reasoning.

## Non-Goals For Initial UI

- No marketing hero page.
- No voice/video controls in the initial UI. Future voice, phone, and
  SMS/Twilio controls should sit on the same text-first conversation, request,
  artifact, and approval contracts.
- No decorative chat gimmicks that obscure state.
- No public multi-operator inbox before mediated conversation basics are solid.
- No persistent draft sync until privacy and retention rules are explicit.
- No generic CRM dashboard.
- No fake analytics, fake urgency, or hidden persuasion mechanics.

## Implemented UI Core

The first implementation slice renders the conversation product surface on
`/chat` and `/conversations` using the `conversation.gateway.v1` contract as the
frontend command model. The browser smoke fixture uses deterministic local
gateway behavior instead of a live provider so it can validate UI state without
inventing a parallel protocol.

Implemented behavior:

- client chat keeps one relationship conversation and hides staff/admin rails;
- staff conversation work starts from queues and opens a narrative handoff brief
  before the transcript;
- timeline states render pending-style local echo, persisted/read receipts,
  edited labels, undo tombstones, reactions, unread divider, presence, typing,
  and structured gateway rejection with retry;
- composer supports optimistic send, enter-to-send, disabled state while
  sending, command rejection, and retry reconciliation by `clientId`;
- recovery UI covers offline, command pending, replay, recovered, and rejected
  states, with pending optimistic messages reconciled by `clientId` rather than
  duplicated;
- timeline controls support jump to first unread and jump to latest anchors;
- mobile composer uses sticky safe-area behavior and stable controls;
- primary message actions have explicit accessible labels, visible focus rings,
  and reduced-motion behavior keeps state legible without relying on smooth
  scrolling;
- product surfaces can render the latest completed surface brief before raw
  surface detail while showing a queued/running/failed refresh state that does
  not block the rest of the page;
- staff conversation detail can render evidence-backed ethical persuasion
  guidance while client chat hides internal prompt mechanics;
- smoke coverage exercises desktop and mobile layout, edit, undo, reactions,
  mark read/unread affordances, typing/presence, retry, recovery/replay,
  reduced motion, no horizontal overflow, and role-gated navigation.

Deferred behavior:

- live daemon WebSocket binding in the browser hook;
- provider/LLM streaming and tool approval UI;
- durable attachment/artifact insertion;
- delivered/displayed precision across multiple devices;
- persistent draft sync;
- device-lab coverage for mobile keyboard viewport behavior beyond automated
  browser smoke checks.
