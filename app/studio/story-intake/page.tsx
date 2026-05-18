import { ProductShell } from "@/components/product-shell";
import { PageTitle, statusClass } from "@/components/system-panels";
import { getStudioStoryIntakeSnapshot } from "@/lib/daemon-client";
import {
  buildStudioStoryIntakeView,
  emptyStudioStoryIntakeView,
  type StudioStoryIntakeRequest,
  type StudioStoryIntakeView,
} from "@/lib/studio-story-intake";
import {
  mobileStepFromSearchParams,
  railModeFromSearchParams,
  roleFromSearchParams,
  type SearchParams,
} from "@/lib/page-role";
import type { ProductRole } from "@/lib/product-navigation";
import { studioViewerForRole } from "@/lib/studio-work";
import { notFound } from "next/navigation";

export const dynamic = "force-dynamic";

export default async function StudioStoryIntakePage({ searchParams }: { searchParams?: SearchParams }) {
  const requestedRole = await roleFromSearchParams(searchParams);
  const viewer = studioViewerForRole(requestedRole);
  if (!viewer) {
    notFound();
  }

  const params = searchParams ? await searchParams : {};
  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams, "content");
  const request = storyIntakeRequestFromParams(params);
  const snapshot = await getStudioStoryIntakeSnapshot(viewer, request);
  const degraded = Boolean(snapshot.degradedReason);
  const view = snapshot.packet ? buildStudioStoryIntakeView(snapshot.packet) : emptyStudioStoryIntakeView();
  const role: ProductRole = requestedRole;

  return (
    <ProductShell role={role} appSpaceId="studio" currentItemId="story-intake" railMode={railMode} mobileStep={mobileStep}>
      <PageTitle
        eyebrow="Studio"
        title="Story Intake"
        description="Check whether the founder story has enough safe information for Ordo to prepare the next story step."
      />

      <section className="brief-panel">
        <div className="meta-row">
          <span>Intake {snapshot.request?.intakeId ?? "pending"}</span>
          <span className={statusClass(degraded ? "error" : view.status)}>{degraded ? "needs attention" : view.status}</span>
        </div>
        <h3 className="panel-title">Story Intake Check</h3>
        <ul className="brief-list">
          {summaryLines(view, degraded).map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      </section>

      {snapshot.degradedReason ? (
        <section className="plain-panel">
          <h3 className="panel-title">Needs Attention</h3>
          <p className="brief-body">Ordo cannot read the local Story Intake record right now. Nothing was published, sent to a provider, or promoted to memory.</p>
          <details>
            <summary>Technical detail</summary>
            <p className="table-subtle">{snapshot.degradedReason}</p>
          </details>
        </section>
      ) : null}

      {!degraded ? (
        <>
          <ReadinessPanel view={view} />
          <WorkflowCompilationPanel view={view} />
          <EvidencePanel view={view} />
          <ClaimsPanel view={view} />
          <DeferredStatesPanel view={view} />
          <LimitationsPanel view={view} />
          <NextActionsPanel view={view} />
        </>
      ) : null}
    </ProductShell>
  );
}

