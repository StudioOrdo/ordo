import { expect, test } from "@playwright/test";

import {
  buildStudioWorkSnapshot,
  deferredStudioActions,
  studioViewerForRole,
  type StudioSurfaceWorkItem,
} from "@/lib/studio-work";

const baseItem = {
  surfaceKind: "studio",
  sourceId: "source_1",
  objectId: "object_1",
  priority: 50,
  actorContext: { rawPrompt: "do not render" },
  connectionContext: { providerPayload: "sk_live_hidden" },
  visibility: "staff",
  createdAt: "2026-05-13T12:00:00.000Z",
  updatedAt: "2026-05-13T12:00:00.000Z",
  projectedAt: "2026-05-13T12:00:01.000Z",
} satisfies Partial<StudioSurfaceWorkItem>;

test.describe("Studio work view model", () => {
  test("groups durable Studio runs and artifacts without leaking contexts", () => {
    const snapshot = buildStudioWorkSnapshot([
      {
        ...baseItem,
        id: "studio_run_1",
        roomKind: "runs",
        sourceKind: "job",
        objectKind: "job",
        title: "Job: studio.video.make",
        summary: "Job from conversation brief is running.",
        status: "running",
        evidenceRefs: ["job:job_smoke_video", "brief:brief_promo"],
        actions: ["inspect_job"],
      },
      {
        ...baseItem,
        id: "studio_artifact_1",
        roomKind: "artifacts",
        sourceKind: "artifact",
        objectKind: "artifact",
        title: "Candidate 30 second promo video",
        summary: "Review candidate package before staging.",
        status: "candidate",
        evidenceRefs: ["artifact:artifact_promo_smoke"],
        actions: ["review_artifact"],
      },
    ]);

    expect(snapshot.totalItems).toBe(2);
    expect(snapshot.runs.items.map((item) => item.id)).toEqual(["studio_run_1"]);
    expect(snapshot.artifacts.items.map((item) => item.id)).toEqual(["studio_artifact_1"]);
    expect(snapshot.statusCounts).toEqual({ candidate: 1, running: 1 });
    expect(snapshot.backedActionLabels).toEqual(["Inspect job", "Review artifact"]);
    expect(JSON.stringify(snapshot)).not.toContain("rawPrompt");
    expect(JSON.stringify(snapshot)).not.toContain("sk_live_hidden");
  });

  test("maps product roles to safe Studio surface viewers", () => {
    expect(studioViewerForRole("studio")).toBe("staff");
    expect(studioViewerForRole("manager")).toBe("staff");
    expect(studioViewerForRole("owner")).toBe("owner");
    expect(studioViewerForRole("admin")).toBe("owner");
    expect(studioViewerForRole("member")).toBeNull();
    expect(studioViewerForRole("anonymous")).toBeNull();
  });

  test("marks unsupported production actions as deferred unless backed", () => {
    expect(deferredStudioActions(["inspect_job", "review_artifact"]).map((action) => action.key)).toEqual([
      "approve_artifact",
      "request_revision",
      "stage_output",
      "generate_media",
      "publish_external",
    ]);
    expect(deferredStudioActions(["approve_artifact"]).map((action) => action.key)).not.toContain("approve_artifact");
  });

  test("keeps empty and non-Studio snapshots explicit", () => {
    const emptySnapshot = buildStudioWorkSnapshot([]);
    expect(emptySnapshot.totalItems).toBe(0);
    expect(emptySnapshot.runs.emptyLabel).toBe("No durable production runs are available.");
    expect(emptySnapshot.artifacts.emptyLabel).toBe("No durable artifacts are available.");
    expect(emptySnapshot.backedActionLabels).toEqual([]);

    const ignoredSnapshot = buildStudioWorkSnapshot([
      {
        ...baseItem,
        id: "support_handoff_1",
        surfaceKind: "support",
        roomKind: "handoffs",
        sourceKind: "handoff",
        objectKind: "handoff",
        title: "Support handoff",
        summary: "This should not render inside Studio.",
        status: "open",
        evidenceRefs: ["handoff:handoff_1"],
        actions: ["claim_handoff"],
      },
    ]);
    expect(ignoredSnapshot.totalItems).toBe(0);
    expect(JSON.stringify(ignoredSnapshot)).not.toContain("Support handoff");
  });
});
