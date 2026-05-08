import { NextResponse } from "next/server";

import { postDaemonJson } from "@/lib/daemon-client";

export async function POST() {
  try {
    const result = await postDaemonJson("/backups/create");
    return NextResponse.json(result);
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Backup request failed." },
      { status: 502 },
    );
  }
}