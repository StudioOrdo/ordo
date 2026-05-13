import { ProductShell } from "@/components/product-shell";
import { OrdoChatPrototype } from "@/components/ordo-chat-prototype";
import { eventsForRoom, statusLabel, type MockActivitySpace, type MockActivityStatus, type MockOrdoEvent } from "@/lib/mock-ordo-activity";
import { mobileStepFromSearchParams, railModeFromSearchParams, roleFromSearchParams, selectedItemIndexFromSearchParams, type ProductMobileStep, type ProductRailMode, type SearchParams } from "@/lib/page-role";
import { appSpaceById, roleHref, type ProductAppSpace, type ProductRole } from "@/lib/product-navigation";

interface AppPlaceholderPageProps {
  searchParams?: SearchParams;
  appSpaceId: ProductAppSpace;
  itemId: string;
  eyebrow: string;
  title: string;
  brief: readonly string[];
  facts?: readonly PlaceholderFact[];
}

interface PlaceholderFact {
  label: string;
  value: string;
}

export async function AppPlaceholderPage({
  searchParams,
  appSpaceId,
  itemId,
  eyebrow,
  title,
  brief,
  facts = [],
}: AppPlaceholderPageProps) {
  const role = await roleFromSearchParams(searchParams);
  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams);
  const selectedItemIndex = await selectedItemIndexFromSearchParams(searchParams);
  const screen = shellScreens[appSpaceId]?.[itemId];
  const collapseSectionRail = screen?.layout === "single-chat";
  const selectedIndex = clampSelectedIndex(selectedItemIndex, screen?.main.length ?? 0);

  return (
    <ProductShell
      role={role}
      appSpaceId={appSpaceId}
      currentItemId={itemId}
      roomEvidenceRail={
        screen && !collapseSectionRail ? (
          <MemberRoomEvidenceRail
            screen={screen}
            appSpaceId={appSpaceId}
            selectedIndex={selectedIndex}
            role={role}
            railMode={railMode}
            currentItemId={itemId}
          />
        ) : undefined
      }
      collapseSectionRail={collapseSectionRail}
      railMode={railMode}
      mobileStep={mobileStep}
    >
      {screen ? (
        <MemberMainPage screen={screen} selectedIndex={selectedIndex} />
      ) : (
        <>
          <section className="brief-panel narrative-brief">
            <span className="eyebrow">{eyebrow}</span>
            <h2 className="panel-title">{title}</h2>
            <div className="brief-grid">
              {brief.map((text, index) => (
                <div key={text} className="brief-block">
                  <span>{placeholderLabels[index] ?? "Boundary"}</span>
                  <p>{text}</p>
                </div>
              ))}
            </div>
          </section>
          {facts.length > 0 ? (
            <section className="plain-panel">
              <h3 className="panel-title">Prototype Contract</h3>
              {facts.map((fact) => (
                <div key={fact.label} className="data-row">
                  <span className="label">{fact.label}</span>
                  <span className="value">{fact.value}</span>
                </div>
              ))}
            </section>
          ) : null}
        </>
      )}
    </ProductShell>
  );
}

const placeholderLabels = ["Purpose", "Primary objects", "Evidence", "Deferred"];

interface MemberScreen {
  eyebrow: string;
  title: string;
  summary: string;
  stage: string;
  primaryAction: string;
  railLabel: string;
  railSummary: string;
  layout?: "default" | "single-chat";
  streamStrategy?: "replace" | "prepend";
  filters?: readonly string[];
  evidence: readonly MemberEvidence[];
  main: readonly MemberMainCard[];
}

interface MemberEvidence {
  label: string;
  value: string;
  state: string;
  readState?: "read" | "unread";
  unreadCount?: number;
  action?: string;
  actions?: readonly MemberAction[];
  evidenceRefs?: readonly string[];
  occurredAt?: string;
}

interface MemberMainCard {
  title: string;
  detail: string;
  meta: string;
  action: string;
  secondaryAction?: string;
  actions?: readonly MemberAction[];
  status?: string;
  statusTone?: MemberStatusTone;
  evidenceRefs?: readonly string[];
  sourceLabel?: string;
  timeline?: readonly string[];
  provenance?: readonly string[];
  steps?: readonly string[];
  currentStep?: number;
}

type MemberStatusTone = "attention" | "blocked" | "candidate" | "ok" | "neutral";

type MemberActionKind = "view" | "reply" | "approve" | "reject" | "request_changes" | "mark_read" | "open_source";

interface MemberAction {
  kind: MemberActionKind;
  label: string;
  tone?: "primary" | "secondary" | "danger";
}

function MemberRoomEvidenceRail({
  screen,
  appSpaceId,
  selectedIndex,
  role,
  railMode,
  currentItemId,
}: {
  screen: MemberScreen;
  appSpaceId: ProductAppSpace;
  selectedIndex: number;
  role: ProductRole;
  railMode: ProductRailMode;
  currentItemId: string;
}) {
  return (
    <>
      <div className="section-heading">
        <span className="eyebrow">{screen.railLabel}</span>
        <h1>{screen.title}</h1>
        <p>{screen.railSummary}</p>
      </div>
      <div className="member-stage-card">
        <span>Room brief</span>
        <strong>{screen.stage}</strong>
      </div>
      {screen.filters?.length ? (
        <div className="member-filter-row" aria-label={`${screen.title} filters`}>
          {screen.filters.map((filter, index) => (
            <span key={filter} className={index === 0 ? "member-filter-pill member-filter-pill-active" : "member-filter-pill"}>
              {filter}
            </span>
          ))}
        </div>
      ) : null}
      <nav className="system-menu" aria-label={`${screen.title} evidence and assets`}>
        {groupEvidence(screen.evidence).map((group) =>
          group.items.length ? (
            <div key={group.label} className="section-link-group">
              <span className="section-link-group-label">{group.label}</span>
              {group.items.map(({ item, originalIndex }) => (
                <a
                  key={`${item.label}-${item.value}`}
                  href={selectedRoomItemHref(appSpaceId, currentItemId, role, railMode, originalIndex, "content")}
                  className={`section-link ${originalIndex === selectedIndex ? "section-link-active" : ""} ${item.readState === "unread" ? "section-link-unread" : ""}`}
                  aria-current={originalIndex === selectedIndex ? "true" : undefined}
                >
                  <span>
                    <strong>{item.value}</strong>
                    <span>{item.state}</span>
                    <small className="section-link-meta">
                      {item.label}
                      {item.occurredAt ? ` · ${timeLabel(item.occurredAt)}` : ""}
                    </small>
                  </span>
                  <span className="section-link-control">
                    {item.unreadCount ? (
                      <span className="section-unread-count" aria-label={`${item.unreadCount} unread`}>
                        {item.unreadCount}
                      </span>
                    ) : (
                      <span className="link-dot" aria-label={item.readState === "read" ? "Read" : "Unread"} />
                    )}
                    <span className="section-link-action-button" aria-label={`${primaryActionForEvidence(item).label} ${item.value}`}>
                      {primaryActionForEvidence(item).label}
                    </span>
                  </span>
                </a>
              ))}
            </div>
          ) : null,
        )}
      </nav>
    </>
  );
}

function MemberMainPage({ screen, selectedIndex }: { screen: MemberScreen; selectedIndex: number }) {
  if (screen.layout === "single-chat") {
    return <OrdoChatPrototype mode="member" />;
  }

  const selectedCard = screen.main[selectedIndex] ?? screen.main[0];

  return (
    <section className="member-main-content" aria-label={`${screen.title} main content`}>
      {selectedCard ? <SelectedEvidenceStage card={selectedCard} selectedIndex={selectedIndex} /> : null}
    </section>
  );
}

