# Conversation Realtime Data Model

Status: Draft schema plan with backend foundation implemented through daemon
schema versions 19 and 20

The conversation data model should extend the current SQLite appliance schema
through ordered daemon migrations. It should reuse existing actor, role,
resource grant, connection, visitor session, handoff, corpus, answer draft,
provider, and realtime event foundations.

## Current Tables To Reuse

| Table | Conversation role |
| --- | --- |
| `actors`, `roles`, `actor_role_memberships` | Local operator, browser session, MCP client, future public/session actors. |
| `resource_grants` | Conversation and corpus access grants. |
| `connections`, `connection_grants`, `connection_events`, `connection_receipts` | Known relationship identities and grant/receipt history. |
| `tracked_entry_points`, `visitor_sessions`, `visitor_session_events` | Anonymous visitor continuity and attribution. |
| `availability_schedules`, `operator_presence` | Owner availability and interruption threshold. |
| `handoff_inbox_items`, `handoff_events`, `handoff_receipts` | Owner attention and handoff decisions. |
| `corpus_sources`, `corpus_items`, `corpus_items_fts` | Access-aware retrieval evidence. |
| `answer_drafts`, `answer_draft_citations` | Existing local answer scaffold, later conversation-linked. |
| `provider_configs`, `vault_items` | Provider configuration and encrypted API key storage. |
| `policy_decisions` | Durable authorization evidence. |
| `realtime_events` | Global cursor log for replayable event projection. |

## Proposed Tables

The first backend foundation implements the canonical conversation, internal
segment/episode candidate, governed handoff, current mode, and replayable
conversation event tables in schema version 19. Schema version 20 adds
participants, messages, revisions, message artifact links, reactions, receipts,
read states, and presence snapshots for the protocol layer. Tags, analysis,
graph candidates, memory, dedicated LLM ledger tables, and business outcome
tables remain planned for later gateway and LLM work. The first LLM gateway
foundation records run, prompt slot, provider start, usage, terminal state, and
final assistant-message evidence in `conversation_events`, `realtime_events`,
and `conversation_messages` rather than introducing the full ledger schema.

### `conversations`

Stores the durable conversation record.

Columns:

