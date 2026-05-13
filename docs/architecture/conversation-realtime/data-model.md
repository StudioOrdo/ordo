# Conversation Realtime Data Model

Status: Draft schema plan with backend foundation implemented through daemon
schema versions 19 through 27

The conversation data model should extend the current SQLite appliance schema
through ordered daemon migrations. It should reuse existing actor, role,
resource grant, connection, visitor session, handoff, corpus, answer draft,
provider, and realtime event foundations.

## Current Tables To Reuse

0.1.8 Interactive Account And LLM Chat should reuse the current durable tables
before adding schema. The expected active work is mostly route/read-model,
session, bootstrap, websocket, and LLM gateway integration on top of actors,
roles, conversations, participants, messages, LLM invocation records, prompt
slot usage, privacy transforms, token ledger entries, and provider
configuration. Each implementation issue should begin with fresh diagnosis and
add schema only when the existing contract cannot represent the evidence
safely.

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
read states, and presence snapshots for the protocol layer. Tags and business
outcome tables remain planned for later product work. Schema version 21 adds
the first dedicated LLM accounting tables for invocation metadata, prompt slot
usage, and append-only token ledger entries. Schema version 22 adds the first
continuous conversation analysis, brief candidate, and memory candidate
foundation. Schema version 23 adds dedicated knowledge graph node and edge
candidate tables. Schema version 24 adds referral records, business outcomes,
and business outcome attribution candidates. Schema version 25 adds normalized
artifacts, versions, evidence/influence links, and client-facing deliverable
projections. Schema version 26 adds durable surface brief jobs/read models
linked to generated artifacts. Dedicated privacy transform tables remain deferred; privacy
transform run ids are recorded on invocations, while placeholder mappings stay
behind the encrypted local vault boundary.

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

### `conversation_analysis_jobs`

Stores bounded analysis jobs over one or more durable conversation events.

Status: implemented in schema version 22 for deterministic local analysis after
eligible visible message creation. Jobs are idempotent by conversation, analysis
kind, and source message. Provider-backed analysis can later attach `llm_run_id`
and policy evidence, but the current foundation does not call external
providers.

Columns:

- `id TEXT PRIMARY KEY`
- `conversation_id TEXT NOT NULL`
- `segment_id TEXT`
- `analysis_kind TEXT NOT NULL`
- `status TEXT NOT NULL`
- `source_message_id TEXT`
- `source_event_cursor_start INTEGER`
- `source_event_cursor_end INTEGER`
- `input_refs_json TEXT NOT NULL DEFAULT '[]'`
- `output_json TEXT NOT NULL DEFAULT '{}'`
- `policy_decision_id TEXT`
- `llm_run_id TEXT`
- `error_message_hash TEXT`
- `created_at TEXT NOT NULL`
- `started_at TEXT`
- `completed_at TEXT`
- `updated_at TEXT NOT NULL`

Indexes:

- `(conversation_id, created_at DESC)`;
- `(analysis_kind, status, created_at DESC)`.
- `(conversation_id, source_event_cursor_end)`.

### `conversation_analysis_candidates`

Stores evidence-backed operational candidates from analysis, including summary
signals, open questions, action-needed signals, and handoff signals.

Status: implemented in schema version 22. Candidates default to `proposed`.
They carry evidence refs, provenance, prompt slot ids where applicable, content
hashes, confidence, and safe summaries. They are not business truth.

Columns:

