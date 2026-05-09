import Link from "next/link";
import type { ReactNode } from "react";

import {
  adminSystemRailItems,
  businessStaffRailItems,
  isAdminRole,
  isStaffRole,
  roleHref,
  roleLabel,
  topRailItems,
  type ProductRole,
} from "@/lib/product-navigation";

interface Props {
  role: ProductRole;
  currentTopItemId: string;
  currentStaffItemId?: string;
  currentSystemItemId?: string;
  children: ReactNode;
}

export function ProductShell({ role, currentTopItemId, currentStaffItemId, currentSystemItemId, children }: Props) {
  const showStaffRail = isStaffRole(role);
  const showSystemRail = isAdminRole(role);

  return (
    <div className="product-shell" data-role={role}>
      <header className="top-rail">
        <Link href={roleHref("/home", role)} className="product-brand">
          Studio Ordo
        </Link>
        <nav className="top-nav" aria-label="Public and member navigation">
          {topRailItems.map((item) => {
            const active = item.id === currentTopItemId;
            return (
              <Link key={item.id} href={roleHref(item.href, role)} className={active ? "top-nav-link active" : "top-nav-link"} aria-current={active ? "page" : undefined}>
                {item.label}
              </Link>
            );
          })}
        </nav>
        <span className="role-badge">{roleLabel(role)}</span>
      </header>

      <div className="product-body">
        {showStaffRail ? (
          <aside className="staff-rail">
            <span className="rail-label">Business</span>
            <nav className="rail-list" aria-label="Staff business navigation">
              {businessStaffRailItems.map((item) => {
                const active = item.id === currentStaffItemId;
                return (
                  <Link key={item.id} href={roleHref(item.href, role)} className={active ? "rail-link active" : "rail-link"} aria-current={active ? "page" : undefined}>
                    <strong>{item.label}</strong>
                    <span>{item.description}</span>
                  </Link>
                );
              })}
            </nav>
          </aside>
        ) : null}

        {showSystemRail ? (
          <aside className="staff-rail system-rail">
            <span className="rail-label">System</span>
            <nav className="rail-list" aria-label="Admin system navigation">
              {adminSystemRailItems.map((item) => {
                const active = item.id === currentSystemItemId;
                return (
                  <Link key={item.id} href={roleHref(item.href, role)} className={active ? "rail-link active" : "rail-link"} aria-current={active ? "page" : undefined}>
                    <strong>{item.label}</strong>
                    <span>{item.description}</span>
                  </Link>
                );
              })}
            </nav>
          </aside>
        ) : null}

        <main className="product-main">
          <div className="product-content">{children}</div>
        </main>
      </div>
    </div>
  );
}
