import {
  createUiError,
  type ActorRef,
  type EvidenceRef,
  type UiError,
} from "@/lib/ordoos-frontend-contracts";

export type BrowserCapabilityId = "file.hash" | "privacy.redaction_scan";

export type BrowserCapabilityStatus =
  | "candidate"
  | "failed"
  | "canceled"
  | "timed_out"
  | "fallback_required";

export type BrowserCapabilityFallbackReason =
  | "missing_capability"
  | "input_oversized"
  | "timeout"
  | "canceled"
  | "runtime_error";

export interface BrowserCapabilityBudget {
  maxInputBytes: number;
  memoryLimitBytes: number;
  timeoutMs: number;
}

export interface BrowserCapabilityInputMetadata {
  inputId: string;
  label: string;
  mediaType: string;
  byteLength: number;
  estimatedWorkMs?: number;
}

export interface BrowserCapabilityInput {
  metadata: BrowserCapabilityInputMetadata;
  text?: string;
  bytes?: Uint8Array;
}

export interface BrowserCapabilityRequest {
  requestId: string;
  capabilityId: BrowserCapabilityId;
  issuedAt: string;
  actor: ActorRef;
  input: BrowserCapabilityInput;
  budget: BrowserCapabilityBudget;
  cancelToken?: string;
}

export interface BrowserCapabilityBudgetSummary {
  inputBytes: number;
  maxInputBytes: number;
  memoryLimitBytes: number;
  timeoutMs: number;
  estimatedWorkMs: number;
}

export interface BrowserCandidateArtifactRef {
  id: string;
  kind: "browser_candidate";
  durability: "candidate";
  summary: string;
}

export interface BrowserCapabilityResult {
  schemaVersion: "ordo.browser_capability_result.v1";
  resultId: string;
  requestId: string;
  capabilityId: BrowserCapabilityId;
  status: BrowserCapabilityStatus;
  issuedAt: string;
  completedAt: string;
  durationMs: number;
  outputHash?: string;
  candidateEvidenceRefs: readonly EvidenceRef[];
  candidateArtifactRefs: readonly BrowserCandidateArtifactRef[];
  durableArtifactId?: never;
  budget: BrowserCapabilityBudgetSummary;
  fallbackReason?: BrowserCapabilityFallbackReason;
  safeError?: UiError;
  summary: string;
}

export interface BrowserCapability {
  id: BrowserCapabilityId;
  label: string;
  run(request: BrowserCapabilityRequest, runtime: BrowserCapabilityRuntimeContext): Promise<BrowserCapabilityResult>;
}

export interface BrowserCapabilityAvailability {
  capabilityId: BrowserCapabilityId;
  available: boolean;
  label?: string;
  fallbackReason?: BrowserCapabilityFallbackReason;
}

export interface BrowserCapabilityRuntimeContext {
  now: () => string;
  isCanceled: (cancelToken: string | undefined) => boolean;
}

export interface BrowserCapabilityRuntimePort {
  register(capability: BrowserCapability): void;
  cancel(cancelToken: string): void;
  availability(capabilityId: BrowserCapabilityId): BrowserCapabilityAvailability;
  run(request: BrowserCapabilityRequest): Promise<BrowserCapabilityResult>;
}

export class BrowserCapabilityRuntime implements BrowserCapabilityRuntimePort {
  private readonly capabilities = new Map<BrowserCapabilityId, BrowserCapability>();
  private readonly canceled = new Set<string>();

  constructor(private readonly options: { now?: () => string } = {}) {}

  register(capability: BrowserCapability): void {
    this.capabilities.set(capability.id, capability);
  }

  cancel(cancelToken: string): void {
    this.canceled.add(cancelToken);
  }

  availability(capabilityId: BrowserCapabilityId): BrowserCapabilityAvailability {
    const capability = this.capabilities.get(capabilityId);
    if (!capability) {
      return { capabilityId, available: false, fallbackReason: "missing_capability" };
    }
    return { capabilityId, available: true, label: capability.label };
  }

  async run(request: BrowserCapabilityRequest): Promise<BrowserCapabilityResult> {
    const startedAt = this.now();
    const capability = this.capabilities.get(request.capabilityId);
    if (!capability) {
      return fallbackResult(request, startedAt, startedAt, "missing_capability", "Browser capability is unavailable.");
    }

    const budgetFailure = validateBudget(request);
    if (budgetFailure) {
      return fallbackResult(request, startedAt, startedAt, budgetFailure.reason, budgetFailure.message);
    }

    if (this.isCanceled(request.cancelToken)) {
      return terminalResult(request, startedAt, startedAt, "canceled", "canceled", "Browser capability job was canceled.");
    }

    if ((request.input.metadata.estimatedWorkMs ?? 0) > request.budget.timeoutMs) {
      return terminalResult(request, startedAt, startedAt, "timed_out", "timeout", "Browser capability job timed out.");
    }

    try {
      return await capability.run(request, {
        now: () => this.now(),
        isCanceled: (cancelToken) => this.isCanceled(cancelToken),
      });
    } catch {
      return terminalResult(request, startedAt, this.now(), "failed", "runtime_error", "Browser capability job failed.");
    }
  }

