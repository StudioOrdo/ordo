import { type ProductRole } from "@/lib/product-navigation";

export type RoleVisibility =
  | "public"
  | "client"
  | "affiliate"
  | "staff"
  | "manager"
  | "admin"
  | "owner"
  | "system";

export type ActorDisplayKind =
  | "visitor"
  | "client"
  | "affiliate"
  | "staff"
  | "manager"
  | "admin"
  | "owner"
  | "ordo_agent"
  | "system";

export interface ActorRef {
  actorId: string;
  role: ProductRole | "ordo_agent" | "system";
  displayKind: ActorDisplayKind;
}

export type EvidenceKind =
  | "daemon_event"
  | "artifact"
  | "policy_decision"
  | "privacy_decision"
  | "provider_result"
  | "browser_candidate"
  | "user_confirmation";

export interface EvidenceRef {
  id: string;
  kind: EvidenceKind;
  durability: "candidate" | "durable";
  visibility: RoleVisibility;
  summary?: string;
}

export interface FrontendEventEnvelope<TPayload> {
  eventId: string;
  canonicalId?: string;
  clientId?: string;
  sequence: number;
  cursor: string;
  occurredAt: string;
  actor: ActorRef;
  visibility: RoleVisibility;
  kind: string;
  payload: TPayload;
  evidenceRefs?: readonly EvidenceRef[];
}

export interface CommandEnvelope<TPayload> {
  clientId: string;
  intentId: string;
  kind: string;
  issuedAt: string;
  actor: ActorRef;
  payload: TPayload;
}

export type CommandLifecycleState =
  | "local"
  | "queued"
  | "acknowledged"
  | "durable"
  | "rejected";

export interface CommandRecord<TPayload = unknown> {
  envelope: CommandEnvelope<TPayload>;
  state: CommandLifecycleState;
  recoverableIntent: TPayload | null;
  durableEventId?: string;
  canonicalId?: string;
  rejectedError?: UiError;
  acknowledgedAt?: string;
  updatedAt: string;
}

export type MessageLifecycleState =
  | "local"
  | "queued"
  | "acked"
  | "streaming"
  | "durable"
  | "replayed"
  | "rejected";

export interface MessageRecord {
  localId: string;
  clientId: string;
  canonicalId?: string;
  body: string;
  state: MessageLifecycleState;
  sequence?: number;
  cursor?: string;
  evidenceRefs: readonly EvidenceRef[];
  rejectedError?: UiError;
  updatedAt: string;
}

export type StreamLifecycleState =
  | "idle"
  | "started"
  | "delta"
  | "interrupted"
  | "completed"
  | "failed"
  | "recovered";

export interface StreamRecord {
  streamId: string;
  state: StreamLifecycleState;
  chunks: readonly string[];
  startedAt?: string;
  completedAt?: string;
  failedError?: UiError;
}

export interface ReplayState {
  lastSequence: number;
  cursor: string;
  appliedEventIds: readonly string[];
  errors: readonly UiError[];
}

export type UiErrorKind =
  | "user_input_invalid"
  | "permission_denied"
  | "policy_rejected"
  | "privacy_required"
  | "network_transient"
  | "gateway_rejected"
  | "provider_unavailable"
  | "capability_unavailable"
  | "capability_failed"
  | "artifact_validation_failed"
  | "replay_gap"
  | "unknown";

export interface UiErrorPolicy {
  retryable: boolean;
  preservesIntent: boolean;
  requiresUserCorrection: boolean;
  visibleTo: readonly RoleVisibility[];
  telemetryRequired: boolean;
  blocksComposer: boolean;
}

export interface UiError {
  kind: UiErrorKind;
  message: string;
  policy: UiErrorPolicy;
  evidenceRefs?: readonly EvidenceRef[];
}

