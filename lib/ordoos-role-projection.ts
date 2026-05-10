import { type ProductRole } from "@/lib/product-navigation";
import {
  createUiError,
  type EvidenceRef,
  type RoleVisibility,
  type UiError,
} from "@/lib/ordoos-frontend-contracts";

export type ProjectionRoleGroup =
  | "public"
  | "client"
  | "affiliate"
  | "staff"
  | "manager_admin"
  | "owner_system";

export type ProjectionDecision = "allow" | "deny" | "gated" | "scoped";

export type ProjectionCategory =
  | "message_text"
  | "raw_prompt"
  | "provider_payload"
  | "policy_internals"
  | "privacy_placeholder_map"
  | "staff_notes"
  | "confidence_internals"
  | "token_ledger"
  | "staff_routing_details"
  | "accounting_evidence"
  | "artifact_metadata"
  | "browser_candidate_output"
  | "durable_daemon_evidence";

export type AgentScreenObjectKind =
  | "conversation"
  | "message"
  | "brief"
  | "artifact"
  | "action"
  | "candidate"
  | "system";

export type AgentActionKind = "command" | "patch_proposal" | "navigate" | "inspect";

export type AgentEditableFieldKind = "plain_text" | "markdown" | "enum" | "boolean" | "number";

export interface ProjectionPolicyEntry {
  category: ProjectionCategory;
  decisions: Record<ProjectionRoleGroup, ProjectionDecision>;
}

export interface ProjectionSourceValue {
  category: ProjectionCategory | string;
  value: unknown;
  evidenceRefs?: readonly EvidenceRef[];
  durability?: "candidate" | "durable";
  requiredCapability?: string;
  scopeOwnerId?: string;
  summary?: string;
}

export interface ProjectedValue {
  category: ProjectionCategory;
  value: unknown;
  evidenceRefs: readonly EvidenceRef[];
  durability: "candidate" | "durable";
  summary?: string;
}

export interface DeniedProjection {
  category: string;
  reason: string;
  decision: "deny" | "gated" | "scoped" | "unknown";
  error: UiError;
}

export interface RoleProjectionResult {
  role: ProductRole | "system";
  group: ProjectionRoleGroup;
  visible: readonly ProjectedValue[];
  denied: readonly DeniedProjection[];
}

export interface AgentPatchTarget {
  kind: "command" | "json_patch";
  commandKind?: string;
  objectId: string;
  path: string;
}

export interface AgentEditableField {
  fieldId: string;
  label: string;
  kind: AgentEditableFieldKind;
  currentValue: string | number | boolean;
  target: AgentPatchTarget;
  validation: {
    required: boolean;
    maxLength?: number;
    allowedValues?: readonly string[];
  };
}

export interface AgentScreenAction {
  actionId: string;
  label: string;
  kind: AgentActionKind;
  commandKind?: string;
  targetObjectId?: string;
  evidenceRefs: readonly EvidenceRef[];
  constraints: readonly string[];
}

export interface AgentScreenObject {
  objectId: string;
  kind: AgentScreenObjectKind;
  title: string;
  summary: string;
  visibility: RoleVisibility;
  durability: "candidate" | "durable";
  evidenceRefs: readonly EvidenceRef[];
  values: readonly ProjectedValue[];
  editableFields: readonly AgentEditableField[];
  actions: readonly AgentScreenAction[];
}

export interface AgentScreenContext {
  schemaVersion: "ordo.agent_screen_context.v1";
  screenId: string;
  route: string;
  surfaceKind: string;
  viewerRole: ProductRole | "system";
  objects: readonly AgentScreenObject[];
  availableActions: readonly AgentScreenAction[];
  denied: readonly DeniedProjection[];
  constraints: readonly string[];
  generatedAt: string;
}

export interface AgentScreenObjectSource {
  objectId: string;
  kind: AgentScreenObjectKind;
  title: string;
  summary: string;
  visibility: RoleVisibility;
  values: readonly ProjectionSourceValue[];
  editableFields?: readonly AgentEditableField[];
  actions?: readonly AgentScreenAction[];
}

export interface AgentScreenContextSource {
  screenId: string;
  route: string;
  surfaceKind: string;
  generatedAt: string;
  objects: readonly AgentScreenObjectSource[];
  actions?: readonly AgentScreenAction[];
  constraints?: readonly string[];
}

export const projectionPolicy: readonly ProjectionPolicyEntry[] = [
  row("message_text", ["allow", "allow", "allow", "allow", "allow", "allow"]),
  row("raw_prompt", ["deny", "deny", "deny", "deny", "gated", "gated"]),
  row("provider_payload", ["deny", "deny", "deny", "deny", "gated", "gated"]),
  row("policy_internals", ["deny", "deny", "deny", "gated", "gated", "allow"]),
  row("privacy_placeholder_map", ["deny", "deny", "deny", "deny", "gated", "allow"]),
  row("staff_notes", ["deny", "deny", "deny", "allow", "allow", "allow"]),
  row("confidence_internals", ["deny", "deny", "deny", "gated", "gated", "allow"]),
  row("token_ledger", ["deny", "deny", "deny", "deny", "gated", "allow"]),
  row("staff_routing_details", ["deny", "deny", "deny", "allow", "allow", "allow"]),
  row("accounting_evidence", ["deny", "deny", "deny", "gated", "gated", "allow"]),
  row("artifact_metadata", ["allow", "allow", "scoped", "allow", "allow", "allow"]),
  row("browser_candidate_output", ["deny", "deny", "deny", "gated", "gated", "allow"]),
  row("durable_daemon_evidence", ["allow", "allow", "scoped", "allow", "allow", "allow"]),
];

