import Link from "next/link";
import { cookies } from "next/headers";

import { MemberOrdoSurface } from "@/components/member-ordo-surface";
import { PublicTopRail } from "@/components/public-surface-deck";
import { LOCAL_SESSION_COOKIE_NAME, parseLocalSessionCookie } from "@/lib/local-session";
import { type SearchParams } from "@/lib/page-role";

export default async function MyChatPage({ searchParams }: { searchParams?: SearchParams }) {
  const cookieStore = await cookies();
  const session = parseLocalSessionCookie(cookieStore.get(LOCAL_SESSION_COOKIE_NAME)?.value);

  if (!session) {
    return (
      <>
        <PublicTopRail role="anonymous" />
        <main className="auth-page">
          <section className="auth-panel" aria-labelledby="session-required-title">
            <span className="eyebrow">Account</span>
            <h1 id="session-required-title">Login required</h1>
            <p>Start a local appliance session before opening My Ordo.</p>
            <Link href="/login" className="primary-action">
              Login
            </Link>
            <div className="auth-links">
              <Link href="/register">Create account</Link>
            </div>
          </section>
        </main>
      </>
    );
  }

  return await MemberOrdoSurface({ searchParams, roomId: "ordo" });
}
