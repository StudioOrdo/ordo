# Capability Catalog

Status: Implemented seed with 0.1.1 MCP policy tiers

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
- MCP export policy;
- side effects;
- approval requirement.

## Implemented Shape

The Rust daemon owns the first durable catalog in SQLite:

- `capabilities` stores stable capability definitions, schemas, execution
  targets, artifact hints, scheduler eligibility, prompt exposure, MCP export
  policy, side effects, and approval requirements;
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

## Catalog Seed

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
- `tools/list` for capabilities exported by the current MCP policy tiers;
- `tools/call` for exported tools backed by existing daemon functions.

The MCP projection validates JSON-RPC 2.0 request shape before dispatch. Parse
errors, invalid request shapes, unknown methods, and invalid parameters map to
the standard JSON-RPC error codes. `tools/call` validates argument objects
against the capability catalog input schema before running the tool-specific
daemon path, while deeper domain validation remains in the existing Rust kernel
functions.

MCP tool calls do not run arbitrary code. Mutating tools such as
`brief.system.generate` and `backup.create` call the same governed Rust kernel
paths used by HTTP and CLI entrypoints, with origin set to `mcp`.

The 0.1.1 MCP export policy tiers are:

| Policy | Meaning | Exported |
| --- | --- | --- |
| `read_only` | Reads status, catalog, brief, or backup/restore state without writing appliance state. | Yes |
| `local_mutation` | Creates governed local jobs or artifacts and requires the daemon access boundary from the runtime contract. | Yes |
| `operator_confirmed` | Writes local state only after explicit operator input, such as restore preflight confirmation. | Yes |
| `dangerous_none` | Internal, destructive, or not-yet-reviewed operations that must not be projected through MCP. | No |

`/mcp tools/list` includes Ordo metadata for each exported tool:

- `mcpExportPolicy`;
- `sideEffects`;
- `approvalRequirement`.

`backup.create` and `brief.system.generate` are `local_mutation`, not
unqualified public-safe exports. `restore.preflight.validate` is
`operator_confirmed`. Restore execution remains `dangerous_none` and is not
exported through MCP.
