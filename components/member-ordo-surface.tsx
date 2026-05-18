import Link from "next/link";

import { MemberChatGatewayComposer } from "@/components/member-chat-gateway";
import { ProductShell } from "@/components/product-shell";
import {
  memberItemsForRoom,
  memberRoomById,
  selectedMemberItem,
  type MemberAction,
  type MemberEvidenceRef,
  type MemberRoom,
  type MemberRoomId,
  type MemberTimelineEvent,
  type MemberWorkItem as MemberWorkItemRecord,
} from "@/lib/member-ordo-mock";
import {
  mobileStepFromSearchParams,
  railModeFromSearchParams,
  roleFromSearchParams,
  selectedItemIndexFromSearchParams,
  type ProductMobileStep,
  type SearchParams,
} from "@/lib/page-role";
import { roleHref, type ProductRole } from "@/lib/product-navigation";

interface MemberOrdoSurfaceProps {
  roomId: MemberRoomId;
  searchParams?: SearchParams;
}

export async function MemberOrdoSurface({ roomId, searchParams }: MemberOrdoSurfaceProps) {
  const [role, railMode, mobileStep, selectedIndex] = await Promise.all([
    roleFromSearchParams(searchParams),
    railModeFromSearchParams(searchParams),
    mobileStepFromSearchParams(searchParams, roomId === "requests" ? "content" : "rooms"),
    selectedItemIndexFromSearchParams(searchParams),
  ]);
  const room = memberRoomById(roomId);
  const workItems = memberItemsForRoom(room.id);
  const selectedItem = selectedMemberItem(room.id, selectedIndex);
  const conversationOnly = room.id === "ordo";

  return (
    <ProductShell
      role={role}
      appSpaceId="my-ordo"
      currentItemId={room.id}
      railMode={railMode}
      mobileStep={mobileStep}
      selectedItemIndex={selectedIndex}
      collapseSectionRail={conversationOnly}
      roomEvidenceRail={conversationOnly ? undefined : <MemberFrameContent room={room} items={workItems} selectedItem={selectedItem} role={role} />}
    >
      <MemberStage room={room} item={selectedItem} role={role} />
    </ProductShell>
  );
}

export function MemberFrameContent({
  room,
  items,
  selectedItem,
  role,
}: {
  room: MemberRoom;
  items: readonly MemberWorkItemRecord[];
  selectedItem: MemberWorkItemRecord;
  role: ProductRole;
}) {
  return (
    <div className="member-frame-content">
      <header className="member-worklist-header">
        <span className="eyebrow">{room.eyebrow}</span>
        <h1>{room.label}</h1>
        <p>{room.description}</p>
        <strong>{room.brief}</strong>
      </header>

      <MemberWorklist room={room} items={items} selectedItem={selectedItem} role={role} />
    </div>
  );
}

export function MemberWorklist({
  room,
  items,
  selectedItem,
  role,
}: {
  room: MemberRoom;
  items: readonly MemberWorkItemRecord[];
  selectedItem: MemberWorkItemRecord;
  role: ProductRole;
}) {
  const waitingItems = items.filter((item) => isWaitingItem(item));
  const recentItems = items.filter((item) => !isWaitingItem(item));

  return (
    <div className="member-worklist" aria-label={`${room.label} worklist`}>
      <MemberWorklistGroup label={waitingItems.length > 0 ? "Needs action" : "Current"} room={room} items={waitingItems.length > 0 ? waitingItems : recentItems} selectedItem={selectedItem} role={role} />
      {waitingItems.length > 0 && recentItems.length > 0 ? <MemberWorklistGroup label="Recent" room={room} items={recentItems} selectedItem={selectedItem} role={role} /> : null}
    </div>
  );
}

function MemberWorklistGroup({
  label,
  room,
  items,
  selectedItem,
  role,
}: {
  label: string;
  room: MemberRoom;
  items: readonly MemberWorkItemRecord[];
  selectedItem: MemberWorkItemRecord;
  role: ProductRole;
}) {
  if (items.length === 0) {
    return (
      <section className="member-worklist-group">
        <h2>{label}</h2>
        <p className="member-empty-note">Nothing needs attention in this room.</p>
      </section>
    );
  }

  return (
    <section className="member-worklist-group">
      <h2>{label}</h2>
      <div className="member-worklist-items">
        {items.map((item) => {
          const roomIndex = memberItemsForRoom(room.id).findIndex((candidate) => candidate.id === item.id);
          return <MemberWorkItem key={item.id} room={room} item={item} selected={item.id === selectedItem.id} role={role} index={Math.max(roomIndex, 0)} />;
        })}
      </div>
    </section>
  );
}

