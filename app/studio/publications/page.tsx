import { ProductShell } from "@/components/product-shell";
import { StudioMemoryDecisionActions } from "@/components/studio-memory-decision-actions";
import { PageTitle, statusClass } from "@/components/system-panels";
import { getStudioPublicationsSnapshot } from "@/lib/daemon-client";
import {
  buildStudioPublicationsView,
  studioPublicationStatusCounts,
  studioPublicationStatusLabel,
  studioPublicationStatusTone,
  type StudioPublicationComponentView,
  type StudioPublicationDeferredState,
  type StudioMemoryReviewItemView,
  type StudioMemoryReviewPacketView,
  type StudioPublicationMetricView,
  type StudioPublicationSourceView,
  type StudioPublicationsView,
} from "@/lib/studio-publications";
import {
  mobileStepFromSearchParams,
  railModeFromSearchParams,
  roleFromSearchParams,
  type SearchParams,
} from "@/lib/page-role";
import { studioViewerForRole } from "@/lib/studio-work";
import type { ProductRole } from "@/lib/product-navigation";
import { notFound } from "next/navigation";

export const dynamic = "force-dynamic";

export default async function StudioPublicationsPage({ searchParams }: { searchParams?: SearchParams }) {
  const requestedRole = await roleFromSearchParams(searchParams);
  const viewer = studioViewerForRole(requestedRole);
  if (!viewer) {
    notFound();
  }

  const params = searchParams ? await searchParams : {};
  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams, "content");
  const snapshot = await getStudioPublicationsSnapshot(viewer, {
    deckId: firstParam(params.deckId),
    artifactIds: listParam(params.artifactIds ?? params.artifactId),
  });
  const view = snapshot.review && snapshot.learning
    ? buildStudioPublicationsView(snapshot.review, snapshot.learning, snapshot.memoryReviewPackets)
    : null;
  const role: ProductRole = requestedRole;
  const degraded = Boolean(snapshot.degradedReason);

  return (
    <ProductShell role={role} appSpaceId="studio" currentItemId="publications" railMode={railMode} mobileStep={mobileStep}>
      <PageTitle
        eyebrow="Studio"
        title="Publications"
        description="Review what is ready, what still needs a person, and what has not been promoted or published."
      />

      <section className="brief-panel">
        <div className="meta-row">
          <span>Deck {snapshot.deckId}</span>
          <span className={statusClass(degraded ? "error" : view ? view.status : "empty")}>
            {degraded ? "needs attention" : view ? view.status : "empty"}
          </span>
        </div>
        <h3 className="panel-title">Publication Review</h3>
        <ul className="brief-list">
          {summaryLines(view, degraded).map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      </section>

      {snapshot.degradedReason ? (
        <section className="plain-panel">
          <h3 className="panel-title">Needs Attention</h3>
          <p className="brief-body">Ordo cannot read the local publication review right now. Nothing was published, sent to a provider, or promoted to memory.</p>
          <details>
            <summary>Technical detail</summary>
            <p className="table-subtle">{snapshot.degradedReason}</p>
          </details>
        </section>
      ) : null}

      {view ? (
        <>
          <PublicationStatePanel view={view} />
          <StatusSummaryPanel view={view} />
          <ComponentPanel components={view.components} />
          <LearningPanel title="What Happened After Publishing" metrics={[...view.sourceStatus, ...view.contentMetrics]} />
          <MemoryReviewPanel packets={view.memoryReviewPackets} role={role} />
          <PublishEvidencePanel sources={view.publishEvidence} />
          <DeferredStatesPanel states={view.deferredStates} />
          <LimitationsPanel limitations={view.limitations} />
          <NextActionsPanel actions={view.nextActions} />
        </>
      ) : !degraded ? (
        <section className="plain-panel">
          <h3 className="panel-title">Not Ready Yet</h3>
          <p className="brief-body">No Story publication review is ready yet. Ordo is not publishing or promoting anything.</p>
        </section>
      ) : null}
    </ProductShell>
  );
}

function PublicationStatePanel({ view }: { view: StudioPublicationsView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Review Status</h3>
      <div className="data-row">
        <span className="label">Owner review</span>
        <span className="value">
          <span className={statusClass(view.reviewStatus)}>{studioPublicationStatusLabel(view.reviewStatus)}</span>
        </span>
      </div>
      <div className="data-row">
        <span className="label">After-publish learning</span>
        <span className="value">
          <span className={statusClass(view.learningStatus)}>{studioPublicationStatusLabel(view.learningStatus)}</span>
        </span>
      </div>
      <div className="data-row">
        <span className="label">Safe local references</span>
        <span className="value">{view.safeEvidenceRefCount}</span>
      </div>
      <div className="data-row">
        <span className="label">Memory items awaiting review</span>
        <span className="value">{view.memoryCandidateCount}</span>
      </div>
    </section>
  );
}

