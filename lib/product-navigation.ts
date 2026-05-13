import { activeCountForRoom, stateLabelForRoom, unreadCountForRoom, type MockActivitySpace } from "@/lib/mock-ordo-activity";
import { memberRoomWaitingCount, memberRooms } from "@/lib/member-ordo-mock";

export type ProductRole = "anonymous" | "client" | "member" | "affiliate" | "staff" | "studio" | "manager" | "owner" | "admin";
export type ProductAppSpace = "site" | "my-ordo" | "staff" | "studio" | "owner" | "admin";
export type ProductRoleFamily = "guest" | "authenticated" | "staff" | "admin";

export interface ProductUiNamingSettings {
  roleFamilyAliases: Record<ProductRoleFamily, string>;
  roleAliases: Record<ProductRole, string>;
  shellAliases: Record<ProductAppSpace, string>;
}

export type ProductBrandDisplayMode = "logo" | "title" | "logo-and-title";

export interface ProductBrandSettings {
  siteTitle: string;
  displayMode: ProductBrandDisplayMode;
}

export const defaultProductBrandSettings: ProductBrandSettings = {
  siteTitle: "Studio Ordo",
  displayMode: "logo-and-title",
};

export const defaultProductUiNamingSettings: ProductUiNamingSettings = {
  roleFamilyAliases: {
    guest: "Guest",
    authenticated: "User",
    staff: "Staff",
    admin: "Owner",
  },
  roleAliases: {
    anonymous: "Guest",
    client: "Client",
    member: "Member",
    affiliate: "Affiliate",
    staff: "Staff",
    studio: "Studio operator",
    manager: "Manager",
    owner: "Owner",
    admin: "Admin",
  },
  shellAliases: {
    site: "Site",
    "my-ordo": "Ordo",
    staff: "Support",
    studio: "Studio",
    owner: "Business",
    admin: "System",
  },
};

export interface ProductNavItem {
  id: string;
  label: string;
  href: string;
  description: string;
  stateLabel?: string;
  unreadCount?: number;
  readState?: "read" | "unread";
}

export interface ProductAppSpaceDefinition {
  id: ProductAppSpace;
  label: string;
  shortLabel: string;
  href: string;
  description: string;
  items: readonly ProductNavItem[];
}

function streamRoomItem(
  space: MockActivitySpace,
  id: string,
  label: string,
  href: string,
  description: string,
  fallbackStateLabel?: string,
): ProductNavItem {
  const unreadCount = unreadCountForRoom(space, id);
  const activeCount = activeCountForRoom(space, id);

  return {
    id,
    label,
    href,
    description,
    stateLabel: stateLabelForRoom(space, id) ?? fallbackStateLabel,
    unreadCount: unreadCount > 0 ? unreadCount : undefined,
    readState: unreadCount > 0 ? "unread" : activeCount > 0 ? "read" : undefined,
  };
}

export const siteRailItems: readonly ProductNavItem[] = [
  { id: "feed", label: "Feed", href: "/feed", description: "Public scrollytelling updates, offers, asks, and proof." },
  { id: "chat", label: "Ordo", href: "/chat", description: "Relationship conversation." },
  { id: "about", label: "About", href: "/about", description: "Business story and current context." },
];

export const authenticatedRailItems: readonly ProductNavItem[] = [
  ...memberRooms.map((room) => {
    const waitingCount = memberRoomWaitingCount(room.id);
    return {
      id: room.id,
      label: room.label,
      href: room.href,
      description: room.description,
      stateLabel: waitingCount > 0 ? `${waitingCount} waiting` : room.quietState,
      unreadCount: waitingCount > 0 ? waitingCount : undefined,
      readState: waitingCount > 0 ? "unread" : undefined,
    } satisfies ProductNavItem;
  }),
];

export const ownerRailItems: readonly ProductNavItem[] = [
  streamRoomItem("owner", "overview", "Overview", "/owner/overview", "Business performance, owner decisions, and operating signals.", "3 decisions"),
  streamRoomItem("owner", "marketing", "Marketing", "/owner/marketing", "QR events, public feed, source quality, and campaigns.", "meetup QR"),
  streamRoomItem("owner", "revenue", "Revenue", "/owner/revenue", "Trials, extensions, training, and consultation money.", "$3.4k open"),
  streamRoomItem("owner", "offers", "Offers", "/owner/offers", "Offer performance, objections, and conversion.", "trial leads"),
  streamRoomItem("owner", "affiliates", "Affiliates", "/owner/affiliates", "Affiliate outcomes, rewards, and attribution quality.", "2 pending"),
  streamRoomItem("owner", "reports", "Reports", "/owner/reports", "Business reports, journey findings, and owner follow-up drafts.", "3 drafts"),
];

export const staffRailItems: readonly ProductNavItem[] = [
  streamRoomItem("staff", "handoffs", "Handoffs", "/staff/handoffs", "Human-led customer support and delegation."),
  streamRoomItem("staff", "conversations", "Conversations", "/staff/conversations", "Handoffs and active customer work."),
  streamRoomItem("staff", "requests", "Requests", "/staff/requests", "Approvals, support asks, and customer resolutions."),
  streamRoomItem("staff", "reviews", "Reviews", "/staff/reviews", "Consent, approval, and publication workflow."),
  streamRoomItem("staff", "members", "Members", "/staff/members", "Customer, student, affiliate, and prospect relationships."),
];

