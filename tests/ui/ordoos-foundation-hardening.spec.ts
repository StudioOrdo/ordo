import { expect, test } from "@playwright/test";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

import {
  createUiError,
  type ActorRef,
  type EvidenceRef,
  type FrontendEventEnvelope,
} from "@/lib/ordoos-frontend-contracts";
import {
  accessibilityProfile,
  defaultCatalog,
  defaultExperienceSettings,
  experienceAttributes,
  lookupMessage,
  resolveEffectiveExperienceSettings,
  themeManifestById,
  themeTokensToCssVariables,
} from "@/lib/ordoos-experience";
import { composeOrdoShell, isSlotEnabled } from "@/lib/ordoos-shell";
import {
  BrowserCapabilityRuntime,
  createFileHashCapability,
  type BrowserCapabilityRequest,
} from "@/lib/ordoos-browser-capabilities";
import {
  applyRealtimeEvent,
  createRealtimeCommand,
  initialRealtimeState,
  projectRealtimeReadModel,
  queueRealtimeCommand,
  reconcileGatewayAck,
  type RealtimeEventPayload,
} from "@/lib/ordoos-realtime";

const actor: ActorRef = {
  actorId: "actor_client",
  role: "client",
  displayKind: "client",
};

const durableEvidence: EvidenceRef = {
  id: "conversation_event_hardening_1",
  kind: "daemon_event",
  durability: "durable",
  visibility: "client",
  summary: "message persisted",
};

function durableMessageEvent(
  overrides: Partial<FrontendEventEnvelope<RealtimeEventPayload>> = {},
): FrontendEventEnvelope<RealtimeEventPayload> {
  return {
    eventId: "event_hardening_message_1",
    canonicalId: "message_hardening_1",
    clientId: "client_hardening_1",
    sequence: 1,
    cursor: "1",
    occurredAt: "2026-05-10T00:00:01Z",
    actor,
    visibility: "client",
    kind: "message.created",
    payload: { body: "Durable contract message" },
    evidenceRefs: [durableEvidence],
    ...overrides,
  };
}

function capabilityRequest(overrides: Partial<BrowserCapabilityRequest> = {}): BrowserCapabilityRequest {
  return {
    requestId: "request_hardening_hash_1",
    capabilityId: "file.hash",
    issuedAt: "2026-05-10T00:00:00Z",
    actor,
    input: {
      metadata: {
        inputId: "input_hardening_1",
        label: "hardening-note.md",
        mediaType: "text/markdown",
        byteLength: 11,
        estimatedWorkMs: 1,
      },
      text: "hello world",
    },
    budget: {
      maxInputBytes: 1024,
      memoryLimitBytes: 2048,
      timeoutMs: 50,
    },
    ...overrides,
  };
}

