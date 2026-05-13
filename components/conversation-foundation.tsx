"use client";

import { useMemo, useRef, useState, type Dispatch, type Ref, type SetStateAction } from "react";

import {
  CONVERSATION_GATEWAY_ROUTE,
  CONVERSATION_GATEWAY_SCHEMA_VERSION,
  type ConversationCommandType,
} from "@/lib/conversation-protocol";
import {
  conversationQueues,
  queueRowsForRole,
  sampleConversationDetails,
  type ArtifactBriefCard,
  type ConversationDetail,
  type ConversationMessage,
  type ConversationQueueRow,
} from "@/lib/conversation-product";
import { type ProductRole } from "@/lib/product-navigation";

type GatewayStatus = "connected" | "pending" | "rejected" | "offline" | "replaying" | "recovered";
type ComposerStatus = "idle" | "sending" | "failed";

interface GatewayState {
  selectedConversationId: string;
  detail: ConversationDetail;
  gatewayStatus: GatewayStatus;
  composerStatus: ComposerStatus;
  lastClientId?: string;
  lastError?: string;
  sequence: number;
  cursor: number;
  recoveryMessage?: string;
}

export function ClientConversationBrief() {
  const detail = useGatewayConversation("conv_ava", "client");

  return (
    <ConversationExperience
      detail={detail.detail}
      rows={[]}
      selectedConversationId={detail.selectedConversationId}
      role="client"
      gatewayStatus={detail.gatewayStatus}
      composerStatus={detail.composerStatus}
      recoveryMessage={detail.recoveryMessage}
      lastError={detail.lastError}
      onSelectConversation={detail.selectConversation}
      onSendMessage={detail.sendMessage}
      onEditMessage={detail.editMessage}
      onUndoMessage={detail.undoMessage}
      onMarkRead={detail.markRead}
      onMarkUnread={detail.markUnread}
      onReact={detail.toggleReaction}
      onRetry={detail.retryLastMessage}
      onSimulateOffline={detail.simulateOffline}
      onReconnect={detail.reconnectAndReplay}
    />
  );
}

export function StaffConversationQueues({ role }: { role: ProductRole }) {
  const rows = queueRowsForRole(role);
  const initialConversationId = rows[0]?.id ?? "conv_ava";
  const detail = useGatewayConversation(initialConversationId, role);

  return (
    <ConversationExperience
      detail={detail.detail}
      rows={rows}
      selectedConversationId={detail.selectedConversationId}
      role={role}
      gatewayStatus={detail.gatewayStatus}
      composerStatus={detail.composerStatus}
      recoveryMessage={detail.recoveryMessage}
      lastError={detail.lastError}
      onSelectConversation={detail.selectConversation}
      onSendMessage={detail.sendMessage}
      onEditMessage={detail.editMessage}
      onUndoMessage={detail.undoMessage}
      onMarkRead={detail.markRead}
      onMarkUnread={detail.markUnread}
      onReact={detail.toggleReaction}
      onRetry={detail.retryLastMessage}
      onSimulateOffline={detail.simulateOffline}
      onReconnect={detail.reconnectAndReplay}
    />
  );
}

