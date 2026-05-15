# Graph Kernel

Status: target architecture, partially founded by current schema

Ordo is graph-native because the product is about relationships: people,
conversations, offers, asks, artifacts, jobs, events, rewards, benefits,
handoffs, packs, claims, requests, and outcomes. Vectors may help retrieve fuzzy
language later, but the graph explains how business facts connect and why a
surface, recommendation, reward, or handoff is justified.

## Current Foundations

Current code already has graph-shaped foundations:

- job/task DAGs in `process_templates`, `jobs`, `job_tasks`, and
  `job_task_dependencies`;
- event ledgers such as `realtime_events`, `job_events`,
  `conversation_events`, `connection_events`, `handoff_events`,
  `trial_events`, and `reward_events`;
- graph candidates in `knowledge_graph_node_candidates` and
  `knowledge_graph_edge_candidates`;
- artifact lineage through `artifacts`, `artifact_versions`,
  `artifact_links`, and `artifact_deliverables`;
- product relationships across offers, trials, tracked entry points, visitor
  sessions, connections, grants, feedback, reviews, rewards, benefits, and
  outcomes.

These foundations are not yet one coherent graph kernel.

## Ownership Model

```text
Canonical tables own truth.
Events own audit/replay.
Graph tables own relationship traversal and explanation.
Projections/read models own surface experience.
Vectors assist retrieval later, but never own truth.
```

Graph records should point back to canonical records. They should not become a
second source of truth for offers, access grants, jobs, rewards, artifacts, or
public claims.

## SQLite-Backed Graph Model

The first graph kernel should remain SQLite-backed. A separate graph database is
not required for the current appliance.

Recommended tables:

```text
graph_nodes
  id
  node_kind
  resource_kind
  resource_id
  label
  status
  visibility_ceiling
  content_hash
  provenance_json
  created_at
  updated_at

graph_node_aliases
  id
  node_id
  alias_kind
  alias_value
  confidence
  evidence_refs_json
  created_at

graph_edges
  id
  source_node_id
  target_node_id
  relationship_kind
  status
  confidence
  visibility_ceiling
  evidence_refs_json
  provenance_json
  created_at
  updated_at

graph_edge_evidence
  id
  edge_id
  evidence_kind
  evidence_ref
  summary
  created_at

graph_query_audit
  id
  method_name
  viewer_context_json
  input_hash
  output_hash
  policy_decision_id
  created_at

graph_candidate_promotions
  id
  candidate_kind
  candidate_id
  graph_node_id
  graph_edge_id
  decision
  reason
  actor_id
  created_at
```

Indexes should support:

- lookup by `(resource_kind, resource_id)`;
- neighborhood traversal by source and target node;
- filtering by relationship kind;
- filtering by visibility ceiling and status;
- finding evidence for a node or edge;
- bounded recursive traversal with SQLite CTEs.

## Candidate Versus Confirmed State

Current `knowledge_graph_*_candidates` tables are proposal space. They are not
product truth.

Candidate states:

```text
proposed -> confirmed | rejected | superseded
```

Richer Knowledge Pack review flows should use a longer candidate lifecycle:

```text
proposed
-> in_review
-> consensus_met | disputed | needs_more_evidence | rejected
-> promoted | held | superseded
```

Confirmed candidates may create or update graph nodes and edges only through a
promotion path that records:

- source candidate id;
- evidence refs;
- policy decision;
- actor or job responsible;
- event emitted;
- resulting graph node or edge id.

LLM output should normally create candidates, not confirmed graph records.

Generated content follows the same rule. Generated artifacts are evidence, not
truth. A draft homepage, article, image brief, TTS script, video storyboard, or
review note may contain useful claims, but those claims enter memory as
candidate graph facts until confirmed by deterministic evidence, owner
approval, publication, customer feedback, or outcome evidence.

Generated-content memory rules:

- generated artifacts are evidence, not truth;
- draft claims become candidate graph facts;
- approved or published claims may become stronger evidence;
- customer feedback and outcomes can confirm, weaken, or correct claims;
- rejected content can become negative or preference memory when useful;
- raw prompt internals, provider payloads, private artifact text, and private
  task outputs must not become public graph labels or member-visible memory.

