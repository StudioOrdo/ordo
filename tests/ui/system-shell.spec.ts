import { expect, test } from "@playwright/test";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";

const daemonPort = 19080;

interface MockDaemonState {
  backupCreated: boolean;
  reportCreated: boolean;
  requests: string[];
}

function productContentUrl(path: string, testInfo: { project: { name: string } }): string {
  return testInfo.project.name === "mobile-chromium" ? `${path}${path.includes("?") ? "&" : "?"}mobile=content` : path;
}

function productEvidenceUrl(path: string, testInfo: { project: { name: string } }): string {
  return testInfo.project.name === "mobile-chromium" ? `${path}${path.includes("?") ? "&" : "?"}mobile=evidence` : path;
}

test.describe.configure({ mode: "serial" });

test.afterEach(async ({ page }) => {
  await page.close();
});

test("System Brief renders daemon evidence and process provenance", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto(productContentUrl("/admin/system?role=owner", testInfo));

    await expect(page.getByRole("heading", { name: "System Brief" })).toBeVisible();
    await expect(page.locator("main")).toContainText("Mocked system brief is available for browser smoke coverage.");
    await expect(page.getByRole("heading", { name: "Evidence" })).toBeVisible();
    await expect(page.locator("main")).toContainText("Health ok");
    await expect(page.getByRole("heading", { name: "Provenance" })).toBeVisible();
    await expect(page.locator("main")).toContainText("brief.generate");
    await expect(page.locator("main")).toContainText("job_smoke_brief");
  } finally {
    await daemon.close();
  }
});

test("Hosted Trials Systems room renders durable capacity and reset evidence", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto(productContentUrl("/admin/hosted-trials?role=owner", testInfo));

    await expect(page.locator("main").getByRole("heading", { name: "Hosted Trials" })).toBeVisible();
    await expect(page.locator("main")).toContainText("1 / 10 active");
    await expect(page.locator("main")).toContainText("1 waiting");
    await expect(page.locator("main")).toContainText("trial_smoke_active");
    await expect(page.locator("main")).toContainText("trial_smoke_reset_ready");
    await expect(page.locator("main")).toContainText("backup_smoke_1");
    await expect(page.locator("main")).toContainText("ready_for_owner_review");
    await expect(page.locator("main")).toContainText("converted_no_wipe");
    await expect(page.locator("main")).toContainText("destructive wipe unavailable");
    expect(daemon.state.requests).toContain("GET /hosted-trials/capacity");
    expect(daemon.state.requests).toContain("GET /backups");
  } finally {
    await daemon.close();
  }
});

test("Hosted Trials Systems room refuses member role before daemon read", async ({ page }) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto("/admin/hosted-trials?role=member");

    await expect(page.locator("body")).not.toContainText("trial_smoke_active");
    await expect(page.locator("body")).not.toContainText("backup_smoke_1");
    expect(daemon.state.requests).not.toContain("GET /hosted-trials/capacity");
  } finally {
    await daemon.close();
  }
});

test("Owner Offer Builder renders durable offers and explicit deferrals", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto(productContentUrl("/owner/offers?role=owner", testInfo));

    await expect(page.locator("main").getByRole("heading", { name: "Offer Builder" })).toBeVisible();
    await expect(page.locator("main")).toContainText("OrdoStudio NYC Pilot");
    await expect(page.locator("main")).toContainText("ready");
    await expect(page.locator("main")).toContainText("30 days");
    await expect(page.locator("main")).toContainText("Accepted-offer Access grant");
    await expect(page.locator("main")).toContainText("Hosted trial capacity");
    await expect(page.locator("main")).toContainText("Tracked QR entry point");
    await expect(page.locator("main")).toContainText("Feedback/referral rewards");
    await expect(page.locator("main")).toContainText("#248");
    await expect(page.locator("main")).toContainText("Product/workforce packs");
    await expect(page.locator("main")).not.toContainText("sk_live");
    await expect(page.locator("main")).not.toContainText("rawPrompt");
    expect(daemon.state.requests).toContain("GET /offer-builder");
  } finally {
    await daemon.close();
  }
});

test("Owner Offer Builder refuses member role before daemon read", async ({ page }) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto("/owner/offers?role=member");

    await expect(page.locator("body")).not.toContainText("OrdoStudio NYC Pilot");
    expect(daemon.state.requests).not.toContain("GET /offer-builder");
  } finally {
    await daemon.close();
  }
});

test("Owner Growth report renders daemon-backed pilot evidence", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon();
  try {
    for (const role of ["owner", "admin"] as const) {
      await page.goto(productContentUrl(`/owner/reports?role=${role}`, testInfo));

      await expect(page.locator("main").getByRole("heading", { name: "Growth Pilot Report" })).toBeVisible();
      await expect(page.locator("main").getByRole("heading", { name: "Owner Review Brief" })).toBeVisible();
      await expect(page.locator("main")).toContainText("7 / 7 pilot loop checkpoint(s) have local report sections.");
      await expect(page.locator("main")).toContainText("Tracked Entry And Sessions");
      await expect(page.locator("main")).toContainText("Offers And Acceptances");
      await expect(page.locator("main")).toContainText("Hosted Trials, Capacity, Backup, And Reset");
      await expect(page.locator("main")).toContainText("Support Handoffs And Strategy Sessions");
      await expect(page.locator("main")).toContainText("Feedback Requests And Review");
      await expect(page.locator("main")).toContainText("Rewards, Ledger, Benefits, And Balances");
      await expect(page.locator("main")).toContainText("Studio Promo Packages And Publication Evidence");
      await expect(page.locator("main")).toContainText("Local report package export unavailable");
      await expect(page.locator("main")).toContainText("Deterministic report-package export is not implemented");
      await expect(page.locator("main")).toContainText("External publishing is deferred");
      await expect(page.locator("main")).toContainText("Platform analytics are missing");
      const visitorEvidence = page.locator("details.evidence-drilldown", { hasText: "Visitor session visitor_smoke_1" }).first();
      await expect(visitorEvidence).toBeVisible();
      await visitorEvidence.locator("summary").click();
      await expect(visitorEvidence).toContainText("ordo://visitor_session/visitor_smoke_1");
      await expect(page.locator("main")).toContainText("deferred");
      await expect(page.locator("main")).toContainText("missing");
      await expect(page.locator("main")).not.toContainText("sk_live");
      await expect(page.locator("main")).not.toContainText("rawPrompt");
    }
    expect(daemon.state.requests.filter((request) => request === "GET /growth/pilot-report")).toHaveLength(2);
    expect(daemon.state.requests).not.toContain("GET /reports/issues");
  } finally {
    await daemon.close();
  }
});

test("Owner Growth report refuses non-owner roles before daemon read", async ({ page }) => {
  const daemon = await startMockDaemon();
  try {
    for (const role of ["anonymous", "member", "staff", "studio"] as const) {
      await page.goto(role === "anonymous" ? "/owner/reports" : `/owner/reports?role=${role}`);

      await expect(page.locator("body")).not.toContainText("Growth Pilot Report");
      await expect(page.locator("body")).not.toContainText("visitor_smoke_1");
    }
    expect(daemon.state.requests).not.toContain("GET /growth/pilot-report");
  } finally {
    await daemon.close();
  }
});