function StatusSummaryPanel({ view }: { view: StudioPublicationsView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Checklist</h3>
      {studioPublicationStatusCounts().map((status) => (
        <div key={status} className="data-row">
          <span className="label">{studioPublicationStatusLabel(status)}</span>
          <span className="value">
            <span className={statusClass(studioPublicationStatusTone(status))}>{view.sourceStatusCounts[status]}</span>
          </span>
        </div>
      ))}
    </section>
  );
}

function ComponentPanel({ components }: { components: StudioPublicationComponentView[] }) {
  return (
    <section className="plain-panel table-shell">
      <h3 className="panel-title">Review Checklist</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>Component</th>
            <th>Status</th>
            <th>Evidence</th>
            <th>Next</th>
          </tr>
        </thead>
        <tbody>
          {components.length === 0 ? (
            <tr>
              <td colSpan={4} className="table-empty">
                No production review components are available.
              </td>
            </tr>
          ) : (
            components.map((component) => <ComponentRow key={component.key} component={component} />)
          )}
        </tbody>
      </table>
    </section>
  );
}

function ComponentRow({ component }: { component: StudioPublicationComponentView }) {
  return (
    <tr>
      <td>
        <strong>{component.title}</strong>
        <span className="table-subtle">{component.summary}</span>
        {component.limitations.length > 0 ? <span className="table-subtle">{component.limitations.join(", ")}</span> : null}
      </td>
      <td>
        <span className={statusClass(studioPublicationStatusTone(component.evidenceStatus))}>
          {studioPublicationStatusLabel(component.evidenceStatus)}
        </span>
      </td>
      <td>{component.evidenceRefCount} safe local ref(s)</td>
      <td>{component.nextAction}</td>
    </tr>
  );
}

