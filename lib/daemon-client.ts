import {
  buildStudioWorkSnapshot,
  type StudioSurfaceRoom,
  type StudioSurfaceWorkItem,
  type StudioWorkSnapshotView,
  type StudioWorkViewer,
} from "@/lib/studio-work";
import type { GrowthPilotReportResponse } from "@/lib/growth-pilot-report";
import type {
  StoryPublishLearningBrief,
  StudioProductionReviewPacket,
  StudioPublicationsSnapshot,
} from "@/lib/studio-publications";
import type {
  StoryFounderIntakePacket,
  StudioStoryIntakeRequest,
  StudioStoryIntakeSnapshot,
} from "@/lib/studio-story-intake";

export type {
  GrowthPilotEvidenceRef,
  GrowthPilotReportItem,
  GrowthPilotReportLimitation,
  GrowthPilotReportMetric,
  GrowthPilotReportResponse,
  GrowthPilotReportSection,
  GrowthReportSourceStatus,
} from "@/lib/growth-pilot-report";

export type {
  StudioDeferredAction,
  StudioRoomSummary,
  StudioSurfaceRoom,
  StudioSurfaceWorkItem,
  StudioWorkItemView,
  StudioWorkSnapshotView,
  StudioWorkViewer,
} from "@/lib/studio-work";

export interface DaemonCheck {
  name: string;
  status: string;
  detail: string;
}

export interface DaemonHealthReport {
  schemaVersion: string;
  service: string;
  status: string;
  mode?: string;
  checks: DaemonCheck[];
}

export interface DaemonReadinessReport {
  schemaVersion: string;
  service: string;
  status: string;
  checks: DaemonCheck[];
}

export interface BriefEvidence {
  label: string;
  value: string;
  source: string;
}

export interface BriefProcessProvenance {
  jobId: string;
  templateId: string;
  templateVersion: number;
  origin: string;
  status: string;
}

export interface SystemBriefArtifact {
  id: string;
  sectionKey: string;
  jobId: string | null;
  process: BriefProcessProvenance | null;
  version: number;
  title: string;
  summary: string[];
  bodyMarkdown: string;
  evidence: BriefEvidence[];
  limitations: string[];
  visibility: string;
  createdAt: string;
  validUntil: string | null;
}

export interface BackupRestoreTaskSummary {
  key: string;
  label: string;
  status: string;
  required: boolean;
  startedAt: string | null;
  completedAt: string | null;
  errorMessage: string | null;
}

export interface BackupRestoreArtifactSummary {
  id: string;
  artifactKind: string;
  uri: string;
  label: string;
  metadata: Record<string, unknown>;
  createdAt: string;
}

export interface BackupRestoreJobSummary {
  id: string;
  operation: string;
  kind: string;
  status: string;
  progress: {
    totalRequiredTasks: number;
    completedRequiredTasks: number;
    percent: number;
  };
  currentTaskKey: string | null;
  elapsedSeconds: number | null;
  startedAt: string | null;
  completedAt: string | null;
  createdAt: string;
  updatedAt: string;
  failureMessage: string | null;
  artifact: BackupRestoreArtifactSummary | null;
  tasks: BackupRestoreTaskSummary[];
}

export interface HostedTrialCapacityPolicy {
  id: string;
  offerId: string;
  offerSlug: string;
  status: string;
  activeSlotLimit: number;
  activeSlotCount: number;
  waitlistCount: number;
  trialDays: number;
  backupBeforeWipeRequired: boolean;
  resetGraceDays: number;
  metadata: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
}

export interface HostedTrialSlot {
  id: string;
  policyId: string;
  trialId: string;
  acceptanceId: string;
  offerId: string;
  offerSlug: string;
  subjectKind: string;
  subjectId: string;
  status: string;
  allocatedAt: string;
  expiresAt: string;
  releasedAt: string | null;
  releaseReason: string | null;
  backupRequired: boolean;
  backupStatus: string;
  backupEvidenceRefs: string[];
  resetEligibleAt: string | null;
  resetState: string;
  resetGuard: Record<string, unknown>;
  ownerOverride: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
}

