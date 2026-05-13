import { defineConfig, devices } from "@playwright/test";

const nextPort = Number(process.env.ORDO_REAL_DAEMON_NEXT_PORT ?? "3110");

export default defineConfig({
  testDir: "./tests/ui",
  testMatch: /real-daemon-chat\.spec\.ts/,
  fullyParallel: false,
  workers: 1,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: `http://127.0.0.1:${nextPort}`,
    trace: "retain-on-failure",
  },
  projects: [
    {
      name: "real-daemon-desktop-chromium",
      use: { ...devices["Desktop Chrome"], viewport: { width: 1280, height: 800 } },
    },
  ],
});
