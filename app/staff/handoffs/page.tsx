import { ProductShell } from "@/components/product-shell";
import { PageTitle, statusClass } from "@/components/system-panels";
import { getSupportHandoffQueueSnapshot, type SupportHandoffQueueSnapshot } from "@/lib/daemon-client";
import { mobileStepFromSearchParams, railModeFromSearchParams, roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { canAccessAppSpace } from "@/lib/product-navigation";
import type { SupportHandoffQueueItemView } from "@/lib/support-handoffs";

export default async function StaffHandoffsPage({ searchParams }: { searchParams?: SearchParams }) {
  const role = await roleFromSearchParams(searchParams);
  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams, "content");
  const canReadSupportQueue = canAccessAppSpace(role, "staff");
  const snapshot = canReadSupportQueue ? await getSupportHandoffQueueSnapshot() : null;

  return (
    <ProductShell role={role} appSpaceId="staff" currentItemId="handoffs" railMode={railMode} mobileStep={mobileStep}>
      <PageTitle
        eyebrow="Support"
        title="Handoffs"
        description="Claim and work support requests that have local handoff evidence."
      />

      {!canReadSupportQueue ? (
        <section className="plain-panel">
          <h3 className="panel-title">Support Access Required</h3>
          <p className="brief-body">Only support-capable staff can see the handoff queue. Public and member views do not expose staff routing.</p>
        </section>
      ) : snapshot ? (
        <SupportHandoffQueue snapshot={snapshot} />
      ) : null}
    </ProductShell>
  );
}

function SupportHandoffQueue({ snapshot }: { snapshot: SupportHandoffQueueSnapshot }) {
  const degraded = Boolean(snapshot.degradedReason);
  return (
    <>
      <section className="brief-panel">
        <div className="meta-row">
          <span>Local support queue</span>
          <span className={statusClass(degraded ? "error" : snapshot.status)}>{degraded ? "needs attention" : snapshot.status}</span>
        </div>
        <h3 className="panel-title">Handoff Queue</h3>
        <ul className="brief-list">
          {summaryLines(snapshot, degraded).map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      </section>

      {degraded ? (
        <section className="plain-panel">
          <h3 className="panel-title">Needs Attention</h3>
          <p className="brief-body">Ordo cannot read the local support handoff queue right now. No queue item was claimed or changed.</p>
          <details>
            <summary>Technical detail</summary>
            <p className="table-subtle">{snapshot.degradedReason}</p>
          </details>
        </section>
      ) : null}

      <section className="plain-panel">
        <h3 className="panel-title">Queue Status</h3>
        <div className="data-row">
          <span className="label">Open</span>
          <span className="value">{snapshot.openCount}</span>
        </div>
        <div className="data-row">
          <span className="label">Claimed</span>
          <span className="value">{snapshot.claimedCount}</span>
        </div>
        <div className="data-row">
          <span className="label">Closed</span>
          <span className="value">{snapshot.closedCount}</span>
        </div>
        <div className="data-row">
          <span className="label">Safe local references</span>
          <span className="value">{snapshot.evidenceRefCount}</span>
        </div>
      </section>

      <section className="plain-panel table-shell">
        <h3 className="panel-title">Support Requests</h3>
        {snapshot.items.length === 0 ? (
          <p className="brief-body">No daemon-backed handoffs are waiting right now.</p>
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                <th>Request</th>
                <th>Status</th>
                <th>Next action</th>
                <th>Evidence</th>
              </tr>
            </thead>
            <tbody>
              {snapshot.items.map((item) => (
                <SupportHandoffRow key={item.id} item={item} />
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section className="plain-panel">
        <h3 className="panel-title">Known Limits</h3>
        <ul className="brief-list">
          {snapshot.limitations.map((limitation) => (
            <li key={limitation}>{limitation}</li>
          ))}
        </ul>
      </section>
    </>
  );
}

function SupportHandoffRow({ item }: { item: SupportHandoffQueueItemView }) {
  return (
    <tr>
      <td>
        <strong>{item.title}</strong>
        <span className="table-subtle">{item.sourceLabel}</span>
        <span className="table-subtle">{item.assigneeLabel}</span>
      </td>
      <td>
        <span className={statusClass(statusTone(item.status))}>{item.statusLabel}</span>
        <span className="table-subtle">{item.urgency}</span>
      </td>
      <td>
        <span>{item.nextAction}</span>
        <span className="table-subtle">Claim handoff with support.accept_handoff.</span>
      </td>
      <td>
        <span>{item.safeEvidenceRefs.length} safe ref(s)</span>
        {item.safeEvidenceRefs.slice(0, 3).map((ref) => (
          <span key={ref} className="table-subtle">
            {ref}
          </span>
        ))}
        {item.evidenceRefCount > item.safeEvidenceRefs.length ? (
          <span className="table-subtle">Some internal refs are hidden.</span>
        ) : null}
      </td>
    </tr>
  );
}

function statusTone(status: SupportHandoffQueueItemView["status"]): string {
  if (status === "claimed") {
    return "ready";
  }
  if (status === "closed") {
    return "ok";
  }
  return "queued";
}

function summaryLines(snapshot: SupportHandoffQueueSnapshot, degraded: boolean): string[] {
  if (degraded) {
    return [
      "Ordo cannot read the local support handoff queue right now.",
      "Nothing is claimed, reassigned, published, promoted to memory, or sent to providers from this view.",
    ];
  }
  return snapshot.summaryLines;
}