test("Owner Growth report shows daemon-degraded fallback state", async ({ page }, testInfo) => {
  await page.goto(productContentUrl("/owner/reports?role=owner", testInfo));

  await expect(page.locator("main").getByRole("heading", { name: "Growth Pilot Report" })).toBeVisible();
  await expect(page.locator("main")).toContainText("degraded");
  await expect(page.locator("main")).toContainText("Growth report is degraded because the daemon snapshot is unavailable.");
  await expect(page.locator("main")).toContainText("/growth/pilot-report");
});

test("Studio shell renders durable runs and artifacts from surface work items", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto(productContentUrl("/studio?role=studio", testInfo));

    await expect(page.locator("main").getByRole("heading", { name: "Studio Production" })).toBeVisible();
    await expect(page.locator("main")).toContainText("Job: studio.video.make");
    await expect(page.locator("main")).toContainText("Candidate 30 second promo video");
    await expect(page.locator("main")).toContainText("job:job_smoke_video");
    await expect(page.locator("main")).toContainText("artifact:artifact_promo_smoke");
    await expect(page.locator("main")).toContainText("Inspect job");
    await expect(page.locator("main")).toContainText("Review artifact");
    await expect(page.locator("main")).toContainText("Generate media unavailable");
    await expect(page.locator("main")).toContainText("External publishing unavailable");
    await expect(page.locator("main")).not.toContainText("rawPrompt");
    await expect(page.locator("main")).not.toContainText("sk_live_hidden");
    expect(daemon.state.requests).toContain("GET /surface/work-items?viewer=staff&surfaceKind=studio&limit=100");
  } finally {
    await daemon.close();
  }
});

test("Studio artifacts room renders artifact review state without publishing claims", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto(productContentUrl("/studio/artifacts?role=studio", testInfo));

    await expect(page.locator("main").getByRole("heading", { level: 2, name: "Artifacts" })).toBeVisible();
    await expect(page.locator("main")).toContainText("Candidate 30 second promo video");
    await expect(page.locator("main")).toContainText("candidate");
    await expect(page.locator("main")).toContainText("Manual publication package");
    await expect(page.locator("main")).toContainText("staged");
    await expect(page.locator("main")).toContainText("Request revision unavailable");
    await expect(page.locator("main")).toContainText("External publishing unavailable");
    await expect(page.locator("main")).toContainText("Artifact Patch Review");
    await expect(page.locator("main")).toContainText("Landing page copy update");
    await expect(page.locator("main")).toContainText("Preview truncated");
    await expect(page.locator("main")).toContainText("Reject/defer unavailable");
    await expect(page.locator("main")).toContainText("Accept patch");
    await expect(page.locator("main")).not.toContainText("YouTube analytics");
    await expect(page.locator("main")).not.toContainText("rawPrompt");
    await expect(page.locator("main")).not.toContainText("sk_live_hidden");
    expect(daemon.state.requests).toContain("GET /surface/work-items?viewer=staff&surfaceKind=studio&roomKind=artifacts&limit=100");
    expect(daemon.state.requests).toContain("GET /studio/artifact-patches?reviewState=proposed&limit=50");
  } finally {
    await daemon.close();
  }
});

test("Studio artifact patch accept proxy refuses missing Studio role before daemon mutation", async ({ page }) => {
  const daemon = await startMockDaemon();
  try {
    const response = await page.request.post("/api/studio/artifact-patches/patch_copy_1/accept", {
      data: { currentText: "old claim" },
    });

    expect(response.status()).toBe(403);
    expect(daemon.state.requests.some((request) => request.includes("/studio/artifact-patches/patch_copy_1/accept"))).toBe(false);
  } finally {
    await daemon.close();
  }
});

test("Studio shell refuses member role before daemon read", async ({ page }) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto("/studio/factory-jobs?role=member");

    await expect(page.locator("body")).not.toContainText("Job: studio.video.make");
    expect(daemon.state.requests.some((request) => request.includes("/surface/work-items"))).toBe(false);
  } finally {
    await daemon.close();
  }
});

test("Studio shell shows daemon-degraded fallback state", async ({ page }, testInfo) => {
  await page.goto(productContentUrl("/studio?role=studio", testInfo));

  await expect(page.locator("main").getByRole("heading", { name: "Studio Production" })).toBeVisible();
  await expect(page.locator("main")).toContainText("degraded");
  await expect(page.locator("main")).toContainText("Studio snapshot is degraded because the daemon work-item read model is unavailable.");
  await expect(page.locator("main")).toContainText("/surface/work-items?viewer=staff&surfaceKind=studio&limit=100");
});

test("System shell shows daemon-degraded fallback state", async ({ page }, testInfo) => {
  await page.goto(productContentUrl("/admin/system?role=owner", testInfo));

  await expect(page.getByRole("heading", { name: "System Brief" })).toBeVisible();
  await expect(page.locator("main")).toContainText("The daemon is not reachable");
  await expect(page.locator("main")).toContainText("degraded");

  await page.goto("/health");
  await expect(page.getByRole("heading", { name: "Health", exact: true })).toBeVisible();
  await expect(page.locator("main")).toContainText("health unavailable");
  await expect(page.locator("main")).toContainText("ready unavailable");
});

test("Root renders the public Ordo/story surface deck instead of the System Brief", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("heading", { name: /A business appliance/ })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Public navigation" })).toContainText("Home");
  await expect(page.getByRole("navigation", { name: "Public navigation" })).toContainText("Ordo");
  await expect(page.getByRole("navigation", { name: "Visitor account actions" })).toContainText("Login");
  await expect(page.getByRole("navigation", { name: "Visitor account actions" })).toContainText("Register");
  await expect(page.getByLabel("Studio Ordo surface deck")).toContainText("Try OrdoStudio for 30 days");
  await expect(page.getByRole("navigation", { name: "Surface progress" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Open full-screen Ordo" })).toBeVisible();
  await page.goto("/?home=chat&role=admin");
  await expect(page.getByRole("heading", { name: /What should your business do next/ })).toBeVisible();
  await expect(page.getByLabel("Current shell")).toContainText("Site");
  await expect(page.getByLabel("Open user menu")).toHaveCount(0);
  await expect(page.getByRole("navigation", { name: "Member actions" })).toContainText("Open Ordo");
  await expect(page.locator("main")).not.toContainText("The daemon is not reachable");
});

test("Login and register keep the public top rail and enter the authenticated shell", async ({ page }) => {
  await page.goto("/login");

  await expect(page.getByRole("navigation", { name: "Public navigation" })).toContainText("Studio Ordo");
  await expect(page.getByLabel("Studio Ordo home").locator("img")).toHaveAttribute("src", "/logo.png");
  await expect(page.getByRole("navigation", { name: "Visitor account actions" })).toContainText("Register");
  await page.getByRole("link", { name: "Continue" }).click();
  await expect(page.getByRole("navigation", { name: "Ordo room labels" })).toBeVisible();

  await page.goto("/register");
  await expect(page.getByRole("navigation", { name: "Public navigation" })).toContainText("Studio Ordo");
  await page.getByRole("link", { name: "Create account" }).click();
  await expect(page.getByRole("navigation", { name: "Ordo room labels" })).toBeVisible();
});

test("Backup And Restore renders persisted jobs and operator controls", async ({ page }) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto("/backup-restore");

    await expect(page.getByRole("heading", { name: "Backup & Restore" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Create Backup" })).toBeEnabled();
    await expect(page.getByLabel("Backup ID")).toHaveValue("backup_smoke_1");
    await expect(page.getByLabel("Confirmation")).toHaveValue("RESTORE backup_smoke_1");
    await expect(page.getByRole("button", { name: "Validate Restore" })).toBeEnabled();
    await expect(page.locator("main")).toContainText("backup_smoke_1");
    await expect(page.locator("main")).toContainText("/app/.data/backups/backup_smoke_1/manifest.json");
  } finally {
    await daemon.close();
  }
});

test("Admin backup room keeps backup and restore controls in Systems", async ({ page }, testInfo) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto(productContentUrl("/admin/backup?role=owner", testInfo));

    await expect(page.locator("main").getByRole("heading", { name: "Backup & Restore" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Create Backup" })).toBeEnabled();
    await expect(page.getByLabel("Backup ID")).toHaveValue("backup_smoke_1");
    await expect(page.getByRole("button", { name: "Validate Restore" })).toBeEnabled();
    await expect(page.locator("main")).toContainText("backup_smoke_1");
    expect(daemon.state.requests).toContain("GET /backups");
  } finally {
    await daemon.close();
  }
});

test("Backup creation can be triggered through the browser path", async ({ page }) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto("/backup-restore");
    await page.getByRole("button", { name: "Create Backup" }).click();

    await expect(page.getByLabel("Backup ID")).toHaveValue("backup_created_by_smoke");
    await expect(page.locator("main")).toContainText("backup_created_by_smoke");
    expect(daemon.state.requests).toContain("POST /backups/create");
  } finally {
    await daemon.close();
  }
});

test("Logs renders structured diagnostic observations", async ({ page }) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto("/logs");

    await expect(page.getByRole("heading", { name: "Logs" })).toBeVisible();
    await expect(page.locator("main")).toContainText("Backup creation completed.");
    await expect(page.locator("main")).toContainText("job_backup_smoke_1");
    await expect(page.locator("main")).toContainText("backup.create");
  } finally {
    await daemon.close();
  }
});

