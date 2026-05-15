# Pack Kernel

Status: target architecture

Ordo needs a developer ecosystem, but it should not become a generic plugin
marketplace. The useful abstraction is governed business capability packs.

```text
Core owns trust. Packs own workflows.
```

Core provides identity, policy, access, jobs/DAG execution, events, artifacts,
graph relationships, projections, provider gateways, audit, backup, restore,
and local appliance safety.

Packs provide domain workflow: story production, growth operations, support,
QA/security, systems/admin helpers, media production, onboarding, affiliate
operations, and future developer-built business capabilities.

## Internal Packs First

Build internal packs before promising an external marketplace. Internal packs
prove the extension model while keeping product scope controlled.

Initial internal pack families:

- Story Pack: founder intake, story profile, scrollytelling homepage, image
  briefs, generated image artifacts, homepage versions, refresh proposals, video
  storyboard.
- Knowledge Pack: source manifests, rights/provenance records, curated corpora,
  source spans, claim and entity candidates, alias decisions, graph review,
  consensus thresholds, import/export bundles, and access-aware knowledge
  projections.
- Growth Pack: offers, asks, tracked entry points, trials, feedback, rewards,
  benefit grants, affiliate attribution, outcome evidence.
- Support Pack: handoff queues, owner escalation, support packets, receipts,
  attention routing, A2A support response.
- QA/Security Pack: implementation review packets, test-plan coverage,
  validation evidence, security/privacy findings, A2A review handoff.
- Systems/Admin Pack: provider config, backup/restore, hosted trial state,
  wipe/reset, audit, job inspection, operational safety.

## Pack Manifest

A pack manifest should declare:

```json
{
  "id": "studio.story",
  "name": "Studio Story Pack",
  "version": "0.1.0",
  "status": "internal",
  "capabilities": [],
  "permissions": [],
  "workflowTemplates": [],
  "variableSchemas": [],
  "jobTemplates": [],
  "artifactKinds": [],
  "sourceManifestKinds": [],
  "rightsRules": [],
  "analyticsEvents": [],
  "graphNodeKinds": [],
  "graphEdgeKinds": [],
  "canonicalEdgeVocabulary": [],
  "consensusPolicies": [],
  "projectionSurfaces": [],
  "llmMethods": [],
  "providerNeeds": [],
  "approvalRules": [],
  "fixtures": [],
  "migrations": [],
  "uninstall": {
    "mode": "disable_and_preserve_audit"
  }
}
```

Manifest fields must be explicit. A pack cannot gain hidden authority from
prompt text or UI placement.

## Workflow Templates

Packs may own reusable workflow templates. Core owns validation, compilation,
policy, capability dispatch, artifact boundaries, event emission, graph
promotion rules, and projections.

Workflow declarations should include:

- template id and version;
- typed input schema;
- variable schema and allowed variable sources;
- task bindings to registered capabilities or product-shaped methods;
- fanout groups over bounded typed lists;
- artifact output contracts;
- graph candidate contributions;
- analytics events;
- approval gates;
- deterministic provider mocks and fixtures.

See [Workflow Template Kernel](workflow-template-kernel.md) for the full
contract. A pack workflow can compose many approved tools, but it cannot call
arbitrary providers, execute arbitrary code, or smuggle authority through prompt
text.

## Permissions

Permissions should be narrow and product-shaped:

- read canonical records;
- propose graph candidates;
- create artifacts;
- register job templates;
- start jobs;
- emit events;
- update projections;
- call provider through gateway;
- request approval;
- publish after approval;
- prepare egress packet.

Dangerous permissions require explicit approval:

- external egress;
- public publishing;
- access grants;
- reward grants;
- deletion/wipe/reset;
- provider calls with non-fixture data;
- pack installation or upgrade;
- schema migration.

## Capability Registration

Packs register capabilities into the core catalog. Each capability declares:

- input schema;
- output schema;
- side effects;
- required approvals;
- artifact kinds produced;
- events emitted;
- graph nodes/edges affected;
- provider needs;
- deterministic fixture behavior.

Capabilities must be callable only through policy-checked methods. Packs do not
execute arbitrary code paths outside the registered boundary.

## Job And DAG Registration

Packs may register job templates and task DAGs.

Required DAG metadata:

- template id and version;
- variables and schema;
- task keys;
- dependencies;
- required/optional task flags;
- retry policy;
- capability binding per task;
- expected artifact outputs;
- pause/resume/skip semantics when supported;
- result envelope schema.

Core validates DAG shape, capability binding, leases, retries, idempotency, and
events. Packs define workflow, not scheduler authority.

## Artifact Types

Packs may declare artifact kinds:

- generated image;
- homepage version;
- story profile;
- image brief;
- video storyboard;
- TTS audio;
- QA review packet;
- support packet;
- reward review packet;
- affiliate asset;

Artifact records must include provenance, evidence refs, visibility ceiling,
status, content hash, source refs, and approval state when relevant.

## Graph Contributions

Packs may create or propose graph records only when the manifest declares the
node and edge kinds.

