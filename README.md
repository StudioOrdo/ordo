# Ordo

Ordo is an AGPL local-first appliance for organizational intelligence.

It is not mainly a chatbot, SaaS dashboard, CRM clone, or tool marketplace. It
is an attempt to make AI-mediated work observable, governable, portable,
reviewable, and useful for independent businesses and independent software
developers.

The product principle is:

```text
Human decides. Assistant operates. Process governs. Evidence decides what can
be claimed.
```

Ordo is open because the problem is larger than one company. The code matters,
but the code is not the whole project. The project is a way to test a different
software manufacturing model: doctrine in docs, visible work in GitHub,
evidence in pull requests, validation in tests and evals, and human judgment at
the gates.

## Why This Exists

AI makes software and business operations faster to attempt. It does not make
them automatically trustworthy.

The scarce resources are now judgment, QA, security, useful distribution,
evidence, and economic alignment. Ordo exists to explore a different pattern:
independent developers building sovereign software appliances that preserve
local truth, create durable artifacts, and help operators make better decisions
without surrendering their business memory to an opaque platform.

The investor this project is trying to convince first is the developer: the
person who believes software should let independent people make real impact,
benefit from each other's feedback, and create new ways to earn without forcing
everyone into one captive SaaS database.

## What Class Of Software This Is

Ordo is exploring several connected ideas:

- local-first organizational intelligence;
- sovereign AI appliances;
- governed workforce substrates;
- AI-assisted software manufacturing;
- business growth loops based on conversations, offers, asks, artifacts,
  briefs, QA, and feedback;
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

## How This Project Is Built

Ordo is solo-developed and heavily AI-assisted. ChatGPT is used for planning,
strategy, writing, image generation, and product exploration. Coding agents are
used for implementation, QA, landing, and research under explicit GitHub, test,
and evidence rules.

At this stage, the project is being built on a roughly $200/month AI tool
budget. That is part of the thesis: disciplined AI-assisted software
manufacturing can let independent developers attempt infrastructure-scale ideas
before a company exists around them.

The point is not that AI writes perfect software. It does not. The point is
that speed only becomes useful when paired with public evidence, QA, review,
security boundaries, and merge-backed truth.

GitHub is the public manufacturing ledger:

```text
docs -> issue -> test-plan issue -> branch -> commit -> QA -> PR -> merge
-> issue closeout -> state docs
```

Read [Agent Development Workflow](docs/process/agent-development-workflow.md)
for the Research, Execute, QA, and Land operating modes.

## Current Reality

The repository has a strong local appliance foundation, not the full business
product yet.

Implemented foundations include:

- Rust daemon and Next.js appliance runtime;
- SQLite source of truth with ordered migrations;
- process/job/task/event/artifact/brief foundations;
- scheduler, health, readiness, WebSocket projection, and System shell;
- backup creation and restore-preflight safety;
- capability catalog and governed MCP projection;
- public read models, tracked entry points, offers, trials, hosted trial slot
  capacity, waitlist, and reset guard foundations;
- conversation realtime, LLM gateway, privacy egress, token accounting,
  deterministic evals, guarded live evals, reports, and artifact review.

Not built yet:

- Docker/Traefik hosted instance orchestration;
- hosted instance records and control-plane UI;
- transactional email and reminder delivery;
- scheduled Growth rollups from conversations;
- final backup email and return invitation;
- full decommissioning receipts;
- reward ledger, benefit grants, quotas, and affiliate payout automation;
- governed A2A networking and Studio Ordo Prime implementation;
- premium media production executors;
- production public portals.

The product is not ready for production use yet.

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

Read:

- [Studio Ordo Hosted Appliance MVP](docs/business/studio-ordo-mvp.md)
- [Hosted Ordo Control Plane](docs/architecture/hosted-ordo-control-plane.md)
- [Hosted Ordo Lifecycle](docs/architecture/hosted-ordo-lifecycle.md)
- [Notifications And Transactional Email](docs/architecture/notifications-and-transactional-email.md)
- [A2A Studio Ordo Prime](docs/architecture/a2a-studio-prime.md)

## Security And Rapid Response

Ordo treats security as part of the appliance architecture.

AI increases the speed of software creation and the speed of software abuse.
The long-term goal is proactive rapid reaction: automatic QA, deterministic
evals, local diagnostic reports, backup and restore safety, governed egress,
A2A support packets, and member-visible evidence loops.

This is a direction, not a claim that the project already has a mature security
program.

Read [Security And Rapid Response](docs/security-and-rapid-response.md).

## How Developers Can Help

The project currently needs QA more than random feature expansion.

Useful contributions include:

- running the appliance and filing evidence-backed issues;
- reviewing public claims against source and tests;
- improving deterministic evals and smoke coverage;
- testing backup, restore, reports, and chat behavior;
- reviewing security, privacy, egress, and redaction boundaries;
- improving docs where product direction is unclear;
- helping turn broad ideas into small issue/test-plan pairs;
- eventually building governed capabilities, packs, support services, and
  production tools around the AGPL appliance.

Read [Contributing](CONTRIBUTING.md) and
[QA And Verification](docs/qa-and-verification.md).

## Business Model

The business model is not hostage SaaS.

The model is AGPL appliance ownership plus optional hosting, support, premium
production tools, capability packs, governed networks, and services that make
the appliance easier and more valuable to run.

Managed hosting is convenience, not captivity. The user should be able to
inspect, modify, host, back up, and leave with the system that holds their
business memory.

Read [Open Source Business Model](docs/business/open-source-business-model.md).

## Local Development

Requirements:

- Node.js compatible with the current Next.js version;
- npm;
- Rust toolchain compatible with the workspace;
- Docker and Docker Compose for appliance runtime proof;
- Playwright browser dependencies for UI smoke tests.

Install dependencies:

```bash
npm install
```

Initialize local daemon state:

```bash
cargo run -p ordo-daemon -- init-db --db-path .data/local.db
cargo run -p ordo-daemon -- ready-json --db-path .data/local.db
```

Run the appliance development runtime:

```bash
npm run dev
```

`npm run dev` starts the Rust daemon and lets the daemon supervise Next.js. It
preflights toolchains, checks ports, loads `.env.local` without printing secret
values, verifies local Ollama by default, initializes SQLite, and prints daemon
and UI URLs.

Run raw Next.js only when deliberately bypassing daemon integration:

```bash
npm run dev:next
```

Run the daemon separately only when testing daemon APIs without Next.js:

```bash
cargo run -p ordo-daemon -- serve --db-path .data/local.db
```

## Common Commands

Rust daemon:

```bash
cargo run -p ordo-daemon -- health-json
cargo run -p ordo-daemon -- init-db --db-path .data/local.db
cargo run -p ordo-daemon -- ready-json --db-path .data/local.db
cargo run -p ordo-daemon -- list-capabilities-json --db-path .data/local.db
cargo run -p ordo-daemon -- latest-system-brief-json --db-path .data/local.db
cargo run -p ordo-daemon -- generate-system-brief-json --db-path .data/local.db
cargo run -p ordo-daemon -- create-backup-json --db-path .data/local.db
cargo run -p ordo-daemon -- list-backups-json --db-path .data/local.db
```

MCP projection:

```bash
cargo run -p ordo-daemon -- mcp-json --db-path .data/local.db --method tools/list
cargo run -p ordo-daemon -- mcp-json --db-path .data/local.db --method tools/call --params-json '{"name":"system.status.read","arguments":{}}'
```

Project export for external review tools:

```bash
npm run export
```

`npm run export` writes ignored `project-export.txt`. Review it before sharing
outside your machine.

## Docker Appliance Runtime

Create local runtime env only when needed:

```bash
cp .env.example .env.local
```

Do not commit `.env.local`. Compose reads it at runtime through `env_file`, and
the Docker build does not bake it into image layers.

Build and run:

```bash
docker compose build
docker compose up
```

Open `http://localhost:3000` for the UI. The daemon listens on
`http://localhost:17760`.

Persistence lives in the named Compose volume `ordo-data`, mounted at
`/app/.data`. Use `docker compose down -v` only when you intentionally want to
delete appliance state.

## Validation

Use validation proportional to the change. For shared behavior, run:

```bash
npm run typecheck
npm run build
npm run smoke:ui
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

For doc-only changes, `git diff --check` and link/path sanity are usually
enough.

## Docs

Start here:

- [Public Project Brief](docs/public-project-brief.md)
- [Docs Index](docs/README.md)
- [System Overview](docs/system-overview.md)
- [Developer Guide](docs/developer-guide.md)
- [LLM Instructions](llm_instructions.md)
- [LLM Agent Guide](docs/llm-agent-guide.md)
- [State Of The Project](docs/state-of-the-project.md)
- [Eval System](docs/evals/README.md)
- [Issue History](docs/process/issue-history.md)
- [Business Canon](docs/business/README.md)
- [Architecture](docs/architecture/README.md)
- [Process](docs/process/README.md)
- [Decisions](docs/decisions/README.md)
- [Backlog](docs/backlog/README.md)

## License

Ordo is licensed under [AGPL-3.0-only](LICENSE).

The license supports the sovereignty goal: users should be able to inspect,
modify, host, and leave with their system. Hosted modifications should remain
part of the commons.