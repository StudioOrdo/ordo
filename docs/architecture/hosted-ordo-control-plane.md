# Hosted Ordo Control Plane

Status: MVP architecture direction

The hosted control plane lets Studio Ordo run many trial Ordo appliances from
one shared image while keeping each trial's data, media, hostname, and lifecycle
evidence isolated.

## Core Pattern

```text
Studio Ordo control plane
-> hosted Ordo instance record
-> shared Docker image
-> per-trial container
-> per-trial data/media volume
-> Traefik hostname route
-> lifecycle jobs and artifacts
```

One image can run many containers. The address difference comes from routing,
not from building a new image per customer. Example hostnames:

```text
trial1.studioordo.com
keith-demo.studioordo.com
acme-alpha.studioordo.com
```

Traefik maps each hostname to the correct container. The durable owner boundary
is the per-trial volume and the SQLite database inside that trial boundary.

## Control Plane Responsibilities

- record hosted instances and their trial, owner, offer, slot, and route;
- allocate a hostname or subdomain;
- create or attach per-trial data and media volumes;
- start, stop, restart, and inspect containers;
- attach Traefik labels or dynamic route config;
- store provisioning, readiness, backup, and decommission evidence;
- show capacity, waitlist, reminders, and closeout state to Studio Ordo staff;
- never treat WebSocket or container state as canonical when SQLite evidence is
  required.

## Instance State

A hosted instance should have a state machine separate from trial status:

```text
requested
-> provisioning
-> ready
-> onboarding
-> active
-> expiring
-> closeout_pending
-> backup_ready
-> decommission_pending
-> decommissioned
```

Failure states should be explicit:

```text
provision_failed
route_failed
readiness_failed
backup_failed
decommission_failed
```

## Storage Rule

Do not store customer media in the container image or writable container layer.
For MVP, use per-trial mounted volumes. Later, large media can move to governed
object storage such as MinIO or S3-compatible storage while preserving backup
and export semantics.

## Trust Boundary

The control plane may orchestrate containers, but an Ordo appliance still owns
its own truth. Cross-boundary actions need evidence, policy, and receipts:

- provisioning receipt;
- route receipt;
- health/readiness receipt;
- backup artifact reference;
- final export receipt;
- decommission receipt.

No hosted tool should become arbitrary remote code execution. Provisioning is a
bounded platform capability with templates, labels, known image references, and
recorded outcomes.

## What Is Not Built Yet

- hosted instance table and routes;
- Docker/Traefik orchestration commands;
- owner/staff control plane UI;
- route verification job;
- per-trial volume manifest;
- production TLS/domain automation;
- quota/usage ledger for support and premium capabilities.