function SelectedEvidenceStage({ card, selectedIndex }: { card: MemberMainCard; selectedIndex: number }) {
  return (
    <article key={card.title} id={roomItemAnchor(selectedIndex)} className="member-evidence-stage">
      <section className="member-stage-summary-band" aria-label={`${card.title} Ordo summary`}>
        <div className="member-stage-kicker-row">
          <span>Ordo summary</span>
          <span>{card.meta}</span>
          {card.status ? <span className={`member-card-status member-card-status-${card.statusTone ?? "neutral"}`}>{card.status}</span> : null}
        </div>
        <h2>{card.title}</h2>
        <p>{card.detail}</p>
        <p className="member-stage-recommendation">
          <strong>Recommended next action</strong>
          <span>{actionsForCard(card)[0]?.label ?? card.action}</span>
        </p>
        <div className="member-stage-action-row">
          {actionsForCard(card).map((action) => (
            <button
              key={`${action.kind}-${action.label}`}
              type="button"
              className={action.tone === "secondary" ? "member-live-card-action member-live-card-secondary-action" : "member-live-card-action"}
            >
              {action.label}
            </button>
          ))}
        </div>
      </section>

      {card.evidenceRefs?.length ? (
        <section className="member-stage-section" aria-label={`${card.title} evidence references`}>
          <h3>Evidence</h3>
          <div className="member-evidence-ref-row">
            {card.evidenceRefs.slice(0, 5).map((ref) => (
              <span key={ref}>{ref}</span>
            ))}
          </div>
        </section>
      ) : null}

      {card.steps?.length ? (
        <section className="member-stage-section">
          <h3>Progress</h3>
          <ol className="member-job-steps" aria-label={`${card.title} stages`}>
            {card.steps.map((step, stepIndex) => {
              const state = stepIndex < (card.currentStep ?? 0) ? "done" : stepIndex === (card.currentStep ?? 0) ? "current" : "pending";
              return (
                <li key={step} className={`member-job-step member-job-step-${state}`}>
                  <span className="member-job-step-dot" aria-hidden="true" />
                  <span>{step}</span>
                </li>
              );
            })}
          </ol>
        </section>
      ) : null}

      <section className="member-stage-section" aria-label={`${card.title} timeline`}>
        <h3>Timeline</h3>
        <ol className="member-timeline-list">
          {timelineEntriesForCard(card).map((entry) => (
            <li key={`${entry.time}-${entry.label}`}>
              <span className="member-timeline-dot" aria-hidden="true" />
              <span className="member-timeline-time">{entry.time}</span>
              <strong>{entry.label}</strong>
              <p>{entry.body}</p>
            </li>
          ))}
        </ol>
      </section>

      <section className="member-stage-section member-provenance-section" aria-label={`${card.title} provenance`}>
        <h3>Provenance</h3>
        <ul className="member-provenance-list">
          {provenanceForCard(card).map((entry) => (
            <li key={entry}>{entry}</li>
          ))}
        </ul>
      </section>
    </article>
  );
}

function clampSelectedIndex(index: number, itemCount: number): number {
  if (itemCount <= 0) {
    return 0;
  }
  return Math.min(Math.max(index, 0), itemCount - 1);
}

function selectedRoomItemHref(
  appSpaceId: ProductAppSpace,
  currentItemId: string,
  role: ProductRole,
  railMode: ProductRailMode,
  itemIndex: number,
  mobileStep: ProductMobileStep,
): string {
  const appSpace = appSpaceById(appSpaceId);
  const baseHref = appSpace.items.find((item) => item.id === currentItemId)?.href ?? appSpace.href;
  const href = roleHref(baseHref, role);
  const url = new URL(href, "https://ordo.local");
  url.searchParams.set("item", String(itemIndex));
  if (railMode === "collapsed") {
    url.searchParams.set("rail", "collapsed");
  }
  if (mobileStep !== "rooms") {
    url.searchParams.set("mobile", mobileStep);
  }
  const query = url.searchParams.toString();
  return `${url.pathname}${query ? `?${query}` : ""}`;
}

function roomItemAnchor(index: number): string {
  return `room-item-${index}`;
}

function groupEvidence(evidence: readonly MemberEvidence[]): readonly {
  label: string;
  items: readonly { item: MemberEvidence; originalIndex: number }[];
}[] {
  const indexed = evidence.map((item, originalIndex) => ({ item, originalIndex }));

  return [
    {
      label: "Needs action",
      items: indexed.filter(({ item }) => item.readState === "unread"),
    },
    {
      label: "Recent",
      items: indexed.filter(({ item }) => item.readState !== "unread"),
    },
  ];
}

function streamEnhanceScreens(space: MockActivitySpace, screens: Record<string, MemberScreen>): Record<string, MemberScreen> {
  return Object.fromEntries(
    Object.entries(screens).map(([room, screen]) => {
      const events = eventsForRoom(space, room);
      if (!events.length || screen.layout === "single-chat") {
        return [room, screen];
      }

      return [
        room,
        {
          ...screen,
          evidence: screen.streamStrategy === "replace" ? events.map(eventToEvidence) : [...events.map(eventToEvidence), ...screen.evidence.map(asBaselineEvidence)],
          main: screen.streamStrategy === "replace" ? events.map(eventToMainCard) : [...events.map(eventToMainCard), ...screen.main],
        },
      ];
    }),
  );
}

function asBaselineEvidence(item: MemberEvidence): MemberEvidence {
  return {
    ...item,
    readState: item.readState === "unread" ? "read" : item.readState,
    unreadCount: undefined,
  };
}

function eventToEvidence(event: MockOrdoEvent): MemberEvidence {
  const needsAttention = event.status === "unread" || event.status === "waiting_on_you";

  return {
    label: event.kind,
    value: event.title,
    state: event.summary,
    readState: needsAttention ? "unread" : "read",
    unreadCount: needsAttention ? 1 : undefined,
    action: event.action,
    actions: actionsForEvent(event),
    evidenceRefs: event.evidenceRefs,
    occurredAt: event.occurredAt,
  };
}

function eventToMainCard(event: MockOrdoEvent): MemberMainCard {
  return {
    meta: event.kind,
    title: event.title,
    detail: event.summary,
    action: event.action,
    secondaryAction: event.secondaryAction,
    actions: actionsForEvent(event),
    status: statusLabel(event.status),
    statusTone: statusToneForEventStatus(event.status),
    evidenceRefs: event.evidenceRefs,
    sourceLabel: event.sourceLabel,
    timeline: timelineForEvent(event),
    provenance: provenanceForEvent(event),
  };
}

function actionsForEvent(event: MockOrdoEvent): readonly MemberAction[] {
  const primary = actionForLabel(event.action);
  const secondary = event.secondaryAction ? actionForLabel(event.secondaryAction, "secondary") : undefined;
  const utility: MemberAction = event.status === "unread" ? { kind: "mark_read", label: "Mark read", tone: "secondary" } : { kind: "view", label: "View", tone: "secondary" };
  return secondary ? [primary, secondary, utility] : [primary, utility];
}

function actionForLabel(label: string, tone: MemberAction["tone"] = "primary"): MemberAction {
  const normalized = label.toLowerCase();
  if (normalized.includes("approve")) {
    return { kind: "approve", label, tone };
  }
  if (normalized.includes("request changes") || normalized.includes("revise")) {
    return { kind: "request_changes", label, tone };
  }
  if (normalized.includes("reply") || normalized.includes("respond")) {
    return { kind: "reply", label, tone };
  }
  if (normalized.includes("reject")) {
    return { kind: "reject", label, tone: "danger" };
  }
  if (normalized.includes("source") || normalized.includes("evidence")) {
    return { kind: "open_source", label, tone };
  }
  return { kind: "view", label, tone };
}

function actionsForCard(card: MemberMainCard): readonly MemberAction[] {
  if (card.actions?.length) {
    return card.actions;
  }

  const primary = actionForLabel(card.action);
  const secondary = card.secondaryAction ? actionForLabel(card.secondaryAction, "secondary") : undefined;
  return secondary ? [primary, secondary] : [primary];
}

function primaryActionForEvidence(item: MemberEvidence): MemberAction {
  return item.actions?.[0] ?? actionForLabel(item.action ?? "View");
}

function timelineForEvent(event: MockOrdoEvent): readonly string[] {
  return [
    `${timeLabel(event.occurredAt)}: ${event.sourceLabel} recorded ${event.kind} evidence.`,
    `${statusLabel(event.status)}: Ordo projected the item into ${event.rooms.join(", ")}.`,
    `Current step: ${event.action}.`,
  ];
}

function provenanceForEvent(event: MockOrdoEvent): readonly string[] {
  return [`Source: ${event.sourceLabel}`, ...event.evidenceRefs.map((ref) => `Evidence: ${ref}`)];
}

