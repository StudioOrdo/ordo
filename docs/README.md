# Ordo Docs

This folder separates public doctrine from local drafts and imported reference
material.

## Start Here

1. [Project README](../README.md)
2. [Public Project Brief](public-project-brief.md)
3. [Contributing](../CONTRIBUTING.md)
4. [QA And Verification](qa-and-verification.md)
5. [Security And Rapid Response](security-and-rapid-response.md)
6. [Open Source Business Model](business/open-source-business-model.md)
7. [System Overview](system-overview.md)
8. [Developer Guide](developer-guide.md)
9. [Root LLM Instructions](../llm_instructions.md)
10. [LLM Agent Guide](llm-agent-guide.md)
11. [State Of The Project](state-of-the-project.md)
12. [Eval System](evals/README.md)
13. [Issue History](process/issue-history.md)
14. [Agent Development Workflow](process/agent-development-workflow.md)
15. [Business Canon](business/README.md)
16. [Studio Ordo Hosted Appliance MVP](business/studio-ordo-mvp.md)
17. [Hosted Ordo Control Plane](architecture/hosted-ordo-control-plane.md)
18. [Hosted Ordo Lifecycle](architecture/hosted-ordo-lifecycle.md)
19. [Current Product Canon](business/current-product-canon.md)
20. [Product Operating Model](architecture/product-operating-model.md)
21. [Product Language](architecture/product-language.md)
22. [My Ordo Attention Model](architecture/my-ordo-attention-model.md)
23. [Executor Bridge Contract](architecture/executor-bridge-contract.md)
24. [NYC Meetup Current Batch Handoff](process/nyc-meetup-current-batch-handoff.md)
25. [Workforce Substrate](business/workforce-substrate.md)
26. [Agent Execution Protocol](process/agent-execution-protocol.md)
27. [Definition Of Done](process/definition-of-done.md)
28. [Graph Kernel](architecture/graph-kernel.md)
29. [LLM Method Contracts](architecture/llm-method-contracts.md)
30. [Pack Kernel](architecture/pack-kernel.md)
31. [Workflow Template Kernel](architecture/workflow-template-kernel.md)
32. [Architecture](architecture/README.md)
33. [Process](process/README.md)
34. [Decisions](decisions/README.md)
35. [Backlog](backlog/README.md)

## Public Docs

| Area | Purpose |
| --- | --- |
| [public project brief](public-project-brief.md) | Public orientation for developers, reviewers, and early collaborators. |
| [contributing](../CONTRIBUTING.md) | Practical contribution rules, issue expectations, PR evidence, validation, and security boundaries. |
| [QA and verification](qa-and-verification.md) | Public QA invitation, evidence standards, validation matrix, visual review, and deterministic-first testing stance. |
| [security and rapid response](security-and-rapid-response.md) | Public security posture, current safety foundations, rapid response direction, and known gaps. |
| [system overview](system-overview.md) | Current implemented system map for developers, reviewers, and LLM agents. |
| [developer guide](developer-guide.md) | Local setup, Docker, commands, validation, and live eval guard usage. |
| [root LLM instructions](../llm_instructions.md) | Compact first-read orientation for external LLM tools and coding agents. |
| [LLM agent guide](llm-agent-guide.md) | Source-of-truth order, architecture assumptions, risky boundaries, and agent workflow. |
| [evals](evals/README.md) | Deterministic evals, personas, artifact packets, live guards, and finding categories. |
| [business](business/README.md) | Product thesis, business model, governance principles, and UX intent. |
| [open source business model](business/open-source-business-model.md) | AGPL appliance economics, hosted convenience, developer opportunity, and non-lock-in commitments. |
| [Studio Ordo hosted appliance MVP](business/studio-ordo-mvp.md) | Active landing target for hosted trials, control-plane lifecycle, backups, reminders, and platform direction. |
| [current product canon](business/current-product-canon.md) | Current surface-first IA, product spine, UX rules, and trust boundaries. |
| [product operating model](architecture/product-operating-model.md) | Canonical daemon-first surface and object model: My Ordo, Support, Studio, Knowledge, Growth, System, and the governing loop. |
| [product language](architecture/product-language.md) | Plain-language UI vocabulary so member surfaces do not expose architecture jargon. |
| [My Ordo attention model](architecture/my-ordo-attention-model.md) | Member-facing Activity, Requests, Offers, Capabilities, Chat, notification/action taxonomy, and non-technical clarity rules. |
| [Executor bridge contract](architecture/executor-bridge-contract.md) | Boundary for using `ordo_executor` as donor/foundry through import/export contracts rather than repo merge. |
| [NYC meetup current batch handoff](process/nyc-meetup-current-batch-handoff.md) | Current NYC meetup end-state, active support queue batch, next follow-ups, and non-goals. |
| [workforce substrate](business/workforce-substrate.md) | Canonical stance that Ordo packages governed workforces, not a pile of tools. |
| [agent execution protocol](process/agent-execution-protocol.md) | Research, Execute, QA, Land, blocker, and no-close protocol for AI-assisted implementation. |
| [definition of done](process/definition-of-done.md) | Evidence-backed completion states: local-only, QA-passed, landed, closable, and blocked. |
| [graph kernel](architecture/graph-kernel.md) | SQLite-backed graph architecture for relationship traversal, explanation, evidence, and LLM-safe graph methods. |
| [LLM method contracts](architecture/llm-method-contracts.md) | Product-shaped method naming and output rules for useful but unreliable LLMs. |
| [pack kernel](architecture/pack-kernel.md) | Core-trust and pack-workflow architecture for internal packs and future developer ecosystem. |
| [workflow template kernel](architecture/workflow-template-kernel.md) | Typed workflow variables, fanout, governed tool composition, task bindings, and approval gates for reusable pack workflows. |
| [appliance operating discipline](architecture/appliance-operating-discipline.md) | Enterprise-grade execution patterns adapted into a local AI appliance. |
| [target architecture plan](architecture/target-architecture-plan.md) | Clean/CQRS-lite implementation shape for the product canon. |
| [hosted Ordo control plane](architecture/hosted-ordo-control-plane.md) | Shared-image, multi-container, Traefik-routed hosted trial architecture. |
| [hosted Ordo lifecycle](architecture/hosted-ordo-lifecycle.md) | Commissioning, reminders, rollups, backup closeout, and decommissioning jobs. |
| [notifications and transactional email](architecture/notifications-and-transactional-email.md) | Event-driven email policy, schedules, attempts, receipts, and reminder lifecycle. |
| [A2A Studio Ordo Prime](architecture/a2a-studio-prime.md) | Launch wedge for governed feedback, support, lifecycle receipts, and future network priming. |
| [rewards and incentives](architecture/rewards-and-incentives.md) | Reusable Growth rewards, referral, feedback, benefit, and leaderboard architecture. |
| [OrdoStudio NYC pilot](business/ordostudio-nyc-pilot.md) | First wedge workflow for QR-to-trial, handoff, promo production, and rewards. |
| [architecture](architecture/README.md) | System boundaries, runtime direction, and technical decisions. |
| [process](process/README.md) | How work moves through issues, pull requests, checks, review, and release evidence. |
| [agent development workflow](process/agent-development-workflow.md) | Public Research, Execute, QA, and Land workflow for AI-assisted development. |
| [decisions](decisions/README.md) | Accepted architecture and operating decisions. |
| [backlog](backlog/README.md) | High-level MVP specs for future features and issue-ready slices. |

