import { expect, test } from "@playwright/test";

import {
  createUiError,
  type ActorRef,
  type EvidenceRef,
  type FrontendEventEnvelope,
} from "@/lib/ordoos-frontend-contracts";
import {
  applyRealtimeEvent,
  createRealtimeCommand,
  InMemoryRealtimeGateway,
  initialRealtimeState,
  projectRealtimeReadModel,
  queueRealtimeCommand,
  reconcileGatewayAck,
  setRealtimeConnectionStatus,
  type RealtimeEventPayload,
} from "@/lib/ordoos-realtime";

const actor: ActorRef = {
  actorId: "actor_client",
  role: "client",
  displayKind: "client",
};

const durableEvidence: EvidenceRef = {
  id: "conversation_event_1",
  kind: "daemon_event",
  durability: "durable",
  visibility: "client",
  summary: "message persisted",
};

function messageEvent(input: {
  eventId?: string;
  clientId?: string;
  sequence?: number;
  cursor?: string;
  body?: string;
} = {}): FrontendEventEnvelope<RealtimeEventPayload> {
  return {
    eventId: input.eventId ?? "event_message_1",
    canonicalId: "message_1",
    clientId: input.clientId ?? "client_message_1",
    sequence: input.sequence ?? 1,
    cursor: input.cursor ?? "1",
    occurredAt: "2026-05-10T00:00:01Z",
    actor,
    visibility: "client",
    kind: "message.created",
    payload: {
      body: input.body ?? "Durable hello",
    },
    evidenceRefs: [durableEvidence],
  };
}

