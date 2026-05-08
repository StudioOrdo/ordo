import { NextResponse } from "next/server";

import { postDaemonJson } from "@/lib/daemon-client";

export async function POST(request: Request) {
  try {
    const body = await request.json();
    const result = await postDaemonJson("/reports/issues/prepare", body);
    return NextResponse.json(result);
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Report preparation failed." },
      { status: 502 },
    );
  }
}
