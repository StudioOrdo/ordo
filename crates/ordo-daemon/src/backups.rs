use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::kernel::{append_job_event, create_job_from_template};
use crate::templates::require_builtin_template;

const BACKUP_TEMPLATE_ID: &str = "backup.create";
const RESTORE_TEMPLATE_ID: &str = "restore.execute";
const CHECKSUM_ALGORITHM: &str = "fnv1a64";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestoreResponse {
    pub jobs: Vec<BackupRestoreJobSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestoreJobSummary {
    pub id: String,
    pub operation: String,
    pub kind: String,
    pub status: String,
    pub progress: JobProgressSummary,
    pub current_task_key: Option<String>,
    pub elapsed_seconds: Option<i64>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub failure_message: Option<String>,
    pub artifact: Option<JobArtifactSummary>,
    pub tasks: Vec<TaskSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgressSummary {
    pub total_required_tasks: i64,
    pub completed_required_tasks: i64,
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobArtifactSummary {
    pub id: String,
    pub artifact_kind: String,
    pub uri: String,
    pub label: String,
    pub metadata: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub key: String,
    pub label: String,
    pub status: String,
    pub required: bool,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreflightRequest {
    pub backup_id: String,
    pub confirmation: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupManifest {
    schema_version: String,
    backup_id: String,
    created_at: String,
    source_paths: BackupSourcePaths,
    archive: BackupArchiveEvidence,
    database: DatabaseEvidence,
    file_scan: FileScanEvidence,
    integrity: IntegrityEvidence,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupSourcePaths {
    data_dir: String,
    database_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupArchiveEvidence {
    archive_path: String,
    database_snapshot_path: String,
    manifest_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DatabaseEvidence {
    source_size_bytes: u64,
    snapshot_size_bytes: u64,
    checksum: String,
    integrity_check: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileScanEvidence {
    scanned_files: Vec<String>,
    excluded_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IntegrityEvidence {
    checksum_algorithm: String,
    database_snapshot_checksum: String,
    manifest_checksum: Option<String>,
}

struct BackupLock {
    path: PathBuf,
}

impl Drop for BackupLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

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

fn complete_backup_job(connection: &Connection, db_path: &Path, job_id: &str) -> Result<()> {
    let data_dir = data_dir_for(db_path);
    let backups_dir = data_dir.join("backups");
    fs::create_dir_all(&backups_dir)?;
    let backup_id = format!("backup_{}", Uuid::new_v4());
    let archive_path = backups_dir.join(&backup_id);
    let database_snapshot_path = archive_path.join("local.db");
    let manifest_path = archive_path.join("manifest.json");

    set_job_running(connection, job_id)?;
    run_task(
        connection,
        job_id,
        "boundary.check",
        json!({
            "dataDir": path_string(&data_dir),
            "databasePath": path_string(db_path),
            "backupsDir": path_string(&backups_dir),
        }),
    )?;
    let backup_lock = acquire_backup_lock(&backups_dir)?;
    run_task(
        connection,
        job_id,
        "lock.acquire",
        json!({ "lockPath": path_string(&backup_lock.path) }),
    )?;

    let integrity_check = sqlite_integrity_check(connection)?;
    fs::create_dir_all(&archive_path)?;
    snapshot_database(connection, &database_snapshot_path).with_context(|| {
        format!(
            "Failed to snapshot SQLite from {} to {}",
            db_path.display(),
            database_snapshot_path.display()
        )
    })?;
    let database_checksum = checksum_file(&database_snapshot_path)?;
    let source_size_bytes = fs::metadata(db_path)?.len();
    let snapshot_size_bytes = fs::metadata(&database_snapshot_path)?.len();
    run_task(
        connection,
        job_id,
        "sqlite.snapshot",
        json!({
            "snapshotPath": path_string(&database_snapshot_path),
            "sourceSizeBytes": source_size_bytes,
            "snapshotSizeBytes": snapshot_size_bytes,
            "checksum": database_checksum,
            "integrityCheck": integrity_check,
        }),
    )?;

    let file_scan = scan_data_dir(&data_dir, db_path, &backups_dir)?;
    let scanned_files = file_scan.scanned_files.clone();
    let excluded_paths = file_scan.excluded_paths.clone();
    run_task(
        connection,
        job_id,
        "files.scan",
        json!({ "scannedFiles": scanned_files, "excludedPaths": excluded_paths }),
    )?;
    run_task(
        connection,
        job_id,
        "archive.write",
        json!({
            "backupId": backup_id,
            "archivePath": path_string(&archive_path),
            "databaseSnapshotPath": path_string(&database_snapshot_path),
        }),
    )?;

    let manifest = BackupManifest {
        schema_version: "1".to_string(),
        backup_id: backup_id.clone(),
        created_at: Utc::now().to_rfc3339(),
        source_paths: BackupSourcePaths {
            data_dir: path_string(&data_dir),
            database_path: path_string(db_path),
        },
        archive: BackupArchiveEvidence {
            archive_path: path_string(&archive_path),
            database_snapshot_path: path_string(&database_snapshot_path),
            manifest_path: path_string(&manifest_path),
        },
        database: DatabaseEvidence {
            source_size_bytes,
            snapshot_size_bytes,
            checksum: database_checksum.clone(),
            integrity_check,
        },
        file_scan,
        integrity: IntegrityEvidence {
            checksum_algorithm: CHECKSUM_ALGORITHM.to_string(),
            database_snapshot_checksum: database_checksum.clone(),
            manifest_checksum: None,
        },
    };
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    let manifest_checksum = checksum_file(&manifest_path)?;
    run_task(
        connection,
        job_id,
        "manifest.write",
        json!({
            "manifestPath": path_string(&manifest_path),
            "manifestChecksum": manifest_checksum,
        }),
    )?;

    verify_backup_manifest(&manifest_path)?;
    run_task(
        connection,
        job_id,
        "integrity.verify",
        json!({
            "checksumAlgorithm": CHECKSUM_ALGORITHM,
            "databaseSnapshotChecksum": database_checksum,
            "manifestChecksum": manifest_checksum,
        }),
    )?;

    insert_job_artifact(
        connection,
        job_id,
        Some("backup.record"),
        "backup.archive",
        &path_string(&archive_path),
        "Backup archive",
        json!({
            "backupId": backup_id,
            "archivePath": path_string(&archive_path),
            "manifestPath": path_string(&manifest_path),
            "databaseSnapshotPath": path_string(&database_snapshot_path),
            "checksumAlgorithm": CHECKSUM_ALGORITHM,
            "databaseSnapshotChecksum": checksum_file(&database_snapshot_path)?,
            "manifestChecksum": checksum_file(&manifest_path)?,
        }),
    )?;
    run_task(
        connection,
        job_id,
        "backup.record",
        json!({ "backupId": backup_id, "artifactKind": "backup.archive" }),
    )?;
    mark_job_succeeded(connection, job_id, json!({ "backupId": backup_id }))?;
    drop(backup_lock);
    Ok(())
}

fn complete_restore_preflight_job(
    connection: &Connection,
    db_path: &Path,
    job_id: &str,
    request: RestorePreflightRequest,
) -> Result<()> {
    set_job_running(connection, job_id)?;
    let backup_artifact = load_backup_artifact(connection, &request.backup_id)?;
    run_task(
        connection,
        job_id,
        "request.validate",
        json!({ "backupId": request.backup_id, "artifactId": backup_artifact.id }),
    )?;

    let manifest_path = backup_artifact
        .metadata
        .get("manifestPath")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Backup artifact is missing manifestPath metadata"))?;
    let manifest = verify_backup_manifest(Path::new(manifest_path))?;
    run_task(
        connection,
        job_id,
        "archive.verify",
        json!({
            "backupId": manifest.backup_id,
            "manifestPath": manifest_path,
            "checksumAlgorithm": CHECKSUM_ALGORITHM,
        }),
    )?;

    let expected_confirmation = format!("RESTORE {}", request.backup_id);
    if request.confirmation != expected_confirmation {
        bail!("Restore confirmation did not match {expected_confirmation}");
    }
    run_task(
        connection,
        job_id,
        "confirmation.require",
        json!({ "accepted": true }),
    )?;

    let safety_record_path =
        backups_dir_for(db_path)?.join(format!("restore_safety_{job_id}.json"));
    fs::write(
        &safety_record_path,
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": "1",
            "jobId": job_id,
            "backupId": request.backup_id,
            "createdAt": Utc::now().to_rfc3339(),
            "policy": "non_destructive_preflight_only",
        }))?,
    )?;
    insert_job_artifact(
        connection,
        job_id,
        Some("safety.backup"),
        "restore.safety_record",
        &path_string(&safety_record_path),
        "Restore safety record",
        json!({
            "backupId": request.backup_id,
            "safetyRecordPath": path_string(&safety_record_path),
            "destructiveRestoreExecuted": false,
        }),
    )?;
    run_task(
        connection,
        job_id,
        "safety.backup",
        json!({
            "safetyRecordPath": path_string(&safety_record_path),
            "destructiveRestoreExecuted": false,
        }),
    )?;

    mark_task_waiting_for_input(
        connection,
        job_id,
        "lock.acquire",
        "Destructive restore is intentionally stopped before live data replacement.",
    )?;
    for task_key in [
        "sqlite.restore",
        "files.restore",
        "state.verify",
        "app.restart",
        "restore.record",
    ] {
        mark_task_blocked(
            connection,
            job_id,
            task_key,
            "Blocked by non-destructive Phase 4 restore boundary.",
        )?;
    }
    connection.execute(
        "UPDATE jobs SET status = 'waiting_for_input', current_task_key = 'lock.acquire', updated_at = ?1 WHERE id = ?2",
        params![Utc::now().to_rfc3339(), job_id],
    )?;
    append_job_event(
        connection,
        job_id,
        None,
        "restore.preflight.completed",
        json!({ "backupId": request.backup_id, "destructiveRestoreExecuted": false }),
    )?;
    Ok(())
}

fn set_job_running(connection: &Connection, job_id: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE jobs SET status = 'running', started_at = COALESCE(started_at, ?1), updated_at = ?1 WHERE id = ?2",
        params![now, job_id],
    )?;
    append_job_event(connection, job_id, None, "job.started", json!({}))?;
    Ok(())
}

fn run_task(connection: &Connection, job_id: &str, task_key: &str, output: Value) -> Result<()> {
    mark_task_running(connection, job_id, task_key)?;
    mark_task_succeeded(connection, job_id, task_key, output)
}

fn mark_task_running(connection: &Connection, job_id: &str, task_key: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let status: String = connection.query_row(
        "SELECT status FROM job_tasks WHERE job_id = ?1 AND task_key = ?2",
        params![job_id, task_key],
        |row| row.get(0),
    )?;
    if status == "pending" {
        connection.execute(
            "UPDATE job_tasks SET status = 'ready', updated_at = ?1 WHERE job_id = ?2 AND task_key = ?3",
            params![now, job_id, task_key],
        )?;
        append_job_event(
            connection,
            job_id,
            Some(task_key),
            "task.ready",
            json!({ "taskKey": task_key }),
        )?;
    }
    connection.execute(
        "UPDATE job_tasks
         SET status = 'running', attempt_count = attempt_count + 1,
             started_at = COALESCE(started_at, ?1), updated_at = ?1
         WHERE job_id = ?2 AND task_key = ?3",
        params![now, job_id, task_key],
    )?;
    connection.execute(
        "UPDATE jobs SET status = 'running', current_task_key = ?1, updated_at = ?2 WHERE id = ?3",
        params![task_key, now, job_id],
    )?;
    append_job_event(
        connection,
        job_id,
        Some(task_key),
        "task.started",
        json!({ "taskKey": task_key }),
    )?;
    Ok(())
}

fn mark_task_succeeded(
    connection: &Connection,
    job_id: &str,
    task_key: &str,
    output: Value,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE job_tasks
         SET status = 'succeeded', output_json = ?1, completed_at = ?2, updated_at = ?2
         WHERE job_id = ?3 AND task_key = ?4",
        params![output.to_string(), now, job_id, task_key],
    )?;
    update_completed_required_count(connection, job_id)?;
    append_job_event(
        connection,
        job_id,
        Some(task_key),
        "task.succeeded",
        json!({ "taskKey": task_key, "output": output }),
    )?;
    Ok(())
}

fn mark_task_waiting_for_input(
    connection: &Connection,
    job_id: &str,
    task_key: &str,
    message: &str,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE job_tasks SET status = 'waiting_for_input', updated_at = ?1, error_message = ?2 WHERE job_id = ?3 AND task_key = ?4",
        params![now, message, job_id, task_key],
    )?;
    append_job_event(
        connection,
        job_id,
        Some(task_key),
        "task.waiting_for_input",
        json!({ "taskKey": task_key, "message": message }),
    )?;
    Ok(())
}

fn mark_task_blocked(
    connection: &Connection,
    job_id: &str,
    task_key: &str,
    message: &str,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE job_tasks SET status = 'blocked', updated_at = ?1, error_message = ?2 WHERE job_id = ?3 AND task_key = ?4",
        params![now, message, job_id, task_key],
    )?;
    append_job_event(
        connection,
        job_id,
        Some(task_key),
        "task.blocked",
        json!({ "taskKey": task_key, "message": message }),
    )?;
    Ok(())
}

fn mark_job_succeeded(connection: &Connection, job_id: &str, payload: Value) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE jobs
         SET status = 'succeeded', current_task_key = NULL, completed_at = ?1, updated_at = ?1,
             completed_required_task_count = required_task_count
         WHERE id = ?2",
        params![now, job_id],
    )?;
    append_job_event(connection, job_id, None, "job.succeeded", payload)?;
    Ok(())
}

fn mark_job_failed(connection: &Connection, job_id: &str, message: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE jobs
         SET status = 'failed', current_task_key = NULL, completed_at = ?1, updated_at = ?1, failure_message = ?2
         WHERE id = ?3",
        params![now, message, job_id],
    )?;
    append_job_event(
        connection,
        job_id,
        None,
        "job.failed",
        json!({ "message": message }),
    )?;
    Ok(())
}

