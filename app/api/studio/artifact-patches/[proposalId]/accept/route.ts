import { NextResponse } from "next/server";

import { putDaemonJson } from "@/lib/daemon-client";

export async function POST(request: Request, { params }: { params: Promise<{ proposalId: string }> }) {
  try {
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
