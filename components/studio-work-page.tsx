import { ProductShell } from "@/components/product-shell";
import { StudioArtifactPatchReviewPanel } from "@/components/studio-artifact-patch-review";
import { PageTitle, statusClass } from "@/components/system-panels";
import {
  getStudioArtifactPatchSnapshot,
  getStudioWorkSnapshot,
  type StudioDeferredAction,
  type StudioRoomSummary,
  type StudioSurfaceRoom,
  type StudioWorkItemView,
  type StudioWorkSnapshot,
} from "@/lib/daemon-client";
import { mobileStepFromSearchParams, railModeFromSearchParams, roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { canAccessAppSpace, type ProductRole } from "@/lib/product-navigation";
import { studioViewerForRole } from "@/lib/studio-work";
import { notFound } from "next/navigation";

interface StudioWorkPageProps {
  searchParams?: SearchParams;
  currentItemId: string;
  title: string;
  description: string;
  roomKind?: StudioSurfaceRoom;
}

export async function StudioWorkPage({ searchParams, currentItemId, title, description, roomKind }: StudioWorkPageProps) {
  const requestedRole = await roleFromSearchParams(searchParams);
  if (!canAccessAppSpace(requestedRole, "studio")) {
    notFound();
  }
  const viewer = studioViewerForRole(requestedRole);
  if (!viewer) {
    notFound();
  }

  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams);
  const role: ProductRole = requestedRole;
  const snapshot = await getStudioWorkSnapshot(viewer, roomKind);
  const artifactPatchSnapshot = roomKind === "artifacts" ? await getStudioArtifactPatchSnapshot() : null;
  const degraded = Boolean(snapshot.degradedReason);
  const room = roomKind ? roomSummary(snapshot, roomKind) : null;
  const visibleItems = room?.items ?? snapshot.items;

  return (
    <ProductShell role={role} appSpaceId="studio" currentItemId={currentItemId} railMode={railMode} mobileStep={mobileStep}>
      <PageTitle eyebrow="Studio" title={title} description={description} />

      <section className="brief-panel">
        <div className="meta-row">
          <span>As of {snapshot.createdAt}</span>
          <span className={statusClass(degraded ? "error" : visibleItems.length > 0 ? "ready" : "empty")}>
            {degraded ? "degraded" : visibleItems.length > 0 ? "ready" : "empty"}
          </span>
        </div>
        <ul className="brief-list">
          {summaryLines(snapshot, room, degraded).map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      </section>

      {snapshot.degradedReason ? (
        <section className="plain-panel">
          <h3 className="panel-title">State</h3>
          <p className="brief-body">{snapshot.degradedReason}</p>
        </section>
      ) : null}

      {!roomKind ? <StudioRoomOverview snapshot={snapshot} /> : null}

      <StudioWorkTable title={room?.label ?? "All Studio Work"} items={visibleItems} emptyLabel={room?.emptyLabel ?? "No durable Studio work items are available."} />

      {artifactPatchSnapshot ? <StudioArtifactPatchReviewPanel snapshot={artifactPatchSnapshot} /> : null}

      <DeferredActionsPanel actions={snapshot.deferredActions} />
    </ProductShell>
  );
}

function StudioRoomOverview({ snapshot }: { snapshot: StudioWorkSnapshot }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Rooms</h3>
      <div className="data-row">
        <span className="label">Production runs</span>
        <span className="value">
          {snapshot.runs.items.length} item(s)
          <span className="table-subtle">{statusCountText(snapshot.runs.statusCounts)}</span>
        </span>
      </div>
      <div className="data-row">
        <span className="label">Artifacts</span>
        <span className="value">
          {snapshot.artifacts.items.length} item(s)
          <span className="table-subtle">{statusCountText(snapshot.artifacts.statusCounts)}</span>
        </span>
      </div>
      <div className="data-row">
        <span className="label">Backed actions</span>
        <span className="value">{snapshot.backedActionLabels.length > 0 ? snapshot.backedActionLabels.join(", ") : "none"}</span>
      </div>
    </section>
  );
}

function StudioWorkTable({ title, items, emptyLabel }: { title: string; items: StudioWorkItemView[]; emptyLabel: string }) {
  return (
    <section className="plain-panel table-shell">
      <h3 className="panel-title">{title}</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>Work</th>
            <th>Status</th>
            <th>Evidence</th>
            <th>Actions</th>
            <th>Updated</th>
          </tr>
        </thead>
        <tbody>
          {items.length === 0 ? (
            <tr>
              <td colSpan={5} className="table-empty">
                {emptyLabel}
              </td>
            </tr>
          ) : (
            items.map((item) => <StudioWorkRow key={item.id} item={item} />)
          )}
        </tbody>
      </table>
    </section>
  );
}

function StudioWorkRow({ item }: { item: StudioWorkItemView }) {
  return (
    <tr>
      <td>
        <strong>{item.title}</strong>
        <span className="table-subtle">
          {item.roomKind} / {item.objectKind}:{item.objectId}
        </span>
        <span className="table-subtle">{item.summary}</span>
      </td>
      <td>
        <span className={statusClass(item.status)}>{item.status}</span>
        <span className="table-subtle">{item.visibility}</span>
      </td>
      <td>{item.evidenceRefs.length > 0 ? item.evidenceRefs.join(", ") : "none"}</td>
      <td>{item.actionLabels.length > 0 ? item.actionLabels.join(", ") : "none"}</td>
      <td>{item.updatedAt}</td>
    </tr>
  );
}

function DeferredActionsPanel({ actions }: { actions: StudioDeferredAction[] }) {
  return (
    <section className="plain-panel">
      <h3 className="panel-title">Deferred Actions</h3>
      {actions.length === 0 ? (
        <p className="brief-body">All actions in this snapshot are backed by durable state.</p>
      ) : (
        actions.map((action) => (
          <div key={action.key} className="data-row">
            <span className="label">{action.label}</span>
            <span className="value">{action.reason}</span>
          </div>
        ))
      )}
    </section>
  );
}

function roomSummary(snapshot: StudioWorkSnapshot, roomKind: StudioSurfaceRoom): StudioRoomSummary {
  return roomKind === "runs" ? snapshot.runs : snapshot.artifacts;
}

function summaryLines(snapshot: StudioWorkSnapshot, room: StudioRoomSummary | null, degraded: boolean): string[] {
  if (degraded) {
    return ["Studio snapshot is degraded because the daemon work-item read model is unavailable."];
  }
  if (room) {
    if (room.items.length === 0) {
      return [room.emptyLabel];
    }
    return [
      `${room.items.length} durable ${room.label.toLowerCase()} item(s) are available.`,
      `Statuses: ${statusCountText(room.statusCounts)}.`,
      `Backed actions: ${room.backedActionLabels.length > 0 ? room.backedActionLabels.join(", ") : "none"}.`,
    ];
  }
  if (snapshot.totalItems === 0) {
    return ["No durable Studio work items are available yet."];
  }
  return [
    `${snapshot.runs.items.length} production run(s) and ${snapshot.artifacts.items.length} artifact(s) are available.`,
    `Statuses: ${statusCountText(snapshot.statusCounts)}.`,
    `Backed actions: ${snapshot.backedActionLabels.length > 0 ? snapshot.backedActionLabels.join(", ") : "none"}.`,
  ];
}

function statusCountText(statusCounts: Record<string, number>): string {
  const entries = Object.entries(statusCounts).sort(([left], [right]) => left.localeCompare(right));
  if (entries.length === 0) {
    return "none";
  }
  return entries.map(([status, count]) => `${status} ${count}`).join(", ");
}
