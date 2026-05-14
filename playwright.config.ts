import { defineConfig, devices } from "@playwright/test";

const nextPort = 3100;
const daemonPort = 19080;

export default defineConfig({
  testDir: "./tests/ui",
  fullyParallel: false,
  workers: 1,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: `http://127.0.0.1:${nextPort}`,
    trace: "retain-on-failure",
  },
  projects: [
    {
      name: "desktop-chromium",
      use: { ...devices["Desktop Chrome"], viewport: { width: 1280, height: 800 } },
    },
    {
      name: "mobile-chromium",
      use: { ...devices["Pixel 7"], viewport: { width: 390, height: 844 } },
    },
  ],
  webServer: {
    command: `ORDO_DAEMON_URL=http://127.0.0.1:${daemonPort} NEXT_PUBLIC_ORDO_DAEMON_WS_URL=ws://127.0.0.1:${daemonPort}/ws npm run build && mkdir -p .next/standalone/.next && cp -R .next/static .next/standalone/.next/static && cp -R public .next/standalone/public && ORDO_DAEMON_URL=http://127.0.0.1:${daemonPort} NEXT_PUBLIC_ORDO_DAEMON_WS_URL=ws://127.0.0.1:${daemonPort}/ws HOSTNAME=127.0.0.1 PORT=${nextPort} node .next/standalone/server.js`,
    url: `http://127.0.0.1:${nextPort}`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
