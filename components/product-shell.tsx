import Link from "next/link";
import type { ReactNode } from "react";

import { OrdoFrame } from "@/components/ordo-frame";
import type { ProductMobileStep, ProductRailMode } from "@/lib/page-role";
import {
  accessibleAppSpaces,
  appSpaceById,
  appSpaceLabel,
  canAccessAppSpace,
  roleHref,
  roleLabel,
  type ProductAppSpace,
  type ProductAppSpaceDefinition,
  type ProductRole,
} from "@/lib/product-navigation";

interface Props {
  role: ProductRole;
  appSpaceId?: ProductAppSpace;
  currentItemId: string;
  roomEvidenceRail?: ReactNode;
  collapseSectionRail?: boolean;
  railMode?: ProductRailMode;
  mobileStep?: ProductMobileStep;
  selectedItemIndex?: number;
  children: ReactNode;
}

export function ProductShell({
  role,
  appSpaceId = "site",
  currentItemId,
  roomEvidenceRail,
  collapseSectionRail = false,
  railMode = "expanded",
  mobileStep = "rooms",
  selectedItemIndex,
  children,
}: Props) {
  const resolvedAppSpaceId = canAccessAppSpace(role, appSpaceId) ? appSpaceId : "site";
  const appSpace = appSpaceById(resolvedAppSpaceId);
  const shellLabel = appSpaceLabel(appSpace.id);
  const currentItem = appSpace.items.find((item) => item.id === currentItemId);
  const currentRoute = currentItem?.href ?? appSpace.href;
  const effectiveRailMode: ProductRailMode = railMode;
  const nextRailMode: ProductRailMode = effectiveRailMode === "collapsed" ? "expanded" : "collapsed";
  const drawerOpen = effectiveRailMode !== "collapsed";
  const shellLinks = accessibleAppSpaces(role).filter((space) => space.id === "my-ordo");

  return (
    <OrdoFrame
      role={role}
      homeHref={shellHref("/", role, effectiveRailMode, "rooms")}
      topRail={<MemberTopRail role={role} />}
      railNavigation={
        <nav className="product-shell-menu" aria-label="Functional workspaces">
          {shellLinks.map((space) => {
            const active = space.id === appSpace.id;
            const unreadCount = waitingCountForSpace(space);
            const stateLabel = unreadCount > 0 ? `${unreadCount} waiting` : "ready";
            const railLabel = drawerOpen ? `Hide ${appSpaceLabel(space.id)} rooms` : `Show ${appSpaceLabel(space.id)} rooms`;
            return (
              <Link
                key={space.id}
                href={shellHref(active ? currentRoute : space.href, role, active ? nextRailMode : effectiveRailMode, mobileStep, active ? selectedItemIndex : undefined)}
                className={active ? "primary-link primary-link-active" : "primary-link"}
                aria-current={active ? "page" : undefined}
                aria-expanded={active ? drawerOpen : undefined}
                data-shell-id={space.id}
                data-rail-label={active ? railLabel : appSpaceLabel(space.id)}
                data-rail-meta={stateLabel}
              >
                <span className="primary-link-main">
                  <span className="primary-link-symbol" aria-hidden="true">
                    <ShellIcon appSpaceId={space.id} />
                  </span>
                  <span className="primary-link-label">{appSpaceLabel(space.id)}</span>
                  {unreadCount ? (
                    <span className="room-unread-count" aria-label={`${unreadCount} unread`}>
                      {unreadCount}
                    </span>
                  ) : null}
                </span>
                <span className="primary-link-meta">{stateLabel}</span>
              </Link>
            );
          })}
        </nav>
      }
      accountUtility={<ProductRailUserMenu role={role} />}
    >
      <div className="product-shell" data-role={role} data-app-space={appSpace.id} data-rail-mode={effectiveRailMode} data-mobile-step={mobileStep}>
      <div className={collapseSectionRail ? "product-body product-body-no-section" : "product-body"}>
        <aside className="product-nav-drawer" aria-label={`${shellLabel} rooms`} data-drawer-open={drawerOpen ? "true" : "false"}>
          <div className="product-drawer-heading">
            <span className="eyebrow">{shellLabel}</span>
            <h2>{appSpace.label}</h2>
            <p>{appSpace.description}</p>
          </div>
          <nav className="product-drawer-menu" aria-label={`${shellLabel} room labels`}>
            {appSpace.items.map((item) => {
              const active = item.id === currentItemId;
              return (
                <Link
                  key={item.id}
                  href={shellHref(item.href, role, effectiveRailMode, mobileStepForRoom(item.id))}
                  className={active ? "drawer-link drawer-link-active" : "drawer-link"}
                  aria-current={active ? "page" : undefined}
                >
                  <span className="drawer-link-copy">
                    <strong>{item.label}</strong>
                    <span>{item.stateLabel ?? item.description}</span>
                  </span>
                  {item.unreadCount ? (
                    <span className="section-unread-count" aria-label={`${item.unreadCount} unread`}>
                      {item.unreadCount}
                    </span>
                  ) : item.readState === "read" ? (
                    <span className="room-read-dot" aria-label="Read" />
                  ) : null}
                </Link>
              );
            })}
          </nav>
        </aside>

        {collapseSectionRail ? null : (
          <aside className="section-column product-section-column" aria-label={`${shellLabel} evidence and assets`}>
            <MobileStackBar
              eyebrow={shellLabel}
              title={currentItem?.label ?? sectionHeadingForAppSpace(appSpace.id)}
              backHref={shellHref(currentRoute, role, effectiveRailMode, "rooms", selectedItemIndex)}
              backLabel="Rooms"
              nextHref={shellHref(currentRoute, role, effectiveRailMode, "content", selectedItemIndex)}
              nextLabel="Open content"
            />
            {roomEvidenceRail ?? (
              <>
                <div className="section-heading">
                  <span className="eyebrow">{shellLabel}</span>
                  <h1>{currentItem?.label ?? sectionHeadingForAppSpace(appSpace.id)}</h1>
                  <p>{currentItem?.description ?? appSpace.description}</p>
                </div>
                <div className="system-menu" aria-label={`${shellLabel} evidence and assets`}>
                  <div className="section-link section-link-active">
                    <span>
                      <strong>Brief</strong>
                      <span>Current room context and evidence.</span>
                    </span>
                    <span className="link-dot" aria-hidden="true" />
                  </div>
                </div>
              </>
            )}
          </aside>
        )}

        <main className="main-pane product-main-pane">
          <MobileStackBar
            eyebrow={shellLabel}
            title={currentItem?.label ?? sectionHeadingForAppSpace(appSpace.id)}
            backHref={collapseSectionRail ? shellHref(currentRoute, role, effectiveRailMode, "rooms", selectedItemIndex) : shellHref(currentRoute, role, effectiveRailMode, "evidence", selectedItemIndex)}
            backLabel={collapseSectionRail ? "Rooms" : "Evidence"}
          />
          <div className="main-content product-main-content">{children}</div>
        </main>
      </div>
    </div>
    </OrdoFrame>
  );
}

