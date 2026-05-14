import process from "node:process";
import { performance } from "node:perf_hooks";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { loadLocalEnv } from "./local-env.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const env = { ...(await loadLocalEnv(repoRoot)), ...process.env };
const daemonUrl = trimmed(env.ORDO_DAEMON_URL) || "http://127.0.0.1:17760";
const chatWsUrl = trimmed(env.NEXT_PUBLIC_ORDO_DAEMON_CHAT_WS_URL) || "ws://127.0.0.1:17760/chat/ws";
const liveEnabled = env.ORDO_PROVIDER_BENCHMARK_LIVE === "1";
const networkEnabled = env.ORDO_LIVE_LLM_ALLOW_NETWORK === "1";
const budget = trimmed(env.ORDO_LIVE_LLM_BUDGET_USD);
const promptLimit = Math.max(1, Number(env.ORDO_PROVIDER_BENCHMARK_MAX_CASES || 2));
const prompts = [
  "Reply in one short sentence with a calm next step.",
  "Summarize this chat test in seven words.",
].slice(0, promptLimit);

let providerSnapshot = await fetchJson(`${daemonUrl}/providers`);
if (liveEnabled && networkEnabled && budget) {
  await enableBenchmarkProviders(providerSnapshot.providers);
  providerSnapshot = await fetchJson(`${daemonUrl}/providers`);
}
const providers = providerSnapshot.providers
  .filter((provider) => provider.enabled && provider.availableModels?.length)
  .map((provider) => ({
    providerId: provider.providerId,
    providerName: provider.providerName,
    modelId: provider.model || provider.availableModels.find((model) => model.default)?.id || provider.availableModels[0].id,
    live: provider.providerId !== "local",
  }));
providers.unshift({
  providerId: "local",
  providerName: "Local Ollama",
  modelId: env.ORDO_OLLAMA_MODEL || env.OLLAMA_MODEL || "qwen2.5-coder:7b",
  live: false,
});

if (providers.some((provider) => provider.live) && (!liveEnabled || !networkEnabled || !budget)) {
  console.error("Live provider benchmarks are skipped unless ORDO_PROVIDER_BENCHMARK_LIVE=1, ORDO_LIVE_LLM_ALLOW_NETWORK=1, and ORDO_LIVE_LLM_BUDGET_USD are set.");
}

const localSession = await postJson(`${daemonUrl}/local-sessions/register`, {
  mode: "register",
  name: "Provider Benchmark",
  email: `provider-benchmark-${Date.now()}@example.test`,
  password: "local-only-passphrase",
});
const bootstrap = await postJson(`${daemonUrl}/chat/bootstrap`, {
  sessionId: localSession.session.sessionId,
  actorId: localSession.session.actorId,
});
const rows = [];

for (const provider of providers) {
  if (provider.live && (!liveEnabled || !networkEnabled || !budget)) {
    rows.push({ providerId: provider.providerId, modelId: provider.modelId, status: "skipped_live_guard" });
    continue;
  }
  for (const prompt of prompts) {
    rows.push(await runCase(bootstrap.bootstrap, provider, prompt));
  }
}

const summary = summarize(rows);
console.log(JSON.stringify({ createdAt: new Date().toISOString(), daemonUrl, summary, rows }, null, 2));

