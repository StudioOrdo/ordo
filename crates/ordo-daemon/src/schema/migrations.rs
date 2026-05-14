use anyhow::{bail, Result};
use rusqlite::Connection;

use super::*;
pub(crate) type MigrationFn = fn(&Connection) -> Result<()>;

pub(crate) struct SchemaMigration {
    pub(crate) version: i64,
    pub(crate) name: &'static str,
    pub(crate) apply: MigrationFn,
}

pub(crate) const MIGRATIONS: &[SchemaMigration] = &[
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
    SchemaMigration {
        version: 16,
        name: "add_corpus_fts_retrieval_index",
        apply: add_corpus_fts_retrieval_index,
    },
    SchemaMigration {
        version: 17,
        name: "add_answer_draft_spine",
        apply: add_answer_draft_spine,
    },
    SchemaMigration {
        version: 18,
        name: "add_mcp_pack_hardening",
        apply: add_mcp_pack_hardening,
    },
    SchemaMigration {
        version: 19,
        name: "add_conversation_product_foundation",
        apply: add_conversation_product_foundation,
    },
    SchemaMigration {
        version: 20,
        name: "add_conversation_message_protocol_schema",
        apply: add_conversation_message_protocol_schema,
    },
    SchemaMigration {
        version: 21,
        name: "add_llm_token_ledger_schema",
        apply: add_llm_token_ledger_schema,
    },
    SchemaMigration {
        version: 22,
        name: "add_conversation_analysis_foundation",
        apply: add_conversation_analysis_foundation,
    },
    SchemaMigration {
        version: 23,
        name: "add_knowledge_graph_candidate_schema",
        apply: add_knowledge_graph_candidate_schema,
    },
    SchemaMigration {
        version: 24,
        name: "add_business_outcome_attribution_schema",
        apply: add_business_outcome_attribution_schema,
    },
    SchemaMigration {
        version: 25,
        name: "add_artifact_deliverable_contract_schema",
        apply: add_artifact_deliverable_contract_schema,
    },
    SchemaMigration {
        version: 26,
        name: "add_surface_brief_schema",
        apply: add_surface_brief_schema,
    },
    SchemaMigration {
        version: 27,
        name: "add_customer_feedback_review_schema",
        apply: add_customer_feedback_review_schema,
    },
    SchemaMigration {
        version: 28,
        name: "add_actor_experience_preferences_schema",
        apply: add_actor_experience_preferences_schema,
    },
    SchemaMigration {
        version: 29,
        name: "add_local_account_sessions_schema",
        apply: add_local_account_sessions_schema,
    },
    SchemaMigration {
        version: 30,
        name: "add_surface_work_item_projection_schema",
        apply: add_surface_work_item_projection_schema,
    },
    SchemaMigration {
        version: 31,
        name: "add_support_handoff_queue_fields",
        apply: add_support_handoff_queue_fields,
    },
    SchemaMigration {
        version: 32,
        name: "add_offer_acceptance_access_receipts",
        apply: add_offer_acceptance_access_receipts,
    },
    SchemaMigration {
        version: 33,
        name: "add_hosted_trial_capacity_lifecycle",
        apply: add_hosted_trial_capacity_lifecycle,
    },
    SchemaMigration {
        version: 34,
        name: "add_feedback_request_loop",
        apply: add_feedback_request_loop,
    },
    SchemaMigration {
        version: 35,
        name: "add_reward_ledger_and_benefits",
        apply: add_reward_ledger_and_benefits,
    },
    SchemaMigration {
        version: 36,
        name: "add_product_pack_manifest_spine",
        apply: add_product_pack_manifest_spine,
    },
    SchemaMigration {
        version: 37,
        name: "add_artifact_patch_proposal_spine",
        apply: add_artifact_patch_proposal_spine,
    },
    SchemaMigration {
        version: 38,
        name: "add_cron_schedule_expression",
        apply: add_cron_schedule_expression,
    },
    SchemaMigration {
        version: 39,
        name: "add_job_task_lease_state",
        apply: add_job_task_lease_state,
    },
    SchemaMigration {
        version: 40,
        name: "add_compiled_plan_snapshots",
        apply: add_compiled_plan_snapshots,
    },
];

pub(crate) fn validate_migration_order() -> Result<()> {
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
            cron_expression TEXT,
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

fn add_local_account_sessions_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS local_account_sessions (
            session_id TEXT PRIMARY KEY,
            actor_id TEXT NOT NULL,
            email_hash TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            role TEXT NOT NULL,
            issued_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            last_seen_at TEXT NOT NULL,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            FOREIGN KEY (actor_id) REFERENCES actors(id)
        );

        CREATE INDEX IF NOT EXISTS idx_local_account_sessions_actor
            ON local_account_sessions(actor_id);
        CREATE INDEX IF NOT EXISTS idx_local_account_sessions_expires
            ON local_account_sessions(expires_at);
        "#,
    )?;
    Ok(())
}

fn add_surface_work_item_projection_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS surface_work_items (
            id TEXT PRIMARY KEY,
            surface_kind TEXT NOT NULL,
            room_kind TEXT NOT NULL,
            source_kind TEXT NOT NULL,
            source_id TEXT NOT NULL,
            object_kind TEXT NOT NULL,
            object_id TEXT NOT NULL,
            title TEXT NOT NULL,
            summary TEXT NOT NULL,
            status TEXT NOT NULL,
            priority INTEGER NOT NULL DEFAULT 0,
            actor_context_json TEXT NOT NULL DEFAULT '{}',
            connection_context_json TEXT NOT NULL DEFAULT '{}',
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            actions_json TEXT NOT NULL DEFAULT '[]',
            visibility TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            projected_at TEXT NOT NULL,
            UNIQUE(surface_kind, source_kind, source_id)
        );

        CREATE INDEX IF NOT EXISTS idx_surface_work_items_surface_room_priority
            ON surface_work_items(surface_kind, room_kind, priority DESC, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_surface_work_items_source
            ON surface_work_items(source_kind, source_id);
        CREATE INDEX IF NOT EXISTS idx_surface_work_items_visibility
            ON surface_work_items(visibility, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_surface_work_items_status
            ON surface_work_items(status, updated_at DESC);
        "#,
    )?;
    Ok(())
}

