"use client";

import Link from "next/link";
import { useEffect, useMemo, useRef, useState } from "react";

import {
  fallbackHomepageStoryDeck,
  homepageStoryDeckToSlides,
  type HomepageStoryDeckResponse,
  type NarrativeSlideView,
} from "@/lib/scrollytelling-runtime";
import { type ProductRole, roleHref } from "@/lib/product-navigation";

interface ScrollytellingHomepageRuntimeProps {
  role: ProductRole;
  entryPointSlug?: string;
  visitorSessionId?: string;
}

export function ScrollytellingHomepageRuntime({ role, entryPointSlug, visitorSessionId }: ScrollytellingHomepageRuntimeProps) {
  const [deckResponse, setDeckResponse] = useState<HomepageStoryDeckResponse>(() => fallbackHomepageStoryDeck("daemon_unavailable"));
  const [sourceState, setSourceState] = useState<"loading" | "daemon" | "fallback">("loading");
  const [activeIndex, setActiveIndex] = useState(0);
  const slideRefs = useRef<Array<HTMLElement | null>>([]);
  const slides = useMemo(() => homepageStoryDeckToSlides(deckResponse), [deckResponse]);
  const context = useMemo(() => ({ entryPointSlug, visitorSessionId }), [entryPointSlug, visitorSessionId]);

  useEffect(() => {
    let cancelled = false;
    fetch("/api/public/homepage-story", { cache: "no-store" })
      .then(async (response) => {
        if (!response.ok) {
          throw new Error("homepage_story_unavailable");
        }
        return (await response.json()) as HomepageStoryDeckResponse;
      })
      .then((payload) => {
        if (cancelled) {
          return;
        }
        if (payload.deck?.slides?.length) {
          setDeckResponse(payload);
          setSourceState("daemon");
        } else {
          setDeckResponse(fallbackHomepageStoryDeck("missing_deck"));
          setSourceState("fallback");
        }
      })
      .catch(() => {
        if (!cancelled) {
          setDeckResponse(fallbackHomepageStoryDeck("daemon_unavailable"));
          setSourceState("fallback");
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((entry) => entry.isIntersecting)
          .sort((left, right) => right.intersectionRatio - left.intersectionRatio)[0];
        if (!visible) {
          return;
        }
        const nextIndex = slideRefs.current.findIndex((element) => element === visible.target);
        if (nextIndex >= 0) {
          setActiveIndex(nextIndex);
        }
      },
      { root: null, threshold: [0.55, 0.75] },
    );
    slideRefs.current.forEach((element) => {
      if (element) {
        observer.observe(element);
      }
    });
    return () => observer.disconnect();
  }, [slides.length]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) {
        return;
      }
      if (["ArrowDown", "ArrowRight", " ", "PageDown"].includes(event.key)) {
        event.preventDefault();
        const nextIndex = Math.min(activeIndex + 1, slides.length - 1);
        setActiveIndex(nextIndex);
        scrollToSlide(nextIndex, slideRefs.current);
      }
      if (["ArrowUp", "ArrowLeft", "PageUp"].includes(event.key)) {
        event.preventDefault();
        const nextIndex = Math.max(activeIndex - 1, 0);
        setActiveIndex(nextIndex);
        scrollToSlide(nextIndex, slideRefs.current);
      }
      if (/^[1-9]$/.test(event.key)) {
        const target = Number(event.key) - 1;
        if (target < slides.length) {
          event.preventDefault();
          setActiveIndex(target);
          scrollToSlide(target, slideRefs.current);
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [activeIndex, slides.length]);

  return (
    <div
      className="scrolly-runtime"
      data-source-state={sourceState}
      data-ready={deckResponse.readiness.ready ? "true" : "false"}
      data-slide-count={slides.length}
    >
      <StoryProgress slides={slides} activeIndex={activeIndex} />
      <main className="scrolly-track" aria-label="Studio Ordo scrollytelling homepage">
        {slides.map((slide, index) => (
          <NarrativeSlide
            key={slide.id}
            refSetter={(element) => {
              slideRefs.current[index] = element;
            }}
            slide={slide}
            index={index}
            total={slides.length}
            role={role}
            context={context}
            readinessMissing={deckResponse.readiness.missing}
            sourceState={sourceState}
          />
        ))}
      </main>
    </div>
  );
}

function StoryProgress({ slides, activeIndex }: { slides: NarrativeSlideView[]; activeIndex: number }) {
  return (
    <nav className="story-progress" aria-label="Story progress">
      <span className="story-progress-count">
        {String(activeIndex + 1).padStart(2, "0")} / {String(slides.length).padStart(2, "0")}
      </span>
      {slides.map((slide, index) => (
        <a key={slide.id} href={`#story-slide-${slide.id}`} aria-label={`Open slide ${index + 1}: ${slide.title}`}>
          <span>{String(index + 1).padStart(2, "0")}</span>
        </a>
      ))}
    </nav>
  );
}

function NarrativeSlide({
  slide,
  index,
  total,
  role,
  context,
  readinessMissing,
  sourceState,
  refSetter,
}: {
  slide: NarrativeSlideView;
  index: number;
  total: number;
  role: ProductRole;
  context: { entryPointSlug?: string; visitorSessionId?: string };
  readinessMissing: string[];
  sourceState: "loading" | "daemon" | "fallback";
  refSetter: (element: HTMLElement | null) => void;
}) {
  const ctaHref = slide.cta ? roleHref(withEntryContext(slide.cta.href, context), role) : undefined;

  return (
    <section
      ref={refSetter}
      id={`story-slide-${slide.id}`}
      className="scrolly-slide"
      aria-label={slide.title}
      data-motion-profile={slide.motionProfile}
    >
      <div className="scrolly-copy">
        <span className="eyebrow">{slide.eyebrow}</span>
        <h1>{slide.title}</h1>
        <p>{slide.body}</p>
        <p className="reduced-motion-copy">{slide.reducedMotionFallback}</p>
        {slide.sourceLine ? <p className="story-source-line">{slide.sourceLine}</p> : null}
        <div className="story-evidence" aria-label="Evidence and limitations">
          {slide.evidenceRefs.slice(0, 3).map((ref) => (
            <span key={ref}>{ref}</span>
          ))}
          {slide.limitations.slice(0, 4).map((limitation) => (
            <span key={limitation}>{limitation}</span>
          ))}
          {sourceState === "fallback"
            ? readinessMissing.map((missing) => <span key={missing}>{missing}</span>)
            : null}
        </div>
        {slide.cta && ctaHref ? (
          <div className="hero-actions">
            <Link href={ctaHref} className={index === 0 ? "primary-action" : "secondary-action"}>
              {slide.cta.label}
            </Link>
          </div>
        ) : null}
      </div>
      <div className="scrolly-media" aria-hidden="true">
        <span>{String(index + 1).padStart(2, "0")}</span>
        <small>{String(total).padStart(2, "0")}</small>
      </div>
    </section>
  );
}

function scrollToSlide(index: number, slides: Array<HTMLElement | null>) {
  const target = slides[index];
  if (target) {
    target.scrollIntoView({ behavior: prefersReducedMotion() ? "auto" : "smooth", block: "start" });
  }
}

function prefersReducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function withEntryContext(href: string, context: { entryPointSlug?: string; visitorSessionId?: string }): string {
  const url = new URL(href, "https://ordo.local");
  if (context.entryPointSlug) {
    url.searchParams.set("entryPointSlug", context.entryPointSlug);
  }
  if (context.visitorSessionId) {
    url.searchParams.set("visitorSessionId", context.visitorSessionId);
  }
  return `${url.pathname}${url.search}${url.hash}`;
}