test("Reports can prepare and display local evidence packages", async ({ page }) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto("/reports");

    await expect(page.getByRole("heading", { name: "Reports" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Prepare Report" })).toBeDisabled();
    await page.getByLabel("Description").fill("The backup screen displayed stale job evidence.");
    await page.getByLabel("Expected behavior").fill("The report should include logs and jobs.");
    await page.getByLabel("Actual behavior").fill("The operator saw stale data.");
    await page.getByRole("button", { name: "Prepare Report" }).click();

    await expect(page.locator("main")).toContainText("report_created_by_smoke");
    await expect(page.locator("main")).toContainText("diagnostic_logs");
    await expect(page.locator("main")).toContainText("Recent structured logs are attached.");
    expect(daemon.state.requests).toContain("POST /reports/issues/prepare");
  } finally {
    await daemon.close();
  }
});

test("Client chat keeps one relationship conversation and hides staff rails", async ({ page }, testInfo) => {
  const isMobile = testInfo.project.name === "mobile-chromium";
  await page.goto(productContentUrl("/chat?role=client", testInfo));

  if (!isMobile) {
    await expect(page.getByRole("navigation", { name: "Functional workspaces" })).toBeVisible();
    await expect(page.locator('.primary-link[data-shell-id="my-ordo"]')).toHaveCount(1);
    await expect(page.locator('.primary-link[data-shell-id="staff"]')).toHaveCount(0);
  }
  await expect(page.getByRole("heading", { name: "Your conversation with Studio Ordo" })).toBeVisible();
  await expect(page.locator("main")).toContainText("single relationship conversation");
  await expect(page.getByLabel("Deliverable cards")).toContainText("Deliverable: QR card proof");
  await expect(page.getByLabel("Deliverable cards")).not.toContainText("Producing job");
  await expect(page.getByLabel("Conversation timeline")).toContainText("Are metal QR cards included");
  await page.getByLabel("Write a reply").fill("Please send me the digital proof.");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByLabel("Conversation timeline")).toContainText("Please send me the digital proof.");
  await expect(page.getByLabel("Conversation timeline")).toContainText(/Ack message_submit_/);
  await expect(page.getByRole("navigation", { name: "Staff rooms" })).toHaveCount(0);
  await expect(page.getByRole("navigation", { name: "System View rooms" })).toHaveCount(0);
  await expect(page.locator("body")).not.toContainText("Logs");
  await expect(page.locator("body")).not.toContainText("Backup");
});

test("User messages collapse the secondary menu into one primary conversation", async ({ page }, testInfo) => {
  const isMobile = testInfo.project.name === "mobile-chromium";
  await page.goto(productContentUrl("/my/chat?role=client", testInfo));

  if (!isMobile) {
    await expect(page.getByRole("navigation", { name: "Ordo room labels" })).toBeVisible();
  }
  await expect(page.getByRole("complementary", { name: "Ordo evidence and assets" })).toHaveCount(0);
  await expect(page.getByLabel("Studio Ordo conversation")).toBeVisible();
  await expect(page.getByLabel("Ordo relationship brief")).toContainText("One relationship conversation");
  await expect(page.getByLabel("Safe handoff status")).toContainText("Keith handoff remains available");
  await expect(page.getByLabel("Safe handoff status")).toContainText("internal routing and provider details stay hidden");
  await expect(page.getByRole("region", { name: "Message Ordo" })).toBeVisible();
  await expect(page.locator("body")).not.toContainText("Staff-only notes");
});

test("User rooms use the second column for evidence and assets", async ({ page }, testInfo) => {
  const isMobile = testInfo.project.name === "mobile-chromium";
  await page.goto(productEvidenceUrl("/my/capabilities?role=client", testInfo));

  if (!isMobile) {
    const roomLabels = page.getByRole("navigation", { name: "Ordo room labels" });
    await expect(roomLabels).toBeVisible();
    await expect(roomLabels.getByRole("link", { name: /Capabilities/ })).toHaveAttribute("aria-current", "page");
  }
  await expect(page.getByRole("complementary", { name: "Ordo evidence and assets" })).toBeVisible();
  await expect(page.getByLabel("Capabilities worklist")).toContainText("Hosted 30-day trial is active");

  if (isMobile) {
    await page.getByRole("link", { name: /Open content/i }).click();
  }

  const capabilityContent = page.getByLabel("Hosted 30-day trial is active detail");
  await expect(capabilityContent).toContainText("Hosted 30-day trial is active");
  await expect(capabilityContent).toContainText("Why it matters");
  await expect(capabilityContent).toContainText("Timeline");
  await expect(capabilityContent).toContainText("Evidence");
  await expect(capabilityContent).toContainText("Hosted trial");
});

test("Product shell uses the Ordo rail icon to toggle the room drawer", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name === "mobile-chromium", "mobile uses the pane stack instead of desktop rail");
  await page.goto("/my/offers?role=client&rail=collapsed");

  await expect(page.locator(".product-shell")).toHaveAttribute("data-rail-mode", "collapsed");
  await expect(page.getByRole("navigation", { name: "Functional workspaces" })).toBeVisible();
  await expect(page.locator(".rail-collapse-toggle")).toHaveCount(0);
  await expect(page.locator(".product-rail-home").locator("img")).toHaveAttribute("src", "/logo.png");
  const userNav = page.getByRole("navigation", { name: "Ordo room labels" });
  await expect(userNav).toBeHidden();
  const ordoRailLink = page.locator('.product-shell-menu [data-shell-id="my-ordo"]');
  await expect(ordoRailLink).toHaveAttribute("aria-expanded", "false");
  await expect(ordoRailLink).toHaveAttribute("href", "/my/offers?role=client");
  await expect(page.getByRole("complementary", { name: "Ordo evidence and assets" })).toBeVisible();
  await expect(page).toHaveURL(/\/my\/offers\?role=client&rail=collapsed$/);

  await ordoRailLink.click();
  await expect(page.locator(".product-shell")).toHaveAttribute("data-rail-mode", "expanded");
  await expect(userNav).toBeVisible();
  await expect(userNav.getByRole("link", { name: /Offers/ })).toHaveAttribute("aria-current", "page");
  await expect(page).toHaveURL(/\/my\/offers\?role=client$/);
});

