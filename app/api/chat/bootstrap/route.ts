import { cookies } from "next/headers";
import { NextResponse } from "next/server";

import { daemonChatWebSocketUrl, postDaemonJson } from "@/lib/daemon-client";
import { LOCAL_SESSION_COOKIE_NAME, parseLocalSessionCookie } from "@/lib/local-session";

export const runtime = "nodejs";

interface ChatBootstrapTransport {
  route: string;
  protocol: string;
  url?: string;
}

interface ChatBootstrapReadModel {
  schemaVersion: string;
  actorId: string;
  conversationId: string;
  participantId: string;
  assistantParticipantId: string;
  transport: ChatBootstrapTransport;
}

interface ChatBootstrapDaemonResponse {
  bootstrap: ChatBootstrapReadModel;
}

export async function POST() {
  const cookieStore = await cookies();
  const session = parseLocalSessionCookie(cookieStore.get(LOCAL_SESSION_COOKIE_NAME)?.value);

  if (!session) {
    return NextResponse.json({ error: "Start a local appliance session before opening chat." }, { status: 401 });
  }

  try {
    const daemonResponse = await postDaemonJson<ChatBootstrapDaemonResponse>("/chat/bootstrap", {
      sessionId: session.sessionId,
      actorId: session.actorId,
    });

    return NextResponse.json({
      authenticated: true,
      bootstrap: {
        ...daemonResponse.bootstrap,
        transport: {
          ...daemonResponse.bootstrap.transport,
          url: daemonChatWebSocketUrl(),
        },
      },
      status: "ready",
      degradedReason: null,
    });
  } catch {
    return NextResponse.json({
      authenticated: true,
      bootstrap: null,
      status: "degraded",
      degradedReason: "Conversation gateway unavailable; live replies stream through the server without gateway persistence.",
    });
  }
}
