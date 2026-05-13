import { spawn } from "node:child_process";
import { mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import process from "node:process";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const nextPort = Number(process.env.ORDO_REAL_DAEMON_NEXT_PORT ?? "3110");
const daemonPort = Number(process.env.ORDO_REAL_DAEMON_PORT ?? "19180");
const smokeDataDir = resolve(repoRoot, ".data/ui-real-daemon-chat-smoke");
const smokeDbPath = resolve(smokeDataDir, "local.db");
const daemonUrl = `http://127.0.0.1:${daemonPort}`;
const nextUrl = `http://127.0.0.1:${nextPort}`;
const daemonEnv = {
  ...process.env,
  ORDO_DAEMON_URL: daemonUrl,
  NEXT_PUBLIC_ORDO_DAEMON_WS_URL: `ws://127.0.0.1:${daemonPort}/ws`,
  NEXT_PUBLIC_ORDO_DAEMON_CHAT_WS_URL: `ws://127.0.0.1:${daemonPort}/chat/ws`,
  ORDO_REAL_DAEMON_NEXT_PORT: String(nextPort),
  ORDO_REAL_DAEMON_PORT: String(daemonPort),
};

const children = new Set();
let shuttingDown = false;

async function main() {
  await rm(smokeDataDir, { recursive: true, force: true });
  await mkdir(smokeDataDir, { recursive: true });

  const daemon = spawnLogged("daemon", "cargo", [
    "run",
    "-p",
    "ordo-daemon",
    "--",
    "serve",
    "--host",
    "127.0.0.1",
    "--port",
    String(daemonPort),
    "--db-path",
    smokeDbPath,
  ], daemonEnv);
  await waitForUrl(`${daemonUrl}/ready`, 120_000, "daemon readiness");

  await runLogged("next-build", "npm", ["run", "build"], daemonEnv);

  const next = spawnLogged("next", "npm", [
    "run",
    "start",
    "--",
    "--hostname",
    "127.0.0.1",
    "--port",
    String(nextPort),
  ], daemonEnv);
  await waitForUrl(nextUrl, 120_000, "Next server");

  await runLogged("playwright", "npx", [
    "playwright",
    "test",
    "-c",
    "playwright.real-daemon.browser.config.ts",
  ], daemonEnv);

  shutdown();
  await Promise.allSettled([waitForExit(next), waitForExit(daemon)]);
}

function spawnLogged(label, command, args, env) {
  const child = spawn(command, args, {
    cwd: repoRoot,
    env,
    stdio: ["ignore", "pipe", "pipe"],
  });
  children.add(child);
  child.stdout.on("data", (chunk) => process.stdout.write(prefixLines(label, chunk)));
  child.stderr.on("data", (chunk) => process.stderr.write(prefixLines(label, chunk)));
  child.on("exit", (code, signal) => {
    children.delete(child);
    if (!shuttingDown && code !== 0) {
      console.error(`[${label}] exited with ${signal ?? code}`);
    }
  });
  return child;
}

async function runLogged(label, command, args, env) {
  const child = spawnLogged(label, command, args, env);
  const result = await waitForExit(child);
  if (result.code !== 0) {
    throw new Error(`${label} failed with ${result.signal ?? result.code}`);
  }
}

function waitForExit(child) {
  return new Promise((resolve) => {
    if (child.exitCode !== null || child.signalCode !== null) {
      resolve({ code: child.exitCode, signal: child.signalCode });
      return;
    }
    child.once("exit", (code, signal) => resolve({ code, signal }));
  });
}

async function waitForUrl(url, timeoutMs, label) {
  const deadline = Date.now() + timeoutMs;
  let lastError = "unavailable";
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url, { cache: "no-store" });
      if (response.ok) {
        return;
      }
      lastError = `${response.status} ${response.statusText}`;
    } catch (error) {
      lastError = error instanceof Error ? error.message : String(error);
    }
    await delay(500);
  }
  throw new Error(`Timed out waiting for ${label} at ${url}: ${lastError}`);
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function prefixLines(label, chunk) {
  return String(chunk)
    .split(/(?<=\n)/)
    .filter(Boolean)
    .map((line) => `[${label}] ${line}`)
    .join("");
}

function shutdown() {
  shuttingDown = true;
  for (const child of children) {
    if (!child.killed) {
      child.kill("SIGTERM");
    }
  }
}

process.on("SIGINT", () => {
  shutdown();
  process.exitCode = 130;
});
process.on("SIGTERM", () => {
  shutdown();
  process.exitCode = 143;
});
process.on("exit", shutdown);

main().catch((error) => {
  console.error(error);
  shutdown();
  process.exitCode = 1;
});
