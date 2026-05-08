import type { DaemonCheck, SystemSnapshot } from "@/lib/daemon-client";

export function statusClass(status: string): string {
  if (["ok", "ready", "live", "succeeded", "info", "low", "ready_for_review"].includes(status)) return "status-pill status-ok";
  if (["not_ready", "connecting", "running", "waiting_for_input", "blocked", "warn", "medium", "high"].includes(status)) {
    return "status-pill status-warn";
  }
  return "status-pill status-error";
}

export function PageTitle({ eyebrow, title, description }: { eyebrow: string; title: string; description: string }) {
  return (
    <div className="page-title">
      <span className="eyebrow">{eyebrow}</span>
      <h2>{title}</h2>
      <p>{description}</p>
    </div>
  );
}

export function CheckList({ checks }: { checks: readonly DaemonCheck[] }) {
  if (checks.length === 0) {
    return <p className="table-empty">No checks reported.</p>;
  }

  return (
    <div>
      {checks.map((check) => (
        <div className="data-row" key={check.name}>
          <span className="label">{check.name}</span>
          <span className="value">
            <span className={statusClass(check.status)}>{check.status}</span> {check.detail}
          </span>
        </div>
      ))}
    </div>
  );
}

export function SnapshotEvidence({ snapshot }: { snapshot: SystemSnapshot }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Evidence</h3>
      <ul className="evidence-list">
        <li>
          <span className="label">Snapshot</span>
          <span className="value">{snapshot.createdAt}</span>
        </li>
        <li>
          <span className="label">Daemon</span>
          <span className="value">{snapshot.daemonUrl}</span>
        </li>
        <li>
          <span className="label">Health</span>
          <span className="value">{snapshot.health?.status ?? "unavailable"}</span>
        </li>
        <li>
          <span className="label">Readiness</span>
          <span className="value">{snapshot.readiness?.status ?? "unavailable"}</span>
        </li>
      </ul>
    </section>
  );
}