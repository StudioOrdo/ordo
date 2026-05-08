# Resource, Provenance, And Policy Spine

Status: Implemented foundation slice

This spine gives Ordo a small shared vocabulary for accountable local work.

It is now paired with the local access/RBAC foundation. Auth UI and external
identity flows remain future work. The current spine makes protected work
describable as actor, action, resource, capability, decision, and provenance.

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
current governed projection behavior. Exported MCP tool calls also pass through
durable capability role binding before the export-tier decision is accepted.

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

The current implementation uses this vocabulary for local operational reports,
durable local resource grants, and capability role decisions. It does not yet
provide authentication UI, hosted identity, public portals, or access-aware
retrieval.

## Relationship To RBAC

RBAC builds on this spine instead of scattering permission checks across routes.

The current local foundation adds durable actors, roles, memberships, and
resource grants behind the shared policy decision point. Future RBAC work should
add authentication flows, public/session actors, and retrieval access checks.

## Non-Goals

- No authentication UI.
- No external report submission transport.
- No legal, medical, finance, or tax product mode.
- No RAG/vector memory.
- No third-party plugin system.
- No broad Job Kernel V2 implementation.