fn add_support_handoff_queue_fields(connection: &Connection) -> Result<()> {
    ensure_column(
        connection,
        "handoff_inbox_items",
        "reason",
        "TEXT NOT NULL DEFAULT 'Support review requested'",
    )?;
    ensure_column(
        connection,
        "handoff_inbox_items",
        "requested_action",
        "TEXT NOT NULL DEFAULT 'review'",
    )?;
    ensure_column(
        connection,
        "handoff_inbox_items",
        "urgency",
        "TEXT NOT NULL DEFAULT 'normal'",
    )?;
    ensure_column(
        connection,
        "handoff_inbox_items",
        "assignee_actor_id",
        "TEXT",
    )?;
    ensure_column(connection, "handoff_inbox_items", "due_at", "TEXT")?;
    ensure_column(
        connection,
        "handoff_inbox_items",
        "next_action_hint",
        "TEXT",
    )?;
    ensure_column(
        connection,
        "handoff_inbox_items",
        "evidence_refs_json",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(
        connection,
        "handoff_inbox_items",
        "visibility",
        "TEXT NOT NULL DEFAULT 'staff'",
    )?;
    connection.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_handoff_inbox_items_assignee
            ON handoff_inbox_items(assignee_actor_id, delivery_state, urgency, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_handoff_inbox_items_visibility
            ON handoff_inbox_items(visibility, updated_at DESC);
        "#,
    )?;
    Ok(())
}

fn add_offer_acceptance_access_receipts(connection: &Connection) -> Result<()> {
    ensure_column(connection, "offer_acceptances", "idempotency_key", "TEXT")?;
    ensure_column(connection, "offer_acceptances", "access_grant_id", "TEXT")?;
    ensure_column(
        connection,
        "offer_acceptances",
        "receipt_json",
        "TEXT NOT NULL DEFAULT '{}'",
    )?;
    connection.execute_batch(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS idx_offer_acceptances_idempotency
            ON offer_acceptances(offer_id, idempotency_key)
            WHERE idempotency_key IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_offer_acceptances_access_grant
            ON offer_acceptances(access_grant_id);
        "#,
    )?;
    Ok(())
}

fn add_hosted_trial_capacity_lifecycle(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS hosted_trial_capacity_policies (
            id TEXT PRIMARY KEY,
            offer_id TEXT NOT NULL,
            offer_slug TEXT NOT NULL,
            status TEXT NOT NULL,
            active_slot_limit INTEGER NOT NULL,
            trial_days INTEGER NOT NULL,
            backup_before_wipe_required INTEGER NOT NULL DEFAULT 1,
            reset_grace_days INTEGER NOT NULL DEFAULT 0,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(offer_id)
        );

        CREATE INDEX IF NOT EXISTS idx_hosted_trial_capacity_policies_slug
            ON hosted_trial_capacity_policies(offer_slug, status);

        CREATE TABLE IF NOT EXISTS hosted_trial_slots (
            id TEXT PRIMARY KEY,
            policy_id TEXT NOT NULL,
            trial_id TEXT NOT NULL UNIQUE,
            acceptance_id TEXT NOT NULL UNIQUE,
            offer_id TEXT NOT NULL,
            offer_slug TEXT NOT NULL,
            subject_kind TEXT NOT NULL,
            subject_id TEXT NOT NULL,
            status TEXT NOT NULL,
            allocated_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            released_at TEXT,
            release_reason TEXT,
            backup_required INTEGER NOT NULL DEFAULT 1,
            backup_status TEXT NOT NULL DEFAULT 'required',
            backup_evidence_json TEXT NOT NULL DEFAULT '[]',
            reset_eligible_at TEXT,
            reset_state TEXT NOT NULL DEFAULT 'blocked_until_expiration',
            reset_guard_json TEXT NOT NULL DEFAULT '{}',
            owner_override_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (policy_id) REFERENCES hosted_trial_capacity_policies(id) ON DELETE CASCADE,
            FOREIGN KEY (trial_id) REFERENCES trials(id) ON DELETE CASCADE,
            FOREIGN KEY (acceptance_id) REFERENCES offer_acceptances(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_hosted_trial_slots_policy_status
            ON hosted_trial_slots(policy_id, status, expires_at);
        CREATE INDEX IF NOT EXISTS idx_hosted_trial_slots_subject
            ON hosted_trial_slots(subject_kind, subject_id, status);
        CREATE INDEX IF NOT EXISTS idx_hosted_trial_slots_reset
            ON hosted_trial_slots(reset_state, updated_at DESC);

        CREATE TABLE IF NOT EXISTS hosted_trial_waitlist_entries (
            id TEXT PRIMARY KEY,
            policy_id TEXT NOT NULL,
            acceptance_id TEXT NOT NULL UNIQUE,
            offer_id TEXT NOT NULL,
            offer_slug TEXT NOT NULL,
            visitor_session_id TEXT,
            subject_kind TEXT NOT NULL,
            subject_id TEXT NOT NULL,
            status TEXT NOT NULL,
            position INTEGER NOT NULL,
            reason TEXT NOT NULL,
            receipt_json TEXT NOT NULL DEFAULT '{}',
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (policy_id) REFERENCES hosted_trial_capacity_policies(id) ON DELETE CASCADE,
            FOREIGN KEY (acceptance_id) REFERENCES offer_acceptances(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_hosted_trial_waitlist_policy_status
            ON hosted_trial_waitlist_entries(policy_id, status, position);
        CREATE INDEX IF NOT EXISTS idx_hosted_trial_waitlist_offer
            ON hosted_trial_waitlist_entries(offer_id, status, position);
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

fn add_corpus_fts_retrieval_index(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS corpus_items_fts USING fts5(
            item_id UNINDEXED,
            title,
            body_text,
            tokenize = 'unicode61'
        );
        "#,
    )?;

    Ok(())
}

fn add_answer_draft_spine(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS answer_drafts (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            question TEXT NOT NULL,
            prompt_input_json TEXT NOT NULL DEFAULT '{}',
            retrieval_query_json TEXT NOT NULL DEFAULT '{}',
            retrieval_evidence_json TEXT NOT NULL DEFAULT '{}',
            cited_item_ids_json TEXT NOT NULL DEFAULT '[]',
            draft_markdown TEXT NOT NULL,
            limitations_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            created_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (created_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_answer_drafts_status_updated ON answer_drafts(status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_answer_drafts_created ON answer_drafts(created_at DESC);

        CREATE TABLE IF NOT EXISTS answer_draft_citations (
            id TEXT PRIMARY KEY,
            draft_id TEXT NOT NULL,
            corpus_item_id TEXT NOT NULL,
            corpus_source_id TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            rank REAL NOT NULL,
            snippet TEXT NOT NULL,
            evidence_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            FOREIGN KEY (draft_id) REFERENCES answer_drafts(id) ON DELETE CASCADE,
            FOREIGN KEY (corpus_item_id) REFERENCES corpus_items(id) ON DELETE CASCADE,
            FOREIGN KEY (corpus_source_id) REFERENCES corpus_sources(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_answer_draft_citations_draft ON answer_draft_citations(draft_id, rank);
        CREATE INDEX IF NOT EXISTS idx_answer_draft_citations_item ON answer_draft_citations(corpus_item_id);
        "#,
    )?;

    Ok(())
}

fn add_mcp_pack_hardening(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS mcp_packs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            status TEXT NOT NULL,
            manifest_json TEXT NOT NULL DEFAULT '{}',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            created_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (created_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_mcp_packs_status_updated ON mcp_packs(status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS mcp_pack_tools (
            id TEXT PRIMARY KEY,
            pack_id TEXT NOT NULL,
            tool_name TEXT NOT NULL,
            capability_id TEXT NOT NULL,
            input_schema_json TEXT NOT NULL DEFAULT '{}',
            output_contract_json TEXT NOT NULL DEFAULT '{}',
            side_effects_json TEXT NOT NULL DEFAULT '[]',
            approval_requirement TEXT NOT NULL,
            artifact_kinds_json TEXT NOT NULL DEFAULT '[]',
            mcp_export_policy TEXT NOT NULL,
            export_status TEXT NOT NULL,
            disabled_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (pack_id) REFERENCES mcp_packs(id) ON DELETE CASCADE,
            FOREIGN KEY (capability_id) REFERENCES capabilities(id) ON DELETE CASCADE,
            UNIQUE(pack_id, tool_name),
            UNIQUE(pack_id, capability_id)
        );

        CREATE INDEX IF NOT EXISTS idx_mcp_pack_tools_pack ON mcp_pack_tools(pack_id, tool_name);
        CREATE INDEX IF NOT EXISTS idx_mcp_pack_tools_capability ON mcp_pack_tools(capability_id, export_status);
        "#,
    )?;

    Ok(())
}

fn add_conversation_product_foundation(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            surface TEXT NOT NULL,
            subject_kind TEXT NOT NULL,
            subject_id TEXT NOT NULL,
            connection_id TEXT,
            visitor_session_id TEXT,
            status TEXT NOT NULL,
            visibility TEXT NOT NULL,
            privacy_scope TEXT NOT NULL,
            current_segment_id TEXT,
            last_meaningful_change TEXT NOT NULL DEFAULT '',
            unread_count INTEGER NOT NULL DEFAULT 0,
            action_count INTEGER NOT NULL DEFAULT 0,
            summary_json TEXT NOT NULL DEFAULT '{}',
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            closed_at TEXT,
            archived_at TEXT,
            FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE SET NULL,
            FOREIGN KEY (visitor_session_id) REFERENCES visitor_sessions(id) ON DELETE SET NULL,
            FOREIGN KEY (created_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_active_subject
            ON conversations(surface, subject_kind, subject_id)
            WHERE archived_at IS NULL;
        CREATE INDEX IF NOT EXISTS idx_conversations_connection ON conversations(connection_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversations_visitor_session ON conversations(visitor_session_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversations_status_updated ON conversations(status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS conversation_segments (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            segment_kind TEXT NOT NULL,
            title TEXT NOT NULL,
            status TEXT NOT NULL,
            candidate_state TEXT NOT NULL,
            confidence REAL NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            created_by_job_id TEXT,
            source_kind TEXT NOT NULL DEFAULT '',
            source_id TEXT NOT NULL DEFAULT '',
            started_at TEXT NOT NULL,
            ended_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (created_by_job_id) REFERENCES jobs(id) ON DELETE SET NULL
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_conversation_segments_idempotency
            ON conversation_segments(conversation_id, segment_kind, source_kind, source_id, created_by_job_id)
            WHERE source_kind <> '' AND source_id <> '' AND created_by_job_id IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_conversation_segments_conversation ON conversation_segments(conversation_id, started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_segments_kind_status ON conversation_segments(segment_kind, status, started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_segments_candidate ON conversation_segments(candidate_state, updated_at DESC);

        CREATE TABLE IF NOT EXISTS conversation_handoffs (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            segment_id TEXT,
            connection_id TEXT,
            requested_by_actor_id TEXT,
            assigned_to_actor_id TEXT,
            reason TEXT NOT NULL,
            urgency TEXT NOT NULL,
            required_capability_id TEXT NOT NULL,
            evidence_summary TEXT NOT NULL,
            allowed_context_json TEXT NOT NULL DEFAULT '[]',
            status TEXT NOT NULL,
            policy_decision_id TEXT,
            receipt_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            closed_at TEXT,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (segment_id) REFERENCES conversation_segments(id) ON DELETE SET NULL,
            FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE SET NULL,
            FOREIGN KEY (requested_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL,
            FOREIGN KEY (assigned_to_actor_id) REFERENCES actors(id) ON DELETE SET NULL,
            FOREIGN KEY (required_capability_id) REFERENCES capabilities(id) ON DELETE RESTRICT,
            FOREIGN KEY (policy_decision_id) REFERENCES policy_decisions(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_handoffs_assigned
            ON conversation_handoffs(assigned_to_actor_id, status, urgency, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_handoffs_conversation
            ON conversation_handoffs(conversation_id, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_handoffs_connection
            ON conversation_handoffs(connection_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS conversation_modes (
            conversation_id TEXT PRIMARY KEY,
            mode TEXT NOT NULL,
            led_by_actor_id TEXT,
            delegated_to_agent INTEGER NOT NULL DEFAULT 0,
            delegation_scope_json TEXT NOT NULL DEFAULT '[]',
            idle_after TEXT,
            private_reminder_sent_at TEXT,
            last_public_agent_message_at TEXT,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (led_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_modes_mode ON conversation_modes(mode, updated_at DESC);

        CREATE TABLE IF NOT EXISTS conversation_events (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            segment_id TEXT,
            handoff_id TEXT,
            sequence INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL DEFAULT '{}',
            policy_decision_id TEXT,
            realtime_cursor INTEGER,
            occurred_at TEXT NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (segment_id) REFERENCES conversation_segments(id) ON DELETE SET NULL,
            FOREIGN KEY (handoff_id) REFERENCES conversation_handoffs(id) ON DELETE SET NULL,
            FOREIGN KEY (policy_decision_id) REFERENCES policy_decisions(id) ON DELETE SET NULL,
            UNIQUE(conversation_id, sequence)
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_events_conversation
            ON conversation_events(conversation_id, sequence);
        CREATE INDEX IF NOT EXISTS idx_conversation_events_handoff
            ON conversation_events(handoff_id, occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_events_realtime
            ON conversation_events(realtime_cursor);
        "#,
    )?;

    Ok(())
}

fn add_conversation_message_protocol_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS conversation_participants (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            participant_kind TEXT NOT NULL,
            actor_id TEXT,
            connection_id TEXT,
            visitor_session_id TEXT,
            display_name TEXT NOT NULL,
            role TEXT NOT NULL,
            status TEXT NOT NULL,
            privacy_settings_json TEXT NOT NULL DEFAULT '{}',
            metadata_json TEXT NOT NULL DEFAULT '{}',
            joined_at TEXT NOT NULL,
            last_seen_at TEXT,
            left_at TEXT,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (actor_id) REFERENCES actors(id) ON DELETE SET NULL,
            FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE SET NULL,
            FOREIGN KEY (visitor_session_id) REFERENCES visitor_sessions(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_participants_conversation
            ON conversation_participants(conversation_id, status);
        CREATE INDEX IF NOT EXISTS idx_conversation_participants_actor
            ON conversation_participants(actor_id, conversation_id);
        CREATE INDEX IF NOT EXISTS idx_conversation_participants_connection
            ON conversation_participants(connection_id, conversation_id);
        CREATE INDEX IF NOT EXISTS idx_conversation_participants_visitor_session
            ON conversation_participants(visitor_session_id, conversation_id);

        CREATE TABLE IF NOT EXISTS conversation_messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            segment_id TEXT,
            participant_id TEXT NOT NULL,
            message_kind TEXT NOT NULL,
            status TEXT NOT NULL,
            body_markdown TEXT NOT NULL,
            body_format TEXT NOT NULL DEFAULT 'markdown',
            redaction_state TEXT NOT NULL,
            visibility TEXT NOT NULL,
            reply_to_message_id TEXT,
            client_message_id TEXT,
            sequence INTEGER NOT NULL,
            event_cursor INTEGER,
            undo_expires_at TEXT,
            undo_cancelled_at TEXT,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            edited_at TEXT,
            deleted_at TEXT,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (segment_id) REFERENCES conversation_segments(id) ON DELETE SET NULL,
            FOREIGN KEY (participant_id) REFERENCES conversation_participants(id) ON DELETE RESTRICT,
            FOREIGN KEY (reply_to_message_id) REFERENCES conversation_messages(id) ON DELETE SET NULL,
            UNIQUE(conversation_id, sequence),
            UNIQUE(conversation_id, participant_id, client_message_id)
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_messages_conversation_sequence
            ON conversation_messages(conversation_id, sequence ASC);
        CREATE INDEX IF NOT EXISTS idx_conversation_messages_conversation_created
            ON conversation_messages(conversation_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_messages_participant
            ON conversation_messages(participant_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_messages_reply
            ON conversation_messages(reply_to_message_id);

        CREATE TABLE IF NOT EXISTS conversation_message_revisions (
            id TEXT PRIMARY KEY,
            message_id TEXT NOT NULL,
            revision_number INTEGER NOT NULL,
            body_markdown TEXT NOT NULL,
            edited_by_participant_id TEXT NOT NULL,
            reason TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (message_id) REFERENCES conversation_messages(id) ON DELETE CASCADE,
            FOREIGN KEY (edited_by_participant_id) REFERENCES conversation_participants(id) ON DELETE RESTRICT,
            UNIQUE(message_id, revision_number)
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_message_revisions_message
            ON conversation_message_revisions(message_id, revision_number);

        CREATE TABLE IF NOT EXISTS conversation_message_artifacts (
            id TEXT PRIMARY KEY,
            message_id TEXT NOT NULL,
            artifact_kind TEXT NOT NULL,
            artifact_id TEXT NOT NULL,
            label TEXT NOT NULL,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            FOREIGN KEY (message_id) REFERENCES conversation_messages(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_message_artifacts_message
            ON conversation_message_artifacts(message_id, created_at ASC);
        CREATE INDEX IF NOT EXISTS idx_conversation_message_artifacts_artifact
            ON conversation_message_artifacts(artifact_kind, artifact_id);

        CREATE TABLE IF NOT EXISTS conversation_reactions (
            id TEXT PRIMARY KEY,
            message_id TEXT NOT NULL,
            participant_id TEXT NOT NULL,
            reaction_key TEXT NOT NULL,
            reaction_kind TEXT NOT NULL,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            removed_at TEXT,
            FOREIGN KEY (message_id) REFERENCES conversation_messages(id) ON DELETE CASCADE,
            FOREIGN KEY (participant_id) REFERENCES conversation_participants(id) ON DELETE CASCADE
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_conversation_reactions_active
            ON conversation_reactions(message_id, participant_id, reaction_key)
            WHERE removed_at IS NULL;
        CREATE INDEX IF NOT EXISTS idx_conversation_reactions_message
            ON conversation_reactions(message_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS conversation_receipts (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            message_id TEXT,
            participant_id TEXT NOT NULL,
            receipt_kind TEXT NOT NULL,
            event_cursor INTEGER,
            sequence INTEGER,
            payload_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (message_id) REFERENCES conversation_messages(id) ON DELETE SET NULL,
            FOREIGN KEY (participant_id) REFERENCES conversation_participants(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_receipts_participant
            ON conversation_receipts(conversation_id, participant_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_receipts_message
            ON conversation_receipts(message_id, receipt_kind, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_receipts_cursor
            ON conversation_receipts(event_cursor);

        CREATE TABLE IF NOT EXISTS conversation_read_states (
            conversation_id TEXT NOT NULL,
            participant_id TEXT NOT NULL,
            last_delivered_message_id TEXT,
            last_delivered_at TEXT,
            last_displayed_message_id TEXT,
            last_displayed_at TEXT,
            last_read_message_id TEXT,
            last_read_event_cursor INTEGER,
            last_read_at TEXT,
            manual_unread_from_message_id TEXT,
            unread_count INTEGER NOT NULL DEFAULT 0,
            unread_mentions_count INTEGER NOT NULL DEFAULT 0,
            unread_action_count INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (conversation_id, participant_id),
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (participant_id) REFERENCES conversation_participants(id) ON DELETE CASCADE,
            FOREIGN KEY (last_delivered_message_id) REFERENCES conversation_messages(id) ON DELETE SET NULL,
            FOREIGN KEY (last_displayed_message_id) REFERENCES conversation_messages(id) ON DELETE SET NULL,
            FOREIGN KEY (last_read_message_id) REFERENCES conversation_messages(id) ON DELETE SET NULL,
            FOREIGN KEY (manual_unread_from_message_id) REFERENCES conversation_messages(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS conversation_presence_snapshots (
            participant_id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            status TEXT NOT NULL,
            visibility TEXT NOT NULL,
            status_message TEXT,
            device_class TEXT,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            updated_at TEXT NOT NULL,
            expires_at TEXT,
            FOREIGN KEY (participant_id) REFERENCES conversation_participants(id) ON DELETE CASCADE,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_presence_conversation
            ON conversation_presence_snapshots(conversation_id, status, updated_at DESC);
        "#,
    )?;

    Ok(())
}

fn add_llm_token_ledger_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS llm_invocations (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            segment_id TEXT,
            capability_id TEXT NOT NULL,
            provider_id TEXT NOT NULL,
            model_id TEXT NOT NULL,
            status TEXT NOT NULL,
            prompt_hash TEXT NOT NULL,
            privacy_transform_run_ids_json TEXT NOT NULL DEFAULT '[]',
            policy_decision_id TEXT,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            failure_code TEXT,
            failure_message_hash TEXT,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (segment_id) REFERENCES conversation_segments(id) ON DELETE SET NULL,
            FOREIGN KEY (capability_id) REFERENCES capabilities(id) ON DELETE RESTRICT,
            FOREIGN KEY (policy_decision_id) REFERENCES policy_decisions(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_llm_invocations_conversation
            ON llm_invocations(conversation_id, started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_llm_invocations_provider_model
            ON llm_invocations(provider_id, model_id, started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_llm_invocations_capability
            ON llm_invocations(capability_id, started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_llm_invocations_status
            ON llm_invocations(status, started_at DESC);

        CREATE TABLE IF NOT EXISTS llm_prompt_slot_usage (
            id TEXT PRIMARY KEY,
            invocation_id TEXT NOT NULL,
            slot_id TEXT NOT NULL,
            slot_version TEXT NOT NULL,
            source_refs_json TEXT NOT NULL DEFAULT '[]',
            visibility TEXT NOT NULL,
            estimated_tokens INTEGER NOT NULL DEFAULT 0,
            actual_tokens INTEGER,
            content_hash TEXT NOT NULL,
            included INTEGER NOT NULL DEFAULT 1,
            truncation_reason TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (invocation_id) REFERENCES llm_invocations(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_llm_prompt_slot_usage_invocation
            ON llm_prompt_slot_usage(invocation_id, created_at ASC);
        CREATE INDEX IF NOT EXISTS idx_llm_prompt_slot_usage_slot
            ON llm_prompt_slot_usage(slot_id, slot_version, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_llm_prompt_slot_usage_visibility
            ON llm_prompt_slot_usage(visibility, created_at DESC);

        CREATE TABLE IF NOT EXISTS llm_token_ledger_entries (
            id TEXT PRIMARY KEY,
            invocation_id TEXT NOT NULL,
            conversation_id TEXT NOT NULL,
            capability_id TEXT NOT NULL,
            provider_id TEXT NOT NULL,
            model_id TEXT NOT NULL,
            usage_kind TEXT NOT NULL,
            token_count INTEGER NOT NULL,
            estimated_cost_micros INTEGER NOT NULL DEFAULT 0,
            pricing_snapshot_json TEXT NOT NULL DEFAULT '{}',
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            FOREIGN KEY (invocation_id) REFERENCES llm_invocations(id) ON DELETE CASCADE,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (capability_id) REFERENCES capabilities(id) ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_llm_token_ledger_entries_invocation
            ON llm_token_ledger_entries(invocation_id, created_at ASC);
        CREATE INDEX IF NOT EXISTS idx_llm_token_ledger_entries_conversation
            ON llm_token_ledger_entries(conversation_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_llm_token_ledger_entries_provider_model
            ON llm_token_ledger_entries(provider_id, model_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_llm_token_ledger_entries_capability
            ON llm_token_ledger_entries(capability_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_llm_token_ledger_entries_usage_kind
            ON llm_token_ledger_entries(usage_kind, created_at DESC);
        "#,
    )?;

    Ok(())
}

fn add_conversation_analysis_foundation(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS conversation_analysis_jobs (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            segment_id TEXT,
            analysis_kind TEXT NOT NULL,
            status TEXT NOT NULL,
            source_message_id TEXT,
            source_event_cursor_start INTEGER,
            source_event_cursor_end INTEGER,
            input_refs_json TEXT NOT NULL DEFAULT '[]',
            output_json TEXT NOT NULL DEFAULT '{}',
            policy_decision_id TEXT,
            llm_run_id TEXT,
            error_message_hash TEXT,
            created_at TEXT NOT NULL,
            started_at TEXT,
            completed_at TEXT,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (segment_id) REFERENCES conversation_segments(id) ON DELETE SET NULL,
            FOREIGN KEY (source_message_id) REFERENCES conversation_messages(id) ON DELETE SET NULL,
            FOREIGN KEY (policy_decision_id) REFERENCES policy_decisions(id) ON DELETE SET NULL,
            UNIQUE(conversation_id, analysis_kind, source_message_id)
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_analysis_jobs_conversation
            ON conversation_analysis_jobs(conversation_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_analysis_jobs_kind_status
            ON conversation_analysis_jobs(analysis_kind, status, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_analysis_jobs_source_cursor
            ON conversation_analysis_jobs(conversation_id, source_event_cursor_end);

        CREATE TABLE IF NOT EXISTS conversation_analysis_candidates (
            id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL,
            conversation_id TEXT NOT NULL,
            segment_id TEXT,
            candidate_kind TEXT NOT NULL,
            candidate_state TEXT NOT NULL,
            confidence REAL NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            prompt_slot_ids_json TEXT NOT NULL DEFAULT '[]',
            llm_run_id TEXT,
            content_hash TEXT NOT NULL,
            summary_text TEXT NOT NULL,
            body_json TEXT NOT NULL DEFAULT '{}',
            visibility TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (job_id) REFERENCES conversation_analysis_jobs(id) ON DELETE CASCADE,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (segment_id) REFERENCES conversation_segments(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_analysis_candidates_conversation
            ON conversation_analysis_candidates(conversation_id, candidate_kind, candidate_state, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_analysis_candidates_job
            ON conversation_analysis_candidates(job_id, created_at ASC);

        CREATE TABLE IF NOT EXISTS conversation_brief_candidates (
            id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL,
            conversation_id TEXT NOT NULL,
            segment_id TEXT,
            candidate_state TEXT NOT NULL,
            title TEXT NOT NULL,
            brief_markdown TEXT NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            limitations_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            content_hash TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (job_id) REFERENCES conversation_analysis_jobs(id) ON DELETE CASCADE,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (segment_id) REFERENCES conversation_segments(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_brief_candidates_conversation
            ON conversation_brief_candidates(conversation_id, candidate_state, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_brief_candidates_job
            ON conversation_brief_candidates(job_id, created_at ASC);

        CREATE TABLE IF NOT EXISTS conversation_memory_candidates (
            id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL,
            conversation_id TEXT NOT NULL,
            segment_id TEXT,
            memory_kind TEXT NOT NULL,
            candidate_state TEXT NOT NULL,
            confidence REAL NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            content_hash TEXT NOT NULL,
            summary_text TEXT NOT NULL,
            body_json TEXT NOT NULL DEFAULT '{}',
            visibility TEXT NOT NULL,
            approval_status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (job_id) REFERENCES conversation_analysis_jobs(id) ON DELETE CASCADE,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (segment_id) REFERENCES conversation_segments(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_conversation_memory_candidates_conversation
            ON conversation_memory_candidates(conversation_id, candidate_state, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_conversation_memory_candidates_job
            ON conversation_memory_candidates(job_id, created_at ASC);
        "#,
    )?;

    Ok(())
}

fn add_knowledge_graph_candidate_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS knowledge_graph_node_candidates (
            id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL,
            conversation_id TEXT NOT NULL,
            segment_id TEXT,
            source_analysis_candidate_id TEXT,
            node_kind TEXT NOT NULL,
            label TEXT NOT NULL,
            candidate_state TEXT NOT NULL,
            confidence REAL NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            source_event_refs_json TEXT NOT NULL DEFAULT '[]',
            content_hash TEXT NOT NULL,
            visibility TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            state_changed_at TEXT,
            state_reason TEXT,
            FOREIGN KEY (job_id) REFERENCES conversation_analysis_jobs(id) ON DELETE CASCADE,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (segment_id) REFERENCES conversation_segments(id) ON DELETE SET NULL,
            FOREIGN KEY (source_analysis_candidate_id) REFERENCES conversation_analysis_candidates(id) ON DELETE SET NULL,
            UNIQUE(job_id, node_kind, label, content_hash)
        );

        CREATE INDEX IF NOT EXISTS idx_knowledge_graph_node_candidates_conversation
            ON knowledge_graph_node_candidates(conversation_id, candidate_state, node_kind, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_knowledge_graph_node_candidates_job
            ON knowledge_graph_node_candidates(job_id, created_at ASC);

        CREATE TABLE IF NOT EXISTS knowledge_graph_edge_candidates (
            id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL,
            conversation_id TEXT NOT NULL,
            segment_id TEXT,
            source_analysis_candidate_id TEXT,
            source_node_candidate_id TEXT NOT NULL,
            target_node_candidate_id TEXT NOT NULL,
            relationship_kind TEXT NOT NULL,
            label TEXT NOT NULL,
            candidate_state TEXT NOT NULL,
            confidence REAL NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            source_event_refs_json TEXT NOT NULL DEFAULT '[]',
            content_hash TEXT NOT NULL,
            visibility TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            state_changed_at TEXT,
            state_reason TEXT,
            FOREIGN KEY (job_id) REFERENCES conversation_analysis_jobs(id) ON DELETE CASCADE,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (segment_id) REFERENCES conversation_segments(id) ON DELETE SET NULL,
            FOREIGN KEY (source_analysis_candidate_id) REFERENCES conversation_analysis_candidates(id) ON DELETE SET NULL,
            FOREIGN KEY (source_node_candidate_id) REFERENCES knowledge_graph_node_candidates(id) ON DELETE CASCADE,
            FOREIGN KEY (target_node_candidate_id) REFERENCES knowledge_graph_node_candidates(id) ON DELETE CASCADE,
            UNIQUE(job_id, relationship_kind, source_node_candidate_id, target_node_candidate_id, content_hash)
        );

        CREATE INDEX IF NOT EXISTS idx_knowledge_graph_edge_candidates_conversation
            ON knowledge_graph_edge_candidates(conversation_id, candidate_state, relationship_kind, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_knowledge_graph_edge_candidates_job
            ON knowledge_graph_edge_candidates(job_id, created_at ASC);
        "#,
    )?;

    Ok(())
}

fn add_business_outcome_attribution_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS referral_records (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            referrer_connection_id TEXT,
            referred_connection_id TEXT,
            conversation_id TEXT,
            entry_point_id TEXT,
            visitor_session_id TEXT,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            closed_at TEXT,
            FOREIGN KEY (referrer_connection_id) REFERENCES connections(id) ON DELETE SET NULL,
            FOREIGN KEY (referred_connection_id) REFERENCES connections(id) ON DELETE SET NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL,
            FOREIGN KEY (entry_point_id) REFERENCES tracked_entry_points(id) ON DELETE SET NULL,
            FOREIGN KEY (visitor_session_id) REFERENCES visitor_sessions(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_referral_records_status
            ON referral_records(status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_referral_records_referrer
            ON referral_records(referrer_connection_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_referral_records_conversation
            ON referral_records(conversation_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS business_outcomes (
            id TEXT PRIMARY KEY,
            outcome_kind TEXT NOT NULL,
            status TEXT NOT NULL,
            connection_id TEXT,
            conversation_id TEXT,
            segment_id TEXT,
            offer_id TEXT,
            ask_id TEXT,
            artifact_id TEXT,
            entry_point_id TEXT,
            visitor_session_id TEXT,
            referral_id TEXT,
            value_micros INTEGER,
            currency TEXT,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            occurred_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE SET NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL,
            FOREIGN KEY (segment_id) REFERENCES conversation_segments(id) ON DELETE SET NULL,
            FOREIGN KEY (offer_id) REFERENCES offers(id) ON DELETE SET NULL,
            FOREIGN KEY (entry_point_id) REFERENCES tracked_entry_points(id) ON DELETE SET NULL,
            FOREIGN KEY (visitor_session_id) REFERENCES visitor_sessions(id) ON DELETE SET NULL,
            FOREIGN KEY (referral_id) REFERENCES referral_records(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_business_outcomes_kind_status
            ON business_outcomes(outcome_kind, status, occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_business_outcomes_conversation
            ON business_outcomes(conversation_id, occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_business_outcomes_connection
            ON business_outcomes(connection_id, occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_business_outcomes_offer
            ON business_outcomes(offer_id, occurred_at DESC);
        CREATE INDEX IF NOT EXISTS idx_business_outcomes_entry_point
            ON business_outcomes(entry_point_id, occurred_at DESC);

        CREATE TABLE IF NOT EXISTS business_outcome_attributions (
            id TEXT PRIMARY KEY,
            outcome_id TEXT NOT NULL,
            attribution_kind TEXT NOT NULL,
            source_id TEXT NOT NULL,
            influence_role TEXT NOT NULL,
            candidate_state TEXT NOT NULL,
            confidence REAL NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            state_changed_at TEXT,
            state_reason TEXT,
            FOREIGN KEY (outcome_id) REFERENCES business_outcomes(id) ON DELETE CASCADE,
            UNIQUE(outcome_id, attribution_kind, source_id, influence_role)
        );

        CREATE INDEX IF NOT EXISTS idx_business_outcome_attributions_outcome
            ON business_outcome_attributions(outcome_id, candidate_state, created_at ASC);
        CREATE INDEX IF NOT EXISTS idx_business_outcome_attributions_source
            ON business_outcome_attributions(attribution_kind, source_id, candidate_state, created_at DESC);
        "#,
    )?;

    Ok(())
}

fn add_artifact_deliverable_contract_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS artifacts (
            id TEXT PRIMARY KEY,
            artifact_kind TEXT NOT NULL,
            title TEXT NOT NULL,
            status TEXT NOT NULL,
            visibility_ceiling TEXT NOT NULL,
            summary TEXT NOT NULL,
            source_kind TEXT,
            source_id TEXT,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            content_hash TEXT NOT NULL,
            storage_uri TEXT,
            health_status TEXT,
            created_by_job_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (created_by_job_id) REFERENCES jobs(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_artifacts_kind_status
            ON artifacts(artifact_kind, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_artifacts_source
            ON artifacts(source_kind, source_id);
        CREATE INDEX IF NOT EXISTS idx_artifacts_visibility
            ON artifacts(visibility_ceiling, updated_at DESC);

        CREATE TABLE IF NOT EXISTS artifact_versions (
            id TEXT PRIMARY KEY,
            artifact_id TEXT NOT NULL,
            version INTEGER NOT NULL,
            content_hash TEXT NOT NULL,
            storage_uri TEXT,
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE,
            UNIQUE(artifact_id, version)
        );

        CREATE INDEX IF NOT EXISTS idx_artifact_versions_artifact
            ON artifact_versions(artifact_id, version DESC);

        CREATE TABLE IF NOT EXISTS artifact_links (
            id TEXT PRIMARY KEY,
            artifact_id TEXT NOT NULL,
            link_kind TEXT NOT NULL,
            source_kind TEXT NOT NULL,
            source_id TEXT NOT NULL,
            relation TEXT NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE,
            UNIQUE(artifact_id, link_kind, source_kind, source_id, relation)
        );

        CREATE INDEX IF NOT EXISTS idx_artifact_links_artifact
            ON artifact_links(artifact_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_artifact_links_source
            ON artifact_links(source_kind, source_id);

        CREATE TABLE IF NOT EXISTS artifact_deliverables (
            id TEXT PRIMARY KEY,
            artifact_id TEXT NOT NULL,
            client_label TEXT NOT NULL,
            status TEXT NOT NULL,
            visibility TEXT NOT NULL,
            summary TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            published_at TEXT,
            FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_artifact_deliverables_artifact
            ON artifact_deliverables(artifact_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_artifact_deliverables_status
            ON artifact_deliverables(status, updated_at DESC);
        "#,
    )?;
    Ok(())
}

fn add_artifact_patch_proposal_spine(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS artifact_patch_proposals (
            id TEXT PRIMARY KEY,
            source_artifact_id TEXT NOT NULL,
            source_version_id TEXT NOT NULL,
            base_hash TEXT NOT NULL,
            proposed_hash TEXT NOT NULL,
            patch_text TEXT NOT NULL,
            preview_json TEXT NOT NULL DEFAULT '{}',
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            review_state TEXT NOT NULL,
            accepted_version_id TEXT,
            proposed_by_actor_id TEXT NOT NULL,
            applied_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            applied_at TEXT,
            FOREIGN KEY (source_artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE,
            FOREIGN KEY (source_version_id) REFERENCES artifact_versions(id) ON DELETE CASCADE,
            FOREIGN KEY (accepted_version_id) REFERENCES artifact_versions(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_artifact_patch_proposals_artifact
            ON artifact_patch_proposals(source_artifact_id, review_state, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_artifact_patch_proposals_version
            ON artifact_patch_proposals(source_version_id, created_at DESC);
        "#,
    )?;

    Ok(())
}

fn add_cron_schedule_expression(connection: &Connection) -> Result<()> {
    ensure_column(connection, "schedules", "cron_expression", "TEXT")
}

fn add_surface_brief_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS surface_briefs (
            id TEXT PRIMARY KEY,
            surface_kind TEXT NOT NULL,
            subject_kind TEXT,
            subject_id TEXT,
            status TEXT NOT NULL,
            artifact_id TEXT,
            title TEXT NOT NULL,
            brief_markdown TEXT NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            limitations_json TEXT NOT NULL DEFAULT '[]',
            created_by_job_id TEXT,
            generated_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT,
            superseded_at TEXT,
            failure_message TEXT,
            FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE SET NULL,
            FOREIGN KEY (created_by_job_id) REFERENCES jobs(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_surface_briefs_subject_status_generated
            ON surface_briefs(surface_kind, subject_kind, subject_id, status, generated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_surface_briefs_artifact
            ON surface_briefs(artifact_id);
        CREATE INDEX IF NOT EXISTS idx_surface_briefs_job
            ON surface_briefs(created_by_job_id);
        "#,
    )?;
    Ok(())
}

fn add_customer_feedback_review_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS customer_feedback (
            id TEXT PRIMARY KEY,
            connection_id TEXT,
            conversation_id TEXT NOT NULL,
            segment_id TEXT,
            message_id TEXT,
            feedback_kind TEXT NOT NULL,
            status TEXT NOT NULL,
            visibility TEXT NOT NULL,
            body_summary TEXT NOT NULL,
            is_starred INTEGER NOT NULL DEFAULT 0,
            source_refs_json TEXT NOT NULL DEFAULT '[]',
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (message_id) REFERENCES conversation_messages(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_customer_feedback_conversation
            ON customer_feedback(conversation_id, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_customer_feedback_connection
            ON customer_feedback(connection_id, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_customer_feedback_visibility
            ON customer_feedback(visibility, status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS feedback_tags (
            id TEXT PRIMARY KEY,
            feedback_id TEXT NOT NULL,
            tag TEXT NOT NULL,
            candidate_state TEXT NOT NULL,
            confidence REAL NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            state_changed_at TEXT,
            state_reason TEXT,
            FOREIGN KEY (feedback_id) REFERENCES customer_feedback(id) ON DELETE CASCADE,
            UNIQUE(feedback_id, tag)
        );

        CREATE INDEX IF NOT EXISTS idx_feedback_tags_feedback_state
            ON feedback_tags(feedback_id, candidate_state, updated_at DESC);

        CREATE TABLE IF NOT EXISTS customer_reviews (
            id TEXT PRIMARY KEY,
            feedback_id TEXT NOT NULL,
            connection_id TEXT,
            conversation_id TEXT NOT NULL,
            status TEXT NOT NULL,
            review_body TEXT NOT NULL,
            publication_visibility TEXT NOT NULL,
            consent_evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            approval_evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            published_at TEXT,
            featured_at TEXT,
            retired_at TEXT,
            FOREIGN KEY (feedback_id) REFERENCES customer_feedback(id) ON DELETE CASCADE,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_customer_reviews_feedback
            ON customer_reviews(feedback_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_customer_reviews_status
            ON customer_reviews(status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_customer_reviews_public
            ON customer_reviews(publication_visibility, status, updated_at DESC);
        "#,
    )?;
    Ok(())
}

fn add_actor_experience_preferences_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS actor_experience_preferences (
            actor_id TEXT PRIMARY KEY,
            schema_version TEXT NOT NULL,
            requested_settings_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (actor_id) REFERENCES actors(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_actor_experience_preferences_updated
            ON actor_experience_preferences(updated_at DESC);
        "#,
    )?;
    Ok(())
}

fn add_feedback_request_loop(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS feedback_requests (
            id TEXT PRIMARY KEY,
            target_kind TEXT NOT NULL,
            target_id TEXT NOT NULL,
            member_actor_id TEXT,
            connection_id TEXT,
            conversation_id TEXT,
            source_kind TEXT NOT NULL,
            source_id TEXT,
            prompt TEXT NOT NULL,
            member_context_summary TEXT NOT NULL,
            status TEXT NOT NULL,
            due_at TEXT,
            priority TEXT NOT NULL DEFAULT 'normal',
            created_by_actor_id TEXT,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            staff_context_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            closed_at TEXT,
            CHECK (status IN (
                'open',
                'responded',
                'follow_up_requested',
                'accepted',
                'rejected',
                'expired',
                'canceled'
            )),
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_feedback_requests_target
            ON feedback_requests(target_kind, target_id, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_feedback_requests_member
            ON feedback_requests(member_actor_id, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_feedback_requests_connection
            ON feedback_requests(connection_id, status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_feedback_requests_status
            ON feedback_requests(status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS feedback_request_responses (
            id TEXT PRIMARY KEY,
            request_id TEXT NOT NULL,
            actor_id TEXT,
            response_kind TEXT NOT NULL,
            body_summary TEXT NOT NULL,
            customer_feedback_id TEXT,
            idempotency_key TEXT,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (request_id) REFERENCES feedback_requests(id) ON DELETE CASCADE,
            FOREIGN KEY (customer_feedback_id) REFERENCES customer_feedback(id) ON DELETE SET NULL,
            UNIQUE(request_id, idempotency_key)
        );

        CREATE INDEX IF NOT EXISTS idx_feedback_request_responses_request
            ON feedback_request_responses(request_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_feedback_request_responses_actor
            ON feedback_request_responses(actor_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS feedback_request_reviews (
            id TEXT PRIMARY KEY,
            request_id TEXT NOT NULL,
            response_id TEXT,
            reviewer_actor_id TEXT,
            decision TEXT NOT NULL,
            tags_json TEXT NOT NULL DEFAULT '[]',
            reason TEXT NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (request_id) REFERENCES feedback_requests(id) ON DELETE CASCADE,
            FOREIGN KEY (response_id) REFERENCES feedback_request_responses(id) ON DELETE SET NULL,
            CHECK (decision IN ('accepted', 'rejected', 'follow_up_requested'))
        );

        CREATE INDEX IF NOT EXISTS idx_feedback_request_reviews_request
            ON feedback_request_reviews(request_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_feedback_request_reviews_decision
            ON feedback_request_reviews(decision, updated_at DESC);

        CREATE TABLE IF NOT EXISTS feedback_reward_eligibility (
            id TEXT PRIMARY KEY,
            request_id TEXT NOT NULL,
            response_id TEXT,
            review_id TEXT,
            actor_id TEXT,
            state TEXT NOT NULL,
            reason TEXT NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (request_id) REFERENCES feedback_requests(id) ON DELETE CASCADE,
            FOREIGN KEY (response_id) REFERENCES feedback_request_responses(id) ON DELETE SET NULL,
            FOREIGN KEY (review_id) REFERENCES feedback_request_reviews(id) ON DELETE SET NULL,
            UNIQUE(request_id, response_id)
        );

        CREATE INDEX IF NOT EXISTS idx_feedback_reward_eligibility_state
            ON feedback_reward_eligibility(state, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_feedback_reward_eligibility_actor
            ON feedback_reward_eligibility(actor_id, updated_at DESC);
        "#,
    )?;
    Ok(())
}

fn add_reward_ledger_and_benefits(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS reward_programs (
            id TEXT PRIMARY KEY,
            slug TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            status TEXT NOT NULL,
            visibility TEXT NOT NULL,
            terms_json TEXT NOT NULL DEFAULT '{}',
            policy_json TEXT NOT NULL DEFAULT '{}',
            starts_at TEXT,
            ends_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_reward_programs_status
            ON reward_programs(status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS reward_rules (
            id TEXT PRIMARY KEY,
            program_id TEXT NOT NULL,
            trigger_kind TEXT NOT NULL,
            status TEXT NOT NULL,
            benefit_kind TEXT NOT NULL,
            benefit_quantity INTEGER NOT NULL,
            benefit_unit TEXT NOT NULL,
            max_quantity_per_actor INTEGER NOT NULL,
            qualification_policy_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (program_id) REFERENCES reward_programs(id) ON DELETE CASCADE,
            UNIQUE(program_id, trigger_kind)
        );

        CREATE INDEX IF NOT EXISTS idx_reward_rules_program_trigger
            ON reward_rules(program_id, trigger_kind, status);

        CREATE TABLE IF NOT EXISTS reward_events (
            id TEXT PRIMARY KEY,
            program_id TEXT NOT NULL,
            rule_id TEXT NOT NULL,
            actor_id TEXT,
            connection_id TEXT,
            source_kind TEXT NOT NULL,
            source_id TEXT NOT NULL,
            state TEXT NOT NULL,
            idempotency_key TEXT NOT NULL UNIQUE,
            reason TEXT NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            qualified_at TEXT,
            granted_at TEXT,
            rejected_at TEXT,
            expired_at TEXT,
            capped_at TEXT,
            reversed_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (program_id) REFERENCES reward_programs(id) ON DELETE CASCADE,
            FOREIGN KEY (rule_id) REFERENCES reward_rules(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_reward_events_actor_state
            ON reward_events(actor_id, state, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_reward_events_connection_state
            ON reward_events(connection_id, state, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_reward_events_source
            ON reward_events(source_kind, source_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_reward_events_program_state
            ON reward_events(program_id, state, updated_at DESC);

        CREATE TABLE IF NOT EXISTS reward_ledger_entries (
            id TEXT PRIMARY KEY,
            event_id TEXT NOT NULL,
            program_id TEXT NOT NULL,
            rule_id TEXT NOT NULL,
            actor_id TEXT,
            connection_id TEXT,
            entry_kind TEXT NOT NULL,
            amount INTEGER NOT NULL,
            unit TEXT NOT NULL,
            benefit_grant_id TEXT,
            reason TEXT NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            reversed_at TEXT,
            reversal_reason TEXT,
            FOREIGN KEY (event_id) REFERENCES reward_events(id) ON DELETE CASCADE,
            FOREIGN KEY (program_id) REFERENCES reward_programs(id) ON DELETE CASCADE,
            FOREIGN KEY (rule_id) REFERENCES reward_rules(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_reward_ledger_entries_event
            ON reward_ledger_entries(event_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_reward_ledger_entries_actor
            ON reward_ledger_entries(actor_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_reward_ledger_entries_benefit
            ON reward_ledger_entries(benefit_grant_id);

        CREATE TABLE IF NOT EXISTS benefit_grants (
            id TEXT PRIMARY KEY,
            event_id TEXT NOT NULL,
            ledger_entry_id TEXT,
            actor_id TEXT,
            connection_id TEXT,
            access_grant_id TEXT,
            trial_id TEXT,
            benefit_kind TEXT NOT NULL,
            amount INTEGER NOT NULL,
            unit TEXT NOT NULL,
            state TEXT NOT NULL,
            starts_at TEXT NOT NULL,
            expires_at TEXT,
            consumed_at TEXT,
            revoked_at TEXT,
            reversed_at TEXT,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            metadata_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (event_id) REFERENCES reward_events(id) ON DELETE CASCADE,
            FOREIGN KEY (ledger_entry_id) REFERENCES reward_ledger_entries(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_benefit_grants_actor_state
            ON benefit_grants(actor_id, state, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_benefit_grants_connection_state
            ON benefit_grants(connection_id, state, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_benefit_grants_access
            ON benefit_grants(access_grant_id, state);
        CREATE INDEX IF NOT EXISTS idx_benefit_grants_trial
            ON benefit_grants(trial_id, state);

        CREATE TABLE IF NOT EXISTS benefit_balances (
            id TEXT PRIMARY KEY,
            program_id TEXT NOT NULL,
            actor_id TEXT,
            connection_id TEXT,
            benefit_kind TEXT NOT NULL,
            unit TEXT NOT NULL,
            total_earned INTEGER NOT NULL DEFAULT 0,
            total_active INTEGER NOT NULL DEFAULT 0,
            total_reversed INTEGER NOT NULL DEFAULT 0,
            cap_quantity INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL,
            UNIQUE(program_id, actor_id, connection_id, benefit_kind, unit),
            FOREIGN KEY (program_id) REFERENCES reward_programs(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_benefit_balances_actor
            ON benefit_balances(actor_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_benefit_balances_connection
            ON benefit_balances(connection_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS qualification_reviews (
            id TEXT PRIMARY KEY,
            event_id TEXT NOT NULL,
            reviewer_actor_id TEXT,
            decision TEXT NOT NULL,
            reason TEXT NOT NULL,
            evidence_refs_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (event_id) REFERENCES reward_events(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_qualification_reviews_event
            ON qualification_reviews(event_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_qualification_reviews_decision
            ON qualification_reviews(decision, updated_at DESC);
        "#,
    )?;
    Ok(())
}

fn add_product_pack_manifest_spine(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS product_packs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            status TEXT NOT NULL,
            manifest_json TEXT NOT NULL DEFAULT '{}',
            validation_json TEXT NOT NULL DEFAULT '{}',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            created_by_actor_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (created_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_product_packs_status_updated
            ON product_packs(status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS product_pack_versions (
            id TEXT PRIMARY KEY,
            pack_id TEXT NOT NULL,
            version TEXT NOT NULL,
            manifest_json TEXT NOT NULL DEFAULT '{}',
            validation_json TEXT NOT NULL DEFAULT '{}',
            provenance_json TEXT NOT NULL DEFAULT '{}',
            installed_by_actor_id TEXT,
            installed_at TEXT NOT NULL,
            FOREIGN KEY (pack_id) REFERENCES product_packs(id) ON DELETE CASCADE,
            FOREIGN KEY (installed_by_actor_id) REFERENCES actors(id) ON DELETE SET NULL,
            UNIQUE(pack_id, version)
        );

        CREATE INDEX IF NOT EXISTS idx_product_pack_versions_pack
            ON product_pack_versions(pack_id, installed_at DESC);

        CREATE TABLE IF NOT EXISTS product_pack_bindings (
            id TEXT PRIMARY KEY,
            pack_id TEXT NOT NULL,
            binding_kind TEXT NOT NULL,
            binding_key TEXT NOT NULL,
            capability_id TEXT,
            template_id TEXT,
            template_version INTEGER,
            artifact_kind TEXT,
            contract_json TEXT NOT NULL DEFAULT '{}',
            visibility_json TEXT NOT NULL DEFAULT '{}',
            access_json TEXT NOT NULL DEFAULT '{}',
            growth_json TEXT NOT NULL DEFAULT '{}',
            limits_json TEXT NOT NULL DEFAULT '{}',
            status TEXT NOT NULL,
            disabled_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (pack_id) REFERENCES product_packs(id) ON DELETE CASCADE,
            FOREIGN KEY (capability_id) REFERENCES capabilities(id) ON DELETE SET NULL,
            FOREIGN KEY (template_id, template_version) REFERENCES process_templates(id, version),
            UNIQUE(pack_id, binding_kind, binding_key)
        );

        CREATE INDEX IF NOT EXISTS idx_product_pack_bindings_pack
            ON product_pack_bindings(pack_id, binding_kind, binding_key);
        CREATE INDEX IF NOT EXISTS idx_product_pack_bindings_capability
            ON product_pack_bindings(capability_id, status);
        CREATE INDEX IF NOT EXISTS idx_product_pack_bindings_template
            ON product_pack_bindings(template_id, template_version, status);
        "#,
    )?;

    Ok(())
}

fn add_job_task_lease_state(connection: &Connection) -> Result<()> {
    ensure_column(connection, "job_tasks", "lease_owner_id", "TEXT")?;
    ensure_column(connection, "job_tasks", "lease_expires_at", "TEXT")?;
    ensure_column(connection, "job_tasks", "claimed_at", "TEXT")?;
    ensure_column(
        connection,
        "job_tasks",
        "retry_policy_json",
        "TEXT NOT NULL DEFAULT '{}'",
    )?;
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_job_tasks_ready_lease
            ON job_tasks(job_id, status, lease_expires_at)",
        [],
    )?;

    Ok(())
}

fn add_compiled_plan_snapshots(connection: &Connection) -> Result<()> {
    ensure_column(
        connection,
        "process_templates",
        "variable_schema_json",
        "TEXT NOT NULL DEFAULT '{}'",
    )?;
    ensure_column(
        connection,
        "jobs",
        "compiled_plan_json",
        "TEXT NOT NULL DEFAULT '{}'",
    )?;
    connection.execute(
        "UPDATE jobs
         SET compiled_plan_json = json_object(
            'schemaVersion', 1,
            'template', json_object(
                'id', template_id,
                'version', template_version,
                'kind', kind,
                'capabilityId', capability_id
            ),
            'variableSchema', json('{}'),
            'input', CASE
                WHEN json_valid(input_json) THEN json(input_json)
                ELSE json('{}')
            END,
            'tasks', json('[]')
         )
         WHERE compiled_plan_json = '{}'",
        [],
    )?;

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
