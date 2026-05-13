import { NextResponse } from "next/server";

import { daemonUrl } from "@/lib/daemon-client";

export const runtime = "nodejs";

interface VisitorSessionRequest {
  entryPointSlug?: unknown;
  sessionId?: unknown;
  userAgent?: unknown;
  attribution?: unknown;
}

export async function POST(request: Request) {
  const body = await readJson(request);
  const entryPointSlug = typeof body.entryPointSlug === "string" ? body.entryPointSlug.trim() : "";

  if (!entryPointSlug) {
    return NextResponse.json({ error: "Entry point slug is required." }, { status: 400 });
  }

  try {
    const daemonResponse = await fetch(`${daemonUrl()}/public/visitor-sessions`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        entryPointSlug,
        sessionId: typeof body.sessionId === "string" && body.sessionId.trim() ? body.sessionId.trim() : undefined,
        userAgent: typeof body.userAgent === "string" && body.userAgent.trim() ? body.userAgent : request.headers.get("user-agent"),
        attribution: isRecord(body.attribution) ? body.attribution : {},
      }),
      cache: "no-store",
    });
    const payload = await daemonResponse.json().catch(() => ({}));
    return NextResponse.json(payload, { status: daemonResponse.status });
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Visitor session route unavailable." },
      { status: 502 },
    );
  }
}

async function readJson(request: Request): Promise<VisitorSessionRequest> {
  try {
    const value = await request.json();
    return isRecord(value) ? value : {};
  } catch {
    return {};
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}