function ConversationExperience({
  detail,
  rows,
  selectedConversationId,
  role,
  gatewayStatus,
  composerStatus,
  recoveryMessage,
  lastError,
  onSelectConversation,
  onSendMessage,
  onEditMessage,
  onUndoMessage,
  onMarkRead,
  onMarkUnread,
  onReact,
  onRetry,
  onSimulateOffline,
  onReconnect,
}: {
  detail: ConversationDetail;
  rows: readonly ConversationQueueRow[];
  selectedConversationId: string;
  role: ProductRole;
  gatewayStatus: GatewayStatus;
  composerStatus: ComposerStatus;
  recoveryMessage?: string;
  lastError?: string;
  onSelectConversation: (conversationId: string) => void;
  onSendMessage: (body: string) => void;
  onEditMessage: (messageId: string, body: string) => void;
  onUndoMessage: (messageId: string) => void;
  onMarkRead: () => void;
  onMarkUnread: () => void;
  onReact: (messageId: string, reactionKey: string) => void;
  onRetry: () => void;
  onSimulateOffline: () => void;
  onReconnect: () => void;
}) {
  const isStaff = role === "staff" || role === "manager" || role === "owner" || role === "admin";
  const timelineRef = useRef<HTMLElement | null>(null);

  function jumpToLatest() {
    timelineRef.current?.querySelector("[data-latest-message='true']")?.scrollIntoView({ block: "end" });
  }

  function jumpToFirstUnread() {
    timelineRef.current?.querySelector("[data-unread-divider='true']")?.scrollIntoView({ block: "center" });
  }

  return (
    <section className="conversation-core" aria-label="Conversation workspace">
      <ConversationList
        rows={rows}
        selectedConversationId={selectedConversationId}
        detail={detail}
        isStaff={isStaff}
        onSelectConversation={onSelectConversation}
      />
      <section className="conversation-detail" aria-labelledby="conversation-title">
        <ConversationHeader detail={detail} gatewayStatus={gatewayStatus} gatewayRoute={CONVERSATION_GATEWAY_ROUTE} />
        <RecoveryBanner gatewayStatus={gatewayStatus} message={recoveryMessage} onReconnect={onReconnect} onSimulateOffline={onSimulateOffline} />
        <NarrativeBrief detail={detail} isStaff={isStaff} />
        <PersuasionGuidance detail={detail} isStaff={isStaff} />
        <ArtifactCards cards={detail.artifactCards} isStaff={isStaff} />
        <div className="timeline-toolbar" aria-label="Timeline navigation">
          <button type="button" className="button-secondary compact-button" onClick={jumpToFirstUnread}>
            Jump to first unread
          </button>
          <button type="button" className="button-secondary compact-button" onClick={jumpToLatest}>
            Jump to latest
          </button>
        </div>
        <MessageTimeline
          ref={timelineRef}
          detail={detail}
          isStaff={isStaff}
          lastError={lastError}
          onEditMessage={onEditMessage}
          onUndoMessage={onUndoMessage}
          onMarkUnread={onMarkUnread}
          onReact={onReact}
          onRetry={onRetry}
        />
        <ConversationComposer status={composerStatus} onSendMessage={onSendMessage} onTypingStart={() => undefined} />
        <div className="conversation-actions" aria-label="Read state actions">
          <button type="button" className="button-secondary compact-button" aria-label="Mark conversation read" onClick={onMarkRead}>
            Mark read
          </button>
          <button type="button" className="button-secondary compact-button" aria-label="Mark conversation unread" onClick={onMarkUnread}>
            Mark unread
          </button>
        </div>
      </section>
    </section>
  );
}

function ConversationList({
  rows,
  selectedConversationId,
  detail,
  isStaff,
  onSelectConversation,
}: {
  rows: readonly ConversationQueueRow[];
  selectedConversationId: string;
  detail: ConversationDetail;
  isStaff: boolean;
  onSelectConversation: (conversationId: string) => void;
}) {
  if (!isStaff) {
    return (
      <aside className="conversation-list" aria-label="Relationship conversation">
        <span className="eyebrow">Ordo</span>
        <h2>Your conversation</h2>
        <button type="button" className="conversation-row active" aria-current="true">
          <span>
            <strong>{detail.connectionLabel}</strong>
            <small>single relationship conversation</small>
          </span>
          <b>{detail.messages.length}</b>
        </button>
        <div className="client-tools" aria-label="Account conversation tools">
          <span>My conversation</span>
          <span>My offers</span>
          <span>My deliverables</span>
          <span>My requests</span>
        </div>
      </aside>
    );
  }

  return (
    <aside className="conversation-list" aria-label="Conversation queues">
      <span className="eyebrow">Conversations</span>
      <h2>Queues</h2>
      <div className="queue-tabs" role="list" aria-label="Queue availability">
        {conversationQueues.map((queue) => (
          <span key={queue.id} className="queue-pill" data-queue={queue.id}>
            {queue.label}
          </span>
        ))}
      </div>
      <div className="queue-list">
        {rows.map((row) => (
          <button
            type="button"
            key={row.id}
            className={row.id === selectedConversationId ? "conversation-row active" : "conversation-row"}
            aria-current={row.id === selectedConversationId ? "true" : undefined}
            onClick={() => onSelectConversation(row.id)}
          >
            <span>
              <strong>{row.connectionLabel}</strong>
              <em>Why this is here</em>
              <small>{row.whyHere}</small>
            </span>
            <b>{row.unreadCount || row.actionCount}</b>
            <dl>
              <div>
                <dt>Urgency</dt>
                <dd>{row.handoff.urgency}</dd>
              </div>
              <div>
                <dt>Status</dt>
                <dd>{row.handoff.status.replaceAll("_", " ")}</dd>
              </div>
              <div>
                <dt>Changed</dt>
                <dd>{row.lastMeaningfulChange}</dd>
              </div>
            </dl>
          </button>
        ))}
      </div>
    </aside>
  );
}

