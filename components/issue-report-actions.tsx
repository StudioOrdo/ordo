"use client";

import { useState } from "react";

import type { IssueReportArtifact, IssueSeverity } from "@/lib/daemon-client";

interface Props {
  disabled: boolean;
  latestReport: IssueReportArtifact | null;
}


export function IssueReportActions({ disabled, latestReport }: Props) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [title, setTitle] = useState("Local diagnostic report");
  const [severity, setSeverity] = useState<IssueSeverity>("medium");
  const [description, setDescription] = useState("");
  const [expectedBehavior, setExpectedBehavior] = useState("");
  const [actualBehavior, setActualBehavior] = useState("");
  const [stepsText, setStepsText] = useState("");
  const [sourceRoute, setSourceRoute] = useState("");
  const [includeHealthSnapshot, setIncludeHealthSnapshot] = useState(true);
  const [includeRecentEvents, setIncludeRecentEvents] = useState(true);
  const [includeRecentJobs, setIncludeRecentJobs] = useState(true);
  const [includeDiagnosticLogs, setIncludeDiagnosticLogs] = useState(true);

  async function prepareReport() {
    setBusy(true);
    setMessage(null);
    try {
      const response = await fetch("/api/reports/issues/prepare", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          title,
          severity,
          description,
          expectedBehavior,
          actualBehavior,
          steps: stepsText.split("\n").map((step) => step.trim()).filter(Boolean),
          sourceRoute,
          includeHealthSnapshot,
          includeRecentEvents,
          includeRecentJobs,
          includeDiagnosticLogs,
          includeBrowserContext: false,
        }),
      });
      if (!response.ok) {
        const payload = (await response.json().catch(() => ({}))) as { error?: string };
        throw new Error(payload.error ?? "Report preparation failed.");
      }
      window.location.reload();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Report preparation failed.");
    } finally {
      setBusy(false);
    }
  }

  async function copyLatestReport() {
    if (!latestReport) return;
    await navigator.clipboard.writeText(latestReport.markdownBody);
    setMessage("Report markdown copied.");
  }

  function exportLatestReport() {
    if (!latestReport) return;
    const blob = new Blob([latestReport.markdownBody], { type: "text/markdown" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${latestReport.id}.md`;
    document.body.appendChild(anchor);
    anchor.click();
    document.body.removeChild(anchor);
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    setMessage("Report markdown exported.");
  }

  const canSubmit = !disabled && !busy && description.trim().length > 0;

  return (
    <section className="plain-panel action-panel">
      <div className="report-form-grid">
        <label>
          <span className="label">Title</span>
          <input className="text-input" value={title} disabled={disabled || busy} onChange={(event) => setTitle(event.target.value)} />
        </label>
        <label>
          <span className="label">Severity</span>
          <select className="text-input" value={severity} disabled={disabled || busy} onChange={(event) => setSeverity(event.target.value as IssueSeverity)}>
            <option value="low">low</option>
            <option value="medium">medium</option>
            <option value="high">high</option>
            <option value="blocker">blocker</option>
          </select>
        </label>
        <label>
          <span className="label">Source route</span>
          <input className="text-input" value={sourceRoute} disabled={disabled || busy} onChange={(event) => setSourceRoute(event.target.value)} />
        </label>
      </div>

      <label>
        <span className="label">Description</span>
        <textarea className="text-input text-area" value={description} disabled={disabled || busy} onChange={(event) => setDescription(event.target.value)} />
      </label>

      <div className="report-form-grid">
        <label>
          <span className="label">Expected behavior</span>
          <textarea className="text-input text-area compact" value={expectedBehavior} disabled={disabled || busy} onChange={(event) => setExpectedBehavior(event.target.value)} />
        </label>
        <label>
          <span className="label">Actual behavior</span>
          <textarea className="text-input text-area compact" value={actualBehavior} disabled={disabled || busy} onChange={(event) => setActualBehavior(event.target.value)} />
        </label>
        <label>
          <span className="label">Steps</span>
          <textarea className="text-input text-area compact" value={stepsText} disabled={disabled || busy} onChange={(event) => setStepsText(event.target.value)} />
        </label>
      </div>

      <div className="checkbox-grid">
        <label><input type="checkbox" checked={includeHealthSnapshot} disabled={disabled || busy} onChange={(event) => setIncludeHealthSnapshot(event.target.checked)} /> Health and readiness</label>
        <label><input type="checkbox" checked={includeRecentEvents} disabled={disabled || busy} onChange={(event) => setIncludeRecentEvents(event.target.checked)} /> Recent events</label>
        <label><input type="checkbox" checked={includeRecentJobs} disabled={disabled || busy} onChange={(event) => setIncludeRecentJobs(event.target.checked)} /> Recent jobs</label>
        <label><input type="checkbox" checked={includeDiagnosticLogs} disabled={disabled || busy} onChange={(event) => setIncludeDiagnosticLogs(event.target.checked)} /> Diagnostic logs</label>
      </div>

      <div className="action-row">
        <button className="button-primary" disabled={!canSubmit} onClick={prepareReport}>Prepare Report</button>
        <button className="button-secondary" disabled={!latestReport} onClick={copyLatestReport}>Copy Markdown</button>
        <button className="button-secondary" disabled={!latestReport} onClick={exportLatestReport}>Export Markdown</button>
        <span className="muted">Local report preparation only.</span>
      </div>

      {message ? <p className="action-message" aria-live="polite">{message}</p> : null}
    </section>
  );
}