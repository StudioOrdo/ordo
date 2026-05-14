# Worker Ordos And A2A MVP

Status: future direction

Architecture direction: [Agent-To-Agent Roadmap](../architecture/agent-to-agent-roadmap.md)

## Why It Matters

Ordo should scale by letting worker appliances do bounded work and return
artifacts, without handing over authority or turning the system into centralized
SaaS.

## MVP Scope

- Define Home Ordo and Worker Ordo roles.
- Assign a job or task bundle to a worker.
- Return artifacts, events, and receipts to Home Ordo.
- Keep Home Ordo as source of truth.
- Define a support-packet style A2A envelope before broader peer networking.

## Durable Product Nouns

- Home Ordo
- Worker Ordo
- Assignment
- Returned Artifact
- Peer Receipt
- A2A Envelope

## Acceptance Criteria

- Worker receives only scoped work and data.
- Returned artifacts include provenance and checksums.
- Home Ordo can accept or reject returned artifacts.
- No worker receives authority over Home Ordo truth.

## Non-Goals

- Global Ordo network.
- Service discovery.
- Marketplace.
- Remote arbitrary execution.

## Validation

- Envelope schema tests.
- Artifact checksum/provenance tests.
- Mock worker round-trip proof.
