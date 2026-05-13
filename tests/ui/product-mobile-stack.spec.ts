import { expect, test } from "@playwright/test";

test.describe("product mobile navigation stack", () => {
  test.beforeEach(async ({ browserName }, testInfo) => {
    test.skip(testInfo.project.name !== "mobile-chromium" || browserName !== "chromium", "mobile-only stack behavior");
  });

  test("member shell advances from rooms to evidence to content with explicit back links", async ({ page }) => {
    await page.goto("/my/offers?role=client");

    const offersRoom = page.locator('.product-nav-drawer .drawer-link[href*="/my/offers"]');
    await expect(offersRoom).toBeVisible();
    await expect(offersRoom).toContainText("Offers");
    await expect(offersRoom).toContainText("trial accepted");
    await expect(page.getByText("Hosted trial accepted")).toBeHidden();
    await expect(page.getByRole("heading", { name: /Hosted 30-day trial accepted/i })).toBeHidden();

    await offersRoom.click();
    await expect(page).toHaveURL(/\/my\/offers\?role=client&mobile=evidence/);
    await expect(page.getByRole("link", { name: /Back to Rooms/i })).toBeVisible();
    await expect(page.getByRole("link", { name: /Open content/i })).toBeVisible();
    await expect(page.getByText("Hosted trial accepted")).toBeVisible();
    await expect(page.getByRole("heading", { name: /Hosted 30-day trial accepted/i })).toBeHidden();
    await expectDocumentLockedToViewport(page);
    await expect(page.locator(".product-section-column")).toHaveJSProperty("scrollTop", 0);

    await page.getByRole("link", { name: /Open content/i }).click();
    await expect(page).toHaveURL(/\/my\/offers\?role=client&mobile=content/);
    await expect(page.getByRole("link", { name: /Back to Evidence/i })).toBeVisible();
    await expect(page.getByRole("heading", { name: /Hosted 30-day trial accepted/i })).toBeVisible();
    await expect(page.getByRole("heading", { name: /Share private trial feedback/i })).toHaveCount(0);
    await expect(page.getByText("Hosted trial accepted")).toBeHidden();
    await expectDocumentLockedToViewport(page);
    await expectColumnOwnsOverflow(page, ".product-main-pane");
  });

  test("single-conversation room opens directly from rooms to content", async ({ page }) => {
    await page.goto("/my/offers?role=client");

    await page.locator('.product-nav-drawer .drawer-link[href*="/my/chat"]').click();
    await expect(page).toHaveURL(/\/my\/chat\?role=client&mobile=content/);
    await expect(page.getByRole("link", { name: /Back to Rooms/i })).toBeVisible();
    await expect(page.getByRole("link", { name: /Back to Evidence/i })).toHaveCount(0);
    await expect(page.getByRole("link", { name: /Open content/i })).toHaveCount(0);
    await expect(page.getByRole("heading", { name: /Talk with Studio Ordo/i })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "Message Ordo" })).toBeVisible();

    await page.getByRole("link", { name: /Back to Rooms/i }).click();
    await expect(page).toHaveURL(/\/my\/chat\?role=client/);
    await expect(page.locator('.product-nav-drawer .drawer-link[href*="/my/chat"]')).toBeVisible();
  });

  test("guest meetup QR chat exposes signup and live handoff actions", async ({ page }) => {
    await page.goto("/chat");

    await expect(page.getByRole("heading", { name: /What should your business do next/i })).toBeVisible();
    await expect(page.getByText("Keith's meetup QR code")).toBeVisible();
    await expect(page.getByRole("button", { name: /Start 30-day trial/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Ask Keith live/i })).toBeVisible();
    await expect(page.getByText("Keith handoff visible to staff")).toBeVisible();
  });

  test("evidence item selection opens only that item in mobile content", async ({ page }) => {
    await page.goto("/my/offers?role=client");

    await page.locator('.product-nav-drawer .drawer-link[href*="/my/offers"]').click();
    await expect(page).toHaveURL(/\/my\/offers\?role=client&mobile=evidence/);

    await page.getByRole("link", { name: /Strategic consultation remains available/i }).click();
    await expect(page).toHaveURL(/\/my\/offers\?role=client&item=\d+&mobile=content/);
    await expect(page.getByRole("heading", { name: /Strategic consultation remains available/i })).toBeVisible();
    await expect(page.getByRole("heading", { name: /Hosted 30-day trial accepted/i })).toHaveCount(0);
    await expect(page.getByRole("heading", { name: /Share private trial feedback/i })).toHaveCount(0);
    await expectDocumentLockedToViewport(page);
    await expectColumnOwnsOverflow(page, ".product-main-pane");
  });
});

async function expectDocumentLockedToViewport(page: import("@playwright/test").Page) {
  const metrics = await page.evaluate(() => ({
    innerHeight: window.innerHeight,
    documentScrollHeight: document.documentElement.scrollHeight,
    bodyScrollHeight: document.body.scrollHeight,
  }));

  expect(metrics.documentScrollHeight).toBe(metrics.innerHeight);
  expect(metrics.bodyScrollHeight).toBe(metrics.innerHeight);
}

async function expectColumnOwnsOverflow(page: import("@playwright/test").Page, selector: string) {
  const metrics = await page.locator(selector).evaluate((element) => ({
    clientHeight: element.clientHeight,
    scrollHeight: element.scrollHeight,
    overflowY: getComputedStyle(element).overflowY,
  }));

  expect(metrics.overflowY).toBe("auto");
  expect(metrics.scrollHeight).toBeGreaterThanOrEqual(metrics.clientHeight);
}
