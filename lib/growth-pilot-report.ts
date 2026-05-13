export const growthReportSourceStatuses = [
  "measured",
  "manual",
  "missing",
  "deferred",
  "unknown",
] as const;

export type GrowthReportSourceStatus =
  (typeof growthReportSourceStatuses)[number];

export interface GrowthPilotEvidenceRef {
  sourceKind: string;
  sourceId: string;
  label: string;
  uri: string;
}

export interface GrowthPilotReportMetric {
  key: string;
  label: string;
  value: number;
  unit: string;
  sourceStatus: GrowthReportSourceStatus;
  evidenceRefs: GrowthPilotEvidenceRef[];
}

export interface GrowthPilotReportItem {
  sourceKind: string;
  sourceId: string;
  label: string;
  status: string;
  sourceStatus: GrowthReportSourceStatus;
  occurredAt: string;
  evidenceRefs: GrowthPilotEvidenceRef[];
}

export interface GrowthPilotReportLimitation {
  key: string;
  label: string;
  detail: string;
  sourceStatus: GrowthReportSourceStatus;
}

export interface GrowthPilotReportSection {
  key: string;
  title: string;
  sourceStatus: GrowthReportSourceStatus;
  metrics: GrowthPilotReportMetric[];
  recentItems: GrowthPilotReportItem[];
  evidenceRefs: GrowthPilotEvidenceRef[];
  limitations: GrowthPilotReportLimitation[];
}

export interface GrowthPilotReportResponse {
  schemaVersion: string;
  generatedAt: string;
  sections: GrowthPilotReportSection[];
  limitations: GrowthPilotReportLimitation[];
}

export type GrowthReportStatusCounts = Record<GrowthReportSourceStatus, number>;

export interface GrowthPilotReportSectionView extends GrowthPilotReportSection {
  metricSummary: string;
  limitationCount: number;
  missingOrDeferredCount: number;
}

export interface GrowthPilotReportView {
  sectionCount: number;
  metricCount: number;
  recentItemCount: number;
  evidenceRefCount: number;
  statusCounts: GrowthReportStatusCounts;
  missingOrDeferredCount: number;
  summaryLines: string[];
  sections: GrowthPilotReportSectionView[];
}

export type GrowthPilotEvidenceAvailability = "available" | "unavailable";

export type GrowthPilotEvidenceReason =
  | "local_owner_admin_ref"
  | "unsupported_scheme"
  | "unsupported_source"
  | "source_mismatch"
  | "empty_ref";

export interface GrowthPilotEvidenceDrilldown {
  key: string;
  label: string;
  sourceKind: string;
  sourceId: string;
  displayRef: string;
  availability: GrowthPilotEvidenceAvailability;
  reason: GrowthPilotEvidenceReason;
  detail: string;
}

export type GrowthPilotLoopCoverage = "covered" | "needs_attention" | "missing";

export interface GrowthPilotLoopBriefItem {
  key: string;
  label: string;
  coverage: GrowthPilotLoopCoverage;
  sourceStatus: GrowthReportSourceStatus;
  metricSummary: string;
  evidenceRefCount: number;
  limitationCount: number;
  detail: string;
}

export interface GrowthPilotReportExportState {
  available: boolean;
  label: string;
  detail: string;
  blockedBy: string | null;
}

export interface GrowthPilotReportBrief {
  title: string;
  generatedAt: string;
  summaryLines: string[];
  limitationLines: string[];
  evidenceDrilldowns: GrowthPilotEvidenceDrilldown[];
  pilotLoop: GrowthPilotLoopBriefItem[];
  exportState: GrowthPilotReportExportState;
}

const growthPilotLoopRequirements = [
  { key: "tracked_entry", label: "Tracked entry and session evidence" },
  { key: "offers", label: "Offer and acceptance evidence" },
  {
    key: "hosted_trials",
    label: "Hosted trial capacity, waitlist, backup, and reset evidence",
  },
  {
    key: "support_handoffs",
    label: "Support handoff and strategy session evidence",
  },
  { key: "feedback", label: "Feedback request and review evidence" },
  { key: "rewards", label: "Reward ledger, benefit, and balance evidence" },
  {
    key: "studio_promos",
    label: "Studio promo package and publication evidence",
  },
] as const;

const allowedGrowthPilotEvidenceSourceKinds = new Set([
  "artifact",
  "artifact_deliverable",
  "benefit_balance",
  "benefit_grant",
  "feedback_request",
  "feedback_response",
  "feedback_reward_eligibility",
  "handoff_inbox_item",
  "hosted_trial_slot",
  "hosted_trial_waitlist_entry",
  "offer",
  "offer_acceptance",
  "reward_event",
  "tracked_entry_point",
  "trial",
  "visitor_session",
]);

