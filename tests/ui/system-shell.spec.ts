import { expect, test } from "@playwright/test";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";

const daemonPort = 19080;

interface MockDaemonState {
  backupCreated: boolean;
  reportCreated: boolean;
  requests: string[];
}

test.describe.configure({ mode: "serial" });

test.afterEach(async ({ page }) => {
  await page.close();
});

test("System Brief renders daemon evidence and process provenance", async ({ page }) => {
  const daemon = await startMockDaemon();
  try {
    await page.goto("/");

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

test("System shell shows daemon-degraded fallback state", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("heading", { name: "System Brief" })).toBeVisible();
  await expect(page.locator("main")).toContainText("The daemon is not reachable");
  await expect(page.locator("main")).toContainText("degraded");

  await page.goto("/health");
  await expect(page.getByRole("heading", { name: "Health", exact: true })).toBeVisible();
  await expect(page.locator("main")).toContainText("health unavailable");
  await expect(page.locator("main")).toContainText("ready unavailable");
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

test("Client chat keeps one relationship conversation and hides staff rails", async ({ page }) => {
  await page.goto("/chat?role=client");

  await expect(page.getByRole("navigation", { name: "Public and member navigation" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Your conversation with Studio Ordo" })).toBeVisible();
  await expect(page.locator("main")).toContainText("single relationship conversation");
  await expect(page.getByLabel("Deliverable cards")).toContainText("Deliverable: QR card proof");
  await expect(page.getByLabel("Deliverable cards")).not.toContainText("Producing job");
  await expect(page.getByLabel("Conversation timeline")).toContainText("Are metal QR cards included");
  await page.getByLabel("Write a reply").fill("Please send me the digital proof.");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByLabel("Conversation timeline")).toContainText("Please send me the digital proof.");
  await expect(page.getByLabel("Conversation timeline")).toContainText(/Ack message_submit_/);
  await expect(page.getByRole("navigation", { name: "Staff business navigation" })).toHaveCount(0);
  await expect(page.getByRole("navigation", { name: "Admin system navigation" })).toHaveCount(0);
  await expect(page.locator("body")).not.toContainText("Logs");
  await expect(page.locator("body")).not.toContainText("Backup");
});

test("Staff navigation defaults to handoff work before relationship memory", async ({ page }) => {
  await page.goto("/conversations?role=staff");

  const staffNav = page.getByRole("navigation", { name: "Staff business navigation" });
  await expect(staffNav).toBeVisible();
  await expect(staffNav.getByRole("link", { name: /Conversations/ })).toBeVisible();
  await expect(staffNav.getByRole("link", { name: /Connections/ })).toBeVisible();
  await expect(staffNav.getByRole("link", { name: /Conversations/ })).toHaveAttribute("aria-current", "page");
  await expect(page.getByRole("navigation", { name: "Admin system navigation" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Queues" })).toBeVisible();
  await expect(page.locator("main")).toContainText("My Handoffs");
  await expect(page.locator("main")).toContainText("Why this is here");
  await expect(page.locator("main")).toContainText("Handoff Brief");
});

test("Premium conversation UI supports edit, undo, retry, unread, reactions, and presence", async ({ page }) => {
  await page.goto("/conversations?role=staff");

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

test("Conversation recovery replays missed state and reconciles pending clientId without duplicates", async ({ page }) => {
  await page.goto("/conversations?role=staff");

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

test("Conversation recovery can replay missed durable state after offline gap", async ({ page }) => {
  await page.goto("/chat?role=client");

  await page.getByRole("button", { name: "Simulate offline" }).click();
  await expect(page.getByLabel("Connection status")).toContainText("Offline");
  await page.getByRole("button", { name: "Reconnect and replay" }).click();

  await expect(page.getByLabel("Connection status")).toContainText("Recovered");
  await expect(page.getByLabel("Conversation timeline")).toContainText("Recovered missed durable conversation state after reconnect.");
});

test("Conversation core mobile layout avoids horizontal overflow", async ({ page }) => {
  await page.goto("/chat?role=client");

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

test("Conversation reduced motion keeps state legible without smooth scroll dependency", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/conversations?role=staff");

  await page.getByRole("button", { name: "Jump to latest" }).click();
  await expect(page.getByLabel("Conversation timeline")).toContainText("Ava is typing");
  const scrollBehavior = await page.locator(".timeline-panel").evaluate((element) => getComputedStyle(element).scrollBehavior);
  expect(scrollBehavior).toBe("auto");
});

test("Admin role can access appliance system rail without leaking it to clients", async ({ page }) => {
  await page.goto("/chat?role=admin");

  const systemNav = page.getByRole("navigation", { name: "Admin system navigation" });
  await expect(systemNav).toBeVisible();
  await expect(systemNav.getByRole("link", { name: /System/ })).toBeVisible();
  await expect(systemNav.getByRole("link", { name: /Logs/ })).toBeVisible();
  await expect(systemNav.getByRole("link", { name: /Backup/ })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Staff business navigation" })).toBeVisible();
});

test("Account tools are role-specific and keep affiliates outside the staff rail", async ({ page }) => {
  await page.goto("/account?role=affiliate");

  await expect(page.getByRole("heading", { name: "Account", exact: true })).toBeVisible();
  await expect(page.locator("main")).toContainText("Affiliate dashboard");
  await expect(page.locator("main")).toContainText("Referral links");
  await expect(page.locator("main")).toContainText("Approved materials");
  await expect(page.getByRole("navigation", { name: "Staff business navigation" })).toHaveCount(0);
  await expect(page.getByRole("navigation", { name: "Admin system navigation" })).toHaveCount(0);
});

test("Product surfaces load latest completed brief before raw surface detail", async ({ page }) => {
  await page.goto("/offers?role=staff");

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
