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
  workflowCompilation?: StoryWorkflowCompilationEvidence | null;
  mutationPerformed: boolean;
  approvalState: string;
  visibilityCeiling: string;
  liveProviderCalled: boolean;
  externalPublishingClaimed: boolean;
  memoryPromotionPerformed: boolean;
  confirmedGraphPromotion: boolean;
  event: unknown | null;
}

export interface StoryWorkflowCompilationEvidence {
  status: string;
  templateId: string;
  templateVersion: number;
  idempotencyKey: string;
  compilationRef: string | null;
  inputHash: string | null;
  evidenceRefs: string[];
  missingInputs: string[];
  limitations: string[];
  safeNextActions: string[];
  resolvedVariables: StoryWorkflowResolvedVariable[];
  taskBindings: StoryWorkflowTaskBindingEvidence[];
  fanoutGroups: StoryWorkflowFanoutEvidence[];
  approvalGates: StoryWorkflowApprovalGateEvidence[];
  providerRequirements: StoryWorkflowProviderRequirementEvidence[];
  liveProviderRequired: boolean;
  taskExecutionPerformed: boolean;
  externalPublishingClaimed: boolean;
  memoryPromotionPerformed: boolean;
  confirmedGraphPromotion: boolean;
}

export interface StoryWorkflowResolvedVariable {
  key: string;
  sourceKind: string;
  visibility: string;
  evidenceRefCount: number;
  valueExposed: boolean;
}

export interface StoryWorkflowTaskBindingEvidence {
  key: string;
  method: string;
  dependsOn: string[];
  visibility: string;
  fanout: string | null;
  providerRequirement: string | null;
  outputArtifactKind: string | null;
}

export interface StoryWorkflowFanoutEvidence {
  key: string;
  itemCount: number;
  maxItems: number;
}

export interface StoryWorkflowApprovalGateEvidence {
  key: string;
  action: string;
  required: boolean;
}

export interface StoryWorkflowProviderRequirementEvidence {
  key: string;
  capability: string;
  mode: string;
  egress: string;
  visibility: string;
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
  workflowCompilation: StudioStoryWorkflowCompilationView | null;
  nextActions: string[];
  summaryLines: string[];
  deferredStates: StudioStoryIntakeDeferredState[];
}

export interface StudioStoryWorkflowCompilationView {
  status: "compiled" | "blocked" | "missing_input";
  templateLabel: string;
  templateVersion: number;
  compilationRef: string;
  safeEvidenceRefCount: number;
  missingInputs: string[];
  limitations: string[];
  nextActions: string[];
  variableCount: number;
  taskCount: number;
  fanoutSummary: string;
  approvalGates: string[];
  providerRequirements: string[];
  taskBindings: StudioStoryWorkflowTaskBindingView[];
}

export interface StudioStoryWorkflowTaskBindingView {
  key: string;
  method: string;
  outputArtifactKind: string;
  visibility: string;
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
  /provider[\s_-]+internal/i,
  /prompt[\s_-]+internal/i,
  /raw[\s_-]+policy/i,
  /owner[\s_-]+only/i,
  /private[\s_-]+artifact[\s_-]+text/i,
  /generated[\s_-]+content[\s_-]+candidate[\s_-]+text/i,
  /graph[\s_-]+certainty/i,
  /staff[\s_-]+routing/i,
  /compiled[\s_-]+plan/i,
  /task[\s_-]+private[\s_-]+payload/i,
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
  const workflowCompilation = workflowCompilationView(packet.workflowCompilation ?? null);
  const nextActions = nextActionsFor(status, missingPrerequisites, workflowCompilation);