export function buildGrowthPilotReportView(
  report: GrowthPilotReportResponse,
): GrowthPilotReportView {
  const statusCounts = emptyGrowthStatusCounts();
  const refsByUri = new Map<string, GrowthPilotEvidenceRef>();
  let metricCount = 0;
  let recentItemCount = 0;

  const sections = report.sections.map((section) => {
    addStatus(statusCounts, section.sourceStatus);
    metricCount += section.metrics.length;
    recentItemCount += section.recentItems.length;
    for (const metric of section.metrics) {
      addStatus(statusCounts, metric.sourceStatus);
      collectEvidenceRefs(refsByUri, metric.evidenceRefs);
    }
    for (const item of section.recentItems) {
      addStatus(statusCounts, item.sourceStatus);
      collectEvidenceRefs(refsByUri, item.evidenceRefs);
    }
    for (const limitation of section.limitations) {
      addStatus(statusCounts, limitation.sourceStatus);
    }
    collectEvidenceRefs(refsByUri, section.evidenceRefs);

    return {
      ...section,
      metricSummary: `${section.metrics.length} metric(s), ${section.recentItems.length} recent item(s), ${section.limitations.length} limitation(s)`,
      limitationCount: section.limitations.length,
      missingOrDeferredCount: countMissingOrDeferred([
        section.sourceStatus,
        ...section.metrics.map((metric) => metric.sourceStatus),
        ...section.recentItems.map((item) => item.sourceStatus),
        ...section.limitations.map((limitation) => limitation.sourceStatus),
      ]),
    };
  });

  for (const limitation of report.limitations) {
    addStatus(statusCounts, limitation.sourceStatus);
  }

  const missingOrDeferredCount = statusCounts.missing + statusCounts.deferred;

  return {
    sectionCount: report.sections.length,
    metricCount,
    recentItemCount,
    evidenceRefCount: refsByUri.size,
    statusCounts,
    missingOrDeferredCount,
    summaryLines: growthReportSummaryLines(
      report.sections.length,
      metricCount,
      recentItemCount,
      missingOrDeferredCount,
    ),
    sections,
  };
}

export function buildGrowthPilotReportBrief(
  report: GrowthPilotReportResponse,
  view = buildGrowthPilotReportView(report),
): GrowthPilotReportBrief {
  const evidenceDrilldowns = collectGrowthPilotEvidenceDrilldowns(report);
  const safeEvidenceRefCount = evidenceDrilldowns.filter(
    (drilldown) => drilldown.availability === "available",
  ).length;
  const pilotLoop = buildGrowthPilotLoopBrief(report);
  const coveredLoopCount = pilotLoop.filter(
    (item) => item.coverage === "covered",
  ).length;

  return {
    title: "Owner Review Brief",
    generatedAt: report.generatedAt,
    summaryLines: [
      `${view.sectionCount} daemon-backed Growth report section(s) are ready for owner review.`,
      `${coveredLoopCount} / ${growthPilotLoopRequirements.length} pilot loop checkpoint(s) have local report sections.`,
      `${safeEvidenceRefCount} safe local evidence reference(s) are available for owner/admin drilldown.`,
      `${view.missingOrDeferredCount} missing or deferred signal(s) remain explicit instead of being inferred as success.`,
    ],
    limitationLines: growthPilotLimitationLines(report),
    evidenceDrilldowns,
    pilotLoop,
    exportState: growthPilotReportExportState(),
  };
}