## Current Reader Path

For the current appliance, read:

1. [Public Project Brief](public-project-brief.md)
2. [System Overview](system-overview.md)
3. [State Of The Project](state-of-the-project.md)
4. [Developer Guide](developer-guide.md)
5. [QA And Verification](qa-and-verification.md)
6. [Eval System](evals/README.md)
7. [System Architecture](architecture/system-architecture.md)
8. [Diagnostics And Reports](architecture/diagnostics-and-reports.md)
9. [Studio Ordo Hosted Appliance MVP](business/studio-ordo-mvp.md)
10. [Hosted Ordo Control Plane](architecture/hosted-ordo-control-plane.md)
11. [Hosted Ordo Lifecycle](architecture/hosted-ordo-lifecycle.md)
12. [Current Product Canon](business/current-product-canon.md)
13. [Workforce Substrate](business/workforce-substrate.md)
14. [Appliance Operating Discipline](architecture/appliance-operating-discipline.md)
15. [Target Architecture Plan](architecture/target-architecture-plan.md)
16. [Rewards And Incentives](architecture/rewards-and-incentives.md)
17. [OrdoStudio NYC Pilot](business/ordostudio-nyc-pilot.md)
18. [Product Shape](business/product-shape.md)
19. [Product Operating Model](architecture/product-operating-model.md)
20. [Product Language](architecture/product-language.md)
21. [My Ordo Attention Model](architecture/my-ordo-attention-model.md)
22. [Executor Bridge Contract](architecture/executor-bridge-contract.md)
23. [NYC Meetup Current Batch Handoff](process/nyc-meetup-current-batch-handoff.md)
24. [Agent Execution Protocol](process/agent-execution-protocol.md)
25. [Definition Of Done](process/definition-of-done.md)
26. [Graph Kernel](architecture/graph-kernel.md)
27. [LLM Method Contracts](architecture/llm-method-contracts.md)
28. [Pack Kernel](architecture/pack-kernel.md)
29. [Workflow Template Kernel](architecture/workflow-template-kernel.md)

For LLM agents, read:

1. [Root LLM Instructions](../llm_instructions.md)
2. [Agent Execution Protocol](process/agent-execution-protocol.md)
3. [Definition Of Done](process/definition-of-done.md)
4. [Implementation Issue Template](process/implementation-issue-template.md)
5. [Test Plan Template](process/test-plan-template.md)
6. [LLM Agent Guide](llm-agent-guide.md)
7. [Product Operating Model](architecture/product-operating-model.md)
8. [Product Language](architecture/product-language.md)
9. [NYC Meetup Current Batch Handoff](process/nyc-meetup-current-batch-handoff.md)
10. [System Overview](system-overview.md)
11. [State Of The Project](state-of-the-project.md)
12. [Eval System](evals/README.md)
13. Current source and tests for the files being changed.

