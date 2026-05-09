# Eval Persona Library

Status: Planned for 0.1.5 Phase 1

This folder will hold markdown personas used by live product journey evals. The
Phase 0 planning pass records the intended contract only; it does not add the
full persona set or parser.

Persona files should use YAML front matter plus a short markdown narrative.

Required front matter shape:

```yaml
id: event_founder_budget_sensitive
display_name: "Budget-Sensitive Founder"
person_type: "small_business_owner"
event_context: "Met owner at a local business event and scanned Studio Ordo QR."
business_context: "Runs a small service business and handles follow-up manually."
personality:
  traits: ["skeptical", "direct", "busy"]
  communication_style: "short mobile messages"
goals:
  - "understand whether OrdoStudio can reduce follow-up work"
objections:
  - "does not want another dashboard"
budget_sensitivity: "high"
urgency: "medium"
privacy_sensitivity: "high"
referral_tendency: "medium"
review_likelihood: "low_until_value_is_clear"
handoff_triggers:
  - "asks for a human before sharing business details"
unsafe_or_edge_case_behavior:
  - "may paste private client details"
expected_pressure_subsystems:
  - "privacy"
  - "offer_ask"
  - "handoff"
evidence_seed_refs:
  - "event_qr_scan"
```

Rules:

- Personas are test fixtures, not facts.
- Persona content must not include real private data.
- Live simulator outputs must be redacted and validated by
  `ordo.eval_simulator_output.v1`.
- Deterministic assertions and durable evidence remain the source of truth.
