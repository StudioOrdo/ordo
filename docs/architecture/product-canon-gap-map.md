# Product Canon Gap Map

Status: planning map as of 2026-05-13

This map compares the current product canon with the docs, backend, and
frontend. Use it to keep implementation slices aligned across all three.

Canonical product stance:

```text
Member View
Studio
Support
Knowledge
Growth
Systems
```

## Summary

The repo already has much of the appliance spine: durable jobs/tasks/events,
artifacts, capability catalog, offers/trials, connections/grants, handoff
inbox, corpus retrieval, answer drafts, feedback/reviews, attribution, and
public read models.

The biggest gap is product assembly. Ordo should become a governed workforce
substrate, but the backend pieces are not yet wired into one product loop:

```text
offer accepted
-> access granted
-> member can use tools/content/templates
-> Ordo compiles a governed DAG
-> requests collect human approval/feedback/consent
-> artifacts and outcomes return to Member View, Studio, Support, Knowledge,
   Growth, and Systems as role-safe projections
```

Architecture stance:

```text
Enterprise operating discipline
+ local appliance ownership
+ agentic execution
+ conversational UX
+ governed community packs
```

This means Ordo should borrow enterprise SaaS patterns where they protect
reliability and trust, then implement them as inspectable appliance machinery:
CQRS-lite, events, audit, policy, scoped grants, durable jobs, retries, leases,
approval gates, artifact provenance, traces, analytics ledgers, and adapter
contracts.

## Process And Architecture Alignment

New development should use these process and architecture contracts before
creating or executing implementation issues:

- [Definition Of Done](../process/definition-of-done.md) for local-only,
  QA-passed, landed, closable, and blocked states.
- [Implementation Issue Template](../process/implementation-issue-template.md)
  for issue shape and acceptance gates.
- [Test Plan Template](../process/test-plan-template.md) for scenario and
  validation coverage.
- [Agent Execution Protocol](../process/agent-execution-protocol.md) for
  Research, Execute, QA, Land, blocker, and no-close behavior.
- [Graph Kernel](graph-kernel.md) for graph-native relationship traversal,
  explanation, and evidence.
- [LLM Method Contracts](llm-method-contracts.md) for product-shaped methods
  designed for useful but unreliable LLMs.
- [Pack Kernel](pack-kernel.md) for internal packs and future developer
  ecosystem boundaries.
- [Workflow Template Kernel](workflow-template-kernel.md) for typed variables,
  bounded fanout, governed tool composition, task bindings, approval gates, and
  deterministic provider mocks.

Issue batches should explicitly call out whether a slice touches graph records,
LLM methods, pack registration, or process completion gates. If it does, the
linked test-plan issue must include coverage for those boundaries.

## Documentation Alignment

Done in this pass:

- Added [Current Product Canon](../business/current-product-canon.md).
- Added [Workforce Substrate](../business/workforce-substrate.md).
- Added [Appliance Operating Discipline](appliance-operating-discipline.md).
- Added [Target Architecture Plan](target-architecture-plan.md).
- Added [Rewards And Incentives](rewards-and-incentives.md).
- Added [OrdoStudio NYC Pilot](../business/ordostudio-nyc-pilot.md).
- Added [Agent-To-Agent Roadmap](agent-to-agent-roadmap.md).
- Retired product guidance was moved out of public docs and kept only in
  ignored local archive material when historical context is explicitly needed.
- Updated [Business Canon](../business/README.md) to point to the current
  surface-first IA.
- Updated [Product Shape](../business/product-shape.md) to use Member View,
  Studio, Support, Knowledge, Growth, and Systems.
- Updated [Product Roadmap](../business/product-roadmap.md) to include
  offer-to-access grants, request/feedback/approval state, Knowledge, Studio
  DAG execution, and Growth value events.
- Updated [Ordo Product IA Contract](ui/ordo-product-ia.md) to treat
  Studio, Knowledge, and Systems as first-class surfaces.
- Updated conversation UX/doctrine docs to point at the current canon instead
  of older rail-first language.

Remaining doc cleanup:

- Older local `_letters` files remain useful historical context but should not
  be treated as canon. They have been retired from active guidance.
- Some implementation docs still use `System` singular because the current
  shipped UI is a System shell. That is acceptable for current-state docs, but
  future product docs should use `Systems` for the canonical surface.
- `Capabilities` should remain an internal/platform term. Member-facing docs
  should prefer `Access`.

