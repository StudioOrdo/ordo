# Ordo / Ordo Executor Implementation State Audit

Date: 2026-05-16
Audited repositories:

- `/Users/kwilliams/Projects/studioOrdo` (`StudioOrdo/ordo`)
- `/Users/kwilliams/Projects/ordo_executor` (`kaw393939/ordo_executor`)

This report treats code and schema as current truth, docs as intended direction, and the owner letter as the reconciliation target.

## 1. Executive Summary

- Ordo is not just a chatbot scaffold. The current `ordo` repo has a substantial local appliance kernel: Rust daemon, SQLite schema, protected route contracts, jobs/tasks/events/artifacts, conversations, capabilities, offers/trials, support handoffs, product pack manifests, workflow templates, graph candidates/promotions, generated-content memory candidates, growth reports, and multiple UI surfaces.
- SQLite is currently the source of truth. Graph, jobs, events, offers, conversations, packs, memory candidates, reports/briefs, and policy/audit records are all SQLite-backed in the daemon.
- The graph/memory trust boundary is one of the strongest implemented areas. Candidate graph and generated-content memory flows require evidence/provenance, promotion is explicit, events are emitted, confirmed graph traversal is visibility-filtered, and tests enforce that generated artifacts create candidates rather than durable truth.
- The pack kernel is partially real. Product pack manifests can declare capabilities, job/workflow/request/artifact/graph/projection/LLM-method bindings, and install/disable events are durable. Full pack state semantics such as mounted, active, required, quarantined, deprecated, superseded, scoped policy, assurance thresholds, and conflict routing are not complete.
- The workflow template kernel is partially real and directly relevant to NYC. Story homepage workflow templates, compilation, idempotency, blocked/missing readiness, and approval-mode tests exist, but open issues #413 and #415 confirm the Story Intake -> compilation evidence -> Studio Preview state chain is not yet complete enough for the demo.
- The product surface model exists in the UI and daemon, but it is uneven. Member, Support, Studio, Knowledge, Growth, System, Offers, Capabilities, Requests, Handoffs, Reports, Artifacts, Events, Jobs, and Packs all have code. Several routes still depend on mock or degraded client read models.
- There is not yet one coherent Request / WorkItem / DecisionQueueItem model. `product_request_spine` and `surface_work_items` are promising projections, but support handoffs, feedback asks, artifact reviews, memory decisions, workflow approvals, system issues, and executor HITL items remain separate mechanisms.
- Support has real handoff inbox/eligibility/claim-related foundations, but the exact target model "any member with `support.accept_handoff` can claim open handoffs first-come-first-served" is not cleanly represented yet. Existing capabilities are closer to `conversation.handoff.manage`, `handoff.inbox.write`, role grants, and staff/operator surfaces.
- Growth is real as an owner pilot report and brief surface, but it is not yet the full PDB/SUMO report model. There is no durable `ReportRequest -> ReportPlan -> DataSnapshot -> AnalyticMethod -> MethodRun -> ReportArtifact -> ReportReview -> GeneratedRequest` chain.
- Ordo Executor is far beyond the old 124-test snapshot: current local tests pass at 299. However, that repo is very dirty, with many important executor, HITL, pack policy, and local hybrid index files uncommitted. Treat executor findings as "local checkout state", not clean upstream truth.
- Executor should remain separate for now. It is best shaped as an evidence-processing engine and knowledge-pack foundry exporting a governed run/pack contract into Ordo, not as an immediate repo merge.
- The minimal Ordo/Executor bridge should be an import/export contract: executor run id, executor version/git rev, input manifest hash, source hashes, artifact refs, evidence refs/spans, candidate records, review decisions, promotion receipts, pack policy, trust-boundary flags, and no raw secrets/model dirs/provider internals.
- Baseline validation is mixed. `ordo` Rust tests pass, typecheck passes, build passes, README daemon smoke passes. `ordo` rustfmt check fails on committed code, and UI smoke fails 4 tests. `ordo_executor` tests pass, but rustfmt check fails on local dirty files.
- The open NYC milestone state is exactly the expected stack: #413/#412, #415/#414, #417/#416, #419/#418, and #272 remain open under `0.1.9 OrdoStudio NYC Pilot Foundations`.
- Recommended NYC sequence: #413, #415, then either #419 before #417 if the public relationship landing is the demo-critical path, or #417 before #419 if following the GitHub #272 batch order. Do not broaden these into executor import, generalized pack assurance, vector DB, or durable memory promotion.

