import { ProductShell } from "@/components/product-shell";
import { PageTitle, statusClass } from "@/components/system-panels";
import { getStudioPublicationsSnapshot } from "@/lib/daemon-client";
import {
  buildStudioPublicationsView,
  studioPublicationStatusCounts,
  studioPublicationStatusLabel,
  studioPublicationStatusTone,
  type StudioPublicationComponentView,
  type StudioPublicationDeferredState,
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
  const mobileStep = await mobileStepFromSearchParams(searchParams);
  const snapshot = await getStudioPublicationsSnapshot(viewer, {
    deckId: firstParam(params.deckId),
    artifactIds: listParam(params.artifactIds ?? params.artifactId),
  });
  const view = snapshot.review && snapshot.learning ? buildStudioPublicationsView(snapshot.review, snapshot.learning) : null;
  const role: ProductRole = requestedRole;
  const degraded = Boolean(snapshot.degradedReason);

  return (
    <ProductShell role={role} appSpaceId="studio" currentItemId="publications" railMode={railMode} mobileStep={mobileStep}>
      <PageTitle
        eyebrow="Studio"
        title="Publications"
        description="Owner/staff Story publication readiness and learning evidence from protected local daemon routes."
      />

      <section className="brief-panel">
        <div className="meta-row">
          <span>Deck {snapshot.deckId}</span>
          <span className={statusClass(degraded ? "error" : view ? view.status : "empty")}>
            {degraded ? "degraded" : view ? view.status : "empty"}
          </span>
        </div>
        <h3 className="panel-title">Story Publication Readiness</h3>
        <ul className="brief-list">
          {summaryLines(view, degraded).map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      </section>

      {snapshot.degradedReason ? (
        <section className="plain-panel">
          <h3 className="panel-title">State</h3>
          <p className="brief-body">Studio Publications evidence is degraded because daemon Story routes are unavailable.</p>
          <p className="table-subtle">{snapshot.degradedReason}</p>
        </section>
      ) : null}

      {view ? (
        <>
          <PublicationStatePanel view={view} />
          <StatusSummaryPanel view={view} />
          <ComponentPanel components={view.components} />
          <LearningPanel title="Story Publish Learning" metrics={[...view.sourceStatus, ...view.contentMetrics]} />
          <PublishEvidencePanel sources={view.publishEvidence} />
          <DeferredStatesPanel states={view.deferredStates} />
          <LimitationsPanel limitations={view.learningLimitations} />
          <NextActionsPanel actions={view.nextActions} />
        </>
      ) : !degraded ? (
        <section className="plain-panel">
          <h3 className="panel-title">State</h3>
          <p className="brief-body">No daemon-backed Story publication evidence is available yet.</p>
        </section>
      ) : null}
    </ProductShell>
  );
}

function PublicationStatePanel({ view }: { view: StudioPublicationsView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Publication Evidence</h3>
      <div className="data-row">
        <span className="label">Production review</span>
        <span className="value">
          <span className={statusClass(view.reviewStatus)}>{studioPublicationStatusLabel(view.reviewStatus)}</span>
        </span>
      </div>
      <div className="data-row">
        <span className="label">Publish learning</span>
        <span className="value">
          <span className={statusClass(view.learningStatus)}>{studioPublicationStatusLabel(view.learningStatus)}</span>
        </span>
      </div>
      <div className="data-row">
        <span className="label">Safe local refs</span>
        <span className="value">{view.safeEvidenceRefCount}</span>
      </div>
      <div className="data-row">
        <span className="label">Memory candidates</span>
        <span className="value">{view.memoryCandidateCount}</span>
      </div>
    </section>
  );
}

function StatusSummaryPanel({ view }: { view: StudioPublicationsView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Source Status</h3>
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
      <h3 className="panel-title">Review Components</h3>
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
      <h3 className="panel-title">Publish Evidence Sources</h3>
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
      <h3 className="panel-title">Deferred Claims</h3>
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
      <h3 className="panel-title">Limitations</h3>
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
      <h3 className="panel-title">Next Actions</h3>
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
      "Studio Publications evidence is degraded because daemon Story routes are unavailable.",
      "External publishing, provider execution, memory promotion, and graph promotion remain unavailable unless daemon evidence says otherwise.",
    ];
  }
  return view?.summaryLines ?? ["No daemon-backed Story publication evidence is available yet."];
}

function firstParam(value: string | string[] | undefined): string | undefined {
  return Array.isArray(value) ? value[0] : value;
}

function listParam(value: string | string[] | undefined): string[] {
  const values = Array.isArray(value) ? value : value ? [value] : [];
  return values.flatMap((item) => item.split(",")).map((item) => item.trim()).filter(Boolean);
}
