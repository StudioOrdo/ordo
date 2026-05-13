import { ProductShell } from "@/components/product-shell";
import { PageTitle, statusClass } from "@/components/system-panels";
import {
  getGrowthPilotReportSnapshot,
  type GrowthPilotEvidenceRef,
  type GrowthPilotReportItem,
  type GrowthPilotReportLimitation,
  type GrowthPilotReportMetric,
  type GrowthPilotReportSection,
} from "@/lib/daemon-client";
import {
  buildGrowthPilotReportView,
  growthReportSourceStatuses,
  growthReportStatusClassToken,
  growthSourceStatusLabel,
  type GrowthPilotReportSectionView,
  type GrowthPilotReportView,
} from "@/lib/growth-pilot-report";
import { mobileStepFromSearchParams, railModeFromSearchParams, roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { isAdminRole, type ProductRole } from "@/lib/product-navigation";
import { notFound } from "next/navigation";

export const dynamic = "force-dynamic";

export default async function OwnerReportsPage({ searchParams }: { searchParams?: SearchParams }) {
  const requestedRole = await roleFromSearchParams(searchParams);
  if (!isAdminRole(requestedRole)) {
    notFound();
  }

  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams);
  const role: ProductRole = requestedRole;
  const snapshot = await getGrowthPilotReportSnapshot();
  const view = snapshot.report ? buildGrowthPilotReportView(snapshot.report) : null;
  const degraded = Boolean(snapshot.degradedReason);

  return (
    <ProductShell role={role} appSpaceId="owner" currentItemId="reports" railMode={railMode} mobileStep={mobileStep}>
      <PageTitle
        eyebrow="Growth"
        title="Growth Pilot Report"
        description="Owner-only NYC pilot evidence across entry, offers, trials, handoffs, feedback, rewards, and Studio promos."
      />

      <section className="brief-panel">
        <div className="meta-row">
          <span>As of {snapshot.generatedAt ?? snapshot.createdAt}</span>
          <span className={statusClass(degraded ? "error" : view ? "ready" : "empty")}>{degraded ? "degraded" : view ? "ready" : "empty"}</span>
        </div>
        <ul className="brief-list">
          {summaryLines(view, degraded).map((line) => (
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

      {view && snapshot.report ? (
        <>
          <StatusSummaryPanel view={view} />
          <LimitationsPanel title="Global Limitations" limitations={snapshot.report.limitations} />
          {view.sections.map((section) => (
            <GrowthReportSectionPanel key={section.key} section={section} />
          ))}
        </>
      ) : !degraded ? (
        <section className="plain-panel">
          <h3 className="panel-title">State</h3>
          <p className="brief-body">No Growth pilot report snapshot is available yet.</p>
        </section>
      ) : null}
    </ProductShell>
  );
}

function StatusSummaryPanel({ view }: { view: GrowthPilotReportView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Source Status</h3>
      {growthReportSourceStatuses.map((status) => (
        <div key={status} className="data-row">
          <span className="label">{growthSourceStatusLabel(status)}</span>
          <span className="value">
            <span className={statusClass(growthReportStatusClassToken(status))}>{view.statusCounts[status]}</span>
          </span>
        </div>
      ))}
    </section>
  );
}

function GrowthReportSectionPanel({ section }: { section: GrowthPilotReportSectionView }) {
  return (
    <>
      <section className="plain-panel table-shell">
        <div className="meta-row">
          <span className={statusClass(growthReportStatusClassToken(section.sourceStatus))}>{growthSourceStatusLabel(section.sourceStatus)}</span>
          <span>{section.metricSummary}</span>
        </div>
        <h3 className="panel-title">{section.title}</h3>
        <table className="data-table">
          <thead>
            <tr>
              <th>Metric</th>
              <th>Value</th>
              <th>Source</th>
              <th>Evidence</th>
            </tr>
          </thead>
          <tbody>
            {section.metrics.length === 0 ? (
              <tr>
                <td colSpan={4} className="table-empty">
                  No metrics are available for this section.
                </td>
              </tr>
            ) : (
              section.metrics.map((metric) => <MetricRow key={metric.key} metric={metric} />)
            )}
          </tbody>
        </table>
      </section>

      <RecentItemsPanel section={section} />
      {section.limitations.length > 0 ? <LimitationsPanel title={`${section.title} Limitations`} limitations={section.limitations} /> : null}
    </>
  );
}

function MetricRow({ metric }: { metric: GrowthPilotReportMetric }) {
  return (
    <tr>
      <td>
        <strong>{metric.label}</strong>
        <span className="table-subtle">{metric.key}</span>
      </td>
      <td>
        {metric.value} {metric.unit}
      </td>
      <td>
        <span className={statusClass(growthReportStatusClassToken(metric.sourceStatus))}>{growthSourceStatusLabel(metric.sourceStatus)}</span>
      </td>
      <td>{metric.evidenceRefs.length > 0 ? metric.evidenceRefs.map((ref) => <EvidenceRefView key={ref.uri} evidenceRef={ref} />) : "none"}</td>
    </tr>
  );
}

function RecentItemsPanel({ section }: { section: GrowthPilotReportSection }) {
  return (
    <section className="plain-panel table-shell">
      <h3 className="panel-title">{section.title} Recent Evidence</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>Evidence</th>
            <th>Status</th>
            <th>Source</th>
            <th>Occurred</th>
          </tr>
        </thead>
        <tbody>
          {section.recentItems.length === 0 ? (
            <tr>
              <td colSpan={4} className="table-empty">
                No recent local evidence is available.
              </td>
            </tr>
          ) : (
            section.recentItems.map((item) => <RecentItemRow key={`${item.sourceKind}:${item.sourceId}`} item={item} />)
          )}
        </tbody>
      </table>
    </section>
  );
}

function RecentItemRow({ item }: { item: GrowthPilotReportItem }) {
  return (
    <tr>
      <td>
        <strong>{item.label}</strong>
        <span className="table-subtle">
          {item.sourceKind}:{item.sourceId}
        </span>
        {item.evidenceRefs.map((ref) => (
          <EvidenceRefView key={ref.uri} evidenceRef={ref} />
        ))}
      </td>
      <td>
        <span className={statusClass(item.status)}>{item.status}</span>
      </td>
      <td>
        <span className={statusClass(growthReportStatusClassToken(item.sourceStatus))}>{growthSourceStatusLabel(item.sourceStatus)}</span>
      </td>
      <td>{item.occurredAt}</td>
    </tr>
  );
}

function LimitationsPanel({ title, limitations }: { title: string; limitations: GrowthPilotReportLimitation[] }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">{title}</h3>
      {limitations.length === 0 ? (
        <p className="brief-body">No limitations are reported for this scope.</p>
      ) : (
        limitations.map((limitation) => <LimitationRow key={limitation.key} limitation={limitation} />)
      )}
    </section>
  );
}

function LimitationRow({ limitation }: { limitation: GrowthPilotReportLimitation }) {
  return (
    <div className="data-row">
      <span className="label">
        <span className={statusClass(growthReportStatusClassToken(limitation.sourceStatus))}>{growthSourceStatusLabel(limitation.sourceStatus)}</span>
      </span>
      <span className="value">
        <strong>{limitation.label}</strong>
        <span className="table-subtle">{limitation.detail}</span>
      </span>
    </div>
  );
}

function EvidenceRefView({ evidenceRef }: { evidenceRef: GrowthPilotEvidenceRef }) {
  return (
    <span className="table-subtle">
      {evidenceRef.label}: {evidenceRef.uri}
    </span>
  );
}

function summaryLines(view: GrowthPilotReportView | null, degraded: boolean): string[] {
  if (degraded) {
    return [
      "Growth report is degraded because the daemon snapshot is unavailable.",
      "Unsupported platform analytics, publishing, payments, OAuth, and provider claims remain unavailable.",
    ];
  }
  if (!view) {
    return ["No daemon-backed Growth report snapshot is available yet."];
  }
  return view.summaryLines;
}
