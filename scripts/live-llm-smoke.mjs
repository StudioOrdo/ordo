import { spawn } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import process from "node:process";

import { loadLocalEnv } from "./local-env.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const localEnv = await loadLocalEnv(repoRoot);
const env = {
  ...localEnv,
  ORDO_LIVE_LLM_EVALS: "1",
  ORDO_LIVE_LLM_ALLOW_NETWORK: "1",
  ORDO_LIVE_LLM_PROVIDER: process.env.ORDO_LIVE_LLM_PROVIDER ?? localEnv.ORDO_LIVE_LLM_PROVIDER ?? "openai",
  ORDO_LIVE_LLM_MODEL: process.env.ORDO_LIVE_LLM_MODEL ?? localEnv.ORDO_LIVE_LLM_MODEL ?? "gpt-5",
  ORDO_LIVE_LLM_MAX_CASES: process.env.ORDO_LIVE_LLM_MAX_CASES ?? localEnv.ORDO_LIVE_LLM_MAX_CASES ?? "1",
  ORDO_LIVE_LLM_BUDGET_USD: process.env.ORDO_LIVE_LLM_BUDGET_USD ?? localEnv.ORDO_LIVE_LLM_BUDGET_USD ?? "0.01",
  ORDO_DB_PATH: process.env.ORDO_DB_PATH ?? localEnv.ORDO_DB_PATH ?? ".data/local.db",
};

const child = spawn("cargo", [
  "run",
  "-p",
  "ordo-daemon",
  "--",
  "run-live-llm-eval-json",
  "--db-path",
  env.ORDO_DB_PATH,
], {
  cwd: repoRoot,
  env,
  stdio: "inherit",
});

child.on("exit", (code, signal) => {
  process.exitCode = signal ? 1 : code ?? 0;
});