# LLM Instructions For Ordo

Status: root orientation for AI agents and external LLM tools

This file is the first thing an LLM should read when it receives this repository
as context. It is a compact map, not a substitute for source, tests, issues,
or the task-specific docs.

## Project Identity

Ordo is an AGPL local-first appliance for organizational intelligence. It is
not mainly a chatbot, SaaS dashboard, CRM clone, or tool marketplace.

The product principle is:

```text
Human decides. Assistant operates. Process governs. Evidence decides what can
be claimed.
```

Ordo explores sovereign software manufacturing: AI-assisted development,
governed work, visible QA, durable evidence, portable backups, and business
systems that independent developers and operators can inspect, modify, host,
and leave with.

## Current Product Direction

The active product target is Studio Ordo as a hosted appliance control plane
for AGPL Ordo appliances.

The hosted MVP loop is:

```text
meet Keith
-> scan QR
-> ask Ordo for a trial
-> capacity or waitlist
-> provision hosted Ordo appliance
-> route through Traefik
-> onboard while trial site is under construction
-> create conversation rollups and Growth briefs
-> ask for feedback and referrals
-> extend, convert, or close out
-> email final backup and return invitation
-> decommission only after export evidence exists
```

Do not collapse Studio Ordo into a generic SaaS site. Managed hosting is
convenience, not captivity.

## What Exists Now

The repository has strong foundations, not the full business product:

- Rust daemon and Next.js appliance runtime;
- SQLite source of truth with ordered migrations;
- job/task/event/artifact/brief foundations;
- scheduler, health, readiness, WebSocket projection, and System shell;
- backup creation and restore-preflight safety;
- capability catalog and governed MCP projection;
- public read models, tracked entry points, offers, trials, hosted trial slot
  capacity, waitlist, and reset guard foundations;
- conversation, realtime, LLM gateway, privacy egress, token accounting,
  deterministic eval, guarded live eval, artifact review, and report
  foundations.

Major future work includes hosted instance orchestration, Traefik control,
transactional email, scheduled Growth rollups, final backup email, full
decommission receipts, reward ledgers, benefit grants, broad A2A networking,
Studio Ordo Prime, premium media executors, and production public portals.

Never present future direction as shipped behavior.

## Source Of Truth Order

When files disagree, trust sources in this order:

1. Current source code and tests.
2. Current schema migrations and route contracts.
3. GitHub issues, test-plan issues, pull requests, and merge evidence for
   accepted work.
4. `docs/state-of-the-project.md` and `docs/system-overview.md`.
5. `docs/business/current-product-canon.md`.
6. `docs/business/studio-ordo-mvp.md`.
7. `docs/architecture/hosted-ordo-control-plane.md` and
   `docs/architecture/hosted-ordo-lifecycle.md`.
8. Current architecture, business, process, and eval docs.
9. Backlog docs as intent.
10. Ignored local drafts only when explicitly requested.

Folders under `docs/` beginning with `_` are private or local workspaces and
are ignored by git. Do not use `docs/_archive`, `docs/_drafts`, `docs/_codex`,
or other underscore folders as active product truth unless the user explicitly
asks you to inspect historical material.

## Architecture Spine

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

Architecture rules:

- SQLite owns canonical truth.
- Events own audit and replay.
- Projections and read models own surface experience.
- WebSocket is a live projection and command transport, not the record.
- Rust owns durable appliance behavior, SQLite migrations, policy, provider
  boundaries, job execution, backup/restore, realtime fanout, and local
  machine-sensitive work.
- Next.js owns product UI, routes, read-model rendering, policy-aware surface
  composition, and interaction states.
- MCP is a governed projection over registered capabilities, not arbitrary code
  execution and not a second execution spine.
- External LLM calls must pass through daemon-owned policy, prompt slots,
  privacy egress, and accounting.

Do not bypass capability, policy, artifact, visibility, audit, Access, Growth,
job/DAG, or projection boundaries.

## GitHub Manufacturing System

GitHub is the public manufacturing ledger.

```text
docs -> issue -> test-plan issue -> branch -> commit -> QA -> PR -> merge
-> issue closeout -> state docs
```

The project uses four agent operating modes:

- Research: refresh code, docs, GitHub milestone, PR state, issue state, and
  batch manifest before creating or updating the next executable batch.
- Execute: implement exactly one accepted implementation issue with a linked
  test-plan issue, TDD, validation, GitHub evidence comments, and a local
  commit.
- QA: review the completed branch adversarially for correctness, architecture,
  security, determinism, and coverage; make only narrow fixes if needed.
- Land: create or update the PR, merge only after QA and clean checks, then
  update implementation issue, test-plan issue, and batch manifest with
  merge-backed evidence.

Do not create commits, branches, PRs, issue comments, issue closes, pushes, or
merges unless the user explicitly asks for that workflow.

## Security And Privacy Posture

Treat Ordo as security-sensitive infrastructure.

- Never print or commit secrets, provider keys, access tokens, `.env.local`
  values, raw private transcripts, or vault material.
- Public/member surfaces must not leak staff routing, provider internals,
  prompt internals, raw policy internals, owner-only data, private artifact
  text, or unsupported capability claims.
- Default validation should be deterministic and network-free.
- Live provider work requires explicit guards, network intent, and budget caps.
- Backup archives contain sensitive appliance state and should be protected
  like the durable `.data` boundary.
- A2A, support packets, reports, and external submission must be approved,
  scoped, redacted, and receipt-backed.

The long-term security direction is proactive rapid reaction through automatic
QA, deterministic evals, local reports, backup/restore safety, governed egress,
A2A support packets, and member-visible evidence loops. Do not claim this is a
mature security program yet.

## Development Discipline

- Read before editing.
- Keep changes small and aligned with existing patterns.
- Use structured APIs and existing helpers over ad hoc parsing.
- Add or update tests when behavior changes.
- Update docs when implementation changes user-visible architecture, runtime,
  workflow, validation, or product truth.
- Preserve unrelated user changes in dirty worktrees.
- Do not reset, revert, or delete unrelated work unless explicitly asked.
- For issue work, prefer `main` plus a scoped branch named
  `codex/issue-<number>-<short-slug>` unless the manifest says otherwise.

Validation should scale with risk. For shared behavior, the full matrix is:

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

## Public Story Tone

Use plain, strong language. Be honest about what exists now, what is MVP next,
and what remains future.

Useful truths:

- The appliance is yours. Studio Ordo helps you run it.
- Hosted convenience without hostage energy.
- Every trial ends with a backup, not a cliff.
- Every conversation should leave evidence the business can use.
- An offer starts the relationship. An ask grows it.
- No dashboards. Read the brief.
- Ordo is not the model. Ordo is the operating harness.

Avoid generic AI hype, false scarcity, unsupported security claims, and
features that pretend future architecture is already shipped.