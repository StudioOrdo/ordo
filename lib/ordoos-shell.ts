import { isAdminRole, isStaffRole, type ProductRole } from "@/lib/product-navigation";

export type OrdoSurfaceKind = "chat_reference" | "system" | "staff_cockpit" | "placeholder";

export type OrdoShellSlotKind =
  | "center_stage"
  | "composer"
  | "evidence_action_rail"
  | "active_work_strip"
  | "experience_menu";

export interface OrdoSurfaceDefinition {
  id: string;
  kind: OrdoSurfaceKind;
  route: string;
  label: string;
  description: string;
  allowedRoles: readonly ProductRole[];
  defaultForRootRoles: readonly ProductRole[];
  slots: readonly OrdoShellSlotKind[];
}

export interface OrdoResolvedSurface {
  surface: OrdoSurfaceDefinition;
  fallback: boolean;
  deniedReason?: string;
}

export interface OrdoShellSlotState {
  kind: OrdoShellSlotKind;
  enabled: boolean;
  label: string;
}

export interface OrdoShellComposition {
  role: ProductRole;
  surface: OrdoSurfaceDefinition;
  slots: readonly OrdoShellSlotState[];
  showStaffNavigation: boolean;
  showSystemNavigation: boolean;
}

const allRoles: readonly ProductRole[] = [
  "anonymous",
  "client",
  "member",
  "affiliate",
  "staff",
  "manager",
  "owner",
  "admin",
];

export const ordoShellSurfaces: readonly OrdoSurfaceDefinition[] = [
  {
    id: "chat",
    kind: "chat_reference",
    route: "/chat",
    label: "Ordo",
    description: "Ordo-first operating surface for relationship work.",
    allowedRoles: allRoles,
    defaultForRootRoles: ["anonymous", "client", "member", "affiliate", "staff", "manager"],
    slots: ["center_stage", "composer", "evidence_action_rail", "active_work_strip", "experience_menu"],
  },
  {
    id: "system",
    kind: "system",
    route: "/",
    label: "System",
    description: "Owner/system appliance brief and operational evidence.",
    allowedRoles: ["owner", "admin"],
    defaultForRootRoles: ["owner", "admin"],
    slots: ["center_stage", "evidence_action_rail", "active_work_strip", "experience_menu"],
  },
  {
    id: "staff-cockpit",
    kind: "staff_cockpit",
    route: "/conversations",
    label: "Staff Cockpit",
    description: "Staff-visible queue and handoff work surface.",
    allowedRoles: ["staff", "manager", "owner", "admin"],
    defaultForRootRoles: [],
    slots: ["center_stage", "composer", "evidence_action_rail", "active_work_strip", "experience_menu"],
  },
  {
    id: "placeholder",
    kind: "placeholder",
    route: "/",
    label: "Unavailable Surface",
    description: "Explicit fallback for unknown or denied surfaces.",
    allowedRoles: allRoles,
    defaultForRootRoles: [],
    slots: ["center_stage", "experience_menu"],
  },
];

export function resolveOrdoSurface(surfaceId: string, role: ProductRole): OrdoResolvedSurface {
  const surface = ordoShellSurfaces.find((candidate) => candidate.id === surfaceId);
  if (!surface) {
    return { surface: placeholderSurface(), fallback: true, deniedReason: "unknown_surface" };
  }
  if (!surface.allowedRoles.includes(role)) {
    return { surface: placeholderSurface(), fallback: true, deniedReason: "role_not_allowed" };
  }
  return { surface, fallback: false };
}

export function resolveRootSurfaceForRole(role: ProductRole): OrdoSurfaceDefinition {
  return (
    ordoShellSurfaces.find((surface) => surface.defaultForRootRoles.includes(role)) ??
    placeholderSurface()
  );
}

export function composeOrdoShell(role: ProductRole, surfaceId: string): OrdoShellComposition {
  const resolved = resolveOrdoSurface(surfaceId, role);
  return {
    role,
    surface: resolved.surface,
    slots: allShellSlots().map((slot) => ({
      kind: slot,
      enabled: resolved.surface.slots.includes(slot),
      label: labelForSlot(slot),
    })),
    showStaffNavigation: isStaffRole(role),
    showSystemNavigation: isAdminRole(role),
  };
}

export function allShellSlots(): readonly OrdoShellSlotKind[] {
  return ["center_stage", "composer", "evidence_action_rail", "active_work_strip", "experience_menu"];
}

export function isSlotEnabled(composition: OrdoShellComposition, slot: OrdoShellSlotKind): boolean {
  return composition.slots.some((candidate) => candidate.kind === slot && candidate.enabled);
}

function placeholderSurface(): OrdoSurfaceDefinition {
  return ordoShellSurfaces.find((surface) => surface.id === "placeholder") ?? ordoShellSurfaces[0]!;
}

function labelForSlot(slot: OrdoShellSlotKind): string {
  switch (slot) {
    case "center_stage":
      return "Center stage";
    case "composer":
      return "Composer";
    case "evidence_action_rail":
      return "Evidence and actions";
    case "active_work_strip":
      return "Active work";
    case "experience_menu":
      return "Experience";
  }
}
