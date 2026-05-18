import type { HandoffInboxItemView } from "@/lib/daemon-client";

export type SupportHandoffStatus = "open" | "claimed" | "closed" | "needs_review";

export interface SupportHandoffQueueItemView {
  id: string;
  title: string;
  sourceLabel: string;
  status: SupportHandoffStatus;
  statusLabel: string;
  urgency: string;
  requestedAction: string;
  nextAction: string;
  assigneeLabel: string;
  evidenceRefCount: number;
  safeEvidenceRefs: string[];
  createdAt: string;
  updatedAt: string;
}

export interface SupportHandoffQueueView {
  status: "ready" | "empty";
  openCount: number;
  claimedCount: number;
  closedCount: number;
  evidenceRefCount: number;
  items: SupportHandoffQueueItemView[];
  summaryLines: string[];
  limitations: string[];
}

const SAFE_EVIDENCE_PREFIXES = [
  "tracked_entry_point:",
  "visitor_session:",
  "handoff:",
  "handoff_item:",
  "conversation:",
  "offer:",
  "access:",
  "feedback:",
];

const UNSAFE_TEXT_PATTERNS = [
  /provider/i,
  /prompt/i,
  /policy/i,
  /secret/i,
  /private/i,
  /sk_live/i,
  /api[_-]?key/i,
  /raw/i,
  /internal/i,
];

export function buildSupportHandoffQueueView(items: HandoffInboxItemView[]): SupportHandoffQueueView {
  const queueItems = items.map(projectSupportHandoffItem);
  const openCount = queueItems.filter((item) => item.status === "open" || item.status === "needs_review").length;
  const claimedCount = queueItems.filter((item) => item.status === "claimed").length;
  const closedCount = queueItems.filter((item) => item.status === "closed").length;
  const evidenceRefCount = queueItems.reduce((total, item) => total + item.safeEvidenceRefs.length, 0);
  const status = queueItems.length > 0 ? "ready" : "empty";

  return {
    status,
    openCount,
    claimedCount,
    closedCount,
    evidenceRefCount,
    items: queueItems,
    summaryLines:
      queueItems.length > 0
        ? [
            `${openCount} open handoff(s) need support attention.`,
            `${claimedCount} handoff(s) are already claimed by a support-capable member.`,
            "This queue is local and does not call providers, publish, promote memory, or write graph truth.",
          ]
        : [
            "No daemon-backed handoffs are waiting right now.",
            "Ordo will show first-user relationship and support handoffs here when local evidence exists.",
            "No placeholder customer data is shown as real queue work.",
          ],
    limitations: [
      "Only staff-scoped support queue details are shown here.",
      "Public/member views must not expose staff routing or private support mechanics.",
      "Claim execution remains governed by the protected handoff route and support.accept_handoff policy.",
    ],
  };
}

function projectSupportHandoffItem(item: HandoffInboxItemView): SupportHandoffQueueItemView {
  const status = supportHandoffStatus(item.deliveryState);
  const safeEvidenceRefs = item.evidenceRefs.filter(isSafeEvidenceRef);
  const requestedAction = safeText(item.requestedAction, "Review handoff");
  const nextAction = safeText(item.nextActionHint ?? item.requestedAction, requestedAction);
  const reason = safeText(item.reason, "Support handoff");

  return {
    id: item.id,
    title: reason,
    sourceLabel: supportSourceLabel(item.sourceKind, item.sourceId),
    status,
    statusLabel: supportHandoffStatusLabel(status),
    urgency: safeText(item.urgency, "normal"),
    requestedAction,
    nextAction,
    assigneeLabel: item.assigneeActorId ? "Claimed by support" : "Open for support",
    evidenceRefCount: item.evidenceRefs.length,
    safeEvidenceRefs,
    createdAt: item.createdAt,
    updatedAt: item.updatedAt,
  };
}

function supportHandoffStatus(deliveryState: string): SupportHandoffStatus {
  if (deliveryState === "assigned") {
    return "claimed";
  }
  if (deliveryState === "queued") {
    return "open";
  }
  if (deliveryState === "pending_owner_approval" || deliveryState === "continue_screening") {
    return "needs_review";
  }
  return "closed";
}

export function supportHandoffStatusLabel(status: SupportHandoffStatus): string {
  if (status === "claimed") {
    return "claimed";
  }
  if (status === "closed") {
    return "closed";
  }
  if (status === "needs_review") {
    return "needs review";
  }
  return "open";
}

function supportSourceLabel(sourceKind: string, sourceId: string | null): string {
  const label = sourceKind
    .split("_")
    .filter(Boolean)
    .map((part) => part[0]?.toUpperCase() + part.slice(1))
    .join(" ");
  return sourceId ? `${label || "Source"} ${sourceId}` : label || "Source";
}

function isSafeEvidenceRef(ref: string): boolean {
  return SAFE_EVIDENCE_PREFIXES.some((prefix) => ref.startsWith(prefix)) && !UNSAFE_TEXT_PATTERNS.some((pattern) => pattern.test(ref));
}

function safeText(value: string, fallback: string): string {
  const trimmed = value.trim();
  if (!trimmed || UNSAFE_TEXT_PATTERNS.some((pattern) => pattern.test(trimmed))) {
    return fallback;
  }
  return trimmed;
}