test("Staff navigation defaults to handoff work before relationship memory", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name === "mobile-chromium", "desktop staff shell composition is covered separately from mobile pane stack");
  await page.goto("/staff/conversations?role=staff");

  const staffNav = page.getByRole("navigation", { name: "Support room labels" });
  await expect(staffNav).toBeVisible();
  await expect(staffNav.getByRole("link", { name: /Conversations/ })).toBeVisible();
  await expect(staffNav.getByRole("link", { name: /Handoffs/ })).toBeVisible();
  await expect(staffNav.getByRole("link", { name: /Conversations/ })).toHaveAttribute("aria-current", "page");
  await expect(page.getByRole("navigation", { name: "System room labels" })).toHaveCount(0);
  await expect(page.locator('.primary-link[data-shell-id="staff"]')).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Conversations", exact: true })).toBeVisible();
  await expect(page.locator("main")).toContainText("Maya asked to talk to Keith live");
  await expect(page.locator("main")).toContainText("Take over");
});

test("Premium conversation UI supports edit, undo, retry, unread, reactions, and presence", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name === "mobile-chromium", "staff conversation chrome is desktop-focused in the shell prototype");
  await page.goto(productContentUrl("/chat?role=staff", testInfo));

  await expect(page.getByLabel("Conversation workspace")).toBeVisible();
  await expect(page.getByRole("button", { name: "Jump to first unread" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Jump to latest" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Simulate offline" })).toBeVisible();
  await expect(page.getByLabel("Narrative brief")).toContainText("What is happening");
  await expect(page.getByLabel("Artifact cards")).toContainText("Artifact: Starter QR card proof");
  await expect(page.getByLabel("Artifact cards")).toContainText("Producing job");
  await expect(page.getByLabel("Artifact cards")).toContainText("Storage health");
  await expect(page.getByLabel("Ethical persuasion guidance")).toContainText("ethical_business_persuasion");
  await expect(page.getByLabel("Ethical persuasion guidance")).toContainText("offer_view_starter_3");
  await expect(page.getByLabel("Ethical persuasion guidance")).toContainText("artifact_qr_card_1");
  await expect(page.getByLabel("Ethical persuasion guidance")).toContainText("You can review the digital proof first");
  await expect(page.getByLabel("Conversation timeline")).toContainText("Ava is typing");
  await expect(page.locator(".unread-divider")).toBeVisible();

  await page.getByRole("button", { name: "Edit" }).first().click();
  await page.getByLabel("Edit message").fill("Metal cards are a separate add-on. I can send the first digital proof today.");
  await page.getByRole("button", { name: "Save edit" }).click();
  await expect(page.getByLabel("Conversation timeline")).toContainText("Edited");
  await expect(page.getByLabel("Conversation timeline")).toContainText("first digital proof today");

  await page.getByRole("button", { name: "Ack" }).first().click();
  await expect(page.getByRole("button", { name: /Ack 1/ }).first()).toBeVisible();

  await page.getByRole("button", { name: "Mark unread" }).first().click();
  await expect(page.locator(".unread-divider")).toBeVisible();

  await page.getByRole("button", { name: "Undo" }).first().click();
  await expect(page.getByLabel("Conversation timeline")).toContainText("Message undone");

  await page.getByLabel("Write a reply").fill("fail this command");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.locator(".gateway-error")).toContainText("Gateway rejected command");
  await page.getByRole("button", { name: "Retry" }).click();
  await expect(page.locator(".gateway-error")).toHaveCount(0);
  await expect(page.getByLabel("Conversation timeline")).toContainText("Retried");
});

test("Conversation recovery replays missed state and reconciles pending clientId without duplicates", async ({ page }, testInfo) => {
  await page.goto(productContentUrl("/chat?role=staff", testInfo));

  await page.getByLabel("Write a reply").fill("hold this pending message");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByLabel("Conversation timeline")).toContainText("hold this pending message");
  await expect(page.getByLabel("Conversation timeline")).toContainText("Pending");

  await page.getByRole("button", { name: "Reconnect and replay" }).click();
  await expect(page.getByLabel("Connection status")).toContainText("Recovered");
  await expect(page.getByLabel("Connection status")).toContainText("reconciled by clientId");
  await expect(page.getByLabel("Conversation timeline")).toContainText("Replayed");
  await expect(page.getByText("hold this pending message")).toHaveCount(1);
});

test("Conversation recovery can replay missed durable state after offline gap", async ({ page }, testInfo) => {
  await page.goto(productContentUrl("/chat?role=client", testInfo));

  await page.getByRole("button", { name: "Simulate offline" }).click();
  await expect(page.getByLabel("Connection status")).toContainText("Offline");
  await page.getByRole("button", { name: "Reconnect and replay" }).click();

  await expect(page.getByLabel("Connection status")).toContainText("Recovered");
  await expect(page.getByLabel("Conversation timeline")).toContainText("Recovered missed durable conversation state after reconnect.");
});

test("Conversation core mobile layout avoids horizontal overflow", async ({ page }, testInfo) => {
  await page.goto(productContentUrl("/chat?role=client", testInfo));

  await expect(page.getByLabel("Conversation workspace")).toBeVisible();
  await expect(page.getByLabel("Ethical persuasion guidance")).toHaveCount(0);
  await expect(page.locator("main")).not.toContainText("ethical_business_persuasion");
  await expect(page.locator("main")).not.toContainText("Staff Guidance");
  await expect(page.getByLabel("Conversation composer")).toBeVisible();
  await page.getByLabel("Write a reply").fill("Mobile keyboard safe reply");
  const composerBox = await page.getByLabel("Conversation composer").boundingBox();
  expect(composerBox?.height ?? 0).toBeGreaterThan(80);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(1);
});

test("Conversation reduced motion keeps state legible without smooth scroll dependency", async ({ page }, testInfo) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(productContentUrl("/chat?role=staff", testInfo));

  await page.getByRole("button", { name: "Jump to latest" }).click();
  await expect(page.getByLabel("Conversation timeline")).toContainText("Ava is typing");
  const scrollBehavior = await page.locator(".timeline-panel").evaluate((element) => getComputedStyle(element).scrollBehavior);
  expect(scrollBehavior).toBe("auto");
});

test("System role can access appliance system rail without leaking it to clients", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name === "mobile-chromium", "mobile system route uses the stacked pane contract");
  await page.goto("/admin/system?role=admin");

  const systemNav = page.getByRole("navigation", { name: "System room labels" });
  await expect(systemNav).toBeVisible();
  await expect(systemNav.getByRole("link", { name: /Health/ })).toBeVisible();
  await expect(systemNav.getByRole("link", { name: /Events/ })).toBeVisible();
  await expect(systemNav.getByRole("link", { name: /Hosted Trials/ })).toBeVisible();
  await expect(systemNav.getByRole("link", { name: /Backups/ })).toBeVisible();
  await expect(page.locator('.primary-link[data-shell-id="admin"]')).toHaveCount(0);
  await page.getByLabel("Open account and role menu").click();
  const userMenu = page.locator(".product-rail-menu-panel");
  await expect(userMenu.getByRole("link", { name: "Account", exact: true })).toBeVisible();
  await expect(userMenu.getByRole("link", { name: "Preferences", exact: true })).toBeVisible();
  await expect(userMenu.getByRole("link", { name: "Public home", exact: true })).toBeVisible();
  await expect(userMenu.getByRole("link", { name: "Admin", exact: true })).toHaveCount(0);
});