export interface HostedTrialWaitlistEntry {
  id: string;
  policyId: string;
  acceptanceId: string;
  offerId: string;
  offerSlug: string;
  visitorSessionId: string | null;
  subjectKind: string;
  subjectId: string;
  status: string;
  position: number;
  reason: string;
  receipt: Record<string, unknown>;
  evidenceRefs: string[];
  createdAt: string;
  updatedAt: string;
}

export interface OfferView {
  id: string;
  slug: string;
  title: string;
  summary: string;
  status: string;
  visibility: string;
  publicationState: string;
  trialDays: number;
  sourceKind: string;
  sourceRef: string | null;
  terms: Record<string, unknown>;
  metadata: Record<string, unknown>;
  createdByActorId: string | null;
  createdAt: string;
  updatedAt: string;
  publishedAt: string | null;
  archivedAt: string | null;
}

export interface PublicOfferView {
  id: string;
  slug: string;
  title: string;
  summary: string;
  trialDays: number;
  sourceKind: string;
  sourceRef: string | null;
  terms: Record<string, unknown>;
}

export interface OfferBuilderReference {
  key: string;
  label: string;
  status: string;
  detail: string;
  evidenceRefs: string[];
  blockedBy: string | null;
}

export interface OfferBuilderValidation {
  publishable: boolean;
  state: string;
  blockers: string[];
  warnings: string[];
  supportedReferences: OfferBuilderReference[];
  deferredReferences: OfferBuilderReference[];
  evidenceRefs: string[];
}

export interface OfferBuilderOffer {
  offer: OfferView;
  publicPreview: PublicOfferView | null;
  validation: OfferBuilderValidation;
}

export interface RealtimeEventSummary {
  cursor: number;
  schemaVersion: string;
  family: string;
  eventType: string;
  jobId: string | null;
  taskKey: string | null;
  sequence: number | null;
  payload: Record<string, unknown>;
  occurredAt: string;
}

export interface SchedulerOperationsRun {
  id: string;
  jobId: string | null;
  dueAt: string;
  claimedAt: string | null;
  completedAt: string | null;
  status: string;
  hasError: boolean;
}

export interface SchedulerOperationsSchedule {
  id: string;
  name: string;
  templateId: string;
  templateVersion: number;
  scheduleKind: string;
  enabled: boolean;
  timezone: string;
  cronExpression: string | null;
  intervalSeconds: number | null;
  runAt: string | null;
  lastDueAt: string | null;
  nextDueAt: string;
  lastRun: SchedulerOperationsRun | null;
  limitations: string[];
}

export interface SchedulerOperationsResponse {
  generatedAt: string;
  schedules: SchedulerOperationsSchedule[];
}

export interface DiagnosticLogEntry {
  id: string;
  timestamp: string;
  level: string;
  source: string;
  message: string;
  requestId: string | null;
  jobId: string | null;
  taskKey: string | null;
  capabilityId: string | null;
  eventType: string | null;
  errorCode: string | null;
  durationMs: number | null;
  payload: Record<string, unknown>;
}

export interface DiagnosticLogsResponse {
  logs: DiagnosticLogEntry[];
}

export interface DiagnosticLogsSnapshot {
  daemonUrl: string;
  createdAt: string;
  logs: DiagnosticLogEntry[];
  degradedReason: string | null;
}

export type IssueSeverity = "low" | "medium" | "high" | "blocker";
export type IssueReportStatus = "draft" | "ready_for_review" | "exported" | "submitted" | "dismissed";

export interface EvidenceEnvelope {
  source: string;
  collectedAt: string;
  status: string;
  summary: string;
  payload: unknown;
  redactions: string[];
  limits: unknown;
  errors: string[];
}

