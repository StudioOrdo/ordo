import { expect, test } from "@playwright/test";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";

import {
  buildStudioStoryIntakeView,
  type StoryFounderIntakePacket,
} from "@/lib/studio-story-intake";

const daemonPort = 19080;

interface MockDaemonState {
  requests: string[];
  bodies: unknown[];
  mode: "ready" | "blocked";
}

test.describe("Studio Story intake view model", () => {
  test("maps readiness evidence without leaking private intake text", () => {
    const view = buildStudioStoryIntakeView(storyIntakePacketFixture("ready"));

    expect(view.status).toBe("ready");
    expect(view.workflowCompilation?.status).toBe("compiled");
    expect(view.workflowCompilation?.taskCount).toBe(3);
    expect(view.workflowCompilation?.approvalGates).toContain("Publish required");
    expect(view.readinessLabel).toBe("Ready for story planning");
    expect(view.narrativeDeckReady).toBe(true);
    expect(view.safeEvidenceRefCount).toBe(2);
    expect(view.summaryLines).toEqual([
      "Story Intake has a saved production plan for Studio Preview.",
      "3 planned step(s) and 3 safe evidence ref(s) are ready to review.",
      "Nothing has been published, promoted to memory, written to graph truth, sent to providers, or run as a task.",
    ]);
    expect(view.nextActions).toContain("Review the story production plan");
    expect(view.limitations).toContain("Owner review required before public derivative use");
    expect(JSON.stringify(view)).not.toContain("Internal founder note");
    expect(JSON.stringify(view)).not.toContain("provider internal");
    expect(JSON.stringify(view)).not.toContain("prompt internal");
    expect(JSON.stringify(view)).not.toContain("private artifact text");
    expect(JSON.stringify(view)).not.toContain("compiled plan");
    expect(JSON.stringify(view)).not.toContain("task private payload");
    expect(JSON.stringify(view)).not.toContain("graph certainty");
  });

  test("filters unsafe identifier-shaped internal text", () => {
    const packet = storyIntakePacketFixture("ready");
    packet.publicDerivative.claims = [
      {
        claim: "provider_internal_prompt should not render",
        evidenceRefs: ["artifact:founder_note"],
        reviewState: "raw_policy_internal",
        limitations: ["owner_only_data", "staff_routing", "task_private_payload"],
      },
      {
        claim: "Public-safe claim remains visible.",
        evidenceRefs: ["business_fact:positioning"],
        reviewState: "evidence_backed",
        limitations: [],
      },
    ];
    packet.publicDerivative.limitations = ["compiled_plan_inputs", "owner_review_required_before_public_derivative_use"];

    const view = buildStudioStoryIntakeView(packet);
    const serialized = JSON.stringify(view);

    expect(view.claims.map((claim) => claim.claim)).toEqual(["Public safe claim remains visible"]);
    expect(view.limitations).toContain("Owner review required before public derivative use");
    expect(serialized).not.toContain("provider internal");
    expect(serialized).not.toContain("raw policy");
    expect(serialized).not.toContain("owner only");
    expect(serialized).not.toContain("staff routing");
    expect(serialized).not.toContain("task private payload");
    expect(serialized).not.toContain("compiled plan");
  });

  test("keeps blocked readiness explicit", () => {
    const view = buildStudioStoryIntakeView(storyIntakePacketFixture("blocked"));

    expect(view.status).toBe("blocked");
    expect(view.workflowCompilation?.status).toBe("missing_input");
    expect(view.narrativeDeckReady).toBe(false);
    expect(view.missingPrerequisites).toEqual(["Evidence backed public story pack claims"]);
    expect(view.summaryLines).toEqual([
      "Story Intake cannot prepare the production plan until missing information is resolved.",
      "1 required item(s) still need attention.",
      "Nothing has been published, promoted to memory, written to graph truth, sent to providers, or run as a task.",
    ]);
    expect(view.nextActions).toContain("Resolve evidence backed public story pack claims");
  });
});

