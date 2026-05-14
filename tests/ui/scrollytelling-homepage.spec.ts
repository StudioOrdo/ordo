import { expect, test } from "@playwright/test";

const daemonDeck = {
  profile: {
    positioning: "A local-first operating appliance for business motion.",
    audience: "Solopreneurs",
    primaryCta: {
      label: "Talk with Ordo",
      href: "/chat",
      evidenceRefs: ["business_fact:profile.cta"],
    },
    evidenceRefs: ["business_fact:profile.positioning"],
    limitations: [],
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
        body: "A public story backed by durable evidence.",
        copySlots: [{ slot: "sourceLine", value: "Published public homepage profile" }],
        ctaRefs: [
          {
            label: "Talk with Ordo",
            href: "/chat",
            evidenceRefs: ["business_fact:profile.cta"],
          },
        ],
        evidenceRefs: ["business_fact:homepage.identity"],
        limitations: ["No live image generation claim."],
        motionProfile: "cinematic",
        reducedMotionFallback: "Studio Ordo public story.",
        imageBriefMethod: "homepage.prepare_image_briefs",
      },
      {
        slideId: "proof",
        sectionId: "proof",
        order: 2,
        title: "Trust stays local.",
        body: "Canonical tables, events, artifacts, and projections keep claims inspectable.",
        copySlots: [],
        ctaRefs: [
          {
            label: "Open QR path",
            href: "/e/nyc-pilot",
            evidenceRefs: ["tracked_entry_point:nyc-pilot"],
          },
        ],
        evidenceRefs: ["business_fact:homepage.proof", "tracked_entry_point:nyc-pilot"],
        limitations: [],
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

test.describe("scrollytelling homepage runtime", () => {
  test("renders daemon-backed slides with progress, keyboard navigation, and public-safe evidence", async ({ page }) => {
    const analyticsPayloads: Array<Record<string, unknown>> = [];
    await page.route("**/api/public/homepage-story", async (route) => {
      await route.fulfill({ json: daemonDeck });
    });
    await page.route("**/api/public/story-analytics", async (route) => {
      const payload = route.request().postDataJSON() as Record<string, unknown>;
      analyticsPayloads.push(payload);
      await route.fulfill({
        json: {
          event: {
            id: `content_analytics_event_${analyticsPayloads.length}`,
            eventKind: payload.eventKind,
            sourceStatus: payload.visitorSessionId || payload.entryPointSlug ? "measured" : "missing",
          },
          contextState: payload.visitorSessionId || payload.entryPointSlug ? "measured" : "missing",
          limitations: [],
        },
      });
    });

    await page.goto("/?entryPointSlug=nyc-pilot&visitorSessionId=session_1");

    await expect(page.getByRole("heading", { name: "Studio Ordo" })).toBeVisible();
    await expect(page.getByText("Published public homepage profile")).toBeVisible();
    await expect(page.getByRole("navigation", { name: "Story progress" })).toContainText("01 / 02");
    await expect(page.getByRole("link", { name: "Talk with Ordo" })).toHaveAttribute(
      "href",
      "/chat?entryPointSlug=nyc-pilot&visitorSessionId=session_1",
    );
    await expect(page.locator("[data-chat-fab-launcher='true']")).toHaveAttribute(
      "href",
      "/chat?entryPointSlug=nyc-pilot&visitorSessionId=session_1",
    );
    await expect(page.getByText("provider internals")).toHaveCount(0);
    await expect(page.getByText("prompt internals")).toHaveCount(0);
    await expect(page.getByText("graph certainty")).toHaveCount(0);

    await expect
      .poll(() => analyticsPayloads.filter((payload) => payload.eventKind === "viewed").length)
      .toBeGreaterThanOrEqual(1);
    expect(analyticsPayloads[0]).toMatchObject({
      eventKind: "viewed",
      deckId: "homepage.story.v1",
      deckVersion: 1,
      sectionId: "identity",
      entryPointSlug: "nyc-pilot",
      visitorSessionId: "session_1",
    });

    await page.keyboard.press("ArrowDown");
    await expect(page.getByRole("heading", { name: "Trust stays local." })).toBeVisible();
    await expect(page.getByRole("navigation", { name: "Story progress" })).toContainText("02 / 02");
    await expect
      .poll(() => analyticsPayloads.some((payload) => payload.sectionId === "proof" && payload.eventKind === "viewed"))
      .toBe(true);

    await page.getByRole("link", { name: "Open QR path" }).click();
    await expect
      .poll(() => analyticsPayloads.some((payload) => payload.eventKind === "clicked" && payload.ctaId))
      .toBe(true);
    expect(JSON.stringify(analyticsPayloads)).not.toContain("provider internal");
    expect(JSON.stringify(analyticsPayloads)).not.toContain("prompt internal");
    expect(JSON.stringify(analyticsPayloads)).not.toContain("private artifact text");
  });

  test("falls back with explicit readiness when daemon data is unavailable", async ({ page }) => {
    const analyticsPayloads: Array<Record<string, unknown>> = [];
    await page.route("**/api/public/homepage-story", async (route) => {
      await route.fulfill({ status: 503, json: { error: "daemon unavailable" } });
    });
    await page.route("**/api/public/story-analytics", async (route) => {
      analyticsPayloads.push(route.request().postDataJSON() as Record<string, unknown>);
      await route.fulfill({ json: { event: { id: "unexpected" } } });
    });

    await page.goto("/");

    await expect(page.getByRole("heading", { name: "Studio Ordo" })).toBeVisible();
    await expect(page.getByText("daemon-backed public homepage story deck").first()).toBeVisible();
    await expect(page.getByText("No live image generation").first()).toBeVisible();
    await expect(page.getByRole("navigation", { name: "Story progress" })).toContainText("01 / 02");
    expect(analyticsPayloads).toEqual([]);
  });

  test("mobile reduced-motion view remains readable and uses deterministic slide ids", async ({ page }, testInfo) => {
    test.skip(testInfo.project.name !== "mobile-chromium", "mobile-specific reduced motion smoke");

    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.route("**/api/public/homepage-story", async (route) => {
      await route.fulfill({ json: daemonDeck });
    });

    await page.goto("/");

    const slide = page.locator("#story-slide-identity");
    await expect(slide).toBeVisible();
    await expect(slide.getByText("Studio Ordo public story.")).toBeVisible();

    const metrics = await slide.evaluate((element) => {
      const rect = element.getBoundingClientRect();
      return {
        width: rect.width,
        viewportWidth: window.innerWidth,
        scrollWidth: document.documentElement.scrollWidth,
      };
    });
    expect(metrics.width).toBeLessThanOrEqual(metrics.viewportWidth);
    expect(metrics.scrollWidth).toBeLessThanOrEqual(metrics.viewportWidth + 1);
  });
});