export const uiErrorPolicies: Record<UiErrorKind, UiErrorPolicy> = {
  user_input_invalid: {
    retryable: true,
    preservesIntent: true,
    requiresUserCorrection: true,
    visibleTo: ["public", "client", "affiliate", "staff", "manager", "admin", "owner"],
    telemetryRequired: false,
    blocksComposer: false,
  },
  permission_denied: {
    retryable: false,
    preservesIntent: true,
    requiresUserCorrection: false,
    visibleTo: ["client", "affiliate", "staff", "manager", "admin", "owner"],
    telemetryRequired: true,
    blocksComposer: true,
  },
  policy_rejected: {
    retryable: false,
    preservesIntent: true,
    requiresUserCorrection: true,
    visibleTo: ["client", "affiliate", "staff", "manager", "admin", "owner"],
    telemetryRequired: true,
    blocksComposer: false,
  },
  privacy_required: {
    retryable: true,
    preservesIntent: true,
    requiresUserCorrection: true,
    visibleTo: ["client", "affiliate", "staff", "manager", "admin", "owner"],
    telemetryRequired: true,
    blocksComposer: false,
  },
  network_transient: {
    retryable: true,
    preservesIntent: true,
    requiresUserCorrection: false,
    visibleTo: ["public", "client", "affiliate", "staff", "manager", "admin", "owner"],
    telemetryRequired: true,
    blocksComposer: false,
  },
  gateway_rejected: {
    retryable: false,
    preservesIntent: true,
    requiresUserCorrection: false,
    visibleTo: ["client", "affiliate", "staff", "manager", "admin", "owner"],
    telemetryRequired: true,
    blocksComposer: false,
  },
  provider_unavailable: {
    retryable: true,
    preservesIntent: true,
    requiresUserCorrection: false,
    visibleTo: ["staff", "manager", "admin", "owner"],
    telemetryRequired: true,
    blocksComposer: false,
  },
  capability_unavailable: {
    retryable: false,
    preservesIntent: true,
    requiresUserCorrection: false,
    visibleTo: ["public", "client", "affiliate", "staff", "manager", "admin", "owner"],
    telemetryRequired: true,
    blocksComposer: false,
  },
  capability_failed: {
    retryable: true,
    preservesIntent: true,
    requiresUserCorrection: false,
    visibleTo: ["client", "affiliate", "staff", "manager", "admin", "owner"],
    telemetryRequired: true,
    blocksComposer: false,
  },
  artifact_validation_failed: {
    retryable: false,
    preservesIntent: true,
    requiresUserCorrection: true,
    visibleTo: ["staff", "manager", "admin", "owner"],
    telemetryRequired: true,
    blocksComposer: false,
  },
  replay_gap: {
    retryable: true,
    preservesIntent: true,
    requiresUserCorrection: false,
    visibleTo: ["client", "affiliate", "staff", "manager", "admin", "owner"],
    telemetryRequired: true,
    blocksComposer: false,
  },
  unknown: {
    retryable: false,
    preservesIntent: true,
    requiresUserCorrection: false,
    visibleTo: ["staff", "manager", "admin", "owner"],
    telemetryRequired: true,
    blocksComposer: false,
  },
};

export function createUiError(kind: UiErrorKind, message: string, evidenceRefs: readonly EvidenceRef[] = []): UiError {
  return {
    kind,
    message,
    policy: uiErrorPolicies[kind],
    evidenceRefs,
  };
}

export function createCommandRecord<TPayload>(
  envelope: CommandEnvelope<TPayload>,
  issuedAt: string = envelope.issuedAt,
): CommandRecord<TPayload> {
  return {
    envelope,
    state: "local",
    recoverableIntent: envelope.payload,
    updatedAt: issuedAt,
  };
}

export function transitionCommand<TPayload>(
  command: CommandRecord<TPayload>,
  next:
    | { state: "queued"; at: string }
    | { state: "acknowledged"; at: string }
    | { state: "durable"; at: string; event: FrontendEventEnvelope<unknown> }
    | { state: "rejected"; at: string; error: UiError },
): CommandRecord<TPayload> {
  if (command.state === "durable" || command.state === "rejected") {
    return command;
  }

  if (next.state === "queued") {
    return { ...command, state: "queued", updatedAt: next.at };
  }

  if (next.state === "acknowledged") {
    return { ...command, state: "acknowledged", acknowledgedAt: next.at, updatedAt: next.at };
  }

  if (next.state === "rejected") {
    return {
      ...command,
      state: "rejected",
      rejectedError: next.error,
      recoverableIntent: next.error.policy.preservesIntent ? command.recoverableIntent : null,
      updatedAt: next.at,
    };
  }

  return {
    ...command,
    state: "durable",
    durableEventId: next.event.eventId,
    canonicalId: next.event.canonicalId,
    recoverableIntent: null,
    updatedAt: next.at,
  };
}

