export interface HomepageStoryDeckResponse {
  profile: HomepageStoryProfile;
  deck: HomepageNarrativeDeck;
  readiness: PublicSurfaceReadiness;
  refresh: HomepageStoryRefreshContract;
}

export interface HomepageStoryProfile {
  positioning: string;
  audience: string | null;
  primaryCta: HomepageStoryCta | null;
  evidenceRefs: string[];
  limitations: string[];
}

export interface HomepageNarrativeDeck {
  deckId: string;
  version: number;
  surface: string;
  slides: HomepageNarrativeSlide[];
  evidenceRefs: string[];
  limitations: string[];
}

export interface HomepageNarrativeSlide {
  slideId: string;
  sectionId: string;
  order: number;
  title: string;
  body: string;
  copySlots: HomepageStoryCopySlot[];
  ctaRefs: HomepageStoryCta[];
  evidenceRefs: string[];
  limitations: string[];
  motionProfile: "reduced" | "restrained" | "expressive" | "cinematic" | string;
  reducedMotionFallback: string;
  imageBriefMethod: string | null;
}

export interface HomepageStoryCopySlot {
  slot: string;
  value: unknown;
}

export interface HomepageStoryCta {
  label: string;
  href: string;
  evidenceRefs: string[];
}

export interface HomepageStoryRefreshContract {
  manualRefreshSupported: boolean;
  scheduledRefreshSupported: boolean;
  imageBriefMethod: string;
  liveProviderRequired: boolean;
  limitations: string[];
}

export interface PublicSurfaceReadiness {
  surface: string;
  ready: boolean;
  factCount: number;
  missing: string[];
}

export interface NarrativeSlideView {
  id: string;
  sectionId: string;
  eyebrow: string;
  title: string;
  body: string;
  sourceLine: string | null;
  cta: HomepageStoryCta | null;
  evidenceRefs: string[];
  limitations: string[];
  motionProfile: "reduced" | "restrained" | "expressive" | "cinematic";
  reducedMotionFallback: string;
}

export function homepageStoryDeckToSlides(deckResponse: HomepageStoryDeckResponse): NarrativeSlideView[] {
  const sorted = [...deckResponse.deck.slides].sort((left, right) => left.order - right.order || left.slideId.localeCompare(right.slideId));
  const total = sorted.length;
  return sorted.map((slide, index) => {
    const safeCta = firstSafeCta(slide.ctaRefs);
    const slideEvidenceRefs = stableUnique([...slide.evidenceRefs, ...(safeCta?.evidenceRefs ?? [])]);
    const limitations = stableUnique([...slide.limitations, ...deckResponse.deck.limitations, ...deckResponse.profile.limitations]);
    const title = publicText(slide.title || deckResponse.profile.positioning || "Studio Ordo");
    const body = publicText(slide.body || slide.reducedMotionFallback || deckResponse.profile.positioning);
    const reducedMotionFallback = publicText(slide.reducedMotionFallback || slide.body || slide.title);
    const hasWithheldPublicText = [title, body, reducedMotionFallback].includes(WITHHELD_PUBLIC_TEXT);
    return {
      id: normalizeSlideId(slide.slideId || slide.sectionId || `slide-${index + 1}`),
      sectionId: normalizeSlideId(slide.sectionId || slide.slideId || `slide-${index + 1}`),
      eyebrow: `${String(index + 1).padStart(2, "0")} / ${String(total).padStart(2, "0")}`,
      title,
      body,
      sourceLine: sourceLineFromSlots(slide.copySlots),
      cta: hasWithheldPublicText ? null : safeCta ?? safePublicCta(deckResponse.profile.primaryCta),
      evidenceRefs: slideEvidenceRefs,
      limitations,
      motionProfile: normalizeMotionProfile(slide.motionProfile),
      reducedMotionFallback,
    };
  });
}

