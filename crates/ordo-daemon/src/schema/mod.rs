use anyhow::{bail, Result};
use rusqlite::Connection;
use std::fs;
use std::path::Path;

use crate::capabilities::seed_builtin_capabilities;
use crate::scheduler::ensure_default_system_brief_schedule;
use crate::templates::seed_builtin_templates;

pub mod db;
pub use db::*;

pub mod migrations;
pub(crate) use migrations::*;

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
    "hosted_trial_capacity_policies",
    "hosted_trial_slots",
    "hosted_trial_waitlist_entries",
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
    "conversations",
    "conversation_segments",
    "conversation_handoffs",
    "conversation_modes",
    "conversation_events",
    "conversation_participants",
    "conversation_messages",
    "conversation_message_revisions",
    "conversation_message_artifacts",
    "conversation_reactions",
    "conversation_receipts",
    "conversation_read_states",
    "conversation_presence_snapshots",
    "llm_invocations",
    "llm_prompt_slot_usage",
    "llm_token_ledger_entries",
    "conversation_analysis_jobs",
    "conversation_analysis_candidates",
    "conversation_brief_candidates",
    "conversation_memory_candidates",
    "knowledge_graph_node_candidates",
    "knowledge_graph_edge_candidates",
    "artifacts",
    "artifact_versions",
    "artifact_patch_proposals",
    "artifact_links",
    "artifact_deliverables",
    "surface_briefs",
    "surface_work_items",
    "customer_feedback",
    "feedback_tags",
    "customer_reviews",
    "feedback_requests",
    "feedback_request_responses",
    "feedback_request_reviews",
    "feedback_reward_eligibility",
    "reward_programs",
    "reward_rules",
    "reward_events",
    "reward_ledger_entries",
    "benefit_grants",
    "benefit_balances",
    "qualification_reviews",
    "referral_records",
    "business_outcomes",
    "business_outcome_attributions",
    "corpus_sources",
    "corpus_items",
    "corpus_items_fts",
    "answer_drafts",
    "answer_draft_citations",
    "mcp_packs",
    "mcp_pack_tools",
    "product_packs",
    "product_pack_versions",
    "product_pack_bindings",
    "schedules",
    "scheduled_job_runs",
    "brief_artifacts",
    "preferences",
    "actor_experience_preferences",
    "local_account_sessions",
];

pub const CURRENT_SCHEMA_VERSION: i64 = 37;

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

fn schema_version(connection: &Connection) -> Result<i64> {
    let version = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(version)
}