test("Account tools are role-specific and keep affiliates outside the staff rail", async ({ page }, testInfo) => {
  const isMobile = testInfo.project.name === "mobile-chromium";
  await page.goto(productContentUrl("/account?role=affiliate", testInfo));

  if (!isMobile) {
    await expect(page.getByRole("heading", { name: "Account", exact: true })).toBeVisible();
  }
  await expect(page.locator("body")).toContainText("Identity, access, and security evidence");
  await expect(page.locator("body")).toContainText("Password reset");
  await expect(page.locator("body")).toContainText("User shell");

  if (!isMobile) {
    await page.getByLabel("Open account and role menu").click();
    await expect(page.getByRole("link", { name: "Account", exact: true })).toBeVisible();
    await expect(page.getByRole("link", { name: "Preferences", exact: true })).toBeVisible();
    await expect(page.getByRole("link", { name: "Public home", exact: true })).toBeVisible();
    await expect(page.getByRole("link", { name: "Sign out", exact: true })).toBeVisible();
    await expect(page.locator("body")).not.toContainText("Prototype role");
  }

  await expect(page.getByRole("navigation", { name: "Support room labels" })).toHaveCount(0);
  await expect(page.getByRole("navigation", { name: "System room labels" })).toHaveCount(0);
});

test("Product surfaces load latest completed brief before raw surface detail", async ({ page }, testInfo) => {
  await page.goto(productContentUrl("/offers?role=staff", testInfo));

  const latestBrief = page.getByLabel("Latest completed surface brief");
  await expect(latestBrief).toBeVisible();
  await expect(latestBrief).toContainText("Offer surface brief");
  await expect(latestBrief).toContainText("Refresh running");
  await expect(latestBrief).toContainText("offer_starter");
  await expect(page.getByRole("heading", { name: "Offers", exact: true })).toBeVisible();
  await expect(page.locator("main")).toContainText("Offers describe ways to buy from Studio Ordo.");
});

function startMockDaemon(): Promise<{ close: () => Promise<void>; state: MockDaemonState }> {
  const state: MockDaemonState = { backupCreated: false, reportCreated: false, requests: [] };
  const server = createServer((request, response) => handleRequest(request, response, state));

  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(daemonPort, "127.0.0.1", () => {
      server.off("error", reject);
      resolve({ close: () => closeServer(server), state });
    });
  });
}

function handleRequest(request: IncomingMessage, response: ServerResponse, state: MockDaemonState) {
  const method = request.method ?? "GET";
  const path = request.url ?? "/";
  state.requests.push(`${method} ${path}`);

  if (method === "GET" && path === "/health") {
    return jsonResponse(response, {
      schemaVersion: "1",
      service: "ordo-daemon",
      status: "ok",
      checks: [{ name: "sqlite", status: "ok", detail: "SQLite reachable" }],
    });
  }

  if (method === "GET" && path === "/ready") {
    return jsonResponse(response, {
      schemaVersion: "1",
      service: "ordo-daemon",
      status: "ready",
      checks: [{ name: "schema", status: "ready", detail: "Required tables present" }],
    });
  }

  if (method === "GET" && path === "/briefs/system/latest") {
    return jsonResponse(response, { brief: systemBrief() });
  }

  if (method === "GET" && path === "/backups") {
    return jsonResponse(response, { jobs: backupJobs(state.backupCreated) });
  }

  if (method === "GET" && path === "/hosted-trials/capacity") {
    return jsonResponse(response, hostedTrialCapacity());
  }

  if (method === "GET" && path === "/offer-builder") {
    return jsonResponse(response, offerBuilder());
  }

  if (method === "GET" && path === "/growth/pilot-report") {
    return jsonResponse(response, growthPilotReport());
  }

  if (method === "GET" && path.startsWith("/surface/work-items")) {
    const url = new URL(path, "http://127.0.0.1");
    if (url.searchParams.get("surfaceKind") === "studio") {
      return jsonResponse(response, {
        items: studioSurfaceWorkItems(url.searchParams.get("roomKind")),
      });
    }
  }

  if (method === "GET" && path.startsWith("/studio/artifact-patches")) {
    return jsonResponse(response, { proposals: studioArtifactPatchProposals() });
  }

  if (method === "GET" && path === "/logs?limit=100") {
    return jsonResponse(response, { logs: diagnosticLogs() });
  }

  if (method === "GET" && path === "/reports/issues") {
    return jsonResponse(response, { reports: issueReports(state.reportCreated).map(issueReportSummary) });
  }

  if (method === "GET" && path.startsWith("/reports/issues/")) {
    const reportId = path.split("/").pop() ?? "report_smoke_1";
    return jsonResponse(response, { report: issueReport(reportId, reportId.includes("created") ? "job_report_created" : "job_report_smoke_1") });
  }

  if (method === "POST" && path === "/backups/create") {
    state.backupCreated = true;
    return jsonResponse(response, { job: backupJobs(true)[0] });
  }

  if (method === "POST" && path === "/restore/validate") {
    return jsonResponse(response, { job: restoreJob() });
  }

  if (method === "POST" && path === "/reports/issues/prepare") {
    state.reportCreated = true;
    return jsonResponse(response, { reports: issueReports(true).map(issueReportSummary) });
  }

  response.writeHead(404, { "content-type": "application/json" });
  response.end(JSON.stringify({ error: `Unhandled mock daemon route: ${method} ${path}` }));
}

function diagnosticLogs() {
  return [
    {
      id: "log_smoke_backup",
      timestamp: "2026-05-08T12:00:03.000Z",
      level: "info",
      source: "backup",
      message: "Backup creation completed.",
      requestId: null,
      jobId: "job_backup_smoke_1",
      taskKey: "backup.record",
      capabilityId: "backup.create",
      eventType: "backup.create.completed",
      errorCode: null,
      durationMs: 2000,
      payload: { backupId: "backup_smoke_1" },
    },
    {
      id: "log_smoke_report",
      timestamp: "2026-05-08T12:00:04.000Z",
      level: "warn",
      source: "reports",
      message: "Report source included bounded diagnostics.",
      requestId: null,
      jobId: "job_report_smoke_1",
      taskKey: "diagnostics.collect",
      capabilityId: "issue.report.prepare",
      eventType: "task.succeeded",
      errorCode: null,
      durationMs: null,
      payload: { sources: ["diagnostic_logs"] },
    },
  ];
}

function issueReports(includeCreatedReport: boolean) {
  const reports = [issueReport("report_smoke_1", "job_report_smoke_1")];
  if (includeCreatedReport) {
    reports.unshift(issueReport("report_created_by_smoke", "job_report_created"));
  }
  return reports;
}

