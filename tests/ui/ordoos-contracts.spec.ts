import { expect, test } from "@playwright/test";

import {
  applyReplayEvent,
  createCommandRecord,
  createLocalMessage,
  createUiError,
  initialReplayState,
  transitionCommand,
  transitionMessage,
  transitionStream,
  uiErrorPolicies,
  type ActorRef,
  type EvidenceRef,
  type FrontendEventEnvelope,
  type StreamRecord,
} from "@/lib/ordoos-frontend-contracts";

const actor: ActorRef = {
  actorId: "actor_client_1",
  role: "client",
  displayKind: "client",
};

const durableDaemonEvidence: EvidenceRef = {
  id: "event_message_1",
  kind: "daemon_event",
  durability: "durable",
  visibility: "client",
  summary: "message.created",
};

function event(sequence: number, overrides: Partial<FrontendEventEnvelope<{ body: string }>> = {}): FrontendEventEnvelope<{ body: string }> {
  return {
    eventId: `event_${sequence}`,
    canonicalId: `message_${sequence}`,
    clientId: `client_${sequence}`,
    sequence,
    cursor: String(sequence),
    occurredAt: `2026-05-10T00:00:0${sequence}Z`,
    actor,
    visibility: "client",
    kind: "message.created",
    payload: { body: `durable body ${sequence}` },
    evidenceRefs: [durableDaemonEvidence],
    ...overrides,
  };
}

