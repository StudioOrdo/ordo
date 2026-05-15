import { expect, test } from "@playwright/test";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";

import {
  buildStudioPublicationsView,
  type GeneratedContentMemoryReviewPacket,
  type StudioProductionReviewPacket,
  type StoryPublishLearningBrief,
} from "@/lib/studio-publications";

const daemonPort = 19080;

interface MockDaemonState {
  requests: string[];
  requestBodies: unknown[];
  mode: "ready" | "empty" | "memory-degraded";
}

test.describe("Studio Publications generated-content memory review view model", () => {
  test("surfaces safe memory review state and decision affordances without leaking candidate internals", () => {
    const view = buildStudioPublicationsView(storyReviewFixture(), storyLearningFixture(), [memoryPacketFixture()]);

    expect(view.memoryCandidateCount).toBe(1);
    expect(view.memoryReviewPackets).toHaveLength(1);
    expect(view.memoryReviewPackets[0]).toMatchObject({
      artifactId: "Story deck",
      candidateCount: 1,
      evidenceRefCount: 2,
      confirmedGraphPromotion: false,
      liveProviderCalled: false,
    });
    expect(view.memoryReviewPackets[0]?.items[0]).toMatchObject({
      candidateId: "memory_candidate:story:1",
      state: "proposed",
      memoryTier: "candidate",
      canApprove: true,
      canReject: true,
      memoryEffect: "candidate_only",
    });
    expect(JSON.stringify(view)).toContain("Homepage story positioning candidate");
    expect(JSON.stringify(view)).not.toContain("rawPrompt");
    expect(JSON.stringify(view)).not.toContain("provider internal");
    expect(JSON.stringify(view)).not.toContain("prompt internal");
    expect(JSON.stringify(view)).not.toContain("private artifact text");
    expect(JSON.stringify(view)).not.toContain("generated-content candidate text");
    expect(JSON.stringify(view)).not.toContain("graph certainty");
    expect(JSON.stringify(view)).not.toContain("body should not render");
  });

  test("keeps no-candidate memory review packets explicit", () => {
    const packet = {
      ...memoryPacketFixture(),
      candidateCount: 0,
      evidenceRefs: [],
      items: [],
      limitations: ["no_generated_content_memory_candidates"],
    };

    const view = buildStudioPublicationsView(storyReviewFixture(), storyLearningFixture(), [packet]);

    expect(view.memoryReviewPackets).toHaveLength(1);
    expect(view.memoryReviewPackets[0]?.candidateCount).toBe(0);
    expect(view.memoryReviewPackets[0]?.items).toEqual([]);
    expect(view.memoryReviewPackets[0]?.limitations).toContain("No generated content memory candidates");
  });
});

test.describe.configure({ mode: "serial" });

test.afterEach(async ({ page }) => {
  await page.close();
});

test("Studio Publications renders memory review candidates and records approve decision through protected daemon route", async ({
  page,
}, testInfo) => {
  const daemon = await startMockDaemon("ready");
  try {
    await page.goto(productContentUrl("/studio/publications?role=studio&artifactIds=story_deck", testInfo));

    await expect(page.locator("main")).toContainText("Generated-Content Memory Review");
    await expect(page.locator("main")).toContainText("Homepage story positioning candidate");
    await expect(page.locator("main")).toContainText("Memory candidate:story:1");
    await expect(page.locator("main")).toContainText("Approve");
    await expect(page.locator("main")).toContainText("Reject");
    await expect(page.locator("main")).not.toContainText("body should not render");
    await expect(page.locator("main")).not.toContainText("private artifact text");
    await expect(page.locator("main")).not.toContainText("provider internal");

    await page.getByRole("button", { name: "Approve memory candidate memory_candidate:story:1" }).click();
    await expect(page.locator("main")).toContainText("Decision recorded.");

    expect(
      daemon.state.requests.some((request) =>
        request.startsWith("GET /studio/generated-content-memory/story_deck/review?"),
      ),
    ).toBe(true);
    expect(
      daemon.state.requests.some((request) =>
        request === "POST /studio/generated-content-memory/candidates/memory_candidate%3Astory%3A1/decision",
      ),
    ).toBe(true);
    expect(daemon.state.requestBodies).toContainEqual({
      decision: "approved",
      reason: "Owner/staff approved candidate memory from Studio Publications.",
      evidenceRefs: ["artifact:story_deck", "memory_candidate:story:1"],
    });
  } finally {
    await daemon.close();
  }
});

