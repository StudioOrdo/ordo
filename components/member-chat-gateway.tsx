"use client";

import { FormEvent, useEffect, useRef, useState } from "react";

import {
  CONVERSATION_GATEWAY_SCHEMA_VERSION,
  type ConversationGatewayDurability,
  type ConversationGatewayEnvelope,
  type ConversationGatewayOp,
  type ConversationGatewayScope,
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
  tone: "member" | "ordo" | "safe_status";
  body: string;
  status?: "local" | "acknowledged" | "persisted" | "llm_requested" | "llm_streaming" | "llm_completed" | "llm_failed";
  messageId?: string;
}

type RunState = "checking" | "degraded" | "connecting" | "connected" | "failed";

interface GatewayAckPayload {
  messageId?: string;
}

interface MessageCreatedPayload {
  messageId?: string;
  clientMessageId?: string;
}

interface LlmDeltaPayload {
  runId?: string;
  delta?: unknown;
}

interface LlmRunPayload {
  runId?: string;
  code?: unknown;
  message?: unknown;
}

interface ChatStreamEventPayload {
  delta?: unknown;
  message?: unknown;
  error?: unknown;
}

const OFFLINE_REASON = "Conversation gateway unavailable; reconnecting to the local daemon.";
const CHAT_BOOTSTRAP_RETRY_MS = 2_500;

