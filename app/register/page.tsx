import Link from "next/link";

import { LocalSessionForm } from "@/components/local-session-form";
import { PublicTopRail } from "@/components/public-surface-deck";

export default function RegisterPage() {
  return (
    <>
      <PublicTopRail role="anonymous" />
      <main className="auth-page">
        <section className="auth-panel" aria-labelledby="register-title">
          <span className="eyebrow">Account</span>
          <h1 id="register-title">Register</h1>
          <p>Create a local appliance session to open your My Ordo conversation.</p>
          <LocalSessionForm mode="register" submitLabel="Create account" />
          <div className="auth-links">
            <Link href="/login">Already have an account</Link>
          </div>
        </section>
      </main>
    </>
  );
}