- `id TEXT PRIMARY KEY`
- `job_id TEXT NOT NULL`
- `conversation_id TEXT NOT NULL`
- `segment_id TEXT`
- `candidate_kind TEXT NOT NULL`
- `candidate_state TEXT NOT NULL`
- `confidence REAL NOT NULL`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `provenance_json TEXT NOT NULL DEFAULT '{}'`
- `prompt_slot_ids_json TEXT NOT NULL DEFAULT '[]'`
- `llm_run_id TEXT`
- `content_hash TEXT NOT NULL`
- `summary_text TEXT NOT NULL`
- `body_json TEXT NOT NULL DEFAULT '{}'`
- `visibility TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Indexes:

- `(conversation_id, candidate_kind, candidate_state, created_at DESC)`;
- `(job_id, created_at ASC)`.

### `conversation_brief_candidates`

Stores candidate narrative briefs for a conversation or segment.

Status: implemented in schema version 22. Brief candidates cite durable
conversation evidence and remain proposed until a governed product surface
confirms or supersedes them. Full surface brief jobs remain owned by #105.

Columns:

- `id TEXT PRIMARY KEY`
- `job_id TEXT NOT NULL`
- `conversation_id TEXT NOT NULL`
- `segment_id TEXT`
- `candidate_state TEXT NOT NULL`
- `title TEXT NOT NULL`
- `brief_markdown TEXT NOT NULL`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `limitations_json TEXT NOT NULL DEFAULT '[]'`
- `provenance_json TEXT NOT NULL DEFAULT '{}'`
- `content_hash TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Indexes:

- `(conversation_id, candidate_state, created_at DESC)`;
- `(job_id, created_at ASC)`.

### `conversation_memory_candidates`

Stores proposed relationship-memory candidates from conversation analysis.

Status: implemented in schema version 22. Memory candidates cite durable
conversation evidence and use `approval_status = requires_approval`; they do
not automatically promote into corpus, business facts, or durable relationship
truth.

Columns:

- `id TEXT PRIMARY KEY`
- `job_id TEXT NOT NULL`
- `conversation_id TEXT NOT NULL`
- `segment_id TEXT`
- `memory_kind TEXT NOT NULL`
- `candidate_state TEXT NOT NULL`
- `confidence REAL NOT NULL`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `provenance_json TEXT NOT NULL DEFAULT '{}'`
- `content_hash TEXT NOT NULL`
- `summary_text TEXT NOT NULL`
- `body_json TEXT NOT NULL DEFAULT '{}'`
- `visibility TEXT NOT NULL`
- `approval_status TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Indexes:

- `(conversation_id, candidate_state, created_at DESC)`;
- `(job_id, created_at ASC)`.

### `knowledge_graph_node_candidates`

Stores proposed graph node candidates from conversation analysis evidence.
These are staff/admin-facing candidate records, not business truth. A candidate
may be confirmed, rejected, or superseded later without mutating the source
message, analysis job, or conversation event evidence.

Columns:

- `id TEXT PRIMARY KEY`
- `job_id TEXT NOT NULL`
- `conversation_id TEXT NOT NULL`
- `segment_id TEXT`
- `source_analysis_candidate_id TEXT`
- `node_kind TEXT NOT NULL`
- `label TEXT NOT NULL`
- `candidate_state TEXT NOT NULL`
- `confidence REAL NOT NULL`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `provenance_json TEXT NOT NULL DEFAULT '{}'`
- `source_event_refs_json TEXT NOT NULL DEFAULT '[]'`
- `content_hash TEXT NOT NULL`
- `visibility TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`
- `state_changed_at TEXT`
- `state_reason TEXT`

Indexes:

- `(conversation_id, candidate_state, node_kind, created_at DESC)`;
- `(job_id, created_at ASC)`.

### `knowledge_graph_edge_candidates`

Stores proposed graph relationship candidates between node candidates. Edge
candidates require source/target node candidates, durable evidence refs, and
provenance. They remain candidates until governed confirmation; they do not
write to business facts, corpus memory, offer/ask attribution, or relationship
truth automatically.

Columns:

- `id TEXT PRIMARY KEY`
- `job_id TEXT NOT NULL`
- `conversation_id TEXT NOT NULL`
- `segment_id TEXT`
- `source_analysis_candidate_id TEXT`
- `source_node_candidate_id TEXT NOT NULL`
- `target_node_candidate_id TEXT NOT NULL`
- `relationship_kind TEXT NOT NULL`
- `label TEXT NOT NULL`
- `candidate_state TEXT NOT NULL`
- `confidence REAL NOT NULL`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `provenance_json TEXT NOT NULL DEFAULT '{}'`
- `source_event_refs_json TEXT NOT NULL DEFAULT '[]'`
- `content_hash TEXT NOT NULL`
- `visibility TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`
- `state_changed_at TEXT`
- `state_reason TEXT`

Indexes:

- `(conversation_id, candidate_state, relationship_kind, created_at DESC)`;
- `(job_id, created_at ASC)`.

### `business_outcomes`

Stores offer, ask, referral, and relationship-to-outcome evidence.

Status: implemented in schema version 24. Outcomes require evidence refs and
provenance. Public offer acceptance records an evidence-backed
`offer_acceptance` outcome when the offer/trial acceptance is persisted.
Payments, payout automation, and external analytics integrations remain out of
scope.

Columns:

- `id TEXT PRIMARY KEY`
- `outcome_kind TEXT NOT NULL`
- `status TEXT NOT NULL`
- `connection_id TEXT`
- `conversation_id TEXT`
- `segment_id TEXT`
- `offer_id TEXT`
- `ask_id TEXT`
- `artifact_id TEXT`
- `entry_point_id TEXT`
- `visitor_session_id TEXT`
- `referral_id TEXT`
- `value_micros INTEGER`
- `currency TEXT`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `provenance_json TEXT NOT NULL DEFAULT '{}'`
- `occurred_at TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Outcome kinds include `offer_acceptance`, `ask_response`, `referral`,
`qualified_lead`, `conversion`, `retained_customer`, `declined`, `voided`, and
`expired`. The current implementation records offer acceptance outcomes and
supports the broader shape for asks, referrals, artifacts, entry points,
visitor sessions, and conversations without inventing missing evidence.

