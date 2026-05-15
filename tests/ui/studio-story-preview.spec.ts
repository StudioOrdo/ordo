import { expect, test } from "@playwright/test";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";

import { buildStudioStoryPreviewView, type StudioStoryPreviewInput } from "@/lib/studio-story-preview";
import type { HomepageStoryDeckResponse } from "@/lib/scrollytelling-runtime";
import type { StoryPublishLearningBrief, StudioProductionReviewPacket } from "@/lib/studio-publications";

const daemonPort = 19080;

interface MockDaemonState {
  requests: string[];
  mode: "ready" | "missing";
}

test.describe("Studio Story preview view model", () => {
  test("assembles safe slides with publication readiness and deferred states", () => {
    const view = buildStudioStoryPreviewView(previewInputFixture("ready"));

    expect(view.status).toBe("manual");
    expect(view.deckId).toBe("homepage.story.v1");
    expect(view.slideCount).toBe(2);
    expect(view.safeEvidenceRefCount).toBe(5);
    expect(view.summaryLines).toEqual([
      "2 protected preview slide(s) are assembled from daemon-backed homepage story evidence.",
      "3 Story publication evidence component(s) are available for owner/staff review.",
      "Preview reads do not publish, mutate analytics truth, promote memory, promote graph truth, call providers, or execute tasks.",
    ]);
    expect(view.nextActions).toContain("Request manual publish approval");
    expect(view.deferredStates.map((state) => state.key)).toEqual(
      expect.arrayContaining(["external_publishing", "memory_promotion", "graph_promotion", "live_provider"]),
    );
    expect(JSON.stringify(view)).not.toContain("provider_internal");
    expect(JSON.stringify(view)).not.toContain("prompt_internal");
    expect(JSON.stringify(view)).not.toContain("private_artifact_text");
    expect(JSON.stringify(view)).not.toContain("generated_content_candidate_text");
    expect(JSON.stringify(view)).not.toContain("graph_certainty");
  });

  test("keeps missing preview evidence explicit", () => {
    const view = buildStudioStoryPreviewView(previewInputFixture("missing"));

    expect(view.status).toBe("missing");
    expect(view.slideCount).toBe(0);
    expect(view.summaryLines).toEqual([
      "No protected preview slides are available from daemon-backed homepage story evidence.",
      "Missing or degraded publication evidence remains explicit.",
      "Preview reads do not publish, mutate analytics truth, promote memory, promote graph truth, call providers, or execute tasks.",
    ]);
    expect(view.nextActions).toContain("Resolve daemon-backed homepage story deck");
  });
});

test.describe.configure({ mode: "serial" });

test.afterEach(async ({ page }) => {
  await page.close();
});

test("Studio Story Preview renders protected deck and publication readiness", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon("ready");
  try {
    await page.goto(productContentUrl("/studio/story-preview?role=studio", testInfo));

    await expect(page.locator("main").getByRole("heading", { name: "Story Preview", exact: true })).toBeVisible();
    await expect(page.locator("main")).toContainText("Homepage Story Preview");
    await expect(page.locator("main")).toContainText("Studio Ordo");
    await expect(page.locator("main")).toContainText("Trust stays local.");
    await expect(page.locator("main")).toContainText("Story Publication Readiness");
    await expect(page.locator("main")).toContainText("Request manual publish approval");
    await expect(page.locator("main")).toContainText("Preview reads do not publish");
    await expect(page.locator("main")).not.toContainText("provider_internal");
    await expect(page.locator("main")).not.toContainText("prompt_internal");
    await expect(page.locator("main")).not.toContainText("private_artifact_text");
    await expect(page.locator("main")).not.toContainText("generated_content_candidate_text");
    await expect(page.locator("main")).not.toContainText("graph_certainty");
    expect(daemon.state.requests.some((request) => request === "GET /public/homepage-story")).toBe(true);
    expect(daemon.state.requests.some((request) => request.startsWith("GET /studio/story-production-review?"))).toBe(true);
    expect(daemon.state.requests.some((request) => request.startsWith("GET /studio/story-publish-learning?"))).toBe(true);
  } finally {
    await daemon.close();
  }
});

