# Conversation Realtime Architecture

Status: Draft contract for the next mediated chat and AI streaming slice

Ordo chat should be a high-performance local conversation system, not a widget
bolted onto the System shell. The long-term goal is a luxurious, low-latency,
Discord-class conversation experience where messages, AI streaming, typing,
presence, read/unread, receipts, approvals, privacy transforms, token usage, and
analysis all share one governed event spine.

## Architectural Intent

The conversation layer should support:

- one canonical client-visible relationship conversation per user or visitor
  identity, with internal episodes/segments for topics, sessions, handoffs, and
  provider runs;
- role-aware surfaces where clients participate, staff operate handoff queues,
  and admins operate the appliance;
- brief-first read models where the default detail surface answers what is
  happening, what changed, what to do next, why it matters, evidence, and
  limitations;
- bidirectional browser-to-daemon WebSocket commands;
- durable message history and replay after reconnect;
- ephemeral typing, presence, draft, and AI activity indicators;
- precise read/unread and delivery/read receipts;
- central control over every external LLM call;
- reversible privacy placeholders before provider egress;
- token accounting by prompt slot, model, provider, capability, conversation,
  user, and business purpose;
- continuous conversation analysis for episodes, tags, summaries, briefs,
  handoff decisions, knowledge graph candidates, ethical recommendation
  candidates, and action detection;
- Customer Feedback and Review as evidence-backed business intelligence that
  can become public proof only through consent and approval;
- Home/About as a public narrative brief composed from offers, asks, latest
  activity, artifacts, reviews, outcomes, and chat calls to action;
- Offers and Asks as both human-readable pages and future machine-readable
  business intent objects;
- UI behavior that feels instant while reconciling to daemon truth.

## Current System Anchors

The existing implementation already has several reusable foundations.

| Current anchor | Existing responsibility | Conversation use |
| --- | --- | --- |
| `RealtimeEvent` in `crates/ordo-daemon/src/events.rs` | Durable event shape with global cursor replay. | Extend with conversation family events and per-conversation sequencing. |
| `/events` replay | Cursor-based replay for persisted events. | Recover missed durable chat events and receipt changes after reconnect. |
| `/ws` in `crates/ordo-daemon/src/server.rs` | Outbound broadcast of persisted events. | Evolve into a bidirectional gateway or add `/chat/ws` for commands plus fanout. |
| Capability catalog | Registered capabilities and execution policy. | Every chat command, LLM run, tool approval, and analysis job binds to a capability. |
| Policy audit | Durable actor/action/resource/capability decisions. | Persist authorization evidence for message submission, handoff, model calls, and tool use. |
| Provider configs | Local provider records for Anthropic, OpenAI, DeepSeek, and local providers. | Source of enabled model configuration; keys remain write-only and controlled by daemon. |
| Knowledge corpus and answer drafts | Access-aware retrieval and local evidence draft scaffold. | Retrieval slot for chat answers and briefable conversation summaries. |
| Connections foundation | Relationship records, grants, events, receipts. | Conversation participants can resolve to connections and inherit explicit grants. |
| Availability and handoff | Operator presence, handoff eligibility, inbox items, receipts. | Live handoff state and owner attention events appear inside conversation surfaces. |

## Runtime Boundaries

| Layer | Responsibility |
| --- | --- |
| Rust daemon | Conversation command validation, durable messages, receipts, read state, replay, presence room broker, LLM gateway, privacy transform, token ledger, analysis scheduling, policy audit, and SQLite migrations. |
| Next.js | Chat UI, optimistic local rendering, read models, keyboard/mobile interactions, accessibility, route composition, and visual states. |
| SQLite | Source of truth for conversations, participants, messages, receipts, read state, analysis outputs, token ledger, privacy mappings, and durable events. |
| WebSocket | Live projection and bidirectional command transport. It does not replace SQLite. |
| External LLM providers | Only receive daemon-mediated, policy-approved, privacy-transformed prompts. They never receive direct browser traffic. |

## Product Intelligence Surfaces

Ordo's product shape is broader than chat. Conversation is the active surface,
but business intelligence comes from the links between conversations,
relationships, offers, asks, artifacts, feedback, reviews, referrals, outcomes,
briefs, and jobs.

Customer Feedback should be modeled as private business intelligence first. It
can cite conversation messages, segments, offers, asks, artifacts, referrals,
and outcomes. Reviews are consented/published feedback; testimonials are
curated public proof. Feedback-derived tags and graph links remain candidates
until confirmed.

Home/About should be a public narrative brief made of billboards. Each
billboard needs linked evidence and a detail target. Generated billboard drafts
must be owner-governed through pinned/dynamic/draft/published/retired states so
the public page does not shift unpredictably.

Offers and Asks should remain pages people can read and intent objects future
agents can inspect. External A2A is out of scope for the current milestone, but
the data and eval language should not reduce offers/asks to static marketing
copy.

## Control Point

All external LLM traffic must flow through a Rust-owned LLM Gateway:

```text
Next chat UI
  -> daemon conversation command
  -> policy decision
  -> prompt builder slots
  -> privacy egress firewall
  -> provider adapter
  -> normalized stream events
  -> durable final message/artifact
  -> token ledger and analysis jobs
```

Next.js must not call Anthropic, OpenAI, DeepSeek, or local model endpoints
directly. MCP must not introduce a second provider path. The gateway is the
single point for policy, privacy, usage accounting, provider choice, retries,
tool mediation, and model-output reconstruction.

## Conversation Spine

Conversation actions should follow a predictable path:

```text
Command -> Authorization -> State Mutation -> Durable Event -> Broadcast -> Read Model -> Analysis
```

Example for a user message:

1. Browser sends `message.submit` with a local `clientMessageId`.
2. Daemon authenticates the session and resolves actor/participant.
3. Daemon authorizes `conversation.message.create` against conversation,
   visibility, grants, and handoff state.
4. Daemon persists `conversation_messages` and a durable `message.created`
   event in the same transaction.
5. Daemon emits the persisted event through the conversation room and global
   realtime projection.
6. UI reconciles the optimistic message to the canonical message id and cursor.
7. Daemon queues analysis work for summary, entities, action items, privacy
   classification, unread rollups, and possible LLM response.

## Durable And Ephemeral Split

Durable events rebuild truth. Ephemeral events make the room feel alive.

Durable examples:

- conversation created, closed, paused, resumed, escalated;
- participant joined, left, muted, granted, revoked;
- message created, edited, deleted, pinned, unpinned;
- reaction added or removed;
- delivery/read receipt recorded;
- unread state changed;
- LLM run started, tool requested, tool approved, final text completed;
- token usage recorded;
- privacy transform applied;
- analysis completed;
- brief candidate created.

Ephemeral examples:

- typing started, stopped, expired;
- presence heartbeat;
- participant viewing conversation;
- draft changed without content;
- LLM thinking, retrieving, using tool, streaming delta;
- connection latency and recovery state.

Ephemeral events may be rate-limited, coalesced, and dropped. Durable events may
not be dropped after persistence.

## One Conversation Per User

The product should expose one canonical relationship conversation per client,
member, user, or visitor identity per surface, with episodes/segments underneath
it. Clients should not see fragmented support tickets, threads, internal
routing, confidence scores, policy state, or LLM orchestration.

Examples:

- owner-to-Ordo operator conversation;
- visitor-to-business mediated conversation;
- connection support conversation;
- handoff conversation segment for a specific attention item;
- AI run segment inside a conversation.

This avoids scattering context across many disconnected threads while still
allowing scoped episodes and segments for attribution, privacy, handoff work,
model context, session continuity, and archival.

Anonymous visitors should receive a conversation tied to `visitor_sessions`.
When they become a known connection, the conversation should attach the new
connection identity without losing the visitor session history.

## Role-Aware Product Surfaces

Navigation and read models must separate participation, business work, and
appliance operation.

| Surface | Intended audience | Canonical shape |
| --- | --- | --- |
| Top rail | Public users, clients, members, affiliates, staff, owners | `Studio Ordo`, Chat, Home, Offers, Asks, Latest, Account. |
| Business staff rail | Staff and owner roles | Today, Conversations, Connections, Offers, Asks, Customer Feedback, Affiliates, Artifacts, Jobs, Reports. |
| Admin/system rail | Owner/admin roles | System, Knowledge, Events, Logs, Backup, Settings. |

Ordinary staff should not see health, logs, backup, readiness, events, or other
appliance internals as primary navigation. Staff conversation defaults should be
work queues:

- `My Handoffs`;
- `Team Queue`;
- `All Conversations` for authorized manager/admin/owner views.

`All Conversations` is not the default business staff surface.

## Governed Handoffs And Agent Etiquette

A handoff is a governed object with reason, urgency, assignment, required
capability, allowed context, status, receipts, and evidence. It is not merely a
conversation status label.

Every handoff should show a handoff brief before the transcript. The brief
should cite durable messages/events and explain why the conversation is in the
queue, what the customer wants, what Ordo already said, relevant offers/asks or
artifacts, suggested reply, risk/constraint, and provenance.

When a human staff member is actively leading a conversation, the Ordo agent
must not post publicly unless tagged, delegated, or policy requires
intervention. The daemon should distinguish `agent_led`, `human_led_active`,
`human_led_idle`, `assistive_private`, `needs_handoff`, and
`returned_to_agent` modes. Idle recovery should privately remind the staff
member first; public holding messages or agent takeover require configured
policy and delegation.

## Prompt Builder Slots

Every LLM run should be compiled from named slots. Slots make token accounting,
privacy, access checks, and debugging possible.

Initial slots:

- `system_policy`
- `ordo_identity`
- `ethical_business_persuasion`
- `actor_context`
- `viewer_access_scope`
- `business_truth`
- `retrieval_evidence`
- `conversation_summary`
- `recent_conversation_window`
- `user_request`
- `available_tools`
- `output_contract`
- `egress_limits`

