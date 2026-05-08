use anyhow::{bail, Result};
use rusqlite::Connection;
use std::fs;
use std::path::Path;

use crate::capabilities::seed_builtin_capabilities;
use crate::scheduler::ensure_default_system_brief_schedule;
use crate::templates::seed_builtin_templates;

pub const REQUIRED_TABLES: &[&str] = &[
    "capabilities",
    "process_templates",
    "jobs",
    "job_tasks",
    "job_task_dependencies",
    "job_events",
    "realtime_events",
    "diagnostic_logs",
    "job_artifacts",
    "issue_report_artifacts",
    "schedules",
    "scheduled_job_runs",
    "brief_artifacts",
    "preferences",
];

pub const CURRENT_SCHEMA_VERSION: i64 = 5;

type MigrationFn = fn(&Connection) -> Result<()>;

struct SchemaMigration {
    version: i64,
    name: &'static str,
    apply: MigrationFn,
}

const MIGRATIONS: &[SchemaMigration] = &[
    SchemaMigration {
        version: 1,
        name: "create_initial_appliance_schema",
        apply: create_initial_appliance_schema,
    },
    SchemaMigration {
        version: 2,
        name: "add_capability_bindings",
        apply: add_capability_bindings,
    },
    SchemaMigration {
        version: 3,
        name: "add_mcp_policy_metadata",
        apply: add_mcp_policy_metadata,
    },
    SchemaMigration {
        version: 4,
        name: "add_realtime_event_replay",
        apply: add_realtime_event_replay,
    },
    SchemaMigration {
        version: 5,
        name: "add_diagnostics_and_reports",
        apply: add_diagnostics_and_reports,
    },
];

pub fn init_database(db_path: &Path) -> Result<()> {
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let connection = Connection::open(db_path)?;
    init_schema(&connection)?;
    seed_builtin_capabilities(&connection)?;
    seed_builtin_templates(&connection)?;
    ensure_default_system_brief_schedule(&connection)?;
    Ok(())
}

pub fn init_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    validate_migration_order()?;
    let current_version = schema_version(connection)?;
    if current_version > CURRENT_SCHEMA_VERSION {
        bail!(
            "Database schema version {current_version} is newer than supported version {CURRENT_SCHEMA_VERSION}"
        );
    }

    for migration in MIGRATIONS {
        if migration.version > current_version {
            (migration.apply)(connection)?;
            set_schema_version(connection, migration.version)?;
        }
    }

    Ok(())
}

fn create_initial_appliance_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS process_templates (
            id TEXT NOT NULL,
            capability_id TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL,
            name TEXT NOT NULL,
            version INTEGER NOT NULL,
            description TEXT NOT NULL,
            tasks_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (id, version)
        );

        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            template_id TEXT NOT NULL,
            template_version INTEGER NOT NULL,
            capability_id TEXT NOT NULL DEFAULT '',
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
            capability_id TEXT NOT NULL DEFAULT '',
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

        CREATE TABLE IF NOT EXISTS realtime_events (
            cursor INTEGER PRIMARY KEY AUTOINCREMENT,
            schema_version TEXT NOT NULL,
            family TEXT NOT NULL,
            event_type TEXT NOT NULL,
            job_id TEXT,
            task_key TEXT,
            job_sequence INTEGER,
            payload_json TEXT NOT NULL DEFAULT '{}',
            occurred_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_realtime_events_cursor ON realtime_events(cursor);
        CREATE INDEX IF NOT EXISTS idx_realtime_events_family_cursor ON realtime_events(family, cursor);

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

        CREATE TABLE IF NOT EXISTS capabilities (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            description TEXT NOT NULL,
            family TEXT NOT NULL,
            input_schema_json TEXT NOT NULL DEFAULT '{}',
            output_contract_json TEXT NOT NULL DEFAULT '{}',
            roles_allowed_json TEXT NOT NULL DEFAULT '[]',
            execution_target TEXT NOT NULL,
            timeout_seconds INTEGER NOT NULL DEFAULT 30,
            retry_policy_json TEXT NOT NULL DEFAULT '{}',
            artifact_kinds_json TEXT NOT NULL DEFAULT '[]',
            scheduler_eligible INTEGER NOT NULL DEFAULT 0,
            prompt_exposure TEXT NOT NULL DEFAULT 'internal',
            mcp_export_policy TEXT NOT NULL DEFAULT 'dangerous_none',
            side_effects_json TEXT NOT NULL DEFAULT '[]',
            approval_requirement TEXT NOT NULL DEFAULT 'not_exported',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_capabilities_family ON capabilities(family, id);
        "#,
    )?;

    Ok(())
}