test("Studio Publications refuses member memory decisions before daemon writes", async ({ page }) => {
  const daemon = await startMockDaemon("ready");
  try {
    const response = await page.request.post(
      "/api/studio/generated-content-memory/candidates/memory_candidate%3Astory%3A1/decision?role=member",
      {
        data: {
          decision: "approved",
          reason: "should not reach daemon",
          evidenceRefs: ["memory_candidate:story:1"],
        },
      },
    );

    expect(response.status()).toBe(403);
    expect(daemon.state.requests).toEqual([]);
  } finally {
    await daemon.close();
  }
});

test("Studio Publications rejects unsupported memory decisions before daemon writes", async ({ page }) => {
  const daemon = await startMockDaemon("ready");
  try {
    const response = await page.request.post(
      "/api/studio/generated-content-memory/candidates/memory_candidate%3Astory%3A1/decision?role=studio",
      {
        data: {
          decision: "published",
          reason: "unsupported from Studio Publications",
          evidenceRefs: ["memory_candidate:story:1"],
        },
      },
    );

    expect(response.status()).toBe(400);
    expect(daemon.state.requests).toEqual([]);
  } finally {
    await daemon.close();
  }
});

test("Studio Publications keeps empty generated-content memory review state explicit", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon("empty");
  try {
    await page.goto(productContentUrl("/studio/publications?role=studio&artifactIds=story_deck", testInfo));

    await expect(page.locator("main")).toContainText("Generated-Content Memory Review");
    await expect(page.locator("main")).toContainText("No generated-content memory candidates are available for owner/staff review.");
  } finally {
    await daemon.close();
  }
});

test("Studio Publications reports degraded memory review route without inventing promotion", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon("memory-degraded");
  try {
    await page.goto(productContentUrl("/studio/publications?role=studio&artifactIds=story_deck", testInfo));

    await expect(page.locator("main")).toContainText("degraded");
    await expect(page.locator("main")).toContainText("/studio/generated-content-memory/story_deck/review");
    await expect(page.locator("main")).toContainText("Memory promotion not performed");
  } finally {
    await daemon.close();
  }
});

