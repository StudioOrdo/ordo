import type { ProductRole } from "@/lib/product-navigation";

export type StudioSurfaceRoom = "runs" | "artifacts";
export type StudioWorkViewer = "staff" | "owner";

export interface StudioSurfaceWorkItem {
  id: string;
  surfaceKind: string;
  roomKind: string;
  sourceKind: string;
  sourceId: string;
  objectKind: string;
  objectId: string;
  title: string;
  summary: string;
  status: string;
  priority: number;
  actorContext: Record<string, unknown>;
  connectionContext: Record<string, unknown>;
  evidenceRefs: string[];
  actions: string[];
  visibility: string;
  createdAt: string;
  updatedAt: string;
  projectedAt: string;
}

export interface StudioWorkItemView {
  id: string;
  roomKind: StudioSurfaceRoom;
  sourceKind: string;
  sourceId: string;
  objectKind: string;
  objectId: string;
  title: string;
  summary: string;
  status: string;
  priority: number;
  evidenceRefs: string[];
  actions: string[];
  actionLabels: string[];
  visibility: string;
  createdAt: string;
  updatedAt: string;
  projectedAt: string;
}

export interface StudioDeferredAction {
  key: string;
  label: string;
  reason: string;
}

export interface StudioRoomSummary {
  roomKind: StudioSurfaceRoom;
  label: string;
  emptyLabel: string;
  items: StudioWorkItemView[];
  statusCounts: Record<string, number>;
  backedActionLabels: string[];
}

export interface StudioWorkSnapshotView {
  totalItems: number;
  items: StudioWorkItemView[];
  runs: StudioRoomSummary;
  artifacts: StudioRoomSummary;
  statusCounts: Record<string, number>;
  backedActions: string[];
  backedActionLabels: string[];
  deferredActions: StudioDeferredAction[];
}

const roomLabels: Record<StudioSurfaceRoom, { label: string; emptyLabel: string }> = {
  runs: {
    label: "Production Runs",
    emptyLabel: "No durable production runs are available.",
  },
  artifacts: {
    label: "Artifacts",
    emptyLabel: "No durable artifacts are available.",
  },
};

const actionLabels: Record<string, string> = {
  inspect_job: "Inspect job",
  review_artifact: "Review artifact",
  approve_artifact: "Approve artifact",
  request_revision: "Request revision",
  stage_output: "Stage output",
};

const baselineDeferredActions: StudioDeferredAction[] = [
  {
    key: "approve_artifact",
    label: "Approve unavailable",
    reason: "Artifact approval needs a durable mutation route before the UI can present it as an action.",
  },
  {
    key: "request_revision",
    label: "Request revision unavailable",
    reason: "Revision requests need a canonical request/artifact transition before the UI can submit them.",
  },
  {
    key: "stage_output",
    label: "Stage output unavailable",
    reason: "Staging must be backed by artifact publication state before this control is enabled.",
  },
  {
    key: "generate_media",
    label: "Generate media unavailable",
    reason: "Promo media generation belongs to the staged package workflow and is not part of this baseline.",
  },
  {
    key: "publish_external",
    label: "External publishing unavailable",
    reason: "TikTok, YouTube, OAuth, and platform analytics are out of scope until guarded adapters exist.",
  },
];

export function studioViewerForRole(role: ProductRole): StudioWorkViewer | null {
  if (role === "studio" || role === "manager") {
    return "staff";
  }
  if (role === "owner" || role === "admin") {
    return "owner";
  }
  return null;
}

export function buildStudioWorkSnapshot(items: readonly StudioSurfaceWorkItem[]): StudioWorkSnapshotView {
  const safeItems = items
    .filter((item) => item.surfaceKind === "studio" && isStudioSurfaceRoom(item.roomKind))
    .map(toStudioWorkItemView)
    .sort((left, right) => {
      if (right.priority !== left.priority) {
        return right.priority - left.priority;
      }
      if (right.updatedAt !== left.updatedAt) {
        return right.updatedAt.localeCompare(left.updatedAt);
      }
      return left.id.localeCompare(right.id);
    });
  const backedActions = uniqueSorted(safeItems.flatMap((item) => item.actions));

  return {
    totalItems: safeItems.length,
    items: safeItems,
    runs: buildRoomSummary("runs", safeItems),
    artifacts: buildRoomSummary("artifacts", safeItems),
    statusCounts: countBy(safeItems.map((item) => item.status)),
    backedActions,
    backedActionLabels: backedActions.map(studioActionLabel),
    deferredActions: deferredStudioActions(backedActions),
  };
}

export function deferredStudioActions(backedActions: readonly string[]): StudioDeferredAction[] {
  const backed = new Set(backedActions);
  return baselineDeferredActions.filter((action) => !backed.has(action.key));
}

export function studioActionLabel(action: string): string {
  return actionLabels[action] ?? humanizeIdentifier(action);
}

function buildRoomSummary(roomKind: StudioSurfaceRoom, items: StudioWorkItemView[]): StudioRoomSummary {
  const roomItems = items.filter((item) => item.roomKind === roomKind);
  const backedActions = uniqueSorted(roomItems.flatMap((item) => item.actions));
  return {
    roomKind,
    label: roomLabels[roomKind].label,
    emptyLabel: roomLabels[roomKind].emptyLabel,
    items: roomItems,
    statusCounts: countBy(roomItems.map((item) => item.status)),
    backedActionLabels: backedActions.map(studioActionLabel),
  };
}

function toStudioWorkItemView(item: StudioSurfaceWorkItem): StudioWorkItemView {
  const roomKind = isStudioSurfaceRoom(item.roomKind) ? item.roomKind : "runs";
  return {
    id: item.id,
    roomKind,
    sourceKind: item.sourceKind,
    sourceId: item.sourceId,
    objectKind: item.objectKind,
    objectId: item.objectId,
    title: item.title,
    summary: item.summary,
    status: item.status,
    priority: item.priority,
    evidenceRefs: [...item.evidenceRefs],
    actions: [...item.actions],
    actionLabels: item.actions.map(studioActionLabel),
    visibility: item.visibility,
    createdAt: item.createdAt,
    updatedAt: item.updatedAt,
    projectedAt: item.projectedAt,
  };
}

function isStudioSurfaceRoom(value: string): value is StudioSurfaceRoom {
  return value === "runs" || value === "artifacts";
}

function countBy(values: string[]): Record<string, number> {
  return values.reduce<Record<string, number>>((counts, value) => {
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {});
}

function uniqueSorted(values: string[]): string[] {
  return [...new Set(values)].sort((left, right) => studioActionLabel(left).localeCompare(studioActionLabel(right)));
}

function humanizeIdentifier(value: string): string {
  return value
    .split(/[_\s.-]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}
