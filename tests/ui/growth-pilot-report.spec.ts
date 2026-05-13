import { expect, test } from "@playwright/test";

import {
  buildGrowthPilotReportView,
  growthSourceStatusLabel,
  growthSourceStatusTone,
  type GrowthPilotReportResponse,
} from "@/lib/growth-pilot-report";

test.describe("Growth pilot report view model", () => {
  test("summarizes daemon report data without leaking raw internals", () => {
    const view = buildGrowthPilotReportView(growthReportFixture());

    expect(view.sectionCount).toBe(2);
    expect(view.metricCount).toBe(4);
    expect(view.recentItemCount).toBe(2);
    expect(view.evidenceRefCount).toBe(2);
    expect(view.statusCounts).toEqual({
      measured: 4,
      manual: 2,
      missing: 3,
      deferred: 2,
      unknown: 0,
    });
    expect(view.summaryLines).toEqual([
      "2 Growth report section(s) are backed by the daemon snapshot.",
      "4 metric(s) and 2 recent evidence item(s) are available for owner review.",
      "5 missing or deferred signal(s) remain explicit instead of being treated as success metrics.",
    ]);
    expect(view.sections[0]?.metricSummary).toBe("2 metric(s), 1 recent item(s), 1 limitation(s)");
    expect(JSON.stringify(view)).not.toContain("rawPrompt");
    expect(JSON.stringify(view)).not.toContain("sk_live");
  });

  test("keeps source status labels and tones stable", () => {
    expect(growthSourceStatusLabel("measured")).toBe("measured");
    expect(growthSourceStatusLabel("manual")).toBe("manual");
    expect(growthSourceStatusLabel("missing")).toBe("missing");
    expect(growthSourceStatusLabel("deferred")).toBe("deferred");
    expect(growthSourceStatusLabel("unknown")).toBe("unknown");
    expect(growthSourceStatusTone("measured")).toBe("ok");
    expect(growthSourceStatusTone("manual")).toBe("warn");
    expect(growthSourceStatusTone("missing")).toBe("error");
    expect(growthSourceStatusTone("deferred")).toBe("warn");
    expect(growthSourceStatusTone("unknown")).toBe("error");
  });
});

function growthReportFixture(): GrowthPilotReportResponse {
  return {
    schemaVersion: "ordo.growth_pilot_report.v1",
    generatedAt: "2026-05-13T18:00:00.000Z",
    limitations: [
      {
        key: "external_publishing_deferred",
        label: "External publishing is deferred",
        detail: "No platform publishing API is called by this report.",
        sourceStatus: "deferred",
      },
    ],
    sections: [
      {
        key: "tracked_entry",
        title: "Tracked Entry And Sessions",
        sourceStatus: "measured",
        metrics: [
          {
            key: "visitor_sessions",
            label: "Visitor sessions",
            value: 4,
            unit: "sessions",
            sourceStatus: "measured",
            evidenceRefs: [
              {
                sourceKind: "visitor_session",
                sourceId: "visitor_smoke_1",
                label: "Visitor session visitor_smoke_1",
                uri: "ordo://visitor_session/visitor_smoke_1",
              },
            ],
          },
          {
            key: "platform_performance_metrics",
            label: "Platform performance metrics",
            value: 0,
            unit: "metrics",
            sourceStatus: "missing",
            evidenceRefs: [],
          },
        ],
        recentItems: [
          {
            sourceKind: "visitor_session",
            sourceId: "visitor_smoke_1",
            label: "Visitor session visitor_smoke_1",
            status: "active",
            sourceStatus: "measured",
            occurredAt: "2026-05-13T17:00:00.000Z",
            evidenceRefs: [
              {
                sourceKind: "visitor_session",
                sourceId: "visitor_smoke_1",
                label: "Visitor session visitor_smoke_1",
                uri: "ordo://visitor_session/visitor_smoke_1",
              },
            ],
          },
        ],
        evidenceRefs: [
          {
            sourceKind: "visitor_session",
            sourceId: "visitor_smoke_1",
            label: "Visitor session visitor_smoke_1",
            uri: "ordo://visitor_session/visitor_smoke_1",
          },
        ],
        limitations: [
          {
            key: "offer_view_events_missing",
            label: "Individual offer views are not tracked yet",
            detail: "Per-offer view events are not durable yet.",
            sourceStatus: "missing",
          },
        ],
      },
      {
        key: "studio_promos",
        title: "Studio Promo Packages And Publication Evidence",
        sourceStatus: "manual",
        metrics: [
          {
            key: "staged_manual_packages",
            label: "Staged manual promo packages",
            value: 1,
            unit: "packages",
            sourceStatus: "manual",
            evidenceRefs: [
              {
                sourceKind: "artifact",
                sourceId: "artifact_promo_smoke",
                label: "Promo package artifact artifact_promo_smoke",
                uri: "ordo://artifact/artifact_promo_smoke",
              },
            ],
          },
          {
            key: "external_publications",
            label: "External platform publications",
            value: 0,
            unit: "publications",
            sourceStatus: "deferred",
            evidenceRefs: [],
          },
        ],
        recentItems: [
          {
            sourceKind: "artifact",
            sourceId: "artifact_promo_smoke",
            label: "Promo package artifact artifact_promo_smoke",
            status: "staged_manual",
            sourceStatus: "measured",
            occurredAt: "2026-05-13T16:00:00.000Z",
            evidenceRefs: [
              {
                sourceKind: "artifact",
                sourceId: "artifact_promo_smoke",
                label: "Promo package artifact artifact_promo_smoke",
                uri: "ordo://artifact/artifact_promo_smoke",
              },
            ],
          },
        ],
        evidenceRefs: [
          {
            sourceKind: "artifact",
            sourceId: "artifact_promo_smoke",
            label: "Promo package artifact artifact_promo_smoke",
            uri: "ordo://artifact/artifact_promo_smoke",
          },
        ],
        limitations: [
          {
            key: "platform_analytics_missing",
            label: "Platform analytics are missing",
            detail: "Views, watch time, and conversions need a future governed integration.",
            sourceStatus: "missing",
          },
        ],
      },
    ],
  };
}