  private now(): string {
    return this.options.now?.() ?? new Date().toISOString();
  }

  private isCanceled(cancelToken: string | undefined): boolean {
    return cancelToken ? this.canceled.has(cancelToken) : false;
  }
}

export function createFileHashCapability(): BrowserCapability {
  return {
    id: "file.hash",
    label: "File hash",
    async run(request, runtime) {
      if (runtime.isCanceled(request.cancelToken)) {
        const now = runtime.now();
        return terminalResult(request, request.issuedAt, now, "canceled", "canceled", "Browser capability job was canceled.");
      }
      const now = runtime.now();
      const outputHash = await sha256Hex(inputBytes(request.input));
      const evidenceRef = browserCandidateEvidenceRef(request, outputHash);
      return {
        schemaVersion: "ordo.browser_capability_result.v1",
        resultId: resultIdFor(request),
        requestId: request.requestId,
        capabilityId: request.capabilityId,
        status: "candidate",
        issuedAt: request.issuedAt,
        completedAt: now,
        durationMs: request.input.metadata.estimatedWorkMs ?? 0,
        outputHash,
        candidateEvidenceRefs: [evidenceRef],
        candidateArtifactRefs: [
          {
            id: `browser_candidate_artifact:${request.requestId}`,
            kind: "browser_candidate",
            durability: "candidate",
            summary: `Candidate hash for ${request.input.metadata.label}`,
          },
        ],
        budget: budgetSummary(request),
        summary: "Browser-local candidate hash generated; daemon validation required before durable artifact identity.",
      };
    },
  };
}

export function stableBrowserCapabilityResultJson(result: BrowserCapabilityResult): string {
  return JSON.stringify(result);
}

function validateBudget(
  request: BrowserCapabilityRequest,
): { reason: BrowserCapabilityFallbackReason; message: string } | null {
  if (request.input.metadata.byteLength > request.budget.maxInputBytes) {
    return {
      reason: "input_oversized",
      message: "Browser capability input exceeds the configured size budget.",
    };
  }
  if (request.budget.timeoutMs <= 0) {
    return {
      reason: "timeout",
      message: "Browser capability timeout budget is exhausted.",
    };
  }
  return null;
}

function fallbackResult(
  request: BrowserCapabilityRequest,
  startedAt: string,
  completedAt: string,
  fallbackReason: BrowserCapabilityFallbackReason,
  message: string,
): BrowserCapabilityResult {
  return terminalResult(request, startedAt, completedAt, "fallback_required", fallbackReason, message);
}

function terminalResult(
  request: BrowserCapabilityRequest,
  startedAt: string,
  completedAt: string,
  status: Exclude<BrowserCapabilityStatus, "candidate">,
  fallbackReason: BrowserCapabilityFallbackReason,
  message: string,
): BrowserCapabilityResult {
  return {
    schemaVersion: "ordo.browser_capability_result.v1",
    resultId: resultIdFor(request),
    requestId: request.requestId,
    capabilityId: request.capabilityId,
    status,
    issuedAt: request.issuedAt,
    completedAt,
    durationMs: startedAt === completedAt ? 0 : request.input.metadata.estimatedWorkMs ?? 0,
    candidateEvidenceRefs: [],
    candidateArtifactRefs: [],
    budget: budgetSummary(request),
    fallbackReason,
    safeError: createUiError(errorKindFor(status), message),
    summary: message,
  };
}

function errorKindFor(status: Exclude<BrowserCapabilityStatus, "candidate">) {
  if (status === "fallback_required") {
    return "capability_unavailable";
  }
  if (status === "failed") {
    return "capability_failed";
  }
  return "capability_failed";
}

function resultIdFor(request: BrowserCapabilityRequest): string {
  return `browser_capability_result:${request.requestId}`;
}

function browserCandidateEvidenceRef(request: BrowserCapabilityRequest, outputHash: string): EvidenceRef {
  return {
    id: `browser_candidate:${request.requestId}:${outputHash}`,
    kind: "browser_candidate",
    durability: "candidate",
    visibility: "client",
    summary: `${request.capabilityId} produced candidate output for ${request.input.metadata.label}`,
  };
}

function budgetSummary(request: BrowserCapabilityRequest): BrowserCapabilityBudgetSummary {
  return {
    inputBytes: request.input.metadata.byteLength,
    maxInputBytes: request.budget.maxInputBytes,
    memoryLimitBytes: request.budget.memoryLimitBytes,
    timeoutMs: request.budget.timeoutMs,
    estimatedWorkMs: request.input.metadata.estimatedWorkMs ?? 0,
  };
}

function inputBytes(input: BrowserCapabilityInput): Uint8Array {
  if (input.bytes) {
    return input.bytes;
  }
  return new TextEncoder().encode(input.text ?? "");
}

async function sha256Hex(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", bytes.slice().buffer);
  return `sha256:${Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("")}`;
}