export function buildGrowthPilotEvidenceDrilldown(
  ref: GrowthPilotEvidenceRef,
): GrowthPilotEvidenceDrilldown {
  const label = safeEvidenceLabel(ref.label, ref.sourceKind, ref.sourceId);
  const sourceKind = safeRefSegment(ref.sourceKind);
  const sourceId = safeRefSegment(ref.sourceId);
  const key = `${sourceKind}:${sourceId}`;

  if (!ref.uri || !ref.sourceKind || !ref.sourceId) {
    return {
      key,
      label,
      sourceKind,
      sourceId,
      displayRef: "empty evidence ref withheld",
      availability: "unavailable",
      reason: "empty_ref",
      detail:
        "Evidence reference is incomplete, so the owner report keeps it unavailable instead of linking to uncertain data.",
    };
  }

  if (!allowedGrowthPilotEvidenceSourceKinds.has(ref.sourceKind)) {
    return unavailableEvidenceDrilldown(
      "unsupported_source",
      "Unsupported evidence ref",
      "withheld",
      "withheld",
      "unsupported_source",
      "unsupported local evidence ref withheld",
    );
  }

  let parsed: URL;
  try {
    parsed = new URL(ref.uri);
  } catch {
    return unavailableEvidenceDrilldown(
      key,
      label,
      sourceKind,
      sourceId,
      "unsupported_scheme",
      "external evidence ref withheld",
    );
  }

  if (parsed.protocol !== "ordo:") {
    return unavailableEvidenceDrilldown(
      key,
      label,
      sourceKind,
      sourceId,
      "unsupported_scheme",
      "external evidence ref withheld",
    );
  }

  const uriSourceKind = decodeURIComponent(parsed.hostname);
  const uriSourceId = decodeURIComponent(parsed.pathname.replace(/^\/+/, ""));
  if (uriSourceKind !== ref.sourceKind || uriSourceId !== ref.sourceId) {
    return unavailableEvidenceDrilldown(
      key,
      label,
      sourceKind,
      sourceId,
      "source_mismatch",
      "mismatched local evidence ref withheld",
    );
  }

  const displayRef = `ordo://${safeRefSegment(uriSourceKind)}/${safeRefSegment(uriSourceId)}`;
  return {
    key,
    label,
    sourceKind,
    sourceId,
    displayRef,
    availability: "available",
    reason: "local_owner_admin_ref",
    detail: `Local ${sourceKind} evidence ref ${sourceId} is available for owner/admin drilldown.`,
  };
}

export function growthSourceStatusLabel(
  status: GrowthReportSourceStatus,
): string {
  return status.replaceAll("_", " ");
}

export function growthSourceStatusTone(
  status: GrowthReportSourceStatus,
): "ok" | "warn" | "error" {
  if (status === "measured") {
    return "ok";
  }
  if (status === "manual" || status === "deferred") {
    return "warn";
  }
  return "error";
}

export function growthReportStatusClassToken(
  status: GrowthReportSourceStatus,
): string {
  return growthSourceStatusTone(status);
}

function growthReportSummaryLines(
  sectionCount: number,
  metricCount: number,
  recentItemCount: number,
  missingOrDeferredCount: number,
): string[] {
  if (sectionCount === 0) {
    return ["No Growth report sections are available yet."];
  }

  return [
    `${sectionCount} Growth report section(s) are backed by the daemon snapshot.`,
    `${metricCount} metric(s) and ${recentItemCount} recent evidence item(s) are available for owner review.`,
    `${missingOrDeferredCount} missing or deferred signal(s) remain explicit instead of being treated as success metrics.`,
  ];
}

function buildGrowthPilotLoopBrief(
  report: GrowthPilotReportResponse,
): GrowthPilotLoopBriefItem[] {
  const sectionsByKey = new Map(
    report.sections.map((section) => [section.key, section]),
  );

  return growthPilotLoopRequirements.map((requirement) => {
    const section = sectionsByKey.get(requirement.key);
    if (!section) {
      return {
        key: requirement.key,
        label: requirement.label,
        coverage: "missing",
        sourceStatus: "missing",
        metricSummary: "0 metric(s), 0 recent item(s), 0 limitation(s)",
        evidenceRefCount: 0,
        limitationCount: 0,
        detail:
          "No local report section exists yet; do not infer completion for this pilot checkpoint.",
      };
    }

    const safeEvidenceRefCount = collectSectionEvidenceDrilldowns(
      section,
    ).filter((drilldown) => drilldown.availability === "available").length;
    return {
      key: requirement.key,
      label: requirement.label,
      coverage: coverageForStatus(section.sourceStatus),
      sourceStatus: section.sourceStatus,
      metricSummary: `${section.metrics.length} metric(s), ${section.recentItems.length} recent item(s), ${section.limitations.length} limitation(s)`,
      evidenceRefCount: safeEvidenceRefCount,
      limitationCount: section.limitations.length,
      detail: `${section.title} is represented by daemon report data with ${safeEvidenceRefCount} safe local evidence ref(s).`,
    };
  });
}

function collectGrowthPilotEvidenceDrilldowns(
  report: GrowthPilotReportResponse,
): GrowthPilotEvidenceDrilldown[] {
  const drilldownsByKey = new Map<string, GrowthPilotEvidenceDrilldown>();
  for (const section of report.sections) {
    for (const drilldown of collectSectionEvidenceDrilldowns(section)) {
      setBestEvidenceDrilldown(drilldownsByKey, drilldown);
    }
  }
  return Array.from(drilldownsByKey.values()).sort((left, right) =>
    left.key.localeCompare(right.key),
  );
}