export function MemberChatGatewayComposer() {
  const socketRef = useRef<WebSocket | null>(null);
  const bootstrapRef = useRef<ChatBootstrapReadModel | null>(null);
  const submittedMessagesRef = useRef(new Map<string, string>());
  const messageSequenceRef = useRef(0);
  const [runState, setRunState] = useState<RunState>("checking");
  const [bootstrap, setBootstrap] = useState<ChatBootstrapReadModel | null>(null);
  const [degradedReason, setDegradedReason] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [messages, setMessages] = useState<PreviewMessage[]>([]);

  useEffect(() => {
    let cancelled = false;
    let retryTimer: ReturnType<typeof setTimeout> | null = null;

    function scheduleBootstrapRetry() {
      if (cancelled || retryTimer) return;
      retryTimer = setTimeout(() => {
        retryTimer = null;
        bootstrapChat();
      }, CHAT_BOOTSTRAP_RETRY_MS);
    }

    async function bootstrapChat() {
      try {
        const response = await fetch("/api/chat/bootstrap", { method: "POST" });
        const payload = (await response.json()) as ChatBootstrapResponse;
        if (cancelled) return;
        if (!response.ok || payload.status !== "ready" || !payload.bootstrap?.transport.url) {
          setRunState("degraded");
          setDegradedReason(payload.degradedReason ?? OFFLINE_REASON);
          scheduleBootstrapRetry();
          return;
        }

        setBootstrap(payload.bootstrap);
        bootstrapRef.current = payload.bootstrap;
        setRunState("connecting");
        const socket = new WebSocket(payload.bootstrap.transport.url);
        socketRef.current = socket;

        socket.addEventListener("open", () => {
          socket.send(JSON.stringify(gatewayEnvelope("identify", "gateway.identify", null, { actorId: payload.bootstrap?.actorId, participantId: payload.bootstrap?.participantId })));
          socket.send(JSON.stringify(gatewayEnvelope("command", "conversation.subscribe", payload.bootstrap?.conversationId ?? null, { afterSequence: 0, afterCursor: 0, limit: 50 })));
          setRunState("connected");
          setDegradedReason(null);
        });

        socket.addEventListener("message", (event) => {
          const frame = parseGatewayFrame(event.data);
          if (frame?.op === "ack") handleAckFrame(frame);
          else if (frame?.op === "dispatch") handleDispatchFrame(frame);
          else if (frame?.op === "error") handleErrorFrame(frame);
        });

        socket.addEventListener("close", () => {
          if (!cancelled) {
            socketRef.current = null;
            setRunState("degraded");
            setDegradedReason(OFFLINE_REASON);
            scheduleBootstrapRetry();
          }
        });
        socket.addEventListener("error", () => {
          setRunState("failed");
          setDegradedReason(OFFLINE_REASON);
          scheduleBootstrapRetry();
        });
      } catch {
        if (!cancelled) {
          setRunState("degraded");
          setDegradedReason(OFFLINE_REASON);
          scheduleBootstrapRetry();
        }
      }
    }

    bootstrapChat();
    return () => {
      cancelled = true;
      if (retryTimer) clearTimeout(retryTimer);
      socketRef.current?.close();
      socketRef.current = null;
      bootstrapRef.current = null;
    };
  }, []);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const body = draft.trim();
    if (!body) return;

    messageSequenceRef.current += 1;
    const clientMessageId = `browser_message_${Date.now()}_${messageSequenceRef.current}`;
    submittedMessagesRef.current.set(clientMessageId, body);
    setDraft("");
    setMessages((current) => [...current, { id: clientMessageId, speaker: "You", tone: "member", body, status: "local" }]);

    const socket = socketRef.current;
    const activeBootstrap = bootstrapRef.current ?? bootstrap;
    const runId = `llm_run_${clientMessageId}`;
    setMessages((current) => (current.some((message) => message.id === runId) ? current : [...current, { id: runId, speaker: "Ordo", tone: "safe_status", body: "", status: "llm_requested" }]));

    if (runState === "connected" && activeBootstrap && socket?.readyState === WebSocket.OPEN) {
      socket.send(
        JSON.stringify(
          gatewayEnvelope("command", "message.submit", activeBootstrap.conversationId, {
            participantId: activeBootstrap.participantId,
            bodyMarkdown: body,
            clientMessageId,
            messageKind: "human",
            visibility: "participants",
          }),
        ),
      );

      void streamAssistantReply(runId, body, false);
    } else {
      void streamAssistantReply(runId, body, true);
    }
  }

  function handleAckFrame(frame: ConversationGatewayEnvelope) {
    if (frame.type === "identify.ack" || frame.type === "conversation.subscribe.ack") {
      setRunState("connected");
      setDegradedReason(null);
      return;
    }

    const payload = frame.payload as GatewayAckPayload;
    if (frame.type === "message.submit.ack" && frame.clientId) {
      setMessages((current) => current.map((message) => (message.id === frame.clientId ? { ...message, status: "acknowledged", messageId: payload.messageId } : message)));
    }

    if (frame.type === "llm.run.request.ack") {
      const runId = (frame.payload as LlmRunPayload).runId;
      if (typeof runId === "string") {
        setMessages((current) => current.map((message) => (message.id === runId ? { ...message, status: "llm_requested" } : message)));
      }
    }
  }

  function handleDispatchFrame(frame: ConversationGatewayEnvelope) {
    if (frame.type === "command.rejected") {
      handleErrorFrame(frame);
      return;
    }

    if (frame.type === "llm.text.delta") {
      const payload = frame.payload as LlmDeltaPayload;
      if (typeof payload.runId === "string" && typeof payload.delta === "string") appendAssistantDelta(payload.runId, payload.delta);
      return;
    }

    if (frame.type === "llm.text.completed" || frame.type === "llm.run.completed") {
      const runId = (frame.payload as LlmRunPayload).runId;
      if (typeof runId === "string") completeAssistantRun(runId);
      return;
    }

    if (frame.type === "llm.run.failed") {
      const payload = frame.payload as LlmRunPayload;
      if (typeof payload.runId === "string") failAssistantRun(payload.runId, safeErrorMessage(payload));
      return;
    }

    if (frame.type !== "message.created") return;
    const payload = frame.payload as MessageCreatedPayload;
    if (!payload.clientMessageId) return;
    setMessages((current) => current.map((message) => (message.id === payload.clientMessageId ? { ...message, status: "persisted", messageId: payload.messageId ?? message.messageId } : message)));
  }

  function handleErrorFrame(frame: ConversationGatewayEnvelope) {
    setRunState("failed");
    setDegradedReason(safeErrorMessage(frame.payload));
  }

  function appendAssistantDelta(runId: string, delta: string) {
    setMessages((current) => current.map((message) => (message.id === runId ? { ...message, tone: "ordo", status: "llm_streaming", body: `${message.body}${delta}` } : message)));
  }

  function completeAssistantRun(runId: string) {
    setMessages((current) =>
      current.map((message) =>
        message.id === runId
          ? {
              ...message,
              status: message.body.trim() ? "llm_completed" : "llm_failed",
              tone: message.body.trim() ? message.tone : "safe_status",
              body: message.body.trim() ? message.body : "No live reply returned. Try again from this conversation.",
            }
          : message,
      ),
    );
  }

  function failAssistantRun(runId: string, error: string) {
    setMessages((current) => {
      const body = safeErrorMessage({ message: error });
      const next = current.map((message) => (message.id === runId ? { ...message, tone: "safe_status" as const, status: "llm_failed" as const, body } : message));
      return next.some((message) => message.id === runId) ? next : [...next, { id: runId, speaker: "Ordo", tone: "safe_status", status: "llm_failed", body }];
    });
  }

  async function streamAssistantReply(runId: string, body: string, failOnUnavailable: boolean) {
    try {
      const response = await fetch("/api/chat/stream", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ message: body }),
      });

      if (!response.ok || !response.body) {
        if (failOnUnavailable) {
          failAssistantRun(runId, degradedReason ?? OFFLINE_REASON);
        }
        return;
      }

      await readChatStream(response, runId);
    } catch {
      if (failOnUnavailable) {
        failAssistantRun(runId, degradedReason ?? OFFLINE_REASON);
      }
    }
  }

  async function readChatStream(response: Response, runId: string) {
    const reader = response.body?.getReader();
    if (!reader) return;

    const decoder = new TextDecoder();
    let buffer = "";

    while (true) {
      const { value, done } = await reader.read();
      buffer += decoder.decode(value ?? new Uint8Array(), { stream: !done });
      const frames = buffer.split(/\n\n/);
      buffer = frames.pop() ?? "";

      for (const frame of frames) {
        handleChatStreamFrame(frame, runId);
      }

      if (done) break;
    }

    if (buffer.trim()) {
      handleChatStreamFrame(buffer, runId);
    }
  }

  function handleChatStreamFrame(frame: string, runId: string) {
    const lines = frame.split(/\n/);
    const event = lines.find((line) => line.startsWith("event:"))?.slice("event:".length).trim() ?? "message";
    const data = lines
      .filter((line) => line.startsWith("data:"))
      .map((line) => line.slice("data:".length).trim())
      .join("\n");
    const payload = parseChatStreamPayload(data);

    if (event === "delta" && typeof payload.delta === "string") {
      appendAssistantDelta(runId, payload.delta);
      return;
    }

    if (event === "completed") {
      completeAssistantRun(runId);
      return;
    }

    if (event === "error") {
      failAssistantRun(runId, safeErrorMessage(payload));
    }
  }

  return (
    <section className="member-stage-composer-wrap" aria-label="Message Ordo">
      <p>{statusLabel(runState, degradedReason)}</p>
      {messages.length ? (
        <div className="member-chat-preview-run" aria-label="Live chat run state">
          {runState === "degraded" && degradedReason ? <p className="member-chat-safe-status">{degradedReason}</p> : null}
          {messages.map((message) => (
            <div key={message.id} className={`member-conversation-message member-conversation-message-${message.tone}`}>
              <div className="member-conversation-message-header">
                <strong>{message.speaker}</strong>
                {message.status ? <span>{messageStatusLabel(message.status)}</span> : null}
              </div>
              {message.status === "llm_requested" ? (
                <p className="member-chat-typing-indicator" aria-label="Drafting answer">
                  <span />
                  <span />
                  <span />
                </p>
              ) : (
                <p>{message.body}</p>
              )}
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
        <input id="member-message-ordo" type="text" placeholder="Message Ordo" value={draft} onChange={(event) => setDraft(event.target.value)} />
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

function gatewayEnvelope(op: ConversationGatewayOp, type: string, conversationId: string | null, payload: Record<string, unknown>): ConversationGatewayEnvelope {
  return {
    schemaVersion: CONVERSATION_GATEWAY_SCHEMA_VERSION,
    op,
    type,
    clientId: payload.clientMessageId && typeof payload.clientMessageId === "string" ? payload.clientMessageId : `browser_${type.replace(/[^a-z0-9]+/gi, "_")}_${Date.now()}`,
    conversationId: conversationId ?? undefined,
    durability: durabilityForOp(op),
    scope: scopeForOp(op),
    payload,
    occurredAt: new Date().toISOString(),
  };
}

function parseGatewayFrame(data: unknown): ConversationGatewayEnvelope | null {
  if (typeof data !== "string") return null;
  try {
    const parsed = JSON.parse(data) as ConversationGatewayEnvelope;
    return parsed.schemaVersion === CONVERSATION_GATEWAY_SCHEMA_VERSION ? parsed : null;
  } catch {
    return null;
  }
}

function parseChatStreamPayload(data: string): ChatStreamEventPayload {
  if (!data) return {};
  try {
    const parsed = JSON.parse(data) as ChatStreamEventPayload;
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function safeErrorMessage(payload: unknown): string {
  if (!payload || typeof payload !== "object") return "Live reply failed safely. Try again from this conversation.";
  const record = payload as Record<string, unknown>;
  const candidate = typeof record.message === "string" && record.message.trim() ? record.message : typeof record.error === "string" && record.error.trim() ? record.error : "";
  if (!candidate) return "Live reply failed safely. Try again from this conversation.";
  return sanitizeSafeMessage(candidate);
}

function sanitizeSafeMessage(message: string): string {
  const trimmed = message.trim();
  if (!trimmed) return "Live reply failed safely. Try again from this conversation.";
  if (/(sk-[a-z0-9_-]+|OPENAI_API_KEY|API__OPENAI_API_KEY|Authorization|Bearer|provider payload|raw prompt|prompt)/i.test(trimmed)) {
    return "Live reply failed safely. Try again from this conversation.";
  }
  return trimmed.length > 240 ? `${trimmed.slice(0, 237)}...` : trimmed;
}

function durabilityForOp(op: ConversationGatewayOp): ConversationGatewayDurability {
  return op === "command" ? "ephemeral" : "durable";
}

function scopeForOp(op: ConversationGatewayOp): ConversationGatewayScope {
  return op === "command" ? "user" : "conversation";
}

function messageStatusLabel(status: NonNullable<PreviewMessage["status"]>): string {
  switch (status) {
    case "local":
      return "local";
    case "acknowledged":
      return "acknowledged by /chat/ws";
    case "persisted":
      return "saved by /chat/ws";
    case "llm_requested":
      return "typing";
    case "llm_streaming":
      return "streaming";
    case "llm_completed":
      return "complete";
    case "llm_failed":
      return "live reply failed";
  }
}

function statusLabel(runState: RunState, degradedReason: string | null): string {
  switch (runState) {
    case "checking":
      return "Ordo - checking local chat bootstrap";
    case "connecting":
      return "Ordo - connecting to /chat/ws";
    case "connected":
      return "Ordo - connected to /chat/ws; live replies stream through the server";
    case "failed":
      return degradedReason ?? OFFLINE_REASON;
    case "degraded":
      return degradedReason ?? OFFLINE_REASON;
  }
}
