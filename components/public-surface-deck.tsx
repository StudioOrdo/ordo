import Link from "next/link";

import { OrdoFrame } from "@/components/ordo-frame";
import { OrdoChatPrototype } from "@/components/ordo-chat-prototype";
import { ScrollytellingHomepageRuntime } from "@/components/scrollytelling-homepage-runtime";
import { type PublicEntryContext } from "@/lib/public-entry-context";
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
  entryContext?: PublicEntryContext;
  entryPointSlug?: string;
  visitorSessionId?: string;
}

export type PublicHomeMode = "story" | "chat";

export function PublicSurfaceDeck({
  role,
  configuredHomeMode = "story",
  surfaceMode = configuredHomeMode,
  entryContext,
  entryPointSlug,
  visitorSessionId,
}: PublicSurfaceDeckProps) {
  const surfaces = surfaceMode === "chat" ? [surfaceDefinitions.chat] : [surfaceDefinitions.about, surfaceDefinitions.feed];

  return (
    <OrdoFrame
      role={role}
      homeHref={publicHref("/", role, configuredHomeMode)}
      topRail={<PublicTopRail role={role} configuredHomeMode={configuredHomeMode} activeAppSpaceId="site" showBrand={false} />}
    >
      <div className="public-deck-shell" data-role={role} data-home-mode={configuredHomeMode} data-surface-mode={surfaceMode}>
        {surfaceMode === "story" ? (
          <ScrollytellingHomepageRuntime role={role} entryPointSlug={entryPointSlug} visitorSessionId={visitorSessionId} />
        ) : null}

        {surfaceMode === "chat" ? (
          <main className="public-surface-track" aria-label="Studio Ordo surface deck">
            {surfaces.map((surface, index) => (
              <section key={surface.id} id={surface.id} className={`public-surface-slide public-surface-${surface.id}`} aria-label={surface.label}>
                <div className="surface-count">{String(index + 1).padStart(2, "0")} / {String(surfaces.length).padStart(2, "0")}</div>
                {surface.id === "chat" ? <ChatSurfaceSlide role={role} entryContext={entryContext} /> : null}
              </section>
            ))}
          </main>
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

function ChatSurfaceSlide({ role, entryContext }: { role: ProductRole; entryContext?: PublicEntryContext }) {
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
        <OrdoChatPrototype mode="guest" entryContext={entryContext} />
      </div>
    </div>
  );
}

const surfaceDefinitions = {
  chat: { id: "chat", label: "Full-screen Ordo", shortLabel: "Ordo" },
  about: { id: "about", label: "About story", shortLabel: "About" },
  feed: { id: "feed", label: "Public feed", shortLabel: "Feed" },
} as const;

function AboutIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path d="M12 11v6" />
      <path d="M12 7h.01" />
      <path d="M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0z" />
    </svg>
  );
}
