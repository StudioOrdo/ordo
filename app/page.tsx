import { SystemShell } from "@/components/system-shell";
import { PageTitle, SnapshotEvidence, statusClass } from "@/components/system-panels";
import { getSystemSnapshot } from "@/lib/daemon-client";

export const dynamic = "force-dynamic";

export default async function SystemBriefPage() {
  const snapshot = await getSystemSnapshot();
  const healthStatus = snapshot.health?.status ?? "unavailable";
  const readinessStatus = snapshot.readiness?.status ?? "unavailable";
  const isDegraded = Boolean(snapshot.degradedReason);

  const bullets = isDegraded
    ? [
        "The daemon is not reachable, so the shell is showing the fallback System Brief.",
        "Health, readiness, and realtime evidence are unavailable until the appliance spine is online.",
        "The latest attempted daemon snapshot is recorded below.",
      ]
    : [
        `The daemon health endpoint reports ${healthStatus}.`,
        `The daemon readiness endpoint reports ${readinessStatus}.`,
        "This is the current System Brief surface until scheduled brief artifacts are available.",
      ];

  return (
    <SystemShell currentItemId="brief" websocketUrl={snapshot.degradedReason ? null : snapshot.websocketUrl}>
      <PageTitle
        eyebrow="Brief"
        title="System Brief"
        description="The latest plain-language view of what matters in the appliance."
      />

      <section className="brief-panel">
        <div className="meta-row">
          <span>Created {snapshot.createdAt}</span>
          <span className={statusClass(isDegraded ? "error" : healthStatus)}>{isDegraded ? "degraded" : healthStatus}</span>
        </div>
        <ul className="brief-list">
          {bullets.map((bullet) => (
            <li key={bullet}>{bullet}</li>
          ))}
        </ul>
      </section>

      <SnapshotEvidence snapshot={snapshot} />

      <section className="plain-panel">
        <h3 className="panel-title">Limitations</h3>
        <p className="brief-body">
          No durable brief artifact exists yet. The page is reading live daemon evidence when it is available and preserving a plain fallback when it is not.
        </p>
      </section>
    </SystemShell>
  );
}