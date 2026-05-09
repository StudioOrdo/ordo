# MCP Packs And Tool Hardening MVP

Status: backend foundation ready for PR

## Why It Matters

Customization should change the work Ordo can do without changing the trust
boundary. Domain tools and packs must register through the same governed spine.

## MVP Scope

- Define a domain pack manifest format.
- Require tool identity, capability id, input schema, output contract, side
  effects, approval requirement, and artifact expectations.
- Validate pack capabilities against policy tiers.
- Keep dangerous tools non-exported until explicit approval flows exist.
- Add install/list/disable metadata for local packs.
- Keep packs as governed metadata over existing capabilities, not executable
  plugin payloads.

## Backend Foundation

- Durable `mcp_packs` records store local pack identity, status, manifest JSON,
  provenance, and timestamps.
- Durable `mcp_pack_tools` records store tool identity, capability mapping,
  input schema, output contract, side effects, approval requirement, artifact
  expectations, MCP export policy, export status, and disabled state.
- Pack install/update validates every tool against an existing catalog
  capability before persistence.
- MCP `tools/list` and `tools/call` check durable pack metadata so disabled
  tools are hidden and blocked.
- Dangerous non-exported capabilities are recorded as blocked if referenced by
  a pack manifest.
- No pack manifest can add a new executor, shell command, remote code path, or
  provider/model transport.

## Durable Product Nouns

- Capability Pack
- Tool Manifest
- Adapter
- Approval Requirement
- Artifact Contract
- Pack Export State

## Acceptance Criteria

- A pack cannot bypass capability registration.
- Tool schemas are validated before execution.
- Side effects and export policy are visible.
- Disabling a pack removes or hides its exported tools.
- Dangerous tools remain non-exported or blocked.
- Unknown arbitrary execution capabilities are rejected.
- Protected daemon routes record policy audit evidence for pack management.

## Non-Goals

- Hosted marketplace.
- Remote untrusted code execution.
- Arbitrary shell/native/plugin execution.
- External egress or hosted registry sync.
- Provider/model orchestration.
- A2A networking.

## Validation

- Manifest validation tests.
- Capability catalog tests.
- MCP tools/list and tools/call tests.
- Protected route audit tests.
- Disabled/blocked tool tests.
