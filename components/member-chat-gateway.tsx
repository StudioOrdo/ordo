"use client";

import { FormEvent, useEffect, useRef, useState } from "react";

import {
  CONVERSATION_GATEWAY_SCHEMA_VERSION,
  type ConversationGatewayEnvelope,
  type ConversationGatewayOp,
  type ConversationGatewayScope,
  type ConversationGatewayDurability,
} from "@/lib/conversation-protocol";

interface ChatBootstrapTransport {
  route: string;
  protocol: string;
  url?: string;
}

interface ChatBootstrapReadModel {
  actorId: string;
  conversationId: string;
  participantId: string;
  assistantParticipantId: string;
  transport: ChatBootstrapTransport;
}

interface ChatBootstrapResponse {
  authenticated: boolean;
  bootstrap: ChatBootstrapReadModel | null;
  status: "ready" | "degraded";
  degradedReason: string | null;
}

interface PreviewMessage {
  id: string;
  speaker: "You" | "Ordo";
  tone: "member" | "safe_status";
  body: string;
  status?: "local" | "acknowledged" | "persisted";
  messageId?: string;
}

type RunState = "checking" | "degraded" | "connecting" | "connected" | "failed";

interface GatewayAckPayload {
  conversationId?: string;
  messageId?: string;
}

interface MessageCreatedPayload {
  messageId?: string;
  participantId?: string;
  clientMessageId?: string;
}

const OFFLINE_REASON = "Daemon chat bootstrap route unavailable; using local preview chat.";

export function MemberChatGatewayComposer() {
  const socketRef = useRef<WebSocket | null>(null);
  const [runState, setRunState] = useState<RunState>("checking");
  const [bootstrap, setBootstrap] = useState<ChatBootstrapReadModel | null>(null);
  const [degradedReason, setDegradedReason] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [messages, setMessages] = useState<PreviewMessage[]>([]);

  useEffect(() => {
    let cancelled = false;

    async function bootstrapChat() {
      try {
        const response = await fetch("/api/chat/bootstrap", { method: "POST" });
        const payload = (await response.json()) as ChatBootstrapResponse;

        if (cancelled) return;
        if (!response.ok || payload.status !== "ready" || !payload.bootstrap?.transport.url) {
          setRunState("degraded");
          setDegradedReason(payload.degradedReason ?? OFFLINE_REASON);
          return;
        }

        setBootstrap(payload.bootstrap);
        setRunState("connecting");
        const socket = new WebSocket(payload.bootstrap.transport.url);
        socketRef.current = socket;

        socket.addEventListener("open", () => {
          socket.send(
            JSON.stringify(
              gatewayEnvelope("identify", "gateway.identify", null, {
                actorId: payload.bootstrap?.actorId,
                participantId: payload.bootstrap?.participantId,
              }),
            ),
          );
          socket.send(
            JSON.stringify(
              gatewayEnvelope("command", "conversation.subscribe", payload.bootstrap?.conversationId ?? null, {
                afterSequence: 0,
                afterCursor: 0,
                limit: 50,
              }),
            ),
          );
          setRunState("connected");
          setDegradedReason(null);
        });

        socket.addEventListener("message", (event) => {
          const frame = parseGatewayFrame(event.data);
          if (frame?.op === "ack") {
            handleAckFrame(frame);
            return;
          }
          if (frame?.op === "dispatch") {
            handleDispatchFrame(frame);
            return;
          }
          if (frame?.op === "error") {
            setRunState("failed");
            setDegradedReason(safeErrorMessage(frame.payload));
          }
        });

        socket.addEventListener("close", () => {
          if (!cancelled) {
            setRunState("degraded");
            setDegradedReason("Conversation gateway disconnected; using local preview chat.");
          }
        });

        socket.addEventListener("error", () => {
          setRunState("failed");
          setDegradedReason("Conversation gateway is unavailable; using local preview chat.");
        });
      } catch {
        if (!cancelled) {
          setRunState("degraded");
          setDegradedReason(OFFLINE_REASON);
        }
      }
    }

    bootstrapChat();
    return () => {
      cancelled = true;
      socketRef.current?.close();
      socketRef.current = null;
    };
  }, []);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const body = draft.trim();
    if (!body) return;

    const clientMessageId = `browser_message_${Date.now()}`;
    setDraft("");
    setMessages((current) => [
      ...current,
      { id: clientMessageId, speaker: "You", tone: "member", body, status: "local" },
    ]);

    const socket = socketRef.current;
    if (runState === "connected" && bootstrap && socket?.readyState === WebSocket.OPEN) {
      socket.send(
        JSON.stringify(
          gatewayEnvelope("command", "message.submit", bootstrap.conversationId, {
            participantId: bootstrap.participantId,
            bodyMarkdown: body,
            clientMessageId,
            messageKind: "human",
            visibility: "participants",
          }),
        ),
      );
      return;
    }

    setMessages((current) => [
      ...current,
      {
        id: `${clientMessageId}_offline`,
        speaker: "Ordo",
        tone: "safe_status",
        body: "Local preview only. Start the daemon to send this through the conversation gateway.",
      },
    ]);
  }

  function handleAckFrame(frame: ConversationGatewayEnvelope) {
    if (frame.type === "identify.ack" || frame.type === "conversation.subscribe.ack") {
      setRunState("connected");
      setDegradedReason(null);
      return;
    }

    if (frame.type !== "message.submit.ack" || !frame.clientId) return;
    const payload = frame.payload as GatewayAckPayload;
    setMessages((current) =>
      current.map((message) =>
        message.id === frame.clientId
          ? {
              ...message,
              status: "acknowledged",
              messageId: payload.messageId,
            }
          : message,
      ),
    );
  }

  function handleDispatchFrame(frame: ConversationGatewayEnvelope) {
    if (frame.type !== "message.created") return;
    const payload = frame.payload as MessageCreatedPayload;
    if (!payload.clientMessageId) return;

    setMessages((current) =>
      current.map((message) =>
        message.id === payload.clientMessageId
          ? {
              ...message,
              status: "persisted",
              messageId: payload.messageId ?? message.messageId,
            }
          : message,
      ),
    );
  }

  return (
    <section className="member-stage-composer-wrap" aria-label="Message Ordo">
      <p>{statusLabel(runState, degradedReason)}</p>
      {messages.length ? (
        <div className="member-chat-preview-run" aria-label="Local chat run state">
          {messages.map((message) => (
            <div key={message.id} className={`member-conversation-message member-conversation-message-${message.tone}`}>
              <div className="member-conversation-message-header">
                <strong>{message.speaker}</strong>
                {message.status ? <span>{messageStatusLabel(message.status)}</span> : null}
              </div>
              <p>{message.body}</p>
            </div>
          ))}
        </div>
      ) : null}
      <form className="member-stage-composer" onSubmit={handleSubmit}>
        <button type="button" aria-label="Add context">
          +
        </button>
        <label className="visually-hidden" htmlFor="member-message-ordo">
          Message Ordo
        </label>
        <input
          id="member-message-ordo"
          type="text"
          placeholder="Message Ordo"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
        />
        <button type="button" aria-label="Voice input">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 4a3 3 0 0 0-3 3v5a3 3 0 0 0 6 0V7a3 3 0 0 0-3-3Z" />
            <path d="M5 11a7 7 0 0 0 14 0" />
            <path d="M12 18v3" />
          </svg>
        </button>
        <button type="submit" className="member-stage-send" aria-label="Send message">
          →
        </button>
      </form>
      <small>Ordo can make mistakes. Use durable evidence for important decisions.</small>
    </section>
  );
}

