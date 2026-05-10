# OrdoOS UI/UX Technical Foundation

Status: definitive frontend architecture foundation for 0.1.6

Ordo's frontend should be rebuilt as an operating surface for an agentic
relationship appliance. This document defines the technical substrate before
final visual design, advanced browser processing, GPU visuals, or
scrollytelling surfaces begin.

The goal for 0.1.6 is intentionally narrow: prove the foundation. The reference
UI may be visually plain. Correctness, determinism, accessibility,
internationalization, role safety, and replay behavior come first.

## Scope Boundaries

### Required For 0.1.6

0.1.6 must ship only the foundation needed to support future elite UI work:

- typed frontend domain model;
- canonical event and command envelopes;
- command, message, stream, and replay state machines;
- UI error taxonomy;
- role-safe projection model and leakage tests;
- minimal Ordo shell skeleton;
- chat-first reference surface;
- experience settings manifest;
- effective settings model constrained by role;
- CSS token substrate;
- accessibility and i18n plumbing;
- gateway and read-model adapter contracts;
- browser capability runtime skeleton;
- one or two lightweight proof capabilities, such as `file.hash` and
  `privacy.redaction_scan`;
- replay and idempotency tests;
- role projection tests;
- durability-claim tests.

### Designed For But Not Implemented Yet

The foundation should make these straightforward, but they are not required in
0.1.6:

- final visual design;
- production-grade onboarding surfaces;
- advanced composer polish;
- rich capability cards;
- full report review UI;
- full file upload workflow;
- real browser document extraction beyond a lightweight proof;
- full theme library;
- complete translation catalogs.

### Future Capabilities Enabled By The Foundation

These are explicitly future work:

- OCR;
- PDF page rasterization;
- DOCX extraction;
- archive inspection;
- image normalization;
- audio/video metadata extraction;
- media processing;
- local embeddings/search;
- GPU topology rendering;
- WebGPU signal fields;
- advanced journey report rendering;
- scrollytelling Home/About surfaces.

### Donor Patterns / Inspiration Only

Donor patterns are reference material, not dependencies. Any reused pattern
must be re-expressed through the new ports, manifests, reducers, projection
contracts, and tests. Do not preserve old assumptions merely because a prior
implementation used them.

Useful donor areas:

- `../ordoSite` runtime theme manifest, accessibility settings, conversational
  UI commands, chat shell, markdown-safe streaming, capability cards, progress
  strip, and browser/WASM runtime.
- `../testing` scroll-linked presentation stages, reduced-motion-aware motion
  components, and scrollytelling mechanics.

## Product Frame

Ordo is not a generic SaaS dashboard and not a chatbot widget. It is an
agentic orchestration operating system for a solopreneur or small operator.

The frontend coordinates:

- relationship conversations;
- public/client/staff/affiliate/admin/owner projections;
- handoffs, modes, and delegation;
- offers, trials, feedback, reviews, referrals, and outcomes;
- evidence, events, replay, reports, and artifact review;
- provider, privacy, policy, and accounting boundaries;
- browser-local candidate work;
- theme, accessibility, motion, performance, and internationalization.

Every surface should answer:

- What is happening?
- Who is acting?
- Is it durable?
- Is it private?
- Can I intervene?
- What evidence supports this?
- What changed because of my action?

## Architecture Shape

The frontend is organized around five layers:

```text
ports and adapters
  -> domain contracts and state machines
  -> role-safe projections
  -> experience control plane
  -> render surfaces
```

Effects live at the edge. Core frontend logic is pure, typed, deterministic,
and testable. Components render view models; they do not invent product truth.

## Canonical Frontend Contracts

These five contracts are the backbone of implementation.

### 1. Command Contract

```text
user intent
  -> local intent id
  -> command envelope with client id
  -> gateway acknowledgement or rejection
  -> durable result or preserved retryable intent
```

Rules:

- Every command has a stable `intentId` and `clientId`.
- UI may optimistically render local intent.
- UI must not claim durability until a durable daemon event arrives.
- Rejected commands preserve recoverable user intent.

### 2. Message Contract

```text
local -> queued -> acked -> durable
                        -> streaming -> durable final
                        -> rejected
                        -> replayed
```

Rules:

- `local`, `queued`, `acked`, `durable`, `streaming`, `replayed`, and
  `rejected` are distinct states.
- A message cannot be both `durable` and `rejected`.
- Replayed messages reconcile by canonical id, `clientId`, sequence, and
  cursor.