## 2. Repo Status

### StudioOrdo/ordo

- Path: `/Users/kwilliams/Projects/studioOrdo`
- Branch: `main`
- Remote: `origin git@github.com:StudioOrdo/ordo.git`
- Initial `git status --short`: clean.
- Post-audit status before writing this artifact: clean.
- Latest commits:
  - `3913856 Merge pull request #420 from StudioOrdo/codex/docs-knowledge-pack-dag-clarifications`
  - `bbbd873 Docs: clarify DAG and LLM boundaries`
  - `5692d72 Docs: capture Knowledge Pack graph direction`
  - Recent history includes PRs #411, #407, #405, #403, #401, #394, #392, #390.

Commands run:

| Command | Result |
| --- | --- |
| `git status --short` | Clean before audit artifact creation. |
| `git branch --show-current` | `main` |
| `git log --oneline -20` | Latest commit `3913856`; docs/knowledge-pack DAG clarifications are merged. |
| `git remote -v` | `git@github.com:StudioOrdo/ordo.git` |
| `npm install` | Passed; packages up to date. Reported 2 vulnerabilities: 1 moderate, 1 high. |
| `cargo fmt --all -- --check` | Failed. Large committed rustfmt drift across daemon files including `backups`, `capabilities`, `corpus`, `diagnostics`, `eval_harness`, `policy_audit`, `reports`, `server/state`. |
| `cargo test --workspace` | Passed: 572 tests, 0 failed. |
| `npm run typecheck` | Passed: `next typegen && tsc --noEmit`. |
| `npm run build` | Passed. Next loaded `.env.local`; build generated static routes. |
| `npm run smoke:ui` | Failed: 282 passed, 4 failed, 16 skipped, 38 did not run. Failures are stale/changed UI expectations in `tests/ui/product-navigation.spec.ts:71` and `tests/ui/system-shell.spec.ts:269`, on desktop and mobile. |
| README daemon smoke: temp DB `init-db`, `ready-json`, `list-capabilities-json` | Passed. Readiness returned `status: ready`; capability output was 137,053 bytes. |
| `gh issue list --repo StudioOrdo/ordo ...` | Confirmed #412-#419 and #272 open in `0.1.9 OrdoStudio NYC Pilot Foundations`. |

UI smoke failures:

- `tests/ui/product-navigation.spec.ts:71`: Studio rail expected `knowledge,factory-jobs,artifacts,publications,templates`; actual also includes `story-intake` and `story-preview`.
- `tests/ui/system-shell.spec.ts:269`: root page expected heading `/A business appliance/`; current root no longer exposes that heading.

Local environment risks:

- `.env.local` exists and is ignored. It was not read. The Next build loaded it.
- Do not commit `.env.local` or generated Playwright traces.

### Ordo Executor

- Path: `/Users/kwilliams/Projects/ordo_executor`
- Branch: `main`
- Remote: `origin git@github.com:kaw393939/ordo_executor.git`
- Latest commits:
  - `c417ab2 Merge pull request #25 from kaw393939/issue-24-collection-ingest-child-task-orchestrator`
  - `66b1bb9 Issue #24 QA: document deterministic child-task orchestration`
  - `bffe668 Issue #24: add deterministic collection ingest child task planner`

Current uncommitted state:

- Modified: `Cargo.lock`, `Cargo.toml`, `docs/README.md`, `docs/architecture/knowledge-ingestion-architecture.md`, `src/main.rs`.
- Many deleted legacy docs under `docs/`.
- Many untracked docs under `docs/architecture`, `docs/executors`, `docs/reference`, `docs/workflows`.
- Many untracked source modules including `src/ordo_pack_contract.rs`, `src/pack_policy.rs`, `src/promotion_preview.rs`, `src/hitl_*`, `src/local_*`, `src/uap_ingest.rs`, `src/video_frame_sample.rs`, `src/artifact_analyze_model.rs`.
- Secret/model risks: untracked `.env.local`, `.models/`, and generated output files including `assess_output.json`, `stage_output.json`, `fmt_output.txt`, `before_counts.txt`, `after_counts.txt`.

Commands run:

| Command | Result |
| --- | --- |
| `git status --short` | Dirty, with modified, deleted, and many untracked files. |
| `git branch --show-current` | `main` |
| `git log --oneline -20` | Latest commit `c417ab2`. |
| `git remote -v` | `git@github.com:kaw393939/ordo_executor.git` |
| `cargo fmt --all -- --check` | Failed. Large rustfmt drift, especially in untracked local files such as `hitl_decision_record.rs`, `hitl_queue_inspect.rs`, `pack_policy.rs`, `promotion_preview.rs`. |
| `cargo test --workspace` | Passed: 299 tests, 0 failed. |
| README-style CLI smoke: `executor list`, `executor describe command.run`, dry-run persona job | Passed. Executor list includes 52 executor ids in local checkout. |

Executor list observed:

`command.run`, `persona.render`, `source.manifest`, `audiobook.ingest`, `media.probe`, `audio.extract`, `audio.split`, `transcript.import_or_generate`, `transcript.generate_local`, `uap.ingest`, `archive.unpack`, `collection.ingest`, `document.pdf.extract_text`, `document.ocr`, `document.clean_text`, `document.normalize`, `document.render_markdown`, `document.chunk`, `corpus.prepare`, `entity.extract`, `entity.disambiguate`, `vector.index`, `graph.candidates.extract`, `graph.review`, `graph.promote`, `video.frame_sample`, `claim.review`, `claim.promote`, `hitl.queue`, `artifact.index`, `artifact.analyze_model`, `artifact.promote`, `artifact.record`, `artifact.review`, `executor.export_pack`, `executor.local_text_extract`, `executor.stage_local_text_review`, `executor.seed_local_text_hitl`, `executor.prepare_local_text_review_packets`, `executor.prepare_local_text_review_decision_templates`, `executor.stage_local_text_review_decisions`, `executor.plan_local_text_truth_promotion`, `executor.promote_local_text_truth`, `executor.plan_local_hybrid_index`, `executor.build_local_hybrid_index`, `executor.stage_local_hybrid_index_review`, `executor.promote_local_hybrid_index_vectors`, `executor.assess_hybrid_readiness`, `executor.inspect_pack_state`, `executor.inspect_pack_policy`, `executor.write_default_pack_policy`, `executor.inspect_hitl_queue`, `executor.record_hitl_decision`, `executor.resolve_hitl_queue`, `executor.preview_promotion`, `executor.simulate_operator_loop`.

## 3. Implementation Map

| Concept | Docs | Code | DB | UI | Tests | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Member | `docs/system-overview.md` | actor/roles/memberships, local session, conversation gateway, product shell libs | `actors`, `actor_role_assignments`, `memberships`, `resource_grants`, conversations | `app/my/*`, member chat, product shell | UI local-session, role projection, conversation tests | Partial. Personal cockpit exists but still fragmented/mocked in places. |
| Support | system overview, resource policy | `availability.rs`, conversation handoffs, handoff inbox handlers | `handoff_inbox_items`, `handoff_eligibility`, `handoff_events`, conversation handoffs | `app/staff/handoffs`, staff support routes | availability/handoff/conversation route tests | Partial. Real queue exists; generalized support-capable member claim model incomplete. |
| Studio | workflow template doc | `workflow_templates.rs`, `story_*`, `studio_*`, job kernel | jobs/tasks/artifacts/workflow compilations | `app/studio/*`, Story Intake/Preview/Publications | many Story/Studio UI and Rust tests | Partial and demo-critical. |
| Knowledge | graph/knowledge docs | `knowledge_graph.rs`, `generated_content_memory.rs`, corpus modules | graph candidate/confirmed tables, memory candidates, corpus | Studio Knowledge/Publications memory, limited Knowledge surface | strong Rust tests | Strong foundation, incomplete generalized review/conflict queue. |
| Growth | system overview | `growth_report.rs`, rewards/referrals/analytics | `content_analytics_events`, rewards, benefits, feedback/outcomes, briefs | owner Growth report route/view | Rust + UI growth pilot tests | Partial. Pilot PDB exists; SUMO model absent. |
| System | README/system docs/security docs | protected routes, backups, readiness, diagnostics, policy, provider config | diagnostics, backups/jobs, provider/vault/policy | system shell routes | route contract/security/schema tests | Strong local appliance foundation. |
| Offers | overview | offers/trials/offer builder modules | offers, acceptances, trials, capacity/waitlist/slots | owner offer builder, member offers | system-shell offer tests, schema tests | Partial. Access/capability/pack grant reconciliation incomplete. |
| Requests | target letter, partial docs | `product_request_spine.rs`, `surface_work_items.rs`, feedback/handoff/artifact/memory separate flows | `product_request_spine`, work items, source-specific tables | `app/my/requests`, staff requests | request spine/surface work tests | Needs reconciliation into canonical Request. |
| Capabilities | README/system docs | `capabilities/*`, registry, route gates, product pack bindings | capabilities/roles/grants/bindings | member capabilities, owner/system | capability and route-boundary tests | Real, but naming and support capability target need cleanup. |
| Packs | `pack-kernel.md` | `product_packs.rs` | `product_packs`, `product_pack_versions`, `product_pack_bindings` | member packs/system surfaces | product pack tests | Partial. Install/disable/bindings real; lifecycle/policy scoping incomplete. |
| DAGs / Jobs / Tasks | workflow template docs, graph doc boundary | `kernel.rs`, `workflow_templates.rs`, scheduler | jobs/tasks/dependencies/compilations | Studio work/factory jobs | 572 Rust suite includes kernel/workflow tests | Strong job kernel; workflow UI state still incomplete. |
| Artifacts | README/system docs | artifacts, versions, links, reviews, patches | artifact tables | Studio artifacts/publications | artifact/security/story tests | Real. |
| Events | README/system docs | realtime events, conversation events, surface timeline | events/realtime/conversation events | realtime UI | route/realtime/event tests | Real. |
| Reports / Briefs | system overview | `reports`, `growth_report`, `surface_briefs` | reports, support packets, briefs | owner reports/growth/system | report/growth/surface brief tests | Partial. Not full analytic-method report model. |
| Handoffs | support docs/overview | availability/conversations/handoff inbox | handoff tables | staff handoffs, public tracked entry route target | availability/handoff tests | Partial. First-user tracked entry handoff is open #419. |