fn update_completed_required_count(connection: &Connection, job_id: &str) -> Result<()> {
    let completed_required_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM job_tasks WHERE job_id = ?1 AND required = 1 AND status IN ('succeeded', 'skipped')",
        [job_id],
        |row| row.get(0),
    )?;
    connection.execute(
        "UPDATE jobs SET completed_required_task_count = ?1, updated_at = ?2 WHERE id = ?3",
        params![completed_required_count, Utc::now().to_rfc3339(), job_id],
    )?;
    Ok(())
}

fn insert_job_artifact(
    connection: &Connection,
    job_id: &str,
    task_key: Option<&str>,
    artifact_kind: &str,
    uri: &str,
    label: &str,
    metadata: Value,
) -> Result<String> {
    let artifact_id = format!("artifact_{}", Uuid::new_v4());
    connection.execute(
        "INSERT INTO job_artifacts (id, job_id, task_key, artifact_kind, uri, label, metadata_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            artifact_id,
            job_id,
            task_key,
            artifact_kind,
            uri,
            label,
            metadata.to_string(),
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(artifact_id)
}

fn load_job_summary(connection: &Connection, job_id: &str) -> Result<BackupRestoreJobSummary> {
    let row = connection.query_row(
        "SELECT id, template_id, kind, status, current_task_key, required_task_count,
                completed_required_task_count, started_at, completed_at, created_at, updated_at, failure_message
         FROM jobs WHERE id = ?1",
        [job_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, Option<String>>(11)?,
            ))
        },
    )?;
    let (
        id,
        template_id,
        kind,
        status,
        current_task_key,
        required_task_count,
        completed_required_task_count,
        started_at,
        completed_at,
        created_at,
        updated_at,
        failure_message,
    ) = row;
    let percent = if required_task_count == 0 {
        0
    } else {
        ((completed_required_task_count * 100) / required_task_count) as u8
    };
    let operation = match template_id.as_str() {
        BACKUP_TEMPLATE_ID => "backup".to_string(),
        RESTORE_TEMPLATE_ID => "restore".to_string(),
        other => other.to_string(),
    };

    Ok(BackupRestoreJobSummary {
        id: id.clone(),
        operation,
        kind,
        status,
        progress: JobProgressSummary {
            total_required_tasks: required_task_count,
            completed_required_tasks: completed_required_task_count,
            percent,
        },
        current_task_key,
        elapsed_seconds: elapsed_seconds(started_at.as_deref(), completed_at.as_deref()),
        started_at,
        completed_at,
        created_at,
        updated_at,
        failure_message,
        artifact: load_latest_artifact(connection, &id)?,
        tasks: load_task_summaries(connection, &id)?,
    })
}

