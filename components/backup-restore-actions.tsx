"use client";

import { useMemo, useState } from "react";

interface Props {
  latestBackupId: string | null;
  disabled: boolean;
}

export function BackupRestoreActions({ latestBackupId, disabled }: Props) {
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [backupId, setBackupId] = useState(latestBackupId ?? "");
  const [confirmation, setConfirmation] = useState(latestBackupId ? `RESTORE ${latestBackupId}` : "");
  const [message, setMessage] = useState<string | null>(null);

  const canValidateRestore = useMemo(
    () => backupId.trim().length > 0 && confirmation.trim().length > 0 && !disabled && !busyAction,
    [backupId, busyAction, confirmation, disabled],
  );

  async function runAction(path: string, body?: unknown) {
    setBusyAction(path);
    setMessage(null);
    try {
      const response = await fetch(path, {
        method: "POST",
        headers: body ? { "content-type": "application/json" } : undefined,
        body: body ? JSON.stringify(body) : undefined,
      });
      if (!response.ok) {
        const payload = (await response.json().catch(() => ({}))) as { error?: string };
        throw new Error(payload.error ?? `${path} failed`);
      }
      window.location.reload();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Request failed.");
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <section className="plain-panel action-panel">
      <div className="action-row">
        <button className="button-primary" disabled={disabled || Boolean(busyAction)} onClick={() => runAction("/api/backups/create")}>
          Create Backup
        </button>
        <span className="muted">{busyAction === "/api/backups/create" ? "Creating backup." : "Manual safety capture."}</span>
      </div>

      <div className="restore-form">
        <label>
          <span className="label">Backup ID</span>
          <input className="text-input" value={backupId} disabled={disabled || Boolean(busyAction)} onChange={(event) => setBackupId(event.target.value)} />
        </label>
        <label>
          <span className="label">Confirmation</span>
          <input
            className="text-input"
            value={confirmation}
            disabled={disabled || Boolean(busyAction)}
            onChange={(event) => setConfirmation(event.target.value)}
          />
        </label>
        <button
          className="button-secondary"
          disabled={!canValidateRestore}
          onClick={() => runAction("/api/restores/validate", { backupId, confirmation })}
        >
          Validate Restore
        </button>
      </div>

      {message ? <p className="action-message" aria-live="polite">{message}</p> : null}
    </section>
  );
}