# Install And Provider Setup MVP

Status: backend foundation exists; setup UI not built

## Why It Matters

Ordo needs a first-run path that lets an operator make the local appliance
usable without learning internal daemon routes or secret storage details.

## MVP Scope

- Show install state in the System shell.
- Capture or confirm owner display name, optional owner email, business name,
  and optional workspace label.
- Show provider list, enabled/default state, configured secret source, and
  locked env/file source state.
- Allow write-only provider API key update into the local vault.
- Explain the local vault plainly and honestly.

## Durable Product Nouns

- Install State
- Appliance Owner
- Business Profile
- Provider Config
- Vault Item

## Acceptance Criteria

- A fresh appliance can be completed through a minimal operator flow.
- Provider keys can be entered but never read back in plaintext.
- Env/file managed keys are visibly locked and cannot be overwritten locally.
- Backup/restore docs and tests prove the vault key behavior.
- Protected routes continue recording policy decisions with cataloged capability
  ids.

## Non-Goals

- Hosted identity, OAuth, login, password reset, or billing.
- Provider network validation unless deliberately scoped.
- OS keychain, passphrase, or external secret manager integration.
- Public Ordo surfaces.

## Validation

- Rust tests for install/provider/vault behavior.
- UI smoke test for setup/provider page once built.
- `cargo test --workspace` and relevant frontend validation.