function ConversationHeader({
  detail,
  gatewayStatus,
  gatewayRoute,
}: {
  detail: ConversationDetail;
  gatewayStatus: GatewayStatus;
  gatewayRoute: string;
}) {
  return (
    <header className="conversation-header">
      <div>
        <span className="eyebrow">Relationship Conversation</span>
        <h1 id="conversation-title">{detail.title}</h1>
        <p>{detail.subtitle}</p>
      </div>
      <div className="connection-state" aria-label="Gateway and presence">
        <span className={`status-pill ${gatewayStatus === "rejected" || gatewayStatus === "offline" ? "status-error" : gatewayStatus === "replaying" ? "status-warn" : "status-ok"}`}>
          {gatewayLabel(gatewayStatus)}
        </span>
        <span className="status-pill">{CONVERSATION_GATEWAY_SCHEMA_VERSION}</span>
        <span className="status-pill">{gatewayRoute}</span>
      </div>
    </header>
  );
}

function RecoveryBanner({
  gatewayStatus,
  message,
  onReconnect,
  onSimulateOffline,
}: {
  gatewayStatus: GatewayStatus;
  message?: string;
  onReconnect: () => void;
  onSimulateOffline: () => void;
}) {
  if (gatewayStatus === "connected") {
    return (
      <div className="recovery-banner stable" role="status" aria-label="Connection status">
        <span>Connection steady</span>
        <button type="button" className="button-secondary compact-button" onClick={onSimulateOffline}>
          Simulate offline
        </button>
      </div>
    );
  }

  return (
    <div className={`recovery-banner ${gatewayStatus}`} role="status" aria-label="Connection status">
      <span>{message ?? gatewayLabel(gatewayStatus)}</span>
      <button type="button" className="button-secondary compact-button" onClick={onReconnect}>
        Reconnect and replay
      </button>
    </div>
  );
}

function NarrativeBrief({ detail, isStaff }: { detail: ConversationDetail; isStaff: boolean }) {
  return (
    <section className="brief-panel narrative-brief conversation-brief" aria-label="Narrative brief">
      <div className="brief-heading-row">
        <div>
          <span className="eyebrow">{isStaff ? "Handoff Brief" : "Brief"}</span>
          <h2 className="panel-title">{detail.connectionLabel}</h2>
        </div>
        <span className="status-pill">{detail.mode.replaceAll("_", " ")}</span>
      </div>
      <div className="brief-grid">
        <BriefBlock title="What is happening" text={detail.narrativeBrief.whatIsHappening} />
        <BriefBlock title="What changed" text={detail.narrativeBrief.whatChanged} />
        <BriefBlock title="What to do next" text={detail.narrativeBrief.nextStep} />
        <BriefBlock title="Why it matters" text={detail.narrativeBrief.whyItMatters} />
        <BriefBlock title="Evidence" text={detail.narrativeBrief.evidence} />
        <BriefBlock title="Limitations" text={detail.narrativeBrief.limitation} />
      </div>
    </section>
  );
}

function PersuasionGuidance({ detail, isStaff }: { detail: ConversationDetail; isStaff: boolean }) {
  if (!isStaff || !detail.persuasionGuidance) {
    return null;
  }

  return (
    <section className="persuasion-panel" aria-label="Ethical persuasion guidance">
      <div className="brief-heading-row">
        <div>
          <span className="eyebrow">Staff Guidance</span>
          <h2 className="panel-title">Ethical business persuasion</h2>
        </div>
        <span className="status-pill">{detail.persuasionGuidance.slotVersion}</span>
      </div>
      <div className="brief-grid">
        <BriefBlock title="Slot" text={`${detail.persuasionGuidance.slotId} / ${detail.persuasionGuidance.useCase}`} />
        <BriefBlock title="Principles" text={detail.persuasionGuidance.principles.join(", ")} />
        <BriefBlock title="Reasoning" text={detail.persuasionGuidance.staffReasoning} />
        <BriefBlock title="Client-safe language" text={detail.persuasionGuidance.clientSafeSuggestion} />
        <BriefBlock title="Evidence" text={detail.persuasionGuidance.evidenceRefs.join(", ")} />
        <BriefBlock title="Sources" text={detail.persuasionGuidance.sourceRefs.join(", ")} />
      </div>
    </section>
  );
}

