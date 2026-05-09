export const CONVERSATION_GATEWAY_SCHEMA_VERSION = "conversation.gateway.v1" as const;
export const CONVERSATION_GATEWAY_ROUTE = "/chat/ws" as const;

export type ConversationGatewayOp =
  | "hello"
  | "identify"
  | "subscribe"
  | "unsubscribe"
  | "command"
  | "dispatch"
  | "ack"
  | "heartbeat"
  | "resume"
  | "replay"
  | "error";

export type ConversationGatewayDurability = "durable" | "ephemeral" | "read_model";
export type ConversationGatewayScope = "connection" | "user" | "conversation" | "system" | "run";

export type ConversationCommandType =
  | "conversation.subscribe"
  | "conversation.replay_after_cursor"
  | "message.submit"
  | "message.edit"
  | "message.delete"
  | "message.undo"
  | "message.react"
  | "message.mark_read"
  | "message.mark_unread"
  | "typing.start"
  | "typing.stop"
  | "presence.update"
  | "handoff.accept"
  | "handoff.decline"
  | "handoff.assign"
  | "handoff.return_to_agent"
  | "agent.delegate"
  | "agent.takeover";

export type ConversationGatewayEnvelope<TPayload = unknown> = {
  schemaVersion: typeof CONVERSATION_GATEWAY_SCHEMA_VERSION;
  op: ConversationGatewayOp;
  type: string;
  clientId?: string;
  serverId?: string;
  conversationId?: string;
  segmentId?: string;
  sequence?: number;
  cursor?: number;
  durability: ConversationGatewayDurability;
  scope: ConversationGatewayScope;
  payload: TPayload;
  occurredAt: string;
};

export type ConversationGatewayErrorPayload = {
  code:
    | "invalid_envelope"
    | "unsupported_protocol_version"
    | "unsupported_operation"
    | "unsupported_command"
    | "command_failed"
    | "client_lagged"
    | "auth_required"
    | "policy_denied"
    | "review_required"
    | "rate_limited"
    | "conversation_not_found"
    | "participant_not_found"
    | "idempotency_conflict"
    | "provider_unavailable"
    | "privacy_transform_failed"
    | "token_budget_exceeded";
  message: string;
  policyDecisionId?: string;
  retryable: boolean;
};

export type ConversationReplayCursor = {
  conversationId: string;
  afterSequence: number;
  afterCursor?: number;
  limit: number;
};

export const conversationCommandCapabilities: Record<ConversationCommandType, string> = {
  "conversation.subscribe": "conversation.read",
  "conversation.replay_after_cursor": "conversation.read",
  "message.submit": "conversation.message.create",
  "message.edit": "conversation.message.edit",
  "message.delete": "conversation.message.delete",
  "message.undo": "conversation.message.delete",
  "message.react": "conversation.reaction.write",
  "message.mark_read": "conversation.receipt.write",
  "message.mark_unread": "conversation.receipt.write",
  "typing.start": "conversation.presence.write",
  "typing.stop": "conversation.presence.write",
  "presence.update": "conversation.presence.write",
  "handoff.accept": "conversation.handoff.manage",
  "handoff.decline": "conversation.handoff.manage",
  "handoff.assign": "conversation.handoff.manage",
  "handoff.return_to_agent": "conversation.handoff.manage",
  "agent.delegate": "conversation.agent.delegate",
  "agent.takeover": "conversation.agent.delegate",
};

export const conversationProtocolFixtures = {
  subscribe: {
    schemaVersion: CONVERSATION_GATEWAY_SCHEMA_VERSION,
    op: "command",
    type: "conversation.subscribe",
    clientId: "client_subscribe_1",
    conversationId: "conversation_1",
    durability: "ephemeral",
    scope: "conversation",
    payload: { afterSequence: 0, afterCursor: 0 },
    occurredAt: "2026-05-09T00:00:00Z",
  } satisfies ConversationGatewayEnvelope,
  messageSubmit: {
    schemaVersion: CONVERSATION_GATEWAY_SCHEMA_VERSION,
    op: "command",
    type: "message.submit",
    clientId: "client_message_1",
    conversationId: "conversation_1",
    durability: "durable",
    scope: "conversation",
    payload: { bodyMarkdown: "Hello", clientMessageId: "client_msg_1" },
    occurredAt: "2026-05-09T00:00:00Z",
  } satisfies ConversationGatewayEnvelope,
  policyDenied: {
    schemaVersion: CONVERSATION_GATEWAY_SCHEMA_VERSION,
    op: "error",
    type: "command.rejected",
    clientId: "client_message_1",
    durability: "ephemeral",
    scope: "user",
    payload: {
      code: "policy_denied",
      message: "Message submission is not allowed for this conversation.",
      policyDecisionId: "policy_decision_1",
      retryable: false,
    },
    occurredAt: "2026-05-09T00:00:00Z",
  } satisfies ConversationGatewayEnvelope<ConversationGatewayErrorPayload>,
};