Indexes:

- `(outcome_kind, status, occurred_at DESC)`;
- `(conversation_id, occurred_at DESC)`;
- `(connection_id, occurred_at DESC)`;
- `(offer_id, occurred_at DESC)`;
- `(entry_point_id, occurred_at DESC)`.

### `business_outcome_attributions`

Stores proposed attribution candidates for an outcome. Attribution is
evidence-backed influence, not automatic credit assignment.

Status: implemented in schema version 24. Attributions default to `proposed`
and support confirmed, rejected, and superseded lifecycle transitions. Public
offer acceptance proposes direct offer influence and, when evidence exists,
visitor-session and entry-point influence.

Columns:

- `id TEXT PRIMARY KEY`
- `outcome_id TEXT NOT NULL`
- `attribution_kind TEXT NOT NULL`
- `source_id TEXT NOT NULL`
- `influence_role TEXT NOT NULL`
- `candidate_state TEXT NOT NULL`
- `confidence REAL NOT NULL`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `provenance_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`
- `state_changed_at TEXT`
- `state_reason TEXT`

Attribution kinds include `conversation`, `message`, `artifact`,
`entry_point`, `visitor_session`, `offer`, `ask`, `referral`, and `campaign`.
Influence roles are `first_touch`, `assisted`, `direct`, `confirming`, and
`excluded`.

Indexes:

- `(outcome_id, candidate_state, created_at ASC)`;
- `(attribution_kind, source_id, candidate_state, created_at DESC)`.

### `referral_records`

Stores durable referral evidence before or alongside outcomes.

Status: implemented in schema version 24. Referral records require evidence
refs and provenance, and may link to referrer/referred connections,
conversation, entry point, and visitor session evidence. External affiliate
payouts remain out of scope.

Columns:

- `id TEXT PRIMARY KEY`
- `status TEXT NOT NULL`
- `referrer_connection_id TEXT`
- `referred_connection_id TEXT`
- `conversation_id TEXT`
- `entry_point_id TEXT`
- `visitor_session_id TEXT`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `provenance_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`
- `closed_at TEXT`

Referral statuses include `captured`, `qualified`, `converted`, `lost`, and
`voided`.

### `artifacts`

Stores `Artifact` as the canonical internal system noun for durable
knowledge/business objects. Existing operational tables such as
`job_artifacts`, `brief_artifacts`, issue report artifacts, backup artifacts,
and support packets remain valid producers or specialized records; normalized
`artifacts` rows provide the product contract for conversation cards, evidence
links, attribution influence, and future surface briefs.

Columns:

- `id TEXT PRIMARY KEY`
- `artifact_kind TEXT NOT NULL`
- `title TEXT NOT NULL`
- `status TEXT NOT NULL`
- `visibility_ceiling TEXT NOT NULL`
- `summary TEXT NOT NULL`
- `source_kind TEXT`
- `source_id TEXT`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `provenance_json TEXT NOT NULL DEFAULT '{}'`
- `content_hash TEXT NOT NULL`
- `storage_uri TEXT`
- `health_status TEXT`
- `created_by_job_id TEXT`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Indexes:

- `(artifact_kind, status, updated_at DESC)`;
- `(source_kind, source_id)`;
- `(visibility_ceiling, updated_at DESC)`.

### `artifact_versions`

Tracks durable artifact revisions by content hash and storage pointer without
requiring a storage backend or editor UI in this slice.

Columns:

- `id TEXT PRIMARY KEY`
- `artifact_id TEXT NOT NULL`
- `version INTEGER NOT NULL`
- `content_hash TEXT NOT NULL`
- `storage_uri TEXT`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`

Unique:

- `(artifact_id, version)`.

### `artifact_links`

Links artifacts to concrete evidence and business objects. Links require real
source ids so artifact influence cannot be invented.

Columns:

- `id TEXT PRIMARY KEY`
- `artifact_id TEXT NOT NULL`
- `link_kind TEXT NOT NULL`
- `source_kind TEXT NOT NULL`
- `source_id TEXT NOT NULL`
- `relation TEXT NOT NULL`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `provenance_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`

Unique:

- `(artifact_id, link_kind, source_kind, source_id, relation)`.

### `artifact_deliverables`

Stores optional client-facing `Deliverable` projections from internal artifacts.
Deliverables expose client-safe labels and summaries without leaking internal
provenance, storage, job, or policy mechanics.

Columns:

- `id TEXT PRIMARY KEY`
- `artifact_id TEXT NOT NULL`
- `client_label TEXT NOT NULL`
- `status TEXT NOT NULL`
- `visibility TEXT NOT NULL`
- `summary TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`
- `published_at TEXT`

### `surface_briefs`

Stores latest completed evidence-backed briefs for major UI surfaces. Brief
refresh jobs update this table and linked `surface.brief` artifacts without
blocking the UI from loading the previous completed brief. The #105 foundation
implements deterministic generation first; provider-backed synthesis remains
behind the governed LLM path.

Columns:

- `id TEXT PRIMARY KEY`
- `surface_kind TEXT NOT NULL`
- `subject_kind TEXT`
- `subject_id TEXT`
- `status TEXT NOT NULL`
- `artifact_id TEXT`
- `title TEXT NOT NULL`
- `brief_markdown TEXT NOT NULL`
- `evidence_refs_json TEXT NOT NULL DEFAULT '[]'`
- `limitations_json TEXT NOT NULL DEFAULT '[]'`
- `created_by_job_id TEXT`
- `generated_at TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`
- `completed_at TEXT`
- `superseded_at TEXT`
- `failure_message TEXT`

Indexes:

- `(surface_kind, subject_kind, subject_id, status, generated_at DESC)`;
- `(artifact_id)`;
- `(created_by_job_id)`.

### `privacy_transform_runs`

Records each egress privacy transform before provider calls.

Status: event-sourced foundation implemented in the #93 slice. The current
implementation records transform run metadata as durable
`privacy.egress.transformed` conversation events and stores placeholder mappings
as encrypted `vault_items` with metadata for transform id, placeholder, detector
kind, scope, and content hash. Dedicated `privacy_transform_runs` and
`privacy_placeholders` tables remain planned only when query-optimized privacy
inspection needs require them. Schema version 21 records privacy transform run
ids on `llm_invocations` without duplicating raw placeholder values.

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

Status: encrypted vault-backed foundation implemented. Raw sensitive values are
stored only as encrypted vault item values; event, realtime, policy, and
metadata payloads carry hashes and placeholders rather than raw spans.

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

Status: implemented in schema version 21 for token ledger accounting. Every
allowed LLM invocation creates an invocation row before provider work proceeds.
Rows carry prompt hashes, capability/provider/model ids, policy decision ids,
privacy transform run ids, terminal status, and safe failure metadata.

Current tool-governance state remains event-sourced: `llm.tool.requested`,
`llm.tool.approved`, `llm.tool.rejected`, `llm.tool.executing`,
`llm.tool.completed`, and `llm.tool.failed` store the tool request id, run id,
conversation id, requested capability id, actor evidence, evidence refs, redacted
input summary, visibility ceiling, status, policy decision id, and timestamps in
`conversation_events`. A dedicated tool table remains deferred until tool query
volume requires it.

Columns:

- `id TEXT PRIMARY KEY`
- `conversation_id TEXT NOT NULL`
- `segment_id TEXT`
- `capability_id TEXT NOT NULL`
- `provider_id TEXT NOT NULL`
- `model_id TEXT NOT NULL`
- `status TEXT NOT NULL`
- `prompt_hash TEXT NOT NULL`
- `privacy_transform_run_ids_json TEXT NOT NULL DEFAULT '[]'`
- `policy_decision_id TEXT`
- `started_at TEXT NOT NULL`
- `completed_at TEXT`
- `failure_code TEXT`
- `failure_message_hash TEXT`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`