function ArtifactCards({ cards, isStaff }: { cards: readonly ArtifactBriefCard[]; isStaff: boolean }) {
  if (cards.length === 0) {
    return null;
  }

  return (
    <section className="artifact-card-panel" aria-label={isStaff ? "Artifact cards" : "Deliverable cards"}>
      <div className="brief-heading-row">
        <div>
          <span className="eyebrow">{isStaff ? "Artifacts" : "Deliverables"}</span>
          <h2 className="panel-title">{isStaff ? "Artifact briefs" : "Your deliverables"}</h2>
        </div>
        <span className="status-pill">{cards.length}</span>
      </div>
      <div className="artifact-card-grid">
        {cards.map((card) => (
          <article key={card.id} className="artifact-card">
            <header>
              <span className="eyebrow">{isStaff ? "Artifact" : "Deliverable"}</span>
              <h3>{isStaff ? card.systemLabel : card.clientLabel}</h3>
            </header>
            <dl>
              <div>
                <dt>Value</dt>
                <dd>{card.value}</dd>
              </div>
              <div>
                <dt>Use</dt>
                <dd>{card.use}</dd>
              </div>
              <div>
                <dt>Next action</dt>
                <dd>{card.nextAction}</dd>
              </div>
              {isStaff ? (
                <>
                  <div>
                    <dt>Producing job</dt>
                    <dd>{card.producingJob}</dd>
                  </div>
                  <div>
                    <dt>Provenance</dt>
                    <dd>{card.provenance}</dd>
                  </div>
                  <div>
                    <dt>Storage health</dt>
                    <dd>{card.storageHealth}</dd>
                  </div>
                </>
              ) : null}
            </dl>
          </article>
        ))}
      </div>
    </section>
  );
}

const MessageTimeline = function MessageTimeline({
  ref,
  detail,
  isStaff,
  lastError,
  onEditMessage,
  onUndoMessage,
  onMarkUnread,
  onReact,
  onRetry,
}: {
  ref: Ref<HTMLElement>;
  detail: ConversationDetail;
  isStaff: boolean;
  lastError?: string;
  onEditMessage: (messageId: string, body: string) => void;
  onUndoMessage: (messageId: string) => void;
  onMarkUnread: () => void;
  onReact: (messageId: string, reactionKey: string) => void;
  onRetry: () => void;
}) {
  return (
    <section className="timeline-panel" aria-label="Conversation timeline" aria-live="polite" ref={ref}>
      {lastError ? (
        <div className="gateway-error" role="status">
          <strong>Gateway rejected command</strong>
          <span>{lastError}</span>
          <button type="button" className="button-secondary compact-button" onClick={onRetry}>
            Retry
          </button>
        </div>
      ) : null}

      {detail.messages.map((message, index) => (
        <MessageBubble
          key={message.id}
          message={message}
          showUnreadDivider={detail.unreadFromSequence === message.sequence}
          latest={index === detail.messages.length - 1}
          isStaff={isStaff}
          onEditMessage={onEditMessage}
          onUndoMessage={onUndoMessage}
          onMarkUnread={onMarkUnread}
          onReact={onReact}
        />
      ))}

      <div className="presence-row" aria-label="Typing and presence">
        {detail.typing.length > 0 ? <span className="typing-pill">{detail.typing.join(", ")}...</span> : null}
        {detail.presence.map((presence) => (
          <span key={presence} className="status-pill">
            {presence}
          </span>
        ))}
      </div>
    </section>
  );
};

