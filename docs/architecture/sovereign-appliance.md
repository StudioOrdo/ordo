# Sovereign Appliance

## Direction

Ordo should ship as a simple local product image.

The intended core stack is:

- Next.js for product routes, UI, auth, policy, and read models;
- Rust for realtime fanout, native execution, backup/restore, media, and local
  search work;
- SQLite for durable local-first data;
- Docker for portable deployment;
- local files for generated artifacts, backups, and media.

## Boundary Rule

TypeScript owns product meaning.

Rust owns long-running native work, realtime fanout, and machine-sensitive
execution.

SQLite owns durable state.

The browser renders owner-safe read models and should not derive business state
from raw internals.