function WorkflowCompilationPanel({ view }: { view: StudioStoryIntakeView }) {
  const workflow = view.workflowCompilation;
  if (!workflow) {
    return (
      <section className="plain-panel">
        <h3 className="panel-title">Production Plan</h3>
        <p className="brief-body">Ordo needs submitted Story Intake evidence before it can prepare the production plan.</p>
      </section>
    );
  }

  return (
    <section className="plain-panel table-shell">
      <div className="meta-row">
        <span>{workflow.templateLabel}</span>
        <span className={statusClass(workflow.status === "compiled" ? "ok" : "warn")}>{workflow.status}</span>
      </div>
      <h3 className="panel-title">Production Plan</h3>
      <div className="data-row">
        <span className="label">Plan record</span>
        <span className="value">{workflow.compilationRef}</span>
      </div>
      <div className="data-row">
        <span className="label">Safe evidence</span>
        <span className="value">{workflow.safeEvidenceRefCount} safe local ref(s)</span>
      </div>
      <div className="data-row">
        <span className="label">Inputs checked</span>
        <span className="value">{workflow.variableCount} safe input(s)</span>
      </div>
      <div className="data-row">
        <span className="label">Planned grouping</span>
        <span className="value">{workflow.fanoutSummary}</span>
      </div>
      {workflow.missingInputs.length > 0 ? (
        <ul className="brief-list">
          {workflow.missingInputs.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      ) : null}
      <table className="data-table">
        <thead>
          <tr>
            <th>Planned step</th>
            <th>Method</th>
            <th>Artifact</th>
          </tr>
        </thead>
        <tbody>
          {workflow.taskBindings.length === 0 ? (
            <tr>
              <td colSpan={3} className="table-empty">
                No planned steps are shown while the plan is blocked.
              </td>
            </tr>
          ) : (
            workflow.taskBindings.slice(0, 8).map((task) => (
              <tr key={task.key}>
                <td>{task.key}</td>
                <td>{task.method}</td>
                <td>{task.outputArtifactKind}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>
      <ul className="brief-list">
        {[...workflow.approvalGates, ...workflow.providerRequirements, ...workflow.limitations].map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </section>
  );
}

function ReadinessPanel({ view }: { view: StudioStoryIntakeView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">What Ordo Knows</h3>
      <div className="data-row">
        <span className="label">Story plan</span>
        <span className="value">
          <span className={statusClass(view.status === "ready" ? "ok" : view.status === "blocked" ? "warn" : "empty")}>
            {view.readinessLabel}
          </span>
        </span>
      </div>
      <div className="data-row">
        <span className="label">Approval</span>
        <span className="value">{view.approvalState}</span>
      </div>
      <div className="data-row">
        <span className="label">Who can see it</span>
        <span className="value">{view.visibilityCeiling}</span>
      </div>
      <div className="data-row">
        <span className="label">Audience</span>
        <span className="value">{view.audience}</span>
      </div>
      <p className="brief-body">{view.publicSummary}</p>
      {view.missingPrerequisites.length > 0 ? (
        <ul className="brief-list">
          {view.missingPrerequisites.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}

function EvidencePanel({ view }: { view: StudioStoryIntakeView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Why Ordo Knows This</h3>
      <div className="data-row">
        <span className="label">Safe local references</span>
        <span className="value">{view.safeEvidenceRefCount} safe local ref(s)</span>
      </div>
      <div className="data-row">
        <span className="label">Record</span>
        <span className="value">{view.artifactRef}</span>
      </div>
      <div className="data-row">
        <span className="label">Record type</span>
        <span className="value">{view.artifactKind}</span>
      </div>
    </section>
  );
}

function ClaimsPanel({ view }: { view: StudioStoryIntakeView }) {
  return (
    <section className="plain-panel table-shell">
      <h3 className="panel-title">Public Claims To Review</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>Claim</th>
            <th>Review</th>
            <th>Evidence</th>
          </tr>
        </thead>
        <tbody>
          {view.claims.length === 0 ? (
            <tr>
              <td colSpan={3} className="table-empty">
                No public-safe derivative claims are available.
              </td>
            </tr>
          ) : (
            view.claims.map((claim) => (
              <tr key={`${claim.claim}-${claim.reviewState}`}>
                <td>
                  <strong>{claim.claim}</strong>
                  {claim.limitations.length > 0 ? <span className="table-subtle">{claim.limitations.join(", ")}</span> : null}
                </td>
                <td>{claim.reviewState}</td>
                <td>{claim.evidenceRefCount} safe local ref(s)</td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </section>
  );
}

function DeferredStatesPanel({ view }: { view: StudioStoryIntakeView }) {
  return (
    <section className="plain-panel table-shell">
      <h3 className="panel-title">Not Done Yet</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>Boundary</th>
            <th>State</th>
          </tr>
        </thead>
        <tbody>
          {view.deferredStates.map((state) => (
            <tr key={state.key}>
              <td>{state.label}</td>
              <td>{state.detail}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

function LimitationsPanel({ view }: { view: StudioStoryIntakeView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Known Limits</h3>
      <ul className="brief-list">
        {view.limitations.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </section>
  );
}

function NextActionsPanel({ view }: { view: StudioStoryIntakeView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">What To Do Next</h3>
      <ul className="brief-list">
        {view.nextActions.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </section>
  );
}

function summaryLines(view: StudioStoryIntakeView, degraded: boolean): string[] {
  if (degraded) {
    return [
      "Ordo cannot read the local Story Intake record right now.",
      "Readiness is unknown until safe intake evidence is available.",
      "Nothing has been published, promoted to memory, written to graph truth, sent to providers, or run as a task.",
    ];
  }
  return view.summaryLines;
}

function storyIntakeRequestFromParams(params: Record<string, string | string[] | undefined>): StudioStoryIntakeRequest | null {
  const intakeId = firstParam(params.intakeId);
  const founderStory = firstParam(params.founderStory);
  const businessStance = firstParam(params.businessStance);
  if (!intakeId || !founderStory || !businessStance) {
    return null;
  }

  return {
    intakeId,
    founderStory,
    businessStance,
    audience: firstParam(params.audience) ?? null,
    evidenceRefs: listParam(params.evidenceRefs ?? params.evidenceRef),
  };
}

function firstParam(value: string | string[] | undefined): string | undefined {
  return Array.isArray(value) ? value[0] : value;
}

function listParam(value: string | string[] | undefined): string[] {
  const values = Array.isArray(value) ? value : value ? [value] : [];
  return [
    ...new Set(values.flatMap((item) => item.split(",")).map((item) => item.trim()).filter(Boolean)),
  ].sort();
}
