import { NextResponse } from "next/server";

import { putDaemonJson } from "@/lib/daemon-client";
import { canAccessAppSpace, resolveProductRole } from "@/lib/product-navigation";

export async function POST(request: Request, { params }: { params: Promise<{ proposalId: string }> }) {
  try {
    const role = resolveProductRole(request.headers.get("x-ordo-product-role") ?? undefined);
    if (!canAccessAppSpace(role, "studio")) {
      return NextResponse.json({ error: "Studio artifact patch review is restricted to Studio operators." }, { status: 403 });
    }
    const { proposalId } = await params;
    const body = (await request.json()) as { currentText?: string };
    const result = await putDaemonJson(`/studio/artifact-patches/${encodeURIComponent(proposalId)}/accept`, {
      currentText: body.currentText ?? "",
    });
    return NextResponse.json(result);
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Artifact patch accept failed." },
      { status: 502 },
    );
  }
}