export function MemberWorkItem({
  room,
  item,
  selected,
  role,
  index,
}: {
  room: MemberRoom;
  item: MemberWorkItemRecord;
  selected: boolean;
  role: ProductRole;
  index: number;
}) {
  const primaryAction = item.actions.find((action) => action.primary) ?? item.actions[0];
  const detailHref = memberItemHref(room.href, role, index, "content");
  const actionHref = primaryAction?.href ? roleHref(primaryAction.href, role) : undefined;

  return (
    <article className={selected ? "member-work-item member-work-item-selected" : "member-work-item"} aria-current={selected ? "true" : undefined}>
      <Link href={detailHref} className="member-work-item-copy">
        <strong>{item.title}</strong>
        <span>{item.summary}</span>
        <small>
          {item.source} · {item.occurredAt}
        </small>
      </Link>
      <span className="member-work-item-side">
        <MemberStatusBadge status={item.status} readState={item.readState} />
        {primaryAction ? (
          actionHref ? (
            <Link href={actionHref} className="member-work-item-action" aria-label={`${primaryAction.label}: ${item.title}`}>
              <MemberActionIcon action={primaryAction} />
              <span>{primaryAction.label}</span>
            </Link>
          ) : (
            <span className="member-work-item-action">
              <MemberActionIcon action={primaryAction} />
              <span>{primaryAction.label}</span>
            </span>
          )
        ) : null}
      </span>
    </article>
  );
}

export function MemberStage({ room, item, role }: { room: MemberRoom; item: MemberWorkItemRecord; role: ProductRole }) {
  if (room.id === "ordo") {
    return <MemberConversationStage item={item} role={role} />;
  }

  return (
    <article className="member-main-content member-evidence-stage" aria-label={`${item.title} detail`}>
      <MemberSummaryBand room={room} item={item} role={role} />
      <MemberTimeline timeline={item.timeline} />
      <MemberEvidenceRefs refs={item.evidenceRefs} />
    </article>
  );
}

function MemberConversationStage({ item, role }: { item: MemberWorkItemRecord; role: ProductRole }) {
  return (
    <article className="member-main-content member-conversation-stage" aria-label="Studio Ordo conversation">
      <header className="member-conversation-stage-header">
        <span className="eyebrow">Ordo</span>
        <h1>Talk with Studio Ordo</h1>
        <section aria-label="Ordo relationship brief">
          <p>One relationship conversation with Studio Ordo. {item.summary}</p>
        </section>
        <dl className="member-conversation-status-list" aria-label="Safe handoff status">
          <div>
            <dt>Handoff status</dt>
            <dd>Keith handoff remains available as safe status only; internal routing and provider details stay hidden.</dd>
          </div>
        </dl>
      </header>
      <MemberConversationPreview item={item} role={role} />
      <MemberStageComposer />
    </article>
  );
}

export function MemberSummaryBand({ room, item, role }: { room: MemberRoom; item: MemberWorkItemRecord; role: ProductRole }) {
  return (
    <section className="member-stage-summary-band" aria-label={`${item.title} Ordo summary`}>
      <div className="member-stage-kicker-row">
        <span>{item.kind}</span>
        <span>{room.label}</span>
        <MemberStatusBadge status={item.status} readState={item.readState} />
      </div>
      <h2>{item.title}</h2>
      <p className="member-stage-summary-lede">{item.summaryBand.whatHappened}</p>
      <div className="member-stage-why">
        <strong>Why it matters</strong>
        <p>{item.summaryBand.whyItMatters}</p>
      </div>
      <MemberActionRow actions={item.actions} role={role} />
    </section>
  );
}

