export type StudioPublicationSourceStatus = "measured" | "manual" | "missing" | "deferred" | "unknown";
export type StudioPublicationStatusTone = "ok" | "warn" | "error";

export interface StudioProductionReviewComponent {
  key: string;
  status: string;
  artifactRef: string | null;
  artifactKind: string | null;
  title: string | null;
  summary: string | null;
  visibility: string;
  evidenceStatus: string;
  evidenceRefs: string[];
  limitations: string[];
  recommendedNextAction: string;
}

export interface StudioProductionReviewPacket {
  schemaVersion: string;
  status: string;
  audience: string;
  readOnly: boolean;
  mutationPerformed: boolean;
  confirmedGraphPromotion: boolean;
  liveProviderCalled: boolean;
  externalPublishingClaimed: boolean;
  deckId: string | null;
  evidenceRefs: string[];
  limitations: string[];
  missingPrerequisites: string[];
  recommendedNextActions: string[];
  components: StudioProductionReviewComponent[];
  analyticsSummary: unknown | null;
  memoryReviewPackets: StudioMemoryReviewPacket[];
}

export interface StoryPublishLearningMetric {
  key: string;
  label: string;
  value: number;
  sourceStatus: string;
  evidenceRefs: string[];
}

export interface StoryPublishLearningSource {
  sourceKind: string;
  sourceId: string;
  status: string;
  sourceStatus: string;
  evidenceRefs: string[];
  limitations: string[];
}

export interface StoryPublishMemoryLearningSummary {
  candidateCount: number;
  stateCounts: StoryPublishLearningMetric[];
  evidenceRefs: string[];
  limitations: string[];
  confirmedGraphPromotion: boolean;
  memoryPromotionPerformed: boolean;
}

export interface StoryPublishOutcomeLearningSummary {
  outcomeCount: number;
  attributionState: string;
  evidenceRefs: string[];
  limitations: string[];
}

export interface StoryPublishRewardLearningSummary {
  rewardEventCount: number;
  grantedCount: number;
  evidenceRefs: string[];
  limitations: string[];
}

export interface StudioMemoryReviewPacket {
  candidateCount?: number;
  evidenceRefs?: string[];
  limitations?: string[];
}

export interface StoryPublishLearningBrief {
  schemaVersion: string;
  status: string;
  audience: string;
  deckId: string;
  readOnly: boolean;
  mutationPerformed: boolean;
  confirmedGraphPromotion: boolean;
  memoryPromotionPerformed: boolean;
  liveProviderCalled: boolean;
  externalPublishingClaimed: boolean;
  sourceStatus: StoryPublishLearningMetric[];
  contentMetrics: StoryPublishLearningMetric[];
  publishEvidence: StoryPublishLearningSource[];
  memorySummary: StoryPublishMemoryLearningSummary;
  outcomeSummary: StoryPublishOutcomeLearningSummary;
  rewardSummary: StoryPublishRewardLearningSummary;
  evidenceRefs: string[];
  limitations: string[];
  recommendedNextActions: string[];
  analyticsSummary: unknown | null;
  memoryReviewPackets: StudioMemoryReviewPacket[];
}

export interface StudioPublicationsSnapshot {
  daemonUrl: string;
  createdAt: string;
  deckId: string;
  artifactIds: string[];
  viewer: "staff" | "owner";
  review: StudioProductionReviewPacket | null;
  learning: StoryPublishLearningBrief | null;
  degradedReason: string | null;
}

export interface StudioPublicationComponentView {
  key: string;
  label: string;
  status: string;
  evidenceStatus: StudioPublicationSourceStatus;
  artifactRef: string | null;
  artifactKind: string | null;
  title: string;
  summary: string;
  visibility: string;
  evidenceRefCount: number;
  limitations: string[];
  nextAction: string;
}

export interface StudioPublicationMetricView {
  key: string;
  label: string;
  value: number;
  sourceStatus: StudioPublicationSourceStatus;
  evidenceRefCount: number;
}