## Backend Gap Map

### Strong Foundations Already Present

- Job/task DAG creation, dependency validation, events, and artifacts.
- Capability catalog with schemas, execution targets, side effects, approval
  requirement, scheduler eligibility, and artifact kinds.
- Public surface read models for About, Offers, Asks, and Feed.
- Tracked entry points and visitor sessions.
- Offers, offer acceptance, trial lifecycle, and offer acceptance attribution.
- Connections, connection grants, resource grants, and revocation.
- Availability, presence, handoff inbox, and receipts.
- Conversation gateway, durable conversations, handoffs, modes, and LLM gateway.
- Corpus sources/items, SQLite FTS retrieval, and answer draft scaffolds.
- Feedback, feedback tags, customer reviews, business outcomes, and attribution.
- MCP pack metadata and capability binding.

### Backend Gaps Against Canon

1. **CQRS-Lite Surface Projection**
   - Canonical tables exist and events exist, but product surfaces still lack
     unified projection tables.
   - Need command handlers that mutate canonical truth, append events, and
     update or schedule read-model projection.
   - First target should be `surface_work_items` for Member View, Studio,
     Support, Knowledge, Growth, and Systems.

2. **Offer To Access**
   - Missing first-class entitlement/access-grant model that turns accepted
     offers into usable member access.
   - Existing `resource_grants` and `connection_grants` can supply policy
     mechanics, but the product needs offer-derived access records.

3. **Product / Workforce Packs**
   - MCP packs exist, but product packs are different: packaged workforces that
     bind tools, content scopes, prompts, variables, job templates, request
     templates, limits, approval rules, and growth metrics.
   - Need a durable pack manifest that binds offers to access without allowing
     arbitrary execution.

4. **Compiled Job Plans**
   - Current templates are built-in and job creation validates DAGs.
   - Missing user-copyable compiled plans with variables, schema validation,
     versioning, and safe capability binding.

5. **Generic Task Executor**
   - Current kernel creates jobs/tasks and marks ready tasks.
   - Missing leases, worker assignment, retry execution, cancel/pause/resume,
     task result envelopes, and executor dispatch by capability target.

6. **Requests As Product Objects**
   - Handoffs, feedback, reviews, and tool approvals exist as separate
     foundations.
   - Missing one product-level request/read-model contract for approvals,
     feedback, consent, scheduling, artifact review, QA follow-up, and missing
     information.

7. **Surface Read Models**
   - Public read models exist.
   - Missing canonical read models for Member View, Studio, Support, Knowledge,
     Growth, and Systems that project shared objects into role-safe work items.

8. **Knowledge Packs**
   - Corpus/retrieval exists.
   - Missing signed/importable content-pack manifests, pack lifecycle, pack
     revocation, and offer/access integration.

9. **Growth Value Loop**
   - Attribution and business outcomes exist.
   - Missing Growth surface contracts for value events, content performance,
     offer performance, referral reward state, and business health briefs.

10. **Rewards And Benefit Grants**
    - Feedback, reviews, attribution, and trials exist as separate foundations.
    - Missing reusable reward programs, reward rules, reward events, reward
      ledger entries, benefit grants, balances, review state, reversal state,
      and opt-in leaderboard projections.
    - First policy target should support qualified referral grants of seven
      hosted days and accepted-feedback hosted-time grants.

11. **Hosted Trial Capacity And Reset**
    - Trial records exist.
    - Missing hosted trial capacity, waitlist, expiration/reset scheduling,
      backup-before-wipe state, and extension caps.
    - First target should support the OrdoStudio NYC pilot: 10 active hosted
      spots, 30-day trial window, backup/export before wipe, and reward-based
      extensions.

12. **Media/Studio Execution**
   - Artifacts and jobs exist.
   - Missing Studio-specific media capabilities, render DAGs, browser/WASM
     candidate validation, MetaVisKit/AVFoundation/native executor envelopes,
     and media QC artifact contracts.

13. **Text-First Explanation Spine**
    - Briefs, events, and artifacts exist.
    - Missing one contract that guarantees every job state, request, artifact,
      QR path, approval, and failure can be explained in text for accessibility,
      future voice/phone/SMS, and agent-to-agent handoff.

