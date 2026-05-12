import Link from "next/link";
import type { ReactNode } from "react";

import type { ProductRole } from "@/lib/product-navigation";

interface OrdoFrameProps {
  role: ProductRole;
  homeHref: string;
  topRail: ReactNode;
  railNavigation?: ReactNode;
  accountUtility?: ReactNode;
  children: ReactNode;
}

export function OrdoFrame({ role, homeHref, topRail, railNavigation, accountUtility, children }: OrdoFrameProps) {
  return (
    <div className="ordo-frame" data-role={role}>
      <aside className="ordo-frame-rail" aria-label="Workspace navigation">
        <Link href={homeHref} className="product-rail-home" aria-label="Studio Ordo public home">
          <span className="product-rail-mark" aria-hidden="true">
            <img src="/logo.png" alt="" className="brand-logo-image" />
          </span>
          <span className="product-rail-brand-copy">
            <strong>Studio Ordo</strong>
            <span>Site</span>
          </span>
        </Link>
        <div className="ordo-frame-rail-main">{railNavigation}</div>
        <div className="ordo-frame-rail-footer">{accountUtility}</div>
      </aside>
      <div className="ordo-frame-top">{topRail}</div>
      <div className="ordo-frame-content">{children}</div>
    </div>
  );
}