function collectSectionEvidenceDrilldowns(
  section: GrowthPilotReportSection,
): GrowthPilotEvidenceDrilldown[] {
  const refs = [
    ...section.evidenceRefs,
    ...section.metrics.flatMap((metric) => metric.evidenceRefs),
    ...section.recentItems.flatMap((item) => item.evidenceRefs),
  ];
  const drilldownsByKey = new Map<string, GrowthPilotEvidenceDrilldown>();
  for (const ref of refs) {
    const drilldown = buildGrowthPilotEvidenceDrilldown(ref);
    setBestEvidenceDrilldown(drilldownsByKey, drilldown);
  }
  return Array.from(drilldownsByKey.values());
}

function setBestEvidenceDrilldown(
  drilldownsByKey: Map<string, GrowthPilotEvidenceDrilldown>,
  drilldown: GrowthPilotEvidenceDrilldown,
) {
  const existing = drilldownsByKey.get(drilldown.key);
  if (
    !existing ||
    (existing.availability === "unavailable" &&
      drilldown.availability === "available")
  ) {
    drilldownsByKey.set(drilldown.key, drilldown);
  }
}

function coverageForStatus(
  status: GrowthReportSourceStatus,
): GrowthPilotLoopCoverage {
  if (status === "measured" || status === "manual") {
    return "covered";
  }
  if (status === "deferred") {
    return "needs_attention";
  }
  return "missing";
}

function growthPilotLimitationLines(
  report: GrowthPilotReportResponse,
): string[] {
  const limitationLines = [
    "No live platform analytics, external publishing, payments, OAuth, provider behavior, uptime, or AI-authored claims are inferred.",
  ];
  for (const limitation of report.limitations) {
    limitationLines.push(
      `${redactSensitiveText(limitation.label)}: ${redactSensitiveText(limitation.detail)}`,
    );
  }
  for (const section of report.sections) {
    for (const limitation of section.limitations) {
      limitationLines.push(
        `${redactSensitiveText(limitation.label)}: ${redactSensitiveText(limitation.detail)}`,
      );
    }
  }
  return Array.from(new Set(limitationLines));
}

function growthPilotReportExportState(): GrowthPilotReportExportState {
  return {
    available: false,
    label: "Local report package export unavailable",
    detail:
      "Deterministic report-package export is not implemented for the owner Growth report route yet; use the on-screen brief and local evidence refs.",
    blockedBy: "deterministic_export_package",
  };
}

function unavailableEvidenceDrilldown(
  key: string,
  label: string,
  sourceKind: string,
  sourceId: string,
  reason: GrowthPilotEvidenceReason,
  displayRef: string,
): GrowthPilotEvidenceDrilldown {
  return {
    key,
    label,
    sourceKind,
    sourceId,
    displayRef,
    availability: "unavailable",
    reason,
    detail:
      "Evidence reference is not a safe matching local Ordo ref, so it is withheld from owner drilldown.",
  };
}

function emptyGrowthStatusCounts(): GrowthReportStatusCounts {
  return {
    measured: 0,
    manual: 0,
    missing: 0,
    deferred: 0,
    unknown: 0,
  };
}

function addStatus(
  counts: GrowthReportStatusCounts,
  status: GrowthReportSourceStatus,
) {
  counts[status] += 1;
}

function collectEvidenceRefs(
  refsByUri: Map<string, GrowthPilotEvidenceRef>,
  refs: readonly GrowthPilotEvidenceRef[],
) {
  for (const ref of refs) {
    if (!refsByUri.has(ref.uri)) {
      refsByUri.set(ref.uri, ref);
    }
  }
}

function countMissingOrDeferred(statuses: GrowthReportSourceStatus[]): number {
  return statuses.filter(
    (status) => status === "missing" || status === "deferred",
  ).length;
}

function safeEvidenceLabel(
  label: string,
  sourceKind: string,
  sourceId: string,
): string {
  const fallback =
    `${safeRefSegment(sourceKind)} ${safeRefSegment(sourceId)}`.trim();
  const safeLabel = redactSensitiveText(
    label.trim() || fallback || "Local evidence ref",
  );
  return safeLabel.slice(0, 160);
}

function safeRefSegment(value: string): string {
  const safe = redactSensitiveText(value).replace(/[^A-Za-z0-9_.:-]/g, "_");
  return (safe || "unknown").slice(0, 96);
}

function redactSensitiveText(value: string): string {
  return value
    .replace(/sk[-_][A-Za-z0-9_-]+/g, "[redacted]")
    .replace(/rawPrompt/gi, "[redacted]");
}
