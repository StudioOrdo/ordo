# Appliance Runtime

Status: Draft contract for Ordo 0.1.1 trust boundary

Ordo runs as one local appliance.

The appliance should package mature operating patterns without requiring
hosted enterprise infrastructure. SQLite, the Rust daemon, and Next.js are the
default runtime boundary. Queues, schedulers, read models, event replay, and
artifact records should remain local unless the operator explicitly installs a
different execution target.

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

These supervision events are durable system lifecycle events in 0.1.1. They are
persisted before WebSocket fanout so clients can replay missed events after a
cursor.

## Next.js

Next owns product surfaces:

- System Brief page;
- System second-column menu;
- health, backup, schedule, preference, and event read models;
- route-level policy checks;
- user interaction that creates jobs through the daemon or shared kernel.

Next should not be the only process responsible for appliance survival.

## Network And Access Boundary

The daemon is the internal appliance authority. It is not a public product API
surface by default.

The default 0.1.1 access posture is:

- `/health` and `/ready` remain unauthenticated so Docker and local operators can
  probe the appliance;
- read-model routes such as `/capabilities`, `/backups`, and
  `/briefs/system/latest` remain available for the local System shell;
- mutating daemon routes such as `/briefs/system/generate`, `/backups/create`,
  and `/restore/validate` require either loopback access to the daemon or a
  valid daemon access token;
- `/mcp` requires the same loopback-or-token boundary;
- WebSocket projection remains read-only runtime projection and is not an
  execution boundary.

Loopback access means the request reaches the daemon from the same network
namespace, such as the bundled Next.js server calling `http://127.0.0.1:17760`
inside the appliance. Non-loopback requests to protected routes must provide the
configured daemon access token using either `Authorization: Bearer <token>` or
`X-Ordo-Daemon-Token: <token>`.

The token is configured with `--daemon-access-token` or
`ORDO_DAEMON_ACCESS_TOKEN`. This is a first local trust-boundary guard, not a
multi-user RBAC system.

Local Compose binds both published ports to host loopback for development:

- `127.0.0.1:3000` for the Next.js management UI;
- `127.0.0.1:17760` for daemon health, readiness, development inspection, and
  local WebSocket projection.

Production-like deployment should expose the UI or reverse-proxy entrypoint and
avoid publishing the daemon port directly unless the protected daemon routes are
intentionally token-gated for that environment.

## Docker Boundary

The appliance packages one Docker image with `.data` as the durable volume.

The image should not require external queues, external schedulers, hosted
databases, or hosted realtime infrastructure for core behavior.

Enterprise systems often externalize queues, stream processors, search
clusters, job workers, and observability stacks. Ordo should first implement
small appliance-native versions: SQLite-backed jobs, events, projections,
diagnostics, reports, and artifact envelopes. External services are adapters,
not prerequisites.

## Current Runtime Shape

The Phase 5 Docker appliance keeps the Rust daemon as PID 1. The daemon starts
and monitors the Next.js standalone server as a child process.

Container ports:

- `3000` for the Next.js management UI;
- `17760` for daemon health, readiness, API routes, and WebSocket projection.

Docker Compose mounts the named `ordo-data` volume at `/app/.data`. SQLite,
backup archives, restore safety records, and generated runtime artifacts stay
inside that mounted boundary. Local `.data` is excluded from the image build
context.
