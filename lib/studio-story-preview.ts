import {
  homepageStoryDeckToSlides,
  type HomepageStoryDeckResponse,
  type NarrativeSlideView,
} from "@/lib/scrollytelling-runtime";
import {
  buildStudioPublicationsView,
  type StoryPublishLearningBrief,
  type StudioProductionReviewPacket,
  type StudioPublicationDeferredState,
  type StudioPublicationSourceStatus,
  type StudioPublicationsView,
} from "@/lib/studio-publications";
import {
  buildStudioStoryIntakeView,
  type StoryFounderIntakePacket,
  type StudioStoryWorkflowCompilationView,
} from "@/lib/studio-story-intake";

export interface StudioStoryPreviewSnapshot {
  daemonUrl: string;
  createdAt: string;
  viewer: "staff" | "owner";
  deckId: string;
  deck: HomepageStoryDeckResponse | null;
  review: StudioProductionReviewPacket | null;
  learning: StoryPublishLearningBrief | null;
  intakePacket: StoryFounderIntakePacket | null;
  degradedReason: string | null;
}

export interface StudioStoryPreviewInput {
  deck: HomepageStoryDeckResponse | null;
  review: StudioProductionReviewPacket | null;
  learning: StoryPublishLearningBrief | null;
  intakePacket?: StoryFounderIntakePacket | null;
  degradedReason: string | null;
}

export type StudioStoryWorkflowStateKey =
  | "compiled"
  | "blocked"
  | "missing_input"
  | "awaiting_approval"
  | "ready"
  | "degraded";

export type StudioStoryWorkflowStateTone = "ok" | "warn" | "error" | "muted";

export interface StudioStoryWorkflowStateView {
  key: StudioStoryWorkflowStateKey;
  label: string;
  detail: string;
  tone: StudioStoryWorkflowStateTone;
  active: boolean;
}

export interface StudioStoryPreviewSlideView {
  id: string;
  eyebrow: string;
  title: string;
  body: string;
  sourceLine: string | null;
  motionProfile: NarrativeSlideView["motionProfile"];
  evidenceRefCount: number;
  limitations: string[];
  ctaLabel: string | null;
  ctaHref: string | null;
}

export interface StudioStoryPreviewView {
  status: StudioPublicationSourceStatus | "degraded";
  deckId: string;
  workflowState: StudioStoryWorkflowStateView;
  workflowStates: StudioStoryWorkflowStateView[];
  workflowCompilation: StudioStoryWorkflowCompilationView | null;
  readinessLabel: string;
  slideCount: number;
  publicationEvidenceCount: number;
  safeEvidenceRefCount: number;
  summaryLines: string[];
  slides: StudioStoryPreviewSlideView[];
  publication: StudioPublicationsView | null;
  limitations: string[];
  deferredStates: StudioPublicationDeferredState[];
  nextActions: string[];
}

const noMutationLine =
  "Opening Preview does not publish, promote memory, write graph truth, call providers, or run tasks.";

export function buildStudioStoryPreviewView(input: StudioStoryPreviewInput): StudioStoryPreviewView {
  const publication = input.review && input.learning ? buildStudioPublicationsView(input.review, input.learning) : null;
  const intakeView = input.intakePacket ? buildStudioStoryIntakeView(input.intakePacket) : null;
  const workflowCompilation = intakeView?.workflowCompilation ?? null;
  const deckReady = Boolean(input.deck?.readiness.ready);
  const sourceSlides = input.deck && deckReady ? homepageStoryDeckToSlides(input.deck) : [];
  const slides = sourceSlides.map(previewSlideView);
  const publicationEvidenceCount = publication
    ? publication.componentCount + publication.metricCount + publication.publishEvidenceCount
    : 0;
  const safeEvidenceRefCount = safeEvidenceRefCountFor([
    ...(input.deck?.deck.evidenceRefs ?? []),
    ...(input.review?.evidenceRefs ?? []),
    ...(input.learning?.evidenceRefs ?? []),
    ...(input.intakePacket?.workflowCompilation?.evidenceRefs ?? []),
  ]);
  const degraded = Boolean(input.degradedReason);
  const missing = !degraded && (slides.length === 0 || !publication);
  const status = degraded ? "degraded" : missing ? "missing" : publication?.status ?? "manual";
  const workflowState = currentWorkflowState(workflowCompilation, Boolean(publication), slides.length, degraded);
  const workflowStates = workflowStateRows(workflowCompilation, Boolean(publication), slides.length, degraded);
  const limitations = stableSafeList([
    ...(input.deck?.deck.limitations ?? []),
    ...(input.deck?.profile.limitations ?? []),
    ...(input.deck?.refresh.limitations ?? []),
    ...(workflowCompilation?.limitations ?? []),
    ...(publication?.limitations ?? []),
    ...(input.degradedReason ? ["Local Story Preview evidence is unavailable right now."] : []),
  ]);
  const nextActions = stableSafeList([
    ...(workflowCompilation?.nextActions ?? []),
    ...(publication?.nextActions ?? []),
    ...(!workflowCompilation ? ["Submit safe Story Intake evidence for workflow state"] : []),
    ...(workflowCompilation?.status === "missing_input" ? workflowCompilation.missingInputs.map((item) => `Resolve ${item}`) : []),
    ...(slides.length === 0 ? ["Add live homepage story content"] : []),
    ...(!publication ? ["Add publication review evidence"] : []),
  ]);

  return {
    status,
    deckId: safeIdentifier(input.deck?.deck.deckId ?? input.review?.deckId ?? input.learning?.deckId ?? "homepage.story.v1"),
    workflowState,
    workflowStates,
    workflowCompilation,
    readinessLabel: deckReady ? "Preview deck ready" : "Preview evidence missing",
    slideCount: slides.length,
    publicationEvidenceCount,
    safeEvidenceRefCount,
    summaryLines: summaryLines(slides.length, publicationEvidenceCount, degraded, missing, workflowState),
    slides,
    publication,
    limitations,
    deferredStates: publication?.deferredStates ?? defaultDeferredStates(),
    nextActions,
  };
}

