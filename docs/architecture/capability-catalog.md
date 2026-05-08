# Capability Catalog

Status: Implemented seed for Ordo 0.1.0

The capability catalog is the source of truth for what Ordo can do.

MCP is a projection of the catalog, not the spine of the product. Chat, UI,
scheduler, process templates, and local daemon execution are also projections.

## Capability Definition

Each capability should define:

- stable name;
- label and plain-language description;
- family, such as system, brief, backup, restore, media, content, relationship;
- input schema;
- output contract or output hint;
- roles allowed to request, execute, inspect, and administer it;
- execution target, such as Rust, TypeScript, browser, native process, or MCP sidecar;
- timeout and retry policy;
- artifact kinds it can produce;
- scheduler eligibility;
- prompt exposure policy;
- MCP export policy.

## Implemented Shape

The Rust daemon owns the first durable catalog in SQLite:

- `capabilities` stores stable capability definitions, schemas, execution
  targets, artifact hints, scheduler eligibility, prompt exposure, and MCP
  export policy;
- `process_templates.capability_id`, `jobs.capability_id`, and
  `job_tasks.capability_id` copy the governed capability binding into each run;
- built-in templates are validated against the catalog during seed;
- job creation validates the template and task capability IDs before inserting
  the job DAG.

The daemon exposes the catalog at `/capabilities` and through the CLI command
`list-capabilities-json`.

## Registry Rule

Process templates may only reference registered task kinds. Ordo must not run
arbitrary code from user-authored JSON.

Users may copy, edit, reuse, and schedule processes, but each task kind remains
governed by catalog schema, permissions, and executor binding.

## 0.1.0 Catalog Seed

The first catalog should be small:

- system status and appliance runtime status read tools;
- System Brief read and generation capabilities;
- backup listing and creation capabilities;
- restore preflight validation capability;
- all task kinds currently used by `system.health.check`,
  `brief.system.generate`, `backup.create`, and `restore.execute` process
  templates.

The exact implementation may continue to collapse or split task kinds, but the
architecture keeps the catalog as the authority.

## MCP Projection

The daemon exposes a small JSON-RPC MCP projection at `/mcp`. It supports:

- `initialize`;
- `ping`;
- `tools/list` for capabilities with `mcp_export_policy = safe_system_tool`;
- `tools/call` for safe system tools backed by existing daemon functions.

MCP tool calls do not run arbitrary code. Mutating tools such as
`brief.system.generate` and `backup.create` call the same governed Rust kernel
paths used by HTTP and CLI entrypoints, with origin set to `mcp`.
