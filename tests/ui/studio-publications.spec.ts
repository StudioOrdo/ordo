import { expect, test } from "@playwright/test";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";

import {
  buildStudioPublicationsView,
  studioPublicationStatusTone,
  type StudioProductionReviewPacket,
  type StoryPublishLearningBrief,
} from "@/lib/studio-publications";

const daemonPort = 19080;

interface MockDaemonState {
  requests: string[];
  mode: "ready" | "partial";
}

test.describe("Studio Publications view model", () => {
  test("summarizes production review and learning evidence without leaking internals", () => {
    const view = buildStudioPublicationsView(storyReviewFixture(), storyLearningFixture());

    expect(view.status).toBe("manual");
    expect(view.componentCount).toBe(3);
    expect(view.metricCount).toBe(3);
    expect(view.publishEvidenceCount).toBe(2);
    expect(view.safeEvidenceRefCount).toBe(7);
    expect(view.memoryCandidateCount).toBe(2);
    expect(view.missingOrDeferredCount).toBe(4);
    expect(view.sourceStatusCounts).toEqual({
      measured: 2,
      manual: 3,
      missing: 2,
      deferred: 2,
      unknown: 0,
    });
    expect(view.summaryLines).toEqual([
      "3 Story production component(s) are represented by daemon review evidence.",
      "3 learning metric(s) and 2 publication evidence source(s) are available for owner/staff review.",
      "4 missing or deferred signal(s) remain explicit instead of being treated as publication success.",
    ]);
    expect(view.deferredStates.map((state) => state.key)).toContain("external_publishing");
    expect(view.deferredStates.map((state) => state.key)).toContain("memory_promotion");
    expect(view.components[0]?.limitations).toContain("External publishing not claimed");
    expect(view.reviewLimitations).toContain("Story production review is read only");
    expect(view.learningLimitations).toContain("External analytics missing");
    expect(view.limitations).toContain("Story production review is read only");
    expect(view.limitations).toContain("External analytics missing");
    expect(JSON.stringify(view)).not.toContain("rawPrompt");
    expect(JSON.stringify(view)).not.toContain("sk_live");
    expect(JSON.stringify(view)).not.toContain("provider internal");
    expect(JSON.stringify(view)).not.toContain("prompt internal");
    expect(JSON.stringify(view)).not.toContain("private artifact text");
    expect(JSON.stringify(view)).not.toContain("generated-content candidate text");
    expect(JSON.stringify(view)).not.toContain("graph certainty");
  });

  test("keeps missing daemon evidence explicit instead of inventing publication success", () => {
    const review = {
      ...storyReviewFixture(),
      status: "partial",
      components: [],
      evidenceRefs: [],
      missingPrerequisites: ["story_homepage_publish_approval_package"],
      recommendedNextActions: ["create_story_publish_approval_package"],
    };
    const learning = {
      ...storyLearningFixture(),
      status: "missing",
      sourceStatus: [],
      contentMetrics: [],
      publishEvidence: [],
      evidenceRefs: [],
      limitations: ["content_analytics_missing", "manual_publish_evidence_missing"],
      recommendedNextActions: ["record_manual_publish_evidence"],
    };

    const view = buildStudioPublicationsView(review, learning);

    expect(view.status).toBe("missing");
    expect(view.componentCount).toBe(0);
    expect(view.metricCount).toBe(0);
    expect(view.publishEvidenceCount).toBe(0);
    expect(view.summaryLines).toEqual([
      "No daemon-backed Story production review components are available yet.",
      "No Story publish learning metrics are available yet.",
      "Missing or deferred publication evidence remains explicit.",
    ]);
    expect(view.nextActions).toEqual([
      "Create story publish approval package",
      "Record manual publish evidence",
    ]);
  });

  test("keeps source status tones stable", () => {
    expect(studioPublicationStatusTone("measured")).toBe("ok");
    expect(studioPublicationStatusTone("manual")).toBe("warn");
    expect(studioPublicationStatusTone("missing")).toBe("error");
    expect(studioPublicationStatusTone("deferred")).toBe("warn");
    expect(studioPublicationStatusTone("fixture")).toBe("warn");
    expect(studioPublicationStatusTone("unknown")).toBe("error");
  });
});

test.describe.configure({ mode: "serial" });

test.afterEach(async ({ page }) => {
  await page.close();
});

