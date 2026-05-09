# Ordo

Ordo is a local-first operating system for one-person businesses.

The owner works in conversation. Behind the conversation, Ordo remembers
context, routes work, keeps evidence, runs governed production loops, and brings
results back with enough proof to trust, revise, or reject them.

Ordo is not a chat widget, dashboard bundle, or tool marketplace.

The product principle is:

> Human decides. Assistant operates. Process governs.

## What Ordo Is For

Small expert businesses often know what needs to happen, but the work is spread
across chat, files, tools, notes, follow-ups, content, offers, and
relationships.

Ordo exists to absorb operational drag while preserving human authority.

It is built for operators who need:

- durable memory and context;
- governed work instead of ad hoc prompting;
- evidence-backed outputs;
- relationship continuity;
- public and private offers;
- content and media production with QA;
- a clear view of what needs attention next.

## Product Shape

Chat is the operating interface.

The UI is the governance layer.

A typical loop is:

1. State the intent in conversation.
2. Ground the request in evidence and context.
3. Turn it into governed work.
4. Produce an artifact, offer, content item, or relationship outcome.
5. Run QA when the output needs review.
6. Publish, share, send privately, or follow up.
7. Measure what happened.
8. Recommend the next useful action.

## Project Philosophy

Ordo is not trying to become another SaaS dashboard. It is an open appliance for
observable AI work: intent, evidence, capability, decision, artifact, and review
should be inspectable by the people using the system.

If the browser made the web usable, Ordo is trying to make AI work governable.
The code is one expression of that philosophy; the durable goal is a repeatable,
auditable system of work that people can run, study, modify, and leave with.

Ordo is happily self-funded and community-first. AI-assisted development gives
the project unusual velocity, but the limiting factor is intentionally human:
manual QA, code review, functional review, public evidence, and judgment.
Velocity without verification is waste.

## Technical Direction

Ordo is being designed as a sovereign appliance:

- one Docker image;
- SQLite for durable local-first state;
- Next.js for product routes, UI, auth, policy, and read models;
- Rust for realtime fanout, native execution, backup/restore, media, and local
  search work;
- local files for generated artifacts, backups, and media;
- no required external infrastructure for the core product.

Managed hosting should be convenience, not captivity.

The first release target was
[Ordo 0.1.0 Appliance Specimen](docs/process/release-0.1.0.md): a small,
working proof of the core architecture before product depth. The current
backend readiness map is
[Backend Handoff Package 0.1.2](docs/process/backend-handoff-package-0.1.2.md).

Read the [system architecture contract](docs/architecture/system-architecture.md)
for the 0.1.0 design.

## Current Build

The current repository contains the local appliance foundation, not the full
business product yet.

Implemented now:

- one Docker appliance image with a Rust daemon supervising a Next.js management
  UI;
- SQLite state with ordered schema migrations;
- a reusable process/job/task kernel with durable events and artifacts;
- System Brief, Health, Backup And Restore, Schedules, Preferences, Events,
  Logs, and Reports surfaces in the System shell;
- structured diagnostic logs and local issue report preparation;
- backup creation and restore preflight safety;
- a capability catalog and local MCP JSON-RPC projection with policy tiers;
- a resource/provenance policy spine and durable local access foundation for
  system and owner resources;
- persisted realtime event replay plus WebSocket projection;
- protected local backend route contracts for install/provider state, business
  facts, entry points, offers/trials, connections, availability/handoff,
  reports/support packets, corpus retrieval, answer drafts, and MCP pack
  metadata;
- public-safe daemon read models for About, Offers, Asks, Feed, public entry
  point resolution, visitor session creation, and public offer acceptance.

Not implemented yet:

- public Chat, About, Offers, Asks, and Feed frontend product surfaces;
- authentication UI, hosted identity, public portals, and product-depth access
  enforcement;
- embeddings, vector search, provider-backed answer generation, chat retrieval
  UI, and content packs;
- hosted trial orchestration;
- Worker Ordos, A2A networking, external report submission, support packet
  transport, and arbitrary third-party MCP execution.

Reports are local evidence packages today. Ordo can prepare, preview, copy, and
export a markdown report from appliance diagnostics, but it does not submit
reports to GitHub, support systems, or other Ordos yet.

## Near-Term Product Direction

The next product layers should connect the working appliance spine to the
solopreneur operating system:

- Chat becomes the primary interface: clients get one persistent relationship
  conversation, staff work handoff queues and briefs, and admins operate the
  appliance.
- About becomes the public business story.
- Offers describe what can be bought.
- Asks, referrals, outcomes, artifacts, and conversations become measurable
  parts of the business loop instead of disconnected content.
- Feed publishes composite public artifacts for people and machines.
- Future product-depth RBAC keeps public, signed-in, owner/admin, and per-user
  private data separate on top of the local access foundation.
- Knowledge/RAG grounds answers in approved corpus material with provenance.
- Brief-first surfaces and evidence-backed recommendations guide the next useful
  action without becoming a generic CRM, dashboard, or support inbox.
- Content packs become portable, human-approved knowledge products.
- Worker Ordos and A2A remain future architecture for scaling bounded work while
  one Home Ordo owns canonical truth.

## Software Manufacturing

This repository builds Ordo in public using the same process Ordo asks the
product to use:

