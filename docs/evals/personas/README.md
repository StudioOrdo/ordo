# Eval Persona Library

Status: Implemented for 0.1.5 Phase 1

This folder holds the committed synthetic persona fixtures used by live product
journey evals. Personas are markdown files with constrained YAML front matter
plus a short narrative body. They are eval fixtures, not facts or claims about
real people.

The backend validator is `crates/ordo-daemon/src/eval_personas.rs`. It parses
all `*.md` files in this folder except `README.md`, validates them
deterministically, and returns the personas sorted by `persona_id` for later
multi-case live runner planning.

## Implemented Schema

Each persona uses schema version `ordo.live_eval_persona.v1`.

Required scalar fields:

- `schema_version`
- `persona_id`
- `display_name`
- `person_type`
- `event_context`
- `business_context`
- `communication_style`
- `budget_sensitivity`
- `urgency_level`
- `privacy_sensitivity`
- `referral_tendency`
- `review_likelihood`
- `handoff_likelihood`
- `offer_interest`

Required list fields:

- `personality_traits`
- `goals`
- `objections`
- `unsafe_or_edge_case_behaviors`
- `trial_success_criteria`
- `expected_eval_pressure_subsystems`
- `ethical_persuasion_allowed_principles`
- `redaction_notes`

Allowed `person_type` values:

- `solo_consultant`
- `local_service_business_owner`
- `creative_freelancer`
- `agency_operator`
- `nonprofit_community_organizer`
- `skeptical_technical_founder`
- `privacy_sensitive_professional`
- `budget_constrained_early_adopter`
- `affiliate_referrer`
- `dissatisfied_trial_user`

Allowed level values for budget, urgency, privacy, referral, and handoff fields:

- `low`
- `medium`
- `high`

Allowed `review_likelihood` values:

- `unlikely`
- `low_until_value_is_clear`
- `medium`
- `high`

Allowed pressure subsystems match the simulator contract:

- `privacy`
- `policy`
- `handoff`
- `delegation`
- `feedback_review`
- `home_about`
- `offer_ask`
- `accounting_budget`
- `provider`
- `artifact_review`
- `simulator_fixture`

Allowed ethical persuasion principles:

- `reciprocity`
- `commitment_consistency`
- `social_proof`
- `authority`
- `liking`
- `scarcity`
- `unity`

## Validation Rules

The validator rejects:

- missing required fields;
- unsupported schema versions;
- unsupported person types, levels, review likelihoods, pressure subsystems, or
  persuasion principles;
- duplicate `persona_id` values;
- empty goals, objections, safety behaviors, trial success criteria, pressure
  subsystem lists, persuasion principle lists, or redaction notes;
- raw emails, phone numbers, API-key-shaped strings, bearer tokens, and
  configured private terms in front matter or narrative body.

The committed library currently includes ten synthetic personas covering event
QR leads, privacy-sensitive prospects, budget pressure, affiliate/referral
behavior, staff handoff triggers, review consent boundaries, and dissatisfied
trial feedback.
