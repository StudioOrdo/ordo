"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";

interface PublicEntrySessionBridgeProps {
  entryPointSlug: string;
  chatHref: string;
  destinationHref: string;
  locationLabel?: string;
  locationKind?: string;
}

type SessionState = "starting" | "ready" | "unavailable";
type HandoffState = "idle" | "requesting" | "ready" | "unavailable";

interface RelationshipHandoffStatus {
  state?: string;
  summary?: string;
  nextStep?: string;
  limitations?: string[];
}

export function PublicEntrySessionBridge({
  entryPointSlug,
  chatHref,
  destinationHref,
  locationLabel,
  locationKind,
}: PublicEntrySessionBridgeProps) {
  const storageKey = useMemo(() => `ordo.visitorSession.${entryPointSlug}`, [entryPointSlug]);
  const [sessionId, setSessionId] = useState<string>();
  const [state, setState] = useState<SessionState>("starting");
  const [handoffState, setHandoffState] = useState<HandoffState>("idle");
  const [handoffStatus, setHandoffStatus] = useState<RelationshipHandoffStatus>();
  const chatContextHref = useMemo(
    () => hrefWithEntryContext(chatHref, entryPointSlug, sessionId),
    [chatHref, entryPointSlug, sessionId],
  );
  const destinationContextHref = useMemo(
    () => hrefWithEntryContext(destinationHref, entryPointSlug, sessionId),
    [destinationHref, entryPointSlug, sessionId],
  );

  useEffect(() => {
    let cancelled = false;
    const existingSessionId = readLocalStorage(storageKey);
    const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    const attribution: Record<string, unknown> = {
      source: "public_entry_landing",
      medium: "qr",
      scanOccurredAt: new Date().toISOString(),
      timeZone,
    };

    if (locationLabel) {
      attribution.location = {
        label: locationLabel,
        kind: locationKind ?? "manual",
        source: "query_parameter",
        precision: "manual",
      };
    }

    fetch("/api/public/visitor-sessions", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        entryPointSlug,
        sessionId: existingSessionId,
        userAgent: window.navigator.userAgent,
        attribution,
      }),
    })
      .then(async (response) => {
        if (!response.ok) {
          throw new Error("session_unavailable");
        }
        return response.json() as Promise<{ id?: string }>;
      })
      .then((session) => {
        if (cancelled) {
          return;
        }
        if (session.id) {
          writeLocalStorage(storageKey, session.id);
          writeLocalStorage(
            "ordo.lastVisitorEntry",
            JSON.stringify({
              entryPointSlug,
              visitorSessionId: session.id,
              recordedAt: new Date().toISOString(),
            }),
          );
          setSessionId(session.id);
          setHandoffState("requesting");
          return requestRelationshipHandoff(entryPointSlug, session.id, locationLabel, locationKind)
            .then((status) => {
              if (!cancelled) {
                setHandoffStatus(status);
                setHandoffState("ready");
              }
            })
            .catch(() => {
              if (!cancelled) {
                setHandoffState("unavailable");
              }
            });
        }
        setState("ready");
      })
      .then(() => {
        if (!cancelled) {
          setState("ready");
        }
      })
      .catch(() => {
        if (!cancelled) {
          setState("unavailable");
          setHandoffState("unavailable");
        }
      });

    return () => {
      cancelled = true;
    };
  }, [entryPointSlug, locationKind, locationLabel, storageKey]);

  return (
    <div>
      <div className="feed-proof" aria-live="polite">
        {state === "starting" ? <span>Starting visitor session</span> : null}
        {state === "ready" ? <span>Visitor session recorded</span> : null}
        {state === "unavailable" ? <span>Session unavailable</span> : null}
        <span>No hidden location tracking</span>
        <span>No reward for scan alone</span>
        {handoffState === "requesting" ? <span>Relationship handoff starting</span> : null}
        {handoffState === "ready" ? <span>{handoffStatus?.summary ?? "Relationship handoff queued"}</span> : null}
        {handoffState === "unavailable" ? <span>Relationship handoff unavailable</span> : null}
        {handoffStatus?.nextStep ? <span>{handoffStatus.nextStep}</span> : null}
      </div>
      <div className="hero-actions">
        <Link href={chatContextHref} className="primary-action">
          Talk with Ordo
        </Link>
        <Link href={destinationContextHref} className="secondary-action">
          Continue
        </Link>
      </div>
    </div>
  );
}

async function requestRelationshipHandoff(
  entryPointSlug: string,
  visitorSessionId: string,
  locationLabel?: string,
  locationKind?: string,
): Promise<RelationshipHandoffStatus> {
  const response = await fetch(`/api/public/e/${encodeURIComponent(entryPointSlug)}/relationship-handoff`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      visitorSessionId,
      locationLabel,
      locationKind,
      evidenceRefs: [`tracked_entry_point:${entryPointSlug}`, `visitor_session:${visitorSessionId}`],
    }),
  });
  if (!response.ok) {
    throw new Error("relationship_handoff_unavailable");
  }
  const payload = (await response.json()) as { status?: RelationshipHandoffStatus };
  return payload.status ?? {};
}

function hrefWithEntryContext(href: string, entryPointSlug: string, visitorSessionId?: string): string {
  const url = new URL(href, "https://ordo.local");
  url.searchParams.set("entryPointSlug", entryPointSlug);
  if (visitorSessionId) {
    url.searchParams.set("visitorSessionId", visitorSessionId);
  }
  return `${url.pathname}${url.search}${url.hash}`;
}

function readLocalStorage(key: string): string | undefined {
  try {
    return window.localStorage.getItem(key) ?? undefined;
  } catch {
    return undefined;
  }
}

function writeLocalStorage(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // The durable visitor-session write has already happened on the daemon.
  }
}