function MemberConversationPreview({ item, role }: { item: MemberWorkItemRecord; role: ProductRole }) {
  const messages = item.conversationPreview ?? [];

  if (messages.length === 0) {
    return null;
  }

  return (
    <section className="member-conversation-preview" aria-label={`${item.title} conversation preview`}>
      {messages.map((message, index) => (
        <div key={`${index}-${message.speaker}-${message.title ?? message.body}`} className={`member-conversation-message member-conversation-message-${message.tone ?? "ordo"}`}>
          <div className="member-conversation-message-header">
            <strong>{message.speaker}</strong>
            {message.meta ? <span>{message.meta}</span> : null}
          </div>
          {message.title ? <h3>{message.title}</h3> : null}
          <p>{message.body}</p>
          {message.actions?.length ? <MemberActionRow actions={message.actions} role={role} compact /> : null}
        </div>
      ))}
    </section>
  );
}

export function MemberTimeline({ timeline }: { timeline: readonly MemberTimelineEvent[] }) {
  return (
    <section className="member-stage-section" aria-label="Timeline">
      <h3>Timeline</h3>
      <ol className="member-timeline-list">
        {timeline.map((entry) => (
          <li key={`${entry.at}-${entry.title}`}>
            <span className="member-timeline-dot" aria-hidden="true" />
            <span className="member-timeline-time">{entry.at}</span>
            <strong>{entry.title}</strong>
            <p>{entry.summary}</p>
          </li>
        ))}
      </ol>
    </section>
  );
}

export function MemberEvidenceRefs({ refs }: { refs: readonly MemberEvidenceRef[] }) {
  return (
    <section className="member-stage-section member-provenance-section" aria-label="Evidence">
      <h3>Evidence</h3>
      <ul className="member-provenance-list">
        {refs.map((ref) => (
          <li key={ref.id}>
            <span>{ref.kind}</span>
            <strong>{ref.label}</strong>
          </li>
        ))}
      </ul>
    </section>
  );
}

export function MemberActionRow({ actions, role, compact = false }: { actions: readonly MemberAction[]; role: ProductRole; compact?: boolean }) {
  return (
    <div className={compact ? "member-stage-action-row member-action-row-compact" : "member-stage-action-row"} aria-label="Available actions">
      {actions.map((action) => {
        const className = action.primary ? "member-action-button member-action-button-primary" : "member-action-button";
        const key = `${action.kind}-${action.label}-${action.href ?? "button"}`;

        if (action.href) {
          return (
            <Link key={key} href={roleHref(action.href, role)} className={className}>
              <MemberActionIcon action={action} />
              <span>{action.label}</span>
            </Link>
          );
        }

        return (
          <button key={key} type="button" className={className}>
            <MemberActionIcon action={action} />
            <span>{action.label}</span>
          </button>
        );
      })}
    </div>
  );
}

function MemberActionIcon({ action }: { action: MemberAction }) {
  if (action.kind !== "reply" && !action.href?.includes("/my/chat")) {
    return null;
  }

  return (
    <svg className="member-action-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4.75 7.25A3.75 3.75 0 0 1 8.5 3.5h7A3.75 3.75 0 0 1 19.25 7.25v3.5A3.75 3.75 0 0 1 15.5 14.5h-4.35l-4.82 4.14A.8.8 0 0 1 5 18.03V14.2a3.74 3.74 0 0 1-.25-6.95Z" />
      <path d="M8 7h8" />
      <path d="M8 10.5h5.5" />
    </svg>
  );
}

export function MemberStatusBadge({ status, readState }: { status: MemberWorkItemRecord["status"]; readState: MemberWorkItemRecord["readState"] }) {
  return <span className={`member-status-badge member-status-${status}`}>{readState === "unread" ? "unread" : statusLabel(status)}</span>;
}

function MemberStageComposer() {
  return <MemberChatGatewayComposer />;
}

function memberItemHref(roomHref: string, role: ProductRole, index: number, mobileStep: ProductMobileStep): string {
  const href = roleHref(roomHref, role);
  const url = new URL(href, "https://ordo.local");
  url.searchParams.set("item", String(index));
  url.searchParams.set("mobile", mobileStep);
  return `${url.pathname}?${url.searchParams.toString()}`;
}

function isWaitingItem(item: MemberWorkItemRecord): boolean {
  return item.readState === "unread" || item.status === "waiting_on_you";
}

function statusLabel(status: MemberWorkItemRecord["status"]): string {
  switch (status) {
    case "waiting_on_you":
      return "waiting on you";
    case "waiting_on_ordo":
      return "waiting on Ordo";
    default:
      return status;
  }
}