async function runCase(bootstrap, provider, prompt) {
  const socket = new WebSocket(chatWsUrl);
  const runId = `benchmark_run_${provider.providerId}_${Date.now()}`;
  const clientMessageId = `benchmark_message_${provider.providerId}_${Date.now()}`;
  const timings = { startedAt: performance.now(), ackAt: null, firstDeltaAt: null, completedAt: null };
  let deltaCount = 0;
  let approximateOutputChars = 0;
  let failure = null;
  let failureMessage = null;

  await waitForOpen(socket);
  socket.send(JSON.stringify(envelope("identify", "gateway.identify", null, {
    actorId: bootstrap.actorId,
    participantId: bootstrap.participantId,
  })));
  socket.send(JSON.stringify(envelope("command", "conversation.subscribe", bootstrap.conversationId, {
    afterSequence: 0,
    limit: 1,
  })));
  socket.send(JSON.stringify(envelope("command", "message.submit", bootstrap.conversationId, {
    participantId: bootstrap.participantId,
    bodyMarkdown: prompt,
    clientMessageId,
    messageKind: "human",
    visibility: "participants",
  })));
  socket.send(JSON.stringify(envelope("command", "llm.run.request", bootstrap.conversationId, {
    runId,
    assistantParticipantId: bootstrap.assistantParticipantId,
    providerId: provider.providerId,
    modelId: provider.modelId,
    userMessage: prompt,
  })));

  await new Promise((resolve) => {
    const timeout = setTimeout(() => {
      failure = "timeout";
      resolve();
    }, Number(env.ORDO_PROVIDER_BENCHMARK_TIMEOUT_MS || 45000));
    socket.addEventListener("message", (event) => {
      const frame = JSON.parse(event.data);
      if (frame.type === "llm.run.request.ack") timings.ackAt = performance.now();
      if (frame.op === "error" || frame.type === "command.rejected") {
        failure = frame.payload?.code || frame.type || "command_rejected";
        failureMessage = typeof frame.payload?.message === "string" ? frame.payload.message : null;
        timings.completedAt = performance.now();
        clearTimeout(timeout);
        resolve();
      }
      if (frame.type === "llm.text.delta" && frame.payload?.runId === runId) {
        timings.firstDeltaAt ??= performance.now();
        deltaCount += 1;
        approximateOutputChars += String(frame.payload.delta || "").length;
      }
      if (frame.type === "llm.run.failed" && frame.payload?.runId === runId) {
        failure = frame.payload.code || "failed";
        failureMessage = typeof frame.payload.message === "string" ? frame.payload.message : null;
        timings.completedAt = performance.now();
        clearTimeout(timeout);
        resolve();
      }
      if (frame.type === "llm.run.completed" && frame.payload?.runId === runId) {
        timings.completedAt = performance.now();
        clearTimeout(timeout);
        resolve();
      }
    });
  });
  socket.close();

  const completedAt = timings.completedAt ?? performance.now();
  return {
    providerId: provider.providerId,
    modelId: provider.modelId,
    status: failure ? "failed" : "succeeded",
    failureCode: failure,
    failureMessage,
    timeToAckMs: elapsed(timings.startedAt, timings.ackAt),
    timeToFirstTokenMs: elapsed(timings.startedAt, timings.firstDeltaAt),
    totalLatencyMs: Math.round(completedAt - timings.startedAt),
    deltaCount,
    approximateOutputChars,
  };
}

async function enableBenchmarkProviders(providers) {
  for (const provider of providers) {
    if (provider.providerId === "local" || !provider.availableModels?.length) continue;
    const modelId = provider.model || provider.availableModels.find((model) => model.default)?.id || provider.availableModels[0].id;
    await putJson(`${daemonUrl}/providers/${provider.providerId}`, {
      enabled: true,
      defaultProvider: false,
      model: modelId,
    });
  }
}

function summarize(rows) {
  return Object.values(rows.reduce((groups, row) => {
    const key = `${row.providerId}/${row.modelId}`;
    groups[key] ??= { providerId: row.providerId, modelId: row.modelId, samples: [], skipped: 0 };
    if (row.status === "skipped_live_guard") groups[key].skipped += 1;
    else groups[key].samples.push(row);
    return groups;
  }, {})).map((group) => {
    const successful = group.samples.filter((row) => row.status === "succeeded");
    return {
      providerId: group.providerId,
      modelId: group.modelId,
      skipped: group.skipped,
      successRate: group.samples.length ? successful.length / group.samples.length : 0,
      medianTimeToFirstTokenMs: median(successful.map((row) => row.timeToFirstTokenMs).filter(Number.isFinite)),
      medianTotalLatencyMs: median(successful.map((row) => row.totalLatencyMs).filter(Number.isFinite)),
      failureRate: group.samples.length ? 1 - successful.length / group.samples.length : 0,
    };
  }).sort((left, right) => (left.medianTimeToFirstTokenMs ?? Infinity) - (right.medianTimeToFirstTokenMs ?? Infinity));
}

function envelope(op, type, conversationId, payload) {
  return {
    schemaVersion: "conversation.gateway.v1",
    op,
    type,
    clientId: `benchmark_${type.replace(/[^a-z0-9]+/gi, "_")}_${Date.now()}`,
    conversationId: conversationId ?? undefined,
    durability: op === "identify" ? "ephemeral" : "durable",
    scope: op === "identify" ? "user" : "conversation",
    payload,
    occurredAt: new Date().toISOString(),
  };
}

function waitForOpen(socket) {
  return new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", () => reject(new Error("benchmark websocket failed")), { once: true });
  });
}

async function fetchJson(url) {
  const response = await fetch(url, { cache: "no-store" });
  if (!response.ok) throw new Error(`${url} returned ${response.status}`);
  return response.json();
}

async function postJson(url, body) {
  const response = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
    cache: "no-store",
  });
  if (!response.ok) throw new Error(`${url} returned ${response.status}`);
  return response.json();
}

async function putJson(url, body) {
  const response = await fetch(url, {
    method: "PUT",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
    cache: "no-store",
  });
  if (!response.ok) throw new Error(`${url} returned ${response.status}`);
  return response.json();
}

function median(values) {
  if (!values.length) return null;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

function elapsed(start, end) {
  return end ? Math.round(end - start) : null;
}

function trimmed(value) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}
