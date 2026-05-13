use anyhow::{bail, Result};
use rusqlite::Connection;
use std::path::Path;
use uuid::Uuid;
use crate::templates::require_builtin_template;
use crate::kernel::create_job_from_template;
use serde_json::json;
use super::types::*;
use super::core::*;

pub fn create_backup(
    db_path: &Path,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<BackupRestoreJobSummary> {
    let mut connection = Connection::open(db_path)?;
    let template = require_builtin_template(BACKUP_TEMPLATE_ID)?;
    let job_id = create_job_from_template(
        &mut connection,
        &template,
        origin,
        actor_id,
        json!({ "dbPath": path_string(db_path) }),
    )?;

    match complete_backup_job(&connection, db_path, &job_id) {
        Ok(()) => load_job_summary(&connection, &job_id),
        Err(error) => {
            mark_job_failed(&connection, &job_id, &error.to_string())?;
            Err(error)
        }
    }
}

pub fn run_restore_preflight(
    db_path: &Path,
    request: RestorePreflightRequest,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<BackupRestoreJobSummary> {
    let mut connection = Connection::open(db_path)?;
    let template = require_builtin_template(RESTORE_TEMPLATE_ID)?;
    let job_id = create_job_from_template(
        &mut connection,
        &template,
        origin,
        actor_id,
        json!({ "backupId": request.backup_id }),
    )?;

    match complete_restore_preflight_job(&connection, db_path, &job_id, request) {
        Ok(()) => load_job_summary(&connection, &job_id),
        Err(error) => {
            mark_job_failed(&connection, &job_id, &error.to_string())?;
            Err(error)
        }
    }
}

pub fn list_backup_restore_jobs(db_path: &Path) -> Result<BackupRestoreResponse> {
    let connection = Connection::open(db_path)?;
    let mut statement = connection.prepare(
        "SELECT id FROM jobs
         WHERE template_id IN ('backup.create', 'restore.execute')
         ORDER BY created_at DESC
         LIMIT 25",
    )?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut jobs = Vec::new();
    for row in rows {
        jobs.push(load_job_summary(&connection, &row?)?);
    }

    Ok(BackupRestoreResponse { jobs })
}