function gatewayEnvelope(
  op: ConversationGatewayOp,
  type: string,
  conversationId: string | null,
  payload: Record<string, unknown>,
): ConversationGatewayEnvelope {
  return {
    schemaVersion: CONVERSATION_GATEWAY_SCHEMA_VERSION,
    op,
    type,
    clientId: payload.clientMessageId && typeof payload.clientMessageId === "string" ? payload.clientMessageId : `browser_${type.replace(/[^a-z0-9]+/gi, "_")}_${Date.now()}`,
    conversationId: conversationId ?? undefined,
    durability: op === "identify" ? "ephemeral" : ("durable" as ConversationGatewayDurability),
    scope: op === "identify" ? ("user" as ConversationGatewayScope) : ("conversation" as ConversationGatewayScope),
    payload,
    occurredAt: new Date().toISOString(),
  };
}

function parseGatewayFrame(value: unknown): ConversationGatewayEnvelope | null {
  if (typeof value !== "string") return null;
  try {
    return JSON.parse(value) as ConversationGatewayEnvelope;
  } catch {
    return null;
  }
}

function safeErrorMessage(payload: unknown): string {
  return "Conversation gateway rejected the command; using local preview chat.";
}

function messageStatusLabel(status: NonNullable<PreviewMessage["status"]>): string {
  switch (status) {
    case "local":
      return "local";
    case "acknowledged":
      return "acknowledged by /chat/ws";
    case "persisted":
      return "saved by /chat/ws";
  }
}

function statusLabel(runState: RunState, degradedReason: string | null): string {
  switch (runState) {
    case "checking":
      return "Ordo - checking local chat bootstrap";
    case "connecting":
      return "Ordo - connecting to /chat/ws";
    case "connected":
      return "Ordo - connected to /chat/ws";
    case "failed":
      return degradedReason ?? "Conversation gateway is unavailable; using local preview chat.";
    case "degraded":
      return degradedReason ?? OFFLINE_REASON;
  }
}