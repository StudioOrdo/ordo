# Reusable Process Prompts

Status: prompt library

These prompts are short wrappers around the process docs:

- [Research Batch](research-batch.md)
- [Choose Workflow](choose-workflow.md)
- [Execute Issue](execute-issue.md)
- [QA Issue](qa-issue.md)
- [Land Issue](land-issue.md)
- [Triage Issue Queue](triage-issue-queue.md)

They should stay smaller than older stacked prompts. Put durable rules in
process and architecture docs, then reference those docs from prompts.

## Recommended Stack

Use this sequence when pushing through a milestone:

```text
Research Batch
-> Execute Issue
-> QA Issue
-> Land Issue
-> Execute Issue
-> QA Issue
-> Land Issue
```

Repeat Execute/QA/Land until the Batch Execution Manifest is complete, stale,
or blocked. Run Research Batch again after a product/doc shift, stale issue
cleanup need, completed batch, or landing blocker.

Use [Choose Workflow](choose-workflow.md) when the next step is unclear.

Current 0.1.9 focus areas include workflow templates, typed variables/fanout,
generic provider/tool contracts, image generation/review artifacts, generated
content memory, content analytics, Story Pack workflow declarations, and the
scrollytelling publish/analytics loop.
