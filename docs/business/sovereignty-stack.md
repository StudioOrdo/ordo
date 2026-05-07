# Sovereignty Stack

## Core Infrastructure Stance

Small-business AI infrastructure should be portable, inspectable, and
inexpensive enough to survive without enterprise complexity.

The baseline direction:

- SQLite for durable local-first state;
- Docker for portable deployment;
- browser execution for client-side heavy work when appropriate;
- Rust for long-running native work and realtime fanout;
- simple defaults and explicit boundaries.

## AGPL And User Exit Rights

Ordo is AGPL-licensed so hosted modifications remain part of the commons.

Managed hosting is convenience, not captivity.

Users should be able to export their system and run it elsewhere with minimal
friction.

## Architectural Rule

Use simple defaults and explicit boundaries.

Complexity must pay for itself in measured reliability or cost reduction.