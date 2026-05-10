import {
  createCommandRecord,
  createLocalMessage,
  createUiError,
  hasDurableEvidence,
  initialReplayState,
  transitionCommand,
  transitionMessage,
  type ActorRef,
  type CommandEnvelope,
  type CommandRecord,
  type EvidenceRef,
  type FrontendEventEnvelope,
  type MessageRecord,
  type ReplayState,
  type StreamRecord,
  type UiError,
} from "@/lib/ordoos-frontend-contracts";
import {
  projectValuesForRole,
  type DeniedProjection,
  type ProjectionSourceValue,
} from "@/lib/ordoos-role-projection";
import { type ProductRole } from "@/lib/product-navigation";

export type RealtimeConnectionStatus =
  | "idle"
  | "connecting"
  | "connected"
  | "disconnected"
  | "replaying"
  | "failed";

export type RealtimeCommandKind =
  | "message.submit"
  | "message.edit"
  | "message.delete"
  | "message.undo"
  | "message.react"
  | "message.mark_read"
  | "message.mark_unread"
  | "conversation.subscribe"
  | "conversation.replay_after_cursor";

export interface RealtimeGatewayPort {
  connect(): Promise<{ status: RealtimeConnectionStatus }>;
  disconnect(): Promise<{ status: RealtimeConnectionStatus }>;
  sendCommand<TPayload>(command: CommandEnvelope<TPayload>): Promise<RealtimeGatewayAck>;
  replayFromCursor(cursor: string): Promise<readonly FrontendEventEnvelope<RealtimeEventPayload>[]>;
  status(): RealtimeConnectionStatus;
}

export type RealtimeGatewayAck =
  | { type: "ack"; clientId: string; acknowledgedAt: string }
  | { type: "reject"; clientId: string; rejectedAt: string; error: UiError };

export interface RealtimeCommandInput<TPayload> {
  kind: RealtimeCommandKind;
  payload: TPayload;
  actor: ActorRef;
  clientId: string;
  intentId: string;
  issuedAt: string;
}

export interface RealtimeMessagePayload {
  body?: string;
  bodyMarkdown?: string;
  authorLabel?: string;
  authorRole?: ProductRole | "ordo_agent" | "system";
  projectionValues?: readonly ProjectionSourceValue[];
}

export interface RealtimeReadModelMessage {
  localId: string;
  clientId: string;
  canonicalId?: string;
  body: string;
  state: MessageRecord["state"];
  sequence?: number;
  cursor?: string;
  evidenceRefs: readonly EvidenceRef[];
  denied: readonly DeniedProjection[];
}

export interface RealtimeReadModel {
  conversation: {
    conversationId: string;
    status: RealtimeConnectionStatus;
    lastSequence: number;
    cursor: string;
  };
  messages: readonly RealtimeReadModelMessage[];
  composer: {
    status: "idle" | "sending" | "blocked" | "recoverable_error";
    recoverableIntent: unknown | null;
    errors: readonly UiError[];
  };
  activeStream: StreamRecord;
  evidenceRail: {
    evidenceRefs: readonly EvidenceRef[];
    actions: readonly string[];
  };
  replay: ReplayState;
  denied: readonly DeniedProjection[];
}

export interface RealtimeState {
  conversationId: string;
  connectionStatus: RealtimeConnectionStatus;
  commands: readonly CommandRecord[];
  messages: readonly RealtimeTrackedMessage[];
  activeStream: StreamRecord;
  replay: ReplayState;
  errors: readonly UiError[];
}

interface RealtimeTrackedMessage extends MessageRecord {
  values: readonly ProjectionSourceValue[];
}

export type RealtimeEventPayload = RealtimeMessagePayload | Record<string, unknown>;

export function createRealtimeCommand<TPayload>(
  input: RealtimeCommandInput<TPayload>,
): CommandEnvelope<TPayload> {
  return {
    clientId: input.clientId,
    intentId: input.intentId,
    kind: input.kind,
    issuedAt: input.issuedAt,
    actor: input.actor,
    payload: input.payload,
  };
}

export function initialRealtimeState(conversationId: string, cursor = "0"): RealtimeState {
  return {
    conversationId,
    connectionStatus: "idle",
    commands: [],
    messages: [],
    activeStream: {
      streamId: `${conversationId}:stream`,
      state: "idle",
      chunks: [],
    },
    replay: initialReplayState(cursor),
    errors: [],
  };
}

export function setRealtimeConnectionStatus(
  state: RealtimeState,
  status: RealtimeConnectionStatus,
): RealtimeState {
  return { ...state, connectionStatus: status };
}