Indexes:

- `(conversation_id, started_at DESC)`;
- `(provider_id, model_id, started_at DESC)`;
- `(capability_id, started_at DESC)`;
- `(status, started_at DESC)`.

### `llm_prompt_slot_usage`

Stores bill-of-materials accounting for prompt construction.

Status: implemented in schema version 21. Current prompt slot inclusion remains
durable as `llm.prompt.slot.included` conversation events, and each included
slot also receives a `llm_prompt_slot_usage` row plus a
`llm.prompt.slot.accounted` event. Rows store source refs, visibility, stable
token estimates, content hashes, inclusion state, optional provider-allocation
actuals, and truncation reason without storing raw slot text.

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

Status: implemented for v1 without new tables. The ethical persuasion builder
creates a normal prompt slot with `slot_id = ethical_business_persuasion`,
`slot_version = v1`, evidence-backed source refs, visibility ceiling, inclusion
reason, content hash, and deterministic token estimate. It reuses
`llm_prompt_slot_usage` and `llm.prompt.slot.accounted` rather than storing raw
persuasion text or adding a dedicated prompt table.

### `llm_token_ledger_entries`

Stores append-only token usage and cost evidence.

Status: implemented in schema version 21. Provider-reported usage creates
append-only ledger entries for input and output tokens. Cost fields are
conservative estimates; the current deterministic/local provider path records
`estimated_cost_micros = 0` with `costKind: not_priced` metadata rather than
pretending to know external billing.

Columns:

- `id TEXT PRIMARY KEY`
- `invocation_id TEXT NOT NULL`
- `conversation_id TEXT NOT NULL`
- `capability_id TEXT NOT NULL`
- `provider_id TEXT NOT NULL`
- `model_id TEXT NOT NULL`
- `usage_kind TEXT NOT NULL`
- `token_count INTEGER NOT NULL`
- `estimated_cost_micros INTEGER NOT NULL DEFAULT 0`
- `pricing_snapshot_json TEXT NOT NULL DEFAULT '{}'`
- `metadata_json TEXT NOT NULL DEFAULT '{}'`
- `created_at TEXT NOT NULL`

Indexes:

- `(conversation_id, created_at DESC)`;
- `(capability_id, created_at DESC)`;
- `(provider_id, model_id, created_at DESC)`.
- `(usage_kind, created_at DESC)`.

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
- Customer Feedback brief/read models with feedback rows, tags, review
  candidate state, linked source objects, and recommended actions;
- Home/About billboard read models with source object links, evidence refs,
  owner-governed state, and reduced-motion-safe presentation metadata;
- Offer/Ask intent read models that preserve both human-readable presentation
  and future machine-readable matching/referral/proposal metadata.

## Customer Feedback And Review Tables

Implemented in schema version 27:

- `customer_feedback`: private business intelligence captured from durable
  conversation/message evidence. Rows include connection, conversation, segment
  and message refs, kind, status, `private_business_intelligence` visibility,
  star flag, source refs, evidence refs, and provenance.
- `feedback_tags`: feedback tags with `proposed`, `confirmed`, `rejected`, or
  `superseded` candidate state, confidence, evidence refs, and provenance.
