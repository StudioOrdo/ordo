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

export interface SystemSnapshot {
  daemonUrl: string;
  websocketUrl: string;
  createdAt: string;
  health: DaemonHealthReport | null;
  readiness: DaemonReadinessReport | null;
  degradedReason: string | null;
}

const DEFAULT_DAEMON_URL = "http://127.0.0.1:17760";
const DEFAULT_DAEMON_WS_URL = "ws://127.0.0.1:17760/ws";

function daemonUrl(): string {
  return process.env.ORDO_DAEMON_URL?.trim() || DEFAULT_DAEMON_URL;
}

export function daemonWebSocketUrl(): string {
  return process.env.NEXT_PUBLIC_ORDO_DAEMON_WS_URL?.trim() || DEFAULT_DAEMON_WS_URL;
}

async function fetchJson<T>(baseUrl: string, path: string): Promise<T> {
  const response = await fetch(`${baseUrl}${path}`, {
    cache: "no-store",
  });

  if (!response.ok) {
    throw new Error(`${path} responded with ${response.status}`);
  }

  return response.json() as Promise<T>;
}

export async function getSystemSnapshot(): Promise<SystemSnapshot> {
  const baseUrl = daemonUrl();
  const createdAt = new Date().toISOString();

  try {
    const [health, readiness] = await Promise.all([
      fetchJson<DaemonHealthReport>(baseUrl, "/health"),
      fetchJson<DaemonReadinessReport>(baseUrl, "/ready"),
    ]);

    return {
      daemonUrl: baseUrl,
      websocketUrl: daemonWebSocketUrl(),
      createdAt,
      health,
      readiness,
      degradedReason: null,
    };
  } catch (error) {
    return {
      daemonUrl: baseUrl,
      websocketUrl: daemonWebSocketUrl(),
      createdAt,
      health: null,
      readiness: null,
      degradedReason: error instanceof Error ? error.message : "Daemon is unavailable.",
    };
  }
}