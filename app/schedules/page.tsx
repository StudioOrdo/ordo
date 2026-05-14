import { SystemShell } from "@/components/system-shell";
import { PageTitle, statusClass } from "@/components/system-panels";
import {
  getSchedulerOperationsSnapshot,
  getSystemSnapshot,
  type SchedulerOperationsSchedule,
  type SchedulerOperationsRun,
} from "@/lib/daemon-client";

export const dynamic = "force-dynamic";

export default async function SchedulesPage() {
  const [snapshot, schedulesSnapshot] = await Promise.all([getSystemSnapshot(), getSchedulerOperationsSnapshot()]);

  return (
    <SystemShell currentItemId="schedules" websocketUrl={snapshot.degradedReason ? null : snapshot.websocketUrl}>
      <PageTitle
        eyebrow="Clock"
        title="Schedules"
        description="Scheduled work creates jobs from approved process templates."
      />

      {schedulesSnapshot.degradedReason ? (
        <section className="plain-panel">
          <h3 className="panel-title">State</h3>
          <p className="brief-body">{schedulesSnapshot.degradedReason}</p>
        </section>
      ) : null}

      <section className="plain-panel table-shell">
        <table className="data-table">
          <thead>
            <tr>
              <th>Template</th>
              <th>Kind</th>
              <th>State</th>
              <th>Next Due</th>
              <th>Last Run</th>
            </tr>
          </thead>
          <tbody>
            {schedulesSnapshot.schedules.length === 0 ? (
              <tr>
                <td colSpan={5} className="table-empty">
                  {schedulesSnapshot.degradedReason
                    ? "Scheduler operations are unavailable."
                    : "No durable schedules are available yet."}
                </td>
              </tr>
            ) : (
              schedulesSnapshot.schedules.map((schedule) => <ScheduleRow key={schedule.id} schedule={schedule} />)
            )}
          </tbody>
        </table>
      </section>

      <section className="plain-panel">
        <h3 className="panel-title">Evidence</h3>
        <ul className="evidence-list">
          <li>
            <span className="label">Snapshot</span>
            <span className="value">{schedulesSnapshot.createdAt}</span>
          </li>
          <li>
            <span className="label">Daemon read model</span>
            <span className="value">{schedulesSnapshot.generatedAt ?? "unavailable"}</span>
          </li>
          <li>
            <span className="label">Daemon</span>
            <span className="value">{schedulesSnapshot.daemonUrl}</span>
          </li>
        </ul>
      </section>
    </SystemShell>
  );
}

function ScheduleRow({ schedule }: { schedule: SchedulerOperationsSchedule }) {
  const shape = scheduleShape(schedule);
  const lastRun = schedule.lastRun;
  const state = schedule.enabled ? "enabled" : "disabled";

  return (
    <tr>
      <td>
        <strong>{schedule.name}</strong>
        <span className="table-subtle">
          {schedule.templateId} v{schedule.templateVersion}
        </span>
      </td>
      <td>
        {schedule.scheduleKind}
        <span className="table-subtle">{shape}</span>
      </td>
      <td>
        <span className={statusClass(schedule.enabled ? "ready" : "blocked")}>{state}</span>
        {schedule.limitations.length > 0 ? (
          <span className="table-subtle">{schedule.limitations.join(" ")}</span>
        ) : null}
      </td>
      <td>
        {formatDate(schedule.nextDueAt)}
        <span className="table-subtle">{schedule.timezone}</span>
      </td>
      <td>
        {lastRun ? (
          <>
            <span className={statusClass(runStatusTone(lastRun))}>{lastRun.status}</span>
            <span className="table-subtle">{lastRun.jobId ?? lastRun.id}</span>
          </>
        ) : (
          "none"
        )}
      </td>
    </tr>
  );
}

function scheduleShape(schedule: SchedulerOperationsSchedule): string {
  if (schedule.scheduleKind === "cron") {
    return schedule.cronExpression ?? "cron expression unavailable";
  }
  if (schedule.scheduleKind === "interval") {
    return schedule.intervalSeconds === null ? "interval unavailable" : `${schedule.intervalSeconds}s`;
  }
  if (schedule.scheduleKind === "one_shot") {
    return schedule.runAt ? `runs ${formatDate(schedule.runAt)}` : "one-shot time unavailable";
  }
  return "unknown schedule shape";
}

function runStatusTone(run: SchedulerOperationsRun): string {
  if (run.hasError || run.status === "failed") {
    return "error";
  }
  if (run.status === "completed") {
    return "succeeded";
  }
  if (run.status === "claimed") {
    return "running";
  }
  return run.status;
}

function formatDate(value: string): string {
  return new Date(value).toLocaleString();
}
