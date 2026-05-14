import process from "node:process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { loadLocalEnv } from "./local-env.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const env = { ...(await loadLocalEnv(repoRoot)), ...process.env };
const daemonUrl = trimmed(env.ORDO_DAEMON_URL) || "http://127.0.0.1:17760";
const chatWsUrl = trimmed(env.NEXT_PUBLIC_ORDO_DAEMON_CHAT_WS_URL) || chatUrlForDaemon(daemonUrl);
const message = trimmed(env.ORDO_PROVIDER_CHAT_MESSAGE) || trimmed(process.argv.slice(2).join(" "));

if (!message) {
  console.error("Set ORDO_PROVIDER_CHAT_MESSAGE, or pass the message text as arguments, before running provider chat smoke.");
  process.exit(2);
}

const providerSnapshot = await fetchJson(`${daemonUrl}/providers`);
const providers = providerSnapshot.providers
  .filter((provider) => provider.providerId !== "local")
  .filter((provider) => provider.apiKey?.configured)
  .filter((provider) => provider.availableModels?.length)
  .map((provider) => ({
    providerId: provider.providerId,
    providerName: provider.providerName,
    modelId: provider.model || provider.availableModels.find((model) => model.default)?.id || provider.availableModels[0].id,
  }));

if (providers.length === 0) {
  console.error("No configured live providers are available from the daemon provider read model.");
  process.exit(1);
}

const localSession = await postJson(`${daemonUrl}/local-sessions/register`, {
  mode: "register",
  name: "Provider Chat Smoke",
  email: `provider-chat-smoke-${Date.now()}@example.test`,
  password: "local-only-passphrase",
});
const bootstrap = await postJson(`${daemonUrl}/chat/bootstrap`, {
  sessionId: localSession.session.sessionId,
  actorId: localSession.session.actorId,
});

const rows = [];
for (const provider of providers) {
  rows.push(await runProviderChat(bootstrap.bootstrap, provider, message));
}

const failed = rows.filter((row) => row.status !== "succeeded");
console.log(JSON.stringify({
  createdAt: new Date().toISOString(),
  daemonUrl,
  providerCount: providers.length,
  rows,
}, null, 2));

if (failed.length > 0) {
  process.exit(1);
}

async function runProviderChat(bootstrap, provider, userMessage) {
  const socket = new WebSocket(chatWsUrl);
  const runId = `provider_chat_${provider.providerId}_${Date.now()}`;
  const clientMessageId = `provider_chat_message_${provider.providerId}_${Date.now()}`;
  const startedAt = Date.now();
  let status = "timeout";
  let failureCode = null;
  let failureMessage = null;
  let deltaCount = 0;
  let approximateOutputChars = 0;

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
    bodyMarkdown: userMessage,
    clientMessageId,
    messageKind: "human",
    visibility: "participants",
  })));
  socket.send(JSON.stringify(envelope("command", "llm.run.request", bootstrap.conversationId, {
    runId,
    assistantParticipantId: bootstrap.assistantParticipantId,
    providerId: provider.providerId,
    modelId: provider.modelId,
    userMessage,
  })));

  await new Promise((resolve) => {
    const timeout = setTimeout(resolve, Number(env.ORDO_PROVIDER_CHAT_TIMEOUT_MS || 120000));
    socket.addEventListener("message", (event) => {
      const frame = JSON.parse(event.data);
      if (frame.op === "error" || frame.type === "command.rejected") {
        status = "failed";
        failureCode = frame.payload?.code || frame.type || "command_rejected";
        failureMessage = safeFailureMessage(frame.payload?.message);
        clearTimeout(timeout);
        resolve();
      }
      if (frame.type === "llm.text.delta" && frame.payload?.runId === runId) {
        deltaCount += 1;
        approximateOutputChars += String(frame.payload.delta || "").length;
      }
      if (frame.type === "llm.run.failed" && frame.payload?.runId === runId) {
        status = "failed";
        failureCode = frame.payload.code || "llm_run_failed";
        failureMessage = safeFailureMessage(frame.payload.message);
        clearTimeout(timeout);
        resolve();
      }
      if (frame.type === "llm.run.completed" && frame.payload?.runId === runId) {
        status = "succeeded";
        clearTimeout(timeout);
        resolve();
      }
    });
  });
  socket.close();

  return {
    providerId: provider.providerId,
    providerName: provider.providerName,
    modelId: provider.modelId,
    status,
    failureCode,
    failureMessage,
    deltaCount,
    approximateOutputChars,
    totalLatencyMs: Date.now() - startedAt,
  };
}

function envelope(op, type, conversationId, payload) {
  return {
    schemaVersion: "conversation.gateway.v1",
    op,
    type,
    clientId: `provider_chat_${type.replace(/[^a-z0-9]+/gi, "_")}_${Date.now()}`,
    conversationId: conversationId ?? undefined,
    durability: op === "identify" ? "ephemeral" : "durable",
    scope: op === "identify" ? "user" : "conversation",
    payload,
    occurredAt: new Date().toISOString(),
  };
}

function chatUrlForDaemon(url) {
  const parsed = new URL(url);
  parsed.protocol = parsed.protocol === "https:" ? "wss:" : "ws:";
  parsed.pathname = "/chat/ws";
  parsed.search = "";
  return parsed.toString();
}

function waitForOpen(socket) {
  return new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", () => reject(new Error("provider chat websocket failed")), { once: true });
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

function safeFailureMessage(value) {
  return typeof value === "string" ? value.replace(/[A-Za-z0-9_-]*sk-[A-Za-z0-9_-]+/g, "[redacted]") : null;
}

function trimmed(value) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}