- `id TEXT PRIMARY KEY`
- `surface TEXT NOT NULL`
- `subject_kind TEXT NOT NULL`
- `subject_id TEXT`
- `connection_id TEXT`
- `visitor_session_id TEXT`
- `status TEXT NOT NULL`
- `visibility TEXT NOT NULL`
- `privacy_scope TEXT NOT NULL`
- `current_segment_id TEXT`
- `last_meaningful_change TEXT NOT NULL DEFAULT ''`
- `unread_count INTEGER NOT NULL DEFAULT 0`
- `action_count INTEGER NOT NULL DEFAULT 0`
- `summary_json TEXT NOT NULL DEFAULT '{}'`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `created_by_actor_id TEXT`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`
- `closed_at TEXT`
- `archived_at TEXT`

Indexes:

- `(surface, subject_kind, subject_id, status)` for one-conversation lookup;
- `(connection_id, updated_at DESC)`;
- `(visitor_session_id, updated_at DESC)`;
- `(status, updated_at DESC)`.

Uniqueness:

- one active canonical conversation per `(surface, subject_kind, subject_id)`
  when `archived_at IS NULL` where SQLite partial indexes are acceptable.

### `conversation_segments`

Segments scope product episodes, session windows, handoffs, and provider runs
without fragmenting the canonical client-visible conversation. If product
episodes later need semantics that do not fit this table, add an explicit
episode table rather than preserving an overloaded shape.

Columns:

- `id TEXT PRIMARY KEY`
- `conversation_id TEXT NOT NULL`
- `segment_kind TEXT NOT NULL`
- `title TEXT`
- `source_kind TEXT`
- `source_id TEXT`
- `confidence REAL`
- `candidate_state TEXT`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `provenance_json TEXT NOT NULL DEFAULT '{}'`
- `created_by_job_id TEXT`
- `provider_id TEXT`
- `model_id TEXT`
- `status TEXT NOT NULL`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `started_at TEXT NOT NULL`
- `ended_at TEXT`

Indexes:

- `(conversation_id, started_at DESC)`;
- `(segment_kind, status, started_at DESC)`.

Product episode candidate states use the shared candidate vocabulary:
`proposed`, `confirmed`, `rejected`, and `superseded`.

### `conversation_handoffs`

Stores governed staff handoff objects linked to a conversation and episode or
segment. The backend foundation uses `conversation_handoffs` as the durable
conversation product object. The older handoff inbox remains a lower-level
availability/attention primitive and should not define the conversation product
shape.

Columns:

- `id TEXT PRIMARY KEY`
- `conversation_id TEXT NOT NULL`
- `segment_id TEXT`
- `connection_id TEXT`
- `requested_by_actor_id TEXT`
- `assigned_to_actor_id TEXT`
- `reason TEXT NOT NULL`
- `urgency TEXT NOT NULL`
- `required_capability_id TEXT`
- `allowed_context_json TEXT NOT NULL DEFAULT '{}'`
- `evidence_summary TEXT NOT NULL`
- `status TEXT NOT NULL`
- `policy_decision_id TEXT`
- `receipt_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`
- `closed_at TEXT`

Indexes:

- `(assigned_to_actor_id, status, urgency, updated_at DESC)`;
- `(conversation_id, status, updated_at DESC)`;
- `(connection_id, updated_at DESC)`.

Valid lifecycle statuses: `suggested`, `requested`, `accepted`, `declined`,
`assigned`, `in_progress`, `returned_to_agent`, and `closed`.

### `conversation_tags`

Stores evidence-backed operational tags for routing, briefing, attribution, and
relationship memory.

Columns:

- `id TEXT PRIMARY KEY`
- `conversation_id TEXT NOT NULL`
- `segment_id TEXT`
- `tag_key TEXT NOT NULL`
- `candidate_state TEXT NOT NULL`
- `confidence REAL`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `created_by_job_id TEXT`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Indexes:

- `(conversation_id, tag_key, candidate_state)`;
- `(segment_id, tag_key)`.

### `conversation_participants`

Participants link actors, connections, visitors, owners, assistants, and system
agents to conversations.

Columns:

- `id TEXT PRIMARY KEY`
- `conversation_id TEXT NOT NULL`
- `participant_kind TEXT NOT NULL`
- `actor_id TEXT`
- `connection_id TEXT`
- `visitor_session_id TEXT`
- `display_name TEXT NOT NULL`
- `role TEXT NOT NULL`
- `status TEXT NOT NULL`
- `privacy_settings_json TEXT NOT NULL DEFAULT '{}'`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `joined_at TEXT NOT NULL`
- `last_seen_at TEXT`
- `left_at TEXT`

Indexes:

- `(conversation_id, status)`;
- `(actor_id, conversation_id)`;
- `(connection_id, conversation_id)`;
- `(visitor_session_id, conversation_id)`.

### `conversation_messages`

Stores canonical messages and assistant outputs.

Columns:

- `id TEXT PRIMARY KEY`
- `conversation_id TEXT NOT NULL`
- `segment_id TEXT`
- `participant_id TEXT NOT NULL`
- `message_kind TEXT NOT NULL`
- `status TEXT NOT NULL`
- `body_markdown TEXT NOT NULL`
- `body_format TEXT NOT NULL DEFAULT 'markdown'`
- `redaction_state TEXT NOT NULL`
- `visibility TEXT NOT NULL`
- `reply_to_message_id TEXT`
- `client_message_id TEXT`
- `sequence INTEGER NOT NULL`
- `event_cursor INTEGER`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`
- `edited_at TEXT`
- `deleted_at TEXT`
- `undo_expires_at TEXT`
- `undo_cancelled_at TEXT`

Indexes:

- `(conversation_id, sequence ASC)`;
- `(conversation_id, created_at DESC)`;
- `(participant_id, created_at DESC)`;
- `(client_message_id, participant_id)` for idempotency;
- `(reply_to_message_id)`.

### `conversation_message_revisions`

Stores edit history without losing auditability.

Columns:

- `id TEXT PRIMARY KEY`
- `message_id TEXT NOT NULL`
- `revision_number INTEGER NOT NULL`
- `body_markdown TEXT NOT NULL`
- `edited_by_participant_id TEXT NOT NULL`
- `reason TEXT`
- `created_at TEXT NOT NULL`

Unique:

- `(message_id, revision_number)`.

### `conversation_message_artifacts`

