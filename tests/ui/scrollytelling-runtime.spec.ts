import { expect, test } from "@playwright/test";

import { fallbackHomepageStoryDeck, homepageStoryDeckToSlides } from "@/lib/scrollytelling-runtime";

test.describe("scrollytelling runtime mapping", () => {
  test("maps daemon homepage story deck into stable public-safe slides", () => {
    const slides = homepageStoryDeckToSlides({
      profile: {
        positioning: "A local-first operating appliance for solopreneurs.",
        audience: "Solo operators",
        primaryCta: {
          label: "Talk with Ordo",
          href: "/chat?entryPointSlug=nyc-pilot",
          evidenceRefs: ["business_fact:cta"],
        },
        evidenceRefs: ["business_fact:profile"],
        limitations: [],
      },
      deck: {
        deckId: "homepage.story.v1",
        version: 1,
        surface: "homepage",
        slides: [
          {
            slideId: "proof",
            sectionId: "proof",
            order: 2,
            title: "Evidence before claims.",
            body: "Every public claim needs durable evidence.",
            copySlots: [{ slot: "sourceLine", value: "Approved public fact" }],
            ctaRefs: [],
            evidenceRefs: ["business_fact:proof"],
            limitations: [],
            motionProfile: "cinematic",
            reducedMotionFallback: "Evidence before claims.",
            imageBriefMethod: "homepage.prepare_image_briefs",
          },
          {
            slideId: "identity",
            sectionId: "identity",
            order: 1,
            title: "Studio Ordo",
            body: "The public story starts from the business appliance.",
            copySlots: [],
            ctaRefs: [
              {
                label: "View the offer",
                href: "/offers?entryPointSlug=nyc-pilot&visitorSessionId=session_1",
                evidenceRefs: ["offer:trial"],
              },
            ],
            evidenceRefs: ["business_fact:identity"],
            limitations: ["No live publishing claim."],
            motionProfile: "restrained",
            reducedMotionFallback: "Studio Ordo story.",
            imageBriefMethod: null,
          },
          {
            slideId: "unsafe",
            sectionId: "unsafe",
            order: 3,
            title: "Provider internals should never render",
            body: "Prompt internals and graph certainty should be withheld.",
            copySlots: [],
            ctaRefs: [
              {
                label: "Open provider internals",
                href: "/admin/providers",
                evidenceRefs: ["provider internals:secret"],
              },
            ],
            evidenceRefs: ["owner-only:business_fact:unsafe"],
            limitations: ["private artifact text should not render"],
            motionProfile: "cinematic",
            reducedMotionFallback: "Raw policy internals withheld.",
            imageBriefMethod: null,
          },
        ],
        evidenceRefs: ["business_fact:identity", "offer:trial"],
        limitations: ["Live image generation is not part of this projection."],
      },
      readiness: {
        surface: "homepage.story",
        ready: true,
        factCount: 8,
        missing: [],
      },
      refresh: {
        manualRefreshSupported: true,
        scheduledRefreshSupported: true,
        imageBriefMethod: "homepage.prepare_image_briefs",
        liveProviderRequired: false,
        limitations: ["Refresh support is a contract extension point."],
      },
    });

    expect(slides.map((slide) => slide.id)).toEqual(["identity", "proof", "unsafe"]);
    expect(slides[0]).toMatchObject({
      id: "identity",
      eyebrow: "01 / 03",
      title: "Studio Ordo",
      motionProfile: "restrained",
      cta: {
        label: "View the offer",
        href: "/offers?entryPointSlug=nyc-pilot&visitorSessionId=session_1",
      },
    });
    expect(slides[0].evidenceRefs).toEqual(["business_fact:identity", "offer:trial"]);
    expect(slides[2].title).toBe("Public-safe content withheld pending review.");
    expect(slides[2].body).toBe("Public-safe content withheld pending review.");
    expect(slides[2].reducedMotionFallback).toBe("Public-safe content withheld pending review.");
    expect(slides[2].cta).toBeNull();
    expect(JSON.stringify(slides)).not.toContain("provider");
    expect(JSON.stringify(slides)).not.toContain("prompt");
    expect(JSON.stringify(slides)).not.toContain("owner-only");
    expect(JSON.stringify(slides)).not.toContain("private artifact");
  });

  test("fallback deck is explicit about missing daemon readiness", () => {
    const fallback = fallbackHomepageStoryDeck("daemon_unavailable");
    const slides = homepageStoryDeckToSlides(fallback);

    expect(fallback.readiness.ready).toBe(false);
    expect(fallback.readiness.missing).toContain("daemon-backed public homepage story deck");
    expect(slides[0].title).toBe("Studio Ordo");
    expect(slides[0].limitations.join(" ")).toContain("daemon-backed");
    expect(JSON.stringify(fallback)).not.toContain("analytics claim");
  });
});