test("Studio Publications renders Story review and learning evidence from protected daemon routes", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon("ready");
  try {
    await page.goto(productContentUrl("/studio/publications?role=studio", testInfo));

    await expect(page.locator("main").getByRole("heading", { name: "Publications" })).toBeVisible();
    await expect(page.locator("main")).toContainText("Story Publication Readiness");
    await expect(page.locator("main")).toContainText("Narrative deck review package");
    await expect(page.locator("main")).toContainText("Homepage publish approval");
    await expect(page.locator("main")).toContainText("Story Publish Learning");
    await expect(page.locator("main")).toContainText("Manual publish evidence");
    await expect(page.locator("main")).toContainText("External analytics missing");
    await expect(page.locator("main")).toContainText("External publishing not claimed");
    await expect(page.locator("main")).toContainText("Story production review is read only");
    await expect(page.locator("main")).toContainText("Memory promotion not performed");
    await expect(page.locator("main")).not.toContainText("rawPrompt");
    await expect(page.locator("main")).not.toContainText("sk_live");
    await expect(page.locator("main")).not.toContainText("provider internal");
    await expect(page.locator("main")).not.toContainText("prompt internal");
    await expect(page.locator("main")).not.toContainText("private artifact text");
    await expect(page.locator("main")).not.toContainText("generated-content candidate text");
    await expect(page.locator("main")).not.toContainText("graph certainty");
    expect(daemon.state.requests.some((request) => request.startsWith("GET /studio/story-production-review?"))).toBe(true);
    expect(daemon.state.requests.some((request) => request.startsWith("GET /studio/story-publish-learning?"))).toBe(true);
  } finally {
    await daemon.close();
  }
});

test("Studio Publications refuses member role before daemon reads", async ({ page }) => {
  const daemon = await startMockDaemon("ready");
  try {
    await page.goto("/studio/publications?role=member");

    await expect(page.locator("body")).not.toContainText("Story Publication Readiness");
    await expect(page.locator("body")).not.toContainText("Narrative deck review package");
    expect(daemon.state.requests).toEqual([]);
  } finally {
    await daemon.close();
  }
});

test("Studio Publications keeps daemon-degraded state explicit", async ({ page }, testInfo) => {
  await page.goto(productContentUrl("/studio/publications?role=studio", testInfo));

  await expect(page.locator("main").getByRole("heading", { name: "Publications" })).toBeVisible();
  await expect(page.locator("main")).toContainText("degraded");
  await expect(page.locator("main")).toContainText("Studio Publications evidence is degraded because daemon Story routes are unavailable.");
  await expect(page.locator("main")).toContainText("/studio/story-production-review");
  await expect(page.locator("main")).toContainText("/studio/story-publish-learning");
});

async function startMockDaemon(mode: MockDaemonState["mode"]): Promise<{ state: MockDaemonState; close: () => Promise<void> }> {
  const state: MockDaemonState = { requests: [], mode };
  const server = createServer((request, response) => handleRequest(request, response, state));
  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(daemonPort, "127.0.0.1", () => {
      server.off("error", reject);
      resolve();
    });
  });
  return {
    state,
    close: () => closeServer(server),
  };
}

function closeServer(server: Server): Promise<void> {
  return new Promise((resolve, reject) => {
    server.close((error) => (error ? reject(error) : resolve()));
  });
}

function handleRequest(request: IncomingMessage, response: ServerResponse, state: MockDaemonState) {
  const method = request.method ?? "GET";
  const path = request.url ?? "/";
  state.requests.push(`${method} ${path}`);

  if (method === "GET" && path.startsWith("/studio/story-production-review")) {
    return jsonResponse(response, storyReviewFixture());
  }

  if (method === "GET" && path.startsWith("/studio/story-publish-learning")) {
    return jsonResponse(response, storyLearningFixture());
  }

  if (method === "GET" && path.startsWith("/studio/generated-content-memory/")) {
    return jsonResponse(response, {
      schemaVersion: "ordo.generated_content_memory_review_packet.v1",
      artifactId: path.split("/studio/generated-content-memory/")[1]?.split("/review")[0] ?? "unknown",
      sourceArtifactKind: "story_publication_artifact",
      audience: "staff",
      candidateCount: 0,
      sourceArtifactRefs: [],
      workflowRefs: [],
      evidenceRefs: [],
      limitations: ["no_generated_content_memory_candidates"],
      items: [],
      extensionPoints: ["manual_owner_review"],
      confirmedGraphPromotion: false,
      liveProviderCalled: false,
    });
  }

  response.writeHead(404, { "content-type": "application/json" });
  response.end(JSON.stringify({ error: `Unhandled mock daemon route: ${method} ${path}` }));
}

function jsonResponse(response: ServerResponse, body: unknown) {
  response.writeHead(200, { "content-type": "application/json" });
  response.end(JSON.stringify(body));
}

function productContentUrl(path: string, testInfo: { project: { name: string } }): string {
  return testInfo.project.name === "mobile-chromium" ? `${path}${path.includes("?") ? "&" : "?"}mobile=content` : path;
}

