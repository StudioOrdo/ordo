export interface SystemMenuItem {
  id: string;
  label: string;
  description: string;
  href: string;
}

export const systemMenuItems: readonly SystemMenuItem[] = [
  {
    id: "brief",
    label: "Brief",
    description: "Latest system staff report.",
    href: "/",
  },
  {
    id: "health",
    label: "Health",
    description: "Daemon and readiness checks.",
    href: "/health",
  },
  {
    id: "backup-restore",
    label: "Backup & Restore",
    description: "Safety jobs and recovery state.",
    href: "/backup-restore",
  },
  {
    id: "schedules",
    label: "Schedules",
    description: "Due work created by the appliance clock.",
    href: "/schedules",
  },
  {
    id: "preferences",
    label: "Preferences",
    description: "System settings and operator defaults.",
    href: "/preferences",
  },
  {
    id: "events",
    label: "Events",
    description: "Realtime and persisted evidence trail.",
    href: "/events",
  },
  {
    id: "logs",
    label: "Logs",
    description: "Structured diagnostic observations.",
    href: "/logs",
  },
  {
    id: "reports",
    label: "Reports",
    description: "Issue reports and diagnostic packages.",
    href: "/reports",
  },
];