test.describe.configure({ mode: "serial" });

test.afterEach(async ({ page }) => {
  await page.close();
});

test("Studio Story intake renders safe readiness evidence", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon("ready");
  try {
    await page.goto(storyIntakeUrl("studio", testInfo));

    await expect(page.locator("main").getByRole("heading", { name: "Story Intake", exact: true })).toBeVisible();
    await expect(page.locator("main")).toContainText("Story Intake Check");
    await expect(page.locator("main")).toContainText("Production Plan");
    await expect(page.locator("main")).toContainText("compiled");
    await expect(page.locator("main")).toContainText("studio.story.scrollytelling_homepage v1");
    await expect(page.locator("main")).toContainText("workflow_compilation:story_intake_ui");
    await expect(page.locator("main")).toContainText("homepage.createNarrativeDeck");
    await expect(page.locator("main")).toContainText("publish.requestApproval");
    await expect(page.locator("main")).toContainText("2 safe local ref(s)");
    await expect(page.locator("main")).toContainText("Review the story production plan");
    await expect(page.locator("main")).toContainText("Owner review required before public derivative use");
    await expect(page.locator("main")).not.toContainText("Internal founder note");
    await expect(page.locator("main")).not.toContainText("provider internal");
    await expect(page.locator("main")).not.toContainText("prompt internal");
    await expect(page.locator("main")).not.toContainText("private artifact text");
    await expect(page.locator("main")).not.toContainText("compiled plan");
    await expect(page.locator("main")).not.toContainText("task private payload");
    await expect(page.locator("main")).not.toContainText("graph certainty");
    expect(daemon.state.requests).toEqual(["POST /studio/story-founder-intake"]);
    expect(JSON.stringify(daemon.state.bodies[0])).toContain("story-intake-ui");
    expect(JSON.stringify(daemon.state.bodies[0])).toContain("proofEvidenceRefs");
    expect(JSON.stringify(daemon.state.bodies[0])).not.toContain("evidenceRefs");
  } finally {
    await daemon.close();
  }
});

test("Studio Story intake refuses member role before daemon reads", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon("ready");
  try {
    await page.goto(storyIntakeUrl("member", testInfo));

    await expect(page.locator("body")).not.toContainText("Story Intake Check");
    expect(daemon.state.requests).toEqual([]);
  } finally {
    await daemon.close();
  }
});

test("Studio Story intake keeps empty and daemon-degraded states explicit", async ({ page }, testInfo) => {
  await page.goto(productContentUrl("/studio/story-intake?role=studio", testInfo));
  await expect(page.locator("main").getByRole("heading", { name: "Story Intake", exact: true })).toBeVisible();
  await expect(page.locator("main")).toContainText("No Story Intake has been submitted from this workbench yet.");
  await expect(page.locator("main")).toContainText("Ordo will not prepare the story plan until safe intake evidence exists.");

  await page.goto(storyIntakeUrl("studio", testInfo));
  await expect(page.locator("main")).toContainText("needs attention");
  await expect(page.locator("main")).toContainText("Ordo cannot read the local Story Intake record right now.");
  await expect(page.locator("main")).toContainText("/studio/story-founder-intake");
});

async function startMockDaemon(mode: MockDaemonState["mode"]): Promise<{ state: MockDaemonState; close: () => Promise<void> }> {
  const state: MockDaemonState = { requests: [], bodies: [], mode };
  const server = createServer((request, response) => void handleRequest(request, response, state));
  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(daemonPort, "127.0.0.1", () => {
      server.off("error", reject);
      resolve();
    });
  });
  return {
    state,
    close: () => closeServer(server),
  };
}

function closeServer(server: Server): Promise<void> {
  return new Promise((resolve, reject) => {
    server.close((error) => (error ? reject(error) : resolve()));
  });
}