## 4. Product Surface Map

### Member

- Stored in actor/role/membership/resource grant tables plus conversations, offers, requests, capabilities, referrals, benefits, and member-safe projections.
- Displayed under `app/my/*` and product shell routes.
- Good: member-safe projection and UI tests prevent staff/provider/policy leaks.
- Missing: one canonical cockpit model for requests, approvals, notifications, handoffs, and access. Several pages rely on mock/degraded client state.

### Support

- Real backend foundations: handoff inbox, eligibility, assignment/status, conversation handoffs, role-safe conversations.
- Displayed in staff support routes.
- Answer to target questions:
  - First-come-first-served claimable today: partially. Assignment/status machinery exists, but not as the clean target rule "any member with `support.accept_handoff` can claim".
  - Multiple support-capable members: partially. Actors/roles/grants exist, but queue eligibility is still staff/operator shaped rather than a simple support-capability membership model.
  - Handoff in member area: partially through projections/requests, not canonical Request.
  - Global support view: yes, staff support view exists.
  - Conversation visibility/policy: substantially enforced by route contracts and role-safe projection tests.
  - NYC missing: tracked-entry public landing to first-user handoff (#419) and capability-name alignment.

### Studio

- Real construction surface for story intake, story preview, publications, artifacts, templates, and work.
- Real workflow templates and compilation tables.
- Can show some workflow/publication state today. Open #415 means compiled/blocked/missing input/awaiting approval/ready state is not complete enough in Studio Preview.
- Offers can be constructed partially through owner offer builder, but offer -> capabilities -> packs -> workflows is not yet a single clean model.
- Packs can bind workflows/capabilities, but pack-registered executable workflows remain core-validated manifest bindings, not arbitrary pack code.
- Broken jobs becoming Requests: partially through `surface_work_items` and `product_request_spine`, but not canonical.

### Knowledge

- Real graph candidate/promote/confirmed traversal and generated-content memory candidate flows.
- UI exposure is partial: Studio Publications memory review and Knowledge routes exist, but not a full Knowledge cockpit with active/inactive/required packs, ingestion DAGs, graph conflict review, and promotion queues.
- External vectors are not source of truth. Current graph is SQLite-backed.

### Growth

- Real owner Growth pilot report and evidence-safe view model.
- Current report is read-only and explicit about missing data/limitations.
- Missing SUMO report chain: durable report requests/plans/data snapshots/method runs/reviews/generated follow-up requests.

### System

- Strongest complete surface besides core daemon. Readiness, protected route boundaries, backups/restore, diagnostics, provider/vault/policy, route contract tests, schema migrations, and local appliance safety exist.
- Risks: local auth/session is still scaffold-level, `.env.local` can influence build behavior, rustfmt not clean.

## 5. Offer / Request / Capability / Pack Reconciliation

Current state:

- Offer exists and is durable: offer/trial/capacity/waitlist/slot/acceptance tables and owner/member UI.
- Capability exists and is durable: seeded capabilities, route gates, role/grant bindings, product pack capability bindings.
- Pack exists and is durable: manifest install/version/bindings/disable with events.
- Request exists as projection, not canonical: `product_request_spine.rs:29`, `rebuild_product_request_spine` at `product_request_spine.rs:62`, schema at `migrations.rs:3128`. It projects source-specific rows into one list, but the sources still own behavior.

Recommended canonical model:

- `Request`: public/member-friendly object for ask, approval, review, handoff, repair, or decision.
- `DecisionQueueItem` or `WorkItem`: internal routing record derived from Request, pack policy, capability, visibility, priority, due date, and state.
- `Capability`: determines what a member may do and which work they may receive.
- `Offer`: grants access to capabilities, packs, support, network membership, reports, services, or workflows.
- `Pack`: declares workflows, knowledge, policy, assurance, graph contribution boundaries, and request templates; core validates and executes only through explicit kernel boundaries.

How close the code is:

- About halfway structurally, less than halfway conceptually. The tables and projection pieces are there, but the canonical language and ownership are not.
- Do not start by adding another queue. First reconcile existing support handoffs, feedback requests, memory reviews, artifact approvals, workflow approvals, and system issues into the request spine contract.

## 6. Graph and Memory State

Implemented:

- Candidate graph views live in `crates/ordo-daemon/src/knowledge_graph.rs:23` and `:46`.
- Candidate creation and extraction start at `knowledge_graph.rs:157`.
- Candidate state transition is at `knowledge_graph.rs:389`.
- Node promotion is at `knowledge_graph.rs:445`.
- Edge promotion is at `knowledge_graph.rs:526`.
- Neighborhood traversal is at `knowledge_graph.rs:676`.
- Candidate migration is `add_knowledge_graph_candidate_schema` in `migrations.rs:2226`.
- Confirmed graph promotion table is created near `migrations.rs:3242`.
- Tests include evidence/provenance requirements, candidate lifecycle/listing, promotion idempotency/evidence retention, visibility-filtered traversal, sensitive text exclusion, and deterministic extraction.

Current answers:

- Are candidates always evidence-backed? For graph candidate APIs/tests, yes. Inputs require evidence/provenance.
- Can generated content only create candidates, not truth? For generated-content memory and current graph path, yes by tested contract.
- Is promotion explicit and auditable? Yes for graph candidate promotion: reason required, promotion record inserted, events/provenance retained.
- Can graph records point back to canonical records? Partially. Resource linkage/provenance/content hash/evidence refs exist, but not every canonical domain object has a uniform graph linkage contract.
- Are node/edge kinds validated? Partially. They are structured strings and pack manifests can declare kinds, but a global enforced kind registry is incomplete.
- Are edge source/target kinds validated? Partially. Executor contract tests validate endpoints; Ordo daemon does not yet enforce pack-declared edge compatibility as a generalized policy.
- Are evidence requirements enforced per edge kind? No generalized per-kind policy yet.
- Can packs declare node/edge kinds they may create/propose? Yes in manifest shape (`product_packs.rs:132`, `:145`), but enforcement is partial.
- Is there a review queue for graph conflicts? Not a generalized conflict queue. Candidate lifecycle exists; consensus/conflict routing is not built.

Generated-content memory:

- `generated_content_memory.rs` defines memory kind/state/input/decision/review packet at `:17`, `:43`, `:64`, `:93`, `:160`.
- Ingestion starts at `generated_content_memory.rs:201`.
- Decision recording starts at `generated_content_memory.rs:259`.
- Review packet generation starts at `generated_content_memory.rs:353`.
- Schema is `add_generated_content_memory_candidates` at `migrations.rs:3278`.
- Tests enforce generated artifacts propose candidate memory without confirming graph truth, reject provider/prompt/policy/graph-certainty/sensitive markers, keep member packet redacted/read-only, and keep review packets candidate-only.

Gaps versus `graph-kernel.md`:

- No full pack-scoped graph contribution permission enforcement.
- No per-edge evidence policy.
- No generalized graph conflict review queue.
- No active/inactive pack influence on graph/retrieval decisions.
- Vectors are still mostly executor/local-index side, not Ordo retrieval truth.

## 7. Pack and Assurance State

Pack implementation:

- Manifest structs start at `product_packs.rs:25`.
- Bindings include capabilities, job templates, workflow templates, request templates, artifact contracts, graph node kinds, graph edge kinds, projection surfaces, and LLM method contracts (`product_packs.rs:70-171`).
- Install is `install_product_pack` at `product_packs.rs:290`.
- Disable is `disable_product_pack` at `product_packs.rs:597`.
- Tables are created in `add_product_pack_manifest_spine` at `migrations.rs:2951`.
- Tests cover install/list/read/disable, rejection of undeclared methods, hidden authority/secret-shaped manifests, unknown capabilities/templates, and Story pack member summary.

Pack state coverage:

| Target state | Current status |
| --- | --- |
| Installed | Implemented. |
| Mounted | Not clearly separate from installed/enabled. |
| Active | Partial through `status`/enabled bindings, not scoped policy. |
| Inactive | Partial through disable. |
| Required | Not implemented as enforceable state. |
| Quarantined | Not implemented in Ordo pack model. |
| Deprecated/superseded | Not implemented as lifecycle state. |

Pack scoping:

- Global install/version/bindings exist.
- Offer/member/capability/workflow/report/support-policy/knowledge-promotion scoping is not yet a coherent pack-state system.
- Offers do not yet cleanly grant packs as first-class access bundles.

Assurance/HITL:

- Presentation personas exist in executor (`persona.render`) and Ordo eval/simulator areas. They are not operational reviewer personas with policy authority.
- No production `ReviewContextPacket` or `ConsensusArtifact` kernel exists in Ordo.
- Generated-content memory review packet is a narrow domain-specific review packet, not the generalized assurance packet.
- HITL is represented by several queues/decisions: support handoff, feedback request, artifact patch/review, generated memory decision, executor HITL. It is not one generalized queue.
- Packs cannot yet set thresholds such as auto-allow, draft-only, review-all, deny-all in Ordo. Executor local dirty code has `pack_policy.rs`, but that is not integrated into Ordo.

Generalization changes needed:

- Add core `Request` and `DecisionQueueItem` ownership around existing source-specific rows.
- Add pack assurance policy table/schema and validation.
- Add review context packet schema with evidence refs, source object refs, actor/job origin, visibility, policy checks, and allowed actions.
- Add operational reviewer persona outputs as structured opinions only. Presentation persona text must not affect truth/policy.
- Route decisions through policy; promotion remains explicit.

## 8. Executor State

Current executor implementation:

- The local checkout exposes 52 executor ids and passes 299 tests.
- The core safe envelope is in `src/executor.rs`; it requires status/summary/evidence refs and rejects unsafe output keys/content.
- The pack export contract is local and uncommitted in `src/ordo_pack_contract.rs` with `CONTRACT_SCHEMA_VERSION = "ordo.executor_contract.v0"` at line 14.
- Contract structs include `SourceArtifact` (`:306`), `EvidenceSpan` (`:360`), graph candidates (`:534`, `:552`), `ReviewDecision` (`:571`), and `PromotionReceipt` (`:587`).
- `executor.export_pack` is registered/described around `src/main.rs:2185`, `:2239`, and executes through `export_pack` around `:2324`.
- Graph commands are wired around `src/main.rs:2858`.

What is real:

- Artifact record/review/promotion.
- Source manifest and UAP inventory classification.
- Collection child task planning.
- PDF text extraction and OCR planning.
- Document cleaning/normalization/rendering/chunking.
- ZIP safety.
- Media probe, audio split, video frame sampling.
- Local transcript generation/import scaffolding.
- Corpus records.
- Deterministic sparse vector indexes.
- Entity candidates/disambiguation.
- Graph candidates/review/promotion.
- Claim review/promotion.
- HITL queue/inspect/decision/resolve.
- Local text review/promotion and hybrid index workflows.
- Pack export/inspect/policy/promotion preview in local dirty tree.

Trust boundary checks:

- Tests enforce generated analysis is not raw source by default.
- Export pack is metadata-only for UAP source artifacts and does not read downloaded source bytes.
- Model output can create candidates, not truth.
- Promotion requires reason/review.
- Private prompt/provider internals are not public output.
- Local hybrid index writes generated/review/promoted artifacts, not root truth streams until explicit promotion steps.

Risks:

- The executor repo is not clean. The most important functionality is currently in untracked local files.
- `.env.local` and `.models/` are untracked local secrets/model artifacts.
- `command.run` can execute arbitrary local commands; keep operator-controlled.
- There is no durable Ordo import path yet.
- Docs have been reorganized locally but not committed; deleted/added docs need intentional review.

Recommended relationship:

- Keep Executor separate.
- Executor exports packs/runs.
- Ordo imports executor runs as governed artifacts/candidates/review decisions/promotion receipts.
- Do not merge repos until a stable import contract exists and both repos are clean.

Minimal bridge contract:

- `schemaVersion`
- `executorRunId`
- executor id/version/git rev
- pack id/version/policy hash
- input manifest path/hash
- source artifact refs and source hashes
- extraction artifact refs
- evidence span refs
- candidate records
- review decisions
- promotion receipts
- trust-boundary flags
- visibility/access class
- idempotency key
- limitations
- explicit absence of secrets, model paths, prompt/provider internals, and raw generated analysis as evidence

## 9. Demo Readiness Matrix

Verified GitHub issue state:

| Issue | State | Current implementation state | Risk | Smallest safe slice |
| --- | --- | --- | --- | --- |
| #413 / #412 Wire Story Intake to workflow compilation evidence | Open | Story intake artifacts and workflow compilation exist. Need direct evidence link from founder intake/public story readiness into compilation. | Medium. Touches workflow evidence and demo UI; trust boundary if private intake leaks. | Add protected daemon/read-model path exposing compilation evidence refs derived from approved public derivative only. Tests: Rust compilation/evidence, UI safe display, role refusal. |
| #415 / #414 Add Story workflow state to Studio preview | Open | Story preview UI and workflow compilation exist. State mapping incomplete for compiled/blocked/missing input/awaiting approval/ready. | Medium. Public/member-safe state labels must not imply publication or provider success. | Extend Studio Preview view model to show compilation state from daemon. Tests for all five states and daemon-degraded state. |
| #417 / #416 Memory promotion readiness packet | Open | Generated-content memory candidates, decisions, and review packet exist. No promotion readiness packet for approved generated content. | High trust boundary. Must not mutate canonical memory, graph, or vectors. | Add read-only readiness packet for approved candidates with evidence refs, blockers, required actor/policy steps. Tests prove no graph/memory promotion. |
| #419 / #418 First-user relationship landing handoff from tracked entry | Open | Tracked entries, visitor sessions, public routes, support handoff foundations exist. Need polished public-safe handoff path. | Medium/high public surface. Must avoid staff/internal leakage and spammy duplicate handoffs. | Wire `/public/e/:slug` CTA to create/return idempotent relationship handoff request with safe visitor status and support queue projection. |
| #272 Batch Plan | Open | Latest comments confirm no PR blockers after #420. | Process risk if scope expands. | Keep slice boundaries tight; do not include pack import/export or live providers. |

Recommended execution order:

1. #413: it creates the evidence bridge that #415 needs.
2. #415: makes workflow state visible for Studio demo.
3. #419 if NYC needs the public first-user path in the demo narrative; otherwise follow #272 and do #417 first.
4. #417 if not done third: trust-heavy, read-only readiness packet only.

## 10. Risk Register

| Risk | Severity | Evidence | Mitigation |
| --- | --- | --- | --- |
| `ordo` rustfmt failure | Medium | `cargo fmt --all -- --check` failed on committed code. | Run rustfmt as its own hygiene PR or before next feature branch. |
| `ordo` UI smoke failure | Medium | 4 Playwright failures; likely stale expectations. | Update tests or root/rail behavior intentionally before demo. |
| `ordo` npm vulnerabilities | Medium | `npm install` reported 1 moderate, 1 high. | Run `npm audit` and triage; avoid blind `--force` upgrades. |
| `.env.local` build influence | Medium | Next build loaded `.env.local`. | Keep ignored; document required envs; run CI without local env. |
| Executor dirty tree | High | many modified/deleted/untracked files. | Stabilize into commits before using as integration reference. |
| Executor secrets/model artifacts | High | untracked `.env.local`, `.models/`. | Ensure ignored and never committed; audit generated outputs. |
| Generated-analysis contamination | High | Both systems are designed to avoid this, but bridge is not built. | Bridge contract must distinguish raw source, derived extraction, generated analysis, candidate, review, promotion. |
| Policy bypass through UI placement/prompt text | High | Pack assurance not integrated. | Core policy, not UI/prompt/persona, must authorize capabilities and promotion. |
| Fragmented queues | Medium/high | handoffs, requests, memory, artifacts, executor HITL are separate. | Canonical Request/DecisionQueueItem projection. |
| Required pack skipping | Medium | required/quarantined states absent. | Add pack-state policy before active retrieval/decision influence. |
| External egress | Medium/high | Provider config exists; executor command can run local arbitrary commands. | Keep provider/command execution operator-controlled and policy-checked. |
| Data-loss risk | Medium | migrations pass, backup exists, but new import bridge absent. | Import into staging tables first; no direct promotion on import. |

## 11. Recommended Next Slices

1. Story Intake compilation evidence (#413)
   - Acceptance: submitted/approved Story Intake produces workflow compilation evidence refs; private intake text and provider/policy internals are not exposed; idempotent retries return same compilation.
   - Tests: Rust workflow/intake test, route boundary test, UI view-model test.

2. Story Preview workflow state (#415)
   - Acceptance: Studio Preview displays compiled, blocked, missing input, awaiting approval, ready, and degraded states from daemon evidence.
   - Tests: UI view-model tests for all states; protected route test.

3. Public tracked-entry handoff (#419)
   - Acceptance: `/public/e/:slug` can create/return a safe first-user relationship handoff; duplicate submissions group evidence rather than create duplicate queue rows; staff-only context remains hidden.
   - Tests: Rust handoff/visitor session test, public route/UI test, support queue projection test.

4. Memory promotion readiness packet (#417)
   - Acceptance: approved generated-content memory candidates produce read-only readiness packets with evidence, blockers, actor/job origin, allowed next action; no canonical memory, graph, vector, or pack-state mutation.
   - Tests: generated memory Rust test, route test, Studio Publications UI test.

5. Request spine reconciliation issue
   - Acceptance: define canonical Request fields and map support handoff, feedback ask, artifact review, memory decision, workflow approval, and system issue into one projection without changing source ownership.
   - Tests: product request spine projection tests with role-safe visibility.

6. Capability naming cleanup for support
   - Acceptance: introduce/alias `support.accept_handoff`; support queue eligibility derives from capability grants, not staff labels alone.
   - Tests: multi-member support eligibility and claim race/idempotency tests.

7. Pack state lifecycle issue
   - Acceptance: add explicit states for installed, active, inactive, required, quarantined, deprecated/superseded; enforce that inactive packs do not influence retrieval/decision projections and required packs cannot be silently skipped.
   - Tests: product pack state transition and policy tests.

8. Pack assurance policy issue
   - Acceptance: pack can declare assurance preset and thresholds; core validates shape; no autonomous promotion is added.
   - Tests: policy parse/validation and fail-closed routing tests.

9. Executor contract stabilization issue
   - Acceptance: commit/clean executor contract files; `cargo fmt --check` passes; contract summary schema has golden tests.
   - Tests: existing 299 tests plus schema fixture test.

10. Ordo executor import staging issue
   - Acceptance: Ordo can ingest an executor export into staging artifacts/candidates only; no truth promotion; import verifies hashes and trust-boundary flags.
   - Tests: fixture import, tampered hash rejection, generated-analysis quarantine.

## 12. Open Questions for Owner

1. For the NYC demo, is the public first-user relationship landing path (#419) more important than the memory promotion readiness packet (#417)? The GitHub batch order puts #417 before #419, but the demo narrative may justify swapping them after #415.
2. Should `support.accept_handoff` become the canonical capability id, with existing handoff capabilities aliased/migrated, or should we preserve current capability ids and map them into a product label?
3. Should `Request` become a stored canonical table now, or should the next slice keep it as a projection over existing source-specific tables until the NYC stack lands?
4. Is the executor dirty local state intended to become the next upstream baseline, or should the bridge plan target the last clean committed executor state first?
5. Which pack states must exist before demo: only active/inactive/required, or also quarantined/deprecated/superseded?

