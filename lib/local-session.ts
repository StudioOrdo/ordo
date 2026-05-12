import { createHash, createHmac, timingSafeEqual } from "node:crypto";

export const LOCAL_SESSION_COOKIE_NAME = "ordo_local_session";
export const LOCAL_SESSION_MAX_AGE_SECONDS = 60 * 60 * 24 * 30;

const LOCAL_SESSION_SCHEMA_VERSION = "ordo.local-session.v1";
const SESSION_SIGNATURE_SECRET =
  process.env.ORDO_LOCAL_SESSION_SECRET?.trim() || "ordo-local-session-scaffold-v1";

export type LocalSessionMode = "login" | "register";

export interface LocalSessionInput {
  mode: LocalSessionMode;
  name?: unknown;
  email?: unknown;
  password?: unknown;
}

export interface LocalSessionReadModel {
  schemaVersion: typeof LOCAL_SESSION_SCHEMA_VERSION;
  sessionKind: "local_appliance_session";
  sessionId: string;
  actorId: string;
  role: "client";
  displayName: string;
  emailHash: string;
  issuedAt: string;
  expiresAt: string;
}

export interface LocalSessionError {
  code: "invalid_name" | "invalid_email" | "invalid_password";
  message: string;
}

export interface LocalSessionPersistence {
  source: "daemon" | "browser_cookie";
  degradedReason: string | null;
}

export type LocalSessionResult =
  | { ok: true; session: LocalSessionReadModel; cookieValue: string; persistence: LocalSessionPersistence }
  | { ok: false; error: LocalSessionError };

export function createLocalSession(input: LocalSessionInput, now = new Date()): LocalSessionResult {
  const normalizedEmail = normalizeEmail(input.email);
  if (!normalizedEmail) {
    return {
      ok: false,
      error: {
        code: "invalid_email",
        message: "Enter a valid email address.",
      },
    };
  }

  const password = normalizePassword(input.password);
  if (!password) {
    return {
      ok: false,
      error: {
        code: "invalid_password",
        message: "Enter a local session password with at least 8 characters.",
      },
    };
  }

  const displayName =
    input.mode === "register"
      ? normalizeDisplayName(input.name)
      : displayNameFromEmail(normalizedEmail);
  if (!displayName) {
    return {
      ok: false,
      error: {
        code: "invalid_name",
        message: "Enter a display name for this local appliance session.",
      },
    };
  }

  const emailHash = hashValue(normalizedEmail);
  const issuedAt = now.toISOString();
  const expiresAt = new Date(now.getTime() + LOCAL_SESSION_MAX_AGE_SECONDS * 1000).toISOString();
  const session: LocalSessionReadModel = {
    schemaVersion: LOCAL_SESSION_SCHEMA_VERSION,
    sessionKind: "local_appliance_session",
    sessionId: `local_session_${emailHash.slice(0, 32)}`,
    actorId: `actor_local_member_${emailHash.slice(0, 16)}`,
    role: "client",
    displayName,
    emailHash,
    issuedAt,
    expiresAt,
  };

  return {
    ok: true,
    session,
    cookieValue: serializeLocalSession(session),
    persistence: { source: "browser_cookie", degradedReason: null },
  };
}

export function createCookieForDaemonSession(
  session: LocalSessionReadModel,
  degradedReason: string | null = null,
): { cookieValue: string; persistence: LocalSessionPersistence } {
  return {
    cookieValue: serializeLocalSession(session),
    persistence: { source: "daemon", degradedReason },
  };
}

export function parseLocalSessionCookie(cookieValue: string | undefined, now = new Date()): LocalSessionReadModel | null {
  if (!cookieValue) {
    return null;
  }

  const parts = cookieValue.split(".");
  if (parts.length !== 2) {
    return null;
  }

  const [payload, signature] = parts;
  if (!verifySignature(payload, signature)) {
    return null;
  }

  try {
    const parsed = JSON.parse(Buffer.from(payload, "base64url").toString("utf8")) as Partial<LocalSessionReadModel>;
    if (!isLocalSessionReadModel(parsed)) {
      return null;
    }
    if (Date.parse(parsed.expiresAt) <= now.getTime()) {
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

function serializeLocalSession(session: LocalSessionReadModel): string {
  const payload = Buffer.from(JSON.stringify(session), "utf8").toString("base64url");
  return `${payload}.${signPayload(payload)}`;
}

function normalizeEmail(value: unknown): string | null {
  if (typeof value !== "string") {
    return null;
  }
  const email = value.trim().toLowerCase();
  if (email.length < 3 || email.length > 254 || /\s/.test(email)) {
    return null;
  }
  if (!/^[^@]+@[^@]+\.[^@]+$/.test(email)) {
    return null;
  }
  return email;
}

function normalizePassword(value: unknown): string | null {
  if (typeof value !== "string") {
    return null;
  }
  const password = value.trim();
  if (password.length < 8 || password.length > 128) {
    return null;
  }
  return password;
}

function normalizeDisplayName(value: unknown): string | null {
  if (typeof value !== "string") {
    return null;
  }
  const name = value.replace(/[\u0000-\u001f\u007f]/g, " ").replace(/\s+/g, " ").trim();
  if (name.length < 1 || name.length > 80) {
    return null;
  }
  return name;
}

function displayNameFromEmail(email: string): string {
  const localPart = email.split("@")[0]?.replace(/[._-]+/g, " ").replace(/\s+/g, " ").trim();
  return localPart ? localPart.slice(0, 80) : "Local member";
}

function hashValue(value: string): string {
  return createHash("sha256").update(value).digest("hex");
}

function signPayload(payload: string): string {
  return createHmac("sha256", SESSION_SIGNATURE_SECRET).update(payload).digest("base64url");
}

function verifySignature(payload: string, signature: string): boolean {
  const expected = signPayload(payload);
  const expectedBuffer = Buffer.from(expected);
  const actualBuffer = Buffer.from(signature);
  if (expectedBuffer.length !== actualBuffer.length) {
    return false;
  }
  return timingSafeEqual(expectedBuffer, actualBuffer);
}

function isLocalSessionReadModel(value: Partial<LocalSessionReadModel>): value is LocalSessionReadModel {
  return (
    value.schemaVersion === LOCAL_SESSION_SCHEMA_VERSION &&
    value.sessionKind === "local_appliance_session" &&
    typeof value.sessionId === "string" &&
    value.sessionId.startsWith("local_session_") &&
    typeof value.actorId === "string" &&
    value.actorId.startsWith("actor_local_member_") &&
    value.role === "client" &&
    typeof value.displayName === "string" &&
    value.displayName.length > 0 &&
    typeof value.emailHash === "string" &&
    /^[a-f0-9]{64}$/.test(value.emailHash) &&
    typeof value.issuedAt === "string" &&
    Number.isFinite(Date.parse(value.issuedAt)) &&
    typeof value.expiresAt === "string" &&
    Number.isFinite(Date.parse(value.expiresAt))
  );
}