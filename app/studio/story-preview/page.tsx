import { ProductShell } from "@/components/product-shell";
import { PageTitle, statusClass } from "@/components/system-panels";
import { getStudioStoryPreviewSnapshot } from "@/lib/daemon-client";
import {
  buildStudioStoryPreviewView,
  type StudioStoryPreviewSlideView,
  type StudioStoryPreviewView,
  type StudioStoryWorkflowStateView,
} from "@/lib/studio-story-preview";
import type { StudioStoryIntakeRequest } from "@/lib/studio-story-intake";
import {
  mobileStepFromSearchParams,
  railModeFromSearchParams,
  roleFromSearchParams,
  type SearchParams,
} from "@/lib/page-role";
import type { ProductRole } from "@/lib/product-navigation";
import { studioViewerForRole } from "@/lib/studio-work";
import { studioPublicationStatusLabel, studioPublicationStatusTone } from "@/lib/studio-publications";
import { notFound } from "next/navigation";

export const dynamic = "force-dynamic";

export default async function StudioStoryPreviewPage({ searchParams }: { searchParams?: SearchParams }) {
  const requestedRole = await roleFromSearchParams(searchParams);
  const viewer = studioViewerForRole(requestedRole);
  if (!viewer) {
    notFound();
  }

  const params = searchParams ? await searchParams : {};
  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams, "content");
  const storyIntakeRequest = storyIntakeRequestFromParams(params);
  const snapshot = await getStudioStoryPreviewSnapshot(viewer, {
    deckId: firstParam(params.deckId),
    storyIntakeRequest,
  });
  const view = buildStudioStoryPreviewView(snapshot);
  const degraded = Boolean(snapshot.degradedReason);
  const role: ProductRole = requestedRole;

  return (
    <ProductShell role={role} appSpaceId="studio" currentItemId="story-preview" railMode={railMode} mobileStep={mobileStep}>
      <PageTitle
        eyebrow="Studio"
        title="Story Preview"
        description="Review the public story before anything is published, sent to providers, or treated as truth."
      />

      <section className="brief-panel">
        <div className="meta-row">
          <span>Deck {view.deckId}</span>
          <span className={statusClass(degraded ? "error" : studioPublicationStatusTone(view.status))}>{degraded ? "needs attention" : view.status}</span>
        </div>
        <h3 className="panel-title">Story Preview Check</h3>
        <ul className="brief-list">
          {view.summaryLines.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      </section>

      {snapshot.degradedReason ? (
        <section className="plain-panel">
          <h3 className="panel-title">Needs Attention</h3>
          <p className="brief-body">Ordo cannot read all local Story Preview evidence right now. Nothing was published, sent to a provider, or promoted to memory.</p>
          <details>
            <summary>Technical detail</summary>
            <p className="table-subtle">{snapshot.degradedReason}</p>
          </details>
        </section>
      ) : null}

      <PreviewStatePanel view={view} />
      <WorkflowStatePanel view={view} />
      <SlidePreviewPanel slides={view.slides} />
      <PublicationReadinessPanel view={view} />
      <DeferredStatesPanel view={view} />
      <LimitationsPanel view={view} />
      <NextActionsPanel view={view} />
    </ProductShell>
  );
}

function WorkflowStatePanel({ view }: { view: StudioStoryPreviewView }) {
  return (
    <section className="plain-panel table-shell">
      <div className="meta-row">
        <span>{view.workflowCompilation?.templateLabel ?? "studio.story.scrollytelling_homepage"}</span>
        <span className={statusClass(workflowToneClass(view.workflowState))}>{view.workflowState.label}</span>
      </div>
      <h3 className="panel-title">Production Plan Status</h3>
      <p className="brief-body">{view.workflowState.detail}</p>
      {view.workflowCompilation ? (
        <>
          <div className="data-row">
            <span className="label">Plan record</span>
            <span className="value">{view.workflowCompilation.compilationRef}</span>
          </div>
          <div className="data-row">
            <span className="label">Safe evidence</span>
            <span className="value">{view.workflowCompilation.safeEvidenceRefCount} safe local ref(s)</span>
          </div>
          <div className="data-row">
            <span className="label">Planned steps</span>
            <span className="value">{view.workflowCompilation.taskCount}</span>
          </div>
          <div className="data-row">
            <span className="label">Approvals needed</span>
            <span className="value">
              {view.workflowCompilation.approvalGates.length > 0
                ? view.workflowCompilation.approvalGates.join(", ")
                : "none"}
            </span>
          </div>
          {view.workflowCompilation.missingInputs.length > 0 ? (
            <ul className="brief-list">
              {view.workflowCompilation.missingInputs.map((item) => (
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
              {view.workflowCompilation.taskBindings.length === 0 ? (
                <tr>
                  <td colSpan={3} className="table-empty">
                    No planned steps are shown while the plan is blocked.
                  </td>
                </tr>
              ) : (
                view.workflowCompilation.taskBindings.slice(0, 8).map((task) => (
                  <tr key={task.key}>
                    <td>{task.key}</td>
                    <td>{task.method}</td>
                    <td>{task.outputArtifactKind}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </>
      ) : (
        <p className="brief-body">Story Intake has not provided a safe production plan for this Preview request.</p>
      )}
      <table className="data-table">
        <thead>
          <tr>
            <th>State</th>
            <th>Condition</th>
          </tr>
        </thead>
        <tbody>
          {view.workflowStates.map((state) => (
            <tr key={state.key}>
              <td>
                <span className={statusClass(workflowToneClass(state))}>{state.label}</span>
              </td>
              <td>{state.detail}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

function workflowToneClass(state: StudioStoryWorkflowStateView): "ok" | "warn" | "error" | "info" {
  if (state.tone === "muted") return "info";
  return state.tone;
}

function PreviewStatePanel({ view }: { view: StudioStoryPreviewView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Preview Status</h3>
      <div className="data-row">
        <span className="label">Readiness</span>
        <span className="value">{view.readinessLabel}</span>
      </div>
      <div className="data-row">
        <span className="label">Slides</span>
        <span className="value">{view.slideCount}</span>
      </div>
      <div className="data-row">
        <span className="label">Publication review evidence</span>
        <span className="value">{view.publicationEvidenceCount}</span>
      </div>
      <div className="data-row">
        <span className="label">Safe local references</span>
        <span className="value">{view.safeEvidenceRefCount} safe local ref(s)</span>
      </div>
    </section>
  );
}

function SlidePreviewPanel({ slides }: { slides: StudioStoryPreviewSlideView[] }) {
  return (
    <section className="plain-panel table-shell">
      <h3 className="panel-title">Preview Slides</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>Slide</th>
            <th>Motion</th>
            <th>Evidence</th>
            <th>CTA</th>
          </tr>
        </thead>
        <tbody>
          {slides.length === 0 ? (
            <tr>
              <td colSpan={4} className="table-empty">
                    No safe preview slides are available.
              </td>
            </tr>
          ) : (
            slides.map((slide) => <SlidePreviewRow key={slide.id} slide={slide} />)
          )}
        </tbody>
      </table>
    </section>
  );
}

function SlidePreviewRow({ slide }: { slide: StudioStoryPreviewSlideView }) {
  return (
    <tr>
      <td>
        <strong>{slide.title}</strong>
        <span className="table-subtle">{slide.body}</span>
        {slide.sourceLine ? <span className="table-subtle">{slide.sourceLine}</span> : null}
        {slide.limitations.length > 0 ? <span className="table-subtle">{slide.limitations.join(", ")}</span> : null}
      </td>
      <td>{slide.motionProfile}</td>
      <td>{slide.evidenceRefCount} safe local ref(s)</td>
      <td>{slide.ctaLabel && slide.ctaHref ? `${slide.ctaLabel} (${slide.ctaHref})` : "none"}</td>
    </tr>
  );
}

function PublicationReadinessPanel({ view }: { view: StudioStoryPreviewView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Approval And Publishing Status</h3>
      {view.publication ? (
        <>
          <div className="data-row">
            <span className="label">Production review</span>
            <span className="value">{studioPublicationStatusLabel(view.publication.reviewStatus)}</span>
          </div>
          <div className="data-row">
            <span className="label">Publish learning</span>
            <span className="value">{studioPublicationStatusLabel(view.publication.learningStatus)}</span>
          </div>
          <div className="data-row">
            <span className="label">Missing/deferred</span>
            <span className="value">{view.publication.missingOrDeferredCount}</span>
          </div>
        </>
      ) : (
        <p className="brief-body">Approval evidence is missing or unavailable. Ordo is not publishing anything.</p>
      )}
    </section>
  );
}

function DeferredStatesPanel({ view }: { view: StudioStoryPreviewView }) {
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
              <td>
                <span className={statusClass(studioPublicationStatusTone(state.sourceStatus))}>
                  {studioPublicationStatusLabel(state.sourceStatus)}
                </span>{" "}
                {state.detail}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

function LimitationsPanel({ view }: { view: StudioStoryPreviewView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Known Limits</h3>
      <ul className="brief-list">
        {view.limitations.length === 0 ? <li>No limitations reported.</li> : null}
        {view.limitations.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </section>
  );
}

function NextActionsPanel({ view }: { view: StudioStoryPreviewView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">What To Do Next</h3>
      <ul className="brief-list">
        {view.nextActions.length === 0 ? <li>Review preview evidence.</li> : null}
        {view.nextActions.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </section>
  );
}

function firstParam(value: string | string[] | undefined): string | undefined {
  return Array.isArray(value) ? value[0] : value;
}

function storyIntakeRequestFromParams(params: Record<string, string | string[] | undefined>): StudioStoryIntakeRequest | null {
  const intakeId = firstParam(params.intakeId)?.trim();
  const founderStory = firstParam(params.founderStory)?.trim();
  const businessStance = firstParam(params.businessStance)?.trim();
  if (!intakeId || !founderStory || !businessStance) {
    return null;
  }
  return {
    intakeId,
    founderStory,
    businessStance,
    audience: firstParam(params.audience)?.trim() || null,
    evidenceRefs: evidenceRefsFromParam(firstParam(params.evidenceRefs)),
  };
}

function evidenceRefsFromParam(value: string | undefined): string[] {
  return (value ?? "")
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean)
    .slice(0, 12);
}
