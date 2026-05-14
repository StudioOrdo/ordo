# Production Engine

## Operating Shape

The operator stays in one continuous conversation.

When work is requested, Ordo routes it through governed capabilities. If
execution is heavy, it is delegated to durable background or native paths and
returned to the same thread with status, artifacts, and evidence.

The user experience is not "fill out a workflow form." The user directs,
reviews, gives feedback, and approves. Ordo compiles the work into governed
plans, tasks, requests, and artifacts behind the conversation.

Studio is the production surface for this loop. It should expose plans, DAGs,
variables, artifacts, requests, approvals, and evidence as conversationally
reviewable work, not as a form-first workflow builder.

## Production Loop

The primary loop is:

1. intent or brief;
2. context and evidence;
3. planning and execution;
4. artifact delivery and QA;
5. revision and release;
6. follow-up and learning.

## Durable Value

The engine is not just asset generation. It is operational continuity:

- memory of goals and commitments;
- visible execution state;
- reusable artifacts;
- governed quality gates;
- measured outcomes and learning.

## Production DAG Example

A creator workflow should be representable as a reusable job template:

1. research online through approved tools;
2. read and summarize research evidence;
3. write a draft;
4. review the draft against a prompt and rubric;
5. revise the draft;
6. create a script;
7. review the script;
8. revise the script;
9. generate audio;
10. generate image prompts and images at the pacing selected by the model;
11. combine audio, images, captions, and metadata in browser/WASM or native
    media execution;
12. request human or automatic approval;
13. publish or stage for publication;
14. collect analytics such as downloads, access, plays, percent watched, and
    conversion events;
15. create a performance report and recommend the next run.

Independent tasks should run in parallel when dependencies, policy, and budget
allow. Review, approval, consent, and missing-information steps should become
Requests, not hidden blocking states.

The same template should be copyable with new variables. A request like
"make a 12 episode short sequence on the zodiac" should bind topic variables,
content scopes, style rules, approval gates, and growth metrics without making
the user manually wire the DAG.

## Execution Boundary

Production tools can be diverse:

- LLM providers;
- web research tools;
- browser/WASM media workers;
- MCP servers;
- Rust executors;
- native Mac and AVFoundation tools;
- future peer Ordos.

They must return through the same result envelope with artifact refs, evidence
refs, metrics, limitations, and policy decision refs. The executor can vary.
The provenance contract should not.
