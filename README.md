# Ordo

Ordo is a local-first operating system for one-person businesses.

The owner works in conversation. Behind the conversation, Ordo remembers
context, routes work, keeps evidence, runs governed production loops, and brings
results back with enough proof to trust, revise, or reject them.

Ordo is not a chat widget, dashboard bundle, or tool marketplace.

The product principle is:

> Human decides. Assistant operates. Process governs.

## What Ordo Is For

Small expert businesses often know what needs to happen, but the work is spread
across chat, files, tools, notes, follow-ups, content, offers, and
relationships.

Ordo exists to absorb operational drag while preserving human authority.

It is built for operators who need:

- durable memory and context;
- governed work instead of ad hoc prompting;
- evidence-backed outputs;
- relationship continuity;
- public and private offers;
- content and media production with QA;
- a clear view of what needs attention next.

## Product Shape

Chat is the operating interface.

The UI is the governance layer.

A typical loop is:

1. State the intent in conversation.
2. Ground the request in evidence and context.
3. Turn it into governed work.
4. Produce an artifact, offer, content item, or relationship outcome.
5. Run QA when the output needs review.
6. Publish, share, send privately, or follow up.
7. Measure what happened.
8. Recommend the next useful action.

## Technical Direction

Ordo is being designed as a sovereign appliance:

- one Docker image;
- SQLite for durable local-first state;
- Next.js for product routes, UI, auth, policy, and read models;
- Rust for realtime fanout, native execution, backup/restore, media, and local
	search work;
- local files for generated artifacts, backups, and media;
- no required external infrastructure for the core product.

Managed hosting should be convenience, not captivity.

The first release target is [Ordo 0.1.0 Appliance Specimen](docs/process/release-0.1.0.md):
a small, working proof of the core architecture before product depth.

Read the [system architecture contract](docs/architecture/system-architecture.md)
for the 0.1.0 design.

## Software Manufacturing

This repository builds Ordo in public using the same process Ordo asks the
product to use:

```text
evidence -> issue -> accepted scope -> branch -> pull request -> checks -> review -> merge -> release evidence
```

Markdown owns durable doctrine.

GitHub issues own visible work.

Pull requests own implementation evidence.

Nothing is called done without proof.

Read [docs/process/ordo_process.md](docs/process/ordo_process.md) for the
working process.

## Repository Status

This repository is being initialized as the clean Studio Ordo build.

At this stage, the goal is to establish:

- the idea base;
- the public work process;
- architecture decisions;
- contribution rules;
- the first implementation contract.

The product is not ready for production use yet.

## Current Commands

The 0.1.0 Rust appliance spine starts in `crates/ordo-daemon`.

```bash
cargo run -p ordo-daemon -- health-json
cargo run -p ordo-daemon -- init-db --db-path .data/local.db
cargo run -p ordo-daemon -- ready-json --db-path .data/local.db
cargo run -p ordo-daemon -- serve --db-path .data/local.db
```

The Phase 2 system shell is a minimal Next.js app.

```bash
npm install
npm run dev
npm run build
```

Validation:

```bash
npm run check
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Docs

Start here:

- [Docs Index](docs/README.md)
- [Project State](docs/state-of-the-project.md)
- [Business Canon](docs/business/README.md)
- [Architecture](docs/architecture/README.md)
- [Process](docs/process/README.md)
- [Decisions](docs/decisions/README.md)

## License

Ordo is licensed under [AGPL-3.0-only](LICENSE).

The license supports the sovereignty goal: users should be able to inspect,
modify, host, and leave with their system. Hosted modifications should remain
part of the commons.
