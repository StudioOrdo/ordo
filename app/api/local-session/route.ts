import { NextResponse, type NextRequest } from "next/server";

import { LOCAL_SESSION_COOKIE_NAME, parseLocalSessionCookie } from "@/lib/local-session";

export const runtime = "nodejs";

export async function GET(request: NextRequest) {
  const session = parseLocalSessionCookie(request.cookies.get(LOCAL_SESSION_COOKIE_NAME)?.value);
  return NextResponse.json({ authenticated: Boolean(session), session });
}