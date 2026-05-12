import { ProductShell } from "@/components/product-shell";
import { PageTitle, SnapshotEvidence, statusClass } from "@/components/system-panels";
import { BriefEvidence, BriefProcessProvenance, getSystemSnapshot } from "@/lib/daemon-client";
import { mobileStepFromSearchParams, railModeFromSearchParams, roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { isAdminRole, type ProductRole } from "@/lib/product-navigation";

export const dynamic = "force-dynamic";

export default async function AdminSystemPage({ searchParams }: { searchParams?: SearchParams }) {
  const requestedRole = await roleFromSearchParams(searchParams);
  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams);
  const role: ProductRole = isAdminRole(requestedRole) ? requestedRole : "owner";
  const snapshot = await getSystemSnapshot();
  const brief = snapshot.brief;
  const healthStatus = snapshot.health?.status ?? "unavailable";
  const readinessStatus = snapshot.readiness?.status ?? "unavailable";
  const isDegraded = Boolean(snapshot.degradedReason);

  const bullets = brief?.summary ?? fallbackSummary(isDegraded, healthStatus, readinessStatus);
  const limitations = brief?.limitations ?? fallbackLimitations(snapshot.briefError, isDegraded);
  const createdAt = brief?.createdAt ?? snapshot.createdAt;
  const statusLabel = isDegraded ? "degraded" : brief ? `brief v${brief.version}` : healthStatus;

  return (
    <ProductShell role={role} appSpaceId="admin" currentItemId="system" railMode={railMode} mobileStep={mobileStep}>
      <PageTitle eyebrow="System View" title="System Brief" description="The latest plain-language view of what matters in the appliance." />

      <section className="brief-panel">
        <div className="meta-row">
          <span>As of {createdAt}</span>
          <span className={statusClass(isDegraded ? "error" : healthStatus)}>{statusLabel}</span>
        </div>
        <ul className="brief-list">
          {bullets.map((bullet) => (
            <li key={bullet}>{bullet}</li>
          ))}
        </ul>
      </section>

      {brief ? (
        <section className="plain-panel">
          <h3 className="panel-title">Report</h3>
          <div className="brief-body-markdown">{brief.bodyMarkdown}</div>
        </section>
      ) : null}

      {brief ? <BriefEvidencePanel evidence={brief.evidence} /> : <SnapshotEvidence snapshot={snapshot} />}

      <section className="plain-panel">
        <h3 className="panel-title">Limitations</h3>
        <ul className="brief-list">
          {limitations.map((limitation) => (
            <li key={limitation}>{limitation}</li>
          ))}
        </ul>
      </section>

      <section className="plain-panel">
        <h3 className="panel-title">Provenance</h3>
        {brief?.process ? <BriefProvenance process={brief.process} /> : <FallbackProvenance snapshotCreatedAt={snapshot.createdAt} />}
      </section>
    </ProductShell>
  );
}

function fallbackSummary(isDegraded: boolean, healthStatus: string, readinessStatus: string): string[] {
  if (isDegraded) {
    return [
      "The daemon is not reachable, so the shell is showing the fallback System Brief.",
      "Health, readiness, and realtime evidence are unavailable until the appliance spine is online.",
      "The latest attempted daemon snapshot is recorded below.",
    ];
  }

  return [
    `The daemon health endpoint reports ${healthStatus}.`,
    `The daemon readiness endpoint reports ${readinessStatus}.`,
    "No durable System Brief artifact has been published yet.",
  ];
}

function fallbackLimitations(briefError: string | null, isDegraded: boolean): string[] {
  if (briefError) {
    return [`The latest brief artifact could not be read: ${briefError}`];
  }
  if (isDegraded) {
    return ["No durable brief artifact is available while the daemon is unreachable."];
  }
  return ["No completed System Brief artifact exists yet."];
}

function BriefEvidencePanel({ evidence }: { evidence: BriefEvidence[] }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Evidence</h3>
      <ul className="evidence-list">
        {evidence.map((item) => (
          <li key={`${item.label}:${item.source}`}>
            <span className="label">{item.label}</span>
            <span className="value">
              {item.value} <span className="muted">from {item.source}</span>
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}

function BriefProvenance({ process }: { process: BriefProcessProvenance }) {
  return (
    <div>
      <div className="data-row">
        <span className="label">Job</span>
        <span className="value">{process.jobId}</span>
      </div>
      <div className="data-row">
        <span className="label">Process</span>
        <span className="value">
          {process.templateId} v{process.templateVersion}
        </span>
      </div>
      <div className="data-row">
        <span className="label">Origin</span>
        <span className="value">{process.origin}</span>
      </div>
      <div className="data-row">
        <span className="label">Status</span>
        <span className="value">{process.status}</span>
      </div>
    </div>
  );
}

function FallbackProvenance({ snapshotCreatedAt }: { snapshotCreatedAt: string }) {
  return (
    <div>
      <div className="data-row">
        <span className="label">Snapshot</span>
        <span className="value">{snapshotCreatedAt}</span>
      </div>
      <div className="data-row">
        <span className="label">Process</span>
        <span className="value">No completed brief job.</span>
      </div>
    </div>
  );
}
