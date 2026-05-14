# Security And Rapid Response

Status: public posture and direction, not a mature security program claim

Ordo treats security as part of the appliance architecture, not a page of vague
assurances.

AI increases the speed of software creation and the speed of software abuse.
The long-term goal is proactive rapid reaction: automatic QA, deterministic
evals, local diagnostic reports, backup and restore safety, governed egress,
A2A support packets, and member-visible evidence loops.

This is a direction, not a claim that the project already has a complete
security program.

## Current Safety Foundations

The current codebase includes several foundations that matter for security and
recovery:

- SQLite as the local source of truth;
- `.data` as the durable appliance boundary;
- backup creation with manifest and checksum evidence;
- restore preflight before destructive restore work;
- vault and provider configuration foundations;
- daemon-owned provider boundaries;
- privacy egress and placeholdering foundations;
- deterministic evals that do not require live providers;
- guarded live-provider evals with explicit network and budget flags;
- local reports and support packet previews;
- capability catalog policy tiers;
- governed MCP projection instead of arbitrary code execution;
- A2A support-packet direction based on approved artifacts and receipts.

## Security Rules For Contributors And Agents

- Never commit secrets, provider keys, access tokens, `.env.local` values,
  vault files, raw private transcripts, or generated private data.
- Never print secrets in logs, docs, reports, tests, comments, or issue bodies.
- Public and member surfaces must not expose staff routing, provider internals,
  prompt internals, raw policy internals, owner-only data, private artifact
  text, or unsupported capability claims.
- Default tests and evals should be deterministic, provider-free, and
  network-free.
- Live providers require explicit guards, network intent, and budget caps.
- External submissions must be opt-in, scoped, redacted, and receipt-backed.
- Backup archives should be protected like the host and durable `.data` volume.

## Rapid Response Direction

The platform should evolve toward fast, evidence-backed response loops:

```text
local evidence
-> diagnostic report
-> approved support packet
-> Studio Ordo or maintainer review
-> receipt
-> issue or fix plan
-> patch
-> validation
-> merge-backed closeout
```

Future hosted Ordos should be able to submit bounded feedback, support packets,
health summaries, backup-ready receipts, and trial-extension requests to Studio
Ordo Prime through scoped A2A capabilities. They should not expose raw local
truth or permit remote mutation of canonical state.

## Current Gaps

The project still needs:

- a formal vulnerability reporting policy;
- production hardening review;
- broader authentication and access enforcement surfaces;
- hosted identity and tenant isolation proof;
- external support packet transport;
- automated dependency and container scanning policy;
- security-specific test plans for hosted control-plane work;
- documented incident response for hosted trials.

Until those exist, describe Ordo as a serious security-conscious project under
active development, not as a production-hardened security product.