# Ordo 0.1.0 Appliance Specimen

Status: Planned

Milestone: https://github.com/StudioOrdo/ordo/milestone/1

0.1.0 is the first coherent Ordo appliance specimen. It proves the core
architecture before product depth.

## Goal

Demonstrate that Ordo can run as a local-first AI appliance with a reusable
process/job/task kernel, scheduled briefs, durable events, WebSocket progress,
backup/restore safety, and an Ordo-style System shell.

## Required Capabilities

- Rust daemon supervises the appliance and Next.js process.
- SQLite stores process templates, jobs, task DAGs, events, artifacts,
  schedules, briefs, preferences, and backup records.
- Scheduler creates jobs from process templates.
- WebSocket broadcasts persisted events.
- System Brief is the default UI and shows when it was created.
- System menu includes Brief, Health, Backup And Restore, Schedules,
  Preferences, and Events.
- Backup, restore, and brief generation are concrete job templates on the same
  reusable kernel.
- Docker runs the system as one appliance image.
- Capability catalog defines what Ordo can do; MCP is one governed projection.

## Non-Goals

- Full Studio, People, Offers, Today, and Conversations product depth.
- Arbitrary user code execution.
- Hosted infrastructure as a core dependency.
- Time-based progress or unsupported ETA promises.

## Release Evidence

The release cannot be called complete without:

- passing tests for DAG readiness, task-based progress, event persistence,
  scheduler due work, and WebSocket event shape;
- Docker run evidence;
- browser evidence for the System Brief and Backup And Restore pages;
- backup/restore evidence using `.data`;
- documentation of known limitations.