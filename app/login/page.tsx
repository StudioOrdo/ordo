import Link from "next/link";

import { LocalSessionForm } from "@/components/local-session-form";
import { PublicTopRail } from "@/components/public-surface-deck";

export default function LoginPage() {
  return (
    <>
      <PublicTopRail role="anonymous" />
      <main className="auth-page">
        <section className="auth-panel" aria-labelledby="login-title">
          <span className="eyebrow">Account</span>
          <h1 id="login-title">Login</h1>
          <p>Start a local appliance session to open your My Ordo conversation.</p>
          <LocalSessionForm mode="login" submitLabel="Continue" />
          <div className="auth-links">
            <Link href="/register">Create account</Link>
            <Link href="/account">Account status</Link>
          </div>
        </section>
      </main>
    </>
  );
}
