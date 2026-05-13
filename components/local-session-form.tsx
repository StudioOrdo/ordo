"use client";

import { useState, type FormEvent } from "react";

type LocalSessionFormMode = "login" | "register";

interface LocalSessionFormProps {
  mode: LocalSessionFormMode;
  submitLabel: string;
}

interface LocalSessionResponse {
  error?: string;
  redirectTo?: string;
}

export function LocalSessionForm({ mode, submitLabel }: LocalSessionFormProps) {
  const [status, setStatus] = useState<"idle" | "submitting" | "failed">("idle");
  const [error, setError] = useState<string | null>(null);

  async function submitSession(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    const formData = new FormData(form);
    setStatus("submitting");
    setError(null);

    try {
      const response = await fetch(`/api/local-session/${mode}`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          name: formData.get("name"),
          email: formData.get("email"),
          password: formData.get("password"),
        }),
      });
      const payload = (await response.json()) as LocalSessionResponse;
      if (!response.ok || !payload.redirectTo) {
        setStatus("failed");
        setError(payload.error ?? "This local session could not be started.");
        return;
      }
      window.location.assign(payload.redirectTo);
    } catch {
      setStatus("failed");
      setError("This local session could not be started.");
    }
  }

  return (
    <form className="auth-form" onSubmit={submitSession} noValidate>
      {mode === "register" ? (
        <label>
          Name
          <input type="text" name="name" autoComplete="name" maxLength={80} required />
        </label>
      ) : null}
      <label>
        Email
        <input type="email" name="email" autoComplete="email" placeholder="you@example.com" maxLength={254} required />
      </label>
      <label>
        Password
        <input
          type="password"
          name="password"
          autoComplete={mode === "register" ? "new-password" : "current-password"}
          minLength={8}
          maxLength={128}
          required
        />
      </label>
      {error ? (
        <p className="auth-alert" role="alert">
          {error}
        </p>
      ) : null}
      <button type="submit" className="primary-action" disabled={status === "submitting"}>
        {status === "submitting" ? "Starting..." : submitLabel}
      </button>
    </form>
  );
}