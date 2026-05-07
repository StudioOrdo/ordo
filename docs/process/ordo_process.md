# Ordo Operating Process

This document defines the execution process used to build and run Ordo.

## Core Claim

AI leverage is determined by process quality, not prompt cleverness.

## Delivery Loop

Collect -> Decide -> Spec -> QA -> Ground -> Phase -> Implement -> QA ->
Functional review -> Update

## GitHub Manufacturing Loop

The public open-source project uses GitHub as the visible work ledger.

The operating loop becomes:

```text
Collect evidence -> File issue -> Triage -> Accept scope -> Implement branch -> Pull request -> QA evidence -> Functional review -> Merge -> Release evidence
```

Markdown remains the canon for product doctrine, architecture contracts, and
deep evidence. GitHub issues and pull requests carry active work.

## Rules

1. No edit before diagnosis.
2. Specs are contracts.
3. Keep phases small and independently reviewable.
4. Validate at code, test, integration, and functional levels.
5. Preserve evidence and provenance for every meaningful change.
6. Use issues for visible intake and accepted work.
7. Use pull requests for implementation evidence and review.
8. Do not close accepted work without tests, evidence, and functional review.

## Public Work Rules

1. A GitHub issue is the public manufacturing unit after it is accepted.
2. Accepted issues should name the goal, evidence, non-goals, acceptance
   criteria, tests, and closeout evidence.
3. Pull requests should link the issue and include files changed, tests run, QA
   findings, visual evidence when relevant, and remaining risks.
4. Humans keep final authority over acceptance, merge, and release.
5. Do not claim automatic GitHub issue filing or automatic resolution until that
   path is implemented and validated.