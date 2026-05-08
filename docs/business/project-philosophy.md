# Project Philosophy

Status: Public philosophy and product direction

Ordo is not primarily a codebase or a SaaS business. It is a philosophy of AI
work made concrete as an open appliance.

The core claim is:

> AI work should be observable, governable, and owned by the people doing it.

## Ordo Is Not SaaS

SaaS usually asks people to move their work into a vendor's model. Ordo starts
from a different premise: individuals, small teams, educators, and communities
should be able to run their own AI workbench.

Managed hosting can be useful. It should be convenience, not captivity.

Ordo is designed as an appliance someone can run, inspect, modify, back up, and
leave with.

## An Appliance For Observable AI Work

Ordo should make AI-mediated work inspectable.

For meaningful work, the system should preserve:

- intent;
- evidence;
- capability path;
- policy decision;
- job and task history;
- artifact output;
- limitations;
- human review.

The goal is not opaque automation. The goal is repeatable, auditable,
trustworthy systems of work.

## Browser-Like Runtime For AI Work

The browser made the web usable by giving people a common place for navigation,
sessions, forms, downloads, permissions, history, and extensions.

Ordo is trying to play a similar role for AI work:

- chat as interaction;
- briefs as navigation;
- jobs as execution;
- artifacts as outputs;
- logs and reports as history and QA;
- capabilities as governed extensions;
- policy and future RBAC as permission boundaries;
- packs and future Ordo-to-Ordo exchange as portability.

If the browser made the web usable, Ordo is trying to make AI work governable.

## Governance Belongs In The Workbench

Enterprise AI governance should not live only in slide decks, policies, and
checklists. It should be built into the place where AI work happens.

In Ordo, governance should become operational:

- policy as capability metadata;
- work as jobs;
- decisions as durable events;
- outputs as artifacts;
- diagnostics as logs;
- issue reports as evidence packages;
- provenance as first-class metadata;
- human judgment as explicit authority.

The current codebase already implements the local appliance spine, System shell,
jobs, events, artifacts, logs, reports, capability policy, and provenance
foundations. Full RBAC, RAG, hosted trials, Worker Ordos, A2A, content packs,
and external submissions remain planned or future work.

## AI Velocity Makes Verification The Bottleneck

AI can accelerate research, specification, implementation, and test generation.
That does not make work done.

The limiting factor is intentionally human:

- manual QA;
- code review;
- functional review;
- evidence review;
- public issue closeout;
- judgment about whether the output should exist at all.

Ordo treats velocity without verification as waste.

## Community Before Company

Ordo is happily self-funded and community-first. That posture lets the project
move slowly where judgment matters, stay opinionated about architecture, and
avoid premature SaaS pressure.

The early community is developers, educators, AI builders, and operators who
want to learn how to build serious, trustworthy AI systems of work.

## Teaching, Building, And Learning In Public

Ordo is designed to be useful as a teaching and build-in-public system.

Issues, specs, jobs, logs, reports, artifacts, release evidence, and reviews are
not just project management. They are part of the learning system. They show how
AI-assisted work moves from intent to evidence to implementation to review.

## What Exists Today

The current repo implements the appliance foundation:

- Rust daemon and Next.js System shell;
- SQLite migrations and local durable state;
- process/job/task kernel;
- durable events and artifacts;
- System Briefs;
- backup and restore preflight safety;
- diagnostic logs;
- local issue reports;
- capability catalog and MCP projection;
- early policy/provenance foundations.

This is not the full business product yet.

## What Comes Later

Planned and future layers include:

- public Chat, About, Offers, and Feed surfaces;
- full RBAC and user access boundaries;
- knowledge/RAG with provenance;
- hosted trials;
- content packs;
- Worker Ordos;
- A2A support and service exchange;
- external report submission;
- human handoff and governed voice operations.

Those directions should build on the same principle: make AI work observable,
reviewable, and owned by the people using it.
