import Link from "next/link";

import { OrdoFrame } from "@/components/ordo-frame";
import { PublicEntrySessionBridge } from "@/components/public-entry-session-bridge";
import { PublicTopRail } from "@/components/public-surface-deck";
import { daemonUrl } from "@/lib/daemon-client";
import { roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { roleHref, type ProductRole } from "@/lib/product-navigation";

type Params = Promise<{ slug: string }>;

interface PublicEntryPointView {
  slug: string;
  label: string;
  destinationSurface: "about" | "offers" | "asks" | "feed";
  destinationId: string | null;
  publicPath: string;
  qrPayload: Record<string, unknown>;
}

export default async function EntryLandingPage({ params, searchParams }: { params: Params; searchParams?: SearchParams }) {
  const [{ slug }, role] = await Promise.all([params, roleFromSearchParams(searchParams)]);
  const resolvedSearchParams: Record<string, string | string[] | undefined> = searchParams ? await searchParams : {};
  const entry = await resolveEntryPoint(slug);
  const locationLabel = firstQueryValue(resolvedSearchParams.locationLabel);
  const locationKind = firstQueryValue(resolvedSearchParams.locationKind);

  return (
    <OrdoFrame
      role={role}
      homeHref={roleHref("/", role)}
      topRail={<PublicTopRail role={role} activeAppSpaceId="site" locationLabel={entry ? "Tracked entry" : "Entry unavailable"} />}
    >
      <main className="public-surface-track" aria-label="Tracked Studio Ordo entry">
        <section className="public-surface-slide public-surface-about" aria-label="Tracked entry landing">
          {entry ? (
            <EntryResolved role={role} entry={entry} locationLabel={locationLabel} locationKind={locationKind} />
          ) : (
            <EntryUnavailable role={role} />
          )}
        </section>
      </main>
    </OrdoFrame>
  );
}

function EntryResolved({
  role,
  entry,
  locationLabel,
  locationKind,
}: {
  role: ProductRole;
  entry: PublicEntryPointView;
  locationLabel?: string;
  locationKind?: string;
}) {
  const destinationHref = destinationHrefFor(entry.destinationSurface, role);

  return (
    <div className="story-stage">
      <div className="story-card">
        <span className="eyebrow">Studio Ordo entry</span>
        <h1>{entry.label}</h1>
        <p>
          This link is a tracked Studio Ordo entry. Ordo will remember this public-safe context if you continue to the story,
          offer, or conversation.
        </p>
        <PublicEntrySessionBridge
          entryPointSlug={entry.slug}
          chatHref={roleHref("/chat", role)}
          destinationHref={destinationHref}
          locationLabel={locationLabel}
          locationKind={locationKind}
        />
      </div>
      <div className="story-media" aria-hidden="true">
        <span>QR</span>
      </div>
    </div>
  );
}

function EntryUnavailable({ role }: { role: ProductRole }) {
  return (
    <div className="story-stage">
      <div className="story-card">
        <span className="eyebrow">Entry unavailable</span>
        <h1>This Studio Ordo entry is not available.</h1>
        <p>The entry may be disabled, archived, or pointed at material that is not public.</p>
        <div className="hero-actions">
          <Link href={roleHref("/", role)} className="primary-action">
            Open the story
          </Link>
          <Link href={roleHref("/chat", role)} className="secondary-action">
            Talk with Ordo
          </Link>
        </div>
      </div>
      <div className="story-media" aria-hidden="true">
        <span>404</span>
      </div>
    </div>
  );
}

async function resolveEntryPoint(slug: string): Promise<PublicEntryPointView | null> {
  try {
    const response = await fetch(`${daemonUrl()}/public/e/${encodeURIComponent(slug)}`, {
      cache: "no-store",
    });
    if (!response.ok) {
      return null;
    }
    return (await response.json()) as PublicEntryPointView;
  } catch {
    return null;
  }
}

function destinationHrefFor(surface: PublicEntryPointView["destinationSurface"], role: ProductRole): string {
  const path = {
    about: "/about",
    offers: "/offers",
    asks: "/asks",
    feed: "/feed",
  }[surface];
  return roleHref(path, role);
}

function firstQueryValue(value: string | string[] | undefined): string | undefined {
  return Array.isArray(value) ? value[0] : value;
}