test.describe("OrdoOS realtime gateway and read-model foundation", () => {
  test("command queue lifecycle stays optimistic until durable event evidence arrives", () => {
    const command = createRealtimeCommand({
      kind: "message.submit",
      payload: { bodyMarkdown: "Hello", clientMessageId: "client_message_1" },
      actor,
      clientId: "client_message_1",
      intentId: "intent_message_1",
      issuedAt: "2026-05-10T00:00:00Z",
    });
    let state = queueRealtimeCommand(initialRealtimeState("conversation_1"), command, {
      optimisticMessage: { body: "Hello" },
    });

    expect(state.commands[0]?.state).toBe("queued");
    expect(state.messages[0]?.state).toBe("queued");

    state = reconcileGatewayAck(state, {
      type: "ack",
      clientId: "client_message_1",
      acknowledgedAt: "2026-05-10T00:00:00Z",
    });

    expect(state.commands[0]?.state).toBe("acknowledged");
    expect(state.messages[0]?.state).toBe("acked");
    expect(projectRealtimeReadModel(state, "client").messages[0]).toMatchObject({
      body: "Hello",
      state: "acked",
    });

    state = applyRealtimeEvent(state, messageEvent());

    expect(state.commands[0]?.state).toBe("durable");
    expect(state.messages[0]).toMatchObject({
      canonicalId: "message_1",
      state: "durable",
      body: "Durable hello",
      sequence: 1,
      cursor: "1",
    });
    expect(projectRealtimeReadModel(state, "client").evidenceRail.evidenceRefs).toEqual([durableEvidence]);
  });

  test("rejected command preserves recoverable user intent", () => {
    const command = createRealtimeCommand({
      kind: "message.submit",
      payload: { bodyMarkdown: "Retry me", clientMessageId: "client_message_2" },
      actor,
      clientId: "client_message_2",
      intentId: "intent_message_2",
      issuedAt: "2026-05-10T00:00:00Z",
    });
    let state = queueRealtimeCommand(initialRealtimeState("conversation_1"), command, {
      optimisticMessage: { body: "Retry me" },
    });

    state = reconcileGatewayAck(state, {
      type: "reject",
      clientId: "client_message_2",
      rejectedAt: "2026-05-10T00:00:01Z",
      error: createUiError("policy_rejected", "Message rejected by policy."),
    });
    const readModel = projectRealtimeReadModel(state, "client");

    expect(state.commands[0]?.state).toBe("rejected");
    expect(state.commands[0]?.recoverableIntent).toEqual({
      bodyMarkdown: "Retry me",
      clientMessageId: "client_message_2",
    });
    expect(readModel.composer.status).toBe("recoverable_error");
    expect(readModel.composer.recoverableIntent).toEqual({
      bodyMarkdown: "Retry me",
      clientMessageId: "client_message_2",
    });
  });

  test("ack without durable event does not mark message durable", () => {
    const command = createRealtimeCommand({
      kind: "message.submit",
      payload: { bodyMarkdown: "Still not durable", clientMessageId: "client_message_3" },
      actor,
      clientId: "client_message_3",
      intentId: "intent_message_3",
      issuedAt: "2026-05-10T00:00:00Z",
    });
    const state = reconcileGatewayAck(
      queueRealtimeCommand(initialRealtimeState("conversation_1"), command, {
        optimisticMessage: { body: "Still not durable" },
      }),
      {
        type: "ack",
        clientId: "client_message_3",
        acknowledgedAt: "2026-05-10T00:00:01Z",
      },
    );

    expect(state.commands[0]?.state).toBe("acknowledged");
    expect(state.messages[0]?.state).toBe("acked");
    expect(projectRealtimeReadModel(state, "client").messages[0]?.state).toBe("acked");
  });

  test("duplicate durable events are idempotent", () => {
    const first = applyRealtimeEvent(initialRealtimeState("conversation_1"), messageEvent());
    const second = applyRealtimeEvent(first, messageEvent());

    expect(second).toEqual(first);
    expect(projectRealtimeReadModel(second, "client").messages).toHaveLength(1);
  });

  test("replay gap creates explicit replay_gap error", () => {
    const state = applyRealtimeEvent(
      initialRealtimeState("conversation_1"),
      messageEvent({ eventId: "event_message_4", sequence: 3, cursor: "3" }),
    );

    expect(state.replay.errors).toEqual([
      expect.objectContaining({
        kind: "replay_gap",
        message: "Replay gap before sequence 3.",
      }),
    ]);
    expect(projectRealtimeReadModel(state, "client").messages).toEqual([]);
  });

  test("unknown dispatch frame fails closed", () => {
    const state = applyRealtimeEvent(initialRealtimeState("conversation_1"), {
      ...messageEvent(),
      eventId: "event_unknown",
      kind: "provider.payload.raw",
    });

    expect(state.errors).toEqual([
      expect.objectContaining({
        kind: "gateway_rejected",
        message: "Unsupported realtime event provider.payload.raw.",
      }),
    ]);
    expect(projectRealtimeReadModel(state, "client").messages).toEqual([]);
  });

  test("client read model excludes staff provider policy and private internals", () => {
    const state = applyRealtimeEvent(
      initialRealtimeState("conversation_1"),
      {
        ...messageEvent(),
        payload: {
          body: "Client-visible text",
          projectionValues: [
            { category: "message_text", value: "Client-visible text", evidenceRefs: [durableEvidence] },
            { category: "raw_prompt", value: "raw prompt should not leak" },
            { category: "provider_payload", value: { token: "provider-private" } },
            { category: "policy_internals", value: "staff policy internals" },
            { category: "privacy_placeholder_map", value: { secret: "private-map-value" } },
            { category: "staff_notes", value: "staff-only note" },
          ],
        },
      },
    );
    const readModel = projectRealtimeReadModel(state, "client");

    expect(readModel.messages[0]?.body).toBe("Client-visible text");
    expect(JSON.stringify(readModel)).not.toContain("raw prompt should not leak");
    expect(JSON.stringify(readModel)).not.toContain("provider-private");
    expect(JSON.stringify(readModel)).not.toContain("staff policy internals");
    expect(JSON.stringify(readModel)).not.toContain("private-map-value");
    expect(JSON.stringify(readModel)).not.toContain("staff-only note");
    expect(readModel.denied.map((denied) => denied.category)).toEqual([
      "raw_prompt",
      "provider_payload",
      "policy_internals",
      "privacy_placeholder_map",
      "staff_notes",
    ]);
  });

  test("in-memory gateway runs deterministic command and replay flow without network", async () => {
    const gateway = new InMemoryRealtimeGateway({ replayEvents: [messageEvent()] });
    const connected = await gateway.connect();
    const command = createRealtimeCommand({
      kind: "message.submit",
      payload: { bodyMarkdown: "Hello", clientMessageId: "client_message_1" },
      actor,
      clientId: "client_message_1",
      intentId: "intent_message_1",
      issuedAt: "2026-05-10T00:00:00Z",
    });
    let state = setRealtimeConnectionStatus(initialRealtimeState("conversation_1"), connected.status);
    state = queueRealtimeCommand(state, command, { optimisticMessage: { body: "Hello" } });
    state = reconcileGatewayAck(state, await gateway.sendCommand(command));
    for (const event of await gateway.replayFromCursor(state.replay.cursor)) {
      state = applyRealtimeEvent(state, event);
    }

    expect(gateway.status()).toBe("replaying");
    expect(projectRealtimeReadModel(state, "client").messages[0]).toMatchObject({
      body: "Durable hello",
      state: "durable",
    });
  });
});