test.describe("OrdoOS frontend contracts", () => {
  test("command lifecycle preserves recoverable intent on rejection", () => {
    const command = createCommandRecord({
      clientId: "client_command_1",
      intentId: "intent_1",
      kind: "message.submit",
      issuedAt: "2026-05-10T00:00:00Z",
      actor,
      payload: { body: "Please help me." },
    });

    const queued = transitionCommand(command, { state: "queued", at: "2026-05-10T00:00:01Z" });
    const acknowledged = transitionCommand(queued, { state: "acknowledged", at: "2026-05-10T00:00:02Z" });
    const rejected = transitionCommand(acknowledged, {
      state: "rejected",
      at: "2026-05-10T00:00:03Z",
      error: createUiError("gateway_rejected", "Gateway rejected the command."),
    });

    expect(rejected.state).toBe("rejected");
    expect(rejected.recoverableIntent).toEqual({ body: "Please help me." });
    expect(rejected.rejectedError?.policy.preservesIntent).toBe(true);
  });

  test("durable command transitions become terminal", () => {
    const command = createCommandRecord({
      clientId: "client_command_2",
      intentId: "intent_2",
      kind: "message.submit",
      issuedAt: "2026-05-10T00:00:00Z",
      actor,
      payload: { body: "Persist this." },
    });

    const durable = transitionCommand(command, {
      state: "durable",
      at: "2026-05-10T00:00:01Z",
      event: event(1),
    });
    const rejectedAfterDurable = transitionCommand(durable, {
      state: "rejected",
      at: "2026-05-10T00:00:02Z",
      error: createUiError("unknown", "Should not replace terminal state."),
    });

    expect(durable.state).toBe("durable");
    expect(durable.recoverableIntent).toBeNull();
    expect(rejectedAfterDurable).toBe(durable);
  });

  test("message lifecycle requires durable daemon evidence before durable render", () => {
    const local = createLocalMessage({
      localId: "local_1",
      clientId: "client_1",
      body: "Local body",
      at: "2026-05-10T00:00:00Z",
    });

    const queued = transitionMessage(local, { state: "queued", at: "2026-05-10T00:00:01Z" });
    const acked = transitionMessage(queued, { state: "acked", at: "2026-05-10T00:00:02Z" });
    const durable = transitionMessage(acked, {
      state: "durable",
      at: "2026-05-10T00:00:03Z",
      event: event(1),
    });

    expect(durable.state).toBe("durable");
    expect(durable.canonicalId).toBe("message_1");
    expect(durable.sequence).toBe(1);
    expect(durable.cursor).toBe("1");
    expect(durable.body).toBe("durable body 1");
  });

  test("message without durable evidence is rejected instead of durable", () => {
    const local = createLocalMessage({
      localId: "local_2",
      clientId: "client_2",
      body: "Local body",
      at: "2026-05-10T00:00:00Z",
    });

    const rejected = transitionMessage(local, {
      state: "durable",
      at: "2026-05-10T00:00:01Z",
      event: event(2, {
        evidenceRefs: [{ id: "candidate_1", kind: "browser_candidate", durability: "candidate", visibility: "client" }],
      }),
    });

    expect(rejected.state).toBe("rejected");
    expect(rejected.rejectedError?.kind).toBe("gateway_rejected");
    expect(rejected.canonicalId).toBeUndefined();
  });

  test("rejected message cannot become durable afterward", () => {
    const local = createLocalMessage({
      localId: "local_3",
      clientId: "client_3",
      body: "Local body",
      at: "2026-05-10T00:00:00Z",
    });
    const rejected = transitionMessage(local, {
      state: "rejected",
      at: "2026-05-10T00:00:01Z",
      error: createUiError("policy_rejected", "Policy rejected the message."),
    });

    const durableAttempt = transitionMessage(rejected, {
      state: "durable",
      at: "2026-05-10T00:00:02Z",
      event: event(3),
    });

    expect(durableAttempt).toBe(rejected);
    expect(durableAttempt.state).toBe("rejected");
  });

  test("stream lifecycle records deltas and terminal failure", () => {
    const stream: StreamRecord = {
      streamId: "stream_1",
      state: "idle",
      chunks: [],
    };

    const started = transitionStream(stream, { state: "started", at: "2026-05-10T00:00:00Z" });
    const firstDelta = transitionStream(started, { state: "delta", chunk: "Hello" });
    const secondDelta = transitionStream(firstDelta, { state: "delta", chunk: " world" });
    const failed = transitionStream(secondDelta, {
      state: "failed",
      error: createUiError("provider_unavailable", "Provider unavailable."),
    });

    expect(failed.state).toBe("failed");
    expect(failed.chunks).toEqual(["Hello", " world"]);
    expect(failed.failedError?.policy.retryable).toBe(true);
  });

  test("replay is idempotent and detects sequence gaps", () => {
    const initial = initialReplayState();
    const afterFirst = applyReplayEvent(initial, event(1));
    const duplicate = applyReplayEvent(afterFirst, event(1));
    const gap = applyReplayEvent(duplicate, event(3));
    const afterSecond = applyReplayEvent(duplicate, event(2));

    expect(afterFirst.lastSequence).toBe(1);
    expect(duplicate).toBe(afterFirst);
    expect(gap.lastSequence).toBe(1);
    expect(gap.errors.at(-1)?.kind).toBe("replay_gap");
    expect(afterSecond.lastSequence).toBe(2);
    expect(afterSecond.cursor).toBe("2");
  });

  test("UI error policy metadata is complete and deterministic", () => {
    const expectedKinds = [
      "user_input_invalid",
      "permission_denied",
      "policy_rejected",
      "privacy_required",
      "network_transient",
      "gateway_rejected",
      "provider_unavailable",
      "capability_unavailable",
      "capability_failed",
      "artifact_validation_failed",
      "replay_gap",
      "unknown",
    ];

    expect(Object.keys(uiErrorPolicies)).toEqual(expectedKinds);
    for (const policy of Object.values(uiErrorPolicies)) {
      expect(typeof policy.retryable).toBe("boolean");
      expect(typeof policy.preservesIntent).toBe("boolean");
      expect(typeof policy.requiresUserCorrection).toBe("boolean");
      expect(typeof policy.telemetryRequired).toBe("boolean");
      expect(typeof policy.blocksComposer).toBe("boolean");
      expect(policy.visibleTo.length).toBeGreaterThan(0);
    }
  });
});