export interface IssueReportArtifact {
  id: string;
  jobId: string | null;
  status: IssueReportStatus;
  severity: IssueSeverity;
  title: string;
  summary: string;
  description: string;
  sourceRoute: string | null;
  markdownBody: string;
  diagnostics: unknown;
  evidence: EvidenceEnvelope[];
  redactions: string[];
  createdAt: string;
  updatedAt: string;
  exportedAt: string | null;
  submittedAt: string | null;
  externalUrl: string | null;
}

export interface IssueReportSummary {
  id: string;
  jobId: string | null;
  status: IssueReportStatus;
  severity: IssueSeverity;
  title: string;
  summary: string;
  sourceRoute: string | null;
  createdAt: string;
  updatedAt: string;
  exportedAt: string | null;
  submittedAt: string | null;
  externalUrl: string | null;
}

export interface IssueReportsResponse {
  reports: IssueReportSummary[];
}

interface IssueReportDetailResponse {
  report: IssueReportArtifact;
}

export interface IssueReportsSnapshot {
  daemonUrl: string;
  createdAt: string;
  reports: IssueReportSummary[];
  latestReport: IssueReportArtifact | null;
  degradedReason: string | null;
}

export interface GrowthPilotReportSnapshot {
  daemonUrl: string;
  createdAt: string;
  report: GrowthPilotReportResponse | null;
  generatedAt: string | null;
  degradedReason: string | null;
}

export type { StoryPublishLearningBrief, StudioProductionReviewPacket, StudioPublicationsSnapshot };

export interface ProviderReadinessSummary {
  configuredProviderMode: string;
  requestedProviderId: string | null;
  defaultProviderId: string | null;
  liveModeRequested: boolean;
  liveInvocationEnabled: boolean;
  liveInvocationGuard: string;
  credentialsPresent: boolean;
  credentialSource: string;
  missingCredentialProviderIds: string[];
  openai: OpenAiProviderReadiness;
}

export interface OpenAiProviderReadiness {
  providerId: string;
  decision: string;
  modelId: string | null;
  modelSource: string;
  baseUrl: string;
  baseUrlSource: string;
  timeoutMs: number | null;
  timeoutGuard: string;
  budgetMicros: number | null;
  budgetGuard: string;
  maxCases: number | null;
  apiKeyConfigured: boolean;
  apiKeySource: string;
  liveEvalGuard: string;
  networkGuard: string;
  liveInvocationGuard: string;
  readyForGuardedSmoke: boolean;
  reasons: string[];
}

export interface RedactedSecretField {
  configured: boolean;
  source: string;
  locked: boolean;
  redacted: string | null;
}

export interface ProviderConfigView {
  providerId: string;
  providerName: string;
  enabled: boolean;
  defaultProvider: boolean;
  model: string | null;
  availableModels: ProviderModelOption[];
  baseUrl: string | null;
  nonSecretConfig: Record<string, unknown>;
  apiKey: RedactedSecretField;
  createdAt: string;
  updatedAt: string;
}

export interface ProviderModelOption {
  id: string;
  label: string;
  default: boolean;
}

interface ProviderListResponse {
  readiness: ProviderReadinessSummary;
  providers: ProviderConfigView[];
}

export interface ProviderSnapshot {
  daemonUrl: string;
  createdAt: string;
  readiness: ProviderReadinessSummary | null;
  providers: ProviderConfigView[];
  degradedReason: string | null;
}

interface BackupRestoreResponse {
  jobs: BackupRestoreJobSummary[];
}

interface HostedTrialCapacityResponse {
  policies: HostedTrialCapacityPolicy[];
  slots: HostedTrialSlot[];
  waitlist: HostedTrialWaitlistEntry[];
}

interface OfferBuilderResponse {
  offers: OfferBuilderOffer[];
  generatedAt: string;
}