  return {
    status,
    intakeId: safeIdentifier(packet.intakeId),
    readinessLabel: narrativeDeckReady ? "Ready for story planning" : "Needs more information",
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
    workflowCompilation,
    nextActions,
    summaryLines: summaryLines(status, safeEvidenceRefCount, missingPrerequisites.length, workflowCompilation),
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
    missingPrerequisites: ["Safe Story Intake evidence"],
    limitations: ["Readiness is unknown until safe Story Intake evidence is available."],
    claims: [],
    workflowCompilation: null,
    nextActions: ["Submit safe Story Intake evidence"],
    summaryLines: [
      "No Story Intake has been submitted from this workbench yet.",
      "Ordo will not prepare the story plan until safe intake evidence exists.",
      "Nothing has been published, promoted to memory, written to graph truth, sent to providers, or run as a task.",
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

function workflowCompilationView(compilation: StoryWorkflowCompilationEvidence | null): StudioStoryWorkflowCompilationView | null {
  if (!compilation) return null;
  const status = compilation.status === "compiled" ? "compiled" : compilation.missingInputs.length > 0 ? "missing_input" : "blocked";
  const taskBindings = compilation.taskBindings.map((task) => ({
    key: safeIdentifier(task.key),
    method: safeIdentifier(task.method),
    outputArtifactKind: safeIdentifier(task.outputArtifactKind ?? "none"),
    visibility: safeIdentifier(task.visibility),
  }));
  const fanoutSummary = compilation.fanoutGroups.length
    ? compilation.fanoutGroups
        .map((fanout) => `${safeIdentifier(fanout.key)}: ${fanout.itemCount}/${fanout.maxItems}`)
        .join(", ")
    : "No fanout groups";
  const approvalGates = safeList(compilation.approvalGates.map((gate) => `${gate.action}${gate.required ? " required" : " optional"}`));
  const providerRequirements = safeList(
    compilation.providerRequirements.map((provider) => `${provider.capability} via ${provider.mode}, egress ${provider.egress}`),
  );

  return {
    status,
    templateLabel: `${safeIdentifier(compilation.templateId)} v${compilation.templateVersion}`,
    templateVersion: compilation.templateVersion,
    compilationRef: safeArtifactRef(compilation.compilationRef) ?? "workflow_compilation:withheld",
    safeEvidenceRefCount: safeEvidenceRefCountFor(compilation.evidenceRefs),
    missingInputs: safeList(compilation.missingInputs).map(humanizeIdentifier),
    limitations: safeList(compilation.limitations).map(humanizeIdentifier),
    nextActions: safeList(compilation.safeNextActions),
    variableCount: compilation.resolvedVariables.length,
    taskCount: taskBindings.length,
    fanoutSummary,
    approvalGates,
    providerRequirements,
    taskBindings,
  };
}

function nextActionsFor(
  status: StudioStoryIntakeStatus,
  missingPrerequisites: readonly string[],
  workflowCompilation: StudioStoryWorkflowCompilationView | null,
): string[] {
  if (workflowCompilation?.status === "compiled") {
    return [
      "Review the story production plan",
      "Open Story Preview to check the current state",
      "Keep provider work and publishing gated until a person approves",
    ];
  }
  if (workflowCompilation?.status === "missing_input") {
    return [...workflowCompilation.missingInputs.map((item) => `Resolve ${item.toLowerCase()}`), "Keep compilation blocked until inputs are safe"];
  }
  if (status === "ready") {
    return ["Prepare the story production plan", "Review the public-safe summary", "Prepare Story Pack production review"];
  }
  const missingActions = missingPrerequisites.map((item) => `Add ${item.toLowerCase()}`);
  return [...missingActions, "Keep readiness blocked until evidence is safe"];
}

function summaryLines(
  status: StudioStoryIntakeStatus,
  safeEvidenceRefCount: number,
  missingCount: number,
  workflowCompilation: StudioStoryWorkflowCompilationView | null,
): string[] {
  if (workflowCompilation?.status === "compiled") {
    return [
      "Story Intake has a saved production plan for Studio Preview.",
      `${workflowCompilation.taskCount} planned step(s) and ${workflowCompilation.safeEvidenceRefCount} safe evidence ref(s) are ready to review.`,
      "Nothing has been published, promoted to memory, written to graph truth, sent to providers, or run as a task.",
    ];
  }
  if (workflowCompilation?.status === "missing_input") {
    return [
      "Story Intake cannot prepare the production plan until missing information is resolved.",
      `${workflowCompilation.missingInputs.length} required item(s) still need attention.`,
      "Nothing has been published, promoted to memory, written to graph truth, sent to providers, or run as a task.",
    ];
  }
  if (status === "ready") {
    return [
      "Story Intake is ready for story planning.",
      `${safeEvidenceRefCount} safe evidence ref(s) support the public derivative.`,
      "Nothing has been published, promoted to memory, written to graph truth, sent to providers, or run as a task.",
    ];
  }
  return [
    "Story Intake needs more information before story planning can start.",
    `${missingCount} required item(s) still need attention.`,
    "Nothing has been published, promoted to memory, written to graph truth, sent to providers, or run as a task.",
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
  if (/^(artifact|business_fact|content_analytics|content_event|memory_candidate|workflow_compilation|job|event|surface|tracked_entry_point|cta|offer):[A-Za-z0-9_.:-]+$/.test(value)) {
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