export function queueRealtimeCommand<TPayload>(
  state: RealtimeState,
  envelope: CommandEnvelope<TPayload>,
  options: { optimisticMessage?: { body: string; values?: readonly ProjectionSourceValue[] } } = {},
): RealtimeState {
  const queuedCommand = transitionCommand(createCommandRecord(envelope), {
    state: "queued",
    at: envelope.issuedAt,
  });
  const messages = options.optimisticMessage
    ? [
        ...state.messages,
        {
          ...transitionMessage(
            createLocalMessage({
              localId: envelope.intentId,
              clientId: envelope.clientId,
              body: options.optimisticMessage.body,
              at: envelope.issuedAt,
            }),
            { state: "queued", at: envelope.issuedAt },
          ),
          values:
            options.optimisticMessage.values ??
            messageProjectionValues(options.optimisticMessage.body, [], "candidate"),
        },
      ]
    : state.messages;

  return {
    ...state,
    commands: [...state.commands, queuedCommand],
    messages,
  };
}

export function reconcileGatewayAck(
  state: RealtimeState,
  ack: RealtimeGatewayAck,
): RealtimeState {
  if (ack.type === "ack") {
    return {
      ...state,
      commands: state.commands.map((command) =>
        command.envelope.clientId === ack.clientId
          ? transitionCommand(command, { state: "acknowledged", at: ack.acknowledgedAt })
          : command,
      ),
      messages: state.messages.map((message) =>
        message.clientId === ack.clientId
          ? {
              ...transitionMessage(message, { state: "acked", at: ack.acknowledgedAt }),
              values: message.values,
            }
          : message,
      ),
    };
  }

  return {
    ...state,
    commands: state.commands.map((command) =>
      command.envelope.clientId === ack.clientId
        ? transitionCommand(command, { state: "rejected", at: ack.rejectedAt, error: ack.error })
        : command,
    ),
    messages: state.messages.map((message) =>
      message.clientId === ack.clientId
        ? {
            ...transitionMessage(message, { state: "rejected", at: ack.rejectedAt, error: ack.error }),
            values: message.values,
          }
        : message,
    ),
    errors: [...state.errors, ack.error],
  };
}

export function applyRealtimeEvent(
  state: RealtimeState,
  event: FrontendEventEnvelope<RealtimeEventPayload>,
): RealtimeState {
  if (state.replay.appliedEventIds.includes(event.eventId)) {
    return state;
  }

  if (!isSupportedRealtimeEvent(event.kind)) {
    return appendRealtimeError(state, createUiError("gateway_rejected", `Unsupported realtime event ${event.kind}.`));
  }

  const replay = applyReplayWithGapDetection(state.replay, event);
  if (replay.errors.length > state.replay.errors.length) {
    return { ...state, replay, errors: [...state.errors, replay.errors[replay.errors.length - 1]] };
  }

  if (event.kind === "message.created") {
    if (!hasDurableEvidence(event)) {
      return appendRealtimeError(
        { ...state, replay },
        createUiError("gateway_rejected", "Durable message event requires durable daemon evidence."),
      );
    }
    return applyDurableMessageEvent({ ...state, replay }, event);
  }

  return { ...state, replay };
}

export function projectRealtimeReadModel(
  state: RealtimeState,
  viewerRole: ProductRole | "system",
): RealtimeReadModel {
  const denied: DeniedProjection[] = [];
  const messages = state.messages.map((message) => {
    const projected = projectValuesForRole(viewerRole, message.values);
    denied.push(...projected.denied);
    const bodyValue = projected.visible.find((value) => value.category === "message_text")?.value;
    return {
      localId: message.localId,
      clientId: message.clientId,
      canonicalId: message.canonicalId,
      body: typeof bodyValue === "string" ? bodyValue : "",
      state: message.state,
      sequence: message.sequence,
      cursor: message.cursor,
      evidenceRefs: message.evidenceRefs,
      denied: projected.denied,
    };
  });
  const recoverableCommand = [...state.commands].reverse().find((command) => command.recoverableIntent !== null);
  const blockingError = state.errors.find((error) => error.policy.blocksComposer);

  return {
    conversation: {
      conversationId: state.conversationId,
      status: state.connectionStatus,
      lastSequence: state.replay.lastSequence,
      cursor: state.replay.cursor,
    },
    messages,
    composer: {
      status: blockingError
        ? "blocked"
        : recoverableCommand?.state === "rejected"
          ? "recoverable_error"
          : state.commands.some((command) => command.state === "queued" || command.state === "acknowledged")
            ? "sending"
            : "idle",
      recoverableIntent: recoverableCommand?.recoverableIntent ?? null,
      errors: state.errors,
    },
    activeStream: state.activeStream,
    evidenceRail: {
      evidenceRefs: uniqueEvidenceRefs(messages.flatMap((message) => message.evidenceRefs)),
      actions: recoverableCommand?.state === "rejected" ? ["retry"] : [],
    },
    replay: state.replay,
    denied,
  };
}

