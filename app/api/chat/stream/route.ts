import { cookies } from "next/headers";
import { NextResponse } from "next/server";

import { LOCAL_SESSION_COOKIE_NAME, parseLocalSessionCookie } from "@/lib/local-session";

export const runtime = "nodejs";

interface ChatStreamRequest {
  message?: unknown;
}

const MAX_MESSAGE_CHARS = 4_000;

export async function POST(request: Request) {
  const cookieStore = await cookies();
  const session = parseLocalSessionCookie(cookieStore.get(LOCAL_SESSION_COOKIE_NAME)?.value);

  if (!session) {
    return NextResponse.json({ error: "Start a local appliance session before opening chat." }, { status: 401 });
  }

  const payload = await readJson(request);
  if (!payload.ok) {
    return NextResponse.json({ error: "Send a valid JSON chat stream request." }, { status: 400 });
  }

  const message = typeof payload.value.message === "string" ? payload.value.message.trim() : "";
  if (!message) {
    return NextResponse.json({ error: "Enter a message before streaming a reply." }, { status: 400 });
  }
  if (message.length > MAX_MESSAGE_CHARS) {
    return NextResponse.json({ error: "Keep the message shorter before streaming a reply." }, { status: 400 });
  }

  return NextResponse.json(
    { error: "Direct chat streaming is disabled. Member chat replies must run through the daemon conversation gateway." },
    { status: 503 },
  );
}

async function readJson(request: Request): Promise<{ ok: true; value: ChatStreamRequest } | { ok: false }> {
  try {
    const value = (await request.json()) as ChatStreamRequest;
    return { ok: true, value };
  } catch {
    return { ok: false };
  }
}