14. **Workflow Template Kernel**
    - Job/task DAG foundations exist.
    - Missing versioned workflow templates with typed inputs, workflow
      variables, variable bindings, fanout groups, approval gates, visibility
      classes, provider requirements, idempotency/retry policy, deterministic
      mocks, and projection expectations.
    - This is future/needed, not shipped behavior.

15. **Generic Provider / Tool Capability Kernel**
    - Capability catalog and pack binding foundations exist.
    - Missing a reusable provider/tool kernel for image generation, image
      review, TTS, transcription, search, QR generation, page render,
      screenshot QA, and public derivative preparation.
    - Tool capability must remain generic machinery while product-shaped
      methods and workflows own authority.
    - This is future/needed, not shipped behavior.

16. **Generated Content Memory Ingestion**
    - Artifacts, graph candidates, claims, public surfaces, feedback, and
      outcomes exist as separate foundations.
    - Missing a governed path from generated artifacts to extracted candidate
      claims/preferences, owner approval/rejection, publication evidence,
      outcome evidence, and graph memory promotion.
    - Generated content must remain evidence, not automatic truth.
    - This is future/needed, not shipped behavior.

17. **Content Analytics Spine**
    - Growth already has attribution and outcome foundations.
    - Missing event-first content analytics for published artifact/version,
      claim set, generated media version, section impression or scroll
      milestone, CTA click, QR/referral source, request, trial, feedback,
      referral, reward, and downstream outcome.
    - Analytics must be privacy-aware, local-first, outcome-linked, and honest
      about limitations.
    - This is future/needed, not shipped behavior.

18. **Image Generation / Review Artifact Contracts**
    - Artifact foundations exist.
    - Missing first-class contracts for image briefs, generated image variants,
      revised prompts, provider metadata, local file checksums, public
      derivatives, reviewer feedback, alt text, palette, visibility, approval,
      and safe publication state.
    - This is future/needed, not shipped behavior.

19. **Story Pack Workflow Declarations**
    - Pack manifest foundations exist.
    - Missing Story Pack workflow declarations for founder intake, narrative
      deck, image briefs, generated image variants, reviewer feedback,
      scrollytelling draft, QA review, manual/scheduled publish, analytics
      feedback, and memory candidate updates.
    - This is future/needed, not shipped behavior.

## Frontend Gap Map

### Strong Foundations Already Present

- Product shell and placeholder surfaces exist.
- Member rooms exist in fixture form: Ordo, Activity, Offers, Requests, and
  Capabilities.
- Conversation and chat product docs define local echo, replay, state handling,
  streaming, and role-safe projections.
- Browser capability runtime exists for candidate work.
- Public routes exist for Home/About/Offers/Asks/Latest-style projections.

### Frontend Gaps Against Canon

1. **Canonical Surface Navigation**
   - Current code still uses transitional app spaces: `site`, `my-ordo`,
     `staff`, `studio`, `owner`, `admin`.
   - Canon requires Member View, Studio, Support, Knowledge, Growth, Systems.

2. **Access Naming**
   - Current member UI uses `Capabilities`.
   - Canon says member-facing UI should use `Access`; internal code can keep
     capability/grant terminology.

3. **Member Chat Runtime**
   - The member chat surface is in flux in the current worktree.
   - `components/member-chat-gateway.tsx` is deleted while
     `components/member-ordo-surface.tsx` still imports it. This must be
     resolved before the member loop is real.

4. **Studio**
   - Studio pages are mostly placeholders.
   - Missing conversational production runs, job template authoring, DAG
     execution views, variables, artifact review, media production workflow,
     publication prep, and progress/event projection.

5. **Knowledge**
   - Knowledge is currently folded into Studio navigation in places.
   - Canon needs Knowledge as its own surface for corpus, sources, packs,
     provenance, and retrieval readiness.

6. **Growth**
   - Growth does not exist as a first-class surface in the current product
     navigation.
   - Owner/Business pages partially cover this, but the canon needs Growth
     for offers, asks, QR paths, attribution, referrals, content performance,
     value events, rewards, benefit grants, leaderboards, and business health.

7. **Support**
   - Support placeholders exist.
   - Missing durable read-model wiring for handoff queues, customer requests,
     feedback/review triage, QA, and staff-safe conversation briefs.

8. **Role-Safe Projection Layer**
   - Some projection foundations exist.
   - Missing end-to-end projection contracts for each canonical surface and
     room, backed by daemon read models rather than mock data.

