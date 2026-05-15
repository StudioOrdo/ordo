import { NextResponse } from "next/server";

import { postDaemonJson } from "@/lib/daemon-client";
import { canAccessAppSpace, resolveProductRole } from "@/lib/product-navigation";

interface MemoryDecisionRequest {
  decision?: string;
  reason?: string;
  evidenceRefs?: unknown;
}

export async function POST(request: Request, { params }: { params: Promise<{ candidateId: string }> }) {
  try {
    const url = new URL(request.url);
    const role = resolveProductRole(url.searchParams.get("role") ?? request.headers.get("x-ordo-product-role") ?? undefined);
    if (!canAccessAppSpace(role, "studio")) {
      return NextResponse.json({ error: "Generated-content memory decisions are restricted to Studio operators." }, { status: 403 });
    }

    const { candidateId } = await params;
    const body = (await request.json()) as MemoryDecisionRequest;
    if (body.decision !== "approved" && body.decision !== "rejected") {
      return NextResponse.json({ error: "Studio Publications only supports approve or reject memory decisions." }, { status: 400 });
    }
    if (!body.reason?.trim()) {
      return NextResponse.json({ error: "Memory decision reason is required." }, { status: 400 });
    }
    const evidenceRefs = Array.isArray(body.evidenceRefs)
      ? body.evidenceRefs.filter((ref): ref is string => typeof ref === "string" && ref.trim().length > 0)
      : [];
    if (evidenceRefs.length === 0) {
      return NextResponse.json({ error: "Memory decision evidence refs are required." }, { status: 400 });
    }

    const result = await postDaemonJson(`/studio/generated-content-memory/candidates/${encodeURIComponent(candidateId)}/decision`, {
      decision: body.decision,
      reason: body.reason.trim(),
      evidenceRefs,
    });
    return NextResponse.json(result);
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Generated-content memory decision failed." },
      { status: 502 },
    );
  }
}
