# System Architecture

Status: Draft contract for Ordo 0.1.0

Ordo is a local-first AI appliance for one-person businesses. The 0.1.0
release proves the core system before product depth.

## Core Spine

```text
Capability Catalog -> Process Template -> Job -> Task DAG -> Event -> Artifact -> Brief
```

The catalog defines what Ordo can do. Process templates compose cataloged task
kinds into reusable directed acyclic graphs. Jobs are concrete runs of those
templates. Tasks emit durable events and produce artifacts. Briefs turn current
evidence into plain-language staff reports.

## Runtime Boundaries

| Layer | Responsibility |
| --- | --- |
| Rust daemon | Appliance supervision, scheduler, WebSocket pubsub, native/system jobs, health, backup/restore execution. |
| Next.js | Product UI, routes, read models, policy checks, brief rendering, shell navigation. |
| SQLite | Durable local state for capabilities, templates, jobs, tasks, events, artifacts, schedules, briefs, preferences, backups, and ordered schema versioning. |
| Docker | One-image appliance packaging with `.data` as the durable boundary. |

Rust owns long-running appliance behavior. Next owns product meaning and user
experience. SQLite is the source of truth. WebSocket is a live projection, not
the record.

SQLite schema changes are applied through ordered daemon migrations tracked by
`PRAGMA user_version`. Fresh databases and existing 0.1.0 databases use the
same initialization path before catalog/template seeding runs.

## 0.1.0 Product Surface

The default UI is the System Brief. Health details, Backup And Restore,
Schedules, Preferences, and Events live in the second-column System menu.

0.1.0 proves that backup, restore, and brief generation are concrete job
templates running on the reusable kernel, not bespoke flows.

## Non-Goals

- Full business surfaces such as Studio, People, Offers, and Today.
- Arbitrary user-defined code execution.
- External hosted services as required infrastructure.
- Time-based progress promises or unsupported ETAs.