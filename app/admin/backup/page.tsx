import { BackupRestoreActions } from "@/components/backup-restore-actions";
import { ProductShell } from "@/components/product-shell";
import { PageTitle, statusClass } from "@/components/system-panels";
import { BackupRestoreJobSummary, getBackupRestoreSnapshot, getSystemSnapshot } from "@/lib/daemon-client";
import { mobileStepFromSearchParams, railModeFromSearchParams, roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { isAdminRole, type ProductRole } from "@/lib/product-navigation";
import { notFound } from "next/navigation";

export const dynamic = "force-dynamic";

export default async function AdminBackupPage({ searchParams }: { searchParams?: SearchParams }) {
  const requestedRole = await roleFromSearchParams(searchParams);
  if (!isAdminRole(requestedRole)) {
    notFound();
  }
  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams);
  const role: ProductRole = requestedRole;
  const [systemSnapshot, backupSnapshot] = await Promise.all([getSystemSnapshot(), getBackupRestoreSnapshot()]);
  const latestBackupId = latestBackupArtifactId(backupSnapshot.jobs);
  const daemonUnavailable = Boolean(systemSnapshot.degradedReason || backupSnapshot.degradedReason);

  return (
    <ProductShell role={role} appSpaceId="admin" currentItemId="backup" railMode={railMode} mobileStep={mobileStep}>
      <PageTitle
        eyebrow="Systems"
        title="Backup & Restore"
        description="Backup and restore jobs for local appliance safety."
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
    </ProductShell>
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
