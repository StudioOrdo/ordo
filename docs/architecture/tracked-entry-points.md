# Tracked Entry Points And Visitor Sessions

Status: backend foundation implemented

This slice gives Ordo a durable path from public QR/link/campaign entry into a
visitor session without adding UI, analytics dashboards, offer lifecycle state,
or hosted identity.

## What Is Implemented

SQLite stores:

- `tracked_entry_points`: stable slugs, labels, source metadata, public-safe
  destinations, generated public paths, QR payload JSON, attribution metadata,
  status, and timestamps.
- `visitor_sessions`: session records tied to tracked entry points with copied
  destination and attribution context.
- `visitor_session_events`: durable session events for later briefs, offers,
  and attribution work.

The daemon exposes protected owner/operator endpoints:

- `GET /entry-points`
- `POST /entry-points`
- `PUT /entry-points/:entry_point_id`
- `GET /visitor-sessions`

Those routes use the protected daemon access boundary and non-MCP-exported
capability ids.

The daemon also exposes public-safe endpoints:

- `GET /public/e/:slug`
- `POST /public/visitor-sessions`

These endpoints do not expose owner management metadata. They resolve active
entry points and start visitor sessions only when the destination is already
available through the published public surface read models.

## Public Destination Boundary

Tracked entry points can point only at public surface destinations:

- About
- Offers
- Asks
- Feed

The backend checks destination readiness against the public surface read models.
Facts with owner, staff, authenticated, draft, archived, or revoked state cannot
make a tracked entry point publicly resolvable.

## Event Evidence

Starting a visitor session records both:

- a `visitor_session_events` row; and
- a persisted realtime event with type `visitor_session.started`.

The event payload includes session id, entry point id, slug, destination surface,
and destination id. It does not include raw user agent text.

## Non-Goals

- No UI implementation.
- No full analytics dashboard.
- No cookie-heavy ad tracking.
- No affiliate payouts.
- No offer acceptance or trial lifecycle state changes.