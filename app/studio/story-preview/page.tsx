import { ProductShell } from "@/components/product-shell";
import { PageTitle, statusClass } from "@/components/system-panels";
import { getStudioStoryPreviewSnapshot } from "@/lib/daemon-client";
import {
  buildStudioStoryPreviewView,
  type StudioStoryPreviewSlideView,
  type StudioStoryPreviewView,
} from "@/lib/studio-story-preview";
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
  const mobileStep = await mobileStepFromSearchParams(searchParams);
  const snapshot = await getStudioStoryPreviewSnapshot(viewer, {
    deckId: firstParam(params.deckId),
  });
  const view = buildStudioStoryPreviewView(snapshot);
  const degraded = Boolean(snapshot.degradedReason);
  const role: ProductRole = requestedRole;

  return (
    <ProductShell role={role} appSpaceId="studio" currentItemId="story-preview" railMode={railMode} mobileStep={mobileStep}>
      <PageTitle
        eyebrow="Studio"
        title="Story Preview"
        description="Protected owner/staff homepage story preview assembled from public-safe deck and publication readiness evidence."
      />

      <section className="brief-panel">
        <div className="meta-row">
          <span>Deck {view.deckId}</span>
          <span className={statusClass(degraded ? "error" : studioPublicationStatusTone(view.status))}>{degraded ? "degraded" : view.status}</span>
        </div>
        <h3 className="panel-title">Homepage Story Preview</h3>
        <ul className="brief-list">
          {view.summaryLines.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      </section>

      {snapshot.degradedReason ? (
        <section className="plain-panel">
          <h3 className="panel-title">State</h3>
          <p className="brief-body">Studio Story preview evidence is degraded because daemon Story routes are unavailable.</p>
          <p className="table-subtle">{snapshot.degradedReason}</p>
        </section>
      ) : null}

      <PreviewStatePanel view={view} />
      <SlidePreviewPanel slides={view.slides} />
      <PublicationReadinessPanel view={view} />
      <DeferredStatesPanel view={view} />
      <LimitationsPanel view={view} />
      <NextActionsPanel view={view} />
    </ProductShell>
  );
}

function PreviewStatePanel({ view }: { view: StudioStoryPreviewView }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Preview Evidence</h3>
      <div className="data-row">
        <span className="label">Readiness</span>
        <span className="value">{view.readinessLabel}</span>
      </div>
      <div className="data-row">
        <span className="label">Slides</span>
        <span className="value">{view.slideCount}</span>
      </div>
      <div className="data-row">
        <span className="label">Publication evidence</span>
        <span className="value">{view.publicationEvidenceCount}</span>
      </div>
      <div className="data-row">
        <span className="label">Safe local refs</span>
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
                No protected preview slides are available.
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
      <h3 className="panel-title">Story Publication Readiness</h3>
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
        <p className="brief-body">Missing or degraded publication evidence remains explicit.</p>
      )}
    </section>
  );
}

function DeferredStatesPanel({ view }: { view: StudioStoryPreviewView }) {
  return (
    <section className="plain-panel table-shell">
      <h3 className="panel-title">Deferred State</h3>
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
      <h3 className="panel-title">Limitations</h3>
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
      <h3 className="panel-title">Next Actions</h3>
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
