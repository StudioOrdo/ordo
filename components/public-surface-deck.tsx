import Link from "next/link";

import { OrdoFrame } from "@/components/ordo-frame";
import { OrdoChatPrototype } from "@/components/ordo-chat-prototype";
import {
  appSpaceLabel,
  defaultProductBrandSettings,
  type ProductBrandSettings,
  isAdminRole,
  type ProductAppSpace,
  type ProductRole,
  roleHref,
  roleLabel,
} from "@/lib/product-navigation";

interface PublicSurfaceDeckProps {
  role: ProductRole;
  configuredHomeMode?: PublicHomeMode;
  surfaceMode?: PublicHomeMode;
}

export type PublicHomeMode = "story" | "chat";

export function PublicSurfaceDeck({ role, configuredHomeMode = "story", surfaceMode = configuredHomeMode }: PublicSurfaceDeckProps) {
  const surfaces = surfaceMode === "chat" ? [surfaceDefinitions.chat] : [surfaceDefinitions.about, surfaceDefinitions.feed];

  return (
    <OrdoFrame
      role={role}
      homeHref={publicHref("/", role, configuredHomeMode)}
      topRail={<PublicTopRail role={role} configuredHomeMode={configuredHomeMode} activeAppSpaceId="site" showBrand={false} />}
    >
      <div className="public-deck-shell" data-role={role} data-home-mode={configuredHomeMode} data-surface-mode={surfaceMode}>
        {surfaceMode === "story" ? (
          <nav className="public-progress-rail" aria-label="Surface progress">
            <span className="public-progress-count">01 / {String(surfaces.length).padStart(2, "0")}</span>
            {surfaces.map((surface, index) => (
              <a key={surface.id} href={`#${surface.id}`} aria-label={surface.label} title={surface.label}>
                <span>{String(index + 1).padStart(2, "0")}</span>
              </a>
            ))}
          </nav>
        ) : null}

        <main className="public-surface-track" aria-label="Studio Ordo surface deck">
          {surfaces.map((surface, index) => (
            <section key={surface.id} id={surface.id} className={`public-surface-slide public-surface-${surface.id}`} aria-label={surface.label}>
              <div className="surface-count">{String(index + 1).padStart(2, "0")} / {String(surfaces.length).padStart(2, "0")}</div>
              {surface.id === "chat" ? <ChatSurfaceSlide role={role} /> : null}
              {surface.id === "about" ? <AboutSurfaceSlide role={role} /> : null}
              {surface.id === "feed" ? <FeedSurfaceSlide role={role} /> : null}
            </section>
          ))}
        </main>

        {surfaceMode === "story" && configuredHomeMode === "story" ? (
          <Link href={publicHref("/chat", role, configuredHomeMode)} className="public-chat-fab" aria-label="Open full-screen Ordo" data-chat-fab-launcher="true">
            <span className="public-chat-fab-glow" aria-hidden="true" />
            <span className="public-chat-fab-icon" aria-hidden="true">
              <ChatIcon />
            </span>
          </Link>
        ) : null}
      </div>
    </OrdoFrame>
  );
}

export function PublicTopRail({
  role,
  configuredHomeMode = "story",
  activeAppSpaceId,
  brandSettings = defaultProductBrandSettings,
  showBrand = true,
  locationLabel,
}: {
  role: ProductRole;
  configuredHomeMode?: PublicHomeMode;
  activeAppSpaceId?: ProductAppSpace;
  brandSettings?: ProductBrandSettings;
  showBrand?: boolean;
  locationLabel?: string;
}) {
  const alternateHref = configuredHomeMode === "chat" ? publicHref("/about", role, configuredHomeMode) : publicHref("/chat", role, configuredHomeMode);
  const alternateLabel = configuredHomeMode === "chat" ? "About" : "Ordo";
  const homeHref = publicHref("/", role, configuredHomeMode);
  const activeShellLabel = activeAppSpaceId ? appSpaceLabel(activeAppSpaceId) : undefined;

  return (
    <header className="public-top-rail">
      <nav className="public-top-nav" aria-label="Public navigation">
        {showBrand ? (
          <Link href={homeHref} className="public-brand" aria-label={`${brandSettings.siteTitle} home`}>
            {brandSettings.displayMode !== "title" ? (
              <span className="brand-logo-mark" aria-hidden="true">
                <img src="/logo.png" alt="" className="brand-logo-image" />
              </span>
            ) : null}
            {brandSettings.displayMode !== "logo" ? <span className="brand-title">{brandSettings.siteTitle}</span> : null}
          </Link>
        ) : null}
        <Link href={homeHref} className="public-nav-link">
          Home
        </Link>
        <Link href={alternateHref} className="public-nav-link">
          {alternateLabel}
        </Link>
        {role !== "anonymous" ? (
          <span className="public-shell-label" aria-label="Current shell">
            {locationLabel ?? activeShellLabel ?? "Site"}
          </span>
        ) : null}
      </nav>
      {role === "anonymous" ? (
        <nav className="public-auth-nav" aria-label="Visitor account actions">
          <Link href="/login">Login</Link>
          <Link href="/register" className="public-auth-primary">
            Register
          </Link>
        </nav>
      ) : (
        <nav className="public-auth-nav" aria-label="Member actions">
          {isAdminRole(role) ? (
            <Link href={roleHref(configuredHomeMode === "story" ? "/?home=chat" : "/?home=story", role)}>
              {configuredHomeMode === "story" ? "Chat home" : "Story home"}
            </Link>
          ) : null}
          <Link href={roleHref("/my/chat", role)} className="public-auth-primary">
            Open Ordo
          </Link>
        </nav>
      )}
    </header>
  );
}