fn load_task_summaries(connection: &Connection, job_id: &str) -> Result<Vec<TaskSummary>> {
    let mut statement = connection.prepare(
        "SELECT task_key, label, status, required, started_at, completed_at, error_message
         FROM job_tasks WHERE job_id = ?1 ORDER BY created_at ASC",
    )?;
    let rows = statement.query_map([job_id], |row| {
        Ok(TaskSummary {
            key: row.get(0)?,
            label: row.get(1)?,
            status: row.get(2)?,
            required: row.get::<_, i64>(3)? == 1,
            started_at: row.get(4)?,
            completed_at: row.get(5)?,
            error_message: row.get(6)?,
        })
    })?;
    let mut tasks = Vec::new();
    for row in rows {
        tasks.push(row?);
    }
    Ok(tasks)
}

fn load_latest_artifact(
    connection: &Connection,
    job_id: &str,
) -> Result<Option<JobArtifactSummary>> {
    connection
        .query_row(
            "SELECT id, artifact_kind, uri, label, metadata_json, created_at
             FROM job_artifacts WHERE job_id = ?1 ORDER BY created_at DESC LIMIT 1",
            [job_id],
            |row| {
                let metadata_json: String = row.get(4)?;
                Ok(JobArtifactSummary {
                    id: row.get(0)?,
                    artifact_kind: row.get(1)?,
                    uri: row.get(2)?,
                    label: row.get(3)?,
                    metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
                    created_at: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn load_backup_artifact(connection: &Connection, backup_id: &str) -> Result<JobArtifactSummary> {
    let mut statement = connection.prepare(
        "SELECT id, artifact_kind, uri, label, metadata_json, created_at
         FROM job_artifacts WHERE artifact_kind = 'backup.archive'
         ORDER BY created_at DESC",
    )?;
    let rows = statement.query_map([], |row| {
        let metadata_json: String = row.get(4)?;
        Ok(JobArtifactSummary {
            id: row.get(0)?,
            artifact_kind: row.get(1)?,
            uri: row.get(2)?,
            label: row.get(3)?,
            metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
            created_at: row.get(5)?,
        })
    })?;
    for row in rows {
        let artifact = row?;
        let matches_id = artifact
            .metadata
            .get("backupId")
            .and_then(Value::as_str)
            .map(|value| value == backup_id)
            .unwrap_or(false);
        if matches_id {
            return Ok(artifact);
        }
    }
    bail!("Unknown backup artifact: {backup_id}")
}

fn verify_backup_manifest(manifest_path: &Path) -> Result<BackupManifest> {
    let manifest_json = fs::read_to_string(manifest_path)?;
    let manifest: BackupManifest = serde_json::from_str(&manifest_json)?;
    let database_snapshot_path = Path::new(&manifest.archive.database_snapshot_path);
    let checksum = checksum_file(database_snapshot_path)?;
    if checksum != manifest.integrity.database_snapshot_checksum {
        bail!(
            "Backup database checksum mismatch for {}",
            manifest.backup_id
        );
    }
    Ok(manifest)
}

fn sqlite_integrity_check(connection: &Connection) -> Result<String> {
    let result: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    Ok(result)
}

fn snapshot_database(connection: &Connection, target_path: &Path) -> Result<()> {
    connection.execute("VACUUM main INTO ?1", [path_string(target_path)])?;
    Ok(())
}

fn scan_data_dir(data_dir: &Path, db_path: &Path, backups_dir: &Path) -> Result<FileScanEvidence> {
    let mut scanned_files = Vec::new();
    let mut excluded_paths = vec![path_string(db_path), path_string(backups_dir)];
    if data_dir.exists() {
        for entry in fs::read_dir(data_dir)? {
            let path = entry?.path();
            if path == db_path || path == backups_dir || path.starts_with(backups_dir) {
                continue;
            }
            if path.is_file() {
                scanned_files.push(path_string(&path));
            } else if path.is_dir() {
                excluded_paths.push(path_string(&path));
            }
        }
    }
    Ok(FileScanEvidence {
        scanned_files,
        excluded_paths,
    })
}

fn acquire_backup_lock(backups_dir: &Path) -> Result<BackupLock> {
    let path = backups_dir.join(".backup.lock");
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(|| format!("Backup lock is already held at {}", path.display()))?;
    Ok(BackupLock { path })
}

fn checksum_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut checksum: u64 = 0xcbf29ce484222325;
    let mut buffer = [0_u8; 8192];
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        for byte in &buffer[..bytes_read] {
            checksum ^= u64::from(*byte);
            checksum = checksum.wrapping_mul(0x100000001b3);
        }
    }
    Ok(format!("{CHECKSUM_ALGORITHM}:{checksum:016x}"))
}

fn data_dir_for(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn backups_dir_for(db_path: &Path) -> Result<PathBuf> {
    let backups_dir = data_dir_for(db_path).join("backups");
    fs::create_dir_all(&backups_dir)?;
    Ok(backups_dir)
}

fn elapsed_seconds(started_at: Option<&str>, completed_at: Option<&str>) -> Option<i64> {
    let started_at = started_at?;
    let started = DateTime::parse_from_rfc3339(started_at)
        .ok()?
        .with_timezone(&Utc);
    let ended = completed_at
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    Some((ended - started).num_seconds().max(0))
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_database;
    use tempfile::TempDir;

    fn test_db_path(temp_dir: &TempDir) -> PathBuf {
        temp_dir.path().join("local.db")
    }

    #[test]
    fn create_backup_writes_manifest_artifact_and_progress() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = test_db_path(&temp_dir);
        init_database(&db_path).unwrap();

        let job = create_backup(&db_path, "test", Some("tester")).unwrap();
        let artifact = job.artifact.as_ref().unwrap();
        let manifest_path = artifact.metadata["manifestPath"].as_str().unwrap();

        assert_eq!(job.status, "succeeded");
        assert_eq!(job.progress.percent, 100);
        assert_eq!(artifact.artifact_kind, "backup.archive");
        assert!(Path::new(manifest_path).exists());
        assert_eq!(job.tasks.len(), 8);
    }

    #[test]
    fn restore_preflight_requires_confirmation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = test_db_path(&temp_dir);
        init_database(&db_path).unwrap();
        let backup_job = create_backup(&db_path, "test", None).unwrap();
        let backup_id = backup_job.artifact.unwrap().metadata["backupId"]
            .as_str()
            .unwrap()
            .to_string();

        let result = run_restore_preflight(
            &db_path,
            RestorePreflightRequest {
                backup_id,
                confirmation: "no".to_string(),
            },
            "test",
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn restore_preflight_stops_before_destructive_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = test_db_path(&temp_dir);
        init_database(&db_path).unwrap();
        let backup_job = create_backup(&db_path, "test", None).unwrap();
        let backup_id = backup_job.artifact.unwrap().metadata["backupId"]
            .as_str()
            .unwrap()
            .to_string();

        let restore_job = run_restore_preflight(
            &db_path,
            RestorePreflightRequest {
                backup_id: backup_id.clone(),
                confirmation: format!("RESTORE {backup_id}"),
            },
            "test",
            None,
        )
        .unwrap();

        assert_eq!(restore_job.status, "waiting_for_input");
        assert_eq!(restore_job.progress.completed_required_tasks, 4);
        assert_eq!(
            restore_job.current_task_key.as_deref(),
            Some("lock.acquire")
        );
        assert!(restore_job.artifact.is_some());
    }
}