fn add_capability_bindings(connection: &Connection) -> Result<()> {
    ensure_column(
        connection,
        "process_templates",
        "capability_id",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        connection,
        "jobs",
        "capability_id",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        connection,
        "job_tasks",
        "capability_id",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    connection.execute(
        "UPDATE process_templates SET capability_id = kind WHERE capability_id = ''",
        [],
    )?;
    connection.execute(
        "UPDATE jobs SET capability_id = kind WHERE capability_id = ''",
        [],
    )?;
    connection.execute(
        "UPDATE job_tasks SET capability_id = task_kind WHERE capability_id = ''",
        [],
    )?;

    Ok(())
}

fn add_mcp_policy_metadata(connection: &Connection) -> Result<()> {
    ensure_column(
        connection,
        "capabilities",
        "mcp_export_policy",
        "TEXT NOT NULL DEFAULT 'dangerous_none'",
    )?;
    ensure_column(
        connection,
        "capabilities",
        "side_effects_json",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(
        connection,
        "capabilities",
        "approval_requirement",
        "TEXT NOT NULL DEFAULT 'not_exported'",
    )?;
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_capabilities_mcp_export ON capabilities(mcp_export_policy, id)",
        [],
    )?;

    Ok(())
}

fn add_realtime_event_replay(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS realtime_events (
            cursor INTEGER PRIMARY KEY AUTOINCREMENT,
            schema_version TEXT NOT NULL,
            family TEXT NOT NULL,
            event_type TEXT NOT NULL,
            job_id TEXT,
            task_key TEXT,
            job_sequence INTEGER,
            payload_json TEXT NOT NULL DEFAULT '{}',
            occurred_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_realtime_events_cursor ON realtime_events(cursor);
        CREATE INDEX IF NOT EXISTS idx_realtime_events_family_cursor ON realtime_events(family, cursor);
        "#,
    )?;

    Ok(())
}

fn add_diagnostics_and_reports(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS diagnostic_logs (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            level TEXT NOT NULL,
            source TEXT NOT NULL,
            message TEXT NOT NULL,
            request_id TEXT,
            job_id TEXT,
            task_key TEXT,
            capability_id TEXT,
            event_type TEXT,
            error_code TEXT,
            duration_ms INTEGER,
            payload_json TEXT NOT NULL DEFAULT '{}'
        );

        CREATE INDEX IF NOT EXISTS idx_diagnostic_logs_time ON diagnostic_logs(timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_diagnostic_logs_level_time ON diagnostic_logs(level, timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_diagnostic_logs_source_time ON diagnostic_logs(source, timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_diagnostic_logs_job_time ON diagnostic_logs(job_id, timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_diagnostic_logs_task_time ON diagnostic_logs(task_key, timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_diagnostic_logs_capability_time ON diagnostic_logs(capability_id, timestamp DESC);

        CREATE TABLE IF NOT EXISTS issue_report_artifacts (
            id TEXT PRIMARY KEY,
            job_id TEXT,
            status TEXT NOT NULL,
            severity TEXT NOT NULL,
            title TEXT NOT NULL,
            summary TEXT NOT NULL,
            description TEXT NOT NULL,
            source_route TEXT,
            markdown_body TEXT NOT NULL,
            diagnostics_json TEXT NOT NULL DEFAULT '{}',
            evidence_json TEXT NOT NULL DEFAULT '[]',
            redactions_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            exported_at TEXT,
            submitted_at TEXT,
            external_url TEXT,
            FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_issue_report_artifacts_updated ON issue_report_artifacts(updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_issue_report_artifacts_status_updated ON issue_report_artifacts(status, updated_at DESC);
        "#,
    )?;
    Ok(())
}

fn validate_migration_order() -> Result<()> {
    for (index, migration) in MIGRATIONS.iter().enumerate() {
        let expected_version = (index as i64) + 1;
        if migration.version != expected_version {
            bail!(
                "Schema migration {} has version {}, expected {expected_version}",
                migration.name,
                migration.version
            );
        }
    }

    if CURRENT_SCHEMA_VERSION != MIGRATIONS.len() as i64 {
        bail!(
            "Current schema version {CURRENT_SCHEMA_VERSION} does not match migration count {}",
            MIGRATIONS.len()
        );
    }

    Ok(())
}

fn schema_version(connection: &Connection) -> Result<i64> {
    let version = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(version)
}

fn set_schema_version(connection: &Connection, version: i64) -> Result<()> {
    connection.pragma_update(None, "user_version", version)?;
    Ok(())
}

fn ensure_column(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
    definition: &str,
) -> Result<()> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for column in columns {
        if column? == column_name {
            return Ok(());
        }
    }

    connection.execute(
        &format!("ALTER TABLE {table_name} ADD COLUMN {column_name} {definition}"),
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_exists(connection: &Connection, table_name: &str) -> bool {
        let exists: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = ?1",
                [table_name],
                |row| row.get(0),
            )
            .unwrap();
        exists == 1
    }

    fn column_exists(connection: &Connection, table_name: &str, column_name: &str) -> bool {
        let mut statement = connection
            .prepare(&format!("PRAGMA table_info({table_name})"))
            .unwrap();
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap();
        let column_names = columns.collect::<rusqlite::Result<Vec<_>>>().unwrap();
        column_names.iter().any(|column| column == column_name)
    }

    #[test]
    fn initializes_required_tables() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        for table_name in REQUIRED_TABLES {
            assert!(
                table_exists(&connection, table_name),
                "missing table {table_name}"
            );
        }
        assert_eq!(schema_version(&connection).unwrap(), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migrations_are_strictly_ordered() {
        validate_migration_order().unwrap();

        let versions: Vec<i64> = MIGRATIONS
            .iter()
            .map(|migration| migration.version)
            .collect();
        assert_eq!(versions, vec![1, 2, 3, 4, 5]);
        assert_eq!(CURRENT_SCHEMA_VERSION, 5);
    }

    #[test]
    fn init_schema_is_repeatable_and_preserves_data() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO preferences (key, value_json, updated_at) VALUES ('theme', '{\"mode\":\"dark\"}', 'now')",
                [],
            )
            .unwrap();

        init_schema(&connection).unwrap();

        let preference_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM preferences WHERE key = 'theme'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(preference_count, 1);
        assert_eq!(schema_version(&connection).unwrap(), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migrates_legacy_unversioned_database() {
        let connection = Connection::open_in_memory().unwrap();
        create_legacy_unversioned_database(&connection);

        assert_eq!(schema_version(&connection).unwrap(), 0);
        assert!(!column_exists(&connection, "jobs", "capability_id"));
        assert!(!column_exists(
            &connection,
            "capabilities",
            "mcp_export_policy"
        ));
        assert!(!table_exists(&connection, "realtime_events"));

        init_schema(&connection).unwrap();

        assert_eq!(schema_version(&connection).unwrap(), CURRENT_SCHEMA_VERSION);
        assert!(column_exists(
            &connection,
            "process_templates",
            "capability_id"
        ));
        assert!(column_exists(&connection, "jobs", "capability_id"));
        assert!(column_exists(&connection, "job_tasks", "capability_id"));
        assert!(column_exists(
            &connection,
            "capabilities",
            "mcp_export_policy"
        ));
        assert!(column_exists(
            &connection,
            "capabilities",
            "side_effects_json"
        ));
        assert!(column_exists(
            &connection,
            "capabilities",
            "approval_requirement"
        ));
        assert!(table_exists(&connection, "realtime_events"));

        let job_capability_id: String = connection
            .query_row(
                "SELECT capability_id FROM jobs WHERE id = 'job_legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let task_capability_id: String = connection
            .query_row(
                "SELECT capability_id FROM job_tasks WHERE id = 'task_legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(job_capability_id, "system.health.check");
        assert_eq!(task_capability_id, "system.health.probe");
    }

    #[test]
    fn process_templates_allow_multiple_versions() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        connection.execute(
            "INSERT INTO process_templates (
                id, capability_id, kind, name, version, description, tasks_json, created_at, updated_at
             ) VALUES ('brief.system.generate', 'brief.system.generate', 'brief.system.generate', 'System Brief', 1, 'v1', '[]', 'now', 'now')",
            [],
        ).unwrap();
        connection.execute(
            "INSERT INTO process_templates (
                id, capability_id, kind, name, version, description, tasks_json, created_at, updated_at
             ) VALUES ('brief.system.generate', 'brief.system.generate', 'brief.system.generate', 'System Brief', 2, 'v2', '[]', 'now', 'now')",
            [],
        ).unwrap();

        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM process_templates WHERE id = 'brief.system.generate'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 2);
    }

    fn create_legacy_unversioned_database(connection: &Connection) {
        connection
            .execute_batch(
                r#"
                CREATE TABLE process_templates (
                    id TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    name TEXT NOT NULL,
                    version INTEGER NOT NULL,
                    description TEXT NOT NULL,
                    tasks_json TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    PRIMARY KEY (id, version)
                );

                CREATE TABLE jobs (
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
                    failure_message TEXT
                );

                CREATE TABLE job_tasks (
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
                    UNIQUE(job_id, task_key)
                );

                CREATE TABLE capabilities (
                    id TEXT PRIMARY KEY,
                    label TEXT NOT NULL,
                    description TEXT NOT NULL,
                    family TEXT NOT NULL,
                    input_schema_json TEXT NOT NULL DEFAULT '{}',
                    output_contract_json TEXT NOT NULL DEFAULT '{}',
                    roles_allowed_json TEXT NOT NULL DEFAULT '[]',
                    execution_target TEXT NOT NULL,
                    timeout_seconds INTEGER NOT NULL DEFAULT 30,
                    retry_policy_json TEXT NOT NULL DEFAULT '{}',
                    artifact_kinds_json TEXT NOT NULL DEFAULT '[]',
                    scheduler_eligible INTEGER NOT NULL DEFAULT 0,
                    prompt_exposure TEXT NOT NULL DEFAULT 'internal',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                INSERT INTO process_templates (
                    id, kind, name, version, description, tasks_json, created_at, updated_at
                ) VALUES (
                    'system.health.check', 'system.health.check', 'Health Check', 1, 'legacy', '[]', 'now', 'now'
                );

                INSERT INTO jobs (
                    id, template_id, template_version, kind, status, origin, input_json,
                    required_task_count, completed_required_task_count, created_at, updated_at
                ) VALUES (
                    'job_legacy', 'system.health.check', 1, 'system.health.check', 'queued', 'test', '{}', 1, 0, 'now', 'now'
                );

                INSERT INTO job_tasks (
                    id, job_id, task_key, task_kind, label, required, status, input_json, created_at, updated_at
                ) VALUES (
                    'task_legacy', 'job_legacy', 'probe', 'system.health.probe', 'Probe', 1, 'pending', '{}', 'now', 'now'
                );
                "#,
            )
            .unwrap();
    }
}
