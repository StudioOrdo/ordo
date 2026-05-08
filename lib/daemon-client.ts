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

interface BackupRestoreResponse {
  jobs: BackupRestoreJobSummary[];
}

export interface BackupRestoreSnapshot {
  daemonUrl: string;
  createdAt: string;
  jobs: BackupRestoreJobSummary[];
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
const DAEMON_REQUEST_TIMEOUT_MS = 2_000;

export function daemonUrl(): string {
  return process.env.ORDO_DAEMON_URL?.trim() || DEFAULT_DAEMON_URL;
}

export function daemonWebSocketUrl(): string {
  return process.env.NEXT_PUBLIC_ORDO_DAEMON_WS_URL?.trim() || DEFAULT_DAEMON_WS_URL;
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