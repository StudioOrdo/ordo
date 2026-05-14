# QA And Verification

Status: public contributor guide for review, testing, and evidence

Ordo's main current need is QA.

AI-assisted development can create code, specs, tests, and docs quickly. That
does not make the work true. Ordo treats velocity without verification as
waste.

## What Counts As Useful QA

Useful QA makes hidden problems visible with evidence.

Good QA can include:

- reproducing a bug with steps, environment, and expected behavior;
- comparing public claims against source, tests, and state docs;
- finding privacy, visibility, policy, or egress leaks;
- testing deterministic eval artifacts;
- testing backup, restore preflight, report export, and recovery paths;
- reviewing smoke tests and visual behavior across desktop and mobile;
- checking that public/member surfaces do not expose internal state;
- validating that future features are not described as shipped;
- reviewing PR evidence against issue acceptance criteria;
- helping turn broad feedback into a small implementation issue and linked
  test-plan issue.

## Evidence Standards

A high-quality issue or QA comment should include:

- what was tested;
- exact steps or command;
- actual result;
- expected result;
- affected files, route, or surface when known;
- screenshots or logs when relevant, with secrets redacted;
- whether the problem is deterministic or intermittent;
- security/privacy impact if any;
- suggested acceptance criteria if the issue is ready for implementation.

Do not include provider keys, access tokens, private transcripts, raw customer
data, `.env.local` values, or unredacted vault/backup material.

## Validation Matrix

Use validation proportional to the change. For shared behavior, run the full
matrix:

```bash
npm run typecheck
npm run build
npm run smoke:ui
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

For small doc-only work, `git diff --check` and link/path sanity are usually
enough.

For Rust, schema, policy, provider, reward, access, job, artifact, projection,
route, or navigation changes, start with focused tests and then widen.

## Deterministic First

Default validation should be deterministic and network-free. Live provider
tests are manual, guarded, and budgeted.

The eval system should expose evidence through:

- transcript artifacts;
- event ledgers;
- database rows;
- prompt-slot ledgers;
- privacy ledgers;
- token accounting;
- handoff and mode state;
- artifact review findings;
- replay checks.

## Visual Review

Future development should be sliced so the project owner can review work
through the actual site whenever practical.

For UI-facing work, PR evidence should include:

- route or screen reviewed;
- desktop and mobile considerations;
- screenshots or smoke evidence when useful;
- degraded-state behavior;
- unsupported future claims removed or clearly labeled;
- accessibility or keyboard considerations when relevant.

## QA Is Business Learning

QA is not only bug tracking. For Ordo, QA is part of the product loop: feedback
becomes evidence, evidence becomes issues, issues become better process, and
process becomes a more trustworthy appliance.