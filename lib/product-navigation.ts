export type ProductRole = "anonymous" | "client" | "member" | "affiliate" | "staff" | "manager" | "owner" | "admin";

export interface ProductNavItem {
  id: string;
  label: string;
  href: string;
  description: string;
}

export const topRailItems: readonly ProductNavItem[] = [
  { id: "chat", label: "Chat", href: "/chat", description: "Relationship conversation." },
  { id: "home", label: "Home", href: "/home", description: "Business story and current context." },
  { id: "offers", label: "Offers", href: "/offers", description: "Ways to buy from Studio Ordo." },
  { id: "asks", label: "Asks", href: "/asks", description: "Ways to help, refer, sell, or contribute." },
  { id: "latest", label: "Latest", href: "/latest", description: "Recent public and member updates." },
  { id: "account", label: "Account", href: "/account", description: "Your role-specific tools." },
];

export const businessStaffRailItems: readonly ProductNavItem[] = [
  { id: "today", label: "Today", href: "/today", description: "Brief of what needs attention." },
  { id: "conversations", label: "Conversations", href: "/conversations", description: "Handoffs and active customer work." },
  { id: "connections", label: "Connections", href: "/connections", description: "Durable relationship memory." },
  { id: "offers", label: "Offers", href: "/offers", description: "Offer performance and changes." },
  { id: "asks", label: "Asks", href: "/asks", description: "Ask performance and responses." },
  { id: "affiliates", label: "Affiliates", href: "/affiliates", description: "Referral partners and materials." },
  { id: "artifacts", label: "Artifacts", href: "/artifacts", description: "Briefs, reports, exports, and deliverables." },
  { id: "jobs", label: "Jobs", href: "/jobs", description: "Production work and refresh status." },
  { id: "reports", label: "Reports", href: "/reports", description: "Issue reports and evidence packages." },
];

export const adminSystemRailItems: readonly ProductNavItem[] = [
  { id: "system", label: "System", href: "/", description: "Appliance brief and status." },
  { id: "knowledge", label: "Knowledge", href: "/knowledge", description: "Corpus and memory operations." },
  { id: "events", label: "Events", href: "/events", description: "Persisted event evidence." },
  { id: "logs", label: "Logs", href: "/logs", description: "Structured diagnostic observations." },
  { id: "backup", label: "Backup", href: "/backup-restore", description: "Backup and restore jobs." },
  { id: "settings", label: "Settings", href: "/preferences", description: "Appliance preferences." },
];

export function resolveProductRole(rawRole: string | string[] | undefined): ProductRole {
  const role = Array.isArray(rawRole) ? rawRole[0] : rawRole;
  if (
    role === "client" ||
    role === "member" ||
    role === "affiliate" ||
    role === "staff" ||
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

export function roleLabel(role: ProductRole): string {
  switch (role) {
    case "anonymous":
      return "Visitor";
    case "client":
      return "Client";
    case "member":
      return "Member";
    case "affiliate":
      return "Affiliate";
    case "staff":
      return "Staff";
    case "manager":
      return "Manager";
    case "owner":
      return "Owner";
    case "admin":
      return "Admin";
  }
}

export function roleHref(href: string, role: ProductRole): string {
  if (role === "anonymous") {
    return href;
  }
  const separator = href.includes("?") ? "&" : "?";
  return `${href}${separator}role=${role}`;
}
