import { NextResponse } from "next/server";

import { daemonUrl } from "@/lib/daemon-client";

export const runtime = "nodejs";

export async function GET() {
  try {
    const response = await fetch(`${daemonUrl()}/public/homepage-story`, {
      cache: "no-store",
    });
    const payload = await response.json().catch(() => ({}));
    return NextResponse.json(payload, { status: response.status });
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Homepage story deck unavailable." },
      { status: 503 },
    );
  }
}
