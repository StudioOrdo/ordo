use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

use crate::templates::seed_builtin_templates;

pub const REQUIRED_TABLES: &[&str] = &[
    "process_templates",
    "jobs",
    "job_tasks",
    "job_task_dependencies",
    "job_events",
    "job_artifacts",
    "schedules",
    "scheduled_job_runs",
    "brief_artifacts",
    "preferences",
];

pub fn init_database(db_path: &Path) -> Result<()> {
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let connection = Connection::open(db_path)?;
    init_schema(&connection)?;
    seed_builtin_templates(&connection)?;
    Ok(())
}

pub fn init_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS process_templates (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            name TEXT NOT NULL,
            version INTEGER NOT NULL,
            description TEXT NOT NULL,
            tasks_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(id, version)
        );

        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            template_id TEXT NOT NULL,
            template_version INTEGER NOT NULL,
            kind TEXT NOT NULL,
            status TEXT NOT NULL,
            origin TEXT NOT NULL,
            actor_id TEXT,
            input_json TEXT NOT NULL DEFAULT '{}',
            current_task_key TEXT,
            required_task_count INTEGER NOT NULL DEFAULT 0,
            completed_required_task_count INTEGER NOT NULL DEFAULT 0,
            started_at TEXT,
            completed_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            failure_message TEXT,
            FOREIGN KEY (template_id, template_version) REFERENCES process_templates(id, version)
        );

        CREATE INDEX IF NOT EXISTS idx_jobs_status_updated ON jobs(status, updated_at);

        CREATE TABLE IF NOT EXISTS job_tasks (
            id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL,
            task_key TEXT NOT NULL,
            task_kind TEXT NOT NULL,
            label TEXT NOT NULL,
            required INTEGER NOT NULL DEFAULT 1,
            status TEXT NOT NULL,
            input_json TEXT NOT NULL DEFAULT '{}',
            output_json TEXT,
            attempt_count INTEGER NOT NULL DEFAULT 0,
            started_at TEXT,
            completed_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            error_message TEXT,
            UNIQUE(job_id, task_key),
            FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_job_tasks_job_status ON job_tasks(job_id, status);

        CREATE TABLE IF NOT EXISTS job_task_dependencies (
            job_id TEXT NOT NULL,
            task_key TEXT NOT NULL,
            depends_on_task_key TEXT NOT NULL,
            PRIMARY KEY (job_id, task_key, depends_on_task_key),
            FOREIGN KEY (job_id, task_key) REFERENCES job_tasks(job_id, task_key) ON DELETE CASCADE,
            FOREIGN KEY (job_id, depends_on_task_key) REFERENCES job_tasks(job_id, task_key) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS job_events (
            id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL,
            task_key TEXT,
            sequence INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            UNIQUE(job_id, sequence),
            FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_job_events_job_sequence ON job_events(job_id, sequence);

        CREATE TABLE IF NOT EXISTS job_artifacts (
            id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL,
            task_key TEXT,
            artifact_kind TEXT NOT NULL,
            uri TEXT NOT NULL,
            label TEXT NOT NULL,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS schedules (
            id TEXT PRIMARY KEY,
            template_id TEXT NOT NULL,
            template_version INTEGER NOT NULL,
            name TEXT NOT NULL,
            schedule_kind TEXT NOT NULL,
            interval_seconds INTEGER,
            run_at TEXT,
            timezone TEXT NOT NULL DEFAULT 'UTC',
            enabled INTEGER NOT NULL DEFAULT 1,
            last_due_at TEXT,
            next_due_at TEXT NOT NULL,
            payload_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (template_id, template_version) REFERENCES process_templates(id, version)
        );

        CREATE INDEX IF NOT EXISTS idx_schedules_due ON schedules(enabled, next_due_at);

        CREATE TABLE IF NOT EXISTS scheduled_job_runs (
            id TEXT PRIMARY KEY,
            schedule_id TEXT NOT NULL,
            job_id TEXT,
            due_at TEXT NOT NULL,
            claimed_at TEXT,
            completed_at TEXT,
            status TEXT NOT NULL,
            error_message TEXT,
            FOREIGN KEY (schedule_id) REFERENCES schedules(id) ON DELETE CASCADE,
            FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS brief_artifacts (
            id TEXT PRIMARY KEY,
            section_key TEXT NOT NULL,
            job_id TEXT,
            version INTEGER NOT NULL,
            title TEXT NOT NULL,
            summary_json TEXT NOT NULL DEFAULT '[]',
            body_markdown TEXT NOT NULL,
            evidence_json TEXT NOT NULL DEFAULT '{}',
            limitations_json TEXT NOT NULL DEFAULT '[]',
            visibility TEXT NOT NULL,
            created_at TEXT NOT NULL,
            valid_until TEXT,
            FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_brief_artifacts_section_created ON brief_artifacts(section_key, created_at DESC);

        CREATE TABLE IF NOT EXISTS preferences (
            key TEXT PRIMARY KEY,
            value_json TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_required_tables() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        for table_name in REQUIRED_TABLES {
            let exists: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = ?1",
                    [table_name],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "missing table {table_name}");
        }
    }
}