Links messages to artifacts, citations, support packets, briefs, offers, or
other durable Ordo objects.

Columns:

- `id TEXT PRIMARY KEY`
- `message_id TEXT NOT NULL`
- `artifact_kind TEXT NOT NULL`
- `artifact_id TEXT NOT NULL`
- `label TEXT NOT NULL`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`

Indexes:

- `(message_id, created_at ASC)`;
- `(artifact_kind, artifact_id)`.

### `conversation_reactions`

Stores emoji and system reactions.

Columns:

- `id TEXT PRIMARY KEY`
- `message_id TEXT NOT NULL`
- `participant_id TEXT NOT NULL`
- `reaction_key TEXT NOT NULL`
- `reaction_kind TEXT NOT NULL`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`
- `removed_at TEXT`

Unique:

- active reaction uniqueness by `(message_id, participant_id, reaction_key)`.

### `conversation_receipts`

Stores sent, persisted, delivered, displayed, read, unread, and local-recorded
receipt evidence.

Columns:

- `id TEXT PRIMARY KEY`
- `conversation_id TEXT NOT NULL`
- `message_id TEXT`
- `participant_id TEXT NOT NULL`
- `receipt_kind TEXT NOT NULL`
- `event_cursor INTEGER`
- `sequence INTEGER`
- `payload_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`

Indexes:

- `(conversation_id, participant_id, created_at DESC)`;
- `(message_id, receipt_kind, created_at DESC)`;
- `(event_cursor)`.

### `conversation_read_states`

Stores one canonical read boundary per participant per conversation.

Columns:

- `conversation_id TEXT NOT NULL`
- `participant_id TEXT NOT NULL`
- `last_delivered_message_id TEXT`
- `last_delivered_at TEXT`
- `last_displayed_message_id TEXT`
- `last_displayed_at TEXT`
- `last_read_message_id TEXT`
- `last_read_event_cursor INTEGER`
- `last_read_at TEXT`
- `manual_unread_from_message_id TEXT`
- `unread_count INTEGER NOT NULL DEFAULT 0`
- `unread_mentions_count INTEGER NOT NULL DEFAULT 0`
- `unread_action_count INTEGER NOT NULL DEFAULT 0`
- `updated_at TEXT NOT NULL`

Primary key:

- `(conversation_id, participant_id)`.

### `conversation_events`

Stores per-conversation ordered event history. This is separate from the global
`realtime_events` cursor log but can reference it.

Columns:

- `id TEXT PRIMARY KEY`
- `conversation_id TEXT NOT NULL`
- `segment_id TEXT`
- `handoff_id TEXT`
- `sequence INTEGER NOT NULL`
- `event_type TEXT NOT NULL`
- `payload_json TEXT NOT NULL DEFAULT '{}'`
- `policy_decision_id TEXT`
- `realtime_cursor INTEGER`
- `occurred_at TEXT NOT NULL`

Unique:

- `(conversation_id, sequence)`.

Indexes:

- `(conversation_id, sequence ASC)`;
- `(event_type, occurred_at DESC)`;
- `(realtime_cursor)`.

### `conversation_modes`

Stores the current public-posting and leadership mode for a conversation.

Columns:

- `conversation_id TEXT PRIMARY KEY`
- `mode TEXT NOT NULL`
- `led_by_actor_id TEXT`
- `delegated_to_agent INTEGER NOT NULL DEFAULT 0`
- `delegation_scope_json TEXT NOT NULL DEFAULT '[]'`
- `idle_after TEXT`
- `private_reminder_sent_at TEXT`
- `last_public_agent_message_at TEXT`
- `updated_at TEXT NOT NULL`

Valid modes: `agent_led`, `human_led_active`, `human_led_idle`,
`assistive_private`, `needs_handoff`, and `returned_to_agent`.

### `conversation_presence_snapshots`

Stores durable presence snapshots when needed. Fast ephemeral presence lives in
memory.

Columns:

- `participant_id TEXT PRIMARY KEY`
- `conversation_id TEXT`
- `status TEXT NOT NULL`
- `visibility TEXT NOT NULL`
- `status_message TEXT`
- `device_class TEXT`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `updated_at TEXT NOT NULL`
- `expires_at TEXT`

### `conversation_analysis_runs`

Stores analysis jobs over one or more messages.

Columns:

- `id TEXT PRIMARY KEY`
- `conversation_id TEXT NOT NULL`
- `segment_id TEXT`
- `analysis_kind TEXT NOT NULL`
- `status TEXT NOT NULL`
- `input_range_json TEXT NOT NULL DEFAULT '{}'`
- `output_json TEXT NOT NULL DEFAULT '{}'`
- `provider_call_id TEXT`
- `policy_decision_id TEXT`
- `created_at TEXT NOT NULL`
- `completed_at TEXT`
- `error_message TEXT`

Indexes:

- `(conversation_id, created_at DESC)`;
- `(analysis_kind, status, created_at DESC)`.

### `knowledge_graph_candidates`

Stores graph-shaped relationship candidates from conversation analysis. These
are not truth until confirmed through a governed path.

Columns:

- `id TEXT PRIMARY KEY`
- `candidate_kind TEXT NOT NULL`
- `source_node_kind TEXT NOT NULL`
- `source_node_id TEXT NOT NULL`
- `edge_kind TEXT`
- `target_node_kind TEXT`
- `target_node_id TEXT`
- `candidate_state TEXT NOT NULL`
- `confidence REAL`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `created_by_job_id TEXT`
- `policy_decision_id TEXT`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Indexes:

- `(candidate_state, created_at DESC)`;
- `(source_node_kind, source_node_id)`;
- `(target_node_kind, target_node_id)`.

### `business_outcomes`

Stores offer, ask, referral, and relationship-to-outcome evidence. The first
implementation may defer this table until offer/ask attribution work begins,
but conversation schema should not assume offers and asks are only content.

Columns:

- `id TEXT PRIMARY KEY`
- `outcome_kind TEXT NOT NULL`
- `status TEXT NOT NULL`
- `connection_id TEXT`
- `conversation_id TEXT`
- `segment_id TEXT`
- `offer_id TEXT`
- `ask_id TEXT`
- `referral_id TEXT`
- `artifact_id TEXT`
- `entry_point_id TEXT`
- `visitor_session_id TEXT`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `occurred_at TEXT NOT NULL`
- `created_at TEXT NOT NULL`

Outcome kinds include buy-from-us, sell-to-us, referral, partnership, support,
trial-started, trial-converted, and declined/lost.

### `surface_briefs`

Stores latest completed evidence-backed briefs for major UI surfaces. Brief
refresh jobs should update this table or equivalent artifact links without
blocking the UI from loading the previous completed brief.

Columns:

- `id TEXT PRIMARY KEY`
- `surface_kind TEXT NOT NULL`
- `subject_kind TEXT`
- `subject_id TEXT`
- `artifact_id TEXT`
- `status TEXT NOT NULL`
- `brief_markdown TEXT NOT NULL`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `limitations_json TEXT NOT NULL DEFAULT '[]'`
- `created_by_job_id TEXT`
- `generated_at TEXT NOT NULL`
- `created_at TEXT NOT NULL`

Indexes:

- `(surface_kind, subject_kind, subject_id, generated_at DESC)`.

### `privacy_transform_runs`

Records each egress privacy transform before provider calls.

Columns:

- `id TEXT PRIMARY KEY`
- `conversation_id TEXT`
- `segment_id TEXT`
- `provider_call_id TEXT`
- `scope_kind TEXT NOT NULL`
- `detector_version TEXT NOT NULL`
- `transform_version TEXT NOT NULL`
- `finding_count INTEGER NOT NULL`
- `placeholder_count INTEGER NOT NULL`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`

### `privacy_placeholders`

Stores encrypted mappings for reversible reconstruction.

Columns:

- `id TEXT PRIMARY KEY`
- `transform_run_id TEXT NOT NULL`
- `placeholder TEXT NOT NULL`
- `entity_kind TEXT NOT NULL`
- `scope_kind TEXT NOT NULL`
- `scope_id TEXT NOT NULL`
- `ciphertext TEXT NOT NULL`
- `content_hash TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `expires_at TEXT`

Unique:

- `(transform_run_id, placeholder)`;
- optionally `(scope_kind, scope_id, content_hash, entity_kind)` for stable
  conversation-scope placeholders.

### `llm_invocations`

Stores provider call metadata.

Status: planned dedicated ledger table. The current Rust-owned LLM gateway
foundation uses the conversation event stream for invocation metadata until the
privacy egress firewall and token ledger phases add this table.

