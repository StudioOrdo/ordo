import { spawn } from "node:child_process";
import { mkdir, open, readFile, rm, writeFile } from "node:fs/promises";
import net from "node:net";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import process from "node:process";

import { loadLocalEnv } from "./local-env.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const localEnv = await loadLocalEnv(repoRoot);
const command = process.argv[2] ?? "status";
const nextPort = positivePort(localEnv.ORDO_DEV_NEXT_PORT, 3000, "ORDO_DEV_NEXT_PORT");
const daemonPort = positivePort(localEnv.ORDO_DEV_DAEMON_PORT, 17760, "ORDO_DEV_DAEMON_PORT");
const dataDir = resolve(repoRoot, localEnv.ORDO_DEV_DATA_DIR ?? ".data/dev-appliance");
const pidPath = resolve(dataDir, "ordo-dev.pid");
const logPath = resolve(dataDir, "ordo-dev.log");
const daemonUrl = `http://127.0.0.1:${daemonPort}`;
const nextUrl = `http://127.0.0.1:${nextPort}`;

await mkdir(dataDir, { recursive: true });

switch (command) {
  case "start":
    await start();
    break;
  case "stop":
    await stop();
    break;
  case "restart":
    await stop({ quiet: true });
    await start();
    break;
  case "status":
    await status();
    break;
  case "help":
  case "--help":
  case "-h":
    printHelp();
    break;
  default:
    console.error(`Unknown Ordo dev command: ${command}`);
    printHelp();
    process.exit(2);
}

async function start() {
  const current = await currentStatus();
  if (current.daemon.ready && current.next.ok) {
    await writePidIfKnown(current.daemon.pid);
    console.log("Ordo is already running.");
    printStatus(current);
    return;
  }

  if (current.daemon.ok || current.daemon.pid || current.next.pid) {
    console.log("Stopping stale Ordo dev processes before start...");
    await stop({ quiet: true });
  }

  const out = await open(logPath, "a");
  const err = await open(logPath, "a");
  const child = spawn("node", ["scripts/dev-appliance.mjs"], {
    cwd: repoRoot,
    env: { ...process.env, ...localEnv },
    detached: true,
    stdio: ["ignore", out.fd, err.fd],
  });
  child.unref();
  await writeFile(pidPath, `${child.pid}\n`, "utf8");
  await out.close();
  await err.close();

  console.log(`Starting Ordo dev appliance in the background (pid ${child.pid}).`);
  console.log(`Logs: ${relative(logPath)}`);

  const ready = await waitForReady(90_000);
  if (!ready.daemon.ready || !ready.next.ok) {
    console.error("Ordo did not become ready within 90 seconds.");
    printStatus(ready);
    console.error(`Check ${relative(logPath)} for the startup error.`);
    process.exit(1);
  }

  printStatus(ready);
}

async function stop(options = {}) {
  const quiet = options.quiet === true;
  const pids = new Set();
  const pidFilePid = await readPidFile();
  if (pidFilePid) pids.add(pidFilePid);

  const daemonPid = await listenerPid(daemonPort);
  if (daemonPid && (await looksLikeOrdoDaemon())) pids.add(daemonPid);

  const nextPid = await listenerPid(nextPort);
  if (nextPid && (pidFilePid || daemonPid || (await looksLikeNext()))) pids.add(nextPid);

  if (pids.size === 0) {
    await rm(pidPath, { force: true });
    if (!quiet) console.log("Ordo is not running.");
    return;
  }

  for (const pid of pids) {
    await terminatePid(pid);
  }

  await waitForPortsToClose(10_000);
  await rm(pidPath, { force: true });
  if (!quiet) console.log("Ordo dev appliance stopped.");
}

async function status() {
  printStatus(await currentStatus());
}

async function currentStatus() {
  const daemonPid = await listenerPid(daemonPort);
  const nextPid = await listenerPid(nextPort);
  const health = await fetchJson(`${daemonUrl}/health`, 2_000).catch(() => null);
  const ready = await fetchJson(`${daemonUrl}/ready`, 2_000).catch(() => null);
  const next = await fetchHead(nextUrl, 2_000).catch(() => null);
  const runnerPid = await readPidFile();

  return {
    daemonUrl,
    nextUrl,
    logPath,
    runnerPid,
    daemon: {
      pid: daemonPid,
      ok: health?.status === "ok",
      ready: ready?.status === "ok" || ready?.status === "ready",
      healthStatus: health?.status ?? null,
      readyStatus: ready?.status ?? null,
      nextCheck: ready?.checks?.next?.status ?? null,
      message: ready?.checks?.next?.message ?? ready?.message ?? null,
    },
    next: {
      pid: nextPid,
      ok: Boolean(next?.ok),
      status: next?.status ?? null,
    },
  };
}

