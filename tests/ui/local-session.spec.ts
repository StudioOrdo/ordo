import { expect, test } from "@playwright/test";

import { createLocalSession, parseLocalSessionCookie } from "@/lib/local-session";

test.describe("local appliance session scaffold", () => {
  const now = new Date("2026-05-12T12:00:00.000Z");

  test("register creates a client-safe local session read model", () => {
    const result = createLocalSession(
      { mode: "register", name: "Ava Client", email: "AVA@example.com", password: "local-only-pass" },
      now,
    );

    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.session).toMatchObject({
      schemaVersion: "ordo.local-session.v1",
      sessionKind: "local_appliance_session",
      role: "client",
      displayName: "Ava Client",
    });
    expect(result.session.actorId).toMatch(/^actor_local_member_/);
    expect(result.session.emailHash).toMatch(/^[a-f0-9]{64}$/);
    expect(JSON.stringify(result.session)).not.toContain("local-only-pass");
    expect(JSON.stringify(result.session)).not.toContain("AVA@example.com");
  });

  test("login and register are deterministic for the same local email", () => {
    const registered = createLocalSession(
      { mode: "register", name: "Ava Client", email: "ava@example.com", password: "local-only-pass" },
      now,
    );
    const loggedIn = createLocalSession(
      { mode: "login", email: " ava@example.com ", password: "local-only-pass" },
      now,
    );

    expect(registered.ok).toBe(true);
    expect(loggedIn.ok).toBe(true);
    if (!registered.ok || !loggedIn.ok) return;
    expect(loggedIn.session.actorId).toBe(registered.session.actorId);
    expect(loggedIn.session.sessionId).toBe(registered.session.sessionId);
    expect(loggedIn.session.emailHash).toBe(registered.session.emailHash);
  });

  test("invalid inputs fail without echoing secret-like values", () => {
    const secretLikePassword = "super-secret-value";
    const result = createLocalSession(
      { mode: "register", name: "   ", email: "not-an-email", password: secretLikePassword },
      now,
    );

    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).not.toContain(secretLikePassword);
    expect(result.error.message).not.toContain("not-an-email");
  });

  test("long and whitespace-only values are rejected safely", () => {
    const longName = "A".repeat(120);
    const longPassword = "p".repeat(160);

    expect(createLocalSession({ mode: "register", name: longName, email: "ava@example.com", password: "valid-pass" }, now).ok).toBe(false);
    expect(createLocalSession({ mode: "login", email: "   ", password: "valid-pass" }, now).ok).toBe(false);
    expect(createLocalSession({ mode: "login", email: "ava@example.com", password: longPassword }, now).ok).toBe(false);
  });

  test("malformed and expired session cookies do not authenticate", () => {
    const result = createLocalSession(
      { mode: "login", email: "ava@example.com", password: "local-only-pass" },
      now,
    );

    expect(parseLocalSessionCookie("not-a-session", now)).toBeNull();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(parseLocalSessionCookie(result.cookieValue, new Date("2026-06-13T12:00:00.000Z"))).toBeNull();
  });

  test("login form starts a local session and opens member chat", async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel("Email").fill("ava@example.com");
    await page.getByLabel("Password").fill("local-only-pass");
    await page.getByRole("button", { name: "Continue" }).click();

    await expect(page).toHaveURL(/\/my\/chat\?role=client$/);
    await page.goto("/my/chat?role=client&mobile=content");
    await expect(page.getByRole("article", { name: "Studio Ordo conversation" })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "Message Ordo" })).toBeVisible();
  });

  test("chat bootstrap wrapper returns safe degraded metadata when daemon is unavailable", async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel("Email").fill("ava@example.com");
    await page.getByLabel("Password").fill("local-only-pass");
    await page.getByRole("button", { name: "Continue" }).click();
    await expect(page).toHaveURL(/\/my\/chat/);

    const payload = await page.evaluate(async () => {
      const response = await fetch("/api/chat/bootstrap", { method: "POST" });
      return response.json();
    });
    const serialized = JSON.stringify(payload);

    expect(payload).toMatchObject({
      authenticated: true,
      bootstrap: null,
      status: "degraded",
      degradedReason: "Daemon chat bootstrap route unavailable; using local preview chat.",
    });
    expect(serialized).not.toContain("ava@example.com");
    expect(serialized).not.toContain("local-only-pass");
    expect(serialized).not.toContain("OPENAI_API_KEY");
    expect(serialized).not.toContain("sk-");
    expect(serialized).not.toContain("prompt");
  });

  test("member chat surfaces safe degraded run state when daemon is unavailable", async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel("Email").fill("ava@example.com");
    await page.getByLabel("Password").fill("local-only-pass");
    await page.getByRole("button", { name: "Continue" }).click();
    await expect(page).toHaveURL(/\/my\/chat/);
    await page.goto("/my/chat?role=client&mobile=content");

    await expect(page.getByText("Daemon chat bootstrap route unavailable; using local preview chat.")).toBeVisible();
    await page.getByRole("textbox", { name: "Message Ordo" }).fill("Can I use this locally?");
    await page.getByRole("button", { name: "Send message" }).click();

    const runState = page.getByLabel("Local chat run state");
    await expect(runState.getByText("Can I use this locally?")).toBeVisible();
    await expect(runState.getByText("Local preview only. Start the daemon to send this through the conversation gateway.")).toBeVisible();
    await expect(page.getByText("ava@example.com")).toHaveCount(0);
    await expect(page.getByText("local-only-pass")).toHaveCount(0);
    await expect(page.getByText("OPENAI_API_KEY")).toHaveCount(0);
    await expect(page.getByText("sk-")).toHaveCount(0);
  });

  test("member chat sends one message through the browser websocket adapter", async ({ page }) => {
    await page.addInitScript(() => {
      class MockChatWebSocket extends EventTarget {
        static CONNECTING = 0;
        static OPEN = 1;
        static CLOSING = 2;
        static CLOSED = 3;

        readyState = MockChatWebSocket.CONNECTING;
        url: string;
        onopen: ((event: Event) => void) | null = null;
        onmessage: ((event: MessageEvent) => void) | null = null;
        onclose: ((event: CloseEvent) => void) | null = null;
        onerror: ((event: Event) => void) | null = null;

        constructor(url: string) {
          super();
          this.url = url;
          setTimeout(() => {
            this.readyState = MockChatWebSocket.OPEN;
            const event = new Event("open");
            this.onopen?.(event);
            this.dispatchEvent(event);
          }, 0);
        }

        send(value: string) {
          const frame = JSON.parse(value);
          if (frame.type === "gateway.identify") {
            this.emitFrame({
              schemaVersion: "conversation.gateway.v1",
              op: "ack",
              type: "identify.ack",
              clientId: frame.clientId,
              durability: "ephemeral",
              scope: "user",
              payload: { actorId: frame.payload.actorId, participantId: frame.payload.participantId },
              occurredAt: "2026-05-12T12:00:01.000Z",
            });
          }
          if (frame.type === "conversation.subscribe") {
            this.emitFrame({
              schemaVersion: "conversation.gateway.v1",
              op: "ack",
              type: "conversation.subscribe.ack",
              clientId: frame.clientId,
              conversationId: frame.conversationId,
              durability: "ephemeral",
              scope: "conversation",
              payload: { conversationId: frame.conversationId },
              occurredAt: "2026-05-12T12:00:02.000Z",
            });
          }
          if (frame.type === "message.submit") {
            this.emitFrame({
              schemaVersion: "conversation.gateway.v1",
              op: "ack",
              type: "message.submit.ack",
              clientId: frame.clientId,
              conversationId: frame.conversationId,
              durability: "ephemeral",
              scope: "conversation",
              payload: { messageId: "message_browser_ws_1" },
              occurredAt: "2026-05-12T12:00:03.000Z",
            });
            this.emitFrame({
              schemaVersion: "conversation.gateway.v1",
              op: "dispatch",
              type: "message.created",
              serverId: "conversation_member_1:1",
              conversationId: frame.conversationId,
              sequence: 1,
              cursor: 10,
              durability: "durable",
              scope: "conversation",
              payload: {
                messageId: "message_browser_ws_1",
                participantId: frame.payload.participantId,
                clientMessageId: frame.payload.clientMessageId,
              },
              occurredAt: "2026-05-12T12:00:04.000Z",
            });
          }
        }

        close() {
          this.readyState = MockChatWebSocket.CLOSED;
          const event = new CloseEvent("close");
          this.onclose?.(event);
          this.dispatchEvent(event);
        }

        private emitFrame(frame: unknown) {
          setTimeout(() => {
            const event = new MessageEvent("message", { data: JSON.stringify(frame) });
            this.onmessage?.(event);
            this.dispatchEvent(event);
          }, 0);
        }
      }

      window.WebSocket = MockChatWebSocket as unknown as typeof WebSocket;
    });

    await page.route("/api/chat/bootstrap", async (route) => {
      await route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({
          authenticated: true,
          status: "ready",
          degradedReason: null,
          bootstrap: {
            schemaVersion: "ordo.chat-bootstrap.v1",
            actorId: "actor_local_member_mocked",
            conversationId: "conversation_member_1",
            participantId: "participant_member_1",
            assistantParticipantId: "participant_assistant_1",
            transport: {
              route: "/chat/ws",
              protocol: "conversation.gateway.v1",
              url: "ws://127.0.0.1:19080/chat/ws",
            },
          },
        }),
      });
    });

    await page.goto("/login");
    await page.getByLabel("Email").fill("ava@example.com");
    await page.getByLabel("Password").fill("local-only-pass");
    await page.getByRole("button", { name: "Continue" }).click();
    await expect(page).toHaveURL(/\/my\/chat/);
    await page.goto("/my/chat?role=client&mobile=content");

    await expect(page.getByText("Ordo - connected to /chat/ws")).toBeVisible();
    await page.getByRole("textbox", { name: "Message Ordo" }).fill("Please save this test message.");
    await page.getByRole("button", { name: "Send message" }).click();

    const runState = page.getByLabel("Local chat run state");
    await expect(runState.getByText("Please save this test message.")).toBeVisible();
    await expect(runState.getByText("saved by /chat/ws")).toBeVisible();
    await expect(page.getByText("ava@example.com")).toHaveCount(0);
    await expect(page.getByText("local-only-pass")).toHaveCount(0);
    await expect(page.getByText("OPENAI_API_KEY")).toHaveCount(0);
    await expect(page.getByText("prompt")).toHaveCount(0);
  });
});