export interface StudioPublicationSourceView {
  key: string;
  label: string;
  status: string;
  sourceStatus: StudioPublicationSourceStatus;
  evidenceRefCount: number;
  limitations: string[];
}

export interface StudioPublicationDeferredState {
  key: string;
  label: string;
  detail: string;
  sourceStatus: StudioPublicationSourceStatus;
}

export interface StudioPublicationsView {
  status: StudioPublicationSourceStatus;
  reviewStatus: string;
  learningStatus: string;
  deckId: string;
  componentCount: number;
  metricCount: number;
  publishEvidenceCount: number;
  safeEvidenceRefCount: number;
  memoryCandidateCount: number;
  missingOrDeferredCount: number;
  sourceStatusCounts: Record<StudioPublicationSourceStatus, number>;
  summaryLines: string[];
  components: StudioPublicationComponentView[];
  sourceStatus: StudioPublicationMetricView[];
  contentMetrics: StudioPublicationMetricView[];
  publishEvidence: StudioPublicationSourceView[];
  learningLimitations: string[];
  deferredStates: StudioPublicationDeferredState[];
  nextActions: string[];
}

const statuses: readonly StudioPublicationSourceStatus[] = ["measured", "manual", "missing", "deferred", "unknown"];

const unsafeTextPatterns = [
  /rawprompt/i,
  /sk_live/i,
  /provider\s+internal/i,
  /prompt\s+internal/i,
  /private\s+artifact\s+text/i,
  /generated[-\s]+content\s+candidate\s+text/i,
  /graph\s+certainty/i,
  /staff\s+routing/i,
  /owner[-\s]+only\s+data/i,
  /compiled[-\s]+plan/i,
  /task\s+private\s+payload/i,
];

export function buildStudioPublicationsView(
  review: StudioProductionReviewPacket,
  learning: StoryPublishLearningBrief,
): StudioPublicationsView {
  const components = review.components.map(componentView);
  const sourceStatus = learning.sourceStatus.map(metricView);
  const contentMetrics = learning.contentMetrics.map(metricView);
  const publishEvidence = learning.publishEvidence.map(sourceView);
  const learningLimitations = safeList([
    ...learning.limitations,
    ...learning.memorySummary.limitations,
    ...learning.outcomeSummary.limitations,
    ...learning.rewardSummary.limitations,
  ]);
  const statusCounts = emptyStatusCounts();
  for (const component of components) addStatus(statusCounts, component.evidenceStatus);
  for (const metric of sourceStatus) addStatus(statusCounts, metric.sourceStatus);
  for (const metric of contentMetrics) addStatus(statusCounts, metric.sourceStatus);
  for (const source of publishEvidence) addStatus(statusCounts, source.sourceStatus);
  for (const state of learning.memorySummary.stateCounts) addStatus(statusCounts, normalizeSourceStatus(state.sourceStatus));

  const safeEvidenceRefs = safeEvidenceRefCount([
    ...review.evidenceRefs,
    ...learning.evidenceRefs,
    ...learning.memorySummary.evidenceRefs,
    ...learning.outcomeSummary.evidenceRefs,
    ...learning.rewardSummary.evidenceRefs,
  ]);
  const deferredStates = buildDeferredStates(review, learning);
  const nextActions = safeList([...review.recommendedNextActions, ...learning.recommendedNextActions]).map(humanizeIdentifier);
  const missingOrDeferredCount = statusCounts.missing + statusCounts.deferred;
  const status = overallStatus(review.status, learning.status, missingOrDeferredCount);

  return {
    status,
    reviewStatus: safeIdentifier(review.status),
    learningStatus: safeIdentifier(learning.status),
    deckId: safeText(review.deckId ?? learning.deckId, "homepage.story.v1"),
    componentCount: components.length,
    metricCount: sourceStatus.length + contentMetrics.length,
    publishEvidenceCount: publishEvidence.length,
    safeEvidenceRefCount: safeEvidenceRefs,
    memoryCandidateCount: Number.isSafeInteger(learning.memorySummary.candidateCount) ? learning.memorySummary.candidateCount : 0,
    missingOrDeferredCount,
    sourceStatusCounts: statusCounts,
    summaryLines: summaryLines(components.length, sourceStatus.length + contentMetrics.length, publishEvidence.length, missingOrDeferredCount),
    components,
    sourceStatus,
    contentMetrics,
    publishEvidence,
    learningLimitations,
    deferredStates,
    nextActions,
  };
}

