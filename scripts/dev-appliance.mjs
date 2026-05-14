import { spawn } from "node:child_process";
import { mkdir, readFile } from "node:fs/promises";
import net from "node:net";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import process from "node:process";

import { loadLocalEnv } from "./local-env.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const localEnv = await loadLocalEnv(repoRoot);
const nextPort = positivePort(localEnv.ORDO_DEV_NEXT_PORT, 3000, "ORDO_DEV_NEXT_PORT");
const daemonPort = positivePort(localEnv.ORDO_DEV_DAEMON_PORT, 17760, "ORDO_DEV_DAEMON_PORT");
const dataDir = resolve(repoRoot, localEnv.ORDO_DEV_DATA_DIR ?? ".data/dev-appliance");
const dbPath = resolve(dataDir, "local.db");
const daemonUrl = `http://127.0.0.1:${daemonPort}`;
const nextUrl = `http://127.0.0.1:${nextPort}`;
const ollamaBaseUrl = trimmed(localEnv.ORDO_OLLAMA_BASE_URL) ?? trimmed(localEnv.OLLAMA_BASE_URL) ?? "http://127.0.0.1:11434/api";
const ollamaModel = trimmed(localEnv.ORDO_OLLAMA_MODEL) ?? trimmed(localEnv.OLLAMA_MODEL) ?? "qwen2.5-coder:7b";

await mkdir(dataDir, { recursive: true });
await runPreflight();

const env = {
  ...localEnv,
  ORDO_DB_PATH: dbPath,
  ORDO_DAEMON_URL: daemonUrl,
  NEXT_PUBLIC_ORDO_DAEMON_WS_URL: `ws://127.0.0.1:${daemonPort}/ws`,
  NEXT_PUBLIC_ORDO_DAEMON_CHAT_WS_URL: `ws://127.0.0.1:${daemonPort}/chat/ws`,
  ORDO_NEXT_COMMAND: "npm",
  ORDO_NEXT_ARGS: `run dev:next -- --hostname 127.0.0.1 --port ${nextPort}`,
  HOSTNAME: "127.0.0.1",
  PORT: String(nextPort),
};

console.log("Starting Ordo appliance dev runtime");
console.log(`- Next.js: ${nextUrl}`);
console.log(`- Rust daemon: ${daemonUrl}`);
console.log(`- SQLite: ${dbPath}`);
console.log(`- Local Ollama: ${ollamaBaseUrl} (${ollamaModel})`);

const child = spawn("cargo", [
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
  dbPath,
], {
  cwd: repoRoot,
  env,
  stdio: "inherit",
});

let readinessSettled = false;
pollStartupReadiness().catch((error) => {
  if (!readinessSettled) {
    console.error(`Dev runtime readiness check failed: ${error.message}`);
  }
});

process.on("SIGINT", () => child.kill("SIGINT"));
process.on("SIGTERM", () => child.kill("SIGTERM"));

child.on("exit", (code, signal) => {
  readinessSettled = true;
  process.exitCode = signal ? 1 : code ?? 0;
});

async function runPreflight() {
  console.log("Running Ordo dev preflight...");
  await requireCommand("cargo", ["--version"]);
  await requireCommand("npm", ["--version"]);
  await requireNoActiveNextDevServer();
  await requireDaemonPortAvailable(daemonPort);
  await requireFreePort("Next.js", nextPort);
  await checkOllama();
}

async function requireCommand(command, args) {
  await new Promise((resolve, reject) => {
    const probe = spawn(command, args, { cwd: repoRoot, stdio: "ignore" });
    probe.on("error", () => reject(new Error(`${command} is required for npm run dev.`)));
    probe.on("exit", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`${command} ${args.join(" ")} exited with code ${code}.`));
    });
  });
}

async function requireFreePort(label, port) {
  const available = await portIsAvailable(port);
  if (available) return;
  throw new Error(`${label} port ${port} is already in use. Stop the existing process or set ${label === "Rust daemon" ? "ORDO_DEV_DAEMON_PORT" : "ORDO_DEV_NEXT_PORT"}.`);
}

async function requireDaemonPortAvailable(port) {
  const available = await portIsAvailable(port);
  if (available) return;

  const existingUrl = `http://127.0.0.1:${port}`;
  const health = await fetchJson(`${existingUrl}/health`).catch(() => null);
  const ready = await fetchJson(`${existingUrl}/ready`).catch(() => null);
  const pid = await listenerPid(port);
  const processHint = pid ? `pid ${pid}` : "an unknown process";

  if (health?.status === "ok" && ready?.status === "ok") {
    throw new Error(`Ordo already appears to be running on ${existingUrl} (${processHint}). Open ${nextUrl}, or stop it with: kill ${pid ?? "<pid>"}`);
  }

  if (health?.status === "ok") {
    const reason = ready?.checks?.next?.message ?? ready?.message ?? "the existing daemon is not ready";
    throw new Error(`A stale Ordo daemon is already listening on ${existingUrl} (${processHint}), but it is not ready: ${reason}. Stop it with: kill ${pid ?? "<pid>"}, then rerun npm run dev.`);
  }

  throw new Error(`Rust daemon port ${port} is already in use by ${processHint}. Stop that process or set ORDO_DEV_DAEMON_PORT.`);
}

