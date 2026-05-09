# Live Product Journey Evals

Status: 0.1.5 planning contract

This arc extends the completed 0.1.4 eval foundation into realistic,
persona-driven product journeys. The goal is to exercise the business loop a
future owner would care about:

```text
event meeting -> QR scan -> visitor session -> relationship conversation ->
evidence-backed OrdoStudio recommendation -> 30-day trial -> review return ->
feedback/review lifecycle -> affiliate/referral loop -> analyzed report
```

Live LLM calls are allowed only behind explicit guards and spend caps. Default
tests remain deterministic, provider-free, network-free, and CI-safe.

## Current-Code Grounding

The current codebase can already support substantial parts of this journey:

- `entry_points.rs` supports tracked entry points, QR payloads, public
  `/public/e/:slug`, and visitor sessions with attribution.
- `conversations.rs` supports canonical relationship conversations,
  participants, messages, handoffs, modes, delegation, replay, and client-safe
  versus staff-only boundaries.
- `llm_gateway.rs`, `privacy_egress.rs`, and `llm_accounting.rs` support
  daemon-mediated provider calls, privacy transforms, prompt slots, final
  assistant messages, and token ledger rows.
- `live_eval_runner.rs` supports one guarded OpenAI-compatible smoke eval with
  `ORDO_LIVE_LLM_EVALS=1`, `ORDO_LIVE_LLM_ALLOW_NETWORK=1`, provider/model/key
  checks, max-case guard, budget guard, and artifact packets.
- `offers.rs` supports public offer acceptance and starts a 30-day trial by
  default.
- `attribution.rs` records offer-acceptance outcomes and proposed attribution
  for offer, visitor session, and entry point evidence. It also supports
  referral records.
- `feedback.rs` supports private feedback capture, review candidate creation,
  consent/approval/publish/feature/retire lifecycle, and public review listing.
- `connections.rs` supports `affiliate` connections and scoped connection
  grants.
- `eval_harness.rs` writes packet, scorecard, and manifest artifacts.
- `eval_artifact_review.rs` classifies redacted packet findings into subsystem
  categories and writes local review artifacts without filing GitHub issues.
- `eval_simulators.rs` validates customer, operator, and reviewer simulator
  outputs as non-authoritative pressure signals.

Known gaps:

- no persona markdown library or parser exists yet;
- the live runner is intentionally one smoke case and rejects multiple cases;
- no orchestrator runs the full QR-to-trial-to-review journey;
- no outbound email adapter exists; review-request email should start as a
  redacted simulated artifact/link;
- no affiliate referral journey eval ties affiliate connection, referral entry
  point, referred visitor, and outcome attribution together;
- no cross-persona analyzed journey report aggregates conversion, review,
  referral, handoff, privacy, persuasion, and token evidence.

## Product Journey Contract

Each live product journey run should begin from a realistic persona profile.
The runner should then produce durable evidence for:

1. tracked QR or referral entry point;
2. visitor session;
3. relationship conversation and participant state;
4. live LLM conversation through the daemon gateway;
5. privacy egress transform and prompt-slot accounting;
6. respectful, evidence-backed signup recommendation;
7. public offer acceptance and 30-day trial;
8. outcome and attribution records;
9. simulated review-request email/link artifact;
10. return visit and relationship conversation continuity;
11. feedback capture and review candidate;
12. consent/approval/publication boundary;
13. optional affiliate/referral path;
14. staff/admin handoff, delegation, and moderation path where triggered;
15. per-persona packet, scorecard, artifact review, and aggregate report.

## Persona Library Contract

Persona profiles should live under `docs/evals/personas/` as markdown files
with YAML front matter. Phase 1 owns the full library and parser, but the
contract should support:

- `id`
- `display_name`
- `person_type`
- `event_context`
- `business_context`
- `personality`
- `goals`
- `objections`
- `budget_sensitivity`
- `urgency`
- `privacy_sensitivity`
- `referral_tendency`
- `review_likelihood`
- `handoff_triggers`
- `unsafe_or_edge_case_behavior`
- `expected_pressure_subsystems`
- `evidence_seed_refs`

Personas are fixtures, not truth. Their messages can create realistic pressure,
but deterministic assertions and durable evidence remain authoritative.

## Ethical Persuasion Boundary

The journey can test whether Ordo helps a person decide whether OrdoStudio is a
good fit. It must not test manipulation.

Required boundaries:

- use `ethical_business_persuasion` only with explicit evidence/source refs;
- preserve agency in client-facing language;
- state uncertainty and limitations plainly;
- avoid fake scarcity, fake urgency, fake reviews, fake metrics, shame, fear,
  confusion, dependency, hidden pressure, and unsupported authority/social
  proof;
- keep staff/admin reasoning inspectable in artifacts without exposing internal
  mechanics to the client surface;
- treat LLM outputs as candidates and evidence artifacts, not truth.

## Live LLM Guard Contract

Live runs must refuse network/provider work unless all required guards are
present:

- `ORDO_LIVE_LLM_EVALS=1`
- `ORDO_LIVE_LLM_ALLOW_NETWORK=1`
- `ORDO_LIVE_LLM_PROVIDER=openai`
- `ORDO_LIVE_LLM_MODEL=<model>`
- `OPENAI_API_KEY` or `API__OPENAI_API_KEY`

The multi-case runner must also enforce:

- `ORDO_LIVE_LLM_MAX_CASES=<n>`;
- `ORDO_LIVE_LLM_BUDGET_USD=<amount>`;
- per-persona and whole-run token/cost summaries;
- no raw provider secrets, private persona data, raw private prompts, or raw
  provider payloads in artifacts, logs, scorecards, or UI-facing protocol.

## Reporting Contract

The final report for a run should include JSON and Markdown forms:

- run id, source commit, provider/model, guard settings, started/completed
  timestamps;
- persona roster and per-persona status;
- QR/entry-point, visitor-session, conversation, offer, trial, outcome,
  referral, feedback, review, and handoff ids;
- conversion/trial/review/referral/handoff summary;
- privacy egress detector counts and redaction summary;
- prompt-slot and ethical persuasion evidence summary;
- token ledger and budget summary;
- artifact review findings grouped by subsystem;
- local follow-up issue drafts only when evidence supports them;
- explicit gaps that were not exercised.

The report is for developer/owner review. It should not automatically file
GitHub issues until a governed filing path is implemented and accepted.

## 0.1.5 Delivery Order

1. #162 Align live product journey eval canon and GitHub manufacturing setup.
2. #163 Add persona markdown library and parser/validator.
3. #164 Add multi-case live journey runner foundation.
4. #165 Implement QR event to 30-day trial journey.
5. #166 Implement review-request return journey with simulated email/link
   artifact.
6. #167 Implement affiliate referral journey eval.
7. #168 Implement admin/staff handoff and moderation journey evals.
8. #169 Add cross-persona analyzed journey report.
9. #170 Decide whether simulated review-request email remains enough or a
   governed outbound email adapter should become follow-on work.

Real outbound email, broad provider comparison, and UI-heavy browser journeys
remain future work unless a later issue proves they are the smallest useful
next slice.
