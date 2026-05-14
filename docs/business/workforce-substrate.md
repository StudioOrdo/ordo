# Workforce Substrate

Status: canonical product thesis as of 2026-05-13

Current canon: [Current Product Canon](current-product-canon.md)

Ordo is not a toolbox. Ordo is a governed workforce substrate for
solopreneurs and small operators.

The user should feel like they have a producer, researcher, support operator,
growth analyst, social media manager, instructional designer, and systems
operator working through one trusted appliance. The product does not expose a
pile of tools and ask the user to wire them together. Ordo accepts intent,
plans governed work, asks for human judgment when needed, produces artifacts,
and records what happened.

The product stance is:

```text
Ruthlessly simple for the user.
Rigorously disciplined in execution.
```

The discipline comes from adapting mature SaaS and enterprise systems into an
appliance: CQRS read models, events, audits, scoped grants, workflow/DAG
execution, retries, leases, approval gates, provenance, observability,
analytics ledgers, and extension contracts. Ordo should keep the operating
rigor while rejecting enterprise UX complexity.

## Not Tools, Workforces

Ordo should not sell "a video editing system." It should provide the video
editor. It should not sell "a blog." It should provide the social media
manager. It should not sell "instructional design software." It should provide
a personal education channel that can research, draft, review, revise,
publish, measure, and improve.

This does not mean Ordo must build every specialty itself. Ordo Core provides
the stable ground. Community and commercial packs can add domain workforces on
top of the same kernel.

## Pack Shape

A product pack is a packaged workforce. It may include:

- capability bindings;
- content or corpus scopes;
- prompt templates and variable schemas;
- reusable job plans and DAGs;
- request templates for feedback, consent, QA, and approval;
- artifact contracts;
- visibility and publication rules;
- usage limits and reset policy;
- growth metrics and attribution rules.

MCP tools, native tools, WASM workers, browser jobs, and external services are
execution options. They are not the product model. Every pack must register
through Ordo's capability, policy, artifact, request, visibility, and audit
spines.

## Conversational Studio

Studio is a conversational production surface. The operator talks to Ordo,
reviews results, gives feedback, approves, rejects, or redirects. The default
interaction should not require forms.

Forms may exist as optional structured views for inspection, accessibility,
bulk editing, or power-user repair. They must not become the required happy
path for defining work.

The universal production loop is:

```text
natural language intent
-> governed plan
-> visible state
-> artifact candidate
-> review or request
-> revision or approval
-> publication, handoff, or storage
-> measurement and learning
```

## Text-First Interface

Every important Ordo state must be explainable in text:

- what Ordo is doing;
- what it needs from the user;
- what artifact was produced;
- what evidence supports it;
- what limitations remain;
- what action is recommended.

This is an accessibility rule and a future interface rule. Voice models, phone
calls, SMS/Twilio, screen readers, and A2A handoffs should all be projections
of the same text-first request, job, artifact, brief, and approval spine.

## Kernel Boundary

The kernel should stay small:

- Actor
- Connection
- Tracked Entry Point
- Offer
- Access
- Request
- Capability
- Plan / Compiled Plan
- Job / Task
- Artifact
- Event
- Outcome
- Read Model

Complexity belongs in governed packs and execution adapters, not in new
unrelated product primitives.

## What To Avoid

Avoid building:

- a form-first workflow builder;
- a generic plugin marketplace;
- a dashboard that asks the user to interpret raw operations;
- one-off media, blog, course, or support products outside the common kernel;
- custom tools that bypass capability policy, artifact contracts, or audit;
- UI claims that are not backed by durable state.

The better path is shared infrastructure with bespoke outcomes.

## Enterprise Patterns, Appliance Shape

The backend should feel like serious infrastructure. The product should feel
like a trusted workforce.

That means:

- canonical truth stays local and inspectable;
- user-facing surfaces read from projections, not raw operational tables;
- every generated artifact can explain where it came from;
- every cross-boundary action records policy and evidence;
- every pack installs governed work, not a hidden integration island;
- every automation can be replayed, audited, diagnosed, and improved.

Enterprise systems built these patterns for scale and accountability. Ordo
adapts them for sovereignty, accessibility, and solo-operator leverage.