function provenanceForCard(card: MemberMainCard): readonly string[] {
  if (card.provenance?.length) {
    return card.provenance;
  }

  const refs = card.evidenceRefs?.length ? card.evidenceRefs.map((ref) => `Evidence: ${ref}`) : ["Evidence: mock room projection"];
  return card.sourceLabel ? [`Source: ${card.sourceLabel}`, ...refs] : refs;
}

function timelineEntriesForCard(card: MemberMainCard): readonly { time: string; label: string; body: string }[] {
  const timeline = card.timeline?.length
    ? card.timeline
    : [`Now: ${card.meta} evidence selected.`, `Status: ${card.status ?? "ready"}.`, `Current step: ${actionsForCard(card)[0]?.label ?? card.action}.`];

  return timeline.map((entry, index) => {
    const [maybeTime, ...rest] = entry.split(": ");
    const hasTime = rest.length > 0;
    const body = hasTime ? rest.join(": ") : entry;
    const [label, ...detail] = body.split(". ");
    return {
      time: hasTime ? maybeTime : index === 0 ? "Now" : `Step ${index + 1}`,
      label: label.replace(/\.$/, ""),
      body: detail.join(". ") || body,
    };
  });
}

function timeLabel(isoTimestamp: string): string {
  const date = new Date(isoTimestamp);
  if (Number.isNaN(date.getTime())) {
    return isoTimestamp;
  }
  return new Intl.DateTimeFormat("en-US", {
    timeZone: "America/New_York",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(date);
}

function statusToneForEventStatus(status: MockActivityStatus): MemberStatusTone {
  switch (status) {
    case "unread":
    case "waiting_on_you":
      return "attention";
    case "blocked":
      return "blocked";
    case "candidate":
    case "waiting_on_ordo":
      return "candidate";
    case "active":
    case "ready":
    case "scheduled":
    case "done":
      return "ok";
    default:
      return "neutral";
  }
}

const userScreens: Record<string, MemberScreen> = {
  activity: {
    eyebrow: "Ordo",
    title: "Activity",
    summary: "A live action feed of what changed, what is waiting, and where to go next. Items link back to offers, capabilities, requests, referrals, or Ordo.",
    stage: "Waiting on you",
    primaryAction: "Review what needs attention.",
    railLabel: "Attention",
    railSummary: "Unread changes and action items filtered from user-safe evidence.",
    streamStrategy: "replace",
    evidence: [
      { label: "Activity", value: "Choose a path", state: "Offer decision from meetup QR", readState: "unread", unreadCount: 1 },
      { label: "Activity", value: "Ava asked Ordo", state: "Trial or consultation fit", readState: "unread", unreadCount: 1 },
      { label: "Activity", value: "Feedback request", state: "Private feedback waiting", readState: "unread", unreadCount: 1 },
      { label: "Activity", value: "Affiliate terms", state: "Needed before referral tools", readState: "unread", unreadCount: 1 },
      { label: "Activity", value: "Meetup QR", state: "Scanned at event" },
    ],
    main: [
      { meta: "Offer", title: "Choose the right Studio Ordo path", detail: "You came in from a meetup QR code. Pick consultation, trial, affiliate partner, or training access.", action: "Open offers" },
      { meta: "Ordo", title: "Ava asked which path fits", detail: "Answer in the primary relationship conversation without exposing internal routing or provider details.", action: "Open Ordo" },
      { meta: "Request", title: "Share private feedback", detail: "Feedback can stay private unless consent and approval make it public.", action: "Review" },
      { meta: "Referral", title: "Affiliate path available", detail: "Accept terms before tracked links, QR codes, outcomes, or rewards are active.", action: "Review" },
    ],
  },
  chat: {
    eyebrow: "Ordo",
    title: "Ordo",
    summary: "One relationship conversation for every path. Ordo answers first; Keith or staff can take over without making you manage channels.",
    stage: "Relationship active",
    primaryAction: "Talk with Studio Ordo.",
    railLabel: "Conversation",
    railSummary: "One primary relationship thread.",
    layout: "single-chat",
    evidence: [
      { label: "Messages", value: "Primary thread", state: "Ordo plus staff handoff", readState: "unread", unreadCount: 3 },
      { label: "Messages", value: "Offer questions", state: "consultation, trial, affiliate, or training", readState: "unread", unreadCount: 1 },
      { label: "Messages", value: "Capability support", state: "same relationship" },
      { label: "Messages", value: "Student tutoring", state: "enabled by training access" },
    ],
    main: [
      { meta: "Primary", title: "A user asked which path fits", detail: "Answer from offer evidence, not pressure language.", action: "Open" },
      { meta: "Handoff", title: "Staff can take over inside the same conversation", detail: "The user sees continuity, not internal routing.", action: "Open" },
      { meta: "Tutoring", title: "Training questions stay in the same relationship", detail: "Student tools can be enabled by an accepted training offer.", action: "Open" },
      { meta: "Boundary", title: "Staff-only notes stay hidden", detail: "Private guidance belongs in staff views, never on the user screen.", action: "Review" },
    ],
  },
  offers: {
    eyebrow: "Ordo",
    title: "Offers",
    summary: "Offers are paths you can accept. Once accepted, they become capabilities.",
    stage: "Offer selection",
    primaryAction: "Choose what you want from Studio Ordo.",
    railLabel: "Offers",
    railSummary: "Available paths that can become capabilities.",
    streamStrategy: "prepend",
    evidence: [
      { label: "Offer", value: "Strategic consultation", state: "Meetup QR path", readState: "unread", unreadCount: 1 },
      { label: "Offer", value: "30-day Ordo trial", state: "Available" },
      { label: "Offer", value: "Training access", state: "Student path" },
      { label: "Offer", value: "Affiliate partner", state: "Requires terms" },
    ],
    main: [
      { meta: "Consultation", title: "Strategic consultation", detail: "A focused business conversation after meeting Keith at an event.", action: "View" },
      { meta: "Trial", title: "30-day Ordo trial", detail: "A practical test around your real conversations, offers, requests, content, and referrals.", action: "View" },
      { meta: "Training", title: "Training access", detail: "A student path for tutoring, assignment feedback, resources, and progress help.", action: "View" },
      { meta: "Affiliate", title: "Affiliate partner", detail: "A promotion path with tracked links, QR codes, introductions, outcomes, and rewards.", action: "View" },
    ],
  },
  asks: {
    eyebrow: "Ordo",
    title: "Requests",
    summary: "Requests are tracked asks that need a decision, response, approval, review, or resolution. They can come from Ordo, Studio Ordo, staff, or you.",
    stage: "2 waiting on you",
    primaryAction: "Review the selected request.",
    railLabel: "Request queue",
    railSummary: "Simple cards for approvals, feedback asks, meetings, support, and capability-enabled work.",
    streamStrategy: "replace",
    filters: ["All", "Waiting on you", "Waiting on Studio Ordo", "Done"],
    evidence: [
      { label: "Waiting on you", value: "Approve QR card proof", state: "Trial capability · before print", readState: "unread", unreadCount: 1 },
      { label: "Waiting on you", value: "Pick consultation time", state: "Strategic consultation · meeting" },
      { label: "Waiting on you", value: "Private feedback request", state: "Trial follow-up · optional", readState: "unread", unreadCount: 1 },
      { label: "Waiting on Studio Ordo", value: "Trial extension decision", state: "Hosted trial · support" },
      { label: "Waiting on Studio Ordo", value: "Assignment feedback", state: "Training capability · review" },
    ],
    main: [
      { meta: "Selected request", title: "Approve QR card proof", detail: "Studio Ordo needs approval before the QR proof is used for printed cards or event material. Related evidence: trial offer, meetup QR entry, generated proof artifact, and the relationship conversation.", action: "Approve" },
      { meta: "Alternative", title: "Request changes", detail: "Ask Ordo or staff to revise the proof without creating a separate support channel.", action: "Revise" },
      { meta: "Meeting", title: "Pick consultation time", detail: "Choose a time or ask Ordo to coordinate with Keith. The request stays linked to the same relationship conversation.", action: "Schedule" },
      { meta: "Feedback", title: "Private feedback request", detail: "Private feedback is business intelligence. It is not a public review unless consent and approval happen later.", action: "Respond" },
    ],
  },
  packs: {
    eyebrow: "Ordo",
    title: "Capabilities",
    summary: "Accepted offers become capabilities. A capability explains what you can use now: owner access, tools, content, training, trial workspace, affiliate features, or combinations.",
    stage: "Capabilities",
    primaryAction: "Open an accepted capability.",
    railLabel: "Capabilities",
    railSummary: "Accepted offers, receipts, enabled tools, content, and service access.",
    streamStrategy: "prepend",
    evidence: [
      { label: "Capability", value: "Strategic consultation", state: "Unlocked after acceptance" },
      { label: "Capability", value: "30-day Ordo trial", state: "Unlocked after acceptance" },
      { label: "Capability", value: "Training", state: "Unlocked after acceptance" },
      { label: "Capability", value: "Affiliate partner", state: "Unlocked after terms" },
    ],
    main: [
      { meta: "Consultation", title: "Strategic consultation capability", detail: "Owner access, consultation prep, business context, scheduling requests, and recommendation review.", action: "Open" },
      { meta: "Trial", title: "30-day Ordo trial capability", detail: "Hosted trial workspace, reset or extension requests, relationship conversation, offer context, QR tracking, and referrals.", action: "Open" },
      { meta: "Training", title: "Training capability", detail: "Tutoring, assignment feedback, lesson resources, progress help, and student requests.", action: "Open" },
      { meta: "Affiliate", title: "Affiliate partner capability", detail: "Tracked links, event QR codes, introductions, outcome tracking, and rewards.", action: "Open" },
    ],
  },
  affiliate: {
    eyebrow: "Ordo",
    title: "Referrals",
    summary: "Referrals are for accepted or invited affiliates. They show links, QR codes, introductions, outcomes, and rewards.",
    stage: "Affiliate path",
    primaryAction: "Use the affiliate partner capability.",
    railLabel: "Referral evidence",
    railSummary: "Affiliate status, referral assets, outcomes, and rewards.",
    streamStrategy: "prepend",
    evidence: [
      { label: "Referral", value: "Affiliate status", state: "Requires accepted terms" },
      { label: "Referral", value: "My link", state: "Created after acceptance" },
      { label: "Referral", value: "My QR", state: "Created after acceptance" },
      { label: "Referral", value: "Introductions", state: "Tracked when accepted" },
      { label: "Referral", value: "Rewards", state: "Evidence-backed" },
    ],
    main: [
      { meta: "Status", title: "Accept affiliate terms first", detail: "Links and reward claims stay hidden until terms are accepted.", action: "Review" },
      { meta: "Link", title: "Share a tracked Studio Ordo link", detail: "Create a tracked entry point and visitor session for people you send back.", action: "View" },
      { meta: "QR", title: "Use an event QR code", detail: "Use at meetups, events, and in-person introductions.", action: "View" },
      { meta: "Outcome", title: "Track successful referrals", detail: "Credit cites entry point, session, offer, and acceptance evidence.", action: "View" },
      { meta: "Boundary", title: "See only your referral evidence", detail: "Affiliate-safe visibility hides unrelated customers, staff notes, and internals.", action: "Review" },
    ],
  },
  account: {
    eyebrow: "Ordo",
    title: "Account",
    summary: "Identity, access, and security controls for a user without exposing staff or system internals.",
    stage: "Authenticated",
    primaryAction: "Review your account and access state.",
    railLabel: "Account",
    railSummary: "Identity, access, and security evidence.",
    evidence: [
      { label: "Identity", value: "Signed in", state: "mock auth session" },
      { label: "Access", value: "User shell", state: "role-constrained" },
      { label: "Security", value: "Password reset", state: "planned account action" },
    ],
    main: [
      { meta: "Profile", title: "Name and contact preferences", detail: "Production forms should become command-backed account actions.", action: "Edit" },
      { meta: "Security", title: "Password and recovery", detail: "Password reset belongs here and should preserve the shared top rail.", action: "Manage" },
      { meta: "Access", title: "Shell access", detail: "Users can enter user-safe areas; staff, studio, owner, and system remain role-gated.", action: "View" },
    ],
  },
  preferences: {
    eyebrow: "Ordo",
    title: "Preferences",
    summary: "Accessible experience settings saved as requested preferences and resolved safely by role.",
    stage: "Personalized",
    primaryAction: "Tune the interface without unlocking internals.",
    railLabel: "Preferences",
    railSummary: "Saved experience and accessibility settings.",
    evidence: [
      { label: "Font size", value: "Saved", state: "account preference" },
      { label: "Motion", value: "Reduced motion", state: "equivalent status retained" },
      { label: "Privacy display", value: "Role-constrained", state: "cannot reveal owner internals" },
    ],
    main: [
      { meta: "Accessibility", title: "Type, contrast, and motion", detail: "Settings should be simple to trust and easy to change.", action: "Edit" },
      { meta: "Color", title: "Color-blind modes", detail: "Status information must not depend on hue alone.", action: "Edit" },
      { meta: "Safety", title: "Effective settings", detail: "Requested settings are re-resolved before rendering.", action: "Review" },
    ],
  },
};

userScreens.access = userScreens.packs;

const staffScreens: Record<string, MemberScreen> = {
  today: {
    eyebrow: "Support",
    title: "Today",
    summary: "Support sees people, handoffs, trial setup, feedback, and customer work that needs human judgment.",
    stage: "Customer work",
    primaryAction: "Resolve customer-facing work.",
    railLabel: "Support attention",
    railSummary: "Relationship evidence and customer-safe next actions.",
    evidence: [
      { label: "Staff", value: "Maya Patel", state: "meetup QR visitor wants Keith live", readState: "unread", unreadCount: 1 },
      { label: "Staff", value: "Consultation", state: "Marcus Chen needs reminder" },
      { label: "Staff", value: "Feedback", state: "Ava Thompson gave private trial feedback", readState: "unread", unreadCount: 1 },
      { label: "Staff", value: "Handoff", state: "Ordo asked for human review" },
    ],
    main: [
      { meta: "Trial", title: "Join Maya's trial signup thread", detail: "A meetup QR visitor is ready to start the 30-day trial and asked for Keith while he is online.", action: "Take over" },
      { meta: "Consultation", title: "Send consultation reminder", detail: "Keith meets Marcus tomorrow; ask for prep notes without fake urgency.", action: "Send" },
      { meta: "Feedback", title: "Review Ava's private feedback", detail: "Feedback can inform product work, but it is not public proof without consent.", action: "Review" },
    ],
  },
  conversations: {
    eyebrow: "Support",
    title: "Conversations",
    summary: "Handoff conversations and customer threads that need staff judgment.",
    stage: "Handoff queue",
    primaryAction: "Open the selected customer conversation.",
    railLabel: "Conversations",
    railSummary: "Second column lists staff handoffs and active customer threads.",
    streamStrategy: "replace",
    evidence: [
      { label: "Conversation", value: "Maya asked for Keith live", state: "handoff waiting", readState: "unread", unreadCount: 1 },
      { label: "Conversation", value: "Consultation prep", state: "scheduled" },
    ],
    main: [
      {
        meta: "Handoff",
        title: "Maya asked to talk to Keith live",
        detail: "Maya met Keith at the meetup, scanned the QR code, and is ready to start the 30-day trial after a quick human answer.",
        action: "Take over",
        secondaryAction: "Assign",
        status: "unread",
        statusTone: "attention",
        evidenceRefs: ["conversation:maya-patel", "entry:meetup-qr", "offer:30-day-trial"],
      },
      {
        meta: "Consultation",
        title: "Consultation prep",
        detail: "A strategic consultation is scheduled tomorrow. Staff can send a prep reminder and request business context.",
        action: "Open thread",
        status: "scheduled",
        statusTone: "neutral",
        evidenceRefs: ["connection:marcus-chen", "offer:strategic-consultation", "calendar:consultation"],
      },
    ],
  },
  requests: {
    eyebrow: "Support",
    title: "Requests",
    summary: "Customer requests that need a support decision, owner answer, or safe handoff.",
    stage: "2 waiting",
    primaryAction: "Resolve requests without splitting the relationship.",
    railLabel: "Support requests",
    railSummary: "Approvals, help requests, feedback asks, and scheduled follow-ups.",
    streamStrategy: "replace",
    evidence: [
      { label: "Request", value: "Maya wants Keith live", state: "handoff waiting", readState: "unread", unreadCount: 1 },
      { label: "Request", value: "Approve trial extension", state: "owner answer needed", readState: "unread", unreadCount: 1 },
      { label: "Request", value: "Student assignment feedback", state: "training capability active" },
    ],
    main: [
      {
        meta: "Handoff",
        title: "Maya asked to talk to Keith",
        detail: "A meetup QR visitor wants a human answer before accepting the 30-day trial. Support can take over, assign Keith, or ask Ordo to summarize context.",
        action: "Take over",
        secondaryAction: "Ask Ordo",
        status: "waiting on staff",
        statusTone: "attention",
        evidenceRefs: ["conversation:maya-patel", "entry:meetup-qr", "offer:30-day-trial"],
      },
      {
        meta: "Trial",
        title: "Trial extension policy needed",
        detail: "Hosted setup time should not count against the user's trial if the delay came from Studio Ordo setup work.",
        action: "Escalate",
        status: "waiting on owner",
        statusTone: "candidate",
        evidenceRefs: ["trial:30-day", "request:extension", "conversation:maya-patel"],
      },
      {
        meta: "Training",
        title: "Student assignment feedback",
        detail: "A training user asked for feedback on a draft. The answer should stay tied to the training capability and source materials.",
        action: "Open",
        status: "active",
        statusTone: "ok",
        evidenceRefs: ["capability:training", "artifact:assignment-draft"],
      },
    ],
  },
  connections: {
    eyebrow: "Support",
    title: "Connections",
    summary: "Relationship rooms for prospects, trial users, students, affiliates, and customer conversations.",
    stage: "Relationship stages",
    primaryAction: "Inspect active relationships.",
    railLabel: "Connections",
    railSummary: "People and companies with evidence-backed stages.",
    evidence: [
      { label: "Connection", value: "Maya Patel", state: "trial signup handoff" },
      { label: "Connection", value: "Marcus Chen", state: "consultation scheduled" },
      { label: "Connection", value: "Ava Thompson", state: "affiliate candidate" },
      { label: "Connection", value: "Priya Shah", state: "training student" },
    ],
    main: [
      { meta: "Trial", title: "Maya Patel", detail: "Meetup QR lead ready to try Ordo after Keith answers one live question.", action: "Open" },
      { meta: "Consult", title: "Marcus Chen", detail: "Booked a strategic consultation and needs prep reminders.", action: "Open" },
      { meta: "Training", title: "Priya Shah", detail: "Interested in training access and assignment feedback workflows.", action: "Open" },
    ],
  },
  members: {
    eyebrow: "Support",
    title: "Members",
    summary: "People in the relationship system: meetup visitors, trial users, students, affiliates, and customers.",
    stage: "Relationship roster",
    primaryAction: "Open the member with the safest next action.",
    railLabel: "Member evidence",
    railSummary: "Stages, live handoffs, offer state, and safe contact boundaries.",
    streamStrategy: "replace",
    evidence: [
      { label: "Member", value: "Maya Patel", state: "meetup QR · trial decision", readState: "unread", unreadCount: 1 },
      { label: "Member", value: "Marcus Chen", state: "consultation scheduled" },
      { label: "Member", value: "Priya Shah", state: "training student" },
      { label: "Member", value: "Ava Thompson", state: "affiliate candidate" },
    ],
    main: [
      {
        meta: "Trial",
        title: "Maya Patel",
        detail: "Meetup QR visitor considering the 30-day Ordo trial after asking whether consultation or trial should come first.",
        action: "Open member",
        status: "handoff waiting",
        statusTone: "attention",
        evidenceRefs: ["connection:maya-patel", "entry:meetup-qr", "conversation:maya-patel"],
      },
      {
        meta: "Consult",
        title: "Marcus Chen",
        detail: "Booked a strategic consultation and needs a prep reminder before the call.",
        action: "Open member",
        status: "scheduled",
        statusTone: "ok",
        evidenceRefs: ["connection:marcus-chen", "calendar:consultation"],
      },
      {
        meta: "Training",
        title: "Priya Shah",
        detail: "Student path active; assignment feedback and lesson resources should stay in the training capability.",
        action: "Open member",
        status: "active",
        statusTone: "ok",
        evidenceRefs: ["connection:priya-shah", "capability:training"],
      },
    ],
  },
  pipeline: {
    eyebrow: "Support",
    title: "Pipeline",
    summary: "Simple stages for business relationships: waitlist, consultation, trial, student, customer, affiliate, advocate.",
    stage: "18 active relationships",
    primaryAction: "Move people through clear stages.",
    railLabel: "Pipeline evidence",
    railSummary: "Stage changes should cite conversations, offers, or outcomes.",
    evidence: [
      { label: "Stage", value: "Waitlist", state: "6 people" },
      { label: "Stage", value: "Consultation", state: "3 scheduled" },
      { label: "Stage", value: "Trial", state: "4 active" },
      { label: "Stage", value: "Training", state: "5 students" },
    ],
    main: [
      { meta: "Waitlist", title: "Invite two waitlist users", detail: "Capacity is available for two hosted 30-day trials this week.", action: "Review" },
      { meta: "Trial", title: "Convert Maya into the 30-day trial", detail: "Handoff should preserve QR source, conversation evidence, and the offer acceptance path.", action: "Request" },
      { meta: "Student", title: "Training cohort starts Monday", detail: "Five students need onboarding and assignment feedback instructions.", action: "Open" },
    ],
  },
  handoffs: {
    eyebrow: "Support",
    title: "Handoffs",
    summary: "Human takeover points for customer conversations where Ordo should pause or ask for review.",
    stage: "2 waiting",
    primaryAction: "Take over only when useful.",
    railLabel: "Handoff queue",
    railSummary: "Customer-visible continuity, staff-only mechanics.",
    evidence: [
      { label: "Handoff", value: "Maya wants Keith live", state: "waiting on staff", readState: "unread", unreadCount: 1 },
      { label: "Handoff", value: "Consult scope", state: "needs Keith" },
      { label: "Handoff", value: "Feedback consent", state: "policy boundary" },
    ],
    main: [
      { meta: "Trial", title: "Take over Maya's trial signup handoff", detail: "A meetup QR visitor is ready to start the 30-day trial and asked for Keith while he is online.", action: "Take over" },
      { meta: "Consult", title: "Keith should answer scope", detail: "Strategic advice should come from human judgment, with Ordo summarizing evidence.", action: "Assign" },
    ],
  },
  feedback: {
    eyebrow: "Support",
    title: "Feedback",
    summary: "Private intelligence from trials, training, consultations, and support conversations.",
    stage: "Private by default",
    primaryAction: "Review feedback without turning it into public proof.",
    railLabel: "Feedback queue",
    railSummary: "Consent and approval are separate states.",
    evidence: [
      { label: "Feedback", value: "Trial onboarding friction", state: "private" },
      { label: "Feedback", value: "Training assignment value", state: "candidate review" },
      { label: "Feedback", value: "Affiliate concern", state: "needs policy answer" },
    ],
    main: [
      { meta: "Trial", title: "Hosted trial setup felt slow", detail: "Use this to improve onboarding; do not publish as a review.", action: "Triage" },
      { meta: "Training", title: "Student praised feedback loop", detail: "Can ask for consent after the cohort completes.", action: "Prepare" },
    ],
  },
  reviews: {
    eyebrow: "Support",
    title: "Reviews",
    summary: "Review candidates require consent, approval, and publication governance.",
    stage: "Consent boundary",
    primaryAction: "Approve only evidence-backed reviews.",
    railLabel: "Review candidates",
    railSummary: "Feedback is not public proof until governed.",
    evidence: [
      { label: "Review", value: "Student outcome quote", state: "needs consent" },
      { label: "Review", value: "Trial testimonial", state: "draft candidate" },
      { label: "Review", value: "Affiliate story", state: "not approved" },
    ],
    main: [
      { meta: "Consent", title: "Ask student for review consent", detail: "The quote references assignment feedback and should remain private until confirmed.", action: "Ask" },
      { meta: "Approval", title: "Decline unsupported proof", detail: "Reject claims that cannot cite durable outcomes.", action: "Decline" },
    ],
  },
  affiliates: {
    eyebrow: "Support",
    title: "Affiliates",
    summary: "Referral partners, scoped grants, links, QR codes, and reward evidence.",
    stage: "Referral operations",
    primaryAction: "Support affiliates without leaking customer internals.",
    railLabel: "Affiliate evidence",
    railSummary: "Links, QR scans, introductions, outcomes, and rewards.",
    evidence: [
      { label: "Affiliate", value: "Ava Thompson", state: "terms pending" },
      { label: "Affiliate", value: "Meetup QR batch", state: "12 scans" },
      { label: "Affiliate", value: "Reward review", state: "2 pending" },
    ],
    main: [
      { meta: "Terms", title: "Ava can become an affiliate", detail: "Provide terms before generating trackable links or reward claims.", action: "Send" },
      { meta: "Reward", title: "Review two successful referrals", detail: "Credit must cite entry point, session, offer, and acceptance evidence.", action: "Review" },
    ],
  },
};

const studioScreens: Record<string, MemberScreen> = {
  knowledge: {
    eyebrow: "Studio",
    title: "Knowledge",
    summary: "Business truth, lectures, source material, transcripts, proof, and reusable expertise.",
    stage: "Source library",
    primaryAction: "Manage the knowledge that production can trust.",
    railLabel: "Knowledge sources",
    railSummary: "Raw material that can become content, training, or offer support.",
    evidence: [
      { label: "Knowledge", value: "Raw lecture: Agentic OS", state: "needs transcript" },
      { label: "Knowledge", value: "Meetup notes", state: "QR campaign context" },
      { label: "Knowledge", value: "Training outline", state: "student offer source" },
      { label: "Knowledge", value: "Trial doctrine", state: "offer support" },
    ],
    main: [
      { meta: "Lecture", title: "Raw lecture needs production", detail: "Turn source video into transcript, lesson, article, short clips, and student resources.", action: "Create job" },
      { meta: "Offer", title: "30-day trial doctrine", detail: "Source material should explain hosted setup, reset policy, and extension rules.", action: "Review" },
      { meta: "Training", title: "Student curriculum source", detail: "Lessons and assignment feedback tools need durable knowledge sources.", action: "Open" },
    ],
  },
  "content-pillars": {
    eyebrow: "Studio",
    title: "Content Pillars",
    summary: "Reusable themes from the knowledgebase that drive public feed, offers, training, and affiliate material.",
    stage: "4 pillars",
    primaryAction: "Shape content around durable knowledge.",
    railLabel: "Pillars",
    railSummary: "Themes with evidence and production readiness.",
    evidence: [
      { label: "Pillar", value: "Ordo as intent surface", state: "ready" },
      { label: "Pillar", value: "Offers as capabilities", state: "needs examples" },
      { label: "Pillar", value: "Evidence-backed work", state: "ready" },
      { label: "Pillar", value: "Affiliate QR loops", state: "draft" },
    ],
    main: [
      { meta: "Public", title: "Ordo as intent surface", detail: "Supports about story, trial pitch, and owner positioning.", action: "Use" },
      { meta: "Offer", title: "Capabilities", detail: "Explains why accepted offers unlock tools, content, training, or workflows.", action: "Use" },
    ],
  },
  "factory-jobs": {
    eyebrow: "Studio",
    title: "Factory Jobs",
    summary: "Production jobs turn knowledge, requests, and lectures into finished artifacts.",
    stage: "3 active jobs",
    primaryAction: "Track production without pretending candidate output is final.",
    railLabel: "Job stages",
    railSummary: "Progress, review, and publication readiness.",
    evidence: [
      { label: "Job", value: "Lecture to course module", state: "transcribe", readState: "unread", unreadCount: 1 },
      { label: "Job", value: "30-sec concept short", state: "storyboard" },
      { label: "Job", value: "Meetup QR card", state: "review ready" },
    ],
    main: [
      {
        meta: "Lecture",
        title: "Produce finished training content",
        detail: "Raw lecture becomes transcript, lesson, article, quiz, and short video candidates.",
        action: "Open",
        status: "active",
        statusTone: "ok",
        steps: ["source", "transcribe", "lesson", "review", "publish"],
        currentStep: 1,
      },
      {
        meta: "Short",
        title: "Create a 30-second concept video",
        detail: "Candidate media needs review before publication or user delivery.",
        action: "Open",
        status: "candidate",
        statusTone: "candidate",
        steps: ["brief", "storyboard", "generate", "review", "deliver"],
        currentStep: 1,
      },
      {
        meta: "QR",
        title: "Finalize meetup QR card",
        detail: "Artifact should cite campaign, offer, and entry point evidence.",
        action: "Review",
        status: "ready",
        statusTone: "ok",
        steps: ["design", "proof", "review", "print"],
        currentStep: 2,
      },
    ],
  },
  artifacts: {
    eyebrow: "Studio",
    title: "Artifacts",
    summary: "Briefs, posts, videos, QR cards, transcripts, specs, and training materials created by the factory.",
    stage: "Artifact library",
    primaryAction: "Review durable and candidate outputs.",
    railLabel: "Artifacts",
    railSummary: "Outputs with provenance and publication state.",
    evidence: [
      { label: "Artifact", value: "Agentic OS lesson draft", state: "candidate" },
      { label: "Artifact", value: "Meetup QR card", state: "review ready" },
      { label: "Artifact", value: "Trial offer brief", state: "published" },
    ],
    main: [
      { meta: "Candidate", title: "Agentic OS lesson draft", detail: "Needs human review before becoming a student resource.", action: "Review" },
      { meta: "Published", title: "Trial offer brief", detail: "Supports public story and trial conversations.", action: "Open" },
    ],
  },
  media: {
    eyebrow: "Studio",
    title: "Media",
    summary: "Images, video, audio, and source files used by production jobs and offers.",
    stage: "Browser preflight",
    primaryAction: "Prepare media before daemon validation.",
    railLabel: "Media queue",
    railSummary: "Candidate browser work and durable media artifacts.",
    evidence: [
      { label: "Media", value: "Raw lecture video", state: "hash candidate" },
      { label: "Media", value: "Meetup photo set", state: "metadata candidate" },
      { label: "Media", value: "QR card image", state: "daemon validated" },
    ],
    main: [
      { meta: "WASM", title: "Hash raw lecture locally", detail: "Browser capability output stays candidate until daemon validation.", action: "Preflight" },
      { meta: "Review", title: "Approve QR card image", detail: "Image can become public only after evidence and review.", action: "Review" },
    ],
  },
  publications: {
    eyebrow: "Studio",
    title: "Publications",
    summary: "Govern where approved artifacts appear: public feed, about story, offers, training access, affiliate resources.",
    stage: "Publication queue",
    primaryAction: "Publish only approved artifacts.",
    railLabel: "Publication targets",
    railSummary: "Targets, consent, and visibility boundaries.",
    evidence: [
      { label: "Target", value: "Public feed", state: "trial proof ready" },
      { label: "Target", value: "Training access", state: "lesson draft candidate" },
      { label: "Target", value: "Affiliate kit", state: "QR asset ready" },
    ],
    main: [
      { meta: "Feed", title: "Publish trial offer proof", detail: "Public copy must avoid fake urgency, fake metrics, and unsupported claims.", action: "Review" },
      { meta: "Training", title: "Release lesson to students", detail: "Training material goes to Capabilities after approval.", action: "Approve" },
    ],
  },
  templates: {
    eyebrow: "Studio",
    title: "Templates",
    summary: "Repeatable production formats for lecture modules, concept shorts, articles, QR cards, briefs, and specs.",
    stage: "Template library",
    primaryAction: "Define reusable artifact shapes.",
    railLabel: "Templates",
    railSummary: "Inputs, evidence needs, and output constraints.",
    evidence: [
      { label: "Template", value: "Lecture to module", state: "draft" },
      { label: "Template", value: "30-sec short", state: "ready" },
      { label: "Template", value: "Build spec", state: "planned" },
    ],
    main: [
      { meta: "Training", title: "Lecture to module", detail: "Produces transcript, cleaned lesson, quiz, article, and short video candidates.", action: "Edit" },
      { meta: "Build", title: "Spec from conversation", detail: "A future capability can turn user intent into build-ready specs.", action: "Draft" },
    ],
  },
};

const ownerScreens: Record<string, MemberScreen> = {
  brief: {
    eyebrow: "Business",
    title: "Brief",
    summary: "Business is owner governance: money, growth, marketing, offers, content performance, and decisions.",
    stage: "3 decisions",
    primaryAction: "Decide what the business should do next.",
    railLabel: "Owner attention",
    railSummary: "Business-level signals and decisions.",
    evidence: [
      { label: "Decision", value: "Trial extension policy", state: "needs owner decision", readState: "unread", unreadCount: 1 },
      { label: "Decision", value: "Training offer price", state: "draft model" },
      { label: "Decision", value: "Meetup channel", state: "strong QR scans", readState: "unread", unreadCount: 1 },
      { label: "Decision", value: "Lecture production", state: "factory capacity needed", readState: "unread", unreadCount: 1 },
    ],
    main: [
      { meta: "Revenue", title: "Hosted trial extension needs a rule", detail: "One setup delay suggests a reasonable extension policy before more trial users arrive.", action: "Decide" },
      { meta: "Marketing", title: "Meetup QR channel is working", detail: "12 scans, 4 fit conversations, 2 trial requests, and 1 affiliate candidate from the last event.", action: "Inspect" },
      { meta: "Studio", title: "Lecture production can become sellable training", detail: "Raw lectures can become modules, articles, shorts, quizzes, and student resources.", action: "Fund" },
    ],
  },
  revenue: {
    eyebrow: "Business",
    title: "Revenue",
    summary: "Money and access signals from consultations, trials, training, extensions, and affiliate outcomes.",
    stage: "$3.4k open",
    primaryAction: "Review revenue in motion.",
    railLabel: "Revenue evidence",
    railSummary: "Accepted offers, pending decisions, and likely value.",
    evidence: [
      { label: "Revenue", value: "$1.2k consultation", state: "scheduled" },
      { label: "Revenue", value: "$0 trial", state: "hosted setup" },
      { label: "Revenue", value: "$2.2k training cohort", state: "5 students interested" },
      { label: "Revenue", value: "Affiliate rewards", state: "2 pending" },
    ],
    main: [
      { meta: "Consult", title: "Strategic consultation booked", detail: "Marcus is scheduled and needs prep reminders before payment confirmation.", action: "Review" },
      { meta: "Training", title: "Training cohort has early demand", detail: "Five students asked about tutoring, assignment feedback, and lesson resources.", action: "Model" },
      { meta: "Trial", title: "Trial extension could protect conversion", detail: "Hosted setup delays should not count against the user's 30 days.", action: "Decide" },
    ],
  },
  pipeline: {
    eyebrow: "Business",
    title: "Pipeline",
    summary: "Waitlist, consultation, trial, student, affiliate, customer, and advocate movement.",
    stage: "18 active",
    primaryAction: "See where business value is forming.",
    railLabel: "Pipeline",
    railSummary: "Business stages, not staff task queues.",
    evidence: [
      { label: "Pipeline", value: "Waitlist", state: "6" },
      { label: "Pipeline", value: "Consultations", state: "3" },
      { label: "Pipeline", value: "Trials", state: "4" },
      { label: "Pipeline", value: "Students", state: "5" },
    ],
    main: [
      { meta: "Waitlist", title: "Open two hosted trial slots", detail: "Capacity exists if support and setup remain stable.", action: "Approve" },
      { meta: "Student", title: "Training can become the next offer", detail: "Interest is coming from people who want tutoring and custom content.", action: "Plan" },
    ],
  },
  offers: {
    eyebrow: "Business",
    title: "Offers",
    summary: "Offer performance, objections, acceptance, and capability design.",
    stage: "Trial leads",
    primaryAction: "Improve the offers without adding pressure.",
    railLabel: "Offer signals",
    railSummary: "Fit checks, objections, and acceptance evidence.",
    evidence: [
      { label: "Offer", value: "30-day trial", state: "2 requests" },
      { label: "Offer", value: "Consultation", state: "1 scheduled" },
      { label: "Offer", value: "Training", state: "5 interested" },
      { label: "Offer", value: "Affiliate", state: "1 candidate" },
    ],
    main: [
      { meta: "Trial", title: "Clarify hosted setup and reset policy", detail: "People understand the offer better when setup time is separated from trial value.", action: "Revise" },
      { meta: "Training", title: "Package tutoring and feedback", detail: "Students want feedback loops, lesson resources, and progress support.", action: "Design" },
    ],
  },
  marketing: {
    eyebrow: "Business",
    title: "Marketing",
    summary: "QR events, public feed, source quality, campaigns, and content impact.",
    stage: "Meetup QR leading",
    primaryAction: "Decide which channels deserve attention.",
    railLabel: "Marketing evidence",
    railSummary: "Sources, scans, conversations, and outcomes.",
    evidence: [
      { label: "Channel", value: "Meetup QR", state: "12 scans" },
      { label: "Channel", value: "Public feed", state: "trial story viewed" },
      { label: "Channel", value: "Affiliate intro", state: "2 candidates" },
      { label: "Channel", value: "Lecture content", state: "not published" },
    ],
    main: [
      { meta: "Event", title: "Meetup QR is highest signal", detail: "Scans are turning into real conversations, trial requests, and affiliate interest.", action: "Repeat" },
      { meta: "Content", title: "Lecture clips could support marketing", detail: "Short clips from raw lectures can explain Ordo without extra sales pages.", action: "Produce" },
    ],
  },
  referrals: {
    eyebrow: "Business",
    title: "Referrals",
    summary: "Affiliate value, reward decisions, attribution quality, and referred-customer outcomes.",
    stage: "2 reward reviews",
    primaryAction: "Inspect affiliate performance.",
    railLabel: "Referral evidence",
    railSummary: "Outcome-backed referral signals.",
    evidence: [
      { label: "Referral", value: "Ava Thompson", state: "candidate affiliate" },
      { label: "Referral", value: "Two accepted intros", state: "reward review" },
      { label: "Referral", value: "QR assets", state: "ready" },
    ],
    main: [
      { meta: "Affiliate", title: "Ava could promote Ordo", detail: "She has strong fit, but terms and scoped visibility must come first.", action: "Invite" },
      { meta: "Reward", title: "Reward only evidence-backed outcomes", detail: "Credit requires entry point, visitor session, offer, and acceptance evidence.", action: "Review" },
    ],
  },
  content: {
    eyebrow: "Business",
    title: "Content",
    summary: "Factory output, public proof, training resources, and marketing assets from the knowledgebase.",
    stage: "Lecture job ready",
    primaryAction: "Choose which content should be produced.",
    railLabel: "Content performance",
    railSummary: "Produced or planned artifacts with business purpose.",
    evidence: [
      { label: "Content", value: "Raw lecture", state: "production candidate" },
      { label: "Content", value: "Trial offer brief", state: "supporting conversions" },
      { label: "Content", value: "Meetup QR card", state: "used at event" },
    ],
    main: [
      { meta: "Training", title: "Turn lecture into a sellable module", detail: "Produce lesson, article, short video, quiz, and student resource bundle.", action: "Approve" },
      { meta: "Marketing", title: "Publish short proof clips", detail: "Use approved lecture excerpts to explain the operating-system concept.", action: "Plan" },
    ],
  },
};

ownerScreens.overview = {
  ...ownerScreens.brief,
  title: "Overview",
  railLabel: "Business attention",
  primaryAction: "Review business-level decisions.",
};

ownerScreens.affiliates = {
  ...ownerScreens.referrals,
  title: "Affiliates",
  railLabel: "Affiliate evidence",
  primaryAction: "Inspect affiliate performance and reward evidence.",
};

ownerScreens.reports = {
  ...ownerScreens.content,
  title: "Reports",
  summary: "Journey reports, artifact review findings, marketing signals, and owner follow-up drafts.",
  stage: "3 drafts",
  primaryAction: "Review reports before turning findings into work.",
  railLabel: "Report evidence",
  railSummary: "Findings stay candidate until owner review.",
  evidence: [
    { label: "Report", value: "Meetup QR journey", state: "needs review", readState: "unread", unreadCount: 1 },
    { label: "Report", value: "Trial conversion", state: "candidate insight" },
    { label: "Report", value: "Training demand", state: "owner draft" },
  ],
  main: [
    { meta: "Journey", title: "Meetup QR journey report", detail: "Summarizes scans, conversations, trial requests, affiliate interest, and handoff quality.", action: "Review" },
    { meta: "Conversion", title: "Trial path friction", detail: "Setup and extension questions should become owner decisions before the next event.", action: "Decide" },
    { meta: "Training", title: "Training demand signal", detail: "Student interest is strong enough to plan a small offer and production workflow.", action: "Plan" },
  ],
};

const systemScreens: Record<string, MemberScreen> = {
  conversations: {
    eyebrow: "System",
    title: "Conversations",
    summary: "All system conversation streams, handoffs, replay cursors, and trust-boundary evidence.",
    stage: "Global conversation audit",
    primaryAction: "Inspect the selected conversation stream.",
    railLabel: "Conversation evidence",
    railSummary: "Admin sees global conversation projections, not raw provider payloads.",
    streamStrategy: "replace",
    evidence: [
      { label: "Conversation", value: "Maya Patel meetup handoff", state: "staff handoff" },
      { label: "Conversation", value: "Ava Thompson review-return", state: "consent boundary" },
      { label: "Conversation", value: "Meetup QR intake", state: "offer path" },
    ],
    main: [
      {
        meta: "Handoff",
        title: "Maya Patel meetup handoff is visible system-wide",
        detail: "Admin can inspect the QR entry, durable conversation, handoff state, and replay cursor while keeping provider payloads and policy internals out of product views.",
        action: "Inspect replay",
        status: "unread",
        statusTone: "attention",
        evidenceRefs: ["conversation:maya-patel", "handoff:keith-live", "cursor:staff-142"],
      },
      {
        meta: "Review",
        title: "Ava Thompson review-return thread",
        detail: "Review-return conversations remain auditable with consent and approval state separated from private feedback.",
        action: "Review boundary",
        status: "waiting on you",
        statusTone: "attention",
        evidenceRefs: ["conversation:ava-thompson", "feedback:trial-day-4", "review_candidate:ava"],
      },
    ],
  },
  health: {
    eyebrow: "System",
    title: "Health",
    summary: "Daemon liveness, readiness, realtime channels, and service status.",
    stage: "Appliance status",
    primaryAction: "Keep the appliance safe and observable.",
    railLabel: "System evidence",
    railSummary: "Operational state only owner/system operators should see.",
    evidence: [
      { label: "Health", value: "Daemon", state: "reachable" },
      { label: "Health", value: "SQLite", state: "ready" },
      { label: "Health", value: "Realtime", state: "live" },
    ],
    main: [
      { meta: "Readiness", title: "Required tables present", detail: "The appliance can serve mocked product surfaces and system diagnostics.", action: "Inspect" },
      { meta: "Realtime", title: "Global event channel available", detail: "Activity should replay from cursor after reconnect.", action: "Inspect" },
    ],
  },
  events: {
    eyebrow: "System",
    title: "Events",
    summary: "Persisted event evidence for replay, activity projection, and diagnostics.",
    stage: "Cursor replay",
    primaryAction: "Inspect durable event flow.",
    railLabel: "Event stream",
    railSummary: "Raw events are system-scoped; product surfaces get projections.",
    evidence: [
      { label: "Event", value: "visitor_session.started", state: "cursor 118" },
      { label: "Event", value: "offer.accepted", state: "cursor 124" },
      { label: "Event", value: "job.stage.changed", state: "cursor 141" },
    ],
    main: [
      { meta: "Projection", title: "Activity derives from events", detail: "Ordo should show role-safe activity items, not raw event payloads.", action: "Review" },
      { meta: "Replay", title: "Reconnect should resume by cursor", detail: "Duplicate events must be idempotent.", action: "Inspect" },
    ],
  },
  logs: {
    eyebrow: "System",
    title: "Logs",
    summary: "Structured diagnostic observations with strict redaction boundaries.",
    stage: "Owner/admin only",
    primaryAction: "Inspect diagnostics without leaking product internals.",
    railLabel: "Log evidence",
    railSummary: "No raw prompts, provider payloads, private terms, or staff notes.",
    evidence: [
      { label: "Log", value: "gateway.connected", state: "info" },
      { label: "Log", value: "provider.guard.skipped", state: "expected" },
      { label: "Log", value: "activity.projector.mock", state: "prototype" },
    ],
    main: [
      { meta: "Privacy", title: "Logs stay system-scoped", detail: "Customer, staff, and owner business surfaces should receive summaries only.", action: "Inspect" },
    ],
  },
  backup: {
    eyebrow: "System",
    title: "Backup",
    summary: "Backup, restore, and recovery jobs for the appliance.",
    stage: "Recovery safety",
    primaryAction: "Protect durable state.",
    railLabel: "Backup jobs",
    railSummary: "Safety operations separate from business production jobs.",
    evidence: [
      { label: "Backup", value: "Nightly snapshot", state: "ready" },
      { label: "Backup", value: "Restore validation", state: "not run today" },
    ],
    main: [
      { meta: "Snapshot", title: "Nightly backup available", detail: "Durable business evidence should survive restore.", action: "Verify" },
      { meta: "Restore", title: "Run restore validation", detail: "Keep restore confidence separate from production factory jobs.", action: "Validate" },
    ],
  },
  providers: {
    eyebrow: "System",
    title: "Providers",
    summary: "Model and integration configuration with explicit live/network guards.",
    stage: "Guarded",
    primaryAction: "Manage providers without exposing secrets.",
    railLabel: "Provider evidence",
    railSummary: "Keys and raw payloads never belong in UI fixtures.",
    evidence: [
      { label: "Provider", value: "OpenAI-compatible", state: "configured by env" },
      { label: "Guard", value: "Live evals", state: "opt-in only" },
      { label: "Budget", value: "Spend caps", state: "required" },
    ],
    main: [
      { meta: "Guard", title: "Live calls stay opt-in", detail: "Real provider behavior belongs behind explicit environment guards and spend caps.", action: "Review" },
    ],
  },
  access: {
    eyebrow: "System",
    title: "Access",
    summary: "Roles, memberships, workspace grants, and trust boundaries.",
    stage: "Role constrained",
    primaryAction: "Keep workspaces and evidence projections safe.",
    railLabel: "Access evidence",
    railSummary: "Role grants control workspaces, not just menu labels.",
    evidence: [
      { label: "Role", value: "Ordo workspace", state: "authenticated" },
      { label: "Role", value: "Staff/Studio", state: "staff+" },
      { label: "Role", value: "Owner/System", state: "owner/system" },
    ],
    main: [
      { meta: "Shells", title: "Shell access is explicit", detail: "A role can switch shells only when grants allow it.", action: "Review" },
      { meta: "Projection", title: "Client-safe views fail closed", detail: "Staff/provider/policy/private internals should never leak through preferences or routes.", action: "Inspect" },
    ],
  },
  settings: {
    eyebrow: "System",
    title: "Settings",
    summary: "Appliance-level settings, not personal user preferences.",
    stage: "System configuration",
    primaryAction: "Configure the appliance carefully.",
    railLabel: "Settings evidence",
    railSummary: "System settings are owner/system scoped.",
    evidence: [
      { label: "Setting", value: "Brand display", state: "logo and title" },
      { label: "Setting", value: "Homepage mode", state: "owner selectable" },
      { label: "Setting", value: "Provider guards", state: "required" },
    ],
    main: [
      { meta: "Brand", title: "Control logo/title display", detail: "Framework users should customize naming without code changes.", action: "Edit" },
      { meta: "Home", title: "Choose public home mode", detail: "Owner/admin can choose scrollytelling home or full-screen Ordo home.", action: "Edit" },
    ],
  },
};

const shellScreens: Partial<Record<ProductAppSpace, Record<string, MemberScreen>>> = {
  "my-ordo": streamEnhanceScreens("my-ordo", userScreens),
  staff: streamEnhanceScreens("staff", staffScreens),
  studio: streamEnhanceScreens("studio", studioScreens),
  owner: streamEnhanceScreens("owner", ownerScreens),
  admin: streamEnhanceScreens("admin", systemScreens),
};