function ProductRailUserMenu({ role }: { role: ProductRole }) {
  return (
    <details className="product-rail-user-menu">
      <summary aria-label="Open account and role menu">
        <span className="avatar-dot">{roleLabel(role).slice(0, 2)}</span>
        <span className="product-rail-user-copy">
          <strong>{roleLabel(role)}</strong>
          <span>Account</span>
        </span>
      </summary>
      <div className="product-rail-menu-panel">
        <Link href={roleHref("/account", role)} className="menu-link">
          Account
        </Link>
        <Link href={roleHref("/preferences", role)} className="menu-link">
          Preferences
        </Link>
        <Link href={roleHref("/", role)} className="menu-link">
          Public home
        </Link>
        <Link href={roleHref("/", "anonymous")} className="menu-link">
          Sign out
        </Link>
      </div>
    </details>
  );
}

function MemberTopRail({ role }: { role: ProductRole }) {
  return (
    <nav className="member-top-rail" aria-label="Member top navigation">
      <Link href={roleHref("/", role)} className="member-top-link">
        Home
      </Link>
    </nav>
  );
}

function waitingCountForSpace(space: ProductAppSpaceDefinition): number {
  return space.items.reduce((count, item) => count + (item.unreadCount ?? 0), 0);
}

function ShellIcon({ appSpaceId }: { appSpaceId: ProductAppSpace }) {
  if (appSpaceId === "my-ordo") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M5 7.5a4 4 0 0 1 4-4h6a4 4 0 0 1 4 4v3a4 4 0 0 1-4 4h-3.6L7 18v-3.6a4 4 0 0 1-2-3.4z" />
      </svg>
    );
  }
  if (appSpaceId === "staff") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M7 12a5 5 0 0 1 10 0v2" />
        <path d="M5 12v3a2 2 0 0 0 2 2h1v-5H6" />
        <path d="M19 12v3a2 2 0 0 1-2 2h-1v-5h2" />
        <path d="M12 19h3" />
      </svg>
    );
  }
  if (appSpaceId === "studio") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M4 16h16" />
        <path d="M7 16V8l4 3V8l4 3V8h2v8" />
        <path d="M6 19h12" />
      </svg>
    );
  }
  if (appSpaceId === "owner") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M4 18V6" />
        <path d="M4 18h16" />
        <path d="m7 15 3-4 3 2 4-6" />
      </svg>
    );
  }
  if (appSpaceId === "admin") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M12 3 5 6v5c0 4.5 2.8 7.8 7 10 4.2-2.2 7-5.5 7-10V6z" />
        <path d="M9 12h6" />
      </svg>
    );
  }

  return <span>{appSpaceId.slice(0, 1).toUpperCase()}</span>;
}

