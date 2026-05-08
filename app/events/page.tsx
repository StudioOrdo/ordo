import { SystemShell } from "@/components/system-shell";
import { PageTitle } from "@/components/system-panels";
import { getEventReplaySnapshot, getSystemSnapshot, RealtimeEventSummary } from "@/lib/daemon-client";

export const dynamic = "force-dynamic";

export default async function EventsPage() {
  const [snapshot, eventSnapshot] = await Promise.all([getSystemSnapshot(), getEventReplaySnapshot()]);

  return (
    <SystemShell currentItemId="events" websocketUrl={snapshot.degradedReason ? null : snapshot.websocketUrl}>
      <PageTitle
        eyebrow="Trail"
        title="Events"
        description="Realtime events are a live projection of persisted appliance events."
      />

      <section className="plain-panel">
        <h3 className="panel-title">Replay Cursor</h3>
        <p className="brief-body">
          {eventSnapshot.degradedReason
            ? eventSnapshot.degradedReason
            : `Showing persisted events through cursor ${eventSnapshot.nextCursor ?? 0}.`}
        </p>
      </section>

      <section className="plain-panel table-shell">
        <table className="data-table">
          <thead>
            <tr>
              <th>Sequence</th>
              <th>Event</th>
              <th>Source</th>
              <th>Time</th>
            </tr>
          </thead>
          <tbody>
            {eventSnapshot.events.length === 0 ? (
              <tr>
                <td colSpan={4} className="table-empty">
                  No persisted events are available yet.
                </td>
              </tr>
            ) : (
              eventSnapshot.events.map((event) => <EventRow key={event.cursor} event={event} />)
            )}
          </tbody>
        </table>
      </section>
    </SystemShell>
  );
}

function EventRow({ event }: { event: RealtimeEventSummary }) {
  const source = [event.family, event.jobId, event.taskKey].filter(Boolean).join(" / ");

  return (
    <tr>
      <td>{event.cursor}</td>
      <td>{event.eventType}</td>
      <td>{source || "system"}</td>
      <td>{new Date(event.occurredAt).toLocaleString()}</td>
    </tr>
  );
}