Examples:

```text
job PRODUCED artifact
artifact SUPPORTS claim
homepage_section APPEARS_IN homepage_version
visitor_session ACCEPTED offer
referral_record ATTRIBUTED_TO reward_event
pack INSTALLED capability
```

LLM-assisted graph extraction should create candidates first. Confirmed graph
records require deterministic validation or explicit approval.

## Events Emitted

Every pack mutation should emit events through core:

- pack installed/enabled/disabled/upgraded;
- capability registered;
- job started/completed/failed;
- artifact proposed/approved/published;
- graph candidate proposed/confirmed/rejected;
- reward proposed/granted/reversed;
- egress packet prepared/sent/received;
- provider call requested/completed/failed.

Events must avoid raw secrets, prompt internals, private artifact text, and
provider-sensitive payloads.

## Projections Exposed

Packs may contribute to read models for:

- Member View;
- Studio;
- Growth;
- Support;
- Knowledge;
- Systems/Admin.

Projection outputs must be role-safe. Public/member projections should be built
from approved public-safe records and should never expose staff routing,
provider internals, policy internals, owner-only data, or unsupported claims.

## Lifecycle

Pack states:

```text
available -> installed -> enabled -> disabled -> upgraded -> uninstalled
```

Install:

- validate manifest;
- validate permissions;
- apply migrations if approved;
- register capabilities, jobs, artifacts, graph kinds, projections, and methods;
- emit install events.

Disable:

- stop new pack jobs;
- preserve records and audit;
- hide active projections if policy requires;
- keep historical evidence accessible to authorized viewers.

Uninstall:

- preserve audit by default;
- remove active registrations;
- keep or archive pack-created artifacts and graph records according to policy;
- explain impact before action.

## Migrations

Pack migrations must be versioned and namespaced. They should not mutate core
tables except through approved extension points. Migration tests must cover
fresh install, upgrade, disable, and repeatability.

## Fixtures And Deterministic Tests

Each pack must ship fixtures for:

- manifest validation;
- capability inputs and outputs;
- job DAG execution;
- artifact examples;
- graph nodes/edges;
- projection examples;
- provider fakes;
- policy denial cases.

Default tests must not require live providers, live network, real publishing,
real payments, or hosted infrastructure.

## QA And Security Review Packet

Before a pack is enabled broadly, Ordo should prepare a review packet:

- manifest;
- declared permissions;
- migrations;
- capability schemas;
- job templates;
- artifact kinds;
- graph contributions;
- projection outputs;
- provider needs;
- approval rules;
- fixtures;
- validation results;
- known limitations;
- uninstall impact.

This packet is suitable for local QA, external security review, or future A2A
review handoff.

## Boundaries By Pack Family

Story Pack:

- may propose public story, copy, images, narration, and video artifacts;
- may not publish automatically;
- may not invent claims, proof, scarcity, or provider behavior.
- should declare workflow templates for founder intake, narrative deck, image
  briefs, generated image variants, reviewer feedback, scrollytelling draft,
  QA review, manual or scheduled publish, analytics feedback, and memory
  candidate updates.

Story Pack default workflow:

```text
founder/business intake
-> narrative deck
-> image briefs
-> generated image variants
-> reviewer feedback
-> public derivative preparation
-> scrollytelling draft
-> claim and privacy QA
-> manual/scheduled publish approval
-> content analytics events
-> candidate memory updates
```

Growth Pack:

- may record attribution, feedback, rewards, benefits, and outcome evidence;
- may not pay, grant benefits, or fabricate metrics without policy evidence.

Support Pack:

- may prepare handoff and support packets;
- may not leak private diagnostics or send egress without approval.

QA/Security Pack:

- may inspect bounded review packets and produce findings;
- may not merge, close issues, disable systems, or exfiltrate secrets.

Systems/Admin Pack:

- may surface operational state and propose actions;
- may not wipe, reset, restore, publish, or alter providers without explicit
  approval.

Knowledge Pack:

- may package curated sources, artifacts, claims, aliases, graph candidates,
  reviewed graph records, and import/export metadata;
- may propose graph nodes and edges only through declared vocabularies and
  evidence requirements;
- may create human review requests for entity conflicts, rights checks, source
  spans, claim approval, alias confirmation, and graph promotion;
- may not treat imported graph records, Wikipedia-style links, OCR output, LLM
  extraction, or source metadata as automatic truth;
- should use configurable consensus thresholds according to pack use case,
  visibility, source quality, reviewer weight, and public-risk level.

See [Knowledge Pack Kernel](knowledge-pack-kernel.md) for the future graph-pack
contract. This direction is downstream of the current NYC pilot work, not a
replacement for the Story/Growth first-user loop.

## Validation Expectations

Pack-kernel work must test:

- manifest validation;
- permission denial;
- capability registration;
- DAG registration;
- artifact creation;
- graph contribution rules;
- projection redaction;
- lifecycle transitions;
- uninstall/disable impact;
- deterministic provider fakes;
- review packet generation.