interface SurfaceWorkItemsResponse {
  items: StudioSurfaceWorkItem[];
}

interface EventReplayResponse {
  events: RealtimeEventSummary[];
  nextCursor: number | null;
}

export interface BackupRestoreSnapshot {
  daemonUrl: string;
  createdAt: string;
  jobs: BackupRestoreJobSummary[];
  degradedReason: string | null;
}

export interface HostedTrialOperationsSnapshot {
  daemonUrl: string;
  createdAt: string;
  policies: HostedTrialCapacityPolicy[];
  slots: HostedTrialSlot[];
  waitlist: HostedTrialWaitlistEntry[];
  backupJobs: BackupRestoreJobSummary[];
  degradedReason: string | null;
}

export interface OfferBuilderSnapshot {
  daemonUrl: string;
  createdAt: string;
  generatedAt: string | null;
  offers: OfferBuilderOffer[];
  degradedReason: string | null;
}

export interface StudioWorkSnapshot extends StudioWorkSnapshotView {
  daemonUrl: string;
  createdAt: string;
  viewer: StudioWorkViewer;
  roomKind: StudioSurfaceRoom | null;
  degradedReason: string | null;
}

export interface ArtifactPatchPreview {
  changed: boolean;
  addedLines: number;
  removedLines: number;
  hunks: number;
}

export interface StudioArtifactPatchProposal {
  id: string;
  sourceArtifactId: string;
  sourceArtifactKind: string;
  sourceArtifactTitle: string;
  sourceArtifactStatus: string;
  sourceArtifactVisibility: string;
  sourceVersionId: string;
  baseHash: string;
  proposedHash: string;
  preview: ArtifactPatchPreview;
  boundedPatchPreview: string;
  previewTruncated: boolean;
  evidenceRefs: string[];
  provenance: Record<string, unknown>;
  reviewState: string;
  acceptedVersionId: string | null;
  proposedByActorId: string;
  appliedByActorId: string | null;
  createdAt: string;
  updatedAt: string;
  appliedAt: string | null;
}

interface StudioArtifactPatchListResponse {
  proposals: StudioArtifactPatchProposal[];
}

export interface StudioArtifactPatchSnapshot {
  daemonUrl: string;
  createdAt: string;
  proposals: StudioArtifactPatchProposal[];
  degradedReason: string | null;
}

export interface EventReplaySnapshot {
  daemonUrl: string;
  createdAt: string;
  events: RealtimeEventSummary[];
  nextCursor: number | null;
  degradedReason: string | null;
}

export interface SchedulerOperationsSnapshot {
  daemonUrl: string;
  createdAt: string;
  generatedAt: string | null;
  schedules: SchedulerOperationsSchedule[];
  degradedReason: string | null;
}

interface LatestSystemBriefResponse {
  brief: SystemBriefArtifact | null;
}

export interface SystemSnapshot {
  daemonUrl: string;
  websocketUrl: string;
  createdAt: string;
  health: DaemonHealthReport | null;
  readiness: DaemonReadinessReport | null;
  brief: SystemBriefArtifact | null;
  briefError: string | null;
  degradedReason: string | null;
}

const DEFAULT_DAEMON_URL = "http://127.0.0.1:17760";
const DEFAULT_DAEMON_WS_URL = "ws://127.0.0.1:17760/ws";
const DEFAULT_DAEMON_CHAT_WS_URL = "ws://127.0.0.1:17760/chat/ws";
const DAEMON_REQUEST_TIMEOUT_MS = 2_000;

export function daemonUrl(): string {
  return process.env.ORDO_DAEMON_URL?.trim() || DEFAULT_DAEMON_URL;
}

export function daemonWebSocketUrl(): string {
  return process.env.NEXT_PUBLIC_ORDO_DAEMON_WS_URL?.trim() || DEFAULT_DAEMON_WS_URL;
}

