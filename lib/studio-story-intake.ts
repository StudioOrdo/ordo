export type StudioStoryIntakeStatus = "ready" | "blocked" | "empty";

export interface StudioStoryIntakeRequest {
  intakeId: string;
  founderStory: string;
  businessStance: string;
  audience: string | null;
  evidenceRefs: string[];
}

export interface StudioStoryIntakeSnapshot {
  daemonUrl: string;
  createdAt: string;
  viewer: string;
  request: StudioStoryIntakeRequest | null;
  packet: StoryFounderIntakePacket | null;
  degradedReason: string | null;
  emptyReason: string | null;
}

export interface StoryIntakeClaim {
  claim: string;
  evidenceRefs: string[];
  reviewState: string;
  limitations: string[];
}

export interface StoryFounderIntakePublicDerivative {
  intakeId: string;
  summary: string;
  audience: string | null;
  claims: StoryIntakeClaim[];
  stylePreferences: string[];
  offerRefs: string[];
  ctaRefs: string[];
  evidenceRefs: string[];
  limitations: string[];
  visibility: string;
  memoryEffect: string;
}

export interface StoryFounderIntakeReadiness {
  status: string;
  narrativeDeckReady: boolean;
  missing: string[];
  evidenceRefs: string[];
  limitations: string[];
  liveProviderRequired: boolean;
  externalPublishingClaimed: boolean;
  automaticMemoryPromotion: boolean;
  confirmedGraphPromotion: boolean;
}

export interface StoryFounderIntakePacket {
  schemaVersion: string;
  intakeId: string;
  artifactRef: string;
  artifact: {
    id: string;
    artifactKind: string;
    title: string;
    status: string;
    visibilityCeiling: string;
    summary: string;
    sourceKind: string | null;
    sourceId: string | null;
    evidenceRefs: string[];
    provenance: Record<string, unknown>;
    contentHash: string;
    storageUri: string | null;
    healthStatus: string | null;
    createdByJobId: string | null;
    createdAt: string;
    updatedAt: string;
  };
  version: unknown | null;
  publicDerivative: StoryFounderIntakePublicDerivative;
  readiness: StoryFounderIntakeReadiness;
  mutationPerformed: boolean;
  approvalState: string;
  visibilityCeiling: string;
  liveProviderCalled: boolean;
  externalPublishingClaimed: boolean;
  memoryPromotionPerformed: boolean;
  confirmedGraphPromotion: boolean;
  event: unknown | null;
}

export interface StudioStoryIntakeView {
  status: StudioStoryIntakeStatus;
  intakeId: string;
  readinessLabel: string;
  narrativeDeckReady: boolean;
  publicSummary: string;
  audience: string;
  approvalState: string;
  visibilityCeiling: string;
  artifactRef: string;
  artifactKind: string;
  safeEvidenceRefCount: number;
  missingPrerequisites: string[];
  limitations: string[];
  claims: StudioStoryIntakeClaimView[];
  nextActions: string[];
  summaryLines: string[];
  deferredStates: StudioStoryIntakeDeferredState[];
}

export interface StudioStoryIntakeClaimView {
  claim: string;
  reviewState: string;
  evidenceRefCount: number;
  limitations: string[];
}

export interface StudioStoryIntakeDeferredState {
  key: string;
  label: string;
  detail: string;
}

const unsafeTextPatterns = [
  /rawprompt/i,
  /sk[_-]?live/i,
  /provider\s+internal/i,
  /prompt\s+internal/i,
  /raw\s+policy/i,
  /owner[-\s]+only/i,
  /private\s+artifact\s+text/i,
  /generated[-\s]+content\s+candidate\s+text/i,
  /graph\s+certainty/i,
  /staff\s+routing/i,
  /compiled[-\s]+plan/i,
  /task\s+private\s+payload/i,
  /privateNotes/i,
  /secret/i,
];

export function buildStudioStoryIntakeView(packet: StoryFounderIntakePacket): StudioStoryIntakeView {
  const missingPrerequisites = safeList(packet.readiness.missing).map(humanizeIdentifier);
  const limitations = safeList([
    ...packet.publicDerivative.limitations,
    ...packet.readiness.limitations,
  ]).map(humanizeIdentifier);
  const claims = packet.publicDerivative.claims
    .map(claimView)
    .filter((claim) => claim.claim.length > 0);
  const safeEvidenceRefCount = safeEvidenceRefCountFor([
    ...packet.readiness.evidenceRefs,
    ...packet.publicDerivative.evidenceRefs,
    ...packet.artifact.evidenceRefs,
  ]);
  const narrativeDeckReady = Boolean(packet.readiness.narrativeDeckReady);
  const status: StudioStoryIntakeStatus = narrativeDeckReady ? "ready" : "blocked";
  const deferredStates = buildDeferredStates(packet);
  const nextActions = nextActionsFor(status, missingPrerequisites);

  return {
    status,
    intakeId: safeIdentifier(packet.intakeId),
    readinessLabel: narrativeDeckReady ? "Ready for narrative deck" : "Blocked before narrative deck",
    narrativeDeckReady,
    publicSummary: safeText(packet.publicDerivative.summary, "No public-safe intake summary is available."),
    audience: safeText(packet.publicDerivative.audience, "Audience not supplied"),
    approvalState: humanizeIdentifier(safeIdentifier(packet.approvalState)),
    visibilityCeiling: safeIdentifier(packet.visibilityCeiling),
    artifactRef: safeArtifactRef(packet.artifactRef) ?? "artifact:withheld",
    artifactKind: safeIdentifier(packet.artifact.artifactKind),
    safeEvidenceRefCount,
    missingPrerequisites,
    limitations,
    claims,
    nextActions,
    summaryLines: summaryLines(status, safeEvidenceRefCount, missingPrerequisites.length),
    deferredStates,
  };
}