function issueReport(id: string, jobId: string) {
  return {
    id,
    jobId,
    status: "ready_for_review",
    severity: "medium",
    title: "Local diagnostic report",
    summary: "5 evidence sources collected for a medium severity local report.",
    description: "The operator prepared a local diagnostic report.",
    sourceRoute: "/backup-restore",
    markdownBody: "# Local diagnostic report\n\n## Diagnostics Summary\n\n- health: Daemon health is ok.\n- readiness: Daemon readiness is ready.\n- recent_events: Recent events are attached.\n- recent_jobs: Recent jobs are attached.\n- diagnostic_logs: Recent structured logs are attached.\n\n## Limitations\n\nExternal submission transports are not implemented.",
    diagnostics: { localOnly: true, externalSubmission: "not_implemented" },
    evidence: [
      evidence("health", "Daemon health is ok."),
      evidence("readiness", "Daemon readiness is ready."),
      evidence("recent_events", "Recent events are attached."),
      evidence("recent_jobs", "Recent jobs are attached."),
      evidence("diagnostic_logs", "Recent structured logs are attached."),
    ],
    redactions: ["Secrets are redacted."],
    createdAt: "2026-05-08T12:00:04.000Z",
    updatedAt: "2026-05-08T12:00:04.000Z",
    exportedAt: null,
    submittedAt: null,
    externalUrl: null,
  };
}

function issueReportSummary(report: ReturnType<typeof issueReport>) {
  return {
    id: report.id,
    jobId: report.jobId,
    status: report.status,
    severity: report.severity,
    title: report.title,
    summary: report.summary,
    sourceRoute: report.sourceRoute,
    createdAt: report.createdAt,
    updatedAt: report.updatedAt,
    exportedAt: report.exportedAt,
    submittedAt: report.submittedAt,
    externalUrl: report.externalUrl,
  };
}

function evidence(source: string, summary: string) {
  return {
    source,
    collectedAt: "2026-05-08T12:00:04.000Z",
    status: "succeeded",
    summary,
    payload: {},
    redactions: [],
    limits: {},
    errors: [],
  };
}

function systemBrief() {
  return {
    id: "brief_smoke_1",
    sectionKey: "system",
    jobId: "job_smoke_brief",
    version: 1,
    title: "System Brief",
    summary: [
      "Mocked system brief is available for browser smoke coverage.",
      "Health ok and readiness ready are backed by daemon evidence.",
    ],
    bodyMarkdown: "The browser smoke fixture proves the System Brief surface can render durable daemon evidence.",
    evidence: [
      { label: "Health", value: "ok", source: "/health" },
      { label: "Readiness", value: "ready", source: "/ready" },
    ],
    limitations: ["Fixture data only."],
    visibility: "internal",
    createdAt: "2026-05-08T12:00:00.000Z",
    validUntil: null,
    process: {
      jobId: "job_smoke_brief",
      templateId: "brief.generate",
      templateVersion: 1,
      origin: "browser-smoke",
      status: "succeeded",
    },
  };
}

function hostedTrialCapacity() {
  return {
    policies: [
      {
        id: "hosted_trial_capacity_policy_smoke",
        offerId: "offer_smoke_1",
        offerSlug: "nyc-pilot",
        status: "active",
        activeSlotLimit: 10,
        activeSlotCount: 1,
        waitlistCount: 1,
        trialDays: 30,
        backupBeforeWipeRequired: true,
        resetGraceDays: 7,
        metadata: { pilot: "nyc" },
        createdAt: "2026-05-08T12:00:00.000Z",
        updatedAt: "2026-05-08T12:00:00.000Z",
      },
    ],
    slots: [
      hostedTrialSlot("hosted_trial_slot_active", "trial_smoke_active", "active", "pending", "pending", {}, {}),
      hostedTrialSlot(
        "hosted_trial_slot_reset_ready",
        "trial_smoke_reset_ready",
        "expired",
        "ready",
        "ready_for_owner_review",
        {
          backupBeforeWipeRequired: true,
          destructiveWipeAllowed: false,
          reason: "backup_ready_owner_review_required",
          requires: ["explicit_destructive_action"],
        },
        {
          actorId: "actor_local_owner",
          reason: "30-day trial expired after backup export.",
          decidedAt: "2026-05-08T12:00:00.000Z",
        },
      ),
      hostedTrialSlot("hosted_trial_slot_converted", "trial_smoke_converted", "converted", "retained", "converted_no_wipe", {}, {}),
    ],
    waitlist: [
      {
        id: "hosted_trial_waitlist_smoke",
        policyId: "hosted_trial_capacity_policy_smoke",
        acceptanceId: "acceptance_waitlist_smoke",
        offerId: "offer_smoke_1",
        offerSlug: "nyc-pilot",
        visitorSessionId: "visitor_smoke_1",
        subjectKind: "visitor_session",
        subjectId: "visitor_smoke_1",
        status: "waiting",
        position: 1,
        reason: "capacity_full",
        receipt: { title: "Waitlisted" },
        evidenceRefs: ["offer_acceptance:acceptance_waitlist_smoke"],
        createdAt: "2026-05-08T12:00:00.000Z",
        updatedAt: "2026-05-08T12:00:00.000Z",
      },
    ],
  };
}

function offerBuilder() {
  return {
    generatedAt: "2026-05-08T12:00:00.000Z",
    offers: [
      {
        offer: {
          id: "offer_smoke_1",
          slug: "nyc-pilot",
          title: "OrdoStudio NYC Pilot",
          summary: "30 days of experimental hosted OrdoStudio access.",
          status: "available",
          visibility: "public",
          publicationState: "published",
          trialDays: 30,
          sourceKind: "offer_builder",
          sourceRef: "nyc-pilot",
          terms: {
            termsVersion: "2026-05-13",
            trialDays: 30,
            experimentalHosting: true,
            backupBeforeWipeRequired: true,
            humanReviewRequired: true,
          },
          metadata: {},
          createdByActorId: "actor_local_owner",
          createdAt: "2026-05-08T12:00:00.000Z",
          updatedAt: "2026-05-08T12:00:00.000Z",
          publishedAt: "2026-05-08T12:00:00.000Z",
          archivedAt: null,
        },
        publicPreview: {
          id: "offer_smoke_1",
          slug: "nyc-pilot",
          title: "OrdoStudio NYC Pilot",
          summary: "30 days of experimental hosted OrdoStudio access.",
          trialDays: 30,
          sourceKind: "offer_builder",
          sourceRef: "nyc-pilot",
          terms: {
            trialDays: 30,
            termsVersion: "2026-05-13",
            experimentalHosting: true,
            backupBeforeWipeRequired: true,
            humanReviewRequired: true,
            rewards: { status: "not_available_yet", blockedBy: "#248" },
            packs: { status: "not_available_yet", blockedBy: "offer_pack_binding" },
          },
        },
        validation: {
          publishable: true,
          state: "ready",
          blockers: [],
          warnings: [],
          supportedReferences: [
            offerBuilderReference("access_grant", "Accepted-offer Access grant", "available", "resource_grants"),
            offerBuilderReference("hosted_trial_capacity", "Hosted trial capacity", "available", "hosted_trial_slots"),
            offerBuilderReference("tracked_entry_point", "Tracked QR entry point", "available", "tracked_entry_point:entry_nyc"),
            offerBuilderReference("support_handoff_cta", "Support handoff CTA", "available", "handoff_inbox_items"),
          ],
          deferredReferences: [
            offerBuilderReference("reward_ledger", "Feedback/referral rewards", "not_available_yet", "", "#248"),
            offerBuilderReference("product_workforce_packs", "Product/workforce packs", "not_available_yet", "", "offer_pack_binding"),
            offerBuilderReference("external_platforms", "External publishing/payments/OAuth", "out_of_scope", "", "future_guarded_adapters"),
          ],
          evidenceRefs: ["offer:offer_smoke_1", "tracked_entry_point:entry_nyc"],
        },
      },
    ],
  };
}