export function createLocalMessage(input: {
  localId: string;
  clientId: string;
  body: string;
  at: string;
}): MessageRecord {
  return {
    localId: input.localId,
    clientId: input.clientId,
    body: input.body,
    state: "local",
    evidenceRefs: [],
    updatedAt: input.at,
  };
}

export function transitionMessage(
  message: MessageRecord,
  next:
    | { state: "queued"; at: string }
    | { state: "acked"; at: string }
    | { state: "streaming"; at: string }
    | { state: "durable" | "replayed"; at: string; event: FrontendEventEnvelope<{ body?: string }> }
    | { state: "rejected"; at: string; error: UiError },
): MessageRecord {
  if (message.state === "durable" || message.state === "replayed" || message.state === "rejected") {
    return message;
  }

  if (next.state === "queued" || next.state === "acked" || next.state === "streaming") {
    return { ...message, state: next.state, updatedAt: next.at };
  }

  if (next.state === "rejected") {
    return {
      ...message,
      state: "rejected",
      rejectedError: next.error,
      updatedAt: next.at,
    };
  }

  if (!hasDurableEvidence(next.event)) {
    return {
      ...message,
      state: "rejected",
      rejectedError: createUiError("gateway_rejected", "Durable message requires durable daemon evidence."),
      updatedAt: next.at,
    };
  }

  return {
    ...message,
    state: next.state,
    canonicalId: next.event.canonicalId,
    sequence: next.event.sequence,
    cursor: next.event.cursor,
    body: next.event.payload.body ?? message.body,
    evidenceRefs: next.event.evidenceRefs ?? [],
    updatedAt: next.at,
  };
}

export function transitionStream(
  stream: StreamRecord,
  next:
    | { state: "started"; at: string }
    | { state: "delta"; chunk: string }
    | { state: "interrupted" | "completed"; at: string }
    | { state: "failed"; error: UiError },
): StreamRecord {
  if (stream.state === "completed" || stream.state === "failed") {
    return stream;
  }

  if (next.state === "started") {
    return { ...stream, state: "started", startedAt: next.at };
  }

  if (next.state === "delta") {
    return { ...stream, state: "delta", chunks: [...stream.chunks, next.chunk] };
  }

  if (next.state === "failed") {
    return { ...stream, state: "failed", failedError: next.error };
  }

  return { ...stream, state: next.state, completedAt: next.at };
}

export function initialReplayState(cursor = "0"): ReplayState {
  return {
    lastSequence: 0,
    cursor,
    appliedEventIds: [],
    errors: [],
  };
}

export function applyReplayEvent<TPayload>(
  state: ReplayState,
  event: FrontendEventEnvelope<TPayload>,
): ReplayState {
  if (state.appliedEventIds.includes(event.eventId)) {
    return state;
  }

  if (event.sequence > state.lastSequence + 1) {
    return {
      ...state,
      errors: [
        ...state.errors,
        createUiError("replay_gap", `Replay gap before sequence ${event.sequence}.`),
      ],
    };
  }

  if (event.sequence <= state.lastSequence) {
    return {
      ...state,
      appliedEventIds: [...state.appliedEventIds, event.eventId],
      cursor: event.cursor,
    };
  }

  return {
    ...state,
    lastSequence: event.sequence,
    cursor: event.cursor,
    appliedEventIds: [...state.appliedEventIds, event.eventId],
  };
}

export function hasDurableEvidence(event: FrontendEventEnvelope<unknown>): boolean {
  return event.evidenceRefs?.some((ref) => ref.kind === "daemon_event" && ref.durability === "durable") ?? false;
}
