import { SystemShell } from "@/components/system-shell";
import { PageTitle, statusClass } from "@/components/system-panels";
import { DiagnosticLogEntry, getDiagnosticLogsSnapshot, getSystemSnapshot } from "@/lib/daemon-client";

export const dynamic = "force-dynamic";

export default async function LogsPage() {
  const [snapshot, logsSnapshot] = await Promise.all([getSystemSnapshot(), getDiagnosticLogsSnapshot()]);

  return (
    <SystemShell currentItemId="logs" websocketUrl={snapshot.degradedReason ? null : snapshot.websocketUrl}>
      <PageTitle
        eyebrow="Diagnostics"
        title="Logs"
        description="Structured diagnostic observations from the local appliance."
      />

      <section className="plain-panel">
        <h3 className="panel-title">Local Log Store</h3>
        <p className="brief-body">
          {logsSnapshot.degradedReason
            ? logsSnapshot.degradedReason
            : `Showing ${logsSnapshot.logs.length} recent structured logs from ${logsSnapshot.daemonUrl}.`}
        </p>
        <div className="filter-row" aria-label="Available log filters">
          <span className="status-pill">error</span>
          <span className="status-pill">warn</span>
          <span className="status-pill">info</span>
          <span className="status-pill">job</span>
          <span className="status-pill">reports</span>
        </div>
      </section>

      <section className="plain-panel table-shell">
        <table className="data-table log-table">
          <thead>
            <tr>
              <th>Time</th>
              <th>Level</th>
              <th>Source</th>
              <th>Message</th>
              <th>Correlation</th>
            </tr>
          </thead>
          <tbody>
            {logsSnapshot.logs.length === 0 ? (
              <tr>
                <td colSpan={5} className="table-empty">
                  No diagnostic logs are available yet.
                </td>
              </tr>
            ) : (
              logsSnapshot.logs.map((entry) => <LogRow key={entry.id} entry={entry} />)
            )}
          </tbody>
        </table>
      </section>
    </SystemShell>
  );
}


function LogRow({ entry }: { entry: DiagnosticLogEntry }) {
  const correlation = [entry.jobId, entry.taskKey, entry.capabilityId, entry.eventType].filter(Boolean).join(" / ");

  return (
    <tr>
      <td>{new Date(entry.timestamp).toLocaleString()}</td>
      <td><span className={statusClass(entry.level)}>{entry.level}</span></td>
      <td>{entry.source}</td>
      <td>
        {entry.message}
        <details className="detail-block">
          <summary>Payload</summary>
          <pre>{JSON.stringify(entry.payload, null, 2)}</pre>
        </details>
      </td>
      <td>{correlation || "system"}</td>
    </tr>
  );
}