async function listenerPid(port) {
  if (process.platform === "win32") return null;
  return new Promise((resolve) => {
    const probe = spawn("lsof", ["-nP", `-iTCP:${port}`, "-sTCP:LISTEN", "-t"], { cwd: repoRoot, stdio: ["ignore", "pipe", "ignore"] });
    let output = "";
    probe.stdout.on("data", (chunk) => {
      output += chunk.toString();
    });
    probe.on("error", () => resolve(null));
    probe.on("exit", () => {
      const pid = output
        .split(/\s+/)
        .map((value) => Number.parseInt(value, 10))
        .find((value) => Number.isInteger(value) && value > 0);
      resolve(pid ?? null);
    });
  });
}

async function requireNoActiveNextDevServer() {
  const lockPath = resolve(repoRoot, ".next/dev/lock");
  let lock;
  try {
    lock = JSON.parse(await readFile(lockPath, "utf8"));
  } catch (error) {
    if (error?.code === "ENOENT") return;
    console.warn(`Could not read Next.js dev lock at ${lockPath}; continuing with port checks.`);
    return;
  }

  const pid = Number(lock.pid);
  if (!Number.isInteger(pid) || pid <= 0) return;
  if (!processIsRunning(pid)) return;

  const appUrl = typeof lock.appUrl === "string" ? lock.appUrl : `port ${lock.port ?? "unknown"}`;
  throw new Error(`Another Next.js dev server is already running for this repo at ${appUrl} (pid ${pid}). Stop it with: kill ${pid}`);
}

function processIsRunning(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function portIsAvailable(port) {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.once("error", () => resolve(false));
    server.once("listening", () => server.close(() => resolve(true)));
    server.listen(port, "127.0.0.1");
  });
}

async function checkOllama() {
  const requireOllama = localEnv.ORDO_DEV_REQUIRE_OLLAMA !== "0";
  try {
    const tagsUrl = new URL(`${ollamaBaseUrl.replace(/\/+$/, "")}/tags`);
    const response = await fetch(tagsUrl, { cache: "no-store", signal: AbortSignal.timeout(2_500) });
    if (!response.ok) throw new Error(`Ollama tags returned ${response.status}`);
    const payload = await response.json();
    const models = Array.isArray(payload.models) ? payload.models.map((model) => model?.name).filter(Boolean) : [];
    if (!models.includes(ollamaModel)) {
      throw new Error(`Ollama model ${ollamaModel} is not installed. Installed models: ${models.join(", ") || "none"}.`);
    }
  } catch (error) {
    const message = `Local Ollama preflight failed: ${error.message}`;
    if (requireOllama) {
      throw new Error(`${message} Start Ollama and run: ollama pull ${ollamaModel}. Set ORDO_DEV_REQUIRE_OLLAMA=0 only when intentionally developing without Local chat.`);
    }
    console.warn(`${message} Continuing because ORDO_DEV_REQUIRE_OLLAMA=0.`);
  }
}

async function pollStartupReadiness() {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 90_000) {
    if (readinessSettled) return;
    const health = await fetchJson(`${daemonUrl}/health`).catch(() => null);
    const ready = await fetchJson(`${daemonUrl}/ready`).catch(() => null);
    if (health?.status === "ok" && (ready?.status === "ok" || ready?.status === "ready")) {
      readinessSettled = true;
      console.log("Ordo dev runtime is ready.");
      console.log(`Open ${nextUrl}`);
      return;
    }
    await delay(1_000);
  }
  console.warn("Ordo daemon started but readiness did not settle within 90s. Check the daemon/Next output above.");
}

async function fetchJson(url) {
  const response = await fetch(url, { cache: "no-store", signal: AbortSignal.timeout(1_500) });
  if (!response.ok) throw new Error(`${url} returned ${response.status}`);
  return response.json();
}

function positivePort(value, fallback, label) {
  const raw = value ?? String(fallback);
  const parsed = Number.parseInt(raw, 10);
  if (Number.isInteger(parsed) && parsed > 0 && parsed < 65536) return parsed;
  throw new Error(`${label} must be a TCP port between 1 and 65535.`);
}

function trimmed(value) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}