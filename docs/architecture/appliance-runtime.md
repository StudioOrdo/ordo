# Appliance Runtime

Status: Draft contract for Ordo 0.1.0

Ordo 0.1.0 runs as one local appliance.

## Rust Daemon

The Rust daemon is the top-level runtime process in production.

It owns:

- starting and monitoring the Next.js child process;
- health and readiness probes;
- scheduler loop and due-task claiming;
- WebSocket pubsub;
- native/system job execution;
- backup and restore execution;
- writing job, task, schedule, and system events to SQLite.

The daemon may restart Next when health checks fail or when a restore requires
it. Restart attempts must emit operator-visible lifecycle events.

## Next Child Supervision

When the daemon is started with `--next-command`, the Next.js child process is a
required appliance component. The daemon tracks the child in memory and includes
that required-child state in `/ready`.

The 0.1.1 policy is bounded restart:

- the daemon emits `next.supervisor.started` when a child starts;
- any child exit emits `next.supervisor.exited`;
- transient exits schedule up to three `next.supervisor.restart_attempt` events;
- a successful restarted child emits `next.supervisor.recovered`;
- an exhausted restart budget emits `next.supervisor.final_failure` and makes
	`/ready` return `not_ready`.

When no Next supervisor is configured, daemon readiness remains scoped to the
SQLite appliance checks so local daemon-only development keeps working.

These supervision events are realtime system lifecycle events in 0.1.1. Durable
event replay is tracked separately from this runtime supervision slice.

## Next.js

Next owns product surfaces:

- System Brief page;
- System second-column menu;
- health, backup, schedule, preference, and event read models;
- route-level policy checks;
- user interaction that creates jobs through the daemon or shared kernel.

Next should not be the only process responsible for appliance survival.

## Docker Boundary

0.1.0 should package one Docker image with `.data` as the durable volume.

The image should not require external queues, external schedulers, hosted
databases, or hosted realtime infrastructure for core behavior.

## Phase 5 Runtime Shape

The Phase 5 Docker appliance keeps the Rust daemon as PID 1. The daemon starts
and monitors the Next.js standalone server as a child process.

Default container ports:

- `3000` for the Next.js management UI;
- `17760` for daemon health, readiness, API routes, and WebSocket projection.

Docker Compose mounts the named `ordo-data` volume at `/app/.data`. SQLite,
backup archives, restore safety records, and generated runtime artifacts stay
inside that mounted boundary. Local `.data` is excluded from the image build
context.