function printStatus(snapshot) {
  console.log(`Daemon: ${snapshot.daemon.ready ? "ready" : snapshot.daemon.ok ? "healthy but not ready" : "stopped"} ${snapshot.daemon.pid ? `(pid ${snapshot.daemon.pid})` : ""}`);
  console.log(`Next:   ${snapshot.next.ok ? `ready (pid ${snapshot.next.pid ?? "unknown"})` : "stopped"}`);
  console.log(`UI:     ${snapshot.nextUrl}`);
  console.log(`Daemon: ${snapshot.daemonUrl}`);
  console.log(`Logs:   ${relative(snapshot.logPath)}`);
  if (snapshot.daemon.message && !snapshot.daemon.ready) {
    console.log(`Reason: ${snapshot.daemon.message}`);
  }
}

async function waitForReady(timeoutMs) {
  const startedAt = Date.now();
  let snapshot = await currentStatus();
  while (Date.now() - startedAt < timeoutMs) {
    snapshot = await currentStatus();
    if (snapshot.daemon.ready && snapshot.next.ok) return snapshot;
    await delay(1_000);
  }
  return snapshot;
}

async function waitForPortsToClose(timeoutMs) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const daemonOpen = !(await portIsAvailable(daemonPort));
    const nextOpen = !(await portIsAvailable(nextPort));
    if (!daemonOpen && !nextOpen) return;
    await delay(300);
  }
}

async function terminatePid(pid) {
  if (!processIsRunning(pid)) return;
  try {
    process.kill(pid, "SIGTERM");
  } catch {
    return;
  }

  const startedAt = Date.now();
  while (Date.now() - startedAt < 5_000) {
    if (!processIsRunning(pid)) return;
    await delay(200);
  }
  console.warn(`Process ${pid} did not exit after SIGTERM. Stop it manually if it is still holding a port.`);
}

async function looksLikeOrdoDaemon() {
  const health = await fetchJson(`${daemonUrl}/health`, 1_500).catch(() => null);
  return health?.service === "ordo-daemon" || health?.status === "ok";
}

async function looksLikeNext() {
  const response = await fetchHead(nextUrl, 1_500).catch(() => null);
  return Boolean(response?.ok || response?.headers?.get?.("x-powered-by")?.toLowerCase().includes("next"));
}

async function readPidFile() {
  try {
    const pid = Number.parseInt((await readFile(pidPath, "utf8")).trim(), 10);
    return Number.isInteger(pid) && pid > 0 && processIsRunning(pid) ? pid : null;
  } catch {
    return null;
  }
}

async function writePidIfKnown(pid) {
  if (!pid) return;
  await writeFile(pidPath, `${pid}\n`, "utf8");
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

function portIsAvailable(port) {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.once("error", () => resolve(false));
    server.once("listening", () => server.close(() => resolve(true)));
    server.listen(port, "127.0.0.1");
  });
}

async function fetchJson(url, timeoutMs) {
  const response = await fetch(url, { cache: "no-store", signal: AbortSignal.timeout(timeoutMs) });
  if (!response.ok) throw new Error(`${url} returned ${response.status}`);
  return response.json();
}

async function fetchHead(url, timeoutMs) {
  const response = await fetch(url, { method: "HEAD", cache: "no-store", signal: AbortSignal.timeout(timeoutMs) });
  return { ok: response.ok, status: response.status, headers: response.headers };
}

function processIsRunning(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function positivePort(value, fallback, label) {
  const raw = value ?? String(fallback);
  const parsed = Number.parseInt(raw, 10);
  if (Number.isInteger(parsed) && parsed > 0 && parsed < 65536) return parsed;
  throw new Error(`${label} must be a TCP port between 1 and 65535.`);
}

function relative(path) {
  return path.startsWith(`${repoRoot}/`) ? path.slice(repoRoot.length + 1) : path;
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function printHelp() {
  console.log("Usage: node scripts/ordo-dev.mjs <start|stop|restart|status>");
}