function publicHref(path: string, role: ProductRole, configuredHomeMode: PublicHomeMode): string {
  const href = configuredHomeMode === "chat" ? `${path}${path.includes("?") ? "&" : "?"}home=chat` : path;
  return roleHref(href, role);
}

function ChatSurfaceSlide({ role }: { role: ProductRole }) {
  return (
    <div className="chat-home-stage">
      <section className="chat-home-brief" aria-labelledby="chat-title">
        <span className="eyebrow">Studio Ordo</span>
        <h1 id="chat-title">Start with the conversation.</h1>
        <p>
          Ask Ordo what this business does, whether the trial fits, or what to look at next. The same surface can show proof, offers,
          asks, jobs, and relationship evidence without exposing staff or system internals.
        </p>
        <div className="feed-proof">
          <span>Ordo-first</span>
          <span>Evidence-backed</span>
          <span>Role-safe</span>
        </div>
        <div className="hero-actions">
          <a href="#about" className="secondary-action">
            Watch the story
          </a>
          <a href="#feed" className="secondary-action">
            See public proof
          </a>
          {role === "anonymous" ? (
            <Link href={roleHref("/my/chat", "client")} className="secondary-action">
              Sign in
            </Link>
          ) : null}
        </div>
      </section>
      <div className="chat-home-panel" aria-label="Relationship chat prototype">
        <OrdoChatPrototype mode="guest" />
      </div>
    </div>
  );
}

function AboutSurfaceSlide({ role }: { role: ProductRole }) {
  return (
    <div className="story-stage">
      <div className="story-card">
        <span className="eyebrow">About</span>
        <h1 id="about-title">A business appliance for relationships, evidence, and production.</h1>
        <p>
          Ordo turns meetings, QR codes, conversations, offers, feedback, affiliate referrals, and factory output into a live operating
          surface. The public story can be cinematic, but the system underneath stays durable and inspectable.
        </p>
        <div className="hero-actions">
          <a href="#chat" className="primary-action">
            Ask Ordo about this
          </a>
          <a href="#feed" className="secondary-action">
            Continue
          </a>
        </div>
      </div>
      <div className="story-media" aria-hidden="true">
        <span>01</span>
      </div>
    </div>
  );
}

function FeedSurfaceSlide({ role }: { role: ProductRole }) {
  return (
    <div className="feed-stage">
      {feedItems.map((item) => (
        <article key={item.id} className="feed-tile">
          <span className="eyebrow">{item.kicker}</span>
          <h2 id={item.id}>{item.title}</h2>
          <p>{item.body}</p>
          <div className="feed-proof">
            {item.proof.map((proof) => (
              <span key={proof}>{proof}</span>
            ))}
          </div>
          <Link href={roleHref(item.href, role)} className="secondary-action">
            {item.action}
          </Link>
        </article>
      ))}
    </div>
  );
}

const surfaceDefinitions = {
  chat: { id: "chat", label: "Full-screen Ordo", shortLabel: "Ordo" },
  about: { id: "about", label: "About story", shortLabel: "About" },
  feed: { id: "feed", label: "Public feed", shortLabel: "Feed" },
} as const;

const feedItems = [
  {
    id: "offer-trial",
    kicker: "Offer",
    title: "Try OrdoStudio for 30 days.",
    body: "A focused trial for solopreneurs who need customer conversations, offers, referrals, and content production to move together.",
    proof: ["No fake urgency", "Trial evidence", "Plain-language fit check"],
    action: "Ask if it fits",
    href: "/chat",
  },
  {
    id: "ask-affiliate",
    kicker: "Ask",
    title: "Send one good person back to us.",
    body: "Tracked QR codes and referral links make useful introductions visible without turning relationships into a spreadsheet.",
    proof: ["Tracked entry", "Referral evidence", "Reward-ready"],
    action: "Open affiliate path",
    href: "/my/affiliate",
  },
  {
    id: "factory-output",
    kicker: "Factory",
    title: "Knowledge turns into artifacts.",
    body: "Articles, short videos, briefs, QR cards, and offer material should come from the knowledgebase and production jobs.",
    proof: ["Knowledge source", "Job stages", "Published artifact"],
    action: "Open Studio",
    href: "/studio/knowledge",
  },
] as const;

function ChatIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path d="M21 15a2 2 0 0 1-2 2H8l-5 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
    </svg>
  );
}

function AboutIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path d="M12 11v6" />
      <path d="M12 7h.01" />
      <path d="M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0z" />
    </svg>
  );
}
