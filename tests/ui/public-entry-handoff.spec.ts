import { expect, test } from "@playwright/test";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";

const daemonPort = 19080;

interface MockDaemonState {
  requests: string[];
  bodies: unknown[];
  handoffRequests: number;
}

test.describe.configure({ mode: "serial" });

test.afterEach(async ({ page }) => {
  await page.close();
});

test("public tracked entry starts safe idempotent relationship handoff", async ({ page }) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto("/e/nyc-founder-table?locationLabel=NYC%20Founder%20Table&locationKind=event_booth");

    await expect(page.locator("main").getByRole("heading", { name: "NYC Founder Table" })).toBeVisible();
    await expect(page.locator("main")).toContainText("Visitor session recorded");
    await expect(page.locator("main")).toContainText("Your relationship handoff is queued for Support review.");
    await expect(page.locator("main")).toContainText("A support-capable member can claim this request when available.");
    await expect(page.locator("main")).toContainText("No hidden location tracking");
    await expect(page.locator("main")).toContainText("No reward for scan alone");
    await expect(page.locator("main")).not.toContainText("actor_keith");
    await expect(page.locator("main")).not.toContainText("assigneeActorId");
    await expect(page.locator("main")).not.toContainText("destinationId");
    await expect(page.locator("main")).not.toContainText("rawPrompt");
    await expect(page.locator("main")).not.toContainText("provider");
    await expect(page.locator("main")).not.toContainText("policy");
    await expect(page.locator("main")).not.toContainText("graph certainty");
    await expect(page.locator("main")).not.toContainText("memory promotion");

    await page.reload();
    await expect(page.locator("main")).toContainText("Your relationship handoff is queued for Support review.");

    expect(daemon.state.requests).toContain("GET /public/e/nyc-founder-table");
    expect(daemon.state.requests.filter((request) => request === "POST /public/e/nyc-founder-table/relationship-handoff")).toHaveLength(2);
    expect(daemon.state.handoffRequests).toBe(2);
    expect(JSON.stringify(daemon.state.bodies)).toContain("visitor_session_nyc");
    expect(JSON.stringify(daemon.state.bodies)).toContain("tracked_entry_point:nyc-founder-table");
  } finally {
    await daemon.close();
  }
});

async function startMockDaemon(): Promise<{ state: MockDaemonState; close: () => Promise<void> }> {
  const state: MockDaemonState = { requests: [], bodies: [], handoffRequests: 0 };
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

  if (method === "GET" && path === "/public/e/nyc-founder-table") {
    return jsonResponse(response, {
      slug: "nyc-founder-table",
      label: "NYC Founder Table",
      destinationSurface: "about",
      destinationId: null,
      publicPath: "/public/e/nyc-founder-table",
      qrPayload: { kind: "ordo.tracked_entry_point", slug: "nyc-founder-table" },
    });
  }

  if (method === "POST" && path === "/public/visitor-sessions") {
    state.bodies.push(await readBody(request));
    return jsonResponse(response, {
      id: "visitor_session_nyc",
      entryPointSlug: "nyc-founder-table",
      status: "active",
      destinationSurface: "about",
      destinationId: null,
      createdAt: "2026-05-13T10:01:00.000Z",
      lastSeenAt: "2026-05-13T10:01:00.000Z",
    });
  }

  if (method === "POST" && path === "/public/e/nyc-founder-table/relationship-handoff") {
    state.handoffRequests += 1;
    state.bodies.push(await readBody(request));
    return jsonResponse(response, {
      status: {
        requestId: "handoff_relationship_1",
        state: "waiting",
        summary: "Your relationship handoff is queued for Support review.",
        nextStep: "A support-capable member can claim this request when available.",
        sourceKind: "visitor_session",
        sourceId: "visitor_session_nyc",
        evidenceRefs: ["tracked_entry_point:nyc-founder-table", "visitor_session:visitor_session_nyc"],
        allowedActions: ["view_public_status"],
        limitations: [
          "Staff routing stays hidden from public status.",
          "Support claim eligibility is governed by support.accept_handoff.",
        ],
        updatedAt: "2026-05-13T10:02:00.000Z",
      },
    });
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