For future direction, read:

1. [Public Project Brief](public-project-brief.md)
2. [Open Source Business Model](business/open-source-business-model.md)
3. [Project Philosophy](business/project-philosophy.md)
4. [Founding Thesis](business/founding-thesis.md)
5. [Sovereignty Stack](business/sovereignty-stack.md)
6. [Current Product Canon](business/current-product-canon.md)
7. [Product Operating Model](architecture/product-operating-model.md)
8. [Product Language](architecture/product-language.md)
9. [My Ordo Attention Model](architecture/my-ordo-attention-model.md)
10. [Executor Bridge Contract](architecture/executor-bridge-contract.md)
11. [Studio Ordo Hosted Appliance MVP](business/studio-ordo-mvp.md)
12. [Hosted Ordo Control Plane](architecture/hosted-ordo-control-plane.md)
13. [Hosted Ordo Lifecycle](architecture/hosted-ordo-lifecycle.md)
14. [Notifications And Transactional Email](architecture/notifications-and-transactional-email.md)
15. [A2A Studio Ordo Prime](architecture/a2a-studio-prime.md)
16. [Security And Rapid Response](security-and-rapid-response.md)
17. [Workforce Substrate](business/workforce-substrate.md)
18. [Appliance Operating Discipline](architecture/appliance-operating-discipline.md)
19. [Target Architecture Plan](architecture/target-architecture-plan.md)
20. [Graph Kernel](architecture/graph-kernel.md)
21. [LLM Method Contracts](architecture/llm-method-contracts.md)
22. [Pack Kernel](architecture/pack-kernel.md)
23. [Workflow Template Kernel](architecture/workflow-template-kernel.md)
24. [Rewards And Incentives](architecture/rewards-and-incentives.md)
25. [OrdoStudio NYC Pilot](business/ordostudio-nyc-pilot.md)
26. [Ordo Core](business/ordo-core.md)
27. [Product Roadmap](business/product-roadmap.md)
28. [Scaling With Worker Ordos](architecture/scaling-worker-ordos.md)
29. [Agent-To-Agent Roadmap](architecture/agent-to-agent-roadmap.md)
30. [Backlog](backlog/README.md)

## Local Docs Convention

Folders under `docs/` that start with `_` are private or local workspaces and
are ignored by git.

Examples of local ignored workspaces:

- `docs/_drafts/`
- `docs/_research/`
- `docs/_archive/`
- `docs/_debug/`
- `docs/_imports/`

Promote material out of an underscore folder before treating it as public canon.
Keep retired drafts in underscore folders only when historical context is still
useful. Do not link ignored archive material from public docs unless the user
explicitly asks for that history to become public again.

## Source Of Truth

When sources disagree, trust them in this order:

1. Current source code and tests.
2. [Current Product Canon](business/current-product-canon.md) for product IA,
   UX stance, and surface vocabulary.
3. [Product Operating Model](architecture/product-operating-model.md) for the
   daemon-first surface and object model.
4. [Product Language](architecture/product-language.md) for member-facing UI
   copy and plain-language trust explanations.
5. [Studio Ordo Hosted Appliance MVP](business/studio-ordo-mvp.md) for the
   active hosted trial control-plane landing target.
6. [Workforce Substrate](business/workforce-substrate.md) for pack, Studio,
   and user-experience stance.
7. [Appliance Operating Discipline](architecture/appliance-operating-discipline.md)
   for backend architecture discipline and enterprise-pattern adaptation.
8. [Target Architecture Plan](architecture/target-architecture-plan.md) for
   implementation layering, CQRS-lite flow, job kernel direction, and sequence.
9. [Hosted Ordo Control Plane](architecture/hosted-ordo-control-plane.md) and
   [Hosted Ordo Lifecycle](architecture/hosted-ordo-lifecycle.md) for hosted
   trial orchestration direction.
10. [Rewards And Incentives](architecture/rewards-and-incentives.md) for Growth
   rewards, referral, feedback, and benefit-grant architecture.
11. [Graph Kernel](architecture/graph-kernel.md), [LLM Method Contracts](architecture/llm-method-contracts.md),
   [Pack Kernel](architecture/pack-kernel.md), and
   [Workflow Template Kernel](architecture/workflow-template-kernel.md) for
   graph-native memory, product-shaped LLM access, developer pack boundaries,
   and typed workflow composition.
12. [Agent Execution Protocol](process/agent-execution-protocol.md) and
    [Definition Of Done](process/definition-of-done.md) for development
    workflow and completion claims.
13. [State Of The Project](state-of-the-project.md).
14. [System Overview](system-overview.md).
15. [Project README](../README.md).
16. Current business, architecture, process, and decision docs.
17. Local drafts and archived reference material.
