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
- `live_eval_runner.rs` also supports a multi-case live journey planning
  foundation that loads validated personas, applies guard/case/budget caps,
  and writes a redacted manifest without executing the QR-to-trial journey.
- `offers.rs` supports public offer acceptance and starts a 30-day trial by
  default.
- `entry_points.rs` can target explicit public offers, so event QR links can
  resolve to a concrete public 30-day trial offer before the broader public
  surface read model is fully built out.
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

Implemented journey execution:

- #165 adds a deterministic QR-to-trial journey case in
  `live_eval_runner.rs`. It loads a validated persona, creates a public
  OrdoStudio 30-day trial offer, creates a tracked event QR entry point and
  visitor session, opens a visitor-session-backed relationship conversation,
  runs the deterministic daemon LLM gateway path with privacy egress and prompt
  slot accounting, accepts the offer, starts the trial, records the business
  outcome, records offer/session/entry-point attribution, and writes packet,
  scorecard, harness manifest, and journey manifest artifacts.
- The default #165 path remains provider-free and network-free. Live provider
  execution stays behind the existing guard contract and later journey phases.
- The implemented journey asserts no fake urgency, fake scarcity, unsupported
  social proof, raw provider secrets, raw persona narrative, emails, phone
  numbers, configured private terms, or staff internals in artifacts.

Known gaps:

- the smoke eval remains one provider call, while multi-persona live journey
  planning and QR-to-trial execution remain deterministic by default;
- no orchestrator runs the full review-return journey;
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

Status: implemented by #163.

Persona profiles live under `docs/evals/personas/` as synthetic markdown files
with constrained YAML front matter and a narrative body. The backend validator
in `crates/ordo-daemon/src/eval_personas.rs` parses the committed library,
validates required fields, rejects duplicate ids, rejects unsupported pressure
subsystems, and blocks raw emails, phone numbers, API-key-shaped strings,
bearer tokens, and configured private terms.

The implemented schema is `ordo.live_eval_persona.v1` and supports:

- `persona_id`;
- `display_name`;
- `person_type`;
- `event_context`;
- `business_context`;
- `personality_traits`;
- `communication_style`;
- `goals`;
- `objections`;
- `budget_sensitivity`;
- `urgency_level`;
- `privacy_sensitivity`;
- `referral_tendency`;
- `review_likelihood`;
- `handoff_likelihood`;
- `unsafe_or_edge_case_behaviors`;
- `offer_interest`;
- `trial_success_criteria`;
- `expected_eval_pressure_subsystems`;
- `ethical_persuasion_allowed_principles`;
- `redaction_notes`.

The committed library includes ten personas covering a solo consultant, local
service business owner, creative freelancer, agency operator, nonprofit
organizer, skeptical technical founder, privacy-sensitive professional,
budget-constrained early adopter, affiliate/referrer, and dissatisfied trial
user. Personas are fixtures, not truth. Their messages can create realistic
pressure, but deterministic assertions and durable evidence remain
authoritative.

## Multi-Case Runner Foundation

Status: implemented by #164.

The multi-case live journey foundation plans persona-backed cases without
running the product journey. It reuses the persona library and live guard
contract, then writes a JSON manifest with:

- source commit;
- guard status and reason;
- provider/model ids when guards allow planning;
- persona library count;
- selected persona ids;
- max-case and budget cap summary;
- estimated per-case and total cost;
- planned case id per selected persona;
- persona content hashes;
- planned/skipped/blocked case status;
- redaction detector metadata.

Default behavior remains network-free. Missing live/network guards still load
and validate personas, then write a skipped manifest. A budget overrun blocks
before any provider or journey execution. Unknown persona ids are rejected as
configuration errors. QR scan, visitor session, deterministic conversation,
offer acceptance, trial, and attribution execution are implemented by #165.
Review-return, affiliate, handoff, and cross-run report workflows remain in
#166-#169.

## QR-To-Trial Journey Eval

Status: implemented by #165.

The first executable journey case is
`live_journey_qr_to_trial_<persona_id>`. It uses the committed persona library
and a deterministic local provider path so default tests can run without
provider keys or network access.

The case records:

- public OrdoStudio 30-day trial offer;
- tracked `event_qr` entry point with QR payload;
- visitor session created from that entry point;
- canonical relationship conversation for the visitor session;
- anonymous visitor and Ordo agent participants;
- persona-backed visitor message;
- deterministic daemon LLM completion with `ethical_business_persuasion` and
  offer-context prompt slots;
- privacy egress transform metadata;
- prompt-slot accounting and token ledger rows;
- public offer acceptance;
- started 30-day trial;
- business outcome;
- proposed attribution for offer, visitor session, and entry point;
- redacted packet, scorecard, harness manifest, and QR-to-trial journey
  manifest.

The journey manifest is schema `ordo.qr_to_trial_journey_eval.v1`. It stores
durable ids and evidence refs only, not raw persona narrative or private
payloads.

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
4. #165 Implement QR event to 30-day trial journey. Implemented with a
   deterministic persona-backed event QR, visitor session, conversation, LLM
   gateway, offer acceptance, trial, outcome, and attribution eval.
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
