import { NextResponse } from "next/server";

import { postDaemonJson } from "@/lib/daemon-client";
import {
  LOCAL_SESSION_COOKIE_NAME,
  LOCAL_SESSION_MAX_AGE_SECONDS,
  type LocalSessionReadModel,
  createCookieForDaemonSession,
  createLocalSession,
} from "@/lib/local-session";

export const runtime = "nodejs";

export async function POST(request: Request) {
  const body = await readJsonBody(request);
  const localResult = createLocalSession({
    mode: "register",
    name: body.name,
    email: body.email,
    password: body.password,
  });

  if (!localResult.ok) {
    return NextResponse.json({ error: localResult.error.message }, { status: 400 });
  }

  const persisted = await persistWithDaemon("/local-sessions/register", body);
  const session = persisted?.session ?? localResult.session;
  const cookie = persisted
    ? createCookieForDaemonSession(persisted.session)
    : localResult.persistence.source === "browser_cookie"
      ? {
          cookieValue: localResult.cookieValue,
          persistence: {
            source: "browser_cookie" as const,
            degradedReason: "Daemon local session route unavailable; using browser-local scaffold.",
          },
        }
      : localResult;

  const response = NextResponse.json({
    session,
    persistence: cookie.persistence,
    redirectTo: "/my/chat?role=client",
  });
  response.cookies.set(LOCAL_SESSION_COOKIE_NAME, cookie.cookieValue, {
    httpOnly: true,
    maxAge: LOCAL_SESSION_MAX_AGE_SECONDS,
    path: "/",
    sameSite: "lax",
  });
  return response;
}

async function readJsonBody(request: Request): Promise<Record<string, unknown>> {
  try {
    const body = await request.json();
    return body && typeof body === "object" ? (body as Record<string, unknown>) : {};
  } catch {
    return {};
  }
}

async function persistWithDaemon(path: string, body: Record<string, unknown>): Promise<{ session: LocalSessionReadModel } | null> {
  try {
    return await postDaemonJson<{ session: LocalSessionReadModel }>(path, body);
  } catch {
    return null;
  }
}