## Memory Tiers

Use explicit memory tiers instead of one undifferentiated "memory" bucket:

- canonical memory: user/business-entered facts and approved operational
  records owned by canonical tables;
- graph memory: confirmed relationships and evidence paths between canonical
  records, artifacts, claims, people, jobs, packs, and outcomes;
- candidate memory: proposed claims or relationships extracted from generated
  content, conversations, reviews, or imports;
- published memory: claims and artifacts approved for public/member surfaces;
- preference memory: style, tone, audience, recurring choices, and approved
  patterns;
- negative memory: rejected claims, disliked directions, banned language, and
  failed approaches.

Promotion between tiers must record evidence refs, actor or job origin, policy
decision when available, and an event.

## HITL Consensus And Entity Resolution

Knowledge graph curation should use human-in-the-loop review when machine
evidence cannot safely promote a candidate. Ordo should be able to create
requests for:

- entity conflict resolution;
- alias confirmation;
- source-span review;
- claim approval or rejection;
- rights verification;
- graph promotion approval;
- more-evidence requests.

Consensus should be configurable by pack and use case. A private exploratory
pack may accept lower confidence. A paid public education pack, legal/policy
pack, medical pack, or other high-stakes pack should require stronger source
quality, reviewer agreement, and explicit approval.

Consensus inputs may include reviewer count, reviewer role or reputation,
source quality, evidence strength, disagreement history, recency, rights
status, visibility class, external authority links, and correction history.
The consensus score is not public truth by itself; it is evidence used by the
promotion policy.

## Node Kinds

Initial node kinds should mirror Ordo's product spine:

- `actor`
- `connection`
- `conversation`
- `conversation_message`
- `visitor_session`
- `tracked_entry_point`
- `offer`
- `offer_acceptance`
- `trial`
- `request`
- `handoff`
- `artifact`
- `job`
- `job_task`
- `event`
- `claim`
- `homepage_section`
- `story_profile`
- `reward_program`
- `reward_event`
- `benefit_grant`
- `business_outcome`
- `pack`
- `capability`
- `corpus_item`

## Relationship Kinds

Initial relationship kinds:

- `AUTHORED`
- `MENTIONS`
- `SUPPORTS`
- `CONTRADICTS`
- `DERIVED_FROM`
- `PRODUCED`
- `USES`
- `REQUESTED`
- `APPROVED`
- `REJECTED`
- `GRANTED`
- `REVOKED`
- `REFERRED`
- `ATTRIBUTED_TO`
- `ACCEPTED`
- `TRIGGERED`
- `HANDED_OFF_TO`
- `REQUIRES`
- `DEPENDS_ON`
- `INSTALLED`
- `EMITTED`
- `APPEARS_IN`
- `CONTAINS_CLAIM`
- `PUBLISHED_TO`
- `INFLUENCED`
- `REVISED_BY_FEEDBACK`
- `PRODUCED_FROM_INPUT`
- `ALIAS_OF`
- `REFERS_TO`
- `TAUGHT_AT`
- `STUDIED_AT`
- `MEMBER_OF`
- `CREATED_BY`
- `EXHIBITED_AT`
- `OCCURRED_AT`
- `PART_OF_COLLECTION`
- `INFLUENCED_BY`
- `PUBLISHED_BY`

Each relationship kind must define:

- allowed source node kinds;
- allowed target node kinds;
- evidence requirements;
- visibility ceiling rules;
- whether LLMs may propose it;
- whether packs may create it.

Knowledge Packs should avoid generic `RELATED_TO` graphs. Wikipedia-style links,
Wikidata statements, archive metadata, OCR spans, and LLM extraction may seed
candidate edges, but Ordo should map those links to canonical edge verbs that
explain why one thing connects to another. The edge verb is part of the
knowledge product and must carry evidence requirements.

## Graph Import And Export

