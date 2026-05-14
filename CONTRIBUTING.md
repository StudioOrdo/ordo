# Contributing To Ordo

Ordo is a serious AGPL open-source project for local-first organizational
intelligence. The project is early, AI-assisted, QA-heavy, and intentionally
public about its process.

The most useful contributions right now are evidence, QA, review, docs, tests,
and small implementation slices that respect the appliance architecture.

## Start Here

Read these first:

1. [README](README.md)
2. [LLM Instructions](llm_instructions.md)
3. [Public Project Brief](docs/public-project-brief.md)
4. [Developer Guide](docs/developer-guide.md)
5. [QA And Verification](docs/qa-and-verification.md)
6. [Agent Development Workflow](docs/process/agent-development-workflow.md)
7. [State Of The Project](docs/state-of-the-project.md)
8. [Current Product Canon](docs/business/current-product-canon.md)

## How Work Moves

GitHub is the public manufacturing ledger:

```text
docs -> issue -> test-plan issue -> branch -> commit -> QA -> PR -> merge
-> issue closeout -> state docs
```

Accepted work should be small, testable, and tied to evidence. Broad ideas
belong in docs or backlog first, then become issue/test-plan pairs.

## Good Issues

A good issue includes:

- goal;
- current behavior;
- expected behavior;
- evidence or reproduction steps;
- acceptance criteria;
- relevant docs or code areas;
- non-goals;
- validation expectations.

Do not include secrets, private transcripts, provider keys, `.env.local` values,
raw customer data, or unredacted backup/vault material.

## Pull Requests

Pull requests should include:

- linked issue;
- files changed;
- tests and validation run;
- screenshots or visual evidence when UI changes;
- architecture notes when boundaries are touched;
- known residual risks.

Do not broaden scope inside a PR. If new work appears, file or update an issue.

## Validation

Use validation proportional to the change. For shared behavior, run:

```bash
npm run typecheck
npm run build
npm run smoke:ui
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

For doc-only changes, `git diff --check` and link/path sanity are usually
enough.

## Security And Privacy

Treat Ordo as security-sensitive infrastructure.

- Never commit secrets or provider keys.
- Never print `.env.local` values.
- Do not include raw private transcripts, owner-only data, or vault material in
  issues, PRs, reports, or docs.
- Keep public/member surfaces separate from staff, provider, prompt, policy,
  and owner internals.
- Use deterministic validation by default.
- Use live providers only with explicit guards and budget caps.

## Architecture Rules

- SQLite owns canonical truth.
- Events own audit and replay.
- Projections/read models own surface experience.
- Rust owns durable appliance behavior, migrations, policy, provider
  boundaries, jobs, backup/restore, and realtime fanout.
- Next.js owns product UI, routes, read-model display, and interaction state.
- MCP is a governed projection over capabilities, not arbitrary code execution.

When in doubt, ask for review before crossing trust boundaries.