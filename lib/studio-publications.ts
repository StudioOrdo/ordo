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

export interface GeneratedContentMemoryReviewItem {
  candidateId: string;
  memoryKind: string;
  memoryTier: string;
  candidateState: string;
  confidence: number;
  summaryText: string | null;
  body?: string | null;
  bodyRedacted: boolean;
  sourceArtifactRefs: string[];
  workflowRefs: string[];
  evidenceRefs: string[];
  limitations: string[];
  approvalEvidenceRefs: string[];
  publicationEvidenceRefs: string[];
  feedbackEvidenceRefs: string[];
  outcomeEvidenceRefs: string[];
  rejectionEvidenceRefs: string[];
  memoryEffect: string;
  recommendedReviewAction: string;
  confirmedGraphPromotion: boolean;
}

export interface GeneratedContentMemoryPromotionReadinessOrigin {
  artifactRef: string;
  artifactVersionRef?: string | null;
  workflowTemplateRef?: string | null;
  workflowCompilationRef?: string | null;
  jobRef?: string | null;
  actorRef?: string | null;
}

export interface GeneratedContentMemoryPromotionReadinessPacket {
  schemaVersion: string;
  candidateId: string;
  artifactId: string;
  artifactVersionId?: string | null;
  sourceArtifactKind: string;
  audience: string;
  readOnly: boolean;
  promotionReady: boolean;
  currentCandidateState: string;
  memoryKind: string;
  memoryTier: string;
  visibilityClass: string;
  memoryEffect: string;
  origin: GeneratedContentMemoryPromotionReadinessOrigin;
  evidenceRefs: string[];
  decisionRefs: string[];
  blockers: string[];
  allowedNextAction: string;
  limitations: string[];
  memoryPromotionPerformed: boolean;
  confirmedGraphPromotion: boolean;
  vectorMutationPerformed: boolean;
  packStateMutationPerformed: boolean;
  liveProviderCalled: boolean;
}

