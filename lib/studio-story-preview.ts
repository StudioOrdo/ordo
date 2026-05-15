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

export interface StudioStoryPreviewSnapshot {
  daemonUrl: string;
  createdAt: string;
  viewer: "staff" | "owner";
  deckId: string;
  deck: HomepageStoryDeckResponse | null;
  review: StudioProductionReviewPacket | null;
  learning: StoryPublishLearningBrief | null;
  degradedReason: string | null;
}

export interface StudioStoryPreviewInput {
  deck: HomepageStoryDeckResponse | null;
  review: StudioProductionReviewPacket | null;
  learning: StoryPublishLearningBrief | null;
  degradedReason: string | null;
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
  "Preview reads do not publish, mutate analytics truth, promote memory, promote graph truth, call providers, or execute tasks.";

export function buildStudioStoryPreviewView(input: StudioStoryPreviewInput): StudioStoryPreviewView {
  const publication = input.review && input.learning ? buildStudioPublicationsView(input.review, input.learning) : null;
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
  ]);
  const degraded = Boolean(input.degradedReason);
  const missing = !degraded && (slides.length === 0 || !publication);
  const status = degraded ? "degraded" : missing ? "missing" : publication?.status ?? "manual";
  const limitations = stableSafeList([
    ...(input.deck?.deck.limitations ?? []),
    ...(input.deck?.profile.limitations ?? []),
    ...(input.deck?.refresh.limitations ?? []),
    ...(publication?.limitations ?? []),
    ...(input.degradedReason ? [input.degradedReason] : []),
  ]);
  const nextActions = stableSafeList([
    ...(publication?.nextActions ?? []),
    ...(slides.length === 0 ? ["Resolve daemon-backed homepage story deck"] : []),
    ...(!publication ? ["Resolve Story publication readiness evidence"] : []),
  ]);

  return {
    status,
    deckId: safeIdentifier(input.deck?.deck.deckId ?? input.review?.deckId ?? input.learning?.deckId ?? "homepage.story.v1"),
    readinessLabel: deckReady ? "Preview deck ready" : "Preview evidence missing",
    slideCount: slides.length,
    publicationEvidenceCount,
    safeEvidenceRefCount,
    summaryLines: summaryLines(slides.length, publicationEvidenceCount, degraded, missing),
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

function summaryLines(slideCount: number, publicationEvidenceCount: number, degraded: boolean, missing: boolean): string[] {
  if (missing) {
    return [
      "No protected preview slides are available from daemon-backed homepage story evidence.",
      "Missing or degraded publication evidence remains explicit.",
      noMutationLine,
    ];
  }
  if (degraded) {
    return [
      `${slideCount} protected preview slide(s) are assembled from daemon-backed homepage story evidence.`,
      "Some Story publication evidence is degraded or unavailable.",
      noMutationLine,
    ];
  }
  return [
    `${slideCount} protected preview slide(s) are assembled from daemon-backed homepage story evidence.`,
    `${publicationEvidenceCount} Story publication evidence component(s) are available for owner/staff review.`,
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
  if (/^(artifact|business_fact|content_analytics|content_event|memory_candidate|event|surface|tracked_entry_point|cta|offer):[A-Za-z0-9_.:-]+$/.test(value)) {
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