function MemoryReviewPanel({ packets, role }: { packets: StudioMemoryReviewPacketView[]; role: ProductRole }) {
  const items = packets.flatMap((packet) => packet.items);
  return (
    <section className="plain-panel table-shell">
      <h3 className="panel-title">Memory Review Packet</h3>
      {packets.length === 0 || items.length === 0 ? (
        <p className="brief-body">No generated content is ready for owner/staff memory review.</p>
      ) : (
        <table className="data-table">
          <thead>
            <tr>
              <th>Candidate</th>
              <th>Status</th>
              <th>Evidence</th>
              <th>Decision</th>
            </tr>
          </thead>
          <tbody>
            {packets.map((packet) =>
              packet.items.map((item) => <MemoryReviewRow key={`${packet.artifactId}:${item.candidateId}`} item={item} role={role} />),
            )}
          </tbody>
        </table>
      )}
      {packets.length > 0 ? (
        <ul className="brief-list">
          {packets.map((packet) => (
            <li key={packet.artifactId}>
              {packet.artifactId}: {packet.candidateCount} candidate(s), {packet.evidenceRefCount} safe local ref(s).
              {" "}{packet.promotionReadyCount} owner review packet(s) ready; {packet.readinessBlockerCount} blocker(s).
              {packet.confirmedGraphPromotion ? " Graph promotion confirmed." : " Graph promotion not confirmed."}
              {packet.liveProviderCalled ? " Live provider evidence present." : " Live provider not called."}
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}

function MemoryReviewRow({ item, role }: { item: StudioMemoryReviewItemView; role: ProductRole }) {
  return (
    <tr>
      <td>
        <strong>{item.summary}</strong>
        <span className="table-subtle">{item.label}</span>
        <span className="table-subtle">{studioPublicationStatusLabel(item.memoryKind)} / {studioPublicationStatusLabel(item.memoryTier)}</span>
      </td>
      <td>
        <span className={statusClass(item.canApprove || item.canReject ? "warn" : "ready")}>{studioPublicationStatusLabel(item.state)}</span>
        <span className="table-subtle">{item.confidencePercent}% confidence</span>
        <span className="table-subtle">
          Owner review packet:{" "}
          <span className={statusClass(item.promotionReady ? "ready" : "warn")}>
            {item.promotionReady ? "ready" : "blocked"}
          </span>
        </span>
        <span className="table-subtle">Allowed next action: {item.readinessAllowedNextAction}</span>
        {item.readinessBlockers.length > 0 ? <span className="table-subtle">{item.readinessBlockers.join(", ")}</span> : null}
      </td>
      <td>
        {item.evidenceRefCount} safe local ref(s)
        <span className="table-subtle">
          Readiness refs: {item.readinessEvidenceRefCount} evidence, {item.readinessDecisionRefCount} decision
        </span>
      </td>
      <td>
        {item.canApprove || item.canReject ? (
          <StudioMemoryDecisionActions candidateId={item.candidateId} evidenceRefs={item.evidenceRefs} disabled={false} role={role} />
        ) : (
          <span className={statusClass("ready")}>Decision recorded</span>
        )}
        <span className="table-subtle">
          {item.memoryPromotionPerformed ? "Memory promotion performed." : "Memory promotion not performed."}
          {" "}{item.confirmedGraphPromotion ? "Graph promotion confirmed." : "Graph promotion not confirmed."}
          {" "}{item.vectorMutationPerformed ? "Vector mutation performed." : "Vector mutation not performed."}
          {" "}{item.packStateMutationPerformed ? "Pack state changed." : "Pack state unchanged."}
        </span>
      </td>
    </tr>
  );
}

function LearningPanel({ title, metrics }: { title: string; metrics: StudioPublicationMetricView[] }) {
  return (
    <section className="plain-panel table-shell">
      <h3 className="panel-title">{title}</h3>
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
          {metrics.length === 0 ? (
            <tr>
              <td colSpan={4} className="table-empty">
                No publish learning metrics are available.
              </td>
            </tr>
          ) : (
            metrics.map((metric) => <MetricRow key={metric.key} metric={metric} />)
          )}
        </tbody>
      </table>
    </section>
  );
}

function MetricRow({ metric }: { metric: StudioPublicationMetricView }) {
  return (
    <tr>
      <td>
        <strong>{metric.label}</strong>
        <span className="table-subtle">{metric.key}</span>
      </td>
      <td>{metric.value}</td>
      <td>
        <span className={statusClass(studioPublicationStatusTone(metric.sourceStatus))}>
          {studioPublicationStatusLabel(metric.sourceStatus)}
        </span>
      </td>
      <td>{metric.evidenceRefCount} safe local ref(s)</td>
    </tr>
  );
}

function PublishEvidencePanel({ sources }: { sources: StudioPublicationSourceView[] }) {
  return (
    <section className="plain-panel table-shell">
      <h3 className="panel-title">Evidence Sources</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>Source</th>
            <th>Status</th>
            <th>Evidence</th>
            <th>Limitations</th>
          </tr>
        </thead>
        <tbody>
          {sources.length === 0 ? (
            <tr>
              <td colSpan={4} className="table-empty">
                No publish evidence sources are available.
              </td>
            </tr>
          ) : (
            sources.map((source) => <PublishEvidenceRow key={source.key} source={source} />)
          )}
        </tbody>
      </table>
    </section>
  );
}

function PublishEvidenceRow({ source }: { source: StudioPublicationSourceView }) {
  return (
    <tr>
      <td>
        <strong>{source.label}</strong>
        <span className="table-subtle">{source.key}</span>
      </td>
      <td>
        <span className={statusClass(studioPublicationStatusTone(source.sourceStatus))}>{studioPublicationStatusLabel(source.sourceStatus)}</span>{" "}
        {studioPublicationStatusLabel(source.status)}
      </td>
      <td>{source.evidenceRefCount} safe local ref(s)</td>
      <td>{source.limitations.length > 0 ? source.limitations.join(", ") : "none"}</td>
    </tr>
  );
}

function DeferredStatesPanel({ states }: { states: StudioPublicationDeferredState[] }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Not Done Yet</h3>
      {states.length === 0 ? (
        <p className="brief-body">No deferred publication claims are reported.</p>
      ) : (
        states.map((state) => (
          <div key={state.key} className="data-row">
            <span className="label">{state.label}</span>
            <span className="value">
              <span className={statusClass(studioPublicationStatusTone(state.sourceStatus))}>
                {studioPublicationStatusLabel(state.sourceStatus)}
              </span>{" "}
              {state.detail}
            </span>
          </div>
        ))
      )}
    </section>
  );
}

function LimitationsPanel({ limitations }: { limitations: string[] }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Known Limits</h3>
      {limitations.length === 0 ? (
        <p className="brief-body">No Story publish learning limitations are reported.</p>
      ) : (
        <ul className="brief-list">
          {limitations.map((limitation) => (
            <li key={limitation}>{limitation}</li>
          ))}
        </ul>
      )}
    </section>
  );
}

function NextActionsPanel({ actions }: { actions: string[] }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">What To Do Next</h3>
      {actions.length === 0 ? (
        <p className="brief-body">No next actions are reported.</p>
      ) : (
        <ul className="brief-list">
          {actions.map((action) => (
            <li key={action}>{action}</li>
          ))}
        </ul>
      )}
    </section>
  );
}

function summaryLines(view: StudioPublicationsView | null, degraded: boolean): string[] {
  if (degraded) {
    return [
      "Ordo cannot read the local publication review right now.",
      "Nothing is published, sent to providers, promoted to memory, or written to graph truth unless local evidence proves it.",
    ];
  }
  return view?.summaryLines ?? ["No Story publication review is ready yet."];
}

function firstParam(value: string | string[] | undefined): string | undefined {
  return Array.isArray(value) ? value[0] : value;
}

function listParam(value: string | string[] | undefined): string[] {
  const values = Array.isArray(value) ? value : value ? [value] : [];
  return values.flatMap((item) => item.split(",")).map((item) => item.trim()).filter(Boolean);
}
