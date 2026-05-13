import { ProductShell } from "@/components/product-shell";
import { PageTitle, statusClass } from "@/components/system-panels";
import {
  BackupRestoreJobSummary,
  getHostedTrialOperationsSnapshot,
  HostedTrialCapacityPolicy,
  HostedTrialSlot,
  HostedTrialWaitlistEntry,
} from "@/lib/daemon-client";
import { mobileStepFromSearchParams, railModeFromSearchParams, roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { isAdminRole, type ProductRole } from "@/lib/product-navigation";
import { notFound } from "next/navigation";

export const dynamic = "force-dynamic";

export default async function AdminHostedTrialsPage({ searchParams }: { searchParams?: SearchParams }) {
  const requestedRole = await roleFromSearchParams(searchParams);
  if (!isAdminRole(requestedRole)) {
    notFound();
  }
  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams);
  const role: ProductRole = requestedRole;
  const snapshot = await getHostedTrialOperationsSnapshot();
  const activePolicy = snapshot.policies[0] ?? null;
  const latestBackupId = latestBackupArtifactId(snapshot.backupJobs);
  const degraded = Boolean(snapshot.degradedReason);

  return (
    <ProductShell role={role} appSpaceId="admin" currentItemId="hosted-trials" railMode={railMode} mobileStep={mobileStep}>
      <PageTitle
        eyebrow="Systems"
        title="Hosted Trials"
        description="Capacity, expiration, backup/export, and reset guard evidence for hosted pilot appliances."
      />

      <section className="brief-panel">
        <div className="meta-row">
          <span>As of {snapshot.createdAt}</span>
          <span className={statusClass(degraded ? "error" : activePolicy?.status ?? "empty")}>{degraded ? "degraded" : activePolicy?.status ?? "empty"}</span>
        </div>
        <ul className="brief-list">
          {summaryLines(activePolicy, snapshot.slots, snapshot.waitlist, degraded).map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      </section>

      {snapshot.degradedReason ? (
        <section className="plain-panel">
          <h3 className="panel-title">State</h3>
          <p className="brief-body">{snapshot.degradedReason}</p>
        </section>
      ) : null}

      <section className="plain-panel">
        <h3 className="panel-title">Capacity Policy</h3>
        {snapshot.policies.length === 0 ? (
          <p className="brief-body">No hosted trial capacity policy is available.</p>
        ) : (
          snapshot.policies.map((policy) => <PolicySummary key={policy.id} policy={policy} />)
        )}
      </section>

      <section className="plain-panel table-shell">
        <h3 className="panel-title">Trial Slots</h3>
        <table className="data-table">
          <thead>
            <tr>
              <th>Trial</th>
              <th>Status</th>
              <th>Expiration</th>
              <th>Backup</th>
              <th>Reset Guard</th>
              <th>Owner Evidence</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.slots.length === 0 ? (
              <tr>
                <td colSpan={6} className="table-empty">
                  No hosted trial slots are available.
                </td>
              </tr>
            ) : (
              snapshot.slots.map((slot) => <SlotRow key={slot.id} slot={slot} />)
            )}
          </tbody>
        </table>
      </section>

      <section className="plain-panel table-shell">
        <h3 className="panel-title">Waitlist</h3>
        <table className="data-table">
          <thead>
            <tr>
              <th>Position</th>
              <th>Acceptance</th>
              <th>Status</th>
              <th>Reason</th>
              <th>Evidence</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.waitlist.length === 0 ? (
              <tr>
                <td colSpan={5} className="table-empty">
                  No hosted trial waitlist entries are available.
                </td>
              </tr>
            ) : (
              snapshot.waitlist.map((entry) => <WaitlistRow key={entry.id} entry={entry} />)
            )}
          </tbody>
        </table>
      </section>

      <section className="plain-panel">
        <h3 className="panel-title">Backup And Restore</h3>
        <div className="data-row">
          <span className="label">Latest backup</span>
          <span className="value">{latestBackupId ?? "none"}</span>
        </div>
        <div className="data-row">
          <span className="label">Jobs</span>
          <span className="value">{snapshot.backupJobs.length}</span>
        </div>
        <div className="data-row">
          <span className="label">Reset action</span>
          <span className="value">destructive wipe unavailable</span>
        </div>
      </section>
    </ProductShell>
  );
}

