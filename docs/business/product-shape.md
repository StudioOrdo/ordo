# Product Shape

Status: Product direction, not fully implemented

Ordo is a local-first operating system for one-person businesses. The current
repo proves the appliance spine. The full product shape is broader.

The product should feel like a browser-like runtime for AI work: one place to
interact, inspect, approve, remember, and move artifacts across governed
capabilities.

## Implemented Today

The current product surface is the System appliance shell:

- Brief
- Health
- Backup And Restore
- Schedules
- Preferences
- Events
- Logs
- Reports

These surfaces help the operator inspect the appliance, understand what
happened, prepare local diagnostic reports, and verify that work is grounded in
evidence.

## Planned Product Surfaces

The main business product should organize around four owner/public surfaces.
These are planned surfaces, not implemented in the current repo.

### Chat

Chat is the primary operating interface. The owner should be able to ask Ordo to
inspect, plan, create, revise, schedule, and explain work from one place.

Chat should not bypass governance. It should route intent through capabilities,
jobs, evidence, and approvals.

### About

About is the public business story. It should explain who the business serves,
what it believes, what evidence supports its claims, and where visitors should
go next.

This should become a scrollytelling business narrative, not a generic page
builder.

### Offers

Offers describe what can be bought or joined.

An offer should be durable business data, not only marketing copy. It should
connect to audience, fulfillment, evidence, follow-up, and measurement.

### Feed

Feed is the public stream of composite artifacts: articles, podcast outputs,
images, clips, briefs, updates, and syndicated machine-readable content.

The feed should support short-form content marketing while preserving provenance
and source grounding.

## Owner Operating Room

Briefs are how Ordo explains what matters. The owner should not have to inspect
raw tables, logs, or workflow internals first.

The product should bring back:

- the current state;
- what changed;
- what needs attention;
- evidence;
- limitations;
- recommended next action.

## Build-Measure-Learn Loop

Ordo should help a solopreneur experiment faster:

1. define an offer, story, or content hypothesis;
2. publish or send it;
3. observe response;
4. summarize evidence;
5. recommend revisions;
6. preserve what worked as reusable process.

The goal is not just content generation. The goal is lower business overhead
and faster learning.

## Knowledge And Packs

Future Ordo should support curated knowledge with provenance. A user should be
able to build approved content packs from source material and generated
artifacts, then load those packs into another Ordo.

This is not implemented yet. The near-term prerequisite is access-aware
knowledge/RAG with clear corpus provenance.

## Network Direction

Ordo should be useful alone. Later, Ordos may exchange governed artifacts and
requests.

The first likely network use case is support: a trial Ordo prepares a local
issue report, the operator approves sending it, and a maintainer Ordo receives a
provenanced support packet.

Worker Ordos, A2A networking, service discovery, and content-pack exchange are
future direction. They are not current product behavior.
