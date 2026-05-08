# Capability Catalog

Status: Draft contract for Ordo 0.1.0

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

## Registry Rule

Process templates may only reference registered task kinds. Ordo must not run
arbitrary code from user-authored JSON.

Users may copy, edit, reuse, and schedule processes, but each task kind remains
governed by catalog schema, permissions, and executor binding.

## 0.1.0 Catalog Seed

The first catalog should be small:

- `system.health.check`;
- `brief.evidence.collect`;
- `brief.system.generate`;
- `brief.claims.validate`;
- `backup.boundary.check`;
- `backup.archive.write`;
- `backup.integrity.verify`;
- `restore.archive.verify`;
- `restore.execute`;
- `system.next.restart`.

The exact implementation may collapse or split these task kinds, but the
architecture must keep the catalog as the authority.