export function emptyStudioStoryIntakeView(): StudioStoryIntakeView {
  return {
    status: "empty",
    intakeId: "none",
    readinessLabel: "Readiness unknown",
    narrativeDeckReady: false,
    publicSummary: "No Story founder intake has been submitted from this workbench yet.",
    audience: "Audience not supplied",
    approvalState: "Not available",
    visibilityCeiling: "none",
    artifactRef: "artifact:none",
    artifactKind: "story.founder_intake",
    safeEvidenceRefCount: 0,
    missingPrerequisites: ["Protected founder intake evidence"],
    limitations: ["Readiness is unknown until protected intake evidence is available."],
    claims: [],
    nextActions: ["Submit protected founder intake evidence"],
    summaryLines: [
      "No Story founder intake has been submitted from this workbench yet.",
      "Readiness is unknown until protected intake evidence is available.",
      "Provider execution, publishing, memory promotion, graph promotion, rewards, and task execution are not claimed.",
    ],
    deferredStates: baselineDeferredStates(),
  };
}

function claimView(claim: StoryIntakeClaim): StudioStoryIntakeClaimView {
  return {
    claim: safeText(claim.claim, ""),
    reviewState: humanizeIdentifier(safeIdentifier(claim.reviewState)),
    evidenceRefCount: safeEvidenceRefCountFor(claim.evidenceRefs),
    limitations: safeList(claim.limitations).map(humanizeIdentifier),
  };
}

function nextActionsFor(status: StudioStoryIntakeStatus, missingPrerequisites: readonly string[]): string[] {
  if (status === "ready") {
    return ["Create narrative deck", "Review public derivative", "Prepare Story Pack production review"];
  }
  const missingActions = missingPrerequisites.map((item) => `Add ${item.toLowerCase()}`);
  return [...missingActions, "Keep readiness blocked until evidence is safe"];
}

function summaryLines(status: StudioStoryIntakeStatus, safeEvidenceRefCount: number, missingCount: number): string[] {
  if (status === "ready") {
    return [
      "Founder intake is ready for narrative deck assembly.",
      `${safeEvidenceRefCount} safe evidence ref(s) support the public derivative.`,
      "Provider execution, publishing, memory promotion, graph promotion, rewards, and task execution are not claimed.",
    ];
  }
  return [
    "Founder intake is blocked before narrative deck assembly.",
    `${missingCount} missing prerequisite(s) remain explicit.`,
    "Provider execution, publishing, memory promotion, graph promotion, rewards, and task execution are not claimed.",
  ];
}

function buildDeferredStates(packet: StoryFounderIntakePacket): StudioStoryIntakeDeferredState[] {
  const states = baselineDeferredStates();
  if (packet.version !== null) {
    states.push({
      key: "private_version_metadata_withheld",
      label: "Private version metadata withheld",
      detail: "The UI uses artifact refs and public derivatives instead of rendering owner-scoped artifact version metadata.",
    });
  }
  return states;
}

function baselineDeferredStates(): StudioStoryIntakeDeferredState[] {
  return [
    {
      key: "live_provider",
      label: "Live provider not called",
      detail: "Story intake readiness is deterministic and does not call live model or image providers.",
    },
    {
      key: "publishing",
      label: "Publishing not performed",
      detail: "The intake workbench does not publish the homepage or claim external publication.",
    },
    {
      key: "memory_graph",
      label: "Memory and graph promotion not performed",
      detail: "Generated-content memory and graph truth require later explicit review.",
    },
  ];
}

function safeEvidenceRefCountFor(refs: readonly string[]): number {
  return new Set(refs.filter((ref) => safeArtifactRef(ref) !== null)).size;
}

function safeArtifactRef(value: string | null | undefined): string | null {
  if (!value || isUnsafeText(value)) return null;
  if (/^(artifact|business_fact|content_analytics|content_event|memory_candidate|job|event|surface|tracked_entry_point|cta|offer):[A-Za-z0-9_.:-]+$/.test(value)) {
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
