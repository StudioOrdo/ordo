import { expect, test } from "@playwright/test";

test.describe("real daemon deterministic member chat", () => {
  test.skip(process.env.ORDO_REAL_DAEMON_SMOKE !== "1", "real daemon smoke requires ORDO_REAL_DAEMON_SMOKE=1 and a daemon-backed session");

  test("member chat reaches deterministic assistant reply through real daemon websocket", async ({ page }) => {
    await page.goto("/register");
    const localSessionPayload = await page.evaluate(async () => {
      const response = await fetch("/api/local-session/register", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          name: "Ava Real Smoke",
          email: "ava-real-smoke@example.com",
          password: "local-only-pass",
        }),
      });
      return response.json();
    });
    expect(localSessionPayload, "local session route should persist to the real daemon").toMatchObject({
      persistence: { source: "daemon" },
      redirectTo: "/my/chat?role=client",
    });

    const bootstrapResponsePromise = page.waitForResponse("**/api/chat/bootstrap");
    await page.goto("/my/chat?role=client&mobile=content");
    const bootstrapPayload = await (await bootstrapResponsePromise).text();
    await expect(page.getByText("Ordo - connected to /chat/ws")).toBeVisible({ timeout: 15_000 });
    expect(bootstrapPayload, "chat bootstrap route should be ready from the real daemon").toContain(
      '"status":"ready"',
    );

    await page.getByRole("textbox", { name: "Message Ordo" }).fill("Can you answer deterministically from the local daemon?");
    await page.getByRole("button", { name: "Send message" }).click();

    const runState = page.getByLabel("Local chat run state");
    await expect(runState.getByText("Can you answer deterministically from the local daemon?")).toBeVisible();
    await expect(runState.getByText("saved by /chat/ws")).toBeVisible({ timeout: 15_000 });
    await expect(runState.getByText("Drafting answer")).toBeVisible({ timeout: 15_000 });
    await expect(runState.getByText("deterministic reply saved")).toBeVisible({ timeout: 15_000 });

    await expect(page.getByText("ava-real-smoke@example.com")).toHaveCount(0);
    await expect(page.getByText("local-only-pass")).toHaveCount(0);
    await expect(page.getByText("OPENAI_API_KEY")).toHaveCount(0);
    await expect(page.getByText("sk-")).toHaveCount(0);
    await expect(page.getByText("Client asked for a local deterministic reply.")).toHaveCount(0);
    await expect(page.getByText("prompt")).toHaveCount(0);
  });
});
