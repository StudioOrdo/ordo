# MCP Packs And Tool Hardening MVP

Status: local MCP foundation exists

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

## Durable Product Nouns

- Capability Pack
- Tool Manifest
- Adapter
- Approval Requirement
- Artifact Contract

## Acceptance Criteria

- A pack cannot bypass capability registration.
- Tool schemas are validated before execution.
- Side effects and export policy are visible.
- Disabling a pack removes or hides its exported tools.

## Non-Goals

- Hosted marketplace.
- Remote untrusted code execution.
- A2A networking.

## Validation

- Manifest validation tests.
- Capability catalog tests.
- MCP tools/list and tools/call tests.
