# Test Plan Issue Template

Status: required template for linked `Test Plan:` issues

Every implementation issue that changes behavior should have a linked test-plan
issue. The test-plan issue is the evidence contract for proving the
implementation, not a loose checklist.

```md
# Test Plan: <implementation issue title>

Implementation issue: #<number>
Architecture docs:
- <doc path>

## Scope

What behavior must be proven:

What is explicitly out of scope:

## Acceptance Coverage

- [ ] Every implementation acceptance criterion maps to at least one scenario
      or validation command below.
- [ ] Any deferred criterion has a specific reason and follow-up issue.

## Positive Scenario

Given:
When:
Then:
Evidence expected:

## Negative Scenario

Given:
When:
Then:
Evidence expected:

## Edge Scenario

Given:
When:
Then:
Evidence expected:

## Privacy And Security Scenario

Required for public/member, provider, prompt, access, artifact, graph,
reward, support, handoff, pack, or admin/system changes.

Given:
When:
Then:
Evidence expected:

Must verify:
- no staff routing leaks;
- no provider internals leak;
- no prompt or raw policy internals leak;
- no owner-only data or private artifact text leaks;
- unsupported claims fail closed.

## Idempotency, Retry, Or Rollback Scenario

Required when touching jobs, DAGs, events, imports, providers, rewards,
artifacts, graph projection, schedules, requests, handoffs, or publishing.

Given:
When:
Then:
Evidence expected:

If not relevant, explain why:

## Schema Or Migration Safety

Required when touching SQLite schema, migrations, canonical tables, graph
tables, projections, or pack-owned storage.

Checks:
- [ ] migration order is stable;
- [ ] fresh database initializes;
- [ ] existing data is preserved or explicitly migrated;
- [ ] downgrade/rollback limitation is documented when relevant.

If not relevant, explain why:

## Unit Tests

Expected tests:
- `<test path>`: <what it proves>

## Integration Tests

Expected tests:
- `<test path>`: <what it proves>

## E2E Or Smoke Tests

Expected tests:
- `<test path or command>`: <what it proves>

If deferred, rationale and follow-up:

## Deterministic Provider Requirements

Default validation must not require live providers, live network, hosted
capacity, real payments, real publishing, real image generation, real TTS, or
real analytics.

Required fakes/fixtures:
- provider:
- image/audio/video:
- clock/time:
- IDs:
- network:

Guarded live-provider command, if any:

## Validation Commands

Focused:

```sh
<command>
```

Broader:

```sh
<command>
```

Formatting/check:

```sh
git diff --check
```

## Coverage Closeout

To close this test-plan issue, post:

- implementation issue;
- PR URL and merge commit;
- commands run and results;
- scenarios covered;
- scenarios deferred and why;
- residual risk.
```