async function startMockDaemon(mode: MockDaemonState["mode"]): Promise<{ state: MockDaemonState; close: () => Promise<void> }> {
  const state: MockDaemonState = { requests: [], requestBodies: [], mode };
  const server = createServer((request, response) => {
    void handleRequest(request, response, state);
  });
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

async function handleRequest(request: IncomingMessage, response: ServerResponse, state: MockDaemonState) {
  const method = request.method ?? "GET";
  const path = request.url ?? "/";
  state.requests.push(`${method} ${path}`);

  if (method === "GET" && path.startsWith("/studio/story-production-review")) {
    return jsonResponse(response, storyReviewFixture());
  }

  if (method === "GET" && path.startsWith("/studio/story-publish-learning")) {
    return jsonResponse(response, storyLearningFixture());
  }

  if (method === "GET" && path.startsWith("/studio/generated-content-memory/story_deck/review")) {
    if (state.mode === "memory-degraded") {
      response.writeHead(503, { "content-type": "application/json" });
      response.end(JSON.stringify({ error: "memory review unavailable" }));
      return;
    }
    return jsonResponse(response, state.mode === "empty" ? emptyMemoryPacketFixture() : memoryPacketFixture());
  }

  if (method === "POST" && path === "/studio/generated-content-memory/candidates/memory_candidate%3Astory%3A1/decision") {
    state.requestBodies.push(await readJsonBody(request));
    return jsonResponse(response, {
      candidate: {
        id: "memory_candidate:story:1",
        state: "approved",
      },
      event: {
        cursor: 42,
        eventType: "generated_content_memory.decision_recorded",
      },
    });
  }

  response.writeHead(404, { "content-type": "application/json" });
  response.end(JSON.stringify({ error: `Unhandled mock daemon route: ${method} ${path}` }));
}

function jsonResponse(response: ServerResponse, body: unknown) {
  response.writeHead(200, { "content-type": "application/json" });
  response.end(JSON.stringify(body));
}

function readJsonBody(request: IncomingMessage): Promise<unknown> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    request.on("data", (chunk: Buffer) => chunks.push(chunk));
    request.on("error", reject);
    request.on("end", () => {
      try {
        resolve(JSON.parse(Buffer.concat(chunks).toString("utf8")));
      } catch (error) {
        reject(error);
      }
    });
  });
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
    evidenceRefs: ["artifact:story_deck"],
    limitations: ["story_production_review_is_read_only"],
    missingPrerequisites: [],
    recommendedNextActions: ["review_memory_candidates"],
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
        limitations: [],
        recommendedNextAction: "approve_manual_publish_package",
      },
    ],
    analyticsSummary: null,
    memoryReviewPackets: [],
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
    sourceStatus: [],
    contentMetrics: [],
    publishEvidence: [],
    memorySummary: {
      candidateCount: 1,
      stateCounts: [{ key: "proposed", label: "Proposed", value: 1, sourceStatus: "deferred", evidenceRefs: ["memory_candidate:story:1"] }],
      evidenceRefs: ["memory_candidate:story:1"],
      limitations: ["memory_promotion_not_performed"],
      confirmedGraphPromotion: false,
      memoryPromotionPerformed: false,
    },
    outcomeSummary: {
      outcomeCount: 0,
      attributionState: "missing",
      evidenceRefs: [],
      limitations: [],
    },
    rewardSummary: {
      rewardEventCount: 0,
      grantedCount: 0,
      evidenceRefs: [],
      limitations: [],
    },
    evidenceRefs: ["memory_candidate:story:1"],
    limitations: [],
    recommendedNextActions: ["review_memory_candidates"],
    analyticsSummary: null,
    memoryReviewPackets: [],
  };
}

function memoryPacketFixture(): GeneratedContentMemoryReviewPacket {
  return {
    schemaVersion: "ordo.generated_content_memory_review_packet.v1",
    artifactId: "story_deck",
    sourceArtifactKind: "story_narrative_deck",
    audience: "staff",
    candidateCount: 1,
    sourceArtifactRefs: ["artifact:story_deck"],
    workflowRefs: ["job:story_workflow"],
    evidenceRefs: ["memory_candidate:story:1", "artifact:story_deck"],
    limitations: ["candidate_memory_requires_owner_review", "provider internal should not render"],
    items: [
      {
        candidateId: "memory_candidate:story:1",
        memoryKind: "business_positioning",
        memoryTier: "candidate",
        candidateState: "proposed",
        confidence: 0.74,
        summaryText: "Homepage story positioning candidate",
        body: "body should not render private artifact text",
        bodyRedacted: true,
        sourceArtifactRefs: ["artifact:story_deck"],
        workflowRefs: ["job:story_workflow"],
        evidenceRefs: ["memory_candidate:story:1", "artifact:story_deck"],
        limitations: ["generated-content candidate text should not render"],
        approvalEvidenceRefs: [],
        publicationEvidenceRefs: [],
        feedbackEvidenceRefs: [],
        outcomeEvidenceRefs: [],
        rejectionEvidenceRefs: [],
        memoryEffect: "candidate_only",
        recommendedReviewAction: "approve_or_reject_candidate",
        confirmedGraphPromotion: false,
      },
    ],
    extensionPoints: ["manual_owner_review"],
    confirmedGraphPromotion: false,
    liveProviderCalled: false,
  };
}

function emptyMemoryPacketFixture(): GeneratedContentMemoryReviewPacket {
  return {
    ...memoryPacketFixture(),
    candidateCount: 0,
    evidenceRefs: [],
    limitations: ["no_generated_content_memory_candidates"],
    items: [],
  };
}