### 3. Projection Contract

```text
durable evidence -> projection policy -> role-safe view model
```

Rules:

- Components consume projected view models, not raw daemon payloads.
- Projection is role-aware and fixture-tested.
- Client-safe projections never expose staff/provider/policy/private internals.

### 4. Capability Contract

```text
browser candidate work
  -> candidate result envelope
  -> daemon validation
  -> durable artifact identity
```

Rules:

- Browser output is candidate evidence until daemon validation.
- Browser capabilities can improve responsiveness and reduce server work.
- Browser candidates are never authoritative business facts.

### 5. Experience Contract

```text
settings manifest
  -> requested settings
  -> role-constrained effective settings
  -> CSS variables/classes
  -> accessible rendered surface
```

Rules:

- User preference cannot grant access to owner/staff internals.
- Unavailable settings are shown as unavailable instead of silently applied.
- Components consume semantic tokens only.

## Event And Command Envelopes

The frontend uses generic envelopes at its boundaries.

```ts
interface FrontendEventEnvelope<T> {
  eventId: string;
  sequence: number;
  cursor: string;
  occurredAt: string;
  actor: ActorRef;
  visibility: RoleVisibility;
  kind: string;
  payload: T;
  evidenceRefs?: EvidenceRef[];
}

interface CommandEnvelope<T> {
  clientId: string;
  intentId: string;
  kind: string;
  issuedAt: string;
  actor: ActorRef;
  payload: T;
}

interface ActorRef {
  actorId: string;
  role: ProductRole;
  displayKind: "visitor" | "client" | "affiliate" | "staff" | "manager" | "admin" | "owner" | "ordo_agent" | "system";
}

type RoleVisibility =
  | "public"
  | "client"
  | "affiliate"
  | "staff"
  | "manager"
  | "admin"
  | "owner"
  | "system";
```

The renderer must reconcile by `clientId`, canonical ids, sequence, and cursor.
Replay must be idempotent: applying the same durable event sequence more than
once produces the same view model.

## Evidence Contract

Evidence is typed reference data, not decoration.

```ts
interface EvidenceRef {
  id: string;
  kind:
    | "daemon_event"
    | "artifact"
    | "policy_decision"
    | "privacy_decision"
    | "provider_result"
    | "browser_candidate"
    | "user_confirmation";
  durability: "candidate" | "durable";
  visibility: RoleVisibility;
  summary?: string;
}
```

The `EvidenceRail` renders evidence. It does not define what evidence is.
Evidence visibility is enforced before rendering.

## Role-Safe Projection

Projection is a security boundary.

Legend:

- yes: may render in normal view;
- gated: may render only with explicit role/capability and suitable surface;
- no: must not render.

| Projection category | Public | Client | Affiliate | Staff | Manager/Admin | Owner/System |
| --- | --- | --- | --- | --- | --- | --- |
| Message text addressed to viewer | yes | yes | yes | yes | yes | yes |
| Raw prompt | no | no | no | no | gated | gated |
| Provider payload | no | no | no | no | gated | gated |
| Policy internals | no | no | no | gated | gated | yes |
| Privacy placeholder map | no | no | no | no | gated | yes |
| Staff notes | no | no | no | yes | yes | yes |
| Confidence internals | no | no | no | gated | gated | yes |
| Token ledger | no | no | no | no | gated | yes |
| Staff routing details | no | no | no | yes | yes | yes |
| Accounting evidence | no | no | no | gated | gated | yes |
| Artifact metadata | public-safe | client-safe | scoped | yes | yes | yes |
| Browser candidate output | no | no | no | gated | gated | yes |
| Durable daemon evidence | public-safe | client-safe | scoped | yes | yes | yes |

Every projection category must have fixtures proving that client-safe surfaces
do not leak staff/provider/policy/private internals. Projection tests should
cover public, client, affiliate, staff, admin, and owner views.

## UI Error Taxonomy

```ts
type UiErrorKind =
  | "user_input_invalid"
  | "permission_denied"
  | "policy_rejected"
  | "privacy_required"
  | "network_transient"
  | "gateway_rejected"
  | "provider_unavailable"
  | "capability_unavailable"
  | "capability_failed"
  | "artifact_validation_failed"
  | "replay_gap"
  | "unknown";
```

