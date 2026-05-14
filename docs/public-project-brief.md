# Public Project Brief

Status: public-facing orientation for developers, reviewers, and early
collaborators

Ordo is an AGPL local-first appliance for organizational intelligence.

It is not mainly a chatbot, SaaS dashboard, CRM clone, or tool marketplace. It
is an attempt to make AI-mediated work observable, governable, portable,
reviewable, and useful for independent businesses and independent software
developers.

The core principle is:

```text
Human decides. Assistant operates. Process governs. Evidence decides what can
be claimed.
```

## Why This Exists

AI makes software and business operations faster to attempt. It does not make
them automatically trustworthy.

The scarce resources are now judgment, QA, security, useful distribution,
evidence, and economic alignment. Ordo exists to explore a different pattern:
independent developers building sovereign software appliances that preserve
local truth, create durable artifacts, and help operators make better decisions
without surrendering their business memory to an opaque platform.

The project is open because the problem is larger than one company. The code
matters, but the code is not the whole project. The project is a way to test a
new software manufacturing model: doctrine in docs, visible work in GitHub,
evidence in PRs, validation in tests and evals, and human judgment at the gates.

## What Class Of Software This Is

Ordo is exploring several connected ideas:

- local-first organizational intelligence;
- sovereign AI appliances;
- governed workforce substrates;
- AI-assisted software manufacturing;
- business growth loops based on offers, asks, artifacts, briefs, and feedback;
- portable systems that can be hosted for convenience without becoming
  captivity.

The durable spine is:

```text
Capability Catalog
-> Process Template
-> Job
-> Task DAG
-> Event
-> Artifact
-> Brief
```

Conversation is the operating interface. The system behind conversation owns
the hard parts: policy, state, evidence, jobs, artifacts, access, visibility,
and review.

## Current Reality

The repository has a strong appliance foundation, not the full business product
yet.

Implemented foundations include Rust daemon supervision, SQLite migrations,
job/task/event/artifact/brief primitives, backup and restore preflight,
capability catalog, MCP projection, public read models, entry points, offers,
trials, hosted trial capacity foundations, conversation realtime, LLM gateway,
privacy egress, token accounting, deterministic evals, guarded live evals,
reports, and artifact review.

Not built yet: hosted instance orchestration, Traefik control-plane automation,
transactional email, scheduled Growth rollups, final backup email, full
decommissioning receipts, reward ledgers, benefit grants, broad A2A networking,
Studio Ordo Prime, premium media executors, and production public portals.

## Active MVP

The active product target is Studio Ordo as the hosted appliance control plane
for AGPL Ordo appliances.

The first loop is:

```text
meet Keith
-> scan QR
-> ask Ordo for a trial
-> capacity or waitlist
-> hosted Ordo appliance
-> route assignment
-> under-construction onboarding
-> conversation rollups
-> Growth brief
-> feedback and referrals
-> backup and return invitation
-> decommission only after evidence
```

Studio Ordo should win by support, premium capabilities, network effects,
trust, and convenience, not by making it hard to leave.

## How Developers Can Help

The project currently needs QA more than random feature expansion.

Useful contributions include:

- running the appliance and filing evidence-backed issues;
- reviewing public claims against source and tests;
- improving deterministic evals and smoke coverage;
- testing backup, restore, reports, and chat behavior;
- reviewing security, privacy, egress, and redaction boundaries;
- improving docs where the product direction is unclear;
- helping turn broad ideas into small issue/test-plan pairs;
- eventually building governed capabilities, packs, support services, and
  production tools around the AGPL appliance.

Read [QA And Verification](qa-and-verification.md),
[Agent Development Workflow](process/agent-development-workflow.md), and
[Contributing](../CONTRIBUTING.md) before starting work.