function offerBuilderReference(key: string, label: string, status: string, evidenceRef: string, blockedBy: string | null = null) {
  return {
    key,
    label,
    status,
    detail: `${label} is represented by durable daemon state.`,
    evidenceRefs: evidenceRef ? [evidenceRef] : [],
    blockedBy,
  };
}

function growthPilotReport() {
  return {
    schemaVersion: "ordo.growth_pilot_report.v1",
    generatedAt: "2026-05-13T18:00:00.000Z",
    limitations: [
      growthLimitation(
        "external_publishing_deferred",
        "External publishing is deferred",
        "No TikTok, YouTube, OAuth, or platform publishing API is called by this report.",
        "deferred",
      ),
      growthLimitation(
        "platform_analytics_missing",
        "Platform analytics are missing",
        "No platform reach, watch-time, or conversion metric is reported unless future durable evidence exists.",
        "missing",
      ),
    ],
    sections: [
      growthSection("tracked_entry", "Tracked Entry And Sessions", "measured", [
        growthMetric("visitor_sessions", "Visitor sessions", 3, "sessions", "measured", [growthRef("visitor_session", "visitor_smoke_1", "Visitor session")]),
        growthMetric("active_visitor_sessions", "Active visitor sessions", 1, "sessions", "measured"),
      ], [
        growthItem("visitor_session", "visitor_smoke_1", "Visitor session", "active", "measured", "2026-05-13T17:00:00.000Z"),
      ]),
      growthSection("offers", "Offers And Acceptances", "measured", [
        growthMetric("offers", "Offers", 1, "offers", "measured", [growthRef("offer", "offer_smoke_1", "Offer")]),
        growthMetric("offer_acceptances", "Offer acceptances", 1, "acceptances", "measured", [growthRef("offer_acceptance", "acceptance_smoke_1", "Offer acceptance")]),
        growthMetric("individual_offer_view_events", "Individual offer view events", 0, "views", "missing"),
      ], [
        growthItem("offer_acceptance", "acceptance_smoke_1", "Offer acceptance", "accepted", "measured", "2026-05-13T16:55:00.000Z"),
      ], [
        growthLimitation("offer_view_events_missing", "Individual offer views are not tracked yet", "Per-offer view events are not durable yet.", "missing"),
      ]),
      growthSection("hosted_trials", "Hosted Trials, Capacity, Backup, And Reset", "measured", [
        growthMetric("trials", "Hosted trials", 1, "trials", "measured", [growthRef("trial", "trial_smoke_active", "Hosted trial")]),
        growthMetric("active_hosted_slots", "Active hosted trial slots", 1, "slots", "measured"),
        growthMetric("waitlist_entries", "Hosted trial waitlist entries", 1, "entries", "measured", [growthRef("hosted_trial_waitlist_entry", "waitlist_smoke_1", "Hosted trial waitlist entry")]),
      ], [
        growthItem("trial", "trial_smoke_active", "Hosted trial", "started", "measured", "2026-05-13T16:50:00.000Z"),
      ]),
      growthSection("support_handoffs", "Support Handoffs And Strategy Sessions", "measured", [
        growthMetric("handoff_items", "Support handoff items", 1, "items", "measured", [growthRef("handoff_inbox_item", "handoff_strategy_smoke", "Support handoff")]),
        growthMetric("strategy_session_handoffs", "Strategy session handoffs", 1, "items", "measured"),
      ], [
        growthItem("handoff_inbox_item", "handoff_strategy_smoke", "Support handoff", "queued", "measured", "2026-05-13T16:45:00.000Z"),
      ]),
      growthSection("feedback", "Feedback Requests And Review", "measured", [
        growthMetric("feedback_requests", "Feedback requests", 1, "requests", "measured", [growthRef("feedback_request", "feedback_request_smoke", "Feedback request")]),
        growthMetric("accepted_feedback_reviews", "Accepted feedback reviews", 1, "reviews", "measured"),
      ], [
        growthItem("feedback_request", "feedback_request_smoke", "Feedback request", "reviewed", "measured", "2026-05-13T16:40:00.000Z"),
      ]),
      growthSection("rewards", "Rewards, Ledger, Benefits, And Balances", "measured", [
        growthMetric("reward_events", "Reward events", 1, "events", "measured", [growthRef("reward_event", "reward_event_smoke", "Reward event")]),
        growthMetric("benefit_grants", "Benefit grants", 1, "grants", "measured", [growthRef("benefit_grant", "benefit_grant_smoke", "Benefit grant")]),
        growthMetric("public_leaderboard_rank", "Public leaderboard rank", 0, "ranks", "deferred"),
      ], [
        growthItem("reward_event", "reward_event_smoke", "Reward event", "granted", "measured", "2026-05-13T16:35:00.000Z"),
      ], [
        growthLimitation("leaderboard_deferred", "Leaderboard is deferred", "Reward evidence is available to Growth, but public leaderboards are out of scope.", "deferred"),
      ]),
      growthSection("studio_promos", "Studio Promo Packages And Publication Evidence", "measured", [
        growthMetric("promo_video_packages", "Promo video packages", 1, "packages", "measured", [growthRef("artifact", "artifact_promo_smoke", "Promo package artifact")]),
        growthMetric("staged_manual_packages", "Staged manual promo packages", 1, "packages", "manual"),
        growthMetric("external_publications", "External platform publications", 0, "publications", "deferred"),
        growthMetric("platform_performance_metrics", "Platform performance metrics", 0, "metrics", "missing"),
      ], [
        growthItem("artifact", "artifact_promo_smoke", "Promo package artifact", "staged_manual", "measured", "2026-05-13T16:30:00.000Z"),
      ], [
        growthLimitation("external_publication_deferred", "External publishing is deferred", "The promo workflow stages local artifacts only.", "deferred"),
        growthLimitation("platform_analytics_missing", "Platform analytics are missing", "The report does not claim views, watch time, or conversions.", "missing"),
      ]),
    ],
  };
}

function growthSection(key: string, title: string, sourceStatus: string, metrics: ReturnType<typeof growthMetric>[], recentItems: ReturnType<typeof growthItem>[], limitations: ReturnType<typeof growthLimitation>[] = []) {
  const evidenceRefs = [
    ...metrics.flatMap((metric) => metric.evidenceRefs),
    ...recentItems.flatMap((item) => item.evidenceRefs),
  ].filter((ref, index, refs) => refs.findIndex((candidate) => candidate.uri === ref.uri) === index);
  return { key, title, sourceStatus, metrics, recentItems, evidenceRefs, limitations };
}

function growthMetric(key: string, label: string, value: number, unit: string, sourceStatus: string, evidenceRefs: ReturnType<typeof growthRef>[] = []) {
  return { key, label, value, unit, sourceStatus, evidenceRefs };
}

function growthItem(sourceKind: string, sourceId: string, labelPrefix: string, status: string, sourceStatus: string, occurredAt: string) {
  return {
    sourceKind,
    sourceId,
    label: `${labelPrefix} ${sourceId}`,
    status,
    sourceStatus,
    occurredAt,
    evidenceRefs: [growthRef(sourceKind, sourceId, labelPrefix)],
  };
}

function growthRef(sourceKind: string, sourceId: string, labelPrefix: string) {
  return {
    sourceKind,
    sourceId,
    label: `${labelPrefix} ${sourceId}`,
    uri: `ordo://${sourceKind}/${sourceId}`,
  };
}

