import { expect, test } from "@playwright/test";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";

import { buildSupportHandoffQueueView } from "@/lib/support-handoffs";
import type { HandoffInboxItemView } from "@/lib/daemon-client";

const daemonPort = 19080;

interface MockDaemonState {
  requests: string[];
}

test.describe("Support handoff queue view model", () => {
  test("projects daemon handoff items without leaking raw request internals", () => {
    const view = buildSupportHandoffQueueView([
      handoffFixture({
        evidenceRefs: [
          "tracked_entry_point:nyc-founder-table",
          "visitor_session:visitor_session_nyc",
          "provider_internal:do-not-render",
          "private:staff-note",
        ],
        request: {
          rawPrompt: "never show raw prompt",
          privateNote: "never show private note",
        },
        evidence: {
          policyInternal: "never show policy internals",
        },
      }),
      handoffFixture({
        id: "handoff_item_claimed",
        deliveryState: "assigned",
        assigneeActorId: "actor_support_1",
        reason: "Claimed relationship support",
      }),
    ]);

    expect(view.openCount).toBe(1);
    expect(view.claimedCount).toBe(1);
    expect(view.items[0]?.safeEvidenceRefs).toEqual([
      "tracked_entry_point:nyc-founder-table",
      "visitor_session:visitor_session_nyc",
    ]);
    expect(JSON.stringify(view)).not.toContain("rawPrompt");
    expect(JSON.stringify(view)).not.toContain("private note");
    expect(JSON.stringify(view)).not.toContain("policyInternal");
    expect(JSON.stringify(view)).not.toContain("provider_internal");
    expect(view.summaryLines).toContain("1 open handoff(s) need support attention.");
  });
});

test.describe.configure({ mode: "serial" });

test.afterEach(async ({ page }) => {
  await page.close();
});

test("Staff Handoffs renders daemon-backed support queue safely", async ({ page }) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto("/staff/handoffs?role=staff");

    await expect(page.locator("main").getByRole("heading", { name: "Handoffs", exact: true })).toBeVisible();
    await expect(page.locator("main")).toContainText("Handoff Queue");
    await expect(page.locator("main")).toContainText("1 open handoff(s) need support attention.");
    await expect(page.locator("main")).toContainText("First-user relationship handoff requested");
    await expect(page.locator("main")).toContainText("Visitor Session visitor_session_nyc");
    await expect(page.locator("main")).toContainText("Claim handoff with support.accept_handoff.");
    await expect(page.locator("main")).toContainText("tracked_entry_point:nyc-founder-table");
    await expect(page.locator("main")).toContainText("Some internal refs are hidden.");
    await expect(page.locator("main")).not.toContainText("rawPrompt");
    await expect(page.locator("main")).not.toContainText("private intake text");
    await expect(page.locator("main")).not.toContainText("provider_internal");
    await expect(page.locator("main")).not.toContainText("policy_internal");
    expect(daemon.state.requests).toContain("GET /handoff/inbox?limit=100");
  } finally {
    await daemon.close();
  }
});

test("Staff Handoffs refuses member role before daemon queue read", async ({ page }) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto("/staff/handoffs?role=client");

    await expect(page.locator("main")).toContainText("Support Access Required");
    await expect(page.locator("body")).not.toContainText("First-user relationship handoff requested");
    expect(daemon.state.requests).not.toContain("GET /handoff/inbox?limit=100");
  } finally {
    await daemon.close();
  }
});

async function startMockDaemon(): Promise<{ state: MockDaemonState; close: () => Promise<void> }> {
  const state: MockDaemonState = { requests: [] };
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

  if (method === "GET" && path === "/handoff/inbox?limit=100") {
    return jsonResponse(response, {
      items: [
        handoffFixture({
          evidenceRefs: [
            "tracked_entry_point:nyc-founder-table",
            "visitor_session:visitor_session_nyc",
            "provider_internal:do-not-render",
          ],
          request: {
            rawPrompt: "do not render",
            privateText: "private intake text",
          },
          evidence: {
            policyInternal: "policy_internal",
          },
        }),
      ],
    });
  }

  response.writeHead(404, { "content-type": "application/json" });
  response.end(JSON.stringify({ error: `Unhandled mock daemon route: ${method} ${path}` }));
}

function jsonResponse(response: ServerResponse, body: unknown) {
  response.writeHead(200, { "content-type": "application/json" });
  response.end(JSON.stringify(body));
}

function handoffFixture(overrides: Partial<HandoffInboxItemView> = {}): HandoffInboxItemView {
  return {
    id: "handoff_item_relationship",
    sourceKind: "visitor_session",
    sourceId: "visitor_session_nyc",
    destinationKind: "support",
    destinationId: "first_user_relationship",
    reason: "First-user relationship handoff requested",
    requestedAction: "Claim first-user relationship handoff",
    urgency: "normal",
    assigneeActorId: null,
    dueAt: null,
    nextActionHint: "Claim first-user relationship handoff",
    evidenceRefs: ["tracked_entry_point:nyc-founder-table"],
    visibility: "staff",
    request: {},
    evidence: {},
    approvalRequirement: "owner_review_only",
    deliveryState: "queued",
    ownerDecision: null,
    decisionReason: null,
    createdByActorId: "actor_system",
    createdAt: "2026-05-13T10:02:00.000Z",
    updatedAt: "2026-05-13T10:03:00.000Z",
    resolvedAt: null,
    ...overrides,
  };
}
