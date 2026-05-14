import { cookies } from "next/headers";
import { NextResponse } from "next/server";

import { getProviderSnapshot } from "@/lib/daemon-client";
import { LOCAL_SESSION_COOKIE_NAME, parseLocalSessionCookie } from "@/lib/local-session";

export const runtime = "nodejs";

export async function GET() {
  const cookieStore = await cookies();
  const session = parseLocalSessionCookie(cookieStore.get(LOCAL_SESSION_COOKIE_NAME)?.value);

  if (!session) {
    return NextResponse.json({ error: "Start a local appliance session before choosing a provider." }, { status: 401 });
  }

  try {
    const snapshot = await getProviderSnapshot();
    if (snapshot.degradedReason) {
      return NextResponse.json({
        status: "degraded",
        readiness: snapshot.readiness,
        providers: [],
        degradedReason: snapshot.degradedReason,
      });
    }

    return NextResponse.json({
      status: "ready",
      readiness: snapshot.readiness,
      providers: snapshot.providers.map((provider) => ({
        providerId: provider.providerId,
        providerName: provider.providerName,
        enabled: provider.enabled,
        defaultProvider: provider.defaultProvider,
        model: provider.model,
        availableModels: provider.availableModels,
        apiKeyConfigured: provider.apiKey.configured,
        apiKeySource: provider.apiKey.source,
      })),
    });
  } catch {
    return NextResponse.json({
      status: "degraded",
      readiness: null,
      providers: [],
      degradedReason: "Provider read model is unavailable.",
    });
  }
}
