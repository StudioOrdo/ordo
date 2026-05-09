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
    "issue_report_exports",
    "issue_report_status_events",
    "support_packets",
    "support_packet_receipts",
    "actors",
    "roles",
    "actor_role_memberships",
    "resource_grants",
    "policy_decisions",
    "install_state",
    "appliance_owner",
    "business_profile",
    "vault_items",
    "provider_configs",
    "business_facts",
    "tracked_entry_points",
    "visitor_sessions",
    "visitor_session_events",
    "offers",
    "offer_acceptances",
    "trials",
    "trial_events",
    "connections",
    "connection_grants",
    "connection_events",
    "connection_receipts",
    "availability_schedules",
    "operator_presence",
    "handoff_eligibility_decisions",
    "handoff_inbox_items",
    "handoff_events",
    "handoff_receipts",
    "corpus_sources",
    "corpus_items",
    "schedules",
    "scheduled_job_runs",
    "brief_artifacts",
    "preferences",
];

pub const CURRENT_SCHEMA_VERSION: i64 = 15;

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
    SchemaMigration {
        version: 6,
        name: "add_durable_access_model",
        apply: add_durable_access_model,
    },
    SchemaMigration {
        version: 7,
        name: "add_access_aware_corpus_skeleton",
        apply: add_access_aware_corpus_skeleton,
    },
    SchemaMigration {
        version: 8,
        name: "add_policy_decision_audit_trail",
        apply: add_policy_decision_audit_trail,
    },
    SchemaMigration {
        version: 9,
        name: "add_local_install_and_provider_config",
        apply: add_local_install_and_provider_config,
    },
    SchemaMigration {
        version: 10,
        name: "add_business_truth_visibility_publication",
        apply: add_business_truth_visibility_publication,
    },
    SchemaMigration {
        version: 11,
        name: "add_tracked_entry_points_and_visitor_sessions",
        apply: add_tracked_entry_points_and_visitor_sessions,
    },
    SchemaMigration {
        version: 12,
        name: "add_offers_and_trial_lifecycle",
        apply: add_offers_and_trial_lifecycle,
    },
    SchemaMigration {
        version: 13,
        name: "add_connections_foundation",
        apply: add_connections_foundation,
    },
    SchemaMigration {
        version: 14,
        name: "add_availability_and_handoff_inbox",
        apply: add_availability_and_handoff_inbox,
    },
    SchemaMigration {
        version: 15,
        name: "add_report_exports_and_support_packets",
        apply: add_report_exports_and_support_packets,
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

fn add_durable_access_model(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS actors (
            id TEXT PRIMARY KEY,
            actor_kind TEXT NOT NULL,
            display_name TEXT NOT NULL,
            status TEXT NOT NULL,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_actors_kind_status ON actors(actor_kind, status);

        CREATE TABLE IF NOT EXISTS roles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS actor_role_memberships (
            actor_id TEXT NOT NULL,
            role_id TEXT NOT NULL,
            granted_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            PRIMARY KEY (actor_id, role_id),
            FOREIGN KEY (actor_id) REFERENCES actors(id) ON DELETE CASCADE,
            FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
            FOREIGN KEY (granted_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_actor_role_memberships_role ON actor_role_memberships(role_id, actor_id);

        CREATE TABLE IF NOT EXISTS resource_grants (
            id TEXT PRIMARY KEY,
            resource_kind TEXT NOT NULL,
            resource_id TEXT NOT NULL,
            action TEXT NOT NULL,
            subject_kind TEXT NOT NULL,
            subject_id TEXT NOT NULL,
            effect TEXT NOT NULL,
            created_at TEXT NOT NULL,
            expires_at TEXT,
            metadata_json TEXT NOT NULL DEFAULT '{}'
        );

        CREATE INDEX IF NOT EXISTS idx_resource_grants_resource ON resource_grants(resource_kind, resource_id, action);
        CREATE INDEX IF NOT EXISTS idx_resource_grants_subject ON resource_grants(subject_kind, subject_id);
        "#,
    )?;

    seed_access_baseline(connection)
}

fn seed_access_baseline(connection: &Connection) -> Result<()> {
    const SEEDED_AT: &str = "1970-01-01T00:00:00Z";
    connection.execute(
        "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
         VALUES ('actor_system', 'system', 'System', 'active', '{}', ?1, ?1)
         ON CONFLICT(id) DO UPDATE SET actor_kind = excluded.actor_kind, display_name = excluded.display_name, status = excluded.status, updated_at = excluded.updated_at",
        [SEEDED_AT],
    )?;
    connection.execute(
        "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
         VALUES ('actor_local_owner', 'local_owner', 'Local Owner', 'active', '{}', ?1, ?1)
         ON CONFLICT(id) DO UPDATE SET actor_kind = excluded.actor_kind, display_name = excluded.display_name, status = excluded.status, updated_at = excluded.updated_at",
        [SEEDED_AT],
    )?;
    connection.execute(
        "INSERT INTO roles (id, name, description, metadata_json, created_at, updated_at)
         VALUES ('role_system', 'system', 'Internal system authority for local appliance work.', '{}', ?1, ?1)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, description = excluded.description, updated_at = excluded.updated_at",
        [SEEDED_AT],
    )?;
    connection.execute(
        "INSERT INTO roles (id, name, description, metadata_json, created_at, updated_at)
         VALUES ('role_owner', 'owner', 'Local owner authority for this appliance.', '{}', ?1, ?1)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, description = excluded.description, updated_at = excluded.updated_at",
        [SEEDED_AT],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO actor_role_memberships (actor_id, role_id, granted_by_actor_id, created_at)
         VALUES ('actor_system', 'role_system', 'actor_system', ?1)",
        [SEEDED_AT],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO actor_role_memberships (actor_id, role_id, granted_by_actor_id, created_at)
         VALUES ('actor_local_owner', 'role_owner', 'actor_system', ?1)",
        [SEEDED_AT],
    )?;
    seed_resource_grant(
        connection,
        "grant_role_system_all_system",
        "system",
        "*",
        "*",
        "role",
        "role_system",
    )?;
    seed_resource_grant(
        connection,
        "grant_role_owner_all_system",
        "system",
        "*",
        "*",
        "role",
        "role_owner",
    )?;
    seed_resource_grant(
        connection,
        "grant_role_owner_all_owner_system",
        "owner_system",
        "*",
        "*",
        "role",
        "role_owner",
    )?;
    Ok(())
}

fn seed_resource_grant(
    connection: &Connection,
    id: &str,
    resource_kind: &str,
    resource_id: &str,
    action: &str,
    subject_kind: &str,
    subject_id: &str,
) -> Result<()> {
    connection.execute(
        "INSERT OR IGNORE INTO resource_grants (
            id, resource_kind, resource_id, action, subject_kind, subject_id, effect, created_at, expires_at, metadata_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'allow', '1970-01-01T00:00:00Z', NULL, '{}')",
        [id, resource_kind, resource_id, action, subject_kind, subject_id],
    )?;
    Ok(())
}

fn add_access_aware_corpus_skeleton(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS corpus_sources (
            id TEXT PRIMARY KEY,
            source_kind TEXT NOT NULL,
            label TEXT NOT NULL,
            uri TEXT,
            resource_kind TEXT NOT NULL,
            resource_id TEXT NOT NULL,
            status TEXT NOT NULL,
            classification_json TEXT NOT NULL DEFAULT '{}',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_corpus_sources_resource ON corpus_sources(resource_kind, resource_id);
        CREATE INDEX IF NOT EXISTS idx_corpus_sources_status ON corpus_sources(status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS corpus_items (
            id TEXT PRIMARY KEY,
            source_id TEXT NOT NULL,
            item_kind TEXT NOT NULL,
            ordinal INTEGER NOT NULL DEFAULT 0,
            title TEXT NOT NULL,
            body_text TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            resource_kind TEXT NOT NULL,
            resource_id TEXT NOT NULL,
            status TEXT NOT NULL,
            classification_json TEXT NOT NULL DEFAULT '{}',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (source_id) REFERENCES corpus_sources(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_corpus_items_source_ordinal ON corpus_items(source_id, ordinal);
        CREATE INDEX IF NOT EXISTS idx_corpus_items_resource ON corpus_items(resource_kind, resource_id);
        CREATE INDEX IF NOT EXISTS idx_corpus_items_status ON corpus_items(status, updated_at DESC);
        "#,
    )?;

    Ok(())
}

fn add_policy_decision_audit_trail(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS policy_decisions (
            id TEXT PRIMARY KEY,
            decided_at TEXT NOT NULL,
            actor_kind TEXT NOT NULL,
            actor_id TEXT,
            actor_origin TEXT NOT NULL,
            action TEXT NOT NULL,
            resource_kind TEXT NOT NULL,
            resource_id TEXT NOT NULL,
            capability_id TEXT,
            outcome TEXT NOT NULL,
            reason TEXT NOT NULL,
            request_id TEXT,
            job_id TEXT,
            task_key TEXT,
            artifact_id TEXT,
            metadata_json TEXT NOT NULL DEFAULT '{}'
        );

        CREATE INDEX IF NOT EXISTS idx_policy_decisions_time ON policy_decisions(decided_at DESC);
        CREATE INDEX IF NOT EXISTS idx_policy_decisions_outcome_time ON policy_decisions(outcome, decided_at DESC);
        CREATE INDEX IF NOT EXISTS idx_policy_decisions_actor_time ON policy_decisions(actor_kind, actor_id, decided_at DESC);
        CREATE INDEX IF NOT EXISTS idx_policy_decisions_resource_time ON policy_decisions(resource_kind, resource_id, decided_at DESC);
        CREATE INDEX IF NOT EXISTS idx_policy_decisions_capability_time ON policy_decisions(capability_id, decided_at DESC);
        "#,
    )?;

    Ok(())
}

fn add_local_install_and_provider_config(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS appliance_owner (
            id TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            email TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS business_profile (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            workspace_label TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS install_state (
            id TEXT PRIMARY KEY,
            installed INTEGER NOT NULL DEFAULT 0,
            completed_at TEXT,
            owner_id TEXT,
            business_profile_id TEXT,
            default_provider_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (owner_id) REFERENCES appliance_owner(id) ON DELETE SET NULL,
            FOREIGN KEY (business_profile_id) REFERENCES business_profile(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS vault_items (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            label TEXT NOT NULL,
            encrypted_value TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_used_at TEXT,
            metadata_json TEXT NOT NULL DEFAULT '{}'
        );

        CREATE INDEX IF NOT EXISTS idx_vault_items_kind ON vault_items(kind, updated_at DESC);

        CREATE TABLE IF NOT EXISTS provider_configs (
            provider_id TEXT PRIMARY KEY,
            provider_name TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 0,
            default_provider INTEGER NOT NULL DEFAULT 0,
            model TEXT,
            base_url TEXT,
            secret_ref TEXT,
            non_secret_config_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (secret_ref) REFERENCES vault_items(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_provider_configs_enabled ON provider_configs(enabled, provider_id);
        CREATE INDEX IF NOT EXISTS idx_provider_configs_default ON provider_configs(default_provider, provider_id);
        "#,
    )?;

    Ok(())
}

fn add_business_truth_visibility_publication(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS business_facts (
            id TEXT PRIMARY KEY,
            subject_type TEXT NOT NULL,
            subject_id TEXT NOT NULL,
            fact_key TEXT NOT NULL,
            value_json TEXT NOT NULL,
            source_kind TEXT NOT NULL,
            source_label TEXT,
            source_uri TEXT,
            provenance_json TEXT NOT NULL DEFAULT '{}',
            visibility TEXT NOT NULL,
            publication_state TEXT NOT NULL,
            created_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            published_at TEXT,
            archived_at TEXT,
            FOREIGN KEY (created_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_business_facts_subject ON business_facts(subject_type, subject_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_business_facts_key ON business_facts(fact_key, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_business_facts_visibility_publication ON business_facts(visibility, publication_state, updated_at DESC);
        "#,
    )?;

    Ok(())
}

fn add_tracked_entry_points_and_visitor_sessions(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS tracked_entry_points (
            id TEXT PRIMARY KEY,
            slug TEXT NOT NULL UNIQUE,
            label TEXT NOT NULL,
            status TEXT NOT NULL,
            source_kind TEXT NOT NULL,
            source_label TEXT,
            destination_surface TEXT NOT NULL,
            destination_id TEXT,
            public_path TEXT NOT NULL,
            qr_payload_json TEXT NOT NULL DEFAULT '{}',
            attribution_json TEXT NOT NULL DEFAULT '{}',
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            archived_at TEXT,
            FOREIGN KEY (created_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_tracked_entry_points_status ON tracked_entry_points(status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_tracked_entry_points_source ON tracked_entry_points(source_kind, source_label);
        CREATE INDEX IF NOT EXISTS idx_tracked_entry_points_destination ON tracked_entry_points(destination_surface, destination_id);

        CREATE TABLE IF NOT EXISTS visitor_sessions (
            id TEXT PRIMARY KEY,
            entry_point_id TEXT NOT NULL,
            entry_point_slug TEXT NOT NULL,
            status TEXT NOT NULL,
            destination_surface TEXT NOT NULL,
            destination_id TEXT,
            attribution_json TEXT NOT NULL DEFAULT '{}',
            user_agent_hash TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_seen_at TEXT NOT NULL,
            ended_at TEXT,
            FOREIGN KEY (entry_point_id) REFERENCES tracked_entry_points(id) ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_visitor_sessions_entry_point ON visitor_sessions(entry_point_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_visitor_sessions_slug ON visitor_sessions(entry_point_slug, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_visitor_sessions_status ON visitor_sessions(status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS visitor_session_events (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            entry_point_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL DEFAULT '{}',
            occurred_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES visitor_sessions(id) ON DELETE CASCADE,
            FOREIGN KEY (entry_point_id) REFERENCES tracked_entry_points(id) ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_visitor_session_events_session ON visitor_session_events(session_id, occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_visitor_session_events_entry_point ON visitor_session_events(entry_point_id, occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_visitor_session_events_type ON visitor_session_events(event_type, occurred_at DESC);
        "#,
    )?;

    Ok(())
}

fn add_offers_and_trial_lifecycle(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS offers (
            id TEXT PRIMARY KEY,
            slug TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            summary TEXT NOT NULL,
            status TEXT NOT NULL,
            visibility TEXT NOT NULL,
            publication_state TEXT NOT NULL,
            trial_days INTEGER NOT NULL DEFAULT 30,
            source_kind TEXT NOT NULL,
            source_ref TEXT,
            terms_json TEXT NOT NULL DEFAULT '{}',
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            published_at TEXT,
            archived_at TEXT,
            FOREIGN KEY (created_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_offers_public_state ON offers(visibility, publication_state, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_offers_status ON offers(status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_offers_source ON offers(source_kind, source_ref);

        CREATE TABLE IF NOT EXISTS offer_acceptances (
            id TEXT PRIMARY KEY,
            offer_id TEXT NOT NULL,
            offer_slug TEXT NOT NULL,
            offer_title TEXT NOT NULL,
            visitor_session_id TEXT,
            entry_point_id TEXT,
            entry_point_slug TEXT,
            attribution_json TEXT NOT NULL DEFAULT '{}',
            acceptance_context_json TEXT NOT NULL DEFAULT '{}',
            status TEXT NOT NULL,
            accepted_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_offer_acceptances_offer ON offer_acceptances(offer_id, accepted_at DESC);
        CREATE INDEX IF NOT EXISTS idx_offer_acceptances_session ON offer_acceptances(visitor_session_id, accepted_at DESC);
        CREATE INDEX IF NOT EXISTS idx_offer_acceptances_entry_point ON offer_acceptances(entry_point_id, accepted_at DESC);
        CREATE INDEX IF NOT EXISTS idx_offer_acceptances_status ON offer_acceptances(status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS trials (
            id TEXT PRIMARY KEY,
            acceptance_id TEXT NOT NULL,
            offer_id TEXT NOT NULL,
            offer_slug TEXT NOT NULL,
            visitor_session_id TEXT,
            status TEXT NOT NULL,
            started_at TEXT NOT NULL,
            trial_ends_at TEXT NOT NULL,
            converted_at TEXT,
            voided_at TEXT,
            expired_at TEXT,
            follow_up_needed_at TEXT,
            decision_evidence_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (acceptance_id) REFERENCES offer_acceptances(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_trials_acceptance ON trials(acceptance_id);
        CREATE INDEX IF NOT EXISTS idx_trials_offer ON trials(offer_id, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_trials_status ON trials(status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_trials_ends ON trials(trial_ends_at, status);

        CREATE TABLE IF NOT EXISTS trial_events (
            id TEXT PRIMARY KEY,
            trial_id TEXT NOT NULL,
            acceptance_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL DEFAULT '{}',
            occurred_at TEXT NOT NULL,
            FOREIGN KEY (trial_id) REFERENCES trials(id) ON DELETE CASCADE,
            FOREIGN KEY (acceptance_id) REFERENCES offer_acceptances(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_trial_events_trial ON trial_events(trial_id, occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_trial_events_acceptance ON trial_events(acceptance_id, occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_trial_events_type ON trial_events(event_type, occurred_at DESC);
        "#,
    )?;

    Ok(())
}

fn add_connections_foundation(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS connections (
            id TEXT PRIMARY KEY,
            connection_type TEXT NOT NULL,
            display_name TEXT NOT NULL,
            status TEXT NOT NULL,
            identity_json TEXT NOT NULL DEFAULT '{}',
            scope_json TEXT NOT NULL DEFAULT '{}',
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            activated_at TEXT,
            suspended_at TEXT,
            revoked_at TEXT,
            archived_at TEXT,
            FOREIGN KEY (created_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_connections_type_status ON connections(connection_type, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_connections_status ON connections(status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS connection_grants (
            id TEXT PRIMARY KEY,
            connection_id TEXT NOT NULL,
            resource_grant_id TEXT NOT NULL,
            resource_kind TEXT NOT NULL,
            resource_id TEXT NOT NULL,
            action TEXT NOT NULL,
            status TEXT NOT NULL,
            expires_at TEXT,
            grant_reason TEXT,
            created_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            revoked_at TEXT,
            revoked_by_actor_id TEXT,
            revocation_reason TEXT,
            FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE CASCADE,
            FOREIGN KEY (resource_grant_id) REFERENCES resource_grants(id) ON DELETE CASCADE,
            FOREIGN KEY (created_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL,
            FOREIGN KEY (revoked_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_connection_grants_connection ON connection_grants(connection_id, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_connection_grants_resource ON connection_grants(resource_kind, resource_id, action, status);
        CREATE INDEX IF NOT EXISTS idx_connection_grants_expiry ON connection_grants(expires_at, status);

        CREATE TABLE IF NOT EXISTS connection_events (
            id TEXT PRIMARY KEY,
            connection_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL DEFAULT '{}',
            occurred_at TEXT NOT NULL,
            FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_connection_events_connection ON connection_events(connection_id, occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_connection_events_type ON connection_events(event_type, occurred_at DESC);

        CREATE TABLE IF NOT EXISTS connection_receipts (
            id TEXT PRIMARY KEY,
            connection_id TEXT NOT NULL,
            event_id TEXT NOT NULL,
            receipt_kind TEXT NOT NULL,
            payload_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE CASCADE,
            FOREIGN KEY (event_id) REFERENCES connection_events(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_connection_receipts_connection ON connection_receipts(connection_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_connection_receipts_event ON connection_receipts(event_id);
        "#,
    )?;

    Ok(())
}

fn add_availability_and_handoff_inbox(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS availability_schedules (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            timezone TEXT NOT NULL,
            status TEXT NOT NULL,
            windows_json TEXT NOT NULL DEFAULT '[]',
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_availability_schedules_status ON availability_schedules(status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS operator_presence (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            threshold TEXT NOT NULL,
            status_message TEXT,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            updated_by_actor_id TEXT,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (updated_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_operator_presence_status ON operator_presence(status, threshold, updated_at DESC);

        CREATE TABLE IF NOT EXISTS handoff_eligibility_decisions (
            id TEXT PRIMARY KEY,
            intent TEXT NOT NULL,
            connection_id TEXT,
            connection_trust TEXT NOT NULL,
            allowed INTEGER NOT NULL,
            reason TEXT NOT NULL,
            evidence_json TEXT NOT NULL DEFAULT '{}',
            decided_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_handoff_eligibility_decisions_time ON handoff_eligibility_decisions(decided_at DESC);
        CREATE INDEX IF NOT EXISTS idx_handoff_eligibility_decisions_connection ON handoff_eligibility_decisions(connection_id, decided_at DESC);

        CREATE TABLE IF NOT EXISTS handoff_inbox_items (
            id TEXT PRIMARY KEY,
            source_kind TEXT NOT NULL,
            source_id TEXT,
            destination_kind TEXT NOT NULL,
            destination_id TEXT,
            request_json TEXT NOT NULL DEFAULT '{}',
            evidence_json TEXT NOT NULL DEFAULT '{}',
            approval_requirement TEXT NOT NULL,
            delivery_state TEXT NOT NULL,
            owner_decision TEXT,
            decision_reason TEXT,
            created_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            resolved_at TEXT,
            FOREIGN KEY (created_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_handoff_inbox_items_state ON handoff_inbox_items(delivery_state, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_handoff_inbox_items_source ON handoff_inbox_items(source_kind, source_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_handoff_inbox_items_destination ON handoff_inbox_items(destination_kind, destination_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS handoff_events (
            id TEXT PRIMARY KEY,
            handoff_item_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL DEFAULT '{}',
            occurred_at TEXT NOT NULL,
            FOREIGN KEY (handoff_item_id) REFERENCES handoff_inbox_items(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_handoff_events_item ON handoff_events(handoff_item_id, occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_handoff_events_type ON handoff_events(event_type, occurred_at DESC);

        CREATE TABLE IF NOT EXISTS handoff_receipts (
            id TEXT PRIMARY KEY,
            handoff_item_id TEXT NOT NULL,
            event_id TEXT NOT NULL,
            receipt_kind TEXT NOT NULL,
            payload_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            FOREIGN KEY (handoff_item_id) REFERENCES handoff_inbox_items(id) ON DELETE CASCADE,
            FOREIGN KEY (event_id) REFERENCES handoff_events(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_handoff_receipts_item ON handoff_receipts(handoff_item_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_handoff_receipts_event ON handoff_receipts(event_id);
        "#,
    )?;

    Ok(())
}

fn add_report_exports_and_support_packets(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS issue_report_exports (
            id TEXT PRIMARY KEY,
            report_id TEXT NOT NULL,
            export_format TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            content_bytes INTEGER NOT NULL,
            content_text TEXT NOT NULL,
            created_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (report_id) REFERENCES issue_report_artifacts(id) ON DELETE CASCADE,
            FOREIGN KEY (created_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_issue_report_exports_report ON issue_report_exports(report_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS issue_report_status_events (
            id TEXT PRIMARY KEY,
            report_id TEXT NOT NULL,
            from_status TEXT,
            to_status TEXT NOT NULL,
            reason TEXT,
            created_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (report_id) REFERENCES issue_report_artifacts(id) ON DELETE CASCADE,
            FOREIGN KEY (created_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_issue_report_status_events_report ON issue_report_status_events(report_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS support_packets (
            id TEXT PRIMARY KEY,
            report_id TEXT NOT NULL,
            status TEXT NOT NULL,
            destination_kind TEXT NOT NULL,
            destination_id TEXT,
            destination_label TEXT,
            payload_json TEXT NOT NULL DEFAULT '{}',
            payload_hash TEXT NOT NULL,
            approval_required INTEGER NOT NULL,
            approved_by_actor_id TEXT,
            approved_at TEXT,
            created_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (report_id) REFERENCES issue_report_artifacts(id) ON DELETE CASCADE,
            FOREIGN KEY (approved_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL,
            FOREIGN KEY (created_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_support_packets_report ON support_packets(report_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_support_packets_status ON support_packets(status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS support_packet_receipts (
            id TEXT PRIMARY KEY,
            packet_id TEXT NOT NULL,
            receipt_kind TEXT NOT NULL,
            payload_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            FOREIGN KEY (packet_id) REFERENCES support_packets(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_support_packet_receipts_packet ON support_packet_receipts(packet_id, created_at DESC);
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
        assert_eq!(
            versions,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
        assert_eq!(CURRENT_SCHEMA_VERSION, 15);
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
        assert!(table_exists(&connection, "actors"));
        assert!(table_exists(&connection, "roles"));
        assert!(table_exists(&connection, "actor_role_memberships"));
        assert!(table_exists(&connection, "resource_grants"));
        assert!(table_exists(&connection, "policy_decisions"));
        assert!(table_exists(&connection, "install_state"));
        assert!(table_exists(&connection, "appliance_owner"));
        assert!(table_exists(&connection, "business_profile"));
        assert!(table_exists(&connection, "provider_configs"));
        assert!(table_exists(&connection, "business_facts"));
        assert!(table_exists(&connection, "corpus_sources"));
        assert!(table_exists(&connection, "corpus_items"));
        assert!(table_exists(&connection, "tracked_entry_points"));
        assert!(table_exists(&connection, "visitor_sessions"));
        assert!(table_exists(&connection, "visitor_session_events"));
        assert!(table_exists(&connection, "offers"));
        assert!(table_exists(&connection, "offer_acceptances"));
        assert!(table_exists(&connection, "trials"));
        assert!(table_exists(&connection, "trial_events"));
        assert!(table_exists(&connection, "connections"));
        assert!(table_exists(&connection, "connection_grants"));
        assert!(table_exists(&connection, "connection_events"));
        assert!(table_exists(&connection, "connection_receipts"));
        assert!(table_exists(&connection, "availability_schedules"));
        assert!(table_exists(&connection, "operator_presence"));
        assert!(table_exists(&connection, "handoff_eligibility_decisions"));
        assert!(table_exists(&connection, "handoff_inbox_items"));
        assert!(table_exists(&connection, "handoff_events"));
        assert!(table_exists(&connection, "handoff_receipts"));
        assert!(table_exists(&connection, "issue_report_exports"));
        assert!(table_exists(&connection, "issue_report_status_events"));
        assert!(table_exists(&connection, "support_packets"));
        assert!(table_exists(&connection, "support_packet_receipts"));

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

    #[test]
    fn durable_access_baseline_is_seeded() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        let owner_roles: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM actor_role_memberships
                 WHERE actor_id = 'actor_local_owner' AND role_id = 'role_owner'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let system_roles: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM actor_role_memberships
                 WHERE actor_id = 'actor_system' AND role_id = 'role_system'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let owner_grants: i64 = connection
            .query_row(
                "SELECT COUNT(*)
                 FROM resource_grants
                 WHERE subject_kind = 'role' AND subject_id = 'role_owner' AND resource_kind = 'owner_system'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(owner_roles, 1);
        assert_eq!(system_roles, 1);
        assert_eq!(owner_grants, 1);
    }

    #[test]
    fn corpus_skeleton_stores_access_classification_and_provenance_metadata() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        connection
            .execute(
                "INSERT INTO corpus_sources (
                    id, source_kind, label, uri, resource_kind, resource_id, status,
                    classification_json, provenance_json, metadata_json, created_at, updated_at
                 ) VALUES (
                    'corpus_source_owner_manual', 'markdown', 'Owner Manual', 'file://owner.md',
                    'owner_system', 'knowledge_owner_manual', 'approved',
                    '{\"visibility\":\"owner_system\"}',
                    '{\"actor\":{\"id\":\"actor_local_owner\"}}',
                    '{\"note\":\"retrieval safety prep only\"}', 'now', 'now'
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO corpus_items (
                    id, source_id, item_kind, ordinal, title, body_text, content_hash,
                    resource_kind, resource_id, status, classification_json, provenance_json,
                    metadata_json, created_at, updated_at
                 ) VALUES (
                    'corpus_item_owner_manual_1', 'corpus_source_owner_manual', 'chunk', 1,
                    'Owner Manual Chunk', 'Local owner-only operating guidance.', 'sha256:test',
                    'owner_system', 'knowledge_owner_manual', 'approved',
                    '{\"visibility\":\"owner_system\"}',
                    '{\"resource\":{\"kind\":\"corpus_item\"}}',
                    '{\"embedding\":\"not_present\"}', 'now', 'now'
                 )",
                [],
            )
            .unwrap();

        let metadata: String = connection
            .query_row(
                "SELECT metadata_json FROM corpus_items WHERE id = 'corpus_item_owner_manual_1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let classification: String = connection
            .query_row(
                "SELECT classification_json FROM corpus_sources WHERE id = 'corpus_source_owner_manual'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(metadata, "{\"embedding\":\"not_present\"}");
        assert_eq!(classification, "{\"visibility\":\"owner_system\"}");
    }

    #[test]
    fn local_install_and_provider_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "install_state"));
        assert!(table_exists(&connection, "appliance_owner"));
        assert!(table_exists(&connection, "business_profile"));
        assert!(table_exists(&connection, "vault_items"));
        assert!(table_exists(&connection, "provider_configs"));
        assert!(column_exists(&connection, "provider_configs", "secret_ref"));
        assert!(!column_exists(
            &connection,
            "provider_configs",
            "api_key_secret"
        ));
        assert!(column_exists(
            &connection,
            "provider_configs",
            "non_secret_config_json"
        ));
    }

    #[test]
    fn business_truth_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "business_facts"));
        assert!(column_exists(&connection, "business_facts", "value_json"));
        assert!(column_exists(
            &connection,
            "business_facts",
            "provenance_json"
        ));
        assert!(column_exists(&connection, "business_facts", "visibility"));
        assert!(column_exists(
            &connection,
            "business_facts",
            "publication_state"
        ));
    }

    #[test]
    fn tracked_entry_point_and_visitor_session_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "tracked_entry_points"));
        assert!(column_exists(&connection, "tracked_entry_points", "slug"));
        assert!(column_exists(
            &connection,
            "tracked_entry_points",
            "destination_surface"
        ));
        assert!(column_exists(
            &connection,
            "tracked_entry_points",
            "qr_payload_json"
        ));
        assert!(table_exists(&connection, "visitor_sessions"));
        assert!(column_exists(
            &connection,
            "visitor_sessions",
            "entry_point_id"
        ));
        assert!(column_exists(
            &connection,
            "visitor_sessions",
            "attribution_json"
        ));
        assert!(table_exists(&connection, "visitor_session_events"));
        assert!(column_exists(
            &connection,
            "visitor_session_events",
            "event_type"
        ));
    }

    #[test]
    fn offer_and_trial_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "offers"));
        assert!(column_exists(&connection, "offers", "slug"));
        assert!(column_exists(&connection, "offers", "trial_days"));
        assert!(column_exists(&connection, "offers", "publication_state"));
        assert!(table_exists(&connection, "offer_acceptances"));
        assert!(column_exists(
            &connection,
            "offer_acceptances",
            "visitor_session_id"
        ));
        assert!(column_exists(
            &connection,
            "offer_acceptances",
            "attribution_json"
        ));
        assert!(table_exists(&connection, "trials"));
        assert!(column_exists(&connection, "trials", "trial_ends_at"));
        assert!(column_exists(
            &connection,
            "trials",
            "decision_evidence_json"
        ));
        assert!(table_exists(&connection, "trial_events"));
        assert!(column_exists(&connection, "trial_events", "event_type"));
    }

    #[test]
    fn connection_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "connections"));
        assert!(column_exists(&connection, "connections", "connection_type"));
        assert!(column_exists(&connection, "connections", "identity_json"));
        assert!(column_exists(&connection, "connections", "scope_json"));
        assert!(table_exists(&connection, "connection_grants"));
        assert!(column_exists(
            &connection,
            "connection_grants",
            "resource_grant_id"
        ));
        assert!(column_exists(
            &connection,
            "connection_grants",
            "expires_at"
        ));
        assert!(column_exists(
            &connection,
            "connection_grants",
            "revoked_at"
        ));
        assert!(table_exists(&connection, "connection_events"));
        assert!(column_exists(
            &connection,
            "connection_events",
            "event_type"
        ));
        assert!(table_exists(&connection, "connection_receipts"));
        assert!(column_exists(
            &connection,
            "connection_receipts",
            "receipt_kind"
        ));
    }

    #[test]
    fn availability_and_handoff_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "availability_schedules"));
        assert!(column_exists(
            &connection,
            "availability_schedules",
            "windows_json"
        ));
        assert!(table_exists(&connection, "operator_presence"));
        assert!(column_exists(&connection, "operator_presence", "threshold"));
        assert!(table_exists(&connection, "handoff_eligibility_decisions"));
        assert!(column_exists(
            &connection,
            "handoff_eligibility_decisions",
            "evidence_json"
        ));
        assert!(table_exists(&connection, "handoff_inbox_items"));
        assert!(column_exists(
            &connection,
            "handoff_inbox_items",
            "approval_requirement"
        ));
        assert!(column_exists(
            &connection,
            "handoff_inbox_items",
            "delivery_state"
        ));
        assert!(table_exists(&connection, "handoff_events"));
        assert!(column_exists(&connection, "handoff_events", "event_type"));
        assert!(table_exists(&connection, "handoff_receipts"));
        assert!(column_exists(
            &connection,
            "handoff_receipts",
            "receipt_kind"
        ));
    }

    #[test]
    fn report_export_and_support_packet_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "issue_report_exports"));
        assert!(column_exists(
            &connection,
            "issue_report_exports",
            "content_hash"
        ));
        assert!(column_exists(
            &connection,
            "issue_report_exports",
            "content_text"
        ));
        assert!(table_exists(&connection, "issue_report_status_events"));
        assert!(column_exists(
            &connection,
            "issue_report_status_events",
            "to_status"
        ));
        assert!(table_exists(&connection, "support_packets"));
        assert!(column_exists(
            &connection,
            "support_packets",
            "approval_required"
        ));
        assert!(column_exists(
            &connection,
            "support_packets",
            "payload_hash"
        ));
        assert!(table_exists(&connection, "support_packet_receipts"));
        assert!(column_exists(
            &connection,
            "support_packet_receipts",
            "receipt_kind"
        ));
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
