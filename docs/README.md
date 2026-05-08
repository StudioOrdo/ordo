# Ordo Docs

This folder separates public doctrine from local drafts and imported reference
material.

## Start Here

1. [Project README](../README.md)
2. [State Of The Project](state-of-the-project.md)
3. [Business Canon](business/README.md)
4. [Architecture](architecture/README.md)
5. [Process](process/README.md)
6. [Decisions](decisions/README.md)

## Public Docs

| Area | Purpose |
| --- | --- |
| [business](business/README.md) | Product thesis, business model, governance principles, and UX intent. |
| [architecture](architecture/README.md) | System boundaries, runtime direction, and technical decisions. |
| [process](process/README.md) | How work moves through issues, pull requests, checks, review, and release evidence. |
| [decisions](decisions/README.md) | Accepted architecture and operating decisions. |

## Current Reader Path

For the current appliance, read:

1. [State Of The Project](state-of-the-project.md)
2. [System Architecture](architecture/system-architecture.md)
3. [Diagnostics And Reports](architecture/diagnostics-and-reports.md)
4. [Product Shape](business/product-shape.md)

For future direction, read:

1. [Project Philosophy](business/project-philosophy.md)
2. [Founding Thesis](business/founding-thesis.md)
3. [Sovereignty Stack](business/sovereignty-stack.md)
4. [Scaling With Worker Ordos](architecture/scaling-worker-ordos.md)

## Local Docs Convention

Folders under `docs/` that start with `_` are private or local workspaces and
are ignored by git.

Examples:

- `docs/_drafts/`
- `docs/_research/`
- `docs/_archive/`
- `docs/_debug/`
- `docs/_imports/`

Promote material out of an underscore folder before treating it as public canon.

## Source Of Truth

When sources disagree, trust them in this order:

1. Current source code and tests.
2. [State Of The Project](state-of-the-project.md).
3. [Project README](../README.md).
4. Current business, architecture, process, and decision docs.
5. Local drafts and archived reference material.