const projectionCategories = new Set(projectionPolicy.map((entry) => entry.category));

export function roleGroupFor(role: ProductRole | "system"): ProjectionRoleGroup {
  if (role === "anonymous") {
    return "public";
  }
  if (role === "client" || role === "member") {
    return "client";
  }
  if (role === "affiliate") {
    return "affiliate";
  }
  if (role === "staff") {
    return "staff";
  }
  if (role === "manager" || role === "admin") {
    return "manager_admin";
  }
  return "owner_system";
}

export function projectionDecision(
  role: ProductRole | "system",
  category: ProjectionCategory | string,
): ProjectionDecision | "unknown" {
  if (!isProjectionCategory(category)) {
    return "unknown";
  }
  const entry = projectionPolicy.find((candidate) => candidate.category === category);
  return entry?.decisions[roleGroupFor(role)] ?? "unknown";
}

export function projectValuesForRole(
  role: ProductRole | "system",
  values: readonly ProjectionSourceValue[],
  options: { scopedOwnerId?: string; capabilities?: readonly string[] } = {},
): RoleProjectionResult {
  const group = roleGroupFor(role);
  const visible: ProjectedValue[] = [];
  const denied: DeniedProjection[] = [];
  const capabilitySet = new Set(options.capabilities ?? []);

  for (const source of values) {
    const decision = projectionDecision(role, source.category);
    if (!isProjectionCategory(source.category) || decision === "unknown") {
      denied.push(deniedProjection(String(source.category), "Unknown projection category.", "unknown"));
      continue;
    }

    if (decision === "deny") {
      denied.push(deniedProjection(source.category, "Category is not visible to this role.", "deny"));
      continue;
    }

    if (decision === "scoped" && source.scopeOwnerId && source.scopeOwnerId !== options.scopedOwnerId) {
      denied.push(deniedProjection(source.category, "Category requires matching scoped ownership.", "scoped"));
      continue;
    }

    if (decision === "gated" && (!source.requiredCapability || !capabilitySet.has(source.requiredCapability))) {
      denied.push(deniedProjection(source.category, "Category requires an explicit role capability.", "gated"));
      continue;
    }

    visible.push({
      category: source.category,
      value: source.value,
      evidenceRefs: source.evidenceRefs ?? [],
      durability: source.durability ?? "durable",
      summary: source.summary,
    });
  }

  return { role, group, visible, denied };
}

export function buildAgentScreenContext(
  source: AgentScreenContextSource,
  role: ProductRole | "system",
  options: { scopedOwnerId?: string; capabilities?: readonly string[] } = {},
): AgentScreenContext {
  const denied: DeniedProjection[] = [];
  const objects = source.objects.map((object) => {
    const projected = projectValuesForRole(role, object.values, options);
    denied.push(...projected.denied);
    const evidenceRefs = uniqueEvidenceRefs(projected.visible.flatMap((value) => value.evidenceRefs));
    return {
      objectId: object.objectId,
      kind: object.kind,
      title: object.title,
      summary: object.summary,
      visibility: object.visibility,
      durability: projected.visible.some((value) => value.durability === "candidate") ? "candidate" : "durable",
      evidenceRefs,
      values: projected.visible,
      editableFields: object.editableFields ?? [],
      actions: object.actions ?? [],
    } satisfies AgentScreenObject;
  });

  return {
    schemaVersion: "ordo.agent_screen_context.v1",
    screenId: source.screenId,
    route: source.route,
    surfaceKind: source.surfaceKind,
    viewerRole: role,
    objects,
    availableActions: source.actions ?? [],
    denied,
    constraints: source.constraints ?? [],
    generatedAt: source.generatedAt,
  };
}

export function stableAgentScreenContextJson(context: AgentScreenContext): string {
  return JSON.stringify(context);
}

function row(
  category: ProjectionCategory,
  decisions: readonly [
    ProjectionDecision,
    ProjectionDecision,
    ProjectionDecision,
    ProjectionDecision,
    ProjectionDecision,
    ProjectionDecision,
  ],
): ProjectionPolicyEntry {
  const [publicDecision, client, affiliate, staff, managerAdmin, ownerSystem] = decisions;
  return {
    category,
    decisions: {
      public: publicDecision,
      client,
      affiliate,
      staff,
      manager_admin: managerAdmin,
      owner_system: ownerSystem,
    },
  };
}

function isProjectionCategory(category: string): category is ProjectionCategory {
  return projectionCategories.has(category as ProjectionCategory);
}

function deniedProjection(
  category: string,
  reason: string,
  decision: DeniedProjection["decision"],
): DeniedProjection {
  return {
    category,
    reason,
    decision,
    error: createUiError("permission_denied", reason),
  };
}

function uniqueEvidenceRefs(evidenceRefs: readonly EvidenceRef[]): readonly EvidenceRef[] {
  const seen = new Set<string>();
  const unique: EvidenceRef[] = [];
  for (const ref of evidenceRefs) {
    if (!seen.has(ref.id)) {
      unique.push(ref);
      seen.add(ref.id);
    }
  }
  return unique;
}