test.describe("OrdoOS frontend foundation hardening gates", () => {
  test("durability, rejection, replay, and gap gates are explicit", () => {
    const command = createRealtimeCommand({
      kind: "message.submit",
      payload: { bodyMarkdown: "Recover this", clientMessageId: "client_hardening_1" },
      actor,
      clientId: "client_hardening_1",
      intentId: "intent_hardening_1",
      issuedAt: "2026-05-10T00:00:00Z",
    });
    const queued = queueRealtimeCommand(initialRealtimeState("conversation_hardening"), command, {
      optimisticMessage: { body: "Recover this" },
    });
    const rejected = reconcileGatewayAck(queued, {
      type: "reject",
      clientId: "client_hardening_1",
      rejectedAt: "2026-05-10T00:00:02Z",
      error: createUiError("policy_rejected", "Rejected in hardening fixture."),
    });

    expect(rejected.commands[0]?.recoverableIntent).toEqual({
      bodyMarkdown: "Recover this",
      clientMessageId: "client_hardening_1",
    });

    const missingEvidence = applyRealtimeEvent(initialRealtimeState("conversation_hardening"), {
      ...durableMessageEvent(),
      evidenceRefs: [],
    });

    expect(projectRealtimeReadModel(missingEvidence, "client").messages).toEqual([]);
    expect(missingEvidence.errors).toEqual([
      expect.objectContaining({
        kind: "gateway_rejected",
        message: "Durable message event requires durable daemon evidence.",
      }),
    ]);

    const firstReplay = applyRealtimeEvent(initialRealtimeState("conversation_hardening"), durableMessageEvent());
    const duplicateReplay = applyRealtimeEvent(firstReplay, durableMessageEvent());
    expect(duplicateReplay).toEqual(firstReplay);

    const gap = applyRealtimeEvent(
      initialRealtimeState("conversation_hardening"),
      durableMessageEvent({ eventId: "event_gap", sequence: 3, cursor: "3" }),
    );
    expect(gap.replay.errors).toEqual([
      expect.objectContaining({ kind: "replay_gap", message: "Replay gap before sequence 3." }),
    ]);
  });

  test("client-safe read models and long or RTL content do not leak internals", () => {
    const longRtlText = `שלום ${"relationship-summary ".repeat(80)}`.trim();
    const state = applyRealtimeEvent(initialRealtimeState("conversation_hardening"), {
      ...durableMessageEvent(),
      payload: {
        body: longRtlText,
        projectionValues: [
          { category: "message_text", value: longRtlText, evidenceRefs: [durableEvidence] },
          { category: "raw_prompt", value: "raw prompt leak" },
          { category: "provider_payload", value: { payload: "provider payload leak" } },
          { category: "policy_internals", value: "policy internals leak" },
          { category: "privacy_placeholder_map", value: { placeholder: "private placeholder leak" } },
          { category: "staff_notes", value: "staff-only note leak" },
        ],
      },
    });
    const readModel = projectRealtimeReadModel(state, "client");
    const serialized = JSON.stringify(readModel);

    expect(readModel.messages[0]?.body).toBe(longRtlText);
    expect(readModel.messages[0]?.body).toContain("שלום");
    expect(serialized).not.toContain("raw prompt leak");
    expect(serialized).not.toContain("provider payload leak");
    expect(serialized).not.toContain("policy internals leak");
    expect(serialized).not.toContain("private placeholder leak");
    expect(serialized).not.toContain("staff-only note leak");
    expect(readModel.denied.map((denied) => denied.category)).toEqual([
      "raw_prompt",
      "provider_payload",
      "policy_internals",
      "privacy_placeholder_map",
      "staff_notes",
    ]);
  });

  test("experience settings, reduced motion, tokens, and i18n stay contract-driven", () => {
    const resolved = resolveEffectiveExperienceSettings(
      {
        ...defaultExperienceSettings,
        theme: "high_contrast",
        density: "relaxed",
        motion: "cinematic",
        contrast: "high",
        typeScale: "xl",
        colorBlindMode: "tritanopia",
        locale: "en-US",
      },
      { role: "client", reducedMotionRequired: true },
    );
    const profile = accessibilityProfile(resolved);
    const tokens = themeTokensToCssVariables(themeManifestById(resolved.effective.theme));

    expect(experienceAttributes(resolved)).toEqual({
      "data-theme": "high_contrast",
      "data-density": "relaxed",
      "data-motion": "off",
      "data-type-scale": "xl",
      "data-contrast": "high",
      lang: "en-US",
    });
    expect(profile).toMatchObject({
      reducedMotion: true,
      contrast: "high",
      typeScale: "xl",
      density: "relaxed",
      statusPresentation: "text_only",
      liveRegionVerbosity: "status_summary",
    });
    expect(tokens["--ordo-focus-ring"]).toBe("#005fcc");
    expect(tokens["--ordo-motion-duration-base"]).toBe("0ms");
    expect(lookupMessage(defaultCatalog, "ordo.shell.composer")).toMatchObject({
      value: "Composer",
      missing: false,
    });
  });

  test("capability fallback, candidate artifacts, and explicit failures are enforced", async () => {
    const unavailable = await new BrowserCapabilityRuntime({ now: () => "2026-05-10T00:00:00Z" }).run(
      capabilityRequest({ capabilityId: "privacy.redaction_scan" }),
    );
    expect(unavailable).toMatchObject({
      status: "fallback_required",
      fallbackReason: "missing_capability",
      safeError: expect.objectContaining({ kind: "capability_unavailable" }),
    });

    const runtime = new BrowserCapabilityRuntime({ now: () => "2026-05-10T00:00:00Z" });
    runtime.register(createFileHashCapability());
    const candidate = await runtime.run(capabilityRequest());

    expect(candidate.status).toBe("candidate");
    expect(candidate.candidateEvidenceRefs[0]).toEqual(expect.objectContaining({ durability: "candidate" }));
    expect(candidate.candidateArtifactRefs[0]).toEqual(expect.objectContaining({ durability: "candidate" }));
    expect("durableArtifactId" in candidate).toBe(false);
  });

  test("composer remains logically available during non-blocking background work", () => {
    const shell = composeOrdoShell("client", "chat");
    const state = queueRealtimeCommand(
      initialRealtimeState("conversation_hardening"),
      createRealtimeCommand({
        kind: "message.submit",
        payload: { bodyMarkdown: "Non-blocking work", clientMessageId: "client_background_1" },
        actor,
        clientId: "client_background_1",
        intentId: "intent_background_1",
        issuedAt: "2026-05-10T00:00:00Z",
      }),
      { optimisticMessage: { body: "Non-blocking work" } },
    );
    const readModel = projectRealtimeReadModel(state, "client");

    expect(isSlotEnabled(shell, "composer")).toBe(true);
    expect(isSlotEnabled(shell, "active_work_strip")).toBe(true);
    expect(readModel.composer.status).toBe("sending");
    expect(readModel.composer.errors).toEqual([]);
  });

  test("markdown and HTML boundary remains explicit until a sanitizer seam exists", () => {
    const searchedFiles = [
      "lib",
      "components",
      "app",
    ].flatMap((directory) => sourceFiles(directory));
    const unsafeRendererFiles = searchedFiles.filter((file) => {
      const content = readFileSync(file, "utf8");
      return content.includes("dangerouslySetInnerHTML") || content.includes(".innerHTML");
    });

    expect(unsafeRendererFiles).toEqual([]);
  });
});

function sourceFiles(directory: string): string[] {
  const root = join(process.cwd(), directory);
  const entries = readdirSync(root);
  return entries.flatMap((entry) => {
    const fullPath = join(root, entry);
    const stat = statSync(fullPath);
    if (stat.isDirectory()) {
      return sourceFiles(join(directory, entry));
    }
    return /\.(ts|tsx)$/.test(entry) ? [fullPath] : [];
  });
}