function storyReviewFixture(): StudioProductionReviewPacket {
  return {
    schemaVersion: "ordo.story_production_review_packet.v1",
    status: "partial",
    audience: "staff",
    readOnly: true,
    mutationPerformed: false,
    confirmedGraphPromotion: false,
    liveProviderCalled: false,
    externalPublishingClaimed: false,
    deckId: "homepage.story.v1",
    evidenceRefs: ["artifact:story_deck", "artifact:image_review", "content_analytics:homepage.story.v1"],
    limitations: [
      "story_production_review_is_read_only",
      "external_publishing_not_claimed",
      "rawPrompt should not render",
    ],
    missingPrerequisites: ["external_publish_evidence", "platform_analytics"],
    recommendedNextActions: ["collect_manual_publish_evidence", "review_memory_candidates"],
    components: [
      {
        key: "narrative_deck",
        status: "ready",
        artifactRef: "artifact:story_deck",
        artifactKind: "story_narrative_deck",
        title: "Narrative deck review package",
        summary: "Deck is ready for owner publication review.",
        visibility: "owner",
        evidenceStatus: "manual",
        evidenceRefs: ["artifact:story_deck"],
        limitations: ["external_publishing_not_claimed"],
        recommendedNextAction: "approve_manual_publish_package",
      },
      {
        key: "image_review",
        status: "needs_review",
        artifactRef: "artifact:image_review",
        artifactKind: "story_image_review",
        title: "Homepage image review",
        summary: "Image variants need final owner choice.",
        visibility: "owner",
        evidenceStatus: "missing",
        evidenceRefs: ["artifact:image_review"],
        limitations: ["private artifact text should not render"],
        recommendedNextAction: "select_image_variant",
      },
      {
        key: "homepage_publish_approval",
        status: "staged",
        artifactRef: "artifact:publish_package",
        artifactKind: "story_homepage_publish_approval_package",
        title: "Homepage publish approval",
        summary: "Manual local publish package is staged.",
        visibility: "owner",
        evidenceStatus: "measured",
        evidenceRefs: ["artifact:publish_package"],
        limitations: [],
        recommendedNextAction: "record_manual_publish_evidence",
      },
    ],
    analyticsSummary: {
      evidenceRefs: ["content_analytics:homepage.story.v1"],
    },
    memoryReviewPackets: [
      {
        candidateCount: 2,
        evidenceRefs: ["memory_candidate:1", "memory_candidate:2"],
        limitations: ["generated-content candidate text should not render"],
      },
    ],
  };
}

function storyLearningFixture(): StoryPublishLearningBrief {
  return {
    schemaVersion: "ordo.story_publish_learning_brief.v1",
    status: "partial",
    audience: "staff",
    deckId: "homepage.story.v1",
    readOnly: true,
    mutationPerformed: false,
    confirmedGraphPromotion: false,
    memoryPromotionPerformed: false,
    liveProviderCalled: false,
    externalPublishingClaimed: false,
    sourceStatus: [
      { key: "manual_publish", label: "Manual publish evidence", value: 1, sourceStatus: "manual", evidenceRefs: ["artifact:publish_package"] },
      { key: "external_analytics", label: "External analytics", value: 0, sourceStatus: "missing", evidenceRefs: [] },
    ],
    contentMetrics: [
      { key: "story_views", label: "Story views", value: 12, sourceStatus: "measured", evidenceRefs: ["content_event:viewed"] },
    ],
    publishEvidence: [
      {
        sourceKind: "artifact",
        sourceId: "publish_package",
        status: "staged",
        sourceStatus: "manual",
        evidenceRefs: ["artifact:publish_package"],
        limitations: ["manual_local_publish_evidence_only"],
      },
      {
        sourceKind: "external_platform",
        sourceId: "none",
        status: "missing",
        sourceStatus: "deferred",
        evidenceRefs: [],
        limitations: ["external_publishing_deferred"],
      },
    ],
    memorySummary: {
      candidateCount: 2,
      stateCounts: [
        { key: "proposed", label: "Proposed", value: 2, sourceStatus: "deferred", evidenceRefs: ["memory_candidate:1"] },
      ],
      evidenceRefs: ["memory_candidate:1", "memory_candidate:2"],
      limitations: ["memory_promotion_not_performed"],
      confirmedGraphPromotion: false,
      memoryPromotionPerformed: false,
    },
    outcomeSummary: {
      outcomeCount: 0,
      attributionState: "missing",
      evidenceRefs: [],
      limitations: ["outcome_evidence_missing"],
    },
    rewardSummary: {
      rewardEventCount: 0,
      grantedCount: 0,
      evidenceRefs: [],
      limitations: ["reward_event_evidence_missing"],
    },
    evidenceRefs: ["artifact:publish_package", "content_event:viewed", "memory_candidate:1"],
    limitations: ["external_analytics_missing", "outcome_evidence_missing", "prompt internals should not render"],
    recommendedNextActions: ["connect_content_analytics", "review_owner_learning_brief"],
    analyticsSummary: {
      evidenceRefs: ["content_event:viewed"],
    },
    memoryReviewPackets: [],
  };
}
