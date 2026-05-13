# Ordo Docs

This folder separates public doctrine from local drafts and imported reference
material.

## Start Here

1. [Project README](../README.md)
2. [System Overview](system-overview.md)
3. [Developer Guide](developer-guide.md)
4. [LLM Agent Guide](llm-agent-guide.md)
5. [State Of The Project](state-of-the-project.md)
6. [Eval System](evals/README.md)
7. [Issue History](process/issue-history.md)
8. [Business Canon](business/README.md)
9. [Architecture](architecture/README.md)
10. [Process](process/README.md)
11. [Decisions](decisions/README.md)
12. [Backlog](backlog/README.md)

## Public Docs

| Area | Purpose |
| --- | --- |
| [system overview](system-overview.md) | Current implemented system map for developers, reviewers, and LLM agents. |
| [developer guide](developer-guide.md) | Local setup, Docker, commands, validation, and live eval guard usage. |
| [LLM agent guide](llm-agent-guide.md) | Source-of-truth order, architecture assumptions, risky boundaries, and agent workflow. |
| [evals](evals/README.md) | Deterministic evals, personas, artifact packets, live guards, and finding categories. |
| [business](business/README.md) | Product thesis, business model, governance principles, and UX intent. |
| [architecture](architecture/README.md) | System boundaries, runtime direction, and technical decisions. |
| [process](process/README.md) | How work moves through issues, pull requests, checks, review, and release evidence. |
| [decisions](decisions/README.md) | Accepted architecture and operating decisions. |
| [backlog](backlog/README.md) | High-level MVP specs for future features and issue-ready slices. |

## Current Reader Path

For the current appliance, read:

1. [System Overview](system-overview.md)
2. [State Of The Project](state-of-the-project.md)
3. [Developer Guide](developer-guide.md)
4. [Eval System](evals/README.md)
5. [System Architecture](architecture/system-architecture.md)
6. [Diagnostics And Reports](architecture/diagnostics-and-reports.md)
7. [Product Shape](business/product-shape.md)

For LLM agents, read:

1. [LLM Agent Guide](llm-agent-guide.md)
2. [System Overview](system-overview.md)
3. [State Of The Project](state-of-the-project.md)
4. [Eval System](evals/README.md)
5. Current source and tests for the files being changed.

For future direction, read:

1. [Project Philosophy](business/project-philosophy.md)
2. [Founding Thesis](business/founding-thesis.md)
3. [Sovereignty Stack](business/sovereignty-stack.md)
4. [Ordo Core](business/ordo-core.md)
5. [Product Roadmap](business/product-roadmap.md)
6. [Scaling With Worker Ordos](architecture/scaling-worker-ordos.md)
7. [Backlog](backlog/README.md)

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
3. [System Overview](system-overview.md).
4. [Project README](../README.md).
5. Current business, architecture, process, and decision docs.
6. Local drafts and archived reference material.