export interface GeneratedContentMemoryReviewPacket {
  schemaVersion: string;
  artifactId: string;
  sourceArtifactKind: string;
  audience: string;
  candidateCount: number;
  sourceArtifactRefs: string[];
  workflowRefs: string[];
  evidenceRefs: string[];
  limitations: string[];
  items: GeneratedContentMemoryReviewItem[];
  promotionReadinessPackets?: GeneratedContentMemoryPromotionReadinessPacket[];
  extensionPoints: string[];
  confirmedGraphPromotion: boolean;
  liveProviderCalled: boolean;
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
  memoryReviewPackets: GeneratedContentMemoryReviewPacket[];
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

export interface StudioMemoryReviewItemView {
  candidateId: string;
  label: string;
  state: string;
  memoryKind: string;
  memoryTier: string;
  confidencePercent: number;
  summary: string;
  evidenceRefs: string[];
  evidenceRefCount: number;
  limitations: string[];
  memoryEffect: string;
  recommendedReviewAction: string;
  promotionReady: boolean;
  readinessState: "ready" | "blocked";
  readinessBlockers: string[];
  readinessAllowedNextAction: string;
  readinessEvidenceRefs: string[];
  readinessDecisionRefs: string[];
  readinessEvidenceRefCount: number;
  readinessDecisionRefCount: number;
  visibilityClass: string;
  memoryPromotionPerformed: boolean;
  vectorMutationPerformed: boolean;
  packStateMutationPerformed: boolean;
  canApprove: boolean;
  canReject: boolean;
  confirmedGraphPromotion: boolean;
}

export interface StudioMemoryReviewPacketView {
  artifactId: string;
  sourceArtifactKind: string;
  audience: string;
  candidateCount: number;
  evidenceRefs: string[];
  evidenceRefCount: number;
  promotionReadyCount: number;
  readinessBlockerCount: number;
  limitations: string[];
  extensionPoints: string[];
  confirmedGraphPromotion: boolean;
  liveProviderCalled: boolean;
  items: StudioMemoryReviewItemView[];
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
  memoryReviewPackets: StudioMemoryReviewPacketView[];
  reviewLimitations: string[];
  learningLimitations: string[];
  limitations: string[];
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
  generatedMemoryReviewPackets: GeneratedContentMemoryReviewPacket[] = [],
): StudioPublicationsView {
  const components = review.components.map(componentView);
  const sourceStatus = learning.sourceStatus.map(metricView);
  const contentMetrics = learning.contentMetrics.map(metricView);
  const publishEvidence = learning.publishEvidence.map(sourceView);
  const memoryReviewPackets = generatedMemoryReviewPackets.map(memoryReviewPacketView);
  const reviewLimitations = safeList(review.limitations);
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
    ...memoryReviewPackets.flatMap((packet) => packet.evidenceRefs),
    ...memoryReviewPackets.flatMap((packet) => packet.items.flatMap((item) => item.evidenceRefs)),
    ...memoryReviewPackets.flatMap((packet) => packet.items.flatMap((item) => item.readinessEvidenceRefs)),
    ...memoryReviewPackets.flatMap((packet) => packet.items.flatMap((item) => item.readinessDecisionRefs)),
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
    memoryCandidateCount: Math.max(
      Number.isSafeInteger(learning.memorySummary.candidateCount) ? learning.memorySummary.candidateCount : 0,
      memoryReviewPackets.reduce((total, packet) => total + packet.candidateCount, 0),
    ),
    missingOrDeferredCount,
    sourceStatusCounts: statusCounts,
    summaryLines: summaryLines(components.length, sourceStatus.length + contentMetrics.length, publishEvidence.length, missingOrDeferredCount),
    components,
    sourceStatus,
    contentMetrics,
    publishEvidence,
    memoryReviewPackets,
    reviewLimitations,
    learningLimitations,
    limitations: safeList([...reviewLimitations, ...learningLimitations]),
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
  if (status === "fixture") {
    return "deferred";
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

function memoryReviewPacketView(packet: GeneratedContentMemoryReviewPacket): StudioMemoryReviewPacketView {
  const readinessPackets = Array.isArray(packet.promotionReadinessPackets) ? packet.promotionReadinessPackets : [];
  const readinessByCandidate = new Map(readinessPackets.map((readiness) => [safeIdentifier(readiness.candidateId), readiness]));
  const items = packet.items.map((item) => memoryReviewItemView(item, readinessByCandidate.get(safeIdentifier(item.candidateId))));
  const promotionReadyCount = readinessPackets.filter((readiness) => readiness.promotionReady === true).length;
  const readinessBlockerCount = readinessPackets.reduce((total, readiness) => total + safeList(readiness.blockers).length, 0);
  return {
    artifactId: safeText(packet.artifactId, "Unknown artifact"),
    sourceArtifactKind: safeIdentifier(packet.sourceArtifactKind),
    audience: safeIdentifier(packet.audience),
    candidateCount: Number.isSafeInteger(packet.candidateCount) ? Math.max(0, packet.candidateCount) : items.length,
    evidenceRefs: safeEvidenceRefs(packet.evidenceRefs),
    evidenceRefCount: safeEvidenceRefCount(packet.evidenceRefs),
    promotionReadyCount,
    readinessBlockerCount,
    limitations: safeList(packet.limitations),
    extensionPoints: safeList(packet.extensionPoints).map(humanizeIdentifier),
    confirmedGraphPromotion: packet.confirmedGraphPromotion === true,
    liveProviderCalled: packet.liveProviderCalled === true,
    items,
  };
}

function memoryReviewItemView(
  item: GeneratedContentMemoryReviewItem,
  readiness?: GeneratedContentMemoryPromotionReadinessPacket,
): StudioMemoryReviewItemView {
  const candidateId = safeIdentifier(item.candidateId);
  const evidenceRefs = safeEvidenceRefs(item.evidenceRefs);
  const state = safeIdentifier(item.candidateState);
  const readinessBlockers = safeList(readiness?.blockers ?? []);
  const readinessEvidenceRefs = safeEvidenceRefs(readiness?.evidenceRefs ?? []);
  const readinessDecisionRefs = safeEvidenceRefs(readiness?.decisionRefs ?? []);
  return {
    candidateId,
    label: humanizeIdentifier(candidateId),
    state,
    memoryKind: safeIdentifier(item.memoryKind),
    memoryTier: safeIdentifier(item.memoryTier),
    confidencePercent: Number.isFinite(item.confidence) ? Math.max(0, Math.min(100, Math.round(item.confidence * 100))) : 0,
    summary: safeText(item.summaryText, "Candidate summary withheld."),
    evidenceRefs,
    evidenceRefCount: evidenceRefs.length,
    limitations: safeList(item.limitations),
    memoryEffect: safeIdentifier(item.memoryEffect),
    recommendedReviewAction: humanizeIdentifier(item.recommendedReviewAction),
    promotionReady: readiness?.promotionReady === true,
    readinessState: readiness?.promotionReady === true ? "ready" : "blocked",
    readinessBlockers,
    readinessAllowedNextAction: humanizeIdentifier(readiness?.allowedNextAction ?? "readiness_packet_unavailable"),
    readinessEvidenceRefs,
    readinessDecisionRefs,
    readinessEvidenceRefCount: readinessEvidenceRefs.length,
    readinessDecisionRefCount: readinessDecisionRefs.length,
    visibilityClass: safeIdentifier(readiness?.visibilityClass ?? "unknown"),
    memoryPromotionPerformed: readiness?.memoryPromotionPerformed === true,
    vectorMutationPerformed: readiness?.vectorMutationPerformed === true,
    packStateMutationPerformed: readiness?.packStateMutationPerformed === true,
    canApprove: state === "proposed",
    canReject: state === "proposed",
    confirmedGraphPromotion: item.confirmedGraphPromotion === true || readiness?.confirmedGraphPromotion === true,
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
  return safeEvidenceRefs(refs).length;
}

function safeEvidenceRefs(refs: readonly string[]): string[] {
  return [...new Set(refs.filter((ref) => safeArtifactRef(ref) !== null))].sort((left, right) => left.localeCompare(right));
}

function safeArtifactRef(value: string | null | undefined): string | null {
  if (!value || isUnsafeText(value)) return null;
  if (/^(artifact|artifact_version|artifact_review|approval|content_analytics|content_event|feedback|memory_candidate|business_fact|job|event|outcome|publication|surface|tracked_entry_point|workflow_compilation|workflow_template):[A-Za-z0-9_.:-]+$/.test(value)) {
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