function PolicySummary({ policy }: { policy: HostedTrialCapacityPolicy }) {
  return (
    <div>
      <div className="data-row">
        <span className="label">Offer</span>
        <span className="value">{policy.offerSlug}</span>
      </div>
      <div className="data-row">
        <span className="label">Active slots</span>
        <span className="value">
          {policy.activeSlotCount} / {policy.activeSlotLimit} active
        </span>
      </div>
      <div className="data-row">
        <span className="label">Waitlist</span>
        <span className="value">{policy.waitlistCount} waiting</span>
      </div>
      <div className="data-row">
        <span className="label">Trial window</span>
        <span className="value">{policy.trialDays} days</span>
      </div>
      <div className="data-row">
        <span className="label">Backup before wipe</span>
        <span className="value">{policy.backupBeforeWipeRequired ? "required" : "not required"}</span>
      </div>
      <div className="data-row">
        <span className="label">Reset grace</span>
        <span className="value">{policy.resetGraceDays} days</span>
      </div>
    </div>
  );
}

function SlotRow({ slot }: { slot: HostedTrialSlot }) {
  return (
    <tr>
      <td>
        <strong>{slot.trialId}</strong>
        <span className="table-subtle">{slot.offerSlug}</span>
      </td>
      <td>
        <span className={statusClass(slot.status)}>{slot.status}</span>
        {slot.releaseReason ? <span className="table-subtle">{slot.releaseReason}</span> : null}
      </td>
      <td>
        {slot.expiresAt}
        {slot.resetEligibleAt ? <span className="table-subtle">eligible {slot.resetEligibleAt}</span> : null}
      </td>
      <td>
        {slot.backupStatus}
        <span className="table-subtle">{slot.backupEvidenceRefs.length > 0 ? slot.backupEvidenceRefs.join(", ") : "no evidence"}</span>
      </td>
      <td>
        {slot.resetState}
        <span className="table-subtle">{resetGuardText(slot.resetGuard)}</span>
      </td>
      <td>{ownerEvidenceText(slot.ownerOverride)}</td>
    </tr>
  );
}

function WaitlistRow({ entry }: { entry: HostedTrialWaitlistEntry }) {
  return (
    <tr>
      <td>{entry.position}</td>
      <td>
        <strong>{entry.acceptanceId}</strong>
        <span className="table-subtle">{entry.offerSlug}</span>
      </td>
      <td>{entry.status}</td>
      <td>{entry.reason}</td>
      <td>{entry.evidenceRefs.length > 0 ? entry.evidenceRefs.join(", ") : "none"}</td>
    </tr>
  );
}

function summaryLines(
  policy: HostedTrialCapacityPolicy | null,
  slots: HostedTrialSlot[],
  waitlist: HostedTrialWaitlistEntry[],
  degraded: boolean,
): string[] {
  if (degraded) {
    return ["Hosted trial operations are degraded because the daemon snapshot is unavailable."];
  }
  if (!policy) {
    return ["No hosted trial capacity policy has been created yet."];
  }

  const resetReadyCount = slots.filter((slot) => slot.resetState === "ready_for_owner_review").length;
  return [
    `${policy.offerSlug} has ${policy.activeSlotCount} / ${policy.activeSlotLimit} active hosted trial slots.`,
    `${waitlist.length} waiting in the hosted trial waitlist.`,
    policy.backupBeforeWipeRequired ? "Backup/export is required before wipe readiness." : "Backup/export is not required by this policy.",
    `${resetReadyCount} trial slot(s) are marked ready for owner review; destructive wipe unavailable.`,
  ];
}

function resetGuardText(resetGuard: Record<string, unknown>): string {
  if (resetGuard.destructiveWipeAllowed === false) {
    return "destructive wipe unavailable";
  }
  return typeof resetGuard.reason === "string" ? resetGuard.reason : "guard not recorded";
}

function ownerEvidenceText(ownerOverride: Record<string, unknown>): string {
  if (Object.keys(ownerOverride).length === 0) {
    return "no owner override";
  }
  return "owner decision recorded";
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
