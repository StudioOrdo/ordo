# Product Onboarding Surfaces

Status: 0.1.6 planning contract

0.1.5 proved that Ordo can evaluate realistic product journeys with
deterministic and guarded-live evidence. The next arc turns that evidence into
usable product surfaces: the QR/event landing path, public offer and trial
acceptance, client-safe relationship conversation onboarding, review-return
links, affiliate/referral landing, and the first staff/admin review cockpit.

Default development and validation remain deterministic, provider-free,
email-provider-free, network-free, and CI-safe. Live provider calls and real
outbound email remain explicitly guarded or deferred by their own contracts.

## Selected Arc

The selected next milestone is:

```text
0.1.6 Product Onboarding Surfaces
```

This is the highest-leverage next step because the 0.1.5 journey evals already
prove the backend business loop, but the real public, client, affiliate, and
staff product paths are still mostly shells. The product now needs usable
surfaces that can be validated against the same journey evidence.

## Alternatives Considered

### Governed Outbound Communication

Deferred. PR #179 made the current decision explicit: review-request email
remains a governed simulated artifact/link until a later accepted issue defines
owner approval, recipient consent or lawful basis, suppression/unsubscribe,
deliverability, provider-secret handling, audit trail, rate/spend caps,
redaction, no raw fixture emails, and explicit opt-in live/email guards.

### Deeper Live LLM Journey Execution

Deferred. The live runner, OpenAI-compatible adapter, spend guards, persona
library, and cross-run reports exist. More live execution will be more valuable
after the product surfaces produce real route, UI, and user-state evidence.

### Product Ops/Admin Cockpit

Partially included. Staff/admin review work is necessary, but it should follow
the public/client onboarding surfaces it reviews. 0.1.6 includes a first
cockpit slice after QR, offer, chat, review-return, and affiliate/referral
surfaces are grounded.

## Current-Code Grounding

The implementation phases should begin with fresh diagnosis, but the current
repo already provides the substrate:

- `entry_points.rs` supports tracked entry points, public-safe resolution, QR
  payloads, public paths, and visitor sessions.
- `offers.rs` supports public offer listing, public offer acceptance, and
  30-day trial creation.
- `public_surfaces.rs` and `surface_briefs.rs` support public About, Offers,
  Asks, Feed, Home/About narrative, and offer/ask intent read-model contracts.
- `conversations.rs` and `conversation_gateway.rs` support canonical
  relationship conversations, participants, messages, replay, handoffs, modes,
  delegation, and client-safe/staff-only boundaries.
- `feedback.rs` supports private feedback, review candidates, consent,
  approval, publication, featured, and retired states.
- `connections.rs` supports affiliate connections and scoped grants.
- `attribution.rs` supports referral records, business outcomes, and outcome
  attribution candidates.
- `live_eval_runner.rs` and `live_journey_report.rs` provide QR-to-trial,
  review-return, affiliate-referral, admin/staff, and report evidence for
  acceptance.
- Current frontend routes for `/home`, `/offers`, `/asks`, `/latest`, and
  `/chat` preserve the product navigation shape, but public/client onboarding
  behavior remains mostly fixture or shell-driven.

## Product Principles

- The first public screen should be a usable onboarding surface, not a
  marketing-only landing page.
- Public claims must cite durable evidence or be clearly aspirational.
- Ordo can help a person decide whether to try OrdoStudio, but it must preserve
  agency and avoid manipulation.
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

1. #180 Phase 0: Align product onboarding surface canon and GitHub
   manufacturing setup.
2. #181 Phase 1: Add QR/event landing surface contract and route.
3. #182 Phase 2: Implement OrdoStudio trial offer page and acceptance flow.
4. #183 Phase 3: Wire client-safe relationship conversation onboarding.
5. #184 Phase 4: Add review-return surface for simulated review-request links.
6. #185 Phase 5: Add affiliate/referral landing and attribution surface.
7. #186 Phase 6: Add staff/admin onboarding review cockpit slice.
8. #187 Phase 7: Add frontend/e2e smoke coverage tied to live journey eval
   evidence.

## Acceptance Evidence

Each implementation issue should use the completed eval foundation as evidence,
not as a substitute for product validation:

- deterministic backend route/domain tests for changed contracts;
- UI route tests or Playwright smoke coverage for changed surfaces;
- redaction and trust-boundary assertions for client/public output;
- 0.1.5 journey eval compatibility where the surface maps to QR-to-trial,
  review-return, affiliate-referral, admin/staff, or report evidence;
- screenshots or browser evidence for meaningful frontend changes;
- `npm run export` and `git diff --check`;
- the broader Rust/frontend validation matrix required by the files changed.

## Non-Goals

- Do not implement hosted identity, OAuth, payments, billing, or account
  provisioning in 0.1.6 unless a later issue proves it is the smallest safe
  slice.
- Do not add real outbound email delivery in this arc.
- Do not make live provider calls in default CI or route tests.
- Do not build a generic CRM dashboard.
- Do not expose admin/system internals to ordinary staff or clients.
- Do not automatically file GitHub issues from journey reports.
