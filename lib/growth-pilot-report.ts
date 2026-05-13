export const growthReportSourceStatuses = ["measured", "manual", "missing", "deferred", "unknown"] as const;

export type GrowthReportSourceStatus = (typeof growthReportSourceStatuses)[number];

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

export function buildGrowthPilotReportView(report: GrowthPilotReportResponse): GrowthPilotReportView {
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
    summaryLines: growthReportSummaryLines(report.sections.length, metricCount, recentItemCount, missingOrDeferredCount),
    sections,
  };
}

export function growthSourceStatusLabel(status: GrowthReportSourceStatus): string {
  return status.replaceAll("_", " ");
}

export function growthSourceStatusTone(status: GrowthReportSourceStatus): "ok" | "warn" | "error" {
  if (status === "measured") {
    return "ok";
  }
  if (status === "manual" || status === "deferred") {
    return "warn";
  }
  return "error";
}

export function growthReportStatusClassToken(status: GrowthReportSourceStatus): string {
  return growthSourceStatusTone(status);
}

function growthReportSummaryLines(sectionCount: number, metricCount: number, recentItemCount: number, missingOrDeferredCount: number): string[] {
  if (sectionCount === 0) {
    return ["No Growth report sections are available yet."];
  }

  return [
    `${sectionCount} Growth report section(s) are backed by the daemon snapshot.`,
    `${metricCount} metric(s) and ${recentItemCount} recent evidence item(s) are available for owner review.`,
    `${missingOrDeferredCount} missing or deferred signal(s) remain explicit instead of being treated as success metrics.`,
  ];
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

function addStatus(counts: GrowthReportStatusCounts, status: GrowthReportSourceStatus) {
  counts[status] += 1;
}

function collectEvidenceRefs(refsByUri: Map<string, GrowthPilotEvidenceRef>, refs: readonly GrowthPilotEvidenceRef[]) {
  for (const ref of refs) {
    if (!refsByUri.has(ref.uri)) {
      refsByUri.set(ref.uri, ref);
    }
  }
}

function countMissingOrDeferred(statuses: GrowthReportSourceStatus[]): number {
  return statuses.filter((status) => status === "missing" || status === "deferred").length;
}
