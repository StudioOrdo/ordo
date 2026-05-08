import { SystemShell } from "@/components/system-shell";
import { BackupRestoreActions } from "@/components/backup-restore-actions";
import { PageTitle, statusClass } from "@/components/system-panels";
import { BackupRestoreJobSummary, getBackupRestoreSnapshot, getSystemSnapshot } from "@/lib/daemon-client";

export const dynamic = "force-dynamic";

export default async function BackupRestorePage() {
  const [snapshot, backupSnapshot] = await Promise.all([getSystemSnapshot(), getBackupRestoreSnapshot()]);
  const latestBackupId = latestBackupArtifactId(backupSnapshot.jobs);
  const daemonUnavailable = Boolean(snapshot.degradedReason || backupSnapshot.degradedReason);

  return (
    <SystemShell currentItemId="backup-restore" websocketUrl={snapshot.degradedReason ? null : snapshot.websocketUrl}>
      <PageTitle
        eyebrow="Safety"
        title="Backup & Restore"
        description="Backup and restore jobs will run through the shared task DAG kernel."
      />

      <BackupRestoreActions latestBackupId={latestBackupId} disabled={daemonUnavailable} />

      {backupSnapshot.degradedReason ? (
        <section className="plain-panel">
          <h3 className="panel-title">State</h3>
          <p className="brief-body">{backupSnapshot.degradedReason}</p>
        </section>
      ) : null}

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
            {backupSnapshot.jobs.length === 0 ? (
              <tr>
                <td colSpan={6} className="table-empty">
                  No backup or restore jobs are available yet.
                </td>
              </tr>
            ) : (
              backupSnapshot.jobs.map((job) => <JobRow key={job.id} job={job} />)
            )}
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

function JobRow({ job }: { job: BackupRestoreJobSummary }) {
  const artifactLabel = artifactText(job);
  return (
    <tr>
      <td>
        <strong>{job.operation}</strong>
        <span className="table-subtle">{job.id}</span>
      </td>
      <td>
        <span className={statusClass(job.status)}>{job.status}</span>
        {job.failureMessage ? <span className="table-subtle">{job.failureMessage}</span> : null}
      </td>
      <td>
        {job.progress.completedRequiredTasks}/{job.progress.totalRequiredTasks} tasks
        <span className="table-subtle">{job.progress.percent}%</span>
      </td>
      <td>{job.currentTaskKey ?? "complete"}</td>
      <td>{job.elapsedSeconds === null ? "pending" : `${job.elapsedSeconds}s`}</td>
      <td>
        {artifactLabel.primary}
        {artifactLabel.secondary ? <span className="table-subtle">{artifactLabel.secondary}</span> : null}
      </td>
    </tr>
  );
}

function latestBackupArtifactId(jobs: BackupRestoreJobSummary[]): string | null {
  for (const job of jobs) {
    const backupId = job.artifact?.metadata.backupId;
    if (job.operation === "backup" && typeof backupId === "string") {
      return backupId;
    }
  }
  return null;
}

function artifactText(job: BackupRestoreJobSummary): { primary: string; secondary: string | null } {
  if (!job.artifact) {
    return { primary: "none", secondary: null };
  }
  const backupId = job.artifact.metadata.backupId;
  const manifestPath = job.artifact.metadata.manifestPath;
  const safetyRecordPath = job.artifact.metadata.safetyRecordPath;
  return {
    primary: typeof backupId === "string" ? backupId : job.artifact.label,
    secondary: typeof manifestPath === "string" ? manifestPath : typeof safetyRecordPath === "string" ? safetyRecordPath : job.artifact.uri,
  };
}