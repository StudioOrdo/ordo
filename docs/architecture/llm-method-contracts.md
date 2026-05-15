# LLM Method Contracts

Status: target architecture

Ordo should build for useful but unreliable LLMs. Models are good at language,
classification, summarization, and explanation. They are bad at preserving
authority boundaries, picking safe tables, remembering product invariants, or
knowing when missing evidence means "do not answer."

The interface should make the correct action obvious.

## Core Rule

```text
Give the LLM product-shaped methods, not database-shaped power.
```

No arbitrary SQL. No generic `get_context`. No hidden authority. No public or
member method should expose internal routing, prompt internals, provider
internals, raw policy internals, owner-only data, private artifact text, or
unsupported claims.

The same rule applies to tools. A generic provider capability can exist for
image generation, image review, TTS, transcription, search, QR generation,
rendering, or screenshot QA, but LLMs should reach it through product-shaped
methods with policy, artifact, visibility, audit, and evidence contracts.

```text
Generic capability: image.generateVariants
Product method: homepage.generateSectionHeroImage
Workflow use: Story Pack homepage refresh task
```

Generic capability should mean reusable machinery. It must not mean generic
authority.

## LLM Role Boundary

The LLM is a bounded participant in a deterministic operating system. It should
not be treated as the system of record or the scheduler.

LLMs may:

- parse user intent;
- classify requests;
- choose among approved methods or workflow templates when policy allows;
- draft text or briefs;
- summarize evidence;
- explain state;
- review artifacts against a brief;
- interpret ambiguity;
- propose claims, entities, aliases, edges, or next actions.

Deterministic Ordo code owns:

- actor identity and Access;
- policy decisions;
- typed variable resolution;
- workflow and DAG compilation;
- capability dispatch;
- provider gateway selection and approval;
- task leases, retries, pause, resume, skip, cancel, and completion;
- artifacts, versions, visibility, and publication state;
- events, audit, projections, and analytics truth;
- requests and approval gates;
- reward qualification and benefit grants;
- graph confirmation and memory promotion.

```text
The model can say "this sounds like a Story Pack homepage refresh."
Ordo decides whether that workflow is allowed, what variables resolve, what
tasks exist, what can run now, and what must wait for approval.
```

## Method Shape

Each method contract must define:

```text
name
purpose
authority
viewer context
input schema
output schema
visibility ceiling
policy checks
evidence refs
limitations
events emitted, if any
artifact behavior, if any
graph behavior, if any
deterministic test fixtures
```

Method outputs must include:

```json
{
  "status": "evidence_found | missing_evidence | denied | needs_approval | error",
  "summary": "...",
  "evidenceRefs": [],
  "limitations": [],
  "visibilityClass": "public | authenticated | staff | owner",
  "memoryEffect": "none | candidate | confirmed_requires_approval",
  "policyDecisionId": null
}
```

If evidence is missing, the method should say so explicitly. It should not ask
the LLM to infer missing truth.

## Naming Convention

Use product-shaped namespaces:

- `graph.*`
- `claim.*`
- `studio.*`
- `growth.*`
- `support.*`
- `access.*`
- `artifact.*`
- `job.*`
- `homepage.*`
- `pack.*`
- `workflow.*`
- `tool.*`
- `image.*`
- `content.*`
- `memory.*`
- `analytics.*`
- `system.*`

Method names should describe the business question:

Good:

```text
graph.get_customer_context
claim.validate_public_claim
studio.get_artifact_lineage
growth.explain_reward_eligibility
support.prepare_handoff_brief
homepage.propose_story_refresh
```

Avoid:

```text
query_sql
search_database
get_context
analyze_business
run_tool
update_record
```

## Read-Only Versus Mutation

Read-only methods:

- return scoped evidence;
- may append query audit records;
- must not mutate canonical truth;
- may return candidates for the LLM to explain.

Mutation methods:

- require explicit capability and policy checks;
- append events;
- record actor/job origin;
- validate idempotency;
- return artifact, graph, projection, or canonical mutation evidence;
- require approval when touching publishing, access grants, rewards, provider
  egress, external sending, deletion, or irreversible actions.

## Example Method Families

### `graph.*`

- `graph.get_resource_neighborhood`
- `graph.find_evidence_path`
- `graph.explain_relationship`
- `graph.list_open_loops`
- `graph.get_customer_context`
- `graph.get_artifact_lineage`
- `graph.get_pack_impact`

Use for relationship traversal and explanation. Must be access-aware.

### `claim.*`

- `claim.validate_public_claim`
- `claim.get_public_claim_support`
- `claim.list_unsupported_claims`
- `claim.propose_safer_claim`

Use before public/member copy, scrollytelling sections, offers, testimonials, or
growth pages. Must fail closed when proof is missing.