export const studioRailItems: readonly ProductNavItem[] = [
  streamRoomItem("studio", "knowledge", "Knowledge", "/studio/knowledge", "Business truth and source material."),
  streamRoomItem("studio", "factory-jobs", "Jobs", "/studio/factory-jobs", "Production work that creates artifacts."),
  streamRoomItem("studio", "artifacts", "Artifacts", "/studio/artifacts", "Briefs, media, reports, and deliverables."),
  streamRoomItem("studio", "publications", "Publications", "/studio/publications", "Where approved artifacts appear."),
  streamRoomItem("studio", "templates", "Templates", "/studio/templates", "Reusable production formats."),
];

export const adminRailItems: readonly ProductNavItem[] = [
  streamRoomItem("admin", "health", "Health", "/admin/health", "Daemon and readiness checks."),
  streamRoomItem("admin", "events", "Events", "/admin/events", "Persisted event evidence."),
  streamRoomItem("admin", "access", "Access", "/admin/access", "Roles, grants, and trust boundaries."),
  streamRoomItem("admin", "providers", "Providers", "/admin/providers", "Model and integration configuration."),
  streamRoomItem("admin", "hosted-trials", "Hosted Trials", "/admin/hosted-trials", "Hosted trial capacity, waitlist, backup, and reset guards."),
  streamRoomItem("admin", "backup", "Backups", "/admin/backup", "Backup and restore jobs."),
  streamRoomItem("admin", "settings", "Settings", "/admin/settings", "Appliance preferences."),
];

export const appSpaceDefinitions: readonly ProductAppSpaceDefinition[] = [
  {
    id: "site",
    label: "Site",
    shortLabel: "Site",
    href: "/feed",
    description: "Public story, Ordo entry, and business proof.",
    items: siteRailItems,
  },
  {
    id: "my-ordo",
    label: "Ordo",
    shortLabel: "Ordo",
    href: "/my/chat",
    description: "Your Ordo conversation, activity, offers, capabilities, and requests.",
    items: authenticatedRailItems,
  },
  {
    id: "staff",
    label: "Support",
    shortLabel: "Support",
    href: "/staff/conversations",
    description: "Customer interaction, sales, handoffs, and relationship work.",
    items: staffRailItems,
  },
  {
    id: "studio",
    label: "Studio",
    shortLabel: "Studio",
    href: "/studio/knowledge",
    description: "Knowledge management and the production factory.",
    items: studioRailItems,
  },
  {
    id: "owner",
    label: "Business",
    shortLabel: "Business",
    href: "/owner/overview",
    description: "Business performance, marketing, revenue, and owner judgment.",
    items: ownerRailItems,
  },
  {
    id: "admin",
    label: "System",
    shortLabel: "System",
    href: "/admin/health",
    description: "System governance, access, events, and diagnostics.",
    items: adminRailItems,
  },
];

export const productRoles: readonly ProductRole[] = ["anonymous", "client", "member", "affiliate", "staff", "studio", "manager", "owner", "admin"];

export function appSpaceById(appSpaceId: ProductAppSpace): ProductAppSpaceDefinition {
  return appSpaceDefinitions.find((space) => space.id === appSpaceId) ?? appSpaceDefinitions[0];
}

export function resolveProductRole(rawRole: string | string[] | undefined): ProductRole {
  const role = Array.isArray(rawRole) ? rawRole[0] : rawRole;
  if (
    role === "client" ||
    role === "member" ||
    role === "affiliate" ||
    role === "staff" ||
    role === "studio" ||
    role === "manager" ||
    role === "owner" ||
    role === "admin"
  ) {
    return role;
  }
  return "anonymous";
}

export function isStaffRole(role: ProductRole): boolean {
  return role === "staff" || role === "manager" || role === "owner" || role === "admin";
}

export function isAdminRole(role: ProductRole): boolean {
  return role === "owner" || role === "admin";
}

export function isAuthenticatedRole(role: ProductRole): boolean {
  return role !== "anonymous";
}

export function roleFamilyForRole(role: ProductRole): ProductRoleFamily {
  if (role === "anonymous") {
    return "guest";
  }
  if (role === "owner" || role === "admin") {
    return "admin";
  }
  if (role === "staff" || role === "studio" || role === "manager") {
    return "staff";
  }
  return "authenticated";
}

export function canAccessAppSpace(role: ProductRole, appSpaceId: ProductAppSpace): boolean {
  if (appSpaceId === "site") {
    return true;
  }
  if (appSpaceId === "my-ordo") {
    return isAuthenticatedRole(role);
  }
  if (appSpaceId === "staff") {
    return isStaffRole(role);
  }
  if (appSpaceId === "studio") {
    return role === "studio" || role === "manager" || isAdminRole(role);
  }
  if (appSpaceId === "owner") {
    return isAdminRole(role);
  }
  return isAdminRole(role);
}

export function accessibleAppSpaces(role: ProductRole): readonly ProductAppSpaceDefinition[] {
  return appSpaceDefinitions.filter((space) => canAccessAppSpace(role, space.id));
}

export function roleFamilyLabel(role: ProductRole, settings: ProductUiNamingSettings = defaultProductUiNamingSettings): string {
  return settings.roleFamilyAliases[roleFamilyForRole(role)];
}

export function roleLabel(role: ProductRole, settings: ProductUiNamingSettings = defaultProductUiNamingSettings): string {
  return settings.roleAliases[role];
}

export function appSpaceLabel(appSpaceId: ProductAppSpace, settings: ProductUiNamingSettings = defaultProductUiNamingSettings): string {
  return settings.shellAliases[appSpaceId];
}

export function roleHref(href: string, role: ProductRole): string {
  if (role === "anonymous") {
    return href;
  }
  const separator = href.includes("?") ? "&" : "?";
  return `${href}${separator}role=${role}`;
}