fn set_schema_version(connection: &Connection, version: i64) -> Result<()> {
    connection.pragma_update(None, "user_version", version)?;
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
            vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37,
            ]
        );
        assert_eq!(CURRENT_SCHEMA_VERSION, 37);
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
        assert!(table_exists(&connection, "corpus_items_fts"));
        assert!(table_exists(&connection, "answer_drafts"));
        assert!(table_exists(&connection, "answer_draft_citations"));
        assert!(table_exists(&connection, "mcp_packs"));
        assert!(table_exists(&connection, "mcp_pack_tools"));
        assert!(table_exists(&connection, "product_packs"));
        assert!(table_exists(&connection, "product_pack_versions"));
        assert!(table_exists(&connection, "product_pack_bindings"));
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
        assert!(table_exists(&connection, "conversations"));
        assert!(table_exists(&connection, "conversation_segments"));
        assert!(table_exists(&connection, "conversation_handoffs"));
        assert!(table_exists(&connection, "conversation_modes"));
        assert!(table_exists(&connection, "conversation_events"));
        assert!(table_exists(&connection, "conversation_participants"));
        assert!(table_exists(&connection, "conversation_messages"));
        assert!(table_exists(&connection, "conversation_message_revisions"));
        assert!(table_exists(&connection, "conversation_message_artifacts"));
        assert!(table_exists(&connection, "conversation_reactions"));
        assert!(table_exists(&connection, "conversation_receipts"));
        assert!(table_exists(&connection, "conversation_read_states"));
        assert!(table_exists(&connection, "conversation_presence_snapshots"));

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
    fn corpus_fts_retrieval_index_is_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "corpus_items_fts"));
        assert!(column_exists(&connection, "corpus_items_fts", "item_id"));
        assert!(column_exists(&connection, "corpus_items_fts", "body_text"));
    }

    #[test]
    fn answer_draft_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "answer_drafts"));
        assert!(column_exists(
            &connection,
            "answer_drafts",
            "retrieval_evidence_json"
        ));
        assert!(column_exists(
            &connection,
            "answer_drafts",
            "cited_item_ids_json"
        ));
        assert!(column_exists(
            &connection,
            "answer_drafts",
            "provenance_json"
        ));
        assert!(table_exists(&connection, "answer_draft_citations"));
        assert!(column_exists(
            &connection,
            "answer_draft_citations",
            "content_hash"
        ));
        assert!(column_exists(
            &connection,
            "answer_draft_citations",
            "evidence_json"
        ));
    }

    #[test]
    fn mcp_pack_hardening_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "mcp_packs"));
        assert!(column_exists(&connection, "mcp_packs", "manifest_json"));
        assert!(column_exists(&connection, "mcp_packs", "provenance_json"));
        assert!(table_exists(&connection, "mcp_pack_tools"));
        assert!(column_exists(
            &connection,
            "mcp_pack_tools",
            "capability_id"
        ));
        assert!(column_exists(
            &connection,
            "mcp_pack_tools",
            "export_status"
        ));
        assert!(column_exists(
            &connection,
            "mcp_pack_tools",
            "mcp_export_policy"
        ));
    }

    #[test]
    fn product_pack_manifest_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "product_packs"));
        assert!(column_exists(&connection, "product_packs", "manifest_json"));
        assert!(column_exists(
            &connection,
            "product_packs",
            "validation_json"
        ));
        assert!(column_exists(
            &connection,
            "product_packs",
            "provenance_json"
        ));
        assert!(table_exists(&connection, "product_pack_versions"));
        assert!(column_exists(
            &connection,
            "product_pack_versions",
            "manifest_json"
        ));
        assert!(table_exists(&connection, "product_pack_bindings"));
        assert!(column_exists(
            &connection,
            "product_pack_bindings",
            "binding_kind"
        ));
        assert!(column_exists(
            &connection,
            "product_pack_bindings",
            "capability_id"
        ));
        assert!(column_exists(
            &connection,
            "product_pack_bindings",
            "template_id"
        ));
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
        assert!(column_exists(
            &connection,
            "offer_acceptances",
            "idempotency_key"
        ));
        assert!(column_exists(
            &connection,
            "offer_acceptances",
            "access_grant_id"
        ));
        assert!(column_exists(
            &connection,
            "offer_acceptances",
            "receipt_json"
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
    fn hosted_trial_capacity_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "hosted_trial_capacity_policies"));
        assert!(column_exists(
            &connection,
            "hosted_trial_capacity_policies",
            "active_slot_limit"
        ));
        assert!(column_exists(
            &connection,
            "hosted_trial_capacity_policies",
            "backup_before_wipe_required"
        ));
        assert!(table_exists(&connection, "hosted_trial_slots"));
        assert!(column_exists(
            &connection,
            "hosted_trial_slots",
            "backup_status"
        ));
        assert!(column_exists(
            &connection,
            "hosted_trial_slots",
            "reset_state"
        ));
        assert!(column_exists(
            &connection,
            "hosted_trial_slots",
            "reset_guard_json"
        ));
        assert!(table_exists(&connection, "hosted_trial_waitlist_entries"));
        assert!(column_exists(
            &connection,
            "hosted_trial_waitlist_entries",
            "position"
        ));
        assert!(column_exists(
            &connection,
            "hosted_trial_waitlist_entries",
            "receipt_json"
        ));
    }

    #[test]
    fn feedback_request_loop_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "feedback_requests"));
        assert!(column_exists(
            &connection,
            "feedback_requests",
            "target_kind"
        ));
        assert!(column_exists(
            &connection,
            "feedback_requests",
            "member_context_summary"
        ));
        assert!(column_exists(
            &connection,
            "feedback_requests",
            "staff_context_json"
        ));
        assert!(table_exists(&connection, "feedback_request_responses"));
        assert!(column_exists(
            &connection,
            "feedback_request_responses",
            "customer_feedback_id"
        ));
        assert!(column_exists(
            &connection,
            "feedback_request_responses",
            "idempotency_key"
        ));
        assert!(table_exists(&connection, "feedback_request_reviews"));
        assert!(column_exists(
            &connection,
            "feedback_request_reviews",
            "decision"
        ));
        assert!(table_exists(&connection, "feedback_reward_eligibility"));
        assert!(column_exists(
            &connection,
            "feedback_reward_eligibility",
            "state"
        ));
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
        assert!(column_exists(&connection, "handoff_inbox_items", "reason"));
        assert!(column_exists(
            &connection,
            "handoff_inbox_items",
            "requested_action"
        ));
        assert!(column_exists(&connection, "handoff_inbox_items", "urgency"));
        assert!(column_exists(
            &connection,
            "handoff_inbox_items",
            "assignee_actor_id"
        ));
        assert!(column_exists(
            &connection,
            "handoff_inbox_items",
            "evidence_refs_json"
        ));
        assert!(column_exists(
            &connection,
            "handoff_inbox_items",
            "visibility"
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
    fn conversation_product_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "conversations"));
        assert!(column_exists(&connection, "conversations", "subject_kind"));
        assert!(column_exists(
            &connection,
            "conversations",
            "last_meaningful_change"
        ));
        assert!(table_exists(&connection, "conversation_segments"));
        assert!(column_exists(
            &connection,
            "conversation_segments",
            "candidate_state"
        ));
        assert!(column_exists(
            &connection,
            "conversation_segments",
            "provenance_json"
        ));
        assert!(table_exists(&connection, "conversation_handoffs"));
        assert!(column_exists(
            &connection,
            "conversation_handoffs",
            "allowed_context_json"
        ));
        assert!(column_exists(
            &connection,
            "conversation_handoffs",
            "policy_decision_id"
        ));
        assert!(table_exists(&connection, "conversation_modes"));
        assert!(column_exists(
            &connection,
            "conversation_modes",
            "private_reminder_sent_at"
        ));
        assert!(table_exists(&connection, "conversation_events"));
        assert!(column_exists(
            &connection,
            "conversation_events",
            "realtime_cursor"
        ));
    }

    #[test]
    fn conversation_message_protocol_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "conversation_participants"));
        assert!(column_exists(
            &connection,
            "conversation_participants",
            "participant_kind"
        ));
        assert!(column_exists(
            &connection,
            "conversation_participants",
            "privacy_settings_json"
        ));
        assert!(table_exists(&connection, "conversation_messages"));
        assert!(column_exists(
            &connection,
            "conversation_messages",
            "client_message_id"
        ));
        assert!(column_exists(
            &connection,
            "conversation_messages",
            "undo_expires_at"
        ));
        assert!(column_exists(
            &connection,
            "conversation_messages",
            "undo_cancelled_at"
        ));
        assert!(table_exists(&connection, "conversation_message_revisions"));
        assert!(column_exists(
            &connection,
            "conversation_message_revisions",
            "revision_number"
        ));
        assert!(table_exists(&connection, "conversation_message_artifacts"));
        assert!(table_exists(&connection, "conversation_reactions"));
        assert!(column_exists(
            &connection,
            "conversation_reactions",
            "removed_at"
        ));
        assert!(table_exists(&connection, "conversation_receipts"));
        assert!(column_exists(
            &connection,
            "conversation_receipts",
            "receipt_kind"
        ));
        assert!(table_exists(&connection, "conversation_read_states"));
        assert!(column_exists(
            &connection,
            "conversation_read_states",
            "manual_unread_from_message_id"
        ));
        assert!(table_exists(&connection, "conversation_presence_snapshots"));
        assert!(column_exists(
            &connection,
            "conversation_presence_snapshots",
            "expires_at"
        ));
    }

    #[test]
    fn llm_token_ledger_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "llm_invocations"));
        assert!(column_exists(
            &connection,
            "llm_invocations",
            "privacy_transform_run_ids_json"
        ));
        assert!(column_exists(
            &connection,
            "llm_invocations",
            "failure_message_hash"
        ));
        assert!(table_exists(&connection, "llm_prompt_slot_usage"));
        assert!(column_exists(
            &connection,
            "llm_prompt_slot_usage",
            "estimated_tokens"
        ));
        assert!(column_exists(
            &connection,
            "llm_prompt_slot_usage",
            "actual_tokens"
        ));
        assert!(table_exists(&connection, "llm_token_ledger_entries"));
        assert!(column_exists(
            &connection,
            "llm_token_ledger_entries",
            "estimated_cost_micros"
        ));
        assert!(column_exists(
            &connection,
            "llm_token_ledger_entries",
            "pricing_snapshot_json"
        ));
    }

    #[test]
    fn conversation_analysis_tables_are_created() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();

        assert!(table_exists(&connection, "conversation_analysis_jobs"));
        assert!(column_exists(
            &connection,
            "conversation_analysis_jobs",
            "source_event_cursor_end"
        ));
        assert!(column_exists(
            &connection,
            "conversation_analysis_jobs",
            "error_message_hash"
        ));
        assert!(table_exists(
            &connection,
            "conversation_analysis_candidates"
        ));
        assert!(column_exists(
            &connection,
            "conversation_analysis_candidates",
            "candidate_state"
        ));
        assert!(column_exists(
            &connection,
            "conversation_analysis_candidates",
            "prompt_slot_ids_json"
        ));
        assert!(table_exists(&connection, "conversation_brief_candidates"));
        assert!(column_exists(
            &connection,
            "conversation_brief_candidates",
            "limitations_json"
        ));
        assert!(table_exists(&connection, "conversation_memory_candidates"));
        assert!(column_exists(
            &connection,
            "conversation_memory_candidates",
            "approval_status"
        ));
        assert!(table_exists(&connection, "knowledge_graph_node_candidates"));
        assert!(column_exists(
            &connection,
            "knowledge_graph_node_candidates",
            "source_analysis_candidate_id"
        ));
        assert!(column_exists(
            &connection,
            "knowledge_graph_node_candidates",
            "candidate_state"
        ));
        assert!(table_exists(&connection, "knowledge_graph_edge_candidates"));
        assert!(column_exists(
            &connection,
            "knowledge_graph_edge_candidates",
            "source_node_candidate_id"
        ));
        assert!(column_exists(
            &connection,
            "knowledge_graph_edge_candidates",
            "target_node_candidate_id"
        ));
        assert!(table_exists(&connection, "referral_records"));
        assert!(column_exists(
            &connection,
            "referral_records",
            "referrer_connection_id"
        ));
        assert!(table_exists(&connection, "business_outcomes"));
        assert!(column_exists(
            &connection,
            "business_outcomes",
            "evidence_refs_json"
        ));
        assert!(column_exists(&connection, "business_outcomes", "ask_id"));
        assert!(column_exists(
            &connection,
            "business_outcomes",
            "artifact_id"
        ));
        assert!(table_exists(&connection, "business_outcome_attributions"));
        assert!(column_exists(
            &connection,
            "business_outcome_attributions",
            "candidate_state"
        ));
        assert!(column_exists(
            &connection,
            "business_outcome_attributions",
            "influence_role"
        ));
        assert!(table_exists(&connection, "artifacts"));
        assert!(column_exists(
            &connection,
            "artifacts",
            "visibility_ceiling"
        ));
        assert!(column_exists(&connection, "artifacts", "content_hash"));
        assert!(table_exists(&connection, "artifact_versions"));
        assert!(column_exists(
            &connection,
            "artifact_versions",
            "metadata_json"
        ));
        assert!(table_exists(&connection, "artifact_patch_proposals"));
        assert!(column_exists(
            &connection,
            "artifact_patch_proposals",
            "patch_text"
        ));
        assert!(column_exists(
            &connection,
            "artifact_patch_proposals",
            "review_state"
        ));
        assert!(table_exists(&connection, "artifact_links"));
        assert!(column_exists(&connection, "artifact_links", "relation"));
        assert!(table_exists(&connection, "artifact_deliverables"));
        assert!(column_exists(
            &connection,
            "artifact_deliverables",
            "client_label"
        ));
        assert!(table_exists(&connection, "surface_briefs"));
        assert!(column_exists(&connection, "surface_briefs", "artifact_id"));
        assert!(column_exists(
            &connection,
            "surface_briefs",
            "limitations_json"
        ));
        assert!(column_exists(
            &connection,
            "surface_briefs",
            "superseded_at"
        ));
        assert!(table_exists(&connection, "surface_work_items"));
        assert!(column_exists(
            &connection,
            "surface_work_items",
            "surface_kind"
        ));
        assert!(column_exists(
            &connection,
            "surface_work_items",
            "room_kind"
        ));
        assert!(column_exists(
            &connection,
            "surface_work_items",
            "evidence_refs_json"
        ));
        assert!(column_exists(
            &connection,
            "surface_work_items",
            "actions_json"
        ));
        assert!(column_exists(
            &connection,
            "surface_work_items",
            "visibility"
        ));
        assert!(table_exists(&connection, "customer_feedback"));
        assert!(column_exists(
            &connection,
            "customer_feedback",
            "visibility"
        ));
        assert!(column_exists(
            &connection,
            "customer_feedback",
            "evidence_refs_json"
        ));
        assert!(table_exists(&connection, "feedback_tags"));
        assert!(column_exists(
            &connection,
            "feedback_tags",
            "candidate_state"
        ));
        assert!(table_exists(&connection, "customer_reviews"));
        assert!(column_exists(
            &connection,
            "customer_reviews",
            "consent_evidence_refs_json"
        ));
        assert!(column_exists(
            &connection,
            "customer_reviews",
            "approval_evidence_refs_json"
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
