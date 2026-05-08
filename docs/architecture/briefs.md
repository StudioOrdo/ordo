# Briefs

Status: Draft contract for Ordo 0.1.0

Briefs are durable staff reports. They are the default way Ordo explains what
matters before a user inspects tools or diagnostics.

## Brief Contract

A brief must include:

- stable id and version or run reference;
- visible as-of timestamp;
- section or object scope;
- concise summary bullets;
- recommended next action;
- evidence references;
- limitations when evidence is missing;
- visibility policy;
- prior version history or preserved previous artifact.

The UI should tell the user when the brief was created. Page load should read
the latest completed brief immediately. It should not block on an LLM call.

## Generation Flow

System Brief generation is a job:

```text
validate scope -> collect evidence -> build evidence manifest -> generate draft -> validate claims -> save artifact -> publish update
```

Generation may use an LLM when configured. It must have a deterministic fallback
when no model is available.

The prior successful brief must remain visible if generation fails.

## Evidence Rule

Briefs must not invent metrics or hide missing evidence. The evidence packet
and limitation notes are part of the artifact contract.