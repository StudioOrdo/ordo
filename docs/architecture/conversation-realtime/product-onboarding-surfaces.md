# Product Onboarding Surfaces

Status: Superseded 0.1.7 implementation canon

This arc was superseded before implementation after the May 12, 2026 product
direction reset. Issues #205 through #213 were closed as stale/not planned. The
active replacement arc is [Interactive Account And LLM Chat](interactive-account-llm-chat.md),
which first connects local account entry, chat bootstrap, browser `/chat/ws`,
and daemon-owned LLM testing.

0.1.5 proved that Ordo can evaluate realistic product journeys with
deterministic and guarded-live evidence. 0.1.6 proved the OrdoOS frontend
substrate: typed contracts, role-safe projections, clean agent screen context,
shell slots, experience settings, persisted preferences, realtime read models,
browser capability candidates, and hardening gates.

0.1.7 turns those foundations into real product routes. The goal is not final
visual polish. The goal is to make QR/event entry, public offer context,
trial acceptance, client-safe relationship chat, review-return, referral, and
staff/admin review paths usable through role-safe surfaces.

Default development and validation remain deterministic, provider-free,
email-provider-free, network-free, and CI-safe. Live provider calls and real
outbound email remain explicitly guarded or deferred by their own contracts.

## Selected Arc

The superseded milestone was:

```text
0.1.7 Product Onboarding Surfaces
```

This was selected before the current direction reset. It remains useful as a
future-direction reference, but it is not the active implementation queue.

## Alternatives Considered

### Chat-First OrdoOS Operating Surface

Included as Phase 1. Root/chat behavior is the first onboarding surface because
the current root still defaults to the owner/system System Brief. The arc should
move public/client/operator entry toward chat-first OrdoOS behavior while
keeping owner/system appliance evidence available through role-gated routes.

### Product Ops/Admin Cockpit

Partially included. Staff/admin review work is necessary, but it should follow
the public/client paths it reviews. 0.1.7 includes a first cockpit slice after
QR, offer, relationship chat, review-return, and referral paths are grounded.

### Governed Outbound Communication

Deferred. PR #179 made the current decision explicit: review-request email
remains a governed simulated artifact/link until a later accepted issue defines
owner approval, recipient consent or lawful basis, suppression/unsubscribe,
deliverability, provider-secret handling, audit trail, rate/spend caps,
redaction, no raw fixture emails, and explicit opt-in live/email guards.

### Deeper Live LLM Journey Execution

Deferred. The live runner, OpenAI-compatible adapter, spend guards, persona
library, journey evals, and cross-run reports exist. More live execution will
be more valuable after product surfaces produce real route, UI, and user-state
evidence.

### Visual Design / AI Swiss Experience System

Deferred. 0.1.7 may stay visually plain. Final visual language, advanced motion,
scrollytelling, and elite interaction polish should sit on route contracts that
already prove role safety, durability, accessibility, and replay behavior.

## Current-Code Grounding

Each implementation issue must still begin with fresh diagnosis. Current repo
evidence shows the arc is feasible:

- `lib/ordoos-frontend-contracts.ts`, `lib/ordoos-role-projection.ts`,
  `lib/ordoos-shell.ts`, `lib/ordoos-experience.ts`,
  `lib/ordoos-experience-preferences.ts`, `lib/ordoos-realtime.ts`, and
  `lib/ordoos-browser-capabilities.ts` provide the frontend substrate.
- `components/ordo-shell.tsx`, `components/product-shell.tsx`, and
  `components/conversation-foundation.tsx` provide current shell and reference
  conversation surfaces.
- `app/page.tsx` still renders the System Brief as the root owner/system-style
  surface; `/chat`, `/home`, `/offers`, `/asks`, and `/latest` preserve the
  product route shape but remain mostly fixture or shell-driven.
- `entry_points.rs` supports tracked entry points, public-safe resolution, QR
  payloads, public paths, and visitor sessions.
- `offers.rs` supports public offer listing, public offer acceptance, and
  30-day trial creation.
- `public_surfaces.rs` and `surface_briefs.rs` support public About, Offers,
  Asks, Feed, Home/About narrative, and offer/ask read models.
- `conversations.rs` and `conversation_gateway.rs` support relationship
  conversations, participants, messages, replay, handoffs, modes, delegation,
  and client-safe/staff-only boundaries.
- `feedback.rs` supports private feedback, review candidates, consent,
  approval, publication, featured, and retired states.
- `connections.rs` supports affiliate connections and scoped grants.
- `attribution.rs` supports referral records, business outcomes, and outcome
  attribution candidates.
- `live_eval_runner.rs` and `live_journey_report.rs` provide QR-to-trial,
  review-return, affiliate-referral, admin/staff, and report evidence for
  acceptance.

## Product Principles

- The first public screen should be usable product onboarding, not a generic
  appliance dashboard or marketing-only landing page.
- Public claims must cite durable evidence or be clearly aspirational.
- Ordo can help a person decide whether trying OrdoStudio fits their situation,
  but it must preserve agency and avoid manipulation.
- No fake urgency, fake scarcity, fake reviews, fake metrics, unsupported
  authority, or unsupported social proof.
- Public/client surfaces must not expose staff routing, policy internals,
  provider mechanics, prompt contents, confidence scores, token ledgers,
  privacy placeholder maps, or staff-only notes.
- Staff/admin surfaces may inspect evidence and limitations, but ordinary staff
  should not default to appliance internals such as logs, backup, readiness, or
  low-level system events.
- Review-request email remains simulated for this arc unless a future governed
  delivery issue changes that boundary.
- Default tests must remain deterministic and network-free.

## Delivery Order

1. #205 Phase 0: Align product onboarding surface implementation canon.
2. #206 Phase 1: Make root/chat the chat-first OrdoOS entry surface.
3. #207 Phase 2: Add QR/event landing surface.
4. #208 Phase 3: Implement OrdoStudio trial offer page and acceptance flow.
5. #209 Phase 4: Wire client-safe relationship conversation onboarding.
6. #210 Phase 5: Add review-return surface for governed simulated review
   links.
7. #211 Phase 6: Add affiliate/referral landing and attribution surface.
8. #212 Phase 7: Add staff/admin onboarding review cockpit slice.
9. #213 Phase 8: Add route and UI smoke coverage tied to journey eval
   evidence.

## Acceptance Evidence

Each implementation issue should use the completed eval foundation as evidence,
not as a substitute for product validation:

- deterministic backend route/domain tests for changed contracts;
- route, component, or read-model tests for public/client/staff behavior;
- Playwright smoke coverage for visible route changes;
- redaction and trust-boundary assertions for client/public output;
- 0.1.5 journey eval compatibility where the surface maps to QR-to-trial,
  review-return, affiliate-referral, admin/staff, or report evidence;
- screenshots or browser evidence for meaningful frontend changes;
- `npm run export` and `git diff --check`;
- the broader Rust/frontend validation matrix required by the files changed.

## Non-Goals

- Do not implement hosted identity, OAuth, payments, billing, or account
  provisioning in 0.1.7 unless a later issue proves it is the smallest safe
  slice.
- Do not add real outbound email delivery in this arc.
- Do not make live provider calls in default CI or route tests.
- Do not build a generic CRM dashboard.
- Do not expose admin/system internals to ordinary staff or clients.
- Do not automatically file GitHub issues from journey reports.
