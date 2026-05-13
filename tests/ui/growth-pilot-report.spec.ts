import { expect, test } from "@playwright/test";

import {
  buildGrowthPilotEvidenceDrilldown,
  buildGrowthPilotReportBrief,
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

  test("keeps empty reports explicit instead of inventing success metrics", () => {
    const view = buildGrowthPilotReportView({
      schemaVersion: "ordo.growth_pilot_report.v1",
      generatedAt: "2026-05-13T18:00:00.000Z",
      sections: [],
      limitations: [],
    });

    expect(view.sectionCount).toBe(0);
    expect(view.metricCount).toBe(0);
    expect(view.recentItemCount).toBe(0);
    expect(view.evidenceRefCount).toBe(0);
    expect(view.missingOrDeferredCount).toBe(0);
    expect(view.statusCounts).toEqual({
      measured: 0,
      manual: 0,
      missing: 0,
      deferred: 0,
      unknown: 0,
    });
    expect(view.summaryLines).toEqual(["No Growth report sections are available yet."]);
  });

  test("builds a deterministic owner-review brief with explicit pilot-loop gaps and export deferral", () => {
    const brief = buildGrowthPilotReportBrief(growthReportFixture());

    expect(brief.title).toBe("Owner Review Brief");
    expect(brief.summaryLines).toEqual([
      "2 daemon-backed Growth report section(s) are ready for owner review.",
      "2 / 7 pilot loop checkpoint(s) have local report sections.",
      "2 safe local evidence reference(s) are available for owner/admin drilldown.",
      "5 missing or deferred signal(s) remain explicit instead of being inferred as success.",
    ]);
    expect(brief.pilotLoop.map((item) => `${item.key}:${item.coverage}`)).toEqual([
      "tracked_entry:covered",
      "offers:missing",
      "hosted_trials:missing",
      "support_handoffs:missing",
      "feedback:missing",
      "rewards:missing",
      "studio_promos:covered",
    ]);
    expect(brief.limitationLines).toContain("External publishing is deferred: No platform publishing API is called by this report.");
    expect(brief.exportState).toEqual({
      available: false,
      label: "Local report package export unavailable",
      detail:
        "Deterministic report-package export is not implemented for the owner Growth report route yet; use the on-screen brief and local evidence refs.",
      blockedBy: "deterministic_export_package",
    });
    expect(JSON.stringify(brief)).not.toContain("sk_live");
    expect(JSON.stringify(brief)).not.toContain("rawPrompt");
  });

  test("normalizes safe local evidence refs and withholds unsafe or mismatched refs", () => {
    const safe = buildGrowthPilotEvidenceDrilldown({
      sourceKind: "visitor_session",
      sourceId: "visitor_smoke_1",
      label: "Visitor session visitor_smoke_1",
      uri: "ordo://visitor_session/visitor_smoke_1",
    });
    expect(safe).toMatchObject({
      availability: "available",
      displayRef: "ordo://visitor_session/visitor_smoke_1",
      reason: "local_owner_admin_ref",
    });

    const external = buildGrowthPilotEvidenceDrilldown({
      sourceKind: "visitor_session",
      sourceId: "visitor_smoke_1",
      label: "Visitor session sk_live_hidden",
      uri: "https://analytics.example/internal?token=sk_live_hidden",
    });
    expect(external).toMatchObject({
      availability: "unavailable",
      displayRef: "external evidence ref withheld",
      reason: "unsupported_scheme",
    });
    expect(JSON.stringify(external)).not.toContain("https://analytics.example");
    expect(JSON.stringify(external)).not.toContain("sk_live_hidden");

    const mismatch = buildGrowthPilotEvidenceDrilldown({
      sourceKind: "offer",
      sourceId: "offer_smoke_1",
      label: "Offer offer_smoke_1",
      uri: "ordo://trial/trial_smoke_active",
    });
    expect(mismatch).toMatchObject({
      availability: "unavailable",
      displayRef: "mismatched local evidence ref withheld",
      reason: "source_mismatch",
    });

    const internal = buildGrowthPilotEvidenceDrilldown({
      sourceKind: "staff_routing_details",
      sourceId: "route_secret_1",
      label: "Staff route sk_live_hidden",
      uri: "ordo://staff_routing_details/route_secret_1",
    });
    expect(internal).toMatchObject({
      availability: "unavailable",
      displayRef: "unsupported local evidence ref withheld",
      reason: "unsupported_source",
      label: "Unsupported evidence ref",
      sourceKind: "withheld",
      sourceId: "withheld",
    });
    expect(JSON.stringify(internal)).not.toContain("staff_routing_details");
    expect(JSON.stringify(internal)).not.toContain("route_secret_1");
    expect(JSON.stringify(internal)).not.toContain("sk_live_hidden");
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
