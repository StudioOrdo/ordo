import Link from "next/link";
import type { ReactNode } from "react";

import { WebSocketStatus } from "@/components/websocket-status";
import { systemMenuItems } from "@/lib/system-menu";

interface Props {
  currentItemId: string;
  websocketUrl: string;
  children: ReactNode;
}

export function SystemShell({ currentItemId, websocketUrl, children }: Props) {
  return (
    <div className="system-shell">
      <aside className="primary-rail" aria-label="Primary navigation">
        <div className="brand-mark">
          <strong>Ordo</strong>
          <span>0.1.0</span>
        </div>
        <Link href="/" className="primary-link primary-link-active" aria-current="page">
          System
        </Link>
      </aside>

      <aside className="section-column" aria-label="System sections">
        <div className="section-heading">
          <span className="eyebrow">System</span>
          <h1>Appliance</h1>
          <p>Brief first, evidence close, diagnostics behind the selected section.</p>
        </div>

        <nav className="system-menu">
          {systemMenuItems.map((item) => {
            const isActive = item.id === currentItemId;
            return (
              <Link
                key={item.id}
                href={item.href}
                className={`section-link ${isActive ? "section-link-active" : ""}`}
                aria-current={isActive ? "page" : undefined}
              >
                <span>
                  <strong>{item.label}</strong>
                  <span>{item.description}</span>
                </span>
                <span className="link-dot" aria-hidden="true" />
              </Link>
            );
          })}
        </nav>

        <WebSocketStatus url={websocketUrl} />
      </aside>

      <main className="main-pane">
        <div className="main-content">{children}</div>
      </main>
    </div>
  );
}