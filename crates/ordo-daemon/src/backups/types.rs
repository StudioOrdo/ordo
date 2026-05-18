use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub(crate) const CHECKSUM_ALGORITHM: &str = "sha256";
pub(crate) const CHECKSUM_ALGORITHM_VERSION: &str = "1";
pub(crate) const BACKUP_TEMPLATE_ID: &str = "backup.create";
pub(crate) const RESTORE_TEMPLATE_ID: &str = "restore.execute";

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BackupManifest {
    pub(crate) schema_version: String,
    pub(crate) backup_id: String,
    pub(crate) created_at: String,
    pub(crate) source_paths: BackupSourcePaths,
    pub(crate) archive: BackupArchiveEvidence,
    pub(crate) database: DatabaseEvidence,
    pub(crate) file_scan: FileScanEvidence,
    pub(crate) integrity: IntegrityEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BackupSourcePaths {
    pub(crate) data_dir: String,
    pub(crate) database_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BackupArchiveEvidence {
    pub(crate) archive_path: String,
    pub(crate) database_snapshot_path: String,
    pub(crate) manifest_path: String,
    pub(crate) archived_files: Vec<ArchivedFileEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArchivedFileEvidence {
    pub(crate) source_path: String,
    pub(crate) archive_path: String,
    pub(crate) size_bytes: u64,
    pub(crate) checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DatabaseEvidence {
    pub(crate) source_size_bytes: u64,
    pub(crate) snapshot_size_bytes: u64,
    pub(crate) checksum: String,
    pub(crate) integrity_check: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileScanEvidence {
    pub(crate) scanned_files: Vec<String>,
    pub(crate) excluded_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrityEvidence {
    pub(crate) checksum_algorithm: String,
    pub(crate) checksum_algorithm_version: String,
    pub(crate) database_snapshot_checksum: String,
    pub(crate) manifest_checksum: Option<String>,
}

pub(crate) struct BackupLock {
    pub(crate) path: PathBuf,
}

impl Drop for BackupLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