export function daemonChatWebSocketUrl(): string {
  const configuredChatUrl = process.env.NEXT_PUBLIC_ORDO_DAEMON_CHAT_WS_URL?.trim();
  if (configuredChatUrl) {
    return configuredChatUrl;
  }
  const websocketUrl = daemonWebSocketUrl();
  return websocketUrl.endsWith("/ws") ? websocketUrl.replace(/\/ws$/, "/chat/ws") : DEFAULT_DAEMON_CHAT_WS_URL;
}

async function fetchJson<T>(baseUrl: string, path: string): Promise<T> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), DAEMON_REQUEST_TIMEOUT_MS);

  try {
    const response = await fetch(`${baseUrl}${path}`, {
      cache: "no-store",
      signal: controller.signal,
    });

    if (!response.ok) {
      throw new Error(`${path} responded with ${response.status}`);
    }

    return response.json() as Promise<T>;
  } finally {
    clearTimeout(timeout);
  }
}

async function sendDaemonJson<T>(method: "POST" | "PUT", path: string, body?: unknown): Promise<T> {
  const baseUrl = daemonUrl();
  const response = await fetch(`${baseUrl}${path}`, {
    method,
    headers: body ? { "content-type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
    cache: "no-store",
  });

  if (!response.ok) {
    throw new Error(`${path} responded with ${response.status}`);
  }

  return response.json() as Promise<T>;
}

export async function postDaemonJson<T>(path: string, body?: unknown): Promise<T> {
  return sendDaemonJson("POST", path, body);
}

export async function putDaemonJson<T>(path: string, body?: unknown): Promise<T> {
  return sendDaemonJson("PUT", path, body);
}

async function readEndpoint<T>(baseUrl: string, path: string): Promise<{ data: T | null; error: string | null }> {
  try {
    return { data: await fetchJson<T>(baseUrl, path), error: null };
  } catch (error) {
    return {
      data: null,
      error: error instanceof Error ? `${path}: ${error.message}` : `${path}: unavailable`,
    };
  }
}

async function postEndpoint<T>(
  baseUrl: string,
  path: string,
  body: unknown,
): Promise<{ data: T | null; error: string | null }> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), DAEMON_REQUEST_TIMEOUT_MS);

  try {
    const response = await fetch(`${baseUrl}${path}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
      cache: "no-store",
      signal: controller.signal,
    });

    if (!response.ok) {
      throw new Error(`${path} responded with ${response.status}`);
    }

    return { data: (await response.json()) as T, error: null };
  } catch (error) {
    return {
      data: null,
      error: error instanceof Error ? `${path}: ${error.message}` : `${path}: unavailable`,
    };
  } finally {
    clearTimeout(timeout);
  }
}

export async function getBackupRestoreSnapshot(): Promise<BackupRestoreSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();
  const backupResult = await readEndpoint<BackupRestoreResponse>(baseUrl, "/backups");

  return {
    daemonUrl: baseUrl,
    createdAt,
    jobs: backupResult.data?.jobs ?? [],
    degradedReason: backupResult.error,
  };
}

export async function getHostedTrialOperationsSnapshot(): Promise<HostedTrialOperationsSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();
  const [capacityResult, backupResult] = await Promise.all([
    readEndpoint<HostedTrialCapacityResponse>(baseUrl, "/hosted-trials/capacity"),
    readEndpoint<BackupRestoreResponse>(baseUrl, "/backups"),
  ]);
  const degradedReasons = [capacityResult.error, backupResult.error].filter(Boolean);

  return {
    daemonUrl: baseUrl,
    createdAt,
    policies: capacityResult.data?.policies ?? [],
    slots: capacityResult.data?.slots ?? [],
    waitlist: capacityResult.data?.waitlist ?? [],
    backupJobs: backupResult.data?.jobs ?? [],
    degradedReason: degradedReasons.length > 0 ? degradedReasons.join(" ") : null,
  };
}

export async function getOfferBuilderSnapshot(): Promise<OfferBuilderSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();
  const builderResult = await readEndpoint<OfferBuilderResponse>(baseUrl, "/offer-builder");

  return {
    daemonUrl: baseUrl,
    createdAt,
    generatedAt: builderResult.data?.generatedAt ?? null,
    offers: builderResult.data?.offers ?? [],
    degradedReason: builderResult.error,
  };
}

export async function getStudioWorkSnapshot(viewer: StudioWorkViewer, roomKind?: StudioSurfaceRoom): Promise<StudioWorkSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();
  const params = new URLSearchParams({
    viewer,
    surfaceKind: "studio",
  });
  if (roomKind) {
    params.set("roomKind", roomKind);
  }
  params.set("limit", "100");
  const workResult = await readEndpoint<SurfaceWorkItemsResponse>(baseUrl, `/surface/work-items?${params.toString()}`);
  const workSnapshot = buildStudioWorkSnapshot(workResult.data?.items ?? []);

  return {
    ...workSnapshot,
    daemonUrl: baseUrl,
    createdAt,
    viewer,
    roomKind: roomKind ?? null,
    degradedReason: workResult.error,
  };
}

export async function getStudioArtifactPatchSnapshot(): Promise<StudioArtifactPatchSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();
  const patchResult = await readEndpoint<StudioArtifactPatchListResponse>(
    baseUrl,
    "/studio/artifact-patches?reviewState=proposed&limit=50",
  );

  return {
    daemonUrl: baseUrl,
    createdAt,
    proposals: patchResult.data?.proposals ?? [],
    degradedReason: patchResult.error,
  };
}

export async function getStudioPublicationsSnapshot(
  viewer: StudioWorkViewer,
  options: { deckId?: string; artifactIds?: string[] } = {},
): Promise<StudioPublicationsSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();
  const deckId = options.deckId?.trim() || "homepage.story.v1";
  const artifactIds = [
    ...new Set((options.artifactIds ?? []).map((id) => id.trim()).filter(Boolean)),
  ].sort();
  const params = new URLSearchParams({
    audience: viewer,
    deckId,
  });
  if (artifactIds.length > 0) {
    params.set("artifactIds", artifactIds.join(","));
  }

  const [reviewResult, learningResult] = await Promise.all([
    readEndpoint<StudioProductionReviewPacket>(
      baseUrl,
      `/studio/story-production-review?${params.toString()}`,
    ),
    readEndpoint<StoryPublishLearningBrief>(
      baseUrl,
      `/studio/story-publish-learning?${params.toString()}`,
    ),
  ]);
  const degradedReasons = [reviewResult.error, learningResult.error].filter(Boolean);

  return {
    daemonUrl: baseUrl,
    createdAt,
    deckId,
    artifactIds,
    viewer,
    review: reviewResult.data,
    learning: learningResult.data,
    degradedReason: degradedReasons.length > 0 ? degradedReasons.join(" ") : null,
  };
}

export async function getStudioStoryIntakeSnapshot(
  viewer: StudioWorkViewer,
  request: StudioStoryIntakeRequest | null,
): Promise<StudioStoryIntakeSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();
  if (!request) {
    return {
      daemonUrl: baseUrl,
      createdAt,
      viewer,
      request: null,
      packet: null,
      degradedReason: null,
      emptyReason: "No Story founder intake has been submitted from this workbench yet.",
    };
  }

  const result = await postEndpoint<StoryFounderIntakePacket>(baseUrl, "/studio/story-founder-intake", {
    intakeId: request.intakeId,
    founderStory: request.founderStory,
    businessStance: request.businessStance,
    audience: request.audience,
    evidenceRefs: request.evidenceRefs,
    source: "studio_story_intake_workbench",
  });

  return {
    daemonUrl: baseUrl,
    createdAt,
    viewer,
    request,
    packet: result.data,
    degradedReason: result.error,
    emptyReason: null,
  };
}

export async function getEventReplaySnapshot(after?: number): Promise<EventReplaySnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();
  const query = after && after > 0 ? `?after=${after}&limit=100` : "?limit=100";
  const eventResult = await readEndpoint<EventReplayResponse>(baseUrl, `/events${query}`);

  return {
    daemonUrl: baseUrl,
    createdAt,
    events: eventResult.data?.events ?? [],
    nextCursor: eventResult.data?.nextCursor ?? null,
    degradedReason: eventResult.error,
  };
}

export async function getSchedulerOperationsSnapshot(): Promise<SchedulerOperationsSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();
  const schedulesResult = await readEndpoint<SchedulerOperationsResponse>(baseUrl, "/schedules");

  return {
    daemonUrl: baseUrl,
    createdAt,
    generatedAt: schedulesResult.data?.generatedAt ?? null,
    schedules: schedulesResult.data?.schedules ?? [],
    degradedReason: schedulesResult.error,
  };
}

export async function getDiagnosticLogsSnapshot(): Promise<DiagnosticLogsSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();
  const logResult = await readEndpoint<DiagnosticLogsResponse>(baseUrl, "/logs?limit=100");

  return {
    daemonUrl: baseUrl,
    createdAt,
    logs: logResult.data?.logs ?? [],
    degradedReason: logResult.error,
  };
}

export async function getIssueReportsSnapshot(): Promise<IssueReportsSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();
  const reportResult = await readEndpoint<IssueReportsResponse>(baseUrl, "/reports/issues");
  const reports = reportResult.data?.reports ?? [];
  const latestReportId = reports[0]?.id ?? null;
  const latestReportResult = latestReportId
    ? await readEndpoint<IssueReportDetailResponse>(baseUrl, `/reports/issues/${latestReportId}`)
    : { data: null, error: null };
  const degradedReasons = [reportResult.error, latestReportResult.error].filter(Boolean);

  return {
    daemonUrl: baseUrl,
    createdAt,
    reports,
    latestReport: latestReportResult.data?.report ?? null,
    degradedReason: degradedReasons.length > 0 ? degradedReasons.join(" ") : null,
  };
}

export async function getGrowthPilotReportSnapshot(): Promise<GrowthPilotReportSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();
  const reportResult = await readEndpoint<GrowthPilotReportResponse>(baseUrl, "/growth/pilot-report");

  return {
    daemonUrl: baseUrl,
    createdAt,
    report: reportResult.data,
    generatedAt: reportResult.data?.generatedAt ?? null,
    degradedReason: reportResult.error,
  };
}

export async function getProviderSnapshot(): Promise<ProviderSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();
  const providerResult = await readEndpoint<ProviderListResponse>(baseUrl, "/providers");

  return {
    daemonUrl: baseUrl,
    createdAt,
    readiness: providerResult.data?.readiness ?? null,
    providers: providerResult.data?.providers ?? [],
    degradedReason: providerResult.error,
  };
}

export async function getSystemSnapshot(): Promise<SystemSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();

  const [healthResult, readinessResult, briefResult] = await Promise.all([
    readEndpoint<DaemonHealthReport>(baseUrl, "/health"),
    readEndpoint<DaemonReadinessReport>(baseUrl, "/ready"),
    readEndpoint<LatestSystemBriefResponse>(baseUrl, "/briefs/system/latest"),
  ]);

  const degradedReasons = [healthResult.error, readinessResult.error].filter(Boolean);

  return {
    daemonUrl: baseUrl,
    websocketUrl: daemonWebSocketUrl(),
    createdAt,
    health: healthResult.data,
    readiness: readinessResult.data,
    brief: briefResult.data?.brief ?? null,
    briefError: briefResult.error,
    degradedReason: degradedReasons.length > 0 ? degradedReasons.join(" ") : null,
  };
}