```text
evidence -> issue -> accepted scope -> branch -> pull request -> checks -> review -> merge -> release evidence
```

Markdown owns durable doctrine.

GitHub issues own visible work.

Pull requests own implementation evidence.

Nothing is called done without proof.

Read [docs/process/ordo_process.md](docs/process/ordo_process.md) for the
working process.

## Repository Status

This repository has completed the 0.1.0 Appliance Specimen pass, the 0.1.1
appliance trust-boundary stabilization pass, and the 0.1.2 backend readiness
foundation. The current work is still not production business automation; it is
the inspectable local appliance and backend contract foundation that later
product-depth UI and hosted surfaces will use.

The product is not ready for production use yet.

## Current Commands

The Rust appliance daemon starts in `crates/ordo-daemon`. Database startup runs
ordered SQLite migrations and seeders; repeat `init-db` or `serve` runs are
idempotent.

```bash
cargo run -p ordo-daemon -- health-json
cargo run -p ordo-daemon -- init-db --db-path .data/local.db
cargo run -p ordo-daemon -- ready-json --db-path .data/local.db
cargo run -p ordo-daemon -- list-capabilities-json --db-path .data/local.db
cargo run -p ordo-daemon -- mcp-json --db-path .data/local.db --method tools/list
cargo run -p ordo-daemon -- mcp-json --db-path .data/local.db --method tools/call --params-json '{"name":"system.status.read","arguments":{}}'
cargo run -p ordo-daemon -- latest-system-brief-json --db-path .data/local.db
cargo run -p ordo-daemon -- generate-system-brief-json --db-path .data/local.db
cargo run -p ordo-daemon -- create-backup-json --db-path .data/local.db
cargo run -p ordo-daemon -- list-backups-json --db-path .data/local.db
cargo run -p ordo-daemon -- restore-preflight-json --db-path .data/local.db --backup-id <backup_id> --confirmation "RESTORE <backup_id>"
cargo run -p ordo-daemon -- serve --db-path .data/local.db
npm run export
```

`npm run export` writes an ignored `project-export.txt` context bundle for
external AI/code review tools. See [Project Export](docs/process/project-export.md)
for when to use it and what it intentionally excludes.

The current System shell is a Next.js management UI over the local daemon.

```bash
npm install
npm run dev
npm run typecheck
npm run build
npm run smoke:ui
```

## Docker Appliance Runtime

The Phase 5 appliance packages the Rust daemon and Next.js management UI in one
image. The daemon is the top-level process and starts the Next standalone server
as a child process.

Build the image:

```bash
docker compose build
```

Run the appliance:

```bash
docker compose up
```

Then open `http://localhost:3000` for the UI. The daemon is exposed at
`http://localhost:17760` for health, readiness, API routes, and WebSocket
projection.

The daemon also exposes the capability catalog. MCP is protected by the daemon
access boundary: local CLI calls work directly, while HTTP calls to `/mcp` from
outside the daemon network namespace need `ORDO_DAEMON_ACCESS_TOKEN` and a
matching `Authorization: Bearer <token>` or `X-Ordo-Daemon-Token: <token>`
header.

```bash
curl http://localhost:17760/capabilities
curl 'http://localhost:17760/events?after=0&limit=100'
cargo run -p ordo-daemon -- mcp-json --db-path .data/local.db --method tools/list
cargo run -p ordo-daemon -- mcp-json --db-path .data/local.db --method tools/call --params-json '{"name":"system.status.read","arguments":{}}'
```

`tools/list` returns policy metadata such as `read_only`, `local_mutation`, and
`operator_confirmed` so read tools and local mutating tools are distinguishable.
MCP requests are validated as JSON-RPC 2.0 before dispatch, and `tools/call`
arguments are checked against the catalog input schema before any tool runs.
`/events` returns persisted job and system lifecycle events after a cursor so
the UI can recover missed WebSocket events after reconnecting.

Useful runtime commands:

```bash
docker compose logs -f ordo
docker compose stop
docker compose start
docker compose down
```

Persistence is handled by the named Compose volume `ordo-data`, mounted at
`/app/.data` in the container. SQLite lives at `/app/.data/local.db`; backup
archives and restore safety records are written below `/app/.data/backups`.
`docker compose down` preserves that volume. `docker compose down -v` removes
it.

Validation:

```bash
npm run check
npm run smoke:ui
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The UI smoke suite uses Playwright with a lightweight mock daemon on
`127.0.0.1:19080` and a Next.js test server on `127.0.0.1:3100`. It covers
desktop and mobile Chromium viewports for daemon-available and daemon-degraded
shell behavior, System Brief evidence/provenance, Backup And Restore persisted
jobs, operator controls, and the browser backup creation path.

## Docs

Start here:

- [Docs Index](docs/README.md)
- [Project State](docs/state-of-the-project.md)
- [Business Canon](docs/business/README.md)
- [Architecture](docs/architecture/README.md)
- [Process](docs/process/README.md)
- [Decisions](docs/decisions/README.md)

## License

Ordo is licensed under [AGPL-3.0-only](LICENSE).

The license supports the sovereignty goal: users should be able to inspect,
modify, host, and leave with their system. Hosted modifications should remain
part of the commons.
