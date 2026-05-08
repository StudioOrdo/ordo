# Scheduler

Status: Draft contract for Ordo 0.1.0

The scheduler is the appliance clock. It creates jobs from process templates.

## Scheduler Rule

The scheduler never runs arbitrary code. It creates jobs from approved process
templates whose tasks reference cataloged task kinds.

## 0.1.0 Schedule Types

Support interval and one-shot schedules first.

Cron expressions, advanced calendars, and historical ETA prediction can come
later.

## Scheduled Work

Initial scheduled work should include:

- `system.health.check` on a short interval;
- `brief.system.generate` on a periodic interval;
- `backup.create` when automatic backups are enabled.

Schedules should store last due time, next due time, enabled state, owner, and
run history.

## Recovery

The daemon should recover from restarts by inspecting due schedules in SQLite.
Missed work must be explicit: run, skip, or mark missed according to schedule
policy. Silent loss is not acceptable.