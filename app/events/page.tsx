import { SystemShell } from "@/components/system-shell";
import { PageTitle } from "@/components/system-panels";
import { getSystemSnapshot } from "@/lib/daemon-client";

export const dynamic = "force-dynamic";

export default async function EventsPage() {
  const snapshot = await getSystemSnapshot();

  return (
    <SystemShell currentItemId="events" websocketUrl={snapshot.websocketUrl}>
      <PageTitle
        eyebrow="Trail"
        title="Events"
        description="Realtime events are a live projection of persisted appliance events."
      />

      <section className="plain-panel">
        <h3 className="panel-title">Realtime Channel</h3>
        <p className="brief-body">
          The connection indicator in the System menu opens the daemon WebSocket and reports the latest event type.
        </p>
      </section>

      <section className="plain-panel table-shell">
        <table className="data-table">
          <thead>
            <tr>
              <th>Sequence</th>
              <th>Event</th>
              <th>Source</th>
              <th>Time</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td colSpan={4} className="table-empty">
                Persisted event replay lands after the daemon read API grows beyond health and readiness.
              </td>
            </tr>
          </tbody>
        </table>
      </section>
    </SystemShell>
  );
}