export class InMemoryRealtimeGateway implements RealtimeGatewayPort {
  private connectionStatus: RealtimeConnectionStatus = "idle";
  private readonly acks = new Map<string, RealtimeGatewayAck>();
  private readonly replayEvents: FrontendEventEnvelope<RealtimeEventPayload>[];

  constructor(options: { replayEvents?: readonly FrontendEventEnvelope<RealtimeEventPayload>[] } = {}) {
    this.replayEvents = [...(options.replayEvents ?? [])];
  }

  status(): RealtimeConnectionStatus {
    return this.connectionStatus;
  }

  async connect(): Promise<{ status: RealtimeConnectionStatus }> {
    this.connectionStatus = "connected";
    return { status: this.connectionStatus };
  }

  async disconnect(): Promise<{ status: RealtimeConnectionStatus }> {
    this.connectionStatus = "disconnected";
    return { status: this.connectionStatus };
  }

  async sendCommand<TPayload>(command: CommandEnvelope<TPayload>): Promise<RealtimeGatewayAck> {
    return (
      this.acks.get(command.clientId) ?? {
        type: "ack",
        clientId: command.clientId,
        acknowledgedAt: command.issuedAt,
      }
    );
  }

  async replayFromCursor(cursor: string): Promise<readonly FrontendEventEnvelope<RealtimeEventPayload>[]> {
    this.connectionStatus = "replaying";
    const cursorNumber = Number(cursor);
    return this.replayEvents.filter((event) => Number(event.cursor) > cursorNumber);
  }

  scriptAck(ack: RealtimeGatewayAck): void {
    this.acks.set(ack.clientId, ack);
  }
}

function applyDurableMessageEvent(
  state: RealtimeState,
  event: FrontendEventEnvelope<RealtimeEventPayload>,
): RealtimeState {
  const payload = event.payload as RealtimeMessagePayload;
  const body = payload.body ?? payload.bodyMarkdown ?? "";
  const values = payload.projectionValues ?? messageProjectionValues(body, event.evidenceRefs ?? [], "durable");
  const existing = state.messages.find((message) => message.clientId === event.clientId);
  const durableMessage = existing
    ? {
        ...transitionMessage(existing, {
          state: "durable",
          at: event.occurredAt,
          event: { ...event, payload: { body } },
        }),
        values,
      }
    : {
        ...transitionMessage(
          createLocalMessage({
            localId: event.canonicalId ?? event.eventId,
            clientId: event.clientId ?? event.eventId,
            body,
            at: event.occurredAt,
          }),
          {
            state: "replayed",
            at: event.occurredAt,
            event: { ...event, payload: { body } },
          },
        ),
        values,
      };

  return {
    ...state,
    commands: state.commands.map((command) =>
      command.envelope.clientId === event.clientId
        ? transitionCommand(command, { state: "durable", at: event.occurredAt, event })
        : command,
    ),
    messages: existing
      ? state.messages.map((message) => (message.clientId === event.clientId ? durableMessage : message))
      : [...state.messages, durableMessage],
  };
}

function applyReplayWithGapDetection<TPayload>(
  replay: ReplayState,
  event: FrontendEventEnvelope<TPayload>,
): ReplayState {
  if (event.sequence > replay.lastSequence + 1) {
    return {
      ...replay,
      errors: [
        ...replay.errors,
        createUiError("replay_gap", `Replay gap before sequence ${event.sequence}.`),
      ],
    };
  }
  return {
    ...replay,
    lastSequence: Math.max(replay.lastSequence, event.sequence),
    cursor: event.cursor,
    appliedEventIds: [...replay.appliedEventIds, event.eventId],
  };
}

function appendRealtimeError(state: RealtimeState, error: UiError): RealtimeState {
  return {
    ...state,
    errors: [...state.errors, error],
  };
}

function isSupportedRealtimeEvent(kind: string): boolean {
  return kind === "message.created" || kind === "message.updated" || kind === "conversation.read_model.updated";
}

function messageProjectionValues(
  body: string,
  evidenceRefs: readonly EvidenceRef[],
  durability: "candidate" | "durable",
): readonly ProjectionSourceValue[] {
  return [
    {
      category: "message_text",
      value: body,
      evidenceRefs,
      durability,
    },
  ];
}

function uniqueEvidenceRefs(evidenceRefs: readonly EvidenceRef[]): readonly EvidenceRef[] {
  const seen = new Set<string>();
  const unique: EvidenceRef[] = [];
  for (const evidenceRef of evidenceRefs) {
    if (!seen.has(evidenceRef.id)) {
      seen.add(evidenceRef.id);
      unique.push(evidenceRef);
    }
  }
  return unique;
}