- `customer_reviews`: review candidates and public-review lifecycle rows with
  status, review body, publication visibility, consent evidence refs, approval
  evidence refs, evidence refs, provenance, and published/featured/retired
  timestamps.

The first implementation keeps object links through source/evidence refs rather
than adding a dedicated `feedback_object_links` table. A review cannot become
public proof without consent and approval evidence. Staff stars remain internal
business-intelligence signals and are not customer ratings.

Planned product intelligence tables still deferred:

- `home_billboards`: Home/About narrative brief sections with linked object,
  evidence refs, persuasion role, brand archetype role, state, and publication
  metadata.
- `brand_profile`: owner-governed brand archetype and public narrative guidance,
  never a source of unsupported claims.
- `offer_intents` and `ask_intents` or equivalent fields on existing offer/ask
  tables when machine-readable business intent needs structure.

Candidate records should cite durable evidence and provenance, default to
`proposed` where generated, and avoid raw private fixture values in public or
client-facing projections.

The first 0.1.4 product-surface eval slice does not add new tables for
Home/About billboards or Offer/Ask intents. It validates a smaller current
contract built from existing durable rows:

- public `business_facts` with keys such as `about.billboards.<id>.*`,
  `offers.<id>.*`, and `asks.<id>.*`;
- `surface_briefs` and linked `artifacts` where a generated narrative brief
  already exists;
- `customer_reviews`, `business_outcomes`, and offer/referral rows only when
  concrete ids exist.

Dedicated billboard and intent tables should be added only when future evals
show that the fact-backed contract cannot express the needed lifecycle,
governance, or query shape cleanly.

The 0.1.4 simulator contract slice does not add SQLite tables. Customer,
operator, and reviewer simulator outputs are local eval artifacts using
`ordo.eval_simulator_output.v1`. They cite durable rows, packet artifact refs,
and deterministic assertion refs, but they are not promoted into product truth
or runtime state.

The 0.1.5 live product journey eval arc should begin by reusing existing
durable tables rather than adding broad new schema:

- `tracked_entry_points` and `visitor_sessions` for event QR scans and return
  links;
- `conversations`, `conversation_participants`, `conversation_messages`, and
  `conversation_events` for relationship continuity;
- `llm_invocations`, `llm_prompt_slot_usage`, and
  `llm_token_ledger_entries` for guarded live LLM evidence;
- `offers`, `offer_acceptances`, and `trials` for the 30-day OrdoStudio trial
  path;
- `business_outcomes`, `business_outcome_attributions`, and
  `referral_records` for conversion/referral evidence;
- `customer_feedback`, `feedback_tags`, and `customer_reviews` for the review
  return path;
- `artifacts` and eval packet/report files for analyzed journey evidence.
  Review-request email remains represented by a
  `simulated_review_request_email` artifact with `simulated_not_delivered`
  status in 0.1.5. It should cite return-link, trial, conversation, and review
  evidence refs, and it must not require or store a raw recipient address.

Dedicated persona, email, journey-run, or aggregate-report tables should be
added only if the first implementation slices prove that file artifacts and the
existing durable rows cannot express the required evidence cleanly.
Dedicated outbound email tables should not be added until a later accepted
issue defines owner approval, consent/lawful-basis, suppression/unsubscribe,
deliverability, provider-secret, audit, rate/spend, redaction, and opt-in email
guard contracts.

## Migration Order

Recommended migration stages:

1. conversations, segments, participants;
2. messages, revisions, artifacts, reactions;
3. receipts, read states, conversation events;
4. presence snapshots and ephemeral room broker state;
5. LLM invocation, prompt slot usage, and token ledger entries;
6. pricing snapshots when provider pricing evidence exists;
7. privacy transform runs and placeholders when vault-backed events are not
   enough for inspection;
8. analysis jobs, analysis candidates, brief candidates, and memory candidates;
9. knowledge graph node and edge candidates;
10. offer/ask/referral/outcome attribution tables;
11. normalized artifacts, artifact links, and deliverable projections.
12. customer feedback, feedback tags, and review consent/publication.
13. home billboards and brand profile tables after eval evidence proves the
    smallest useful shape.

Each stage should include schema tests, migration tests from an older database,
and route tests for the domain behavior introduced in that stage.