function previewSlideView(slide: NarrativeSlideView): StudioStoryPreviewSlideView {
  const fallback = safeText(slide.reducedMotionFallback, "Preview slide withheld");
  const title = safeText(slide.title, "Preview slide withheld");
  const body = safeText(slide.body, "Preview slide body withheld");
  return {
    id: slide.id,
    eyebrow: slide.eyebrow,
    title: isWithheldText(title) ? fallback : title,
    body: isWithheldText(body) ? fallback : body,
    sourceLine: slide.sourceLine ? safeText(slide.sourceLine, "") : null,
    motionProfile: slide.motionProfile,
    evidenceRefCount: safeEvidenceRefCountFor(slide.evidenceRefs),
    limitations: stableSafeList(slide.limitations),
    ctaLabel: slide.cta ? safeText(slide.cta.label, "") : null,
    ctaHref: slide.cta?.href ?? null,
  };
}

function isWithheldText(value: string): boolean {
  return value === "Public-safe content withheld pending review.";
}

function currentWorkflowState(
  workflow: StudioStoryWorkflowCompilationView | null,
  hasPublication: boolean,
  slideCount: number,
  degraded: boolean,
): StudioStoryWorkflowStateView {
  return workflowStateRows(workflow, hasPublication, slideCount, degraded).find((state) => state.active) ?? workflowState("blocked", true);
}

function workflowStateRows(
  workflow: StudioStoryWorkflowCompilationView | null,
  hasPublication: boolean,
  slideCount: number,
  degraded: boolean,
): StudioStoryWorkflowStateView[] {
  const compiled = workflow?.status === "compiled";
  const missingInput = workflow?.status === "missing_input";
  const blocked = !degraded && (!workflow || workflow.status === "blocked");
  const awaitingApproval = compiled && workflow.approvalGates.length > 0;
  const ready = compiled && !awaitingApproval && slideCount > 0 && hasPublication;

  return [
    workflowState("degraded", degraded, degraded ? "Ordo cannot read all local Preview evidence right now; nothing is treated as done." : undefined),
    workflowState(
      "missing_input",
      !degraded && missingInput,
      missingInput
        ? `${workflow.missingInputs.length} required item(s) still need attention.`
        : "No missing inputs are active.",
    ),
    workflowState(
      "blocked",
      blocked,
      blocked
        ? "Preview is blocked until Story Intake has a saved production plan."
        : "No closed blocker is active.",
    ),
    workflowState(
      "compiled",
      !degraded && compiled && !awaitingApproval && !ready,
      compiled
        ? `${workflow.taskCount} planned step(s) are saved for review only.`
        : "No production plan evidence is available.",
    ),
    workflowState(
      "awaiting_approval",
      !degraded && awaitingApproval,
      awaitingApproval
        ? `${workflow.approvalGates.length} approval gate(s) remain before publishing or provider work.`
        : "No approval gate is active.",
    ),
    workflowState(
      "ready",
      !degraded && ready,
      ready
        ? "Preview is ready for owner/staff review; this does not mean anything was published or run."
        : "Preview is not ready until compilation, slides, and publication evidence are present.",
    ),
  ];
}

