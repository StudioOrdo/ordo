# Access And Local RBAC

Status: Implemented local foundation slice

Ordo now has a durable local access model behind the Resource, Provenance, And
Policy Spine. This is a foundation for RBAC, not a full authentication product.

## Current Shape

The Rust daemon stores access data in SQLite:

- actors;
- roles;
- actor-role memberships;
- resource grants.

Fresh and upgraded databases seed deterministic local baseline records:

- `actor_system` with `role_system`;
- `actor_local_owner` with `role_owner`.

The seeded owner/system roles receive local grants for current system and
owner-system resources. This preserves the current System shell assumption that
the local operator is acting as the appliance owner while giving future access
work durable tables to build on.

## Policy Integration

The policy layer can now evaluate durable resource grants for actor, action, and
resource decisions.

The current implementation distinguishes:

- public resources;
- owner/system resources;
- per-actor private resources.

Protected daemon mutations still require the existing loopback or daemon access
token boundary. Once that local boundary is satisfied, current System shell
protected actions are represented as the local owner actor.

MCP export policy behavior remains governed by the capability catalog. This RBAC
foundation does not expand MCP exposure.

## What This Enables

This slice gives later product work a durable place to enforce access before
adding deeper surfaces such as:

- knowledge/RAG retrieval;
- public pages and chat;
- student/client/customer records;
- worker Ordos;
- A2A handoff;
- domain-specific legal, medical, finance, or tax workflows.

## Non-Goals

- No authentication UI.
- No OAuth, email magic link, passkey, or hosted login flow.
- No public customer, student, or client portal.
- No RAG/vector memory.
- No external integrations.
- No domain-specific legal, medical, finance, or tax mode.
- No broad Job Kernel V2 implementation.

The knowledge corpus skeleton now gives future retrieval work resource identity
and metadata fields to connect to this access path. It does not implement
retrieval, embeddings, or RAG.
