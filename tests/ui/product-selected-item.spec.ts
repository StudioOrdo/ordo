import { expect, test } from "@playwright/test";

test.describe("product second-column item selection", () => {
  test.beforeEach(async ({ browserName }, testInfo) => {
    test.skip(testInfo.project.name !== "desktop-chromium" || browserName !== "chromium", "desktop selection behavior");
  });

  test("member evidence selection renders one selected item in main content", async ({ page }) => {
    await page.goto("/my/offers?role=client");

    await page.getByRole("link", { name: /Strategic consultation remains available/i }).click();
    await expect(page).toHaveURL(/\/my\/offers\?role=client&item=\d+&mobile=content/);
    await expect(page.locator(".product-section-column").getByText("Strategic consultation remains available")).toBeVisible();
    await expect(page.locator(".member-main-content").getByRole("heading", { name: "Strategic consultation remains available" })).toBeVisible();
    await expect(page.locator(".member-main-content").getByRole("heading", { name: "Share private trial feedback" })).toHaveCount(0);
  });

  test("member drawer exposes only focused member rooms", async ({ page }) => {
    await page.goto("/my/activity?role=client");

    const drawer = page.getByRole("navigation", { name: "Ordo room labels" });
    await expect(drawer.getByRole("link", { name: /Ordo/i })).toBeVisible();
    await expect(drawer.getByRole("link", { name: /Activity/i })).toBeVisible();
    await expect(drawer.getByRole("link", { name: /Offers/i })).toBeVisible();
    await expect(drawer.getByRole("link", { name: /Capabilities/i })).toBeVisible();
    await expect(drawer.getByRole("link", { name: /Requests/i })).toBeVisible();
    await expect(drawer.getByRole("link", { name: /Referrals/i })).toHaveCount(0);
    await expect(drawer.locator(".drawer-link-copy strong")).toHaveText([
      "Ordo",
      "Activity",
      "Offers",
      "Requests",
      "Capabilities",
    ]);
  });

  test("Ordo room renders the primary conversation without a worklist", async ({ page }) => {
    await page.goto("/my/chat?role=client");

    await expect(page.locator(".product-section-column")).toHaveCount(0);
    await expect(page.getByRole("heading", { name: /Talk with Studio Ordo/i })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "Message Ordo" })).toBeVisible();
    await expect(page.getByText("Handoff status")).toBeVisible();
  });

  test("Ordo rail icon toggles the room drawer without moving rail cells", async ({ page }) => {
    await page.goto("/my/activity?role=owner");

    await expect(page.locator(".product-shell")).toHaveAttribute("data-rail-mode", "expanded");
    await expect(page.locator(".rail-collapse-toggle")).toHaveCount(0);
    await expect(page.getByRole("navigation", { name: "Ordo room labels" })).toBeVisible();

    const beforeHome = await centerOf(page.locator(".product-rail-home"));
    const beforeCenters = await shellIconCenters(page);
    const railLink = page.locator('.primary-link[data-shell-id="my-ordo"]');

    await expect(railLink).toHaveAttribute("aria-expanded", "true");
    await railLink.click();

    await expect(page).toHaveURL(/\/my\/activity\?role=owner&rail=collapsed/);
    await expect(page.locator(".product-shell")).toHaveAttribute("data-rail-mode", "collapsed");
    await expect(page.getByRole("navigation", { name: "Ordo room labels" })).toBeHidden();

    const collapsedHome = await centerOf(page.locator(".product-rail-home"));
    const collapsedCenters = await shellIconCenters(page);

    expect(collapsedHome.x).toBeCloseTo(beforeHome.x, 0);
    expect(collapsedHome.y).toBeCloseTo(beforeHome.y, 0);
    expect(collapsedCenters).toEqual(beforeCenters);

    await expect(railLink).toHaveAttribute("aria-expanded", "false");
    await railLink.click();

    await expect(page).toHaveURL(/\/my\/activity\?role=owner$/);
    await expect(page.locator(".product-shell")).toHaveAttribute("data-rail-mode", "expanded");
    await expect(page.getByRole("navigation", { name: "Ordo room labels" })).toBeVisible();
    const afterHome = await centerOf(page.locator(".product-rail-home"));
    const afterCenters = await shellIconCenters(page);

    expect(afterHome.x).toBeCloseTo(beforeHome.x, 0);
    expect(afterHome.y).toBeCloseTo(beforeHome.y, 0);
    expect(afterCenters).toEqual(beforeCenters);
  });

  test("public and member share one fixed Ordo frame logo and top rail", async ({ page }) => {
    await page.goto("/");

    const publicHome = await boxOf(page.locator(".product-rail-home"));
    const publicLogo = await boxOf(page.locator(".product-rail-home img"));
    const publicTop = await boxOf(page.locator(".ordo-frame-top"));
    await expect(page.getByRole("navigation", { name: "Visitor account actions" })).toContainText("Login");
    await expect(page.getByRole("navigation", { name: "Ordo room labels" })).toHaveCount(0);

    await page.goto("/my/activity?role=client&rail=collapsed");

    const memberHome = await boxOf(page.locator(".product-rail-home"));
    const memberLogo = await boxOf(page.locator(".product-rail-home img"));
    const memberTop = await boxOf(page.locator(".ordo-frame-top"));
    await expect(page.locator(".product-shell")).toHaveAttribute("data-rail-mode", "collapsed");
    await expect(page.locator(".rail-collapse-toggle")).toHaveCount(0);

    expect(memberHome).toEqual(publicHome);
    expect(memberLogo).toEqual(publicLogo);
    expect(memberTop.x).toBe(72);
    expect(memberTop.y).toBe(0);
    expect(memberTop.height).toBe(72);
    expect(memberTop).toEqual(publicTop);
  });

  test("focused shell exposes only public home, Ordo workspace, and account utility", async ({ page }) => {
    await page.goto("/my/activity?role=owner");

    await expect(page.locator('.primary-link[data-shell-id="my-ordo"]')).toHaveCount(1);
    await expect(page.locator('.primary-link[data-shell-id="staff"]')).toHaveCount(0);
    await expect(page.locator('.primary-link[data-shell-id="studio"]')).toHaveCount(0);
    await expect(page.locator('.primary-link[data-shell-id="owner"]')).toHaveCount(0);
    await expect(page.locator('.primary-link[data-shell-id="admin"]')).toHaveCount(0);
    await expect(page.locator(".product-rail-home")).toHaveAttribute("href", "/?role=owner");
    await expect(page.locator(".product-rail-user-menu")).toHaveCount(1);
  });

  test("selected detail includes action, timeline, and provenance sections", async ({ page }) => {
    await page.goto("/my/requests?role=client");

    await expect(page.locator(".member-main-content").getByRole("heading", { name: "Share private trial feedback" })).toBeVisible();
    await expect(page.locator(".member-main-content").getByRole("link", { name: "Respond in Ordo" })).toHaveAttribute("href", "/my/chat?notice=private-feedback-request&role=client");
    await expect(page.locator(".member-main-content").getByRole("heading", { name: "Timeline" })).toBeVisible();
    await expect(page.locator(".member-main-content").getByRole("heading", { name: "Evidence" })).toBeVisible();
    await expect(page.locator(".member-main-content").getByRole("heading", { name: "Approve QR card proof" })).toHaveCount(0);
  });

  test("feedback request action starts the Ordo conversation with notice context", async ({ page }) => {
    await page.goto("/my/requests?role=client");

    await expect(page.getByRole("link", { name: "Respond in Ordo: Share private trial feedback" })).toHaveAttribute("href", "/my/chat?notice=private-feedback-request&role=client");
    await page.locator(".member-main-content").getByRole("link", { name: "Respond in Ordo" }).click();
    await expect(page).toHaveURL(/\/my\/chat\?notice=private-feedback-request&role=client/);
    await expect(page.getByRole("heading", { name: /Talk with Studio Ordo/i })).toBeVisible();
  });

  test("member desktop shell is viewport locked and columns own overflow", async ({ page }) => {
    await page.goto("/my/activity?role=client");

    const metrics = await page.evaluate(() => ({
      innerHeight: window.innerHeight,
      documentScrollHeight: document.documentElement.scrollHeight,
      bodyScrollHeight: document.body.scrollHeight,
    }));

    expect(metrics.documentScrollHeight).toBe(metrics.innerHeight);
    expect(metrics.bodyScrollHeight).toBe(metrics.innerHeight);
    await expectColumnOwnsOverflow(page, ".product-nav-drawer");
    await expectColumnOwnsOverflow(page, ".product-section-column");
    await expectColumnOwnsOverflow(page, ".product-main-pane");
  });
});

async function shellIconCenters(page: import("@playwright/test").Page) {
  return page.locator(".product-shell-menu .primary-link-symbol").evaluateAll((elements) =>
    elements.map((element) => {
      const box = element.getBoundingClientRect();
      return {
        x: Math.round(box.left + box.width / 2),
        y: Math.round(box.top + box.height / 2),
      };
    }),
  );
}

async function centerOf(locator: import("@playwright/test").Locator) {
  const box = await locator.boundingBox();
  expect(box).not.toBeNull();
  return {
    x: Math.round((box?.x ?? 0) + (box?.width ?? 0) / 2),
    y: Math.round((box?.y ?? 0) + (box?.height ?? 0) / 2),
  };
}

async function boxOf(locator: import("@playwright/test").Locator) {
  const box = await locator.boundingBox();
  expect(box).not.toBeNull();
  return {
    x: Math.round(box?.x ?? 0),
    y: Math.round(box?.y ?? 0),
    width: Math.round(box?.width ?? 0),
    height: Math.round(box?.height ?? 0),
  };
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