function MessageBubble({
  message,
  showUnreadDivider,
  latest,
  isStaff,
  onEditMessage,
  onUndoMessage,
  onMarkUnread,
  onReact,
}: {
  message: ConversationMessage;
  showUnreadDivider: boolean;
  latest: boolean;
  isStaff: boolean;
  onEditMessage: (messageId: string, body: string) => void;
  onUndoMessage: (messageId: string) => void;
  onMarkUnread: () => void;
  onReact: (messageId: string, reactionKey: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(message.body);
  const mine = message.author === "staff";

  return (
    <>
      {showUnreadDivider ? <div className="unread-divider" data-unread-divider="true">Unread</div> : null}
      <article className={`message-bubble ${mine ? "mine" : ""} ${message.state}`} aria-label={`${message.authorLabel} message`} data-latest-message={latest ? "true" : undefined}>
        <header>
          <strong>{message.authorLabel}</strong>
          <span>{message.occurredAt}</span>
        </header>
        {message.state === "deleted" ? (
          <p className="message-tombstone">Message undone. Tombstone retained for conversation integrity.</p>
        ) : editing ? (
          <form
            className="edit-form"
            onSubmit={(event) => {
              event.preventDefault();
              onEditMessage(message.id, draft);
              setEditing(false);
            }}
          >
            <textarea className="text-input text-area compact" value={draft} onChange={(event) => setDraft(event.target.value)} aria-label="Edit message" />
            <div className="inline-actions">
              <button type="submit" className="button-primary compact-button" aria-label={`Save edit for ${message.authorLabel} message`}>
                Save edit
              </button>
              <button type="button" className="button-secondary compact-button" aria-label={`Cancel edit for ${message.authorLabel} message`} onClick={() => setEditing(false)}>
                Cancel
              </button>
            </div>
          </form>
        ) : (
          <p>{message.body}</p>
        )}
        <footer>
          <span>{message.edited ? "Edited · " : ""}{message.receiptLabel}</span>
          <div className="message-actions">
            {message.reactions.map((reaction) => (
              <button
                type="button"
                key={reaction.key}
                className={reaction.selectedByViewer ? "reaction-button selected" : "reaction-button"}
                onClick={() => onReact(message.id, reaction.key)}
              >
                {reaction.label} {reaction.count}
              </button>
            ))}
            <button type="button" className="reaction-button" onClick={() => onReact(message.id, "ack")}>
              Ack
            </button>
            {isStaff && message.canEdit && message.state !== "deleted" ? (
              <button type="button" className="message-action-button" aria-label={`Edit ${message.authorLabel} message`} onClick={() => setEditing(true)}>
                Edit
              </button>
            ) : null}
            {isStaff && message.canUndo && message.state !== "deleted" ? (
              <button type="button" className="message-action-button" aria-label={`Undo ${message.authorLabel} message`} onClick={() => onUndoMessage(message.id)}>
                Undo
              </button>
            ) : null}
            {isStaff ? (
              <button type="button" className="message-action-button" aria-label={`Mark unread from ${message.authorLabel} message`} onClick={onMarkUnread}>
                Mark unread
              </button>
            ) : null}
          </div>
        </footer>
      </article>
    </>
  );
}

function ConversationComposer({
  status,
  onSendMessage,
  onTypingStart,
}: {
  status: ComposerStatus;
  onSendMessage: (body: string) => void;
  onTypingStart: () => void;
}) {
  const [draft, setDraft] = useState("");
  const disabled = status === "sending";

  return (
    <form
      className="conversation-composer"
      aria-label="Conversation composer"
      onSubmit={(event) => {
        event.preventDefault();
        if (draft.trim().length === 0) {
          return;
        }
        onSendMessage(draft.trim());
        setDraft("");
      }}
    >
      <textarea
        className="text-input composer-input"
        value={draft}
        disabled={disabled}
        placeholder="Write a reply"
        aria-label="Write a reply"
        rows={2}
        onFocus={onTypingStart}
        onChange={(event) => {
          onTypingStart();
          setDraft(event.target.value);
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault();
            event.currentTarget.form?.requestSubmit();
          }
        }}
      />
      <button type="submit" className="button-primary send-button" disabled={disabled || draft.trim().length === 0}>
        {status === "sending" ? "Sending" : "Send"}
      </button>
      <span className={status === "failed" ? "composer-state failed" : "composer-state"}>{status === "failed" ? "Retry needed" : "Local echo ready"}</span>
    </form>
  );
}

function BriefBlock({ title, text }: { title: string; text: string }) {
  return (
    <div className="brief-block">
      <span>{title}</span>
      <p>{text}</p>
    </div>
  );
}

function useGatewayConversation(initialConversationId: string, role: ProductRole) {
  const initialDetail = sampleConversationDetails[initialConversationId] ?? sampleConversationDetails.conv_ava;
  const [state, setState] = useState<GatewayState>({
    selectedConversationId: initialDetail.conversationId,
    detail: initialDetail,
      gatewayStatus: "connected",
      composerStatus: "idle",
      sequence: initialDetail.messages.at(-1)?.sequence ?? 0,
      cursor: initialDetail.messages.at(-1)?.sequence ?? 0,
    });

  return useMemo(
    () => ({
      ...state,
      selectConversation(conversationId: string) {
        const nextDetail = sampleConversationDetails[conversationId] ?? state.detail;
        setState({
          selectedConversationId: nextDetail.conversationId,
          detail: nextDetail,
          gatewayStatus: "connected",
          composerStatus: "idle",
          sequence: nextDetail.messages.at(-1)?.sequence ?? 0,
          cursor: nextDetail.messages.at(-1)?.sequence ?? 0,
        });
      },
      sendMessage(body: string) {
        const clientId = createClientId("message.submit");
        const failed = body.toLowerCase().includes("fail");
        const holdPending = body.toLowerCase().includes("hold");
        const nextSequence = state.sequence + 1;
        const sentByStaff = role === "staff" || role === "manager" || role === "owner" || role === "admin";
        const pendingMessage: ConversationMessage = {
          id: failed ? clientId : `message_${nextSequence}`,
          clientId,
          author: sentByStaff ? "staff" : "client",
          authorLabel: sentByStaff ? "Keith" : "You",
          body,
          occurredAt: "Now",
          sequence: nextSequence,
          state: failed ? "failed" : "pending",
          edited: false,
          canEdit: !failed,
          canUndo: !failed,
          receiptLabel: failed ? "Failed" : "Pending",
          reactions: [],
        };
        setState((current) => ({
          ...current,
          detail: { ...current.detail, messages: [...current.detail.messages, pendingMessage], typing: failed ? [] : current.detail.typing },
          gatewayStatus: failed ? "rejected" : "pending",
          composerStatus: failed ? "failed" : "sending",
          lastClientId: clientId,
          lastError: failed ? "The mock gateway returned command.rejected for this body." : undefined,
          sequence: nextSequence,
          cursor: failed ? current.cursor : nextSequence,
          recoveryMessage: failed ? "Command rejected. Inspect and retry when ready." : current.recoveryMessage,
        }));
        if (!failed && !holdPending) {
          window.setTimeout(() => {
            updateMessage(setState, pendingMessage.id, (message) => ({
              ...message,
              state: "persisted",
              receiptLabel: `Ack ${clientId}`,
            }));
            setState((current) => ({ ...current, gatewayStatus: "connected", composerStatus: "idle", cursor: Math.max(current.cursor, nextSequence) }));
          }, 80);
        }
      },
      editMessage(messageId: string, body: string) {
        updateMessage(setState, messageId, (message) => ({
          ...message,
          body,
          edited: true,
          receiptLabel: `Edited · ${createClientId("message.edit")}`,
        }));
      },
      undoMessage(messageId: string) {
        updateMessage(setState, messageId, (message) => ({
          ...message,
          body: "",
          state: "deleted",
          canEdit: false,
          canUndo: false,
          receiptLabel: `Undone · ${createClientId("message.undo")}`,
        }));
      },
      markRead() {
        setState((current) => ({
          ...current,
          recoveryMessage: "Read position advanced without moving the transcript.",
          detail: {
            ...current.detail,
            lastReadSequence: current.sequence,
            unreadFromSequence: 0,
            messages: current.detail.messages.map((message) => ({ ...message, receiptLabel: message.state === "deleted" ? message.receiptLabel : "Read" })),
          },
        }));
      },
      markUnread() {
        setState((current) => ({
          ...current,
          recoveryMessage: "Unread anchor restored from durable read state.",
          detail: {
            ...current.detail,
            unreadFromSequence: Math.max(1, current.sequence),
            lastReadSequence: Math.max(0, current.sequence - 1),
          },
        }));
      },
      toggleReaction(messageId: string, reactionKey: string) {
        updateMessage(setState, messageId, (message) => {
          const existing = message.reactions.find((reaction) => reaction.key === reactionKey);
          if (!existing) {
            return {
              ...message,
              reactions: [...message.reactions, { key: reactionKey, label: labelForReaction(reactionKey), count: 1, selectedByViewer: true }],
            };
          }
          return {
            ...message,
            reactions: message.reactions.map((reaction) =>
              reaction.key === reactionKey
                ? {
                    ...reaction,
                    selectedByViewer: !reaction.selectedByViewer,
                    count: reaction.selectedByViewer ? Math.max(0, reaction.count - 1) : reaction.count + 1,
                  }
                : reaction,
            ),
          };
        });
      },
      retryLastMessage() {
        setState((current) => ({
          ...current,
          gatewayStatus: "connected",
          composerStatus: "idle",
          lastError: undefined,
          recoveryMessage: "Command retried and reconciled by clientId.",
          detail: {
            ...current.detail,
            messages: current.detail.messages.map((message) =>
              message.state === "failed"
                ? { ...message, state: "persisted", canEdit: true, canUndo: true, receiptLabel: `Retried · ${message.clientId ?? "client_retry"}` }
                : message,
            ),
          },
        }));
      },
      simulateOffline() {
        setState((current) => ({
          ...current,
          gatewayStatus: "offline",
          composerStatus: current.composerStatus === "sending" ? "sending" : "idle",
          recoveryMessage: "Offline. Local drafts stay usable and eligible commands wait for replay.",
        }));
      },
      reconnectAndReplay() {
        setState((current) => {
          const pendingMessages = current.detail.messages.filter((message) => message.state === "pending");
          const replayedMessages =
            pendingMessages.length > 0
              ? current.detail.messages.map((message) =>
                  message.state === "pending"
                    ? {
                        ...message,
                        state: "persisted" as const,
                        receiptLabel: `Replayed · ${message.clientId ?? "client_replay"}`,
                      }
                    : message,
                )
              : current.detail.messages.some((message) => message.id === "message_replay_recovery")
                ? current.detail.messages
                : [
                    ...current.detail.messages,
                    {
                      id: "message_replay_recovery",
                      author: "assistant" as const,
                      authorLabel: "Ordo Assistant",
                      body: "Recovered missed durable conversation state after reconnect.",
                      occurredAt: "Recovered",
                      sequence: current.sequence + 1,
                      state: "persisted" as const,
                      edited: false,
                      canEdit: false,
                      canUndo: false,
                      receiptLabel: "Replayed",
                      reactions: [],
                    },
                  ];
          const nextSequence = Math.max(current.sequence, replayedMessages.at(-1)?.sequence ?? current.sequence);
          return {
            ...current,
            gatewayStatus: "recovered",
            composerStatus: "idle",
            lastError: undefined,
            sequence: nextSequence,
            cursor: Math.max(current.cursor, nextSequence),
            recoveryMessage:
              pendingMessages.length > 0
                ? "Recovered. Pending optimistic messages reconciled by clientId without duplicates."
                : "Recovered. Missed durable events replayed from the latest cursor.",
            detail: {
              ...current.detail,
              messages: replayedMessages,
            },
          };
        });
      },
    }),
    [role, state],
  );
}

function gatewayLabel(gatewayStatus: GatewayStatus): string {
  if (gatewayStatus === "offline") {
    return "Offline";
  }
  if (gatewayStatus === "pending") {
    return "Command pending";
  }
  if (gatewayStatus === "rejected") {
    return "Command rejected";
  }
  if (gatewayStatus === "replaying") {
    return "Replaying";
  }
  if (gatewayStatus === "recovered") {
    return "Recovered";
  }
  return "Gateway connected";
}

function updateMessage(
  setState: Dispatch<SetStateAction<GatewayState>>,
  messageId: string,
  updater: (message: ConversationMessage) => ConversationMessage,
) {
  setState((current) => ({
    ...current,
    detail: {
      ...current.detail,
      messages: current.detail.messages.map((message) => (message.id === messageId ? updater(message) : message)),
    },
  }));
}

function createClientId(commandType: ConversationCommandType): string {
  const suffix = Math.random().toString(36).slice(2, 8);
  return `${commandType.replaceAll(".", "_")}_${suffix}`;
}

function labelForReaction(reactionKey: string): string {
  if (reactionKey === "ack") {
    return "Ack";
  }
  if (reactionKey === "clear") {
    return "Clear";
  }
  if (reactionKey === "useful") {
    return "Useful";
  }
  return reactionKey;
}
