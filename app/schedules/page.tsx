import { SystemShell } from "@/components/system-shell";
import { PageTitle } from "@/components/system-panels";
import { getSystemSnapshot } from "@/lib/daemon-client";

export const dynamic = "force-dynamic";

const plannedSchedules = [
  ["system.health.check", "interval", "Template registered"],
  ["brief.system.generate", "interval", "Scheduled hourly"],
  ["backup.create", "interval", "Pending executor"],
] as const;

export default async function SchedulesPage() {
  const snapshot = await getSystemSnapshot();

  return (
    <SystemShell currentItemId="schedules" websocketUrl={snapshot.degradedReason ? null : snapshot.websocketUrl}>
      <PageTitle
        eyebrow="Clock"
        title="Schedules"
        description="Scheduled work creates jobs from approved process templates."
      />

      <section className="plain-panel table-shell">
        <table className="data-table">
          <thead>
            <tr>
              <th>Template</th>
              <th>Kind</th>
              <th>State</th>
            </tr>
          </thead>
          <tbody>
            {plannedSchedules.map(([template, kind, state]) => (
              <tr key={template}>
                <td>{template}</td>
                <td>{kind}</td>
                <td>{state}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </SystemShell>
  );
}