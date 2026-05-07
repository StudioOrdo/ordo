# 0001 Build In Public

Date: 2026-05-07

## Status

Accepted

## Context

Ordo is a process-driven product. It should demonstrate its own operating
principles while it is being built.

## Decision

The Studio Ordo repository will use GitHub issues and pull requests as the
visible work ledger.

Markdown docs will hold durable doctrine. GitHub issues will hold active work.
Pull requests will hold implementation evidence.

## Consequences

- Work should begin with an issue before implementation.
- Pull requests should include checks, evidence, risks, and follow-up.
- Private drafts belong in ignored underscore folders under `docs/`.
- Public claims must distinguish current behavior from planned behavior.

## Reconsider If

The process becomes too heavy for small changes or fails to produce useful
evidence for review.