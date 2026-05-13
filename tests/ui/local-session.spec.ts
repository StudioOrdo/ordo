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
});