export function fallbackHomepageStoryDeck(reason: "daemon_unavailable" | "missing_deck"): HomepageStoryDeckResponse {
  const missing = reason === "daemon_unavailable" ? "daemon-backed public homepage story deck" : "published public homepage slide facts";
  return {
    profile: {
      positioning: "Studio Ordo is a local-first operating appliance for relationship-led business work.",
      audience: "Solopreneurs and small operators",
      primaryCta: {
        label: "Talk with Ordo",
        href: "/chat",
        evidenceRefs: ["readiness:homepage.story"],
      },
      evidenceRefs: ["readiness:homepage.story"],
      limitations: [`Missing ${missing}.`],
    },
    deck: {
      deckId: "homepage.story.fallback",
      version: 1,
      surface: "homepage",
      slides: [
        {
          slideId: "readiness",
          sectionId: "readiness",
          order: 1,
          title: "Studio Ordo",
          body: "The public story runtime is ready, but the daemon-backed narrative deck is not available yet.",
          copySlots: [{ slot: "sourceLine", value: "Readiness state, not generated copy" }],
          ctaRefs: [
            {
              label: "Talk with Ordo",
              href: "/chat",
              evidenceRefs: ["readiness:homepage.story"],
            },
          ],
          evidenceRefs: ["readiness:homepage.story"],
          limitations: [
            `Missing ${missing}.`,
            "No live image generation, video generation, publishing, analytics, or AI capability is claimed.",
          ],
          motionProfile: "reduced",
          reducedMotionFallback: "Studio Ordo story readiness is missing daemon-backed deck data.",
          imageBriefMethod: null,
        },
        {
          slideId: "trust-boundary",
          sectionId: "trust-boundary",
          order: 2,
          title: "Evidence before claims.",
          body: "Ordo shows readiness gaps instead of inventing proof, metrics, publishing status, or provider behavior.",
          copySlots: [{ slot: "sourceLine", value: "Fallback public-surface contract" }],
          ctaRefs: [
            {
              label: "View the QR path",
              href: "/e/nyc-pilot",
              evidenceRefs: ["readiness:tracked_entry_point"],
            },
          ],
          evidenceRefs: ["readiness:homepage.story"],
          limitations: ["Fallback content is deterministic and does not promote generated memory to truth."],
          motionProfile: "reduced",
          reducedMotionFallback: "Evidence before claims.",
          imageBriefMethod: null,
        },
      ],
      evidenceRefs: ["readiness:homepage.story"],
      limitations: [`Missing ${missing}.`, "Fallback is deterministic and public-safe."],
    },
    readiness: {
      surface: "homepage.story",
      ready: false,
      factCount: 0,
      missing: [missing],
    },
    refresh: {
      manualRefreshSupported: false,
      scheduledRefreshSupported: false,
      imageBriefMethod: "homepage.prepare_image_briefs",
      liveProviderRequired: false,
      limitations: ["Refresh and scheduling require daemon-backed public story state."],
    },
  };
}

function sourceLineFromSlots(slots: HomepageStoryCopySlot[]): string | null {
  const source = slots.find((slot) => matchesSourceSlot(slot.slot));
  const value = source ? publicText(source.value) : "";
  return value.trim() ? value : null;
}

function matchesSourceSlot(slot: string): boolean {
  const normalized = slot.toLowerCase();
  return normalized === "sourceline" || normalized === "source_line" || normalized === "evidence" || normalized === "proof";
}

function firstSafeCta(ctas: HomepageStoryCta[]): HomepageStoryCta | null {
  for (const cta of ctas) {
    const safe = safePublicCta(cta);
    if (safe) {
      return safe;
    }
  }
  return null;
}

function safePublicCta(cta: HomepageStoryCta | null): HomepageStoryCta | null {
  if (!cta || !safeHref(cta.href)) {
    return null;
  }
  const label = publicText(cta.label);
  if (!label.trim() || label === WITHHELD_PUBLIC_TEXT) {
    return null;
  }
  return {
    label,
    href: cta.href,
    evidenceRefs: stableUnique(cta.evidenceRefs),
  };
}

function safeHref(href: string): boolean {
  return href.startsWith("/") && !href.startsWith("//");
}

function normalizeMotionProfile(value: string): NarrativeSlideView["motionProfile"] {
  return value === "reduced" || value === "restrained" || value === "expressive" || value === "cinematic" ? value : "restrained";
}

function normalizeSlideId(value: string): string {
  const normalized = value
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return normalized || "slide";
}

function publicText(value: unknown): string {
  const text =
    typeof value === "string"
      ? value
      : typeof value === "number" || typeof value === "boolean"
        ? String(value)
        : "";
  return unsafePublicText(text) ? WITHHELD_PUBLIC_TEXT : text;
}

const WITHHELD_PUBLIC_TEXT = "Public-safe content withheld pending review.";

function unsafePublicText(text: string): boolean {
  const normalized = text.toLowerCase();
  return [
    "staff routing",
    "provider internal",
    "provider secret",
    "prompt internal",
    "raw policy",
    "policy internal",
    "owner-only",
    "private artifact",
    "compiled-plan",
    "compiled plan private",
    "task result private",
    "graph certainty",
    "secret:",
    "api key",
  ].some((marker) => normalized.includes(marker));
}

function stableUnique(values: string[]): string[] {
  return Array.from(new Set(values.filter((value) => value.trim()))).sort();
}