Each slot should record:

- slot id and version;
- source resource ids;
- visibility ceiling;
- policy decision id;
- privacy transform run id;
- estimated tokens before call;
- provider-reported tokens after call when available;
- content hash, not raw prompt content, where possible;
- inclusion reason;
- truncation reason if bounded.

The `ethical_business_persuasion` slot applies Robert Cialdini's principles as
an ethical communication lens only. It may reason about reciprocity,
commitment/consistency, social proof, authority, liking, scarcity, and unity
when evidence supports those signals. It must not invent evidence, fake
scarcity, exploit fear or dependency, hide limitations, present candidates as
facts, or override consent, privacy, safety, or policy.

Implemented v1 behavior keeps that slot daemon-owned. The builder rejects
principles without evidence/source refs, records slot version/source refs/content
hash/token estimates in `llm_prompt_slot_usage`, keeps staff reasoning
inspectable, and emits only client-safe suggestion text to client/public
surfaces. It does not add a public route or direct provider path.

## Privacy Egress Firewall

Before any provider call, the daemon should run a privacy transform:

```text
Detect spans -> classify -> replace with placeholders -> store encrypted map -> send transformed prompt
```

Provider output should only be reconstructed locally for placeholders that match
the current invocation or conversation privacy scope. The first implementation
can use conservative regex and deny-list detectors, but the data model must be
ready for stronger detectors later.

Default placeholder scopes:

- invocation scope for highly sensitive job and public-surface work;
- conversation scope for stable chat pseudonyms;
- actor scope only when a user explicitly allows cross-conversation continuity.

No raw secret, API key, private contact detail, or sensitive identifier should be
written into realtime event payloads, diagnostic logs, or token ledger details.

## Token Ledger

Token accounting should feel like storage analysis on a phone. The owner should
be able to see what uses tokens and why.

Track usage by:

- provider and model;
- conversation and segment;
- participant and actor;
- capability and process template;
- prompt slot;
- input, output, reasoning, cached, and tool tokens;
- estimated cost and pricing snapshot;
- privacy transform run;
- retrieval evidence count and source;
- whether the call produced durable content, analysis, or a discarded draft.

Provider totals are not enough. Ordo should know that, for example, 42% of a
call was retrieval evidence, 23% was conversation history, 12% was tool schemas,
and 8% was policy/system instructions.

## Continuous Analysis

After durable message creation, Ordo should schedule bounded analysis work.
Initial analysis can be local and rule-based; later analysis can use the LLM
Gateway with the same policy, privacy, and token ledger path.

Analysis outputs:

- episode/segment candidates;
- operational tags;
- rolling conversation summary;
- open questions;
- commitments and promised follow-ups;
- handoff eligibility signals;
- offer or trial interest;
- urgency and sentiment;
- privacy and sensitivity classification;
- entities, relationship candidates, and graph candidates;
- offer/ask/referral/outcome attribution candidates;
- ethical recommendation candidates;
- brief candidates;
- corpus memory candidates requiring approval.

Extracted facts are candidates, not truth. They become business truth or corpus
items only through governed approval paths.

## Knowledge Graph Direction

Start with SQLite graph-shaped tables, not a graph database. Keep source of
truth local and inspectable.

Node examples:

- person;
- business;
- conversation;
- episode;
- message;
- offer;
- ask;
- topic;
- problem;
- goal;
- artifact;
- entry point;
- connection.
- referral;
- outcome;
- brief;
- handoff.

Edge examples:

- mentioned;
- asked_about;
- interested_in;
- owns;
- referred_by;
- accepted_offer;
- needs_handoff;
- supports;
- contradicts;
- derived_from.
- cited_by;
- generated_from.

Use in-memory graph libraries later for algorithms if needed, while SQLite
remains the durable record.

## Offers, Asks, Artifacts, And Briefs

Offers and asks are measurable business instruments. Conversation analysis
should eventually connect entry points, QR/link/campaign evidence, referrals,
conversations, artifacts, offer acceptances, ask responses, and outcomes. Do
not expose fake analytics before durable evidence exists.

`Artifact` is the canonical system noun for briefs, reports, exports, backups,
snapshots, generated media, imported knowledge, support packets, offer
materials, QR/card designs, and published content. Client-facing surfaces may
use `Deliverable` when that language is clearer.

Major surfaces should eventually load the latest completed evidence-backed
brief first, then refresh in the background through scheduled or triggered
jobs. Brief generation should be durable, provenance-aware, and non-blocking for
the primary UI.

## Non-Goals For The First Slice

- No voice or video.
- No direct provider calls from Next.js.
- No hosted realtime service dependency.
- No arbitrary model/tool execution through MCP.
- No external chat platform integration.
- No full multi-operator support suite.
- No storing raw typing drafts.
- No broad knowledge graph UI before message, receipt, and analysis foundations
  are durable.