Columns:

- `id TEXT PRIMARY KEY`
- `conversation_id TEXT`
- `segment_id TEXT`
- `capability_id TEXT NOT NULL`
- `provider_id TEXT NOT NULL`
- `model_id TEXT NOT NULL`
- `status TEXT NOT NULL`
- `prompt_hash TEXT NOT NULL`
- `privacy_transform_run_id TEXT`
- `policy_decision_id TEXT`
- `started_at TEXT NOT NULL`
- `completed_at TEXT`
- `failure_code TEXT`
- `failure_message TEXT`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`

### `llm_prompt_slot_usage`

Stores bill-of-materials accounting for prompt construction.

Status: planned dedicated ledger table. Current prompt slot inclusion is durable
as `llm.prompt.slot.included` conversation events with source refs, inclusion
reason, visibility ceiling, and content hashes.

Columns:

- `id TEXT PRIMARY KEY`
- `invocation_id TEXT NOT NULL`
- `slot_id TEXT NOT NULL`
- `slot_version TEXT NOT NULL`
- `source_refs_json TEXT NOT NULL DEFAULT '[]'`
- `visibility TEXT NOT NULL`
- `estimated_tokens INTEGER NOT NULL DEFAULT 0`
- `actual_tokens INTEGER`
- `content_hash TEXT NOT NULL`
- `included INTEGER NOT NULL`
- `truncation_reason TEXT`
- `created_at TEXT NOT NULL`

The `ethical_business_persuasion` slot follows the same accounting rules as
other slots and must carry evidence/source refs when included. Staff-facing
outputs may expose the reasoning and evidence; client-facing outputs should
only show respectful, agency-preserving language.

### `llm_token_ledger_entries`

Stores append-only token usage and cost evidence.

Columns:

- `id TEXT PRIMARY KEY`
- `invocation_id TEXT NOT NULL`
- `conversation_id TEXT`
- `capability_id TEXT NOT NULL`
- `provider_id TEXT NOT NULL`
- `model_id TEXT NOT NULL`
- `usage_kind TEXT NOT NULL`
- `token_count INTEGER NOT NULL`
- `estimated_cost_micros INTEGER`
- `pricing_snapshot_json TEXT NOT NULL DEFAULT '{}'`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`

Indexes:

- `(conversation_id, created_at DESC)`;
- `(capability_id, created_at DESC)`;
- `(provider_id, model_id, created_at DESC)`.

## Event Persistence Pattern

For durable conversation mutations, use a single SQLite transaction:

1. Validate current state.
2. Insert or update domain table rows.
3. Increment conversation sequence.
4. Insert `conversation_events` row.
5. Insert global `realtime_events` row when the event should appear in the
   existing appliance replay log.
6. Insert receipts or read-state rollup updates.
7. Commit.
8. Broadcast the committed event.

Do not broadcast durable events before commit.

## Read Model Strategy

Initial read models can be query-backed from SQLite. Add materialized rollups
only when UI or performance evidence requires them.

Required read models:

- role-aware conversation queues for `My Handoffs`, `Team Queue`, and
  authorized `All Conversations`;
- conversation list rows with why-this-is-here, urgency, handoff status, last
  meaningful change, unread counts, presence summary, and action-needed count;
- conversation detail with messages, participants, receipts, reactions,
  artifacts, and current analysis summary;
- handoff brief with evidence before transcript;
- latest surface brief for business, conversations, connections, offers, asks,
  artifacts, jobs, affiliates, and customers;
- user notification counts by unread, mentions, action-needed, handoff waiting,
  and tool approval waiting;
- token usage breakdown by conversation, provider, capability, model, and prompt
  slot.

## Migration Order

Recommended migration stages:

1. conversations, segments, participants;
2. messages, revisions, artifacts, reactions;
3. receipts, read states, conversation events;
4. presence snapshots and ephemeral room broker state;
5. LLM invocation and prompt slot usage;
6. token ledger entries and pricing snapshots;
7. privacy transform runs and placeholders;
8. analysis runs, tags, knowledge graph candidate tables, and surface briefs;
9. offer/ask/referral/outcome attribution tables.

Each stage should include schema tests, migration tests from an older database,
and route tests for the domain behavior introduced in that stage.