test("Studio Story Preview refuses member role before daemon reads", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon("ready");
  try {
    await page.goto(productContentUrl("/studio/story-preview?role=member", testInfo));

    await expect(page.locator("body")).not.toContainText("Homepage Story Preview");
    expect(daemon.state.requests).toEqual([]);
  } finally {
    await daemon.close();
  }
});

test("Studio Story Preview keeps degraded evidence explicit", async ({ page }, testInfo) => {
  await page.goto(productContentUrl("/studio/story-preview?role=studio", testInfo));

  await expect(page.locator("main").getByRole("heading", { name: "Story Preview", exact: true })).toBeVisible();
  await expect(page.locator("main")).toContainText("degraded");
  await expect(page.locator("main")).toContainText("Studio Story preview evidence is degraded because daemon Story routes are unavailable.");
  await expect(page.locator("main")).toContainText("/public/homepage-story");
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

  if (method === "GET" && path === "/public/homepage-story") {
    return jsonResponse(response, state.mode === "ready" ? homepageDeckFixture() : missingHomepageDeckFixture());
  }

  if (method === "GET" && path.startsWith("/studio/story-production-review")) {
    return jsonResponse(response, storyReviewFixture(state.mode));
  }

  if (method === "GET" && path.startsWith("/studio/story-publish-learning")) {
    return jsonResponse(response, storyLearningFixture(state.mode));
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

function previewInputFixture(mode: "ready" | "missing"): StudioStoryPreviewInput {
  return {
    deck: mode === "ready" ? homepageDeckFixture() : missingHomepageDeckFixture(),
    review: storyReviewFixture(mode),
    learning: storyLearningFixture(mode),
    degradedReason: null,
  };
}

function homepageDeckFixture(): HomepageStoryDeckResponse {
  return {
    profile: {
      positioning: "A local-first operating appliance for business motion.",
      audience: "Solopreneurs",
      primaryCta: {
        label: "Talk with Ordo",
        href: "/chat",
        evidenceRefs: ["business_fact:profile.cta"],
      },
      evidenceRefs: ["business_fact:profile.positioning"],
      limitations: ["provider_internal should not render"],
    },
    deck: {
      deckId: "homepage.story.v1",
      version: 1,
      surface: "homepage",
      slides: [
        {
          slideId: "identity",
          sectionId: "identity",
          order: 1,
          title: "Studio Ordo",
          body: "A protected preview backed by durable evidence.",
          copySlots: [{ slot: "sourceLine", value: "Published public homepage profile" }],
          ctaRefs: [
            {
              label: "Talk with Ordo",
              href: "/chat",
              evidenceRefs: ["business_fact:profile.cta"],
            },
          ],
          evidenceRefs: ["business_fact:homepage.identity"],
          limitations: [],
          motionProfile: "cinematic",
          reducedMotionFallback: "Studio Ordo public story.",
          imageBriefMethod: "homepage.prepare_image_briefs",
        },
        {
          slideId: "unsafe",
          sectionId: "unsafe",
          order: 2,
          title: "prompt_internal should not render",
          body: "private_artifact_text and graph_certainty should not render",
          copySlots: [{ slot: "sourceLine", value: "generated_content_candidate_text should not render" }],
          ctaRefs: [],
          evidenceRefs: ["business_fact:homepage.proof"],
          limitations: ["task_private_payload should not render"],
          motionProfile: "restrained",
          reducedMotionFallback: "Trust stays local.",
          imageBriefMethod: null,
        },
      ],
      evidenceRefs: ["business_fact:homepage.identity", "business_fact:homepage.proof"],
      limitations: ["Live publishing is not part of this projection."],
    },
    readiness: {
      surface: "homepage.story",
      ready: true,
      factCount: 7,
      missing: [],
    },
    refresh: {
      manualRefreshSupported: true,
      scheduledRefreshSupported: true,
      imageBriefMethod: "homepage.prepare_image_briefs",
      liveProviderRequired: false,
      limitations: ["Refresh support is a contract extension point."],
    },
  };
}

function missingHomepageDeckFixture(): HomepageStoryDeckResponse {
  return {
    ...homepageDeckFixture(),
    deck: {
      ...homepageDeckFixture().deck,
      slides: [],
      evidenceRefs: [],
      limitations: ["daemon-backed homepage story deck missing"],
    },
    readiness: {
      surface: "homepage.story",
      ready: false,
      factCount: 0,
      missing: ["daemon-backed homepage story deck"],
    },
  };
}

function storyReviewFixture(mode: "ready" | "missing"): StudioProductionReviewPacket {
  return {
    schemaVersion: "ordo.story_production_review_packet.v1",
    status: mode === "ready" ? "partial" : "missing",
    audience: "staff",
    readOnly: true,
    mutationPerformed: false,
    confirmedGraphPromotion: false,
    liveProviderCalled: false,
    externalPublishingClaimed: false,
    deckId: "homepage.story.v1",
    evidenceRefs: mode === "ready" ? ["artifact:narrative_deck", "business_fact:homepage.identity"] : [],
    limitations: ["Story production review is read only", "provider_internal should not render"],
    missingPrerequisites: mode === "ready" ? ["manual_publish_approval"] : ["homepage_story_deck"],
    recommendedNextActions: mode === "ready" ? ["request_manual_publish_approval"] : ["resolve_daemon_backed_homepage_story_deck"],
    components:
      mode === "ready"
        ? [
            {
              key: "narrative_deck",
              status: "ready",
              artifactRef: "artifact:narrative_deck",
              artifactKind: "story.narrative_deck",
              title: "Narrative deck review package",
              summary: "Public-safe deck is ready for preview.",
              visibility: "owner",
              evidenceStatus: "measured",
              evidenceRefs: ["artifact:narrative_deck"],
              limitations: [],
              recommendedNextAction: "request_manual_publish_approval",
            },
            {
              key: "publish_approval",
              status: "needs_review",
              artifactRef: "artifact:approval",
              artifactKind: "story.publish_approval",
              title: "Homepage publish approval",
              summary: "Manual approval remains required.",
              visibility: "owner",
              evidenceStatus: "manual",
              evidenceRefs: ["artifact:approval"],
              limitations: ["external_publishing_not_claimed"],
              recommendedNextAction: "request_manual_publish_approval",
            },
          ]
        : [],
    analyticsSummary: null,
    memoryReviewPackets: [],
  };
}

function storyLearningFixture(mode: "ready" | "missing"): StoryPublishLearningBrief {
  return {
    schemaVersion: "ordo.story_publish_learning_brief.v1",
    status: mode === "ready" ? "partial" : "missing",
    audience: "staff",
    deckId: "homepage.story.v1",
    readOnly: true,
    mutationPerformed: false,
    confirmedGraphPromotion: false,
    memoryPromotionPerformed: false,
    liveProviderCalled: false,
    externalPublishingClaimed: false,
    sourceStatus: [
      {
        key: "manual_publish_evidence",
        label: "Manual publish evidence",
        value: mode === "ready" ? 1 : 0,
        sourceStatus: mode === "ready" ? "manual" : "missing",
        evidenceRefs: mode === "ready" ? ["content_event:manual_publish"] : [],
      },
    ],
    contentMetrics: [],
    publishEvidence: [],
    memorySummary: {
      candidateCount: 1,
      stateCounts: [],
      evidenceRefs: ["memory_candidate:homepage_story"],
      limitations: ["memory_promotion_not_performed"],
      confirmedGraphPromotion: false,
      memoryPromotionPerformed: false,
    },
    outcomeSummary: {
      outcomeCount: 0,
      attributionState: "missing",
      evidenceRefs: [],
      limitations: ["external_analytics_missing"],
    },
    rewardSummary: {
      rewardEventCount: 0,
      grantedCount: 0,
      evidenceRefs: [],
      limitations: [],
    },
    evidenceRefs: mode === "ready" ? ["content_event:manual_publish", "memory_candidate:homepage_story"] : [],
    limitations: ["external_analytics_missing"],
    recommendedNextActions: mode === "ready" ? ["request_manual_publish_approval"] : ["resolve_daemon_backed_homepage_story_deck"],
    analyticsSummary: null,
    memoryReviewPackets: [],
  };
}