async function handleRequest(request: IncomingMessage, response: ServerResponse, state: MockDaemonState) {
  const method = request.method ?? "GET";
  const path = request.url ?? "/";
  state.requests.push(`${method} ${path}`);

  if (method === "POST" && path === "/studio/story-founder-intake") {
    state.bodies.push(await readBody(request));
    return jsonResponse(response, storyIntakePacketFixture(state.mode));
  }

  response.writeHead(404, { "content-type": "application/json" });
  response.end(JSON.stringify({ error: `Unhandled mock daemon route: ${method} ${path}` }));
}

function readBody(request: IncomingMessage): Promise<unknown> {
  return new Promise((resolve) => {
    const chunks: Buffer[] = [];
    request.on("data", (chunk: Buffer) => chunks.push(chunk));
    request.on("end", () => {
      const body = Buffer.concat(chunks).toString("utf8");
      resolve(body ? JSON.parse(body) : null);
    });
  });
}

function jsonResponse(response: ServerResponse, body: unknown) {
  response.writeHead(200, { "content-type": "application/json" });
  response.end(JSON.stringify(body));
}

function storyIntakeUrl(role: "studio" | "member", testInfo: { project: { name: string } }): string {
  const params = new URLSearchParams({
    role,
    intakeId: "story-intake-ui",
    founderStory: "Studio Ordo turns approved founder evidence into a public story.",
    businessStance: "Ordo is a practical local-first answer to brittle hosted platforms.",
    audience: "founders",
    evidenceRefs: "artifact:founder_note,business_fact:positioning",
  });
  return productContentUrl(`/studio/story-intake?${params.toString()}`, testInfo);
}

function productContentUrl(path: string, testInfo: { project: { name: string } }): string {
  return testInfo.project.name === "mobile-chromium" ? `${path}${path.includes("?") ? "&" : "?"}mobile=content` : path;
}

