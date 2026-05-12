# Eval System

Status: canonical public map for Ordo evals

Ordo uses evals as product pressure tests. The eval system is designed to make
hidden architecture, policy, privacy, handoff, provider, and product problems
visible through durable artifacts rather than screenshots or subjective model
impressions.

Default evals are deterministic, provider-free, network-free, and CI-safe. Live
provider calls are manual, guarded, and budgeted.

## Implemented Pieces

| Piece | Primary source | Purpose |
| --- | --- | --- |
| Deterministic harness | `crates/ordo-daemon/src/eval_harness.rs` | Defines eval cases, steps, assertions, evidence channels, artifact paths, and deterministic clocks. |
| Artifact review | `crates/ordo-daemon/src/eval_artifact_review.rs` | Classifies redacted artifact findings by subsystem. |
| Simulator contracts | `crates/ordo-daemon/src/eval_simulators.rs` | Validates customer, operator, and reviewer simulator outputs as pressure signals. |
| Persona library | `docs/evals/personas/`, `crates/ordo-daemon/src/eval_personas.rs` | Provides ten synthetic personas with validated front matter and redaction rules. |
| Live runner | `crates/ordo-daemon/src/live_eval_runner.rs` | Runs guarded OpenAI-compatible smoke evals and deterministic product journeys. |
| LLM providers | `crates/ordo-daemon/src/llm_gateway.rs` | Provides deterministic, replay, and OpenAI-compatible non-streaming provider adapters. |
| Journey docs | `docs/architecture/conversation-realtime/live-product-journey-evals.md` | Records the completed 0.1.5 journey eval arc. |
| Real LLM readiness | `docs/architecture/conversation-realtime/real-llm-e2e-evals.md` | Records backend-only, replay, and live-provider eval strategy. |

## Evidence Channels

Eval cases should prefer durable evidence:

- SQLite rows;
- conversation events;
- realtime replay;
- policy decisions;
- prompt slot accounting;
- privacy transforms;
- token ledger rows;
- analysis candidates;
- handoff state;
- artifact records;
- surface brief records;
- feedback/review records;
- product surface records.

## Artifact Packet Shape

Eval artifacts are written under local `.data` paths and are not committed by
default. A complete packet can include:

- manifest;
- scorecard;
- redacted transcript;
- timeline;
- event ledger;
- database ledger;
- prompt-slot ledger;
- privacy ledger;
- token ledger;
- analysis candidates;
- handoff ledger;
- replay check;
- artifact review.

Artifacts must not include provider keys, raw private fixture values, raw
provider prompts, or staff-only data in client-visible excerpts.

## Persona Library

Personas live in [personas](personas/README.md). They are synthetic fixtures,
not real people. The validator rejects unsupported schema values, duplicate
ids, empty required lists, raw emails, phone numbers, API-key-shaped strings,
bearer tokens, and configured private terms.

The committed library covers event leads, privacy-sensitive prospects,
budget-constrained prospects, affiliate/referral behavior, staff handoff
triggers, review consent boundaries, and dissatisfied trial feedback.

## Current Journey Coverage

The 0.1.5 journey arc covers:

1. QR/event entry to visitor session.
2. Relationship conversation creation.
3. Deterministic daemon LLM path with privacy egress and prompt-slot
   accounting.
4. Evidence-backed OrdoStudio 30-day trial recommendation.
5. Public offer acceptance and trial creation.
6. Outcome and attribution evidence.
7. Simulated review-request email/link artifact.
8. Return visit and conversation continuity.
9. Feedback capture and review candidate lifecycle.
10. Affiliate/referral path with scoped grants.
11. Admin/staff handoff, delegation, moderation, and grant revocation.
12. Cross-journey analyzed reports.

## Live Provider Guards

The guarded live OpenAI-compatible smoke runner requires explicit network and
spend intent:

```bash
ORDO_LIVE_LLM_EVALS=1
ORDO_LIVE_LLM_ALLOW_NETWORK=1
ORDO_LIVE_LLM_PROVIDER=openai
ORDO_LIVE_LLM_MODEL=<model>
ORDO_LIVE_LLM_MAX_CASES=1
ORDO_LIVE_LLM_BUDGET_USD=0.01
```

`OPENAI_API_KEY` or `API__OPENAI_API_KEY` supplies the provider secret. Use
`.env.local` for local runs and never print the value.

Run:

```bash
cargo run -p ordo-daemon -- run-live-llm-eval-json --db-path .data/local.db
```

## Finding Categories

Artifact review findings should use subsystem categories such as:

- `schema_gap`;
- `event_gap`;
- `policy_gap`;
- `privacy_gap`;
- `prompt_gap`;
- `handoff_gap`;
- `analysis_gap`;
- `accounting_gap`;
- `ux_contract_gap`;
- `provider_gap`.

The goal is to direct the next coding step to the smallest responsible
subsystem.
