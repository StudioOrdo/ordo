import { NextResponse } from "next/server";

import { daemonUrl } from "@/lib/daemon-client";

export const runtime = "nodejs";

interface RelationshipHandoffRequest {
  visitorSessionId?: unknown;
  locationLabel?: unknown;
  locationKind?: unknown;
  evidenceRefs?: unknown;
}

export async function POST(request: Request, { params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params;
  const body = await readJson(request);
  const entryPointSlug = typeof slug === "string" ? slug.trim() : "";
  const visitorSessionId = typeof body.visitorSessionId === "string" ? body.visitorSessionId.trim() : "";

  if (!entryPointSlug) {
    return NextResponse.json({ error: "Entry point slug is required." }, { status: 400 });
  }
  if (!visitorSessionId) {
    return NextResponse.json({ error: "Visitor session id is required." }, { status: 400 });
  }

  try {
    const daemonResponse = await fetch(`${daemonUrl()}/public/e/${encodeURIComponent(entryPointSlug)}/relationship-handoff`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        visitorSessionId,
        locationLabel: typeof body.locationLabel === "string" && body.locationLabel.trim() ? body.locationLabel : undefined,
        locationKind: typeof body.locationKind === "string" && body.locationKind.trim() ? body.locationKind : undefined,
        evidenceRefs: Array.isArray(body.evidenceRefs) ? body.evidenceRefs.filter((value) => typeof value === "string") : undefined,
      }),
      cache: "no-store",
    });
    const payload = await daemonResponse.json().catch(() => ({}));
    return NextResponse.json(payload, { status: daemonResponse.status });
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Relationship handoff route unavailable." },
      { status: 502 },
    );
  }
}

async function readJson(request: Request): Promise<RelationshipHandoffRequest> {
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