| Error kind | Retryable | Preserve intent | User correction | Visibility | Telemetry | Blocks composer |
| --- | --- | --- | --- | --- | --- | --- |
| `user_input_invalid` | no | yes | yes | actor | no raw input | no |
| `permission_denied` | no | yes | maybe | actor/staff | yes | no |
| `policy_rejected` | no | yes | maybe | actor/staff | yes | no |
| `privacy_required` | after correction | yes | yes | actor/staff | yes | maybe |
| `network_transient` | yes | yes | no | actor | yes | no |
| `gateway_rejected` | depends | yes | maybe | actor/staff | yes | no |
| `provider_unavailable` | yes | no | no | staff/admin/owner | yes | no |
| `capability_unavailable` | fallback | yes | no | actor/staff | yes | no |
| `capability_failed` | depends | yes | maybe | actor/staff | yes | no |
| `artifact_validation_failed` | after correction | yes | yes | actor/staff | yes | no |
| `replay_gap` | recover | no | no | staff/admin/owner | yes | maybe |
| `unknown` | maybe | yes | maybe | actor/staff | yes | no |

Errors must map to explicit UI states. Silent failure is not allowed.

## Frontend Threat Model

The frontend must defend against:

- role projection leakage;
- raw provider or policy internals appearing in client views;
- accidental logging of private content;
- unsafe markdown rendering;
- blob URL persistence;
- browser candidate output being mistaken for durable truth;
- malicious or oversized files during local preflight;
- replay duplication;
- sequence/cursor inconsistency;
- model-authored commands escaping into raw execution;
- cross-surface leakage between client, staff, admin, owner, and system views.

Markdown rendering must be sanitized. Links must be safe. Embedded HTML must be
disabled unless an explicit sanitizer and allowlist exist. Model-authored
commands must resolve through typed command envelopes and registries; they must
never execute raw code, shell fragments, or arbitrary browser APIs.

## Observability Requirements

Telemetry must make the substrate debuggable without leaking private content.

Track:

- command lifecycle timing;
- stream start, interruption, completion, and recovery;
- replay gaps;
- duplicate-message prevention;
- role projection and redaction failures;
- browser capability probe results;
- worker timeout, failure, cancellation, and fallback reasons;
- accessibility mode selection;
- performance mode selection;
- render and hydration errors where applicable.

Never log in client-safe telemetry:

- raw prompts;
- provider payloads;
- privacy placeholder maps;
- raw file contents;
- staff-only notes;
- token/accounting internals unless owner/admin-scoped.

## Experience Control Plane

Experience settings are permission-sensitive.

```ts
type ThemeId = "ai_swiss" | "bauhaus" | "fluid" | "high_contrast" | "minimal";
type Density = "compact" | "normal" | "relaxed";
type MotionMode = "off" | "restrained" | "expressive" | "cinematic";
type PerformanceMode = "economy" | "standard" | "enhanced" | "cinematic";
type EvidenceDetail = "brief" | "standard" | "full" | "owner_cockpit";
type PrivacyDisplay = "client_safe" | "staff_evidence" | "owner_internals";
type TypeScale = "sm" | "md" | "lg" | "xl";
type ContrastMode = "standard" | "high";
type ColorBlindMode = "none" | "deuteranopia" | "protanopia" | "tritanopia";
type LocaleId = "en-US";

interface ExperienceSettings {
  theme: ThemeId;
  density: Density;
  motion: MotionMode;
  typeScale: TypeScale;
  contrast: ContrastMode;
  evidenceDetail: EvidenceDetail;
  privacyDisplay: PrivacyDisplay;
  performanceMode: PerformanceMode;
  locale: LocaleId;
  colorBlindMode: ColorBlindMode;
  localComputeEnabled: boolean;
  gpuVisualsEnabled: boolean;
}

interface ExperienceConstraint {
  setting: keyof ExperienceSettings;
  requestedValue: unknown;
  effectiveValue: unknown;
  reason: "role_unavailable" | "capability_unavailable" | "reduced_motion" | "policy";
}

interface EffectiveExperienceSettings {
  requested: ExperienceSettings;
  effective: ExperienceSettings;
  constraints: ExperienceConstraint[];
}
```

A client must never render owner-only internals by changing settings. If a
setting is unavailable for a role or capability profile, the UI should show
that it is unavailable.

### Preference Persistence

Persisted experience preferences are actor/account scoped requested settings,
not effective settings. Readback always re-runs the role/capability/policy
resolver before rendering.

The durable preference contract stores:

- actor id;
- preference schema version;
- requested settings JSON;
- created/updated timestamps.

