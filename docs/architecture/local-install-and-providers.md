# Local Install And Providers

Status: Implemented backend foundation slice

This slice gives Ordo a durable local install and provider configuration spine.
It is a backend foundation for future public surfaces, Connections, handoffs,
and provider-backed AI features. It is not a hosted identity system, login UI,
RBAC expansion, Connections implementation, or frontend install wizard.

## What Is Implemented

The Rust daemon owns local install and provider configuration through SQLite and
protected daemon routes.

SQLite now stores:

- `install_state`: whether local install completed, completion time, local owner
  reference, business profile reference, and default provider reference.
- `appliance_owner`: local owner/operator display identity.
- `business_profile`: local business or workspace label.
- `vault_items`: encrypted local appliance vault values for provider keys and
  future sensitive appliance values.
- `provider_configs`: provider metadata, enable/default flags, model/base URL,
  provider-specific non-secret JSON, and a vault secret reference.

The daemon exposes protected local endpoints:

- `GET /install/state`
- `POST /install/complete`
- `GET /providers`
- `PUT /providers/:provider_id`

All four routes use the protected daemon access boundary. Requests must come
from loopback-to-daemon access or provide the configured daemon access token.
Policy decisions for these protected routes are written to the durable policy
decision audit trail.

## Install State

Fresh databases report an uninstalled appliance. Completing install records:

- local owner display name and optional email;
- business/workspace name and optional workspace label;
- optional default provider id;
- completion timestamp.

Install completion is intentionally single-use. A repeated completion request is
rejected instead of silently rewriting local owner identity.

Completion emits a durable system event:

```text
install.completed
```

The event payload includes stable identifiers and provider id only. It does not
include secrets.

## Provider Configuration

The initial provider catalog includes:

- `anthropic`
- `openai`
- `deepseek`
- `local`

Provider reads return a redacted view. API keys are write-only through the HTTP
surface and are never returned in plaintext. Local API keys are encrypted in the
local appliance vault and referenced from provider configuration rows. Read
models expose only:

- whether a key is configured;
- source: `env`, `file`, `vault`, or `missing`;
- whether the key is locked by env/file configuration;
- a redacted placeholder when configured.

`GET /providers` also returns a provider readiness summary for owner/system UI.
The summary reports the configured provider mode from `ORDO_LIVE_LLM_PROVIDER`,
the requested/default provider ids, whether credentials are present, missing
credential provider ids, and the live invocation guard state. The guard is
reported as read-only status; this slice does not enable live provider calls.

The OpenAI readiness resolver is included under the provider readiness summary.
It reports a redacted decision such as `disabled`, `missing_key`,
`unsupported_model`, `missing_budget`, `ready_but_live_disabled`, or
`ready_for_guarded_smoke`. The resolver checks the selected model against the
daemon catalog, resolves base URL, timeout, max case, budget, key source, live
eval opt-in, and network opt-in, and never returns raw key values. A
`ready_for_guarded_smoke` decision means the manual smoke-run guards are
satisfied. Conversation gateway live provider invocation still requires the
separate owner/developer app guard `ORDO_APP_LIVE_LLM=1`.

Default local app chat routes `providerId: "local"` through the daemon-owned
Ollama adapter at `http://127.0.0.1:11434/api` unless `ORDO_OLLAMA_BASE_URL` or
`OLLAMA_BASE_URL` overrides it. The default local model is `qwen2.5-coder:7b`
unless `ORDO_OLLAMA_MODEL` or `OLLAMA_MODEL` is set. The deterministic
`local_fake/fake-chat` adapter remains available only as an explicit fallback
for tests and eval fixtures. When `ORDO_APP_LIVE_LLM=1` is set and OpenAI
readiness is `ready_for_guarded_smoke`, `llm.run.request` uses the configured
OpenAI-compatible adapter and resolves the key only at the last moment from
env, secret-file, or vault sources. Unsupported providers, missing guards,
missing readiness, or model mismatches return safe `command.rejected` frames
without exposing raw prompts or secrets.

The browser member chat composer does not call a direct Next.js OpenAI stream
route. It submits human messages and assistant run requests through `/chat/ws`;
the direct `/api/chat/stream` route is retained only as a fail-closed boundary
so provider keys cannot bypass daemon policy.

Provider read models include catalog-backed model options for future UI
selection. The browser should render model choices from the daemon response
instead of hard-coding provider model ids. Provider updates reject model ids that
are not in the daemon catalog for catalog-backed providers.

Provider updates can store local metadata and local API keys. Local API keys are
stored as encrypted vault items. Environment or secret-file values take
precedence for secret presence. If an env/file API key is configured for a
provider, local API key updates are rejected with a typed invalid-request
response and a durable event:

```text
provider.settings.rejected_locked
```

Successful provider updates emit:

```text
provider.settings.updated
```

Provider events include provider id, enable/default state, secret presence, and
secret source only. They do not include raw API key values.

## Security Boundary

Provider secrets must remain write-only through daemon read models. They should
not appear in HTTP responses, durable events, policy decision metadata,
diagnostic logs, reports, or error messages.

The current implementation stores encrypted provider secrets in the local SQLite
`vault_items` table. The appliance-local vault key is generated automatically
and stored beside the SQLite database as `vault.key` inside the durable data
boundary. Users do not need to create or remember a passphrase in this slice.

User-facing promise:

```text
Your provider key is stored in the local Ordo vault. Ordo will never show it
again, include it in reports, or send it anywhere except to the provider you
configure.
```

Security honesty: the local appliance vault protects against casual database
inspection and accidental leakage in read models, events, logs, and reports. It
does not protect against a fully compromised host, container, or durable data
volume. Anyone with full access to both the encrypted database and `vault.key`
may be able to decrypt vault contents.

This slice does not require or integrate a hosted secret manager, OS keychain,
or user-managed passphrase. Future secret storage hardening may add passphrase,
keychain, or external secret-manager support without changing the read model
contract.

Local appliance backups preserve restore usability by archiving the SQLite
snapshot and selected data-boundary sidecar files such as `vault.key`. Backup
archives must therefore be protected like the durable `.data` volume.

## Non-Goals

- No frontend install wizard.
- No login, registration, password reset, hosted identity, or OAuth.
- No broad RBAC redesign.
- No Connections UI or connector marketplace.
- No affiliate, trial, public Ordo, or sales-loop implementation.
- No provider network validation.
- No external secret manager requirement.
- No user-managed vault passphrase.
- No OS keychain integration.
