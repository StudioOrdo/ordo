# Workflow Template Kernel

Status: target architecture, not fully implemented

Ordo needs workflow templates so useful work can be composed without turning
the product into a generic workflow builder or a loose bag of provider calls.
The owner should be able to ask for outcomes like "make twelve zodiac images"
or "write an article about aliens and create a matching image" while Ordo
keeps structure, policy, access, artifacts, audit, and publication authority in
deterministic code.

The rule is:

```text
Arbitrary composition of approved typed capabilities.
No arbitrary model/tool access.
```

## Boundary Model

Workflow templates sit between packs and job runs:

- Generic capabilities are reusable tool primitives such as image generation,
  image review, TTS, transcription, search, QR generation, render, screenshot
  QA, and artifact derivative preparation.
- Product-shaped methods wrap those capabilities in Ordo product language such
  as `story.createImageBriefs`, `homepage.generateSectionHeroImage`, or
  `artifact.preparePublicDerivative`.
- Workflow templates declare typed inputs, variables, bindings, fanout groups,
  approval gates, artifact outputs, visibility rules, and task dependencies.
- Jobs and DAG task runs execute one compiled snapshot of a workflow template.
- Packs install reusable workflow declarations and fixtures, but Core validates
  permission, capability, provider, artifact, graph, event, and projection
  boundaries.
- Artifacts preserve produced work, provenance, checksums, visibility,
  approval state, and evidence refs.
- Graph memory links workflows, inputs, artifacts, claims, approvals, feedback,
  events, and outcomes.
- Projections/read models surface the workflow state to Member View, Studio,
  Support, Knowledge, Growth, and Systems.

The workflow template is not product truth. It is a reusable plan for creating
jobs against canonical records, artifacts, graph relationships, and events.

Workflow structure must be deterministic. An LLM may help classify user intent,
choose among approved workflow templates, summarize workflow state, draft task
content, or propose missing inputs, but it must not improvise task structure,
invent dependencies, skip approval gates, or create hidden provider/tool access.
The compiled DAG is Ordo's work contract.

## Template Primitives

Every workflow template should define:

- `template_id` and semantic `version`;
- typed input schema;
- workflow variables with type, source, visibility, and evidence metadata;
- task bindings from variables, artifacts, graph reads, pack config, or prior
  task outputs into product-shaped method inputs;
- fanout groups over bounded lists;
- task output references;
- artifact references and expected artifact kinds;
- approval gates for publishing, external egress, access grants, reward grants,
  deletion, provider calls with non-fixture data, and sensitive review loops;
- visibility classes for inputs, task results, artifacts, and graph records;
- policy and capability requirements;
- provider requirements and fallback/deterministic fixture behavior;
- idempotency and retry behavior;
- audit/event emission;
- projection expectations.

Compiled job plans should snapshot the template version, resolved variable
schema, policy decisions, provider requirements, and task graph. Later template
edits must not change an already-started run.

Compiled plans should be explainable without asking an LLM to reconstruct the
workflow. If a task is blocked, skipped, retried, waiting for a person, or ready
to run, that state should come from job/DAG records and events. The LLM may
translate the state into plain language; it does not own the state.

## Variable Rules

Variables are typed data, not prompt magic.

Rules:

- no arbitrary SQL;
- no unbounded generic context access;
- no unsafe prompt-only string interpolation as core structure;
- variables resolve only from typed inputs, approved pack config, artifacts,
  graph methods, prior task outputs, or canonical records reached through
  product-shaped methods;
- every variable carries visibility metadata and evidence refs when derived
  from existing records;
- private inputs must not leak into public/member artifacts, prompts, graph
  labels, analytics, or projections unless an explicit policy allows the
  transformed public-safe derivative;
- missing variables block compilation or create a Request for human input;
- rejected, expired, superseded, or private artifacts are not valid variable
  sources for public publication unless a reviewed derivative exists.

String interpolation may still be used at the edge to construct a provider
prompt, filename, or copy draft, but the deterministic workflow structure must
come from typed bindings.