9. **Text-First Accessibility**
   - Current UI components often have accessible labels, but product state is
     not yet uniformly explainable as text.
   - Missing text summaries for production runs, QR/entry contexts, artifact
     state, approval requests, and failure recovery.

## Recommended Alignment Slices

1. **Docs Canon Cleanup**
   - Treat `current-product-canon.md` as the product IA source of truth.
   - Treat `target-architecture-plan.md` as the implementation architecture
     source of truth for Clean/CQRS-lite layering and slice order.
   - Mark old `_letters` files as historical references if they create
     confusion.

2. **Frontend IA Rename Slice**
   - Update navigation types and labels toward Member View, Studio, Support,
     Knowledge, Growth, Systems.
   - Rename member `Capabilities` room to `Access`.
   - Keep route aliases where needed to avoid breaking current links.

3. **Member Chat Repair Slice**
   - Restore or replace the member chat gateway component.
   - Ensure the Member View Ordo room can use `/chat/ws` and deterministic LLM
     mode without fixture-only behavior.

4. **Offer-To-Access Backend Slice**
   - Add access/entitlement records created from offer acceptance.
   - Bind access to content scopes, capability ids, job template ids, request
     templates, limits, and expiration/reset policy.

5. **CQRS-Lite Projection Slice**
   - Add the first shared `surface_work_items` projection.
   - Project offers, access, requests, jobs, artifacts, outcomes, and support
     items into surface-safe rows.
   - Keep canonical truth in existing domain tables; projections are rebuilt
     when product meaning changes.

6. **Product / Workforce Pack Manifest Slice**
   - Define product packs separately from MCP packs.
   - Pack manifests should reference registered capabilities and templates only.
   - Include prompt variables, schemas, request templates, artifact contracts,
     policy, limits, and growth measurement.

7. **Request Spine Slice**
   - Add a product-level request contract and read model covering approvals,
     feedback, consent, artifact review, QA, scheduling, and missing info.

8. **Job Kernel V2 Slice**
   - Add compiled plans, variables, leases, worker assignment, retry/cancel, and
     capability-target executor dispatch.
   - Require structured task result envelopes.

9. **Rewards And Benefit Grants Slice**
   - Add reusable reward programs, reward rules, reward events, reward ledger
     entries, benefit grants, balances, and qualification review.
   - Support qualified referral -> seven hosted days and accepted feedback ->
     policy-defined hosted days for the pilot.
   - Keep rewards in Growth and benefit enforcement in Access.

10. **Hosted Trial Capacity And Reset Slice**
   - Add capacity, waitlist, expiration, backup-before-wipe, reset/wipe, and
     extension-cap state for hosted trials.
   - Ensure trial duration changes cite reward or owner decision evidence.

11. **Surface Read Model Slice**
   - Add daemon read models for Member View, Studio, Support, Knowledge,
     Growth, and Systems work items.

12. **Text Explanation Contract Slice**
   - Add a shared text explanation envelope for jobs, tasks, requests,
     artifacts, failures, QR paths, and approvals.
   - Use it as the bridge for accessibility, future voice/phone/SMS, and
     agent-to-agent handoffs.

13. **Workflow Template Kernel Slice**
   - Add versioned workflow template records, typed inputs, variables, bindings,
     fanout expansion, approval gates, and deterministic fixture behavior.
   - Keep templates below product authority: canonical tables own truth,
     events own audit/replay, graph owns relationships, and projections own
     surface experience.

14. **Creative Tool Capability Slice**
   - Add reusable image-generation and image-review capability contracts with
     deterministic mocks.
   - Ensure generated outputs become artifacts with provider metadata,
     visibility, checksums, evidence refs, approval state, and no public leak of
     raw prompts or provider internals.

15. **Generated Content Memory Slice**
   - Extract claims and preferences from generated/published content into graph
     candidates.
   - Require owner approval, publication evidence, feedback, or outcome
     evidence before stronger memory promotion.

16. **Content Analytics Slice**
   - Add event-first content analytics for scrollytelling/public artifacts,
     tracked entry points, section/CTA behavior, requests, trials, feedback,
     referrals, rewards, and outcomes.
   - Keep LLM interpretation separate from analytics truth.

17. **Story Pack Workflow Slice**
   - Declare the Story Pack workflow from founder intake through narrative
     deck, image briefs, generated image variants, review, scrollytelling draft,
     QA, publish approval, analytics, and memory candidates.
