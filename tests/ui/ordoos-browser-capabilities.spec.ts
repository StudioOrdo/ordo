import { expect, test } from "@playwright/test";

import { type ActorRef } from "@/lib/ordoos-frontend-contracts";
import {
  BrowserCapabilityRuntime,
  createFileHashCapability,
  stableBrowserCapabilityResultJson,
  type BrowserCapabilityRequest,
} from "@/lib/ordoos-browser-capabilities";

const actor: ActorRef = {
  actorId: "actor_client",
  role: "client",
  displayKind: "client",
};

function runtime() {
  return new BrowserCapabilityRuntime({ now: () => "2026-05-10T00:00:00Z" });
}

function request(overrides: Partial<BrowserCapabilityRequest> = {}): BrowserCapabilityRequest {
  return {
    requestId: "request_file_hash_1",
    capabilityId: "file.hash",
    issuedAt: "2026-05-10T00:00:00Z",
    actor,
    input: {
      metadata: {
        inputId: "input_1",
        label: "synthetic-note.md",
        mediaType: "text/markdown",
        byteLength: 17,
        estimatedWorkMs: 4,
      },
      text: "synthetic content",
    },
    budget: {
      maxInputBytes: 1024,
      memoryLimitBytes: 2048,
      timeoutMs: 50,
    },
    ...overrides,
  };
}

test.describe("OrdoOS browser capability runtime", () => {
  test("runtime registers and reports file.hash availability", () => {
    const capabilities = runtime();

    expect(capabilities.availability("file.hash")).toEqual({
      capabilityId: "file.hash",
      available: false,
      fallbackReason: "missing_capability",
    });

    capabilities.register(createFileHashCapability());

    expect(capabilities.availability("file.hash")).toEqual({
      capabilityId: "file.hash",
      available: true,
      label: "File hash",
    });
  });

  test("file.hash returns deterministic candidate SHA-256 evidence", async () => {
    const capabilities = runtime();
    capabilities.register(createFileHashCapability());

    const result = await capabilities.run(request());

    expect(result).toMatchObject({
      schemaVersion: "ordo.browser_capability_result.v1",
      resultId: "browser_capability_result:request_file_hash_1",
      requestId: "request_file_hash_1",
      capabilityId: "file.hash",
      status: "candidate",
      outputHash: "sha256:a04dc6c38580462e46df7968a7d91a006455989bb839a6c9c1cf5fb4c4551a47",
      durationMs: 4,
    });
    expect(result.candidateEvidenceRefs).toEqual([
      expect.objectContaining({
        kind: "browser_candidate",
        durability: "candidate",
        visibility: "client",
      }),
    ]);
    expect(result.candidateArtifactRefs).toEqual([
      expect.objectContaining({
        kind: "browser_candidate",
        durability: "candidate",
      }),
    ]);
    expect("durableArtifactId" in result).toBe(false);
  });

  test("same input returns same deterministic result shape under fixed clock", async () => {
    const capabilities = runtime();
    capabilities.register(createFileHashCapability());

    const first = await capabilities.run(request());
    const second = await capabilities.run(request());

    expect(stableBrowserCapabilityResultJson(first)).toBe(stableBrowserCapabilityResultJson(second));
  });

  test("missing capability returns fallback required", async () => {
    const result = await runtime().run(request({ capabilityId: "privacy.redaction_scan" }));

    expect(result.status).toBe("fallback_required");
    expect(result.fallbackReason).toBe("missing_capability");
    expect(result.safeError).toEqual(expect.objectContaining({ kind: "capability_unavailable" }));
    expect(result.candidateEvidenceRefs).toEqual([]);
    expect(result.candidateArtifactRefs).toEqual([]);
  });

  test("oversized input is rejected before hashing", async () => {
    const capabilities = runtime();
    capabilities.register(createFileHashCapability());

    const result = await capabilities.run(
      request({
        input: {
          metadata: {
            inputId: "input_large",
            label: "large.bin",
            mediaType: "application/octet-stream",
            byteLength: 4096,
            estimatedWorkMs: 4,
          },
          text: "synthetic content",
        },
        budget: {
          maxInputBytes: 128,
          memoryLimitBytes: 256,
          timeoutMs: 50,
        },
      }),
    );

    expect(result.status).toBe("fallback_required");
    expect(result.fallbackReason).toBe("input_oversized");
    expect(result.outputHash).toBeUndefined();
  });

  test("cancellation produces canceled result", async () => {
    const capabilities = runtime();
    capabilities.register(createFileHashCapability());
    capabilities.cancel("cancel_file_hash_1");

    const result = await capabilities.run(request({ cancelToken: "cancel_file_hash_1" }));

    expect(result.status).toBe("canceled");
    expect(result.fallbackReason).toBe("canceled");
    expect(result.safeError).toEqual(expect.objectContaining({ kind: "capability_failed" }));
  });

  test("timeout produces timed out result", async () => {
    const capabilities = runtime();
    capabilities.register(createFileHashCapability());

    const result = await capabilities.run(
      request({
        input: {
          metadata: {
            inputId: "input_slow",
            label: "slow.txt",
            mediaType: "text/plain",
            byteLength: 17,
            estimatedWorkMs: 100,
          },
          text: "synthetic content",
        },
        budget: {
          maxInputBytes: 1024,
          memoryLimitBytes: 2048,
          timeoutMs: 50,
        },
      }),
    );

    expect(result.status).toBe("timed_out");
    expect(result.fallbackReason).toBe("timeout");
    expect(result.outputHash).toBeUndefined();
  });

  test("result summaries do not include raw private input content", async () => {
    const capabilities = runtime();
    capabilities.register(createFileHashCapability());

    const result = await capabilities.run(
      request({
        input: {
          metadata: {
            inputId: "input_private",
            label: "private-note.md",
            mediaType: "text/markdown",
            byteLength: 33,
            estimatedWorkMs: 4,
          },
          text: "private fixture value 555-123-4567",
        },
      }),
    );
    const serialized = stableBrowserCapabilityResultJson(result);

    expect(serialized).not.toContain("private fixture value");
    expect(serialized).not.toContain("555-123-4567");
    expect(result.summary).toContain("daemon validation required");
  });
});
