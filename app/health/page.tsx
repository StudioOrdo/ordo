import { SystemShell } from "@/components/system-shell";
import { CheckList, PageTitle, SnapshotEvidence, statusClass } from "@/components/system-panels";
import { getSystemSnapshot } from "@/lib/daemon-client";

export const dynamic = "force-dynamic";

export default async function HealthPage() {
  const snapshot = await getSystemSnapshot();

  return (
    <SystemShell currentItemId="health" websocketUrl={snapshot.websocketUrl}>
      <PageTitle
        eyebrow="Evidence"
        title="Health"
        description="Daemon liveness and readiness reports for the local appliance."
      />

      <section className="plain-panel">
        <h3 className="panel-title">Status</h3>
        <div className="status-grid">
          <span className={statusClass(snapshot.health?.status ?? "error")}>health {snapshot.health?.status ?? "unavailable"}</span>
          <span className={statusClass(snapshot.readiness?.status ?? "error")}>ready {snapshot.readiness?.status ?? "unavailable"}</span>
        </div>
        {snapshot.degradedReason ? <p className="brief-body">{snapshot.degradedReason}</p> : null}
      </section>

      <section className="plain-panel">
        <h3 className="panel-title">Health Checks</h3>
        <CheckList checks={snapshot.health?.checks ?? []} />
      </section>

      <section className="plain-panel">
        <h3 className="panel-title">Readiness Checks</h3>
        <CheckList checks={snapshot.readiness?.checks ?? []} />
      </section>

      <SnapshotEvidence snapshot={snapshot} />
    </SystemShell>
  );
}