Future Ordo should import and export Knowledge Packs as graph packages, not
loose graph dumps. A package should include graph nodes and edges, source
manifests, artifact manifests, source spans, claims, rights metadata, aliases,
review decisions, visibility rules, version/hash metadata, and compatibility
information.

Import rules:

- validate schema, hashes, and signatures when available;
- inspect rights and visibility before use;
- map node and edge kinds to the local canonical vocabulary;
- detect entity, alias, and claim conflicts;
- create candidates and review requests by default;
- promote only when source trust, review policy, and evidence requirements are
  satisfied;
- preserve import provenance.

Export rules:

- verify actor permission and pack scope;
- redact private fields and private artifact text;
- include provenance, citations, review history, rights, and limitations;
- include package hashes or signatures when available;
- never export staff routing, prompt internals, provider internals, raw policy
  internals, secrets, owner-only data, task private payloads, graph certainty
  claims, or unsupported public claims.

## Access-Aware Traversal

Graph traversal must be policy filtered. A viewer should see only nodes and
edges they are allowed to see.

Access inputs:

- viewer role;
- actor id;
- connection id when relevant;
- resource grants;
- visibility ceiling;
- surface context;
- policy decision.

Traversal rules:

- fail closed on unknown node or edge kinds;
- stop at private nodes when viewer lacks access;
- redact labels and summaries when edge existence may be visible but details
  are not;
- never return raw prompt internals, provider internals, owner-only evidence, or
  private artifact text to public/member callers;
- include limitations in every graph method output.

## LLM-Safe Graph Methods

LLMs should not receive arbitrary SQL or generic database access. They should
call explicit graph methods:

```text
graph.get_resource_neighborhood
graph.find_evidence_path
graph.explain_relationship
graph.list_open_loops
graph.get_customer_context
graph.get_artifact_lineage
graph.get_claim_support
graph.get_pack_impact
graph.propose_candidate_edges
```

Every method returns:

- status;
- scoped nodes;
- scoped edges;
- evidence refs;
- omitted/redacted count when useful;
- limitations;
- policy decision id when available.

## Relationship To Job DAGs

The job DAG is execution structure. The graph is business memory.

Do not collapse them.

Job DAG:

- defines tasks and dependencies;
- controls execution state;
- supports leases, retries, pause/resume, skip, and result envelopes.

Graph:

- explains what the job used, produced, affected, and proved;
- links jobs to artifacts, claims, events, requests, packs, and outcomes;
- supports owner/member/staff explanation.

A job can produce graph edges such as:

```text
job PRODUCED artifact
artifact SUPPORTS claim
artifact CONTAINS_CLAIM claim
claim DERIVED_FROM artifact
actor APPROVED claim
actor REJECTED claim
homepage_version PUBLISHED_TO narrative_surface
content_event INFLUENCED business_outcome
feedback REVISED_BY_FEEDBACK claim
workflow_input PRODUCED_FROM_INPUT artifact
job EMITTED event
pack INSTALLED capability
offer ACCEPTED trial
referral ATTRIBUTED_TO reward_event
```

## Pack-Created Graph Records

Packs may contribute graph records only through declared capabilities.

Pack manifest should declare:

- node kinds it can create or propose;
- edge kinds it can create or propose;
- evidence requirements;
- visibility ceilings;
- policy hooks;
- uninstall behavior.

If a pack is disabled, graph records it created should remain auditable but may
be hidden from active projections depending on status and policy.

## Vectors

Future vectors should index text and artifacts for retrieval, not replace graph
truth. Vector hits must resolve to canonical records, corpus items, graph nodes,
or artifacts before they can become evidence.

Vector output should be treated as:

```text
candidate retrieval -> policy filter -> evidence refs -> graph/canonical lookup
```

## Validation Expectations

Graph work must test:

- migration creation and repeatability;
- node and edge uniqueness;
- candidate promotion and rejection;
- evidence requirements;
- access-aware traversal;
- recursive traversal depth limits;
- cycle handling;
- public/member redaction;
- deterministic LLM-safe method output;
- event emission for graph mutations;
- projection consistency when graph records affect surfaces.

Default tests must not require live LLMs, live vectors, or network access.