function MobileStackBar({
  eyebrow,
  title,
  backHref,
  backLabel,
  nextHref,
  nextLabel,
}: {
  eyebrow: string;
  title: string;
  backHref: string;
  backLabel: string;
  nextHref?: string;
  nextLabel?: string;
}) {
  return (
    <div className="mobile-stack-bar" aria-label="Mobile navigation step">
      <Link href={backHref} className="mobile-stack-back" aria-label={`Back to ${backLabel}`}>
        <span aria-hidden="true">‹</span>
        {backLabel}
      </Link>
      <span className="mobile-stack-title">
        <span>{eyebrow}</span>
        <strong>{title}</strong>
      </span>
      {nextHref && nextLabel ? (
        <Link href={nextHref} className="mobile-stack-next" aria-label={nextLabel}>
          {nextLabel}
          <span aria-hidden="true">›</span>
        </Link>
      ) : null}
    </div>
  );
}

function RoomIcon({ appSpaceId, itemId }: { appSpaceId: ProductAppSpace; itemId: string }) {
  if (itemId === "ordo" || itemId === "chat" || itemId === "conversations" || itemId === "messages") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M5 7.5a4 4 0 0 1 4-4h6a4 4 0 0 1 4 4v3a4 4 0 0 1-4 4h-3.6L7 18v-3.6a4 4 0 0 1-2-3.4z" />
      </svg>
    );
  }
  if (itemId === "activity" || itemId === "today" || itemId === "brief" || itemId === "system") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M12 3v4" />
        <path d="M12 17v4" />
        <path d="M3 12h4" />
        <path d="M17 12h4" />
        <path d="M8.5 8.5 6 6" />
        <path d="m18 18-2.5-2.5" />
        <path d="M15.5 8.5 18 6" />
        <path d="M6 18l2.5-2.5" />
      </svg>
    );
  }
  if (itemId === "offers" || itemId === "pipeline" || itemId === "revenue") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M4 6.5h9l7 7-6.5 6.5-7-7z" />
        <path d="M8 9h.01" />
      </svg>
    );
  }
  if (itemId === "capabilities" || itemId === "packs" || itemId === "artifacts" || itemId === "media" || itemId === "backup") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M5 8h14v11H5z" />
        <path d="M8 8V5h8v3" />
      </svg>
    );
  }
  if (itemId === "asks" || itemId === "requests" || itemId === "handoffs" || itemId === "feedback" || itemId === "reviews") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M5 5h14v14H5z" />
        <path d="m8 12 2.5 2.5L16 9" />
      </svg>
    );
  }
  if (itemId === "affiliate" || itemId === "affiliates" || itemId === "referrals" || itemId === "connections") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M8 12a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" />
        <path d="M16 18a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" />
        <path d="M10.8 10.7 13.2 13.3" />
      </svg>
    );
  }
  if (itemId === "knowledge" || itemId === "content-pillars" || itemId === "content" || itemId === "publications" || itemId === "templates") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M5 5.5h9a3 3 0 0 1 3 3v10H8a3 3 0 0 1-3-3z" />
        <path d="M17 8.5h2v10h-2" />
      </svg>
    );
  }
  if (itemId === "factory-jobs" || itemId === "jobs") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M4 16h16" />
        <path d="M7 16V8l4 3V8l4 3V8h2v8" />
        <path d="M6 19h12" />
      </svg>
    );
  }
  if (itemId === "health" || itemId === "events" || itemId === "logs" || itemId === "providers" || itemId === "access" || itemId === "settings") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M12 3 5 6v5c0 4.5 2.8 7.8 7 10 4.2-2.2 7-5.5 7-10V6z" />
        <path d="M9 12h6" />
      </svg>
    );
  }

  return <span>{appSpaceId.slice(0, 1).toUpperCase()}</span>;
}

function shellHref(href: string, role: ProductRole, railMode: ProductRailMode, mobileStep?: ProductMobileStep, selectedItemIndex?: number): string {
  return setShellState(roleHref(href, role), railMode, mobileStep, selectedItemIndex);
}

function setShellState(href: string, railMode: ProductRailMode, mobileStep?: ProductMobileStep, selectedItemIndex?: number): string {
  const url = new URL(href, "https://ordo.local");

  if (railMode === "collapsed") {
    url.searchParams.set("rail", "collapsed");
  } else {
    url.searchParams.delete("rail");
  }

  if (mobileStep && mobileStep !== "rooms") {
    url.searchParams.set("mobile", mobileStep);
  } else {
    url.searchParams.delete("mobile");
  }

  if (selectedItemIndex !== undefined && selectedItemIndex > 0) {
    url.searchParams.set("item", String(selectedItemIndex));
  } else {
    url.searchParams.delete("item");
  }

  const query = url.searchParams.toString();
  return `${url.pathname}${query ? `?${query}` : ""}${url.hash}`;
}

function mobileStepForRoom(itemId: string): ProductMobileStep {
  return itemId === "ordo" ? "content" : "evidence";
}

function sectionHeadingForAppSpace(appSpaceId: ProductAppSpace): string {
  switch (appSpaceId) {
    case "site":
      return "Site";
    case "my-ordo":
      return "Ordo";
    case "staff":
      return "Support";
    case "studio":
      return "Studio";
    case "owner":
      return "Business";
    case "admin":
      return "System";
  }
}
