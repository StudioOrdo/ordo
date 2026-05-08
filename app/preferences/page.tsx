import { SystemShell } from "@/components/system-shell";
import { PageTitle } from "@/components/system-panels";
import { getSystemSnapshot } from "@/lib/daemon-client";

export const dynamic = "force-dynamic";

export default async function PreferencesPage() {
  const snapshot = await getSystemSnapshot();

  return (
    <SystemShell currentItemId="preferences" websocketUrl={snapshot.degradedReason ? null : snapshot.websocketUrl}>
      <PageTitle
        eyebrow="Settings"
        title="Preferences"
        description="System preferences will persist through the appliance SQLite boundary."
      />

      <section className="plain-panel">
        <div className="data-row">
          <span className="label">Daemon URL</span>
          <span className="value">{snapshot.daemonUrl}</span>
        </div>
        <div className="data-row">
          <span className="label">WebSocket URL</span>
          <span className="value">{snapshot.websocketUrl}</span>
        </div>
        <div className="data-row">
          <span className="label">Automatic Backups</span>
          <span className="value">Not configured.</span>
        </div>
      </section>
    </SystemShell>
  );
}