### `studio.*`

- `studio.get_artifact_lineage`
- `studio.prepare_artifact_review_packet`
- `studio.list_production_runs`
- `studio.propose_media_package`

Use for production runs and artifacts. Must not publish by default.

### `growth.*`

- `growth.get_trial_conversion_context`
- `growth.explain_reward_eligibility`
- `growth.list_affiliate_attribution_evidence`
- `growth.prepare_feedback_request`

Use for offers, trials, referrals, feedback, rewards, and attribution. Must not
fake metrics, rewards, scarcity, or conversion evidence.

### `support.*`

- `support.prepare_handoff_brief`
- `support.get_attention_queue_context`
- `support.prepare_security_response_packet`
- `support.prepare_a2a_support_packet`

Use for handoff, support, QA, and security response. Must redact private data
before egress.

### `access.*`

- `access.check_offer_benefit`
- `access.explain_visibility_decision`
- `access.list_actor_grants`
- `access.prepare_grant_review`

Use for benefit and visibility decisions. Must not grant access without policy
and event evidence.

### `artifact.*`

- `artifact.get_lineage`
- `artifact.prepare_patch_proposal`
- `artifact.validate_visibility`
- `artifact.list_public_safe_derivatives`
- `artifact.preparePublicDerivative`

Use for artifact provenance, patches, and derivative content.

### `job.*`

- `job.get_dag_status`
- `job.explain_task_blocker`
- `job.prepare_retry`
- `job.list_open_runs`

Use for deterministic job/DAG state. Must distinguish execution state from
business truth.

### `homepage.*`

- `homepage.get_story_profile`
- `homepage.validate_section_claims`
- `homepage.propose_story_refresh`
- `homepage.prepare_image_briefs`
- `homepage.generateSectionHeroImage`
- `homepage.prepare_video_storyboard`

Use for scrollytelling and story artifacts. AI may add color; Ordo owns
structure and publish decisions.

### `pack.*`

- `pack.inspect_manifest`
- `pack.validate_permissions`
- `pack.get_installed_capabilities`
- `pack.prepare_review_packet`
- `pack.explain_uninstall_impact`

Use for internal and future external packs. Must respect core trust boundaries.

### `workflow.*`

- `workflow.inspectTemplate`
- `workflow.resolveVariables`
- `workflow.expandFanout`
- `workflow.explainBlockedTask`
- `workflow.prepareApprovalGate`

Use for typed workflow template inspection and explanation. Must not expose
private variables, compiled-plan private inputs, provider payloads, or task
private outputs to public/member contexts.

### `tool.*`

- `tool.listApprovedCapabilities`
- `tool.explainCapabilityRequirement`
- `tool.prepareProviderTask`
- `tool.validateDeterministicFixture`

Use for approved capability discovery and provider-gateway preparation. Must
not become `run_tool` or arbitrary provider access.

### `image.*`

- `image.generateVariants`
- `image.reviewAgainstBrief`
- `image.extractPalette`
- `image.createAltText`
- `image.preparePublicDerivative`

Use for reusable image work. Public outputs must hide raw prompts, provider
internals, private brief text, and private review notes unless a public-safe
derivative is approved.

### `content.*`

- `content.extractClaims`
- `content.compareVersionPerformance`
- `content.preparePublicStoryDraft`
- `content.listPublishedClaims`

Use for content artifacts, claims, and public-safe drafts. Generated content is
evidence; it does not automatically become truth.

### `memory.*`

- `memory.proposeCandidateClaims`
- `memory.listCandidateClaimsForReview`
- `memory.explainWhyClaimIsRemembered`
- `memory.recordRejectedDirection`

Use for candidate memory and preference/negative memory. Mutation methods must
distinguish candidate, approved, published, and canonical memory effects.

### `analytics.*`

- `analytics.recordContentEvent`
- `analytics.explainContentPerformance`
- `analytics.linkOutcomeToContent`
- `analytics.prepareLearningBrief`

Use for event-first content analytics and outcome explanation. Must not fake
traffic, conversion, attribution, trial scarcity, rewards, or performance
claims.

## Prompt Slot Use

LLM methods may include prompt slot references, but public/member outputs must
not expose raw prompt internals. Staff views may expose reasoning only when the
architecture doc for that feature allows it and evidence refs are present.

## Testing Requirements

Every LLM method family should have deterministic tests for:

- input validation;
- policy denial;
- missing evidence;
- positive evidence response;
- public/member redaction;
- limitations included;
- no live provider required;
- stable output shape.

Every method output should include evidence refs, limitations, visibility
class, and explicit memory effect. If a method can propose memory updates, the
output must say whether it proposed candidates only, required approval, or made
no memory change.

Live-provider smoke tests are optional, guarded, and never required for default
validation.
