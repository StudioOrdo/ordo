import { spawn } from "node:child_process";
import { mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import process from "node:process";

import { loadLocalEnv } from "./local-env.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const daemonPort = Number(process.env.ORDO_DEV_DAEMON_PORT ?? "17760");
const dataDir = resolve(repoRoot, process.env.ORDO_DEV_DATA_DIR ?? ".data/live-provider-test");
const dbPath = resolve(dataDir, "local.db");
const daemonUrl = `http://127.0.0.1:${daemonPort}`;

await mkdir(dataDir, { recursive: true });
if (await daemonAlreadyListening(daemonUrl)) {
  console.log(`ordo-daemon is already listening at ${daemonUrl}; reusing the existing process.`);
  console.log("If member chat still reports live provider disabled, stop that daemon and rerun this script.");
  process.exit(0);
}

const localEnv = await loadLocalEnv(repoRoot);
const env = {
  ...localEnv,
  ...process.env,
  ORDO_DB_PATH: dbPath,
  ORDO_APP_LIVE_LLM: process.env.ORDO_APP_LIVE_LLM ?? "1",
  ORDO_PROVIDER_BENCHMARK_LIVE: process.env.ORDO_PROVIDER_BENCHMARK_LIVE ?? "1",
  ORDO_LIVE_LLM_ALLOW_NETWORK: process.env.ORDO_LIVE_LLM_ALLOW_NETWORK ?? "1",
  ORDO_LIVE_LLM_BUDGET_USD: process.env.ORDO_LIVE_LLM_BUDGET_USD ?? "0.10",
  ORDO_LIVE_LLM_TIMEOUT_MS: process.env.ORDO_LIVE_LLM_TIMEOUT_MS ?? "120000",
  ORDO_NEXT_COMMAND: "true",
};

const child = spawn(
  "cargo",
  [
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
  ],
  {
    cwd: repoRoot,
    env,
    stdio: "inherit",
  },
);

process.on("SIGINT", () => child.kill("SIGINT"));
process.on("SIGTERM", () => child.kill("SIGTERM"));

child.on("exit", (code, signal) => {
  process.exitCode = signal ? 1 : (code ?? 0);
});

async function daemonAlreadyListening(url) {
  try {
    const response = await fetch(`${url}/health`, { cache: "no-store" });
    return response.ok;
  } catch {
    return false;
  }
}
