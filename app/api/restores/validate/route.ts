import { NextResponse } from "next/server";

import { postDaemonJson } from "@/lib/daemon-client";

export async function POST(request: Request) {
  try {
    const body = (await request.json()) as { backupId?: string; confirmation?: string };
    const result = await postDaemonJson("/restore/validate", {
      backupId: body.backupId ?? "",
      confirmation: body.confirmation ?? "",
    });
    return NextResponse.json(result);
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Restore validation failed." },
      { status: 502 },
    );
  }
}