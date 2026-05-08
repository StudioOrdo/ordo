# Resource, Provenance, And Policy Spine

Status: Implemented pre-RBAC foundation slice

This spine gives Ordo a small shared vocabulary for accountable local work
before full authentication and RBAC exist.

It does not implement durable users, roles, memberships, grants, or auth UI.
Those remain future RBAC work. The current slice prepares the system by making
protected work describable as actor, action, resource, capability, decision, and
provenance.

## Current Shape

The Rust daemon has a policy module with:

- actor kinds such as browser operator and MCP client;
- resource references such as daemon route, capability, job, job artifact,
  brief artifact, issue report, diagnostic log, and process template;
- policy actions such as read, inspect, execute, generate, create, validate,
  prepare, and call tool;
- policy outcomes: allowed, denied, and review required;
- resource classification vocabulary for visibility, purpose, execution, data
  handling, and approval state.

Protected daemon routes now pass through the shared policy decision point. The
current rule preserves the existing local trust boundary: protected mutations are
allowed when the request comes from loopback or includes the configured daemon
access token.

The MCP projection also records a policy decision for tool calls. Capabilities
with the `dangerous_none` MCP export policy remain denied from MCP. Operator
confirmed capabilities surface as review-required decisions while preserving the
current governed projection behavior.

## Provenance

Current local issue report artifacts now carry policy/provenance metadata in
their job artifact metadata. This metadata identifies:

- actor;
- action;
- resource kind and id;
- producing capability;
- producing job;
- process template;
- local high-trust classification.

This gives the appliance the first durable foothold for answering who did what,
through which governed path, and what artifact or evidence proves it.

## High-Trust Boundary

The current vocabulary is intentionally small but shaped for legal, medical,
finance, tax, tutoring, and other high-trust future workflows.

The shipped classification vocabulary includes:

- visibility tiers;
- purpose tiers;
- execution tiers;
- data handling tiers;
- approval states.

The current implementation uses this vocabulary for local operational reports.
It does not yet enforce full per-user or per-resource access controls.

## Relationship To RBAC

RBAC should build on this spine instead of scattering permission checks across
routes.

Future RBAC work should add durable actors, roles, memberships, resource grants,
and retrieval access checks behind the shared policy decision point.

## Non-Goals

- No full RBAC.
- No authentication UI.
- No external report submission transport.
- No legal, medical, finance, or tax product mode.
- No RAG/vector memory.
- No third-party plugin system.
- No broad Job Kernel V2 implementation.