export function studioPublicationStatusTone(status: string): StudioPublicationStatusTone {
  const normalized = normalizeSourceStatus(status);
  if (normalized === "measured") return "ok";
  if (normalized === "manual" || normalized === "deferred") return "warn";
  return "error";
}

export function normalizeSourceStatus(status: string): StudioPublicationSourceStatus {
  if (status === "measured" || status === "manual" || status === "missing" || status === "deferred") {
    return status;
  }
  if (status === "ready" || status === "complete" || status === "published" || status === "staged") {
    return "measured";
  }
  if (status === "partial" || status === "candidate" || status === "needs_review") {
    return "manual";
  }
  return "unknown";
}

export function studioPublicationStatusLabel(status: string): string {
  return humanizeIdentifier(status);
}

export function studioPublicationStatusCounts(): readonly StudioPublicationSourceStatus[] {
  return statuses;
}

function componentView(component: StudioProductionReviewComponent): StudioPublicationComponentView {
  return {
    key: safeIdentifier(component.key),
    label: humanizeIdentifier(component.key),
    status: safeIdentifier(component.status),
    evidenceStatus: normalizeSourceStatus(component.evidenceStatus),
    artifactRef: safeArtifactRef(component.artifactRef),
    artifactKind: component.artifactKind ? safeIdentifier(component.artifactKind) : null,
    title: safeText(component.title, humanizeIdentifier(component.key)),
    summary: safeText(component.summary, "Summary unavailable."),
    visibility: safeIdentifier(component.visibility),
    evidenceRefCount: safeEvidenceRefCount(component.evidenceRefs),
    limitations: safeList(component.limitations),
    nextAction: humanizeIdentifier(component.recommendedNextAction),
  };
}

function metricView(metric: StoryPublishLearningMetric): StudioPublicationMetricView {
  return {
    key: safeIdentifier(metric.key),
    label: safeText(metric.label, humanizeIdentifier(metric.key)),
    value: Number.isFinite(metric.value) ? Math.trunc(metric.value) : 0,
    sourceStatus: normalizeSourceStatus(metric.sourceStatus),
    evidenceRefCount: safeEvidenceRefCount(metric.evidenceRefs),
  };
}

function sourceView(source: StoryPublishLearningSource): StudioPublicationSourceView {
  const sourceKey = `${safeIdentifier(source.sourceKind)}:${safeIdentifier(source.sourceId)}`;
  return {
    key: sourceKey,
    label: humanizeIdentifier(source.sourceKind),
    status: safeIdentifier(source.status),
    sourceStatus: normalizeSourceStatus(source.sourceStatus),
    evidenceRefCount: safeEvidenceRefCount(source.evidenceRefs),
    limitations: safeList(source.limitations),
  };
}