function growthLimitation(key: string, label: string, detail: string, sourceStatus: string) {
  return { key, label, detail, sourceStatus };
}

function studioSurfaceWorkItems(roomKind: string | null) {
  const items = [
    studioWorkItem({
      id: "studio_run_video",
      roomKind: "runs",
      sourceKind: "job",
      sourceId: "job_smoke_video",
      objectKind: "job",
      objectId: "job_smoke_video",
      title: "Job: studio.video.make",
      summary: "Job from conversation brief is running with durable progress evidence.",
      status: "running",
      evidenceRefs: ["job:job_smoke_video", "brief:brief_promo"],
      actions: ["inspect_job"],
    }),
    studioWorkItem({
      id: "studio_artifact_promo",
      roomKind: "artifacts",
      sourceKind: "artifact",
      sourceId: "artifact_promo_smoke",
      objectKind: "artifact",
      objectId: "artifact_promo_smoke",
      title: "Candidate 30 second promo video",
      summary: "Candidate package needs review before any staged manual publication.",
      status: "candidate",
      evidenceRefs: ["artifact:artifact_promo_smoke", "job:job_smoke_video"],
      actions: ["review_artifact"],
    }),
    studioWorkItem({
      id: "studio_artifact_staged",
      roomKind: "artifacts",
      sourceKind: "artifact",
      sourceId: "artifact_manual_publication_smoke",
      objectKind: "artifact",
      objectId: "artifact_manual_publication_smoke",
      title: "Manual publication package",
      summary: "Approved metadata is staged for owner download only.",
      status: "staged",
      evidenceRefs: ["artifact:artifact_manual_publication_smoke"],
      actions: ["review_artifact"],
    }),
  ];

  return roomKind ? items.filter((item) => item.roomKind === roomKind) : items;
}

function studioArtifactPatchProposals() {
  return [
    {
      id: "patch_copy_1",
      sourceArtifactId: "artifact_landing_copy",
      sourceArtifactKind: "markdown",
      sourceArtifactTitle: "Landing page copy update",
      sourceArtifactStatus: "candidate",
      sourceArtifactVisibility: "owner",
      sourceVersionId: "artifact_version_landing_1",
      baseHash: "sha256:base",
      proposedHash: "sha256:proposed",
      preview: {
        changed: true,
        addedLines: 2,
        removedLines: 1,
        hunks: 1,
      },
      boundedPatchPreview: "--- base\n+++ proposed\n@@\n-old claim\n+evidence-backed claim",
      previewTruncated: true,
      evidenceRefs: ["artifact:artifact_landing_copy", "job:job_smoke_video"],
      provenance: {
        source: "artifact_patch_proposals",
      },
      reviewState: "proposed",
      acceptedVersionId: null,
      proposedByActorId: "owner:local_owner",
      appliedByActorId: null,
      createdAt: "2026-05-14T10:00:00Z",
      updatedAt: "2026-05-14T10:01:00Z",
      appliedAt: null,
    },
  ];
}

function studioWorkItem({
  id,
  roomKind,
  sourceKind,
  sourceId,
  objectKind,
  objectId,
  title,
  summary,
  status,
  evidenceRefs,
  actions,
}: {
  id: string;
  roomKind: string;
  sourceKind: string;
  sourceId: string;
  objectKind: string;
  objectId: string;
  title: string;
  summary: string;
  status: string;
  evidenceRefs: string[];
  actions: string[];
}) {
  return {
    id,
    surfaceKind: "studio",
    roomKind,
    sourceKind,
    sourceId,
    objectKind,
    objectId,
    title,
    summary,
    status,
    priority: 70,
    actorContext: {
      actorId: "actor_studio_smoke",
      rawPrompt: "rawPrompt should not render",
    },
    connectionContext: {
      providerSecret: "sk_live_hidden",
      staffRouting: "keith_internal",
    },
    evidenceRefs,
    actions,
    visibility: "staff",
    createdAt: "2026-05-08T12:00:00.000Z",
    updatedAt: "2026-05-08T12:00:02.000Z",
    projectedAt: "2026-05-08T12:00:03.000Z",
  };
}

function hostedTrialSlot(
  id: string,
  trialId: string,
  status: string,
  backupStatus: string,
  resetState: string,
  resetGuard: Record<string, unknown>,
  ownerOverride: Record<string, unknown>,
) {
  return {
    id,
    policyId: "hosted_trial_capacity_policy_smoke",
    trialId,
    acceptanceId: `acceptance_${trialId}`,
    offerId: "offer_smoke_1",
    offerSlug: "nyc-pilot",
    subjectKind: "visitor_session",
    subjectId: `visitor_${trialId}`,
    status,
    allocatedAt: "2026-05-01T12:00:00.000Z",
    expiresAt: "2026-05-31T12:00:00.000Z",
    releasedAt: status === "active" ? null : "2026-06-01T12:00:00.000Z",
    releaseReason: status === "active" ? null : status,
    backupRequired: true,
    backupStatus,
    backupEvidenceRefs: resetState === "ready_for_owner_review" ? ["backup:backup_smoke_1"] : [],
    resetEligibleAt: status === "active" ? null : "2026-06-07T12:00:00.000Z",
    resetState,
    resetGuard,
    ownerOverride,
    createdAt: "2026-05-01T12:00:00.000Z",
    updatedAt: "2026-06-01T12:00:00.000Z",
  };
}

function backupJobs(includeCreatedBackup: boolean) {
  const jobs = [backupJob("backup_smoke_1", "job_backup_smoke_1", "succeeded")];
  if (includeCreatedBackup) {
    jobs.unshift(backupJob("backup_created_by_smoke", "job_backup_created", "succeeded"));
  }
  return jobs;
}

function backupJob(backupId: string, jobId: string, status: string) {
  return {
    id: jobId,
    operation: "backup",
    kind: "system",
    status,
    progress: { totalRequiredTasks: 8, completedRequiredTasks: 8, percent: 100 },
    currentTaskKey: null,
    elapsedSeconds: 2,
    startedAt: "2026-05-08T12:00:00.000Z",
    completedAt: "2026-05-08T12:00:02.000Z",
    createdAt: "2026-05-08T12:00:00.000Z",
    updatedAt: "2026-05-08T12:00:02.000Z",
    failureMessage: null,
    artifact: {
      id: `artifact_${backupId}`,
      artifactKind: "backup.archive",
      uri: `/app/.data/backups/${backupId}`,
      label: "Backup archive",
      metadata: {
        backupId,
        manifestPath: `/app/.data/backups/${backupId}/manifest.json`,
        checksumAlgorithm: "sha256",
        checksumAlgorithmVersion: "1",
      },
      createdAt: "2026-05-08T12:00:02.000Z",
    },
    tasks: [],
  };
}

function restoreJob() {
  return {
    ...backupJob("backup_smoke_1", "job_restore_smoke_1", "waiting_for_input"),
    operation: "restore",
    progress: { totalRequiredTasks: 10, completedRequiredTasks: 4, percent: 40 },
    currentTaskKey: "lock.acquire",
  };
}

function jsonResponse(response: ServerResponse, body: unknown) {
  response.writeHead(200, { "content-type": "application/json" });
  response.end(JSON.stringify(body));
}

function closeServer(server: Server): Promise<void> {
  return new Promise((resolve, reject) => {
    server.close((error) => (error ? reject(error) : resolve()));
  });
}