The frontend preference port loads and saves records without coupling reusable
components to storage. Anonymous visitors receive deterministic safe defaults
without requiring a persisted account. Malformed, unknown, or unsupported stored
values fall back to defaults and produce explicit UI errors.

Accessibility settings are first-class persisted preferences:

- type scale / font size;
- contrast;
- reduced motion;
- color-blind mode;
- density;
- theme;
- locale;
- performance mode.

Preferences must never store raw prompts, provider payloads, policy internals,
messages, private terms, staff notes, raw emails, phone numbers, or secrets.

## CSS Token Governance

Theme manifests define semantic tokens. CSS variables expose runtime values.
Components consume semantic tokens only.

Rules:

- Components must not hardcode role/status colors.
- Components must not create component-local theme truth.
- Tests verify required tokens exist for every theme.
- Theme, density, motion, and type scale changes must work without rewriting
  component internals.

Required first token groups:

- surface;
- text;
- border;
- focus;
- status;
- role;
- evidence;
- spacing;
- radius;
- motion.

## Internationalization

Internationalization is part of the foundation.

Requirements:

- no hardcoded user-facing strings in reusable components;
- locale-aware date, time, number, currency, and relative-time formatting;
- timezone handling for events, reports, trials, and attribution;
- bidirectional text readiness;
- long-string layout resilience;
- `I18nCatalogPort` for catalog lookup;
- tests for long strings, RTL smoke, timezone boundaries, and translated
  labels in narrow layouts.

Use platform `Intl` APIs for formatting. The first catalog can be small, but
the architecture should not require future string hunts.

## Accessibility

Accessibility is part of the core UI state model.

Required:

- full keyboard operation;
- visible focus states;
- skip links and landmark structure;
- screen-reader friendly streaming;
- reduced-motion equivalents;
- high contrast mode;
- density and type scaling;
- ARIA labels for evidence/status controls;
- focus traps for modal/drawer surfaces;
- focus recovery after route/surface changes;
- accessible error and rejection states;
- live regions for important state changes.

Do not announce every streamed token. Assistive technology should receive
meaningful summaries such as response started, checking privacy, reading
evidence, writing answer, response completed, action required, command
rejected, and connection recovered.

## Browser Capability Runtime

0.1.6 implements only the runtime skeleton and one or two lightweight proof
capabilities.

Allowed 0.1.6 proof capabilities:

- `file.hash`;
- `privacy.redaction_scan`;
- `document.metadata_extract` only if cheap and deterministic.

Deferred:

- PDF rasterization;
- OCR;
- DOCX extraction;
- image normalization;
- archive inspection;
- media processing;
- local embeddings/search;
- GPU visual rendering.

Runtime guardrails:

- memory budget per job;
- worker pool concurrency;
- timeout behavior;
- cancellation;
- fallback reason reporting;
- deterministic result envelopes;
- malicious and oversized file fixtures;
- explicit candidate labeling until daemon validation returns durable artifact
  identity.

## OrdoOS Shell And Routing

0.1.6 should introduce only the shell skeleton.

Required slots:

- root shell;
- center stage;
- composer slot;
- evidence/action rail slot;
- active work strip slot;
- user/experience menu slot;
- route/surface registry.

Root routing rule:

- public/unknown user: product/chat entry or onboarding;
- authenticated operator: chat-first operating surface;
- owner/system role: System Brief available through owner/system route;
- staff/admin: cockpit surfaces available by role, not default root unless
  configured.

The current System Brief screen should not remain the default product root once
the new shell is active.

## SOLID And GoF Architecture

The frontend should be internally boring and excellent.

SOLID interpretation:

- components render;
- reducers reduce;
- adapters adapt;
- registries resolve;
- ports abstract;
- domain logic depends on typed ports and events, not concrete transports.

Useful patterns:

- Strategy: projection policies, file processors, redaction scanners.
- Adapter: gateway, read model, storage, worker, WASM, i18n.
- Facade: browser capability runtime and experience settings service.
- Command: user intents and gateway commands.
- Observer: progress, stream, and capability events.
- State: message, handoff, stream, capability, and upload lifecycles.
- Registry: themes, capabilities, cards, commands, routes, projections.
- Factory: worker creation, capability adapters, deterministic fixtures.
- Decorator: redaction, telemetry, timing, guard wrappers.

Avoid inheritance-heavy component systems. Prefer typed composition and small
pure modules.

## Implementation Order

Recommended milestone:

```text
0.1.6 OrdoOS Frontend Architecture Foundation
```

### Phase 1: Canonical Contracts

- domain types;
- event and command envelopes;
- message, stream, command, and replay state machines;
- error taxonomy.

### Phase 2: Projection Safety

- role projection model;
- redaction fixtures;
- client/staff/admin/owner projection tests.

### Phase 3: Shell Substrate

- `OrdoShell` layout skeleton;
- composer slot;
- evidence/action rail slots;
- active work strip slot;
- route/surface registry.

### Phase 4: Experience Substrate

- settings manifest;
- effective settings model;
- CSS token runtime;
- accessibility profile;
- i18n catalog port.

### Phase 4B: Experience Preference Persistence

- actor/account preference storage contract;
- requested-settings serialization;
- role-constrained effective-settings readback;
- accessibility preference round-trip tests;
- malformed stored-value fallback tests.

### Phase 5: Realtime Foundation

- gateway/read-model ports;
- optimistic command queue;
- ack/reject reconciliation;
- replay idempotency tests.

The Phase 5 foundation is a transport seam plus pure reducers, not the final
chat UI. Components consume read models, while gateway adapters own connect,
disconnect, command send, dispatch receive, and replay behavior.

Implemented contract:

- `RealtimeGatewayPort` isolates connect/disconnect/send/replay/status.
- `RealtimeState` keeps commands, optimistic messages, replay state, stream
  state, and explicit UI errors.
- `queueRealtimeCommand` records recoverable user intent and candidate message
  state.
- `reconcileGatewayAck` handles ack/reject without claiming durability.
- `applyRealtimeEvent` applies durable event envelopes, ignores duplicates, and
  rejects unsupported events.
- `projectRealtimeReadModel` produces role-safe conversation, message,
  composer, evidence rail, stream, and replay view models.
- `InMemoryRealtimeGateway` gives deterministic no-network test coverage.

Rules:

- Acked commands remain non-durable until matching durable daemon evidence
  arrives.
- Duplicate event ids are idempotent.
- Sequence gaps produce explicit `replay_gap` errors.
- Client-safe read models are generated through projection policy and must not
  expose raw prompt, provider payload, policy internals, privacy placeholder
  maps, staff routing, or staff-only notes.

### Phase 6: Capability Substrate

- browser capability port;
- worker runtime skeleton;
- `file.hash` or another lightweight proof capability;
- fallback and cancellation tests.

### Phase 7: Hardening

- Playwright shell tests;
- reduced-motion tests;
- long-string and RTL smoke tests;
- performance-mode tests.

Keep the reference UI visually plain. Correctness, determinism,
accessibility, and testability come before elite visuals.

## 0.1.6 Acceptance Gates

The foundation is not accepted unless all of these are true:

- A message cannot render as durable without a durable daemon event.
- A rejected command preserves recoverable user intent.
- Replay of the same event sequence is idempotent.
- Client-safe projections expose no staff/provider/policy/private internals.
- Experience settings change theme, density, motion, and type scale without
  component-local styling.
- Reduced-motion mode displays equivalent status information.
- No reusable component contains hardcoded user-facing strings.
- Missing browser capability renders an explicit fallback state.
- Browser-generated artifacts remain candidate until daemon validation.
- The composer remains interactive during non-blocking background work.
- Unsafe markdown cannot execute scripts or leak dangerous HTML.
- Projection, replay, and capability failures produce explicit error states.

## Non-Goals

- Do not implement final visual design in 0.1.6.
- Do not add WebGPU before the core renderer is correct.
- Do not build scrollytelling surfaces in 0.1.6.
- Do not start with OCR, PDF rasterization, DOCX extraction, media processing,
  archive inspection, local embeddings, or GPU topology rendering.
- Do not make browser WASM authoritative for durable business facts.
- Do not build a generic CRM dashboard.
- Do not preserve awkward current frontend scaffolding for compatibility.
- Do not expose staff/provider/policy/private internals to client surfaces.
- Do not add real outbound email delivery as part of this foundation.

## Closeout Standard

The foundation is ready when future UI implementation agents can:

- add a new surface without inventing global state;
- add a new capability without custom one-off progress logic;
- add a new theme without editing component internals;
- add a new locale without hunting hardcoded strings;
- add a new upload processor without blocking the main thread;
- test role-safe output deterministically;
- replay durable events idempotently;
- render the same durable conversation through client, staff, and owner
  projections without duplicating business logic.