function storyIntakePacketFixture(mode: "ready" | "blocked"): StoryFounderIntakePacket {
  const blocked = mode === "blocked";
  return {
    schemaVersion: "ordo.story_founder_intake_packet.v1",
    intakeId: "story-intake-ui",
    artifactRef: "artifact:story_intake_ui",
    artifact: {
      id: "artifact_story_intake_ui",
      artifactKind: "story.founder_intake",
      title: "Story founder intake story-intake-ui",
      status: "ready_for_review",
      visibilityCeiling: "owner",
      summary: "Studio Ordo turns approved founder evidence into a public story.",
      sourceKind: "story_pack_intake",
      sourceId: "story-intake-ui",
      evidenceRefs: ["artifact:founder_note", "business_fact:positioning"],
      provenance: { privateNotes: "Internal founder note should not render" },
      contentHash: "sha256:story-intake-ui",
      storageUri: "ordo://artifacts/story-founder-intakes/story-intake-ui",
      healthStatus: "contract_only",
      createdByJobId: null,
      createdAt: "2026-05-15T08:00:00.000Z",
      updatedAt: "2026-05-15T08:00:00.000Z",
    },
    version: null,
    publicDerivative: {
      intakeId: "story-intake-ui",
      summary: "Studio Ordo turns approved founder evidence into a public story.",
      audience: "founders",
      claims: blocked
        ? [
            {
              claim: "Generated-content candidate text with graph certainty should not render.",
              evidenceRefs: [],
              reviewState: "needs_review",
              limitations: ["provider internal note should not render"],
            },
          ]
        : [
            {
              claim: "Story Pack claims remain evidence-backed.",
              evidenceRefs: ["business_fact:positioning"],
              reviewState: "evidence_backed",
              limitations: [],
            },
          ],
      stylePreferences: ["plainspoken"],
      offerRefs: ["offer:pilot"],
      ctaRefs: ["cta:talk"],
      evidenceRefs: blocked ? ["artifact:founder_note"] : ["artifact:founder_note", "business_fact:positioning"],
      limitations: ["owner_review_required_before_public_derivative_use", "private artifact text should not render"],
      visibility: "public_derivative",
      memoryEffect: "candidate_only",
    },
    readiness: {
      status: blocked ? "blocked" : "ready_for_narrative_deck",
      narrativeDeckReady: !blocked,
      missing: blocked ? ["evidence_backed_public_story_pack_claims"] : [],
      evidenceRefs: blocked ? ["artifact:founder_note"] : ["artifact:founder_note", "business_fact:positioning"],
      limitations: ["owner_review_required_before_public_derivative_use"],
      liveProviderRequired: false,
      externalPublishingClaimed: false,
      automaticMemoryPromotion: false,
      confirmedGraphPromotion: false,
    },
    workflowCompilation: blocked
      ? {
          status: "blocked",
          templateId: "studio.story.scrollytelling_homepage",
          templateVersion: 1,
          idempotencyKey: "story-founder-intake:story-intake-ui:studio.story.scrollytelling_homepage:v1",
          compilationRef: null,
          inputHash: null,
          evidenceRefs: ["artifact:founder_note"],
          missingInputs: ["evidence_backed_public_story_pack_claims"],
          limitations: ["no_workflow_compilation_was_stored_while_required_inputs_were_missing"],
          safeNextActions: ["resolve_missing_public_safe_workflow_inputs"],
          resolvedVariables: [],
          taskBindings: [],
          fanoutGroups: [],
          approvalGates: [],
          providerRequirements: [],
          liveProviderRequired: false,
          taskExecutionPerformed: false,
          externalPublishingClaimed: false,
          memoryPromotionPerformed: false,
          confirmedGraphPromotion: false,
        }
      : {
          status: "compiled",
          templateId: "studio.story.scrollytelling_homepage",
          templateVersion: 1,
          idempotencyKey: "story-founder-intake:story-intake-ui:studio.story.scrollytelling_homepage:v1",
          compilationRef: "workflow_compilation:story_intake_ui",
          inputHash: "sha256:story-workflow-input",
          evidenceRefs: ["artifact:founder_note", "business_fact:positioning", "workflow_compilation:story_intake_ui"],
          missingInputs: [],
          limitations: ["workflow_compilation_evidence_is_not_task_execution"],
          safeNextActions: ["review_workflow_compilation_evidence"],
          resolvedVariables: [
            {
              key: "founderProfile",
              sourceKind: "input",
              visibility: "private",
              evidenceRefCount: 0,
              valueExposed: false,
            },
          ],
          taskBindings: [
            {
              key: "deck.create",
              method: "homepage.createNarrativeDeck",
              dependsOn: [],
              visibility: "staff",
              fanout: null,
              providerRequirement: "llm.mock",
              outputArtifactKind: "narrative_deck",
            },
            {
              key: "homepage.compile_draft",
              method: "homepage.compileScrollytellingDraft",
              dependsOn: ["deck.create"],
              visibility: "staff",
              fanout: null,
              providerRequirement: null,
              outputArtifactKind: "story.homepage_version",
            },
            {
              key: "publish.approval",
              method: "publish.requestApproval",
              dependsOn: ["homepage.compile_draft"],
              visibility: "staff",
              fanout: null,
              providerRequirement: null,
              outputArtifactKind: "story.homepage_publish_approval_package",
            },
          ],
          fanoutGroups: [{ key: "section", itemCount: 2, maxItems: 12 }],
          approvalGates: [{ key: "manual_publish_approval", action: "publish", required: true }],
          providerRequirements: [
            {
              key: "llm.mock",
              capability: "homepage.createNarrativeDeck",
              mode: "deterministic_mock",
              egress: "none",
              visibility: "staff",
            },
          ],
          liveProviderRequired: false,
          taskExecutionPerformed: false,
          externalPublishingClaimed: false,
          memoryPromotionPerformed: false,
          confirmedGraphPromotion: false,
        },
    mutationPerformed: true,
    approvalState: "needs_review",
    visibilityCeiling: "owner",
    liveProviderCalled: false,
    externalPublishingClaimed: false,
    memoryPromotionPerformed: false,
    confirmedGraphPromotion: false,
    event: null,
  };
}