## Fanout

Fanout repeats a bounded sub-DAG over a typed list. It is useful for image
sets, content series, batch QA, translation, affiliate assets, or campaign
variants.

Fanout rules:

- the collection source must be typed and bounded;
- each item gets a stable item key and idempotency key;
- each item result is its own artifact or task result envelope;
- failures can be retried, skipped, or paused without corrupting sibling items;
- public/member projections show only safe aggregate state unless item-level
  output is approved for that viewer.

## Examples

### Zodiac Image Set

```text
template_id: story.zodiac_image_set
inputs:
  collection_name: "zodiac"
  subjects: ["aries", "taurus", "gemini", ...]
  visual_style: "cinematic editorial"
  output_count_per_subject: 1

fanout subject in subjects:
  1. story.createImageBrief(subject, visual_style)
  2. image.generateVariants(brief, count=output_count_per_subject)
  3. image.reviewAgainstBrief(image, brief)
  4. artifact.preparePublicDerivative(image, review)
```

The collection becomes graph memory only through artifacts, evidence refs, and
candidate/confirmed relationships. The image provider does not own the truth
that a zodiac collection exists.

### Article Plus Image

```text
template_id: content.article_with_image
inputs:
  topic: "aliens"
  audience: "curious small business owners"

tasks:
  1. content.draftArticle(topic, audience)
  2. content.extractClaims(article)
  3. story.createImageBrief(topic, article.summary)
  4. image.generateVariants(brief, count=3)
  5. image.reviewAgainstBrief(images, brief)
  6. artifact.preparePublicDerivative(selected_image, review)
  7. publish.requestApproval(article, selected_image)
```

`topic` is a typed shared variable. It is not repeatedly copied into hidden
prompts without provenance.

### Story Pack Scrollytelling

```text
template_id: studio.story.scrollytelling_homepage
inputs:
  founder_profile
  business_positioning
  offer_ids
  tracked_entry_point_ids
  publish_mode: "manual" | "scheduled"

tasks:
  1. story.captureFounderIntake
  2. homepage.createNarrativeDeck
  3. story.createImageBriefs
  4. image.generateVariants
  5. image.reviewAgainstBrief
  6. artifact.preparePublicDerivatives
  7. homepage.compileScrollytellingDraft
  8. claim.validatePublicClaims
  9. publish.requestApproval
  10. analytics.recordContentPublish
  11. memory.proposeCandidateClaims
```

The LLM may add color and interpretation. Ordo owns deck structure, claim
validation, artifact visibility, approval, publication, analytics, and memory
candidate rules.

In this workflow, AI work should appear as bounded tasks inside the DAG: draft
language, create image briefs, review against a brief, extract claims, explain
performance, or summarize a review packet. Ordo still owns task ordering,
visibility, evidence, approvals, artifact creation, analytics truth, and memory
candidate/promotion boundaries.

## Events And Audit

Workflow execution should emit events for:

- template compiled;
- variable resolved or missing;
- task started, completed, failed, retried, skipped, or paused;
- artifact created, reviewed, approved, rejected, published, or superseded;
- provider call requested, completed, failed, or denied;
- graph candidate proposed, confirmed, rejected, or superseded;
- content event recorded;
- memory candidate proposed;
- approval requested, granted, or denied.

Events should identify records and evidence refs, not raw secrets, prompt
internals, provider payloads, private artifact text, or private task outputs.

## Validation Expectations

Workflow-template work must test:

- variable schema validation and missing variable behavior;
- variable resolution from inputs, artifacts, graph methods, prior outputs, and
  approved pack config;
- fanout DAG expansion, stable item keys, and idempotency;
- retry, skip, pause, and partial failure behavior;
- artifact visibility and public derivative rules;
- policy denial for provider, egress, publish, access, reward, and destructive
  gates;
- deterministic provider mocks;
- event emission and replay-safe projection inputs;
- no live-provider default validation.

Default validation must not require live LLMs, live image generation, live TTS,
live publishing, hosted infrastructure, or network access.
