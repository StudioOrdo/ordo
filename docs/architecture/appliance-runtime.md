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
it. Restart attempts must emit durable events.

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