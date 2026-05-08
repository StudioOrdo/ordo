import { IssueReportActions } from "@/components/issue-report-actions";
import { SystemShell } from "@/components/system-shell";
import { PageTitle, statusClass } from "@/components/system-panels";
import { getIssueReportsSnapshot, getSystemSnapshot, IssueReportArtifact } from "@/lib/daemon-client";

export const dynamic = "force-dynamic";

export default async function ReportsPage() {
  const [snapshot, reportsSnapshot] = await Promise.all([getSystemSnapshot(), getIssueReportsSnapshot()]);
  const latestReport = reportsSnapshot.reports[0] ?? null;
  const daemonUnavailable = Boolean(snapshot.degradedReason || reportsSnapshot.degradedReason);

  return (
    <SystemShell currentItemId="reports" websocketUrl={snapshot.degradedReason ? null : snapshot.websocketUrl}>
      <PageTitle
        eyebrow="Diagnostics"
        title="Reports"
        description="Issue reports and diagnostic packages prepared through the job kernel."
      />

      <IssueReportActions disabled={daemonUnavailable} latestReport={latestReport} />

      {reportsSnapshot.degradedReason ? (
        <section className="plain-panel">
          <h3 className="panel-title">State</h3>
          <p className="brief-body">{reportsSnapshot.degradedReason}</p>
        </section>
      ) : null}

      <section className="plain-panel table-shell">
        <table className="data-table">
          <thead>
            <tr>
              <th>Report</th>
              <th>Severity</th>
              <th>Status</th>
              <th>Job</th>
              <th>Updated</th>
            </tr>
          </thead>
          <tbody>
            {reportsSnapshot.reports.length === 0 ? (
              <tr>
                <td colSpan={5} className="table-empty">
                  No issue reports are available yet.
                </td>
              </tr>
            ) : (
              reportsSnapshot.reports.map((report) => <ReportRow key={report.id} report={report} />)
            )}
          </tbody>
        </table>
      </section>

      <section className="plain-panel">
        <h3 className="panel-title">Evidence Checklist</h3>
        {latestReport ? (
          <ul className="evidence-list report-evidence-list">
            {latestReport.evidence.map((entry) => (
              <li key={entry.source}>
                <span className="label">{entry.source}</span>
                <span className="value">{entry.status}: {entry.summary}</span>
              </li>
            ))}
          </ul>
        ) : (
          <p className="brief-body">Prepare a report to see collected evidence sources.</p>
        )}
      </section>

      <section className="plain-panel">
        <h3 className="panel-title">Latest Report</h3>
        {latestReport ? (
          <pre className="report-preview">{latestReport.markdownBody}</pre>
        ) : (
          <p className="brief-body">No prepared markdown report is available yet.</p>
        )}
      </section>
    </SystemShell>
  );
}


function ReportRow({ report }: { report: IssueReportArtifact }) {
  return (
    <tr>
      <td>
        <strong>{report.title}</strong>
        <span className="table-subtle">{report.id}</span>
      </td>
      <td><span className={statusClass(report.severity)}>{report.severity}</span></td>
      <td><span className={statusClass(report.status)}>{report.status}</span></td>
      <td>{report.jobId ?? "none"}</td>
      <td>{new Date(report.updatedAt).toLocaleString()}</td>
    </tr>
  );
}