function buildDeferredStates(
  review: StudioProductionReviewPacket,
  learning: StoryPublishLearningBrief,
): StudioPublicationDeferredState[] {
  const states: StudioPublicationDeferredState[] = [];
  if (!review.externalPublishingClaimed || !learning.externalPublishingClaimed) {
    states.push({
      key: "external_publishing",
      label: "External publishing not claimed",
      detail: "Publication evidence remains local/manual until governed platform adapters exist.",
      sourceStatus: "deferred",
    });
  }
  if (!review.liveProviderCalled || !learning.liveProviderCalled) {
    states.push({
      key: "live_provider",
      label: "Live provider not called",
      detail: "Default validation remains deterministic and does not require live model or image providers.",
      sourceStatus: "deferred",
    });
  }
  if (!review.confirmedGraphPromotion || !learning.confirmedGraphPromotion) {
    states.push({
      key: "graph_promotion",
      label: "Graph promotion not confirmed",
      detail: "Generated content remains evidence for review and is not treated as graph truth.",
      sourceStatus: "deferred",
    });
  }
  if (!learning.memoryPromotionPerformed || !learning.memorySummary.memoryPromotionPerformed) {
    states.push({
      key: "memory_promotion",
      label: "Memory promotion not performed",
      detail: "Candidate memory needs explicit owner/staff review before promotion.",
      sourceStatus: "deferred",
    });
  }
  return states;
}

function summaryLines(componentCount: number, metricCount: number, publishEvidenceCount: number, missingOrDeferredCount: number): string[] {
  if (componentCount === 0 && metricCount === 0) {
    return [
      "No daemon-backed Story production review components are available yet.",
      "No Story publish learning metrics are available yet.",
      "Missing or deferred publication evidence remains explicit.",
    ];
  }
  return [
    `${componentCount} Story production component(s) are represented by daemon review evidence.`,
    `${metricCount} learning metric(s) and ${publishEvidenceCount} publication evidence source(s) are available for owner/staff review.`,
    `${missingOrDeferredCount} missing or deferred signal(s) remain explicit instead of being treated as publication success.`,
  ];
}

function overallStatus(reviewStatus: string, learningStatus: string, missingOrDeferredCount: number): StudioPublicationSourceStatus {
  if (reviewStatus === "missing" || learningStatus === "missing") {
    return "missing";
  }
  if (missingOrDeferredCount > 0 || reviewStatus === "partial" || learningStatus === "partial") {
    return "manual";
  }
  if (reviewStatus === "complete" && learningStatus === "complete") {
    return "measured";
  }
  return "unknown";
}

function emptyStatusCounts(): Record<StudioPublicationSourceStatus, number> {
  return {
    measured: 0,
    manual: 0,
    missing: 0,
    deferred: 0,
    unknown: 0,
  };
}

function addStatus(counts: Record<StudioPublicationSourceStatus, number>, status: StudioPublicationSourceStatus) {
  counts[status] += 1;
}

function safeEvidenceRefCount(refs: readonly string[]): number {
  return new Set(refs.filter((ref) => safeArtifactRef(ref) !== null)).size;
}

function safeArtifactRef(value: string | null | undefined): string | null {
  if (!value || isUnsafeText(value)) return null;
  if (/^(artifact|content_analytics|content_event|memory_candidate|business_fact|job|event|surface|tracked_entry_point):[A-Za-z0-9_.:-]+$/.test(value)) {
    return value;
  }
  return null;
}

function safeList(values: readonly string[]): string[] {
  return [...new Set(values.map((value) => safeText(value, "")).filter(Boolean))].sort((left, right) => left.localeCompare(right));
}

function safeText(value: string | null | undefined, fallback: string): string {
  if (!value || isUnsafeText(value)) return fallback;
  return humanizeIdentifier(value.trim()).slice(0, 240);
}

function safeIdentifier(value: string): string {
  if (isUnsafeText(value)) return "withheld";
  return value.replace(/[^A-Za-z0-9_.:-]+/g, "_").slice(0, 120) || "unknown";
}

function isUnsafeText(value: string): boolean {
  return unsafeTextPatterns.some((pattern) => pattern.test(value));
}

function humanizeIdentifier(value: string): string {
  const normalized = value
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/[_\s.-]+/g, " ")
    .toLowerCase()
    .trim();
  if (!normalized) return "Unknown";
  return normalized.charAt(0).toUpperCase() + normalized.slice(1);
}
