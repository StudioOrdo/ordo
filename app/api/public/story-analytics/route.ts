import { NextResponse } from "next/server";

import { daemonUrl } from "@/lib/daemon-client";

export const runtime = "nodejs";

interface PublicStoryAnalyticsRequest {
  eventKind?: unknown;
  deckId?: unknown;
  deckVersion?: unknown;
  sectionId?: unknown;
  ctaId?: unknown;
  entryPointSlug?: unknown;
  visitorSessionId?: unknown;
  idempotencyKey?: unknown;
  occurredAt?: unknown;
}

export async function POST(request: Request) {
  const body = await readJson(request);
  const payload = {
    eventKind: stringValue(body.eventKind),
    deckId: stringValue(body.deckId),
    deckVersion: numberValue(body.deckVersion),
    sectionId: optionalStringValue(body.sectionId),
    ctaId: optionalStringValue(body.ctaId),
    entryPointSlug: optionalStringValue(body.entryPointSlug),
    visitorSessionId: optionalStringValue(body.visitorSessionId),
    idempotencyKey: stringValue(body.idempotencyKey),
    occurredAt: optionalStringValue(body.occurredAt),
  };

  if (!payload.eventKind || !payload.deckId || !payload.idempotencyKey) {
    return NextResponse.json(
      { error: "Story analytics event kind, deck id, and idempotency key are required." },
      { status: 400 },
    );
  }

  try {
    const daemonResponse = await fetch(`${daemonUrl()}/public/story-analytics`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(payload),
      cache: "no-store",
    });
    const responsePayload = await daemonResponse.json().catch(() => ({}));
    return NextResponse.json(responsePayload, { status: daemonResponse.status });
  } catch {
    return NextResponse.json({ error: "Story analytics route unavailable." }, { status: 502 });
  }
}

async function readJson(request: Request): Promise<PublicStoryAnalyticsRequest> {
  try {
    const value = await request.json();
    return isRecord(value) ? value : {};
  } catch {
    return {};
  }
}

function stringValue(value: unknown): string {
  return typeof value === "string" ? value.trim() : "";
}

function optionalStringValue(value: unknown): string | undefined {
  const normalized = stringValue(value);
  return normalized || undefined;
}

function numberValue(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? Math.trunc(value) : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}
