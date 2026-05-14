"use client";

import { useState } from "react";

import { statusClass } from "@/components/system-panels";
import type { ProductRole } from "@/lib/product-navigation";

export function StudioArtifactPatchAcceptForm({
  proposalId,
  disabled,
  role,
}: {
  proposalId: string;
  disabled: boolean;
  role: ProductRole;
}) {
  const [currentText, setCurrentText] = useState("");
  const [status, setStatus] = useState<"idle" | "submitting" | "accepted" | "error">("idle");
  const [message, setMessage] = useState<string | null>(null);
  const canSubmit = !disabled && currentText.trim().length > 0 && status !== "submitting";

  async function submitPatch(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit) {
      return;
    }
    setStatus("submitting");
    setMessage(null);
    try {
      const response = await fetch(`/api/studio/artifact-patches/${encodeURIComponent(proposalId)}/accept`, {
        method: "POST",
        headers: { "content-type": "application/json", "x-ordo-product-role": role },
        body: JSON.stringify({ currentText }),
      });
      if (!response.ok) {
        throw new Error(`Accept failed with ${response.status}`);
      }
      setStatus("accepted");
      setMessage("Accepted through governed artifact version history.");
      setCurrentText("");
    } catch (error) {
      setStatus("error");
      setMessage(error instanceof Error ? error.message : "Accept failed.");
    }
  }

  return (
    <form className="restore-form" onSubmit={submitPatch}>
      <label>
        Current artifact text
        <textarea
          className="text-input text-area compact"
          value={currentText}
          disabled={disabled || status === "submitting" || status === "accepted"}
          onChange={(event) => setCurrentText(event.target.value)}
        />
      </label>
      <button className="button-primary" type="submit" disabled={!canSubmit}>
        {status === "submitting" ? "Accepting..." : "Accept patch"}
      </button>
      {message ? <span className={statusClass(status === "error" ? "error" : "ready")}>{message}</span> : null}
    </form>
  );
}
