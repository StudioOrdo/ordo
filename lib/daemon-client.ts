import {
  buildStudioWorkSnapshot,
  type StudioSurfaceRoom,
  type StudioSurfaceWorkItem,
  type StudioWorkSnapshotView,
  type StudioWorkViewer,
} from "@/lib/studio-work";
import type { GrowthPilotReportResponse } from "@/lib/growth-pilot-report";

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

export interface EventReplaySnapshot {
  daemonUrl: string;
  createdAt: string;
  events: RealtimeEventSummary[];
  nextCursor: number | null;
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

export async function postDaemonJson<T>(path: string, body?: unknown): Promise<T> {
  const baseUrl = daemonUrl();
  const response = await fetch(`${baseUrl}${path}`, {
    method: "POST",
    headers: body ? { "content-type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
    cache: "no-store",
  });

  if (!response.ok) {
    throw new Error(`${path} responded with ${response.status}`);
  }

  return response.json() as Promise<T>;
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
