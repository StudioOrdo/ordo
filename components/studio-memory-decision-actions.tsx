"use client";

import { useState } from "react";

import { statusClass } from "@/components/system-panels";
import type { ProductRole } from "@/lib/product-navigation";

export function StudioMemoryDecisionActions({
  candidateId,
  evidenceRefs,
  disabled,
  role,
}: {
  candidateId: string;
  evidenceRefs: string[];
  disabled: boolean;
  role: ProductRole;
}) {
  const [status, setStatus] = useState<"idle" | "submitting" | "recorded" | "error">("idle");
  const [message, setMessage] = useState<string | null>(null);

  async function submitDecision(decision: "approved" | "rejected") {
    if (disabled || status === "submitting") {
      return;
    }
    setStatus("submitting");
    setMessage(null);
    try {
      const response = await fetch(
        `/api/studio/generated-content-memory/candidates/${encodeURIComponent(candidateId)}/decision?role=${encodeURIComponent(role)}`,
        {
          method: "POST",
          headers: { "content-type": "application/json", "x-ordo-product-role": role },
          body: JSON.stringify({
            decision,
            reason:
              decision === "approved"
                ? "Owner/staff approved candidate memory from Studio Publications."
                : "Owner/staff rejected candidate memory from Studio Publications.",
            evidenceRefs,
          }),
        },
      );
      if (!response.ok) {
        throw new Error(`Decision failed with ${response.status}`);
      }
      setStatus("recorded");
      setMessage("Decision recorded.");
    } catch (error) {
      setStatus("error");
      setMessage(error instanceof Error ? error.message : "Decision failed.");
    }
  }

  return (
    <div className="action-row">
      <button
        className="button-secondary"
        type="button"
        disabled={disabled || status === "submitting" || status === "recorded"}
        aria-label={`Approve memory candidate ${candidateId}`}
        onClick={() => void submitDecision("approved")}
      >
        {status === "submitting" ? "Recording..." : "Approve"}
      </button>
      <button
        className="button-secondary"
        type="button"
        disabled={disabled || status === "submitting" || status === "recorded"}
        aria-label={`Reject memory candidate ${candidateId}`}
        onClick={() => void submitDecision("rejected")}
      >
        Reject
      </button>
      {message ? <span className={statusClass(status === "error" ? "error" : "ready")}>{message}</span> : null}
    </div>
  );
}