function workflowState(
  key: StudioStoryWorkflowStateKey,
  active: boolean,
  detail?: string,
): StudioStoryWorkflowStateView {
  const labels: Record<StudioStoryWorkflowStateKey, string> = {
    compiled: "compiled",
    blocked: "blocked",
    missing_input: "missing input",
    awaiting_approval: "awaiting approval",
    ready: "ready",
    degraded: "degraded",
  };
  const defaultDetails: Record<StudioStoryWorkflowStateKey, string> = {
    compiled: "A production plan is saved and read-only.",
    blocked: "Preview is blocked until source evidence is available.",
    missing_input: "Required information is missing.",
    awaiting_approval: "A person still needs to approve the next step.",
    ready: "Preview is ready for owner/staff review.",
    degraded: "Local evidence needs attention.",
  };
  const tone: StudioStoryWorkflowStateTone =
    key === "ready" || key === "compiled"
      ? "ok"
      : key === "degraded"
        ? "error"
        : key === "blocked" || key === "missing_input" || key === "awaiting_approval"
          ? "warn"
          : "muted";
  return {
    key,
    label: labels[key],
    detail: detail ?? defaultDetails[key],
    tone: active ? tone : "muted",
    active,
  };
}

function summaryLines(
  slideCount: number,
  publicationEvidenceCount: number,
  degraded: boolean,
  missing: boolean,
  workflowState: StudioStoryWorkflowStateView,
): string[] {
  if (missing) {
    return [
      "No preview slides are available from the local story evidence yet.",
      `Workflow state is ${workflowState.label}.`,
      "Missing or unavailable publication evidence is shown instead of hidden.",
      noMutationLine,
    ];
  }
  if (degraded) {
    return [
      `${slideCount} preview slide(s) are assembled from local story evidence.`,
      `Workflow state is ${workflowState.label}.`,
      "Some Story publication evidence is degraded or unavailable.",
      noMutationLine,
    ];
  }
  return [
      `${slideCount} preview slide(s) are assembled from local story evidence.`,
    `Workflow state is ${workflowState.label}.`,
    `${publicationEvidenceCount} Story publication evidence item(s) are available for owner/staff review.`,
    noMutationLine,
  ];
}

function defaultDeferredStates(): StudioPublicationDeferredState[] {
  return [
    {
      key: "external_publishing",
      label: "External publishing not claimed",
      detail: "Preview reads do not publish or claim platform publication.",
      sourceStatus: "deferred",
    },
    {
      key: "live_provider",
      label: "Live provider not called",
      detail: "Preview reads use existing evidence and do not call model or image providers.",
      sourceStatus: "deferred",
    },
    {
      key: "memory_promotion",
      label: "Memory promotion not performed",
      detail: "Preview reads do not promote generated content into memory truth.",
      sourceStatus: "deferred",
    },
    {
      key: "graph_promotion",
      label: "Graph promotion not confirmed",
      detail: "Preview reads do not promote generated content into graph truth.",
      sourceStatus: "deferred",
    },
  ];
}

function safeEvidenceRefCountFor(refs: readonly string[]): number {
  return new Set(refs.filter((ref) => safeEvidenceRef(ref) !== null)).size;
}

function safeEvidenceRef(value: string | null | undefined): string | null {
  if (!value || isUnsafeText(value)) return null;
  if (/^(artifact|business_fact|content_analytics|content_event|memory_candidate|workflow_compilation|event|surface|tracked_entry_point|cta|offer):[A-Za-z0-9_.:-]+$/.test(value)) {
    return value;
  }
  return null;
}

function stableSafeList(values: readonly string[]): string[] {
  return [...new Set(values.map((value) => safeText(value, "")).filter(Boolean))].sort((left, right) => left.localeCompare(right));
}

function safeText(value: string | null | undefined, fallback: string): string {
  if (!value || isUnsafeText(value)) return fallback;
  return value.trim().slice(0, 240);
}

function safeIdentifier(value: string): string {
  if (isUnsafeText(value)) return "withheld";
  return value.replace(/[^A-Za-z0-9_.:-]+/g, "_").slice(0, 120) || "unknown";
}

function isUnsafeText(value: string): boolean {
  const normalized = value.toLowerCase().replace(/[_-]+/g, " ");
  return [
    "staff routing",
    "provider internal",
    "provider secret",
    "prompt internal",
    "raw policy",
    "policy internal",
    "owner only",
    "private artifact",
    "compiled plan",
    "task result private",
    "task private payload",
    "generated content candidate",
    "graph certainty",
    "secret:",
    "api key",
    "sk live",
  ].some((marker) => normalized.includes(marker));
}
