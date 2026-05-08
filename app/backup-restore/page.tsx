import { SystemShell } from "@/components/system-shell";
import { PageTitle } from "@/components/system-panels";
import { getSystemSnapshot } from "@/lib/daemon-client";

export const dynamic = "force-dynamic";

export default async function BackupRestorePage() {
  const snapshot = await getSystemSnapshot();

  return (
    <SystemShell currentItemId="backup-restore" websocketUrl={snapshot.websocketUrl}>
      <PageTitle
        eyebrow="Safety"
        title="Backup & Restore"
        description="Backup and restore jobs will run through the shared task DAG kernel."
      />

      <section className="plain-panel table-shell">
        <table className="data-table">
          <thead>
            <tr>
              <th>Job</th>
              <th>Status</th>
              <th>Progress</th>
              <th>Current Task</th>
              <th>Elapsed</th>
              <th>Artifact</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td colSpan={6} className="table-empty">
                No backup or restore jobs are available yet. This table is reserved for task-count progress and artifacts.
              </td>
            </tr>
          </tbody>
        </table>
      </section>

      <section className="plain-panel">
        <h3 className="panel-title">Process Templates</h3>
        <ul className="brief-list">
          <li>backup.create declares backup boundary, lock, snapshot, scan, archive, manifest, verify, and record tasks.</li>
          <li>restore.execute declares confirmation, safety backup, archive verification, restore, app restart, and record tasks.</li>
        </ul>
      </section>
    </SystemShell>
  );
}