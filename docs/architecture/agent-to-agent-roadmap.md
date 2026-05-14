# Agent-To-Agent Roadmap

Status: future direction

Current canon: [Current Product Canon](../business/current-product-canon.md)

Ordo should eventually be able to act as a sovereign node on an agent network.
The goal is not one giant centralized SaaS. The goal is many appliances, each
owning local truth, exchanging bounded requests, artifacts, receipts, and
capability offers under explicit policy.

Implementation must re-check the active Agent2Agent protocol specification
before shipping external compatibility. This document defines Ordo's product
and architecture stance, not a frozen protocol implementation.

## Core Principle

```text
One Ordo owns truth.
Other Ordos or agents may perform bounded work.
Artifacts, receipts, and outcomes flow back through policy.
```

MCP connects agents to tools and data. Agent-to-agent networking connects
agents to other agents or appliances. Ordo should support both without exposing
raw internals.

## Ordo Projection

An external agent should not see Ordo's private database, prompt internals,
staff routing, or raw logs. It should see a governed projection:

- public capability or offer descriptions;
- scoped request envelopes;
- approved artifact packets;
- visibility-safe context;
- required approvals and constraints;
- receipts, status, and outcome events.

## Conceptual Mapping

Ordo can map its local objects to common agent-network concepts:

| Agent-network concept | Ordo projection |
| --- | --- |
| Agent identity or card | Public capability, offer, ask, or service projection |
| Message | Conversation event, request comment, or handoff note |
| Task | Request, job, worker assignment, or support packet |
| Artifact | Ordo artifact with provenance, checksum, visibility, and limits |
| Context id | Connection, conversation, assignment, or handoff context |
| Receipt/status | Event, receipt, outcome, or brief entry |

These mappings should remain adapters around Ordo-native objects. Do not make
an external protocol the internal source of truth.

## First Wedge

The first useful wedge is support packet exchange:

```text
local Ordo
-> approved diagnostic/support artifact
-> maintainer Ordo
-> receipt
-> issue/support brief
-> bounded response artifact
```

This is safer than broad remote execution because it depends on artifacts,
approval, receipt, and bounded egress.

For the hosted appliance MVP, the launch-specific version of this wedge is
[A2A Studio Ordo Prime](a2a-studio-prime.md): hosted Ordos submit governed
feedback, support packets, lifecycle receipts, trial extension requests, and
premium job requests to Studio Ordo Prime without exposing raw appliance
internals.

## Later Wedges

After the support-packet path is durable, Ordo can add:

- Worker Ordo assignments for bounded jobs;
- offer/ask discovery between trusted Ordos;
- referral exchange and attribution receipts;
- content-pack or knowledge-pack review;
- specialist service delegation;
- cross-Ordo artifact QA.

## Required Foundations

Before external agent networking ships, Ordo needs:

- durable actors, connections, and scoped grants;
- access-aware retrieval and artifact visibility;
- request envelopes with approval requirements;
- artifact provenance, checksums, and retention policy;
- egress review and receipt handling;
- task leases, cancellation, retries, and result envelopes;
- audit events for every cross-boundary action.

## Trust Rules

- No hidden egress.
- No remote mutation of canonical local truth.
- No unscoped worker access.
- No artifact accepted without validation.
- No public capability claim that policy cannot enforce.
- No protocol compatibility before local request, artifact, and receipt
  semantics are solid.
