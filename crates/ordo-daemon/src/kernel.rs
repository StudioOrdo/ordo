use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

use crate::capabilities::assert_capability_ids_registered;
use crate::diagnostics::{diagnostic_log, insert_diagnostic_log_connection, NewDiagnosticLogEntry};
use crate::events::{append_realtime_event, append_realtime_event_tx, job_event};
use crate::json_contracts::validate_json_value;
use crate::security::redaction;
use crate::templates::{ProcessTemplate, TaskDefinition};

#[derive(Debug, Clone, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Visited,
}

#[derive(Debug, Clone)]
pub struct TaskState {
    pub key: String,
    pub required: bool,
    pub status: String,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub total_required_tasks: usize,
    pub completed_required_tasks: usize,
    pub percent: u8,
    pub current_task_key: Option<String>,
    pub elapsed_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedTask {
    pub job_id: String,
    pub task_key: String,
    pub worker_id: String,
    pub attempt_count: i64,
    pub claimed_at: DateTime<Utc>,
    pub lease_expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskRetryOutcome {
    Retrying,
    Exhausted,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResultEnvelope {
    pub status: String,
    pub summary: String,
    pub safe_output: Value,
    pub artifact_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
}

pub fn validate_task_dag(tasks: &[TaskDefinition]) -> Result<()> {
    let mut task_keys = BTreeSet::new();
    for task_definition in tasks {
        if !task_keys.insert(task_definition.key.clone()) {
            bail!("Duplicate task key: {}", task_definition.key);
        }
    }

    for task_definition in tasks {
        for dependency in &task_definition.depends_on {
            if !task_keys.contains(dependency) {
                bail!(
                    "Task {} depends on missing task {}",
                    task_definition.key,
                    dependency
                );
            }
        }
    }

    let task_map: BTreeMap<String, Vec<String>> = tasks
        .iter()
        .map(|task_definition| {
            (
                task_definition.key.clone(),
                task_definition.depends_on.clone(),
            )
        })
        .collect();
    let mut visit_state = BTreeMap::new();

    for task_key in task_map.keys() {
        visit_task(task_key, &task_map, &mut visit_state)?;
    }

    Ok(())
}

fn visit_task(
    task_key: &str,
    task_map: &BTreeMap<String, Vec<String>>,
    visit_state: &mut BTreeMap<String, VisitState>,
) -> Result<()> {
    match visit_state.get(task_key) {
        Some(VisitState::Visited) => return Ok(()),
        Some(VisitState::Visiting) => bail!("Task dependency cycle includes {task_key}"),
        None => {}
    }

    visit_state.insert(task_key.to_string(), VisitState::Visiting);
    for dependency in task_map.get(task_key).into_iter().flatten() {
        visit_task(dependency, task_map, visit_state)?;
    }
    visit_state.insert(task_key.to_string(), VisitState::Visited);
    Ok(())
}

pub fn ready_task_keys(tasks: &[TaskState]) -> Vec<String> {
    let status_by_key: BTreeMap<String, String> = tasks
        .iter()
        .map(|task_state| (task_state.key.clone(), task_state.status.clone()))
        .collect();

    tasks
        .iter()
        .filter(|task_state| task_state.status == "pending")
        .filter(|task_state| {
            task_state.depends_on.iter().all(|dependency| {
                matches!(
                    status_by_key.get(dependency).map(String::as_str),
                    Some("succeeded") | Some("skipped")
                )
            })
        })
        .map(|task_state| task_state.key.clone())
        .collect()
}

pub fn calculate_progress(
    tasks: &[TaskState],
    started_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> JobProgress {
    let total_required_tasks = tasks
        .iter()
        .filter(|task_state| task_state.required)
        .count();
    let completed_required_tasks = tasks
        .iter()
        .filter(|task_state| task_state.required)
        .filter(|task_state| task_state.status == "succeeded" || task_state.status == "skipped")
        .count();
    let percent = if total_required_tasks == 0 {
        0
    } else {
        ((completed_required_tasks * 100) / total_required_tasks) as u8
    };
    let current_task_key = tasks
        .iter()
        .find(|task_state| task_state.status == "running")
        .or_else(|| tasks.iter().find(|task_state| task_state.status == "ready"))
        .map(|task_state| task_state.key.clone());

    JobProgress {
        total_required_tasks,
        completed_required_tasks,
        percent,
        current_task_key,
        elapsed_seconds: started_at.map(|started| (now - started).num_seconds().max(0)),
    }
}

pub fn create_job_from_template(
    connection: &mut Connection,
    template: &ProcessTemplate,
    origin: &str,
    actor_id: Option<&str>,
    input: Value,
) -> Result<String> {
    create_job_from_template_with_idempotency(connection, template, origin, actor_id, input, None)
}

pub fn create_job_from_template_with_idempotency(
    connection: &mut Connection,
    template: &ProcessTemplate,
    origin: &str,
    actor_id: Option<&str>,
    input: Value,
    idempotency_key: Option<&str>,
) -> Result<String> {
    validate_task_dag(&template.tasks)?;
    validate_job_capabilities(connection, template)?;
    validate_template_variables(template, &input)?;

    let idempotency_key = normalize_job_idempotency_key(idempotency_key)?;
    let required_task_count = template
        .tasks
        .iter()
        .filter(|task_definition| task_definition.required)
        .count();
    let compiled_plan = compile_plan_snapshot(template, &input);
    let idempotency_request_json = idempotency_key.as_ref().map(|_| {
        canonical_json_string(&json!({
            "template": {
                "id": template.id,
                "version": template.version,
                "kind": template.kind,
                "capabilityId": template.effective_capability_id(),
            },
            "origin": origin,
            "actorId": actor_id,
            "input": input,
            "compiledPlan": compiled_plan,
        }))
    });

    let transaction = connection.transaction()?;
    if let Some(key) = idempotency_key.as_deref() {
        if let Some(existing) = load_job_idempotency_record(
            &transaction,
            &template.id,
            template.version,
            origin,
            actor_id,
            key,
        )? {
            if Some(existing.request_json.as_str()) == idempotency_request_json.as_deref() {
                transaction.commit()?;
                return Ok(existing.job_id);
            }
            bail!("Job idempotency key conflicts with a different request");
        }
    }

    let now = Utc::now().to_rfc3339();
    let job_id = format!("job_{}", Uuid::new_v4());

    transaction.execute(
        "INSERT INTO jobs (
            id, template_id, template_version, capability_id, kind, status, origin, actor_id, input_json,
            compiled_plan_json, idempotency_key, idempotency_request_json, current_task_key,
            required_task_count, completed_required_task_count, started_at, completed_at, created_at,
            updated_at, failure_message
         ) VALUES (?1, ?2, ?3, ?4, ?5, 'queued', ?6, ?7, ?8, ?9, ?10, ?11, NULL, ?12, 0, NULL, NULL, ?13, ?13, NULL)",
        params![
            job_id,
            template.id,
            template.version,
            template.effective_capability_id(),
            template.kind,
            origin,
            actor_id,
            input.to_string(),
            compiled_plan.to_string(),
            idempotency_key.as_deref(),
            idempotency_request_json.as_deref(),
            required_task_count as i64,
            now,
        ],
    )?;

    for task_definition in &template.tasks {
        insert_task(&transaction, &job_id, task_definition, &now)?;
    }

    for task_definition in &template.tasks {
        for dependency in &task_definition.depends_on {
            transaction.execute(
                "INSERT INTO job_task_dependencies (job_id, task_key, depends_on_task_key)
                 VALUES (?1, ?2, ?3)",
                params![job_id, task_definition.key, dependency],
            )?;
        }
    }

    append_job_event_tx(
        &transaction,
        &job_id,
        None,
        "job.created",
        json!({
            "templateId": template.id,
            "templateVersion": template.version,
            "kind": template.kind,
            "origin": origin,
            "compiledPlan": {
                "templateId": template.id,
                "templateVersion": template.version,
                "taskCount": template.tasks.len(),
                "requiredTaskCount": required_task_count,
            },
        }),
    )?;

    let initial_task_states: Vec<TaskState> = template
        .tasks
        .iter()
        .map(|task_definition| TaskState {
            key: task_definition.key.clone(),
            required: task_definition.required,
            status: "pending".to_string(),
            depends_on: task_definition.depends_on.clone(),
        })
        .collect();

    for task_key in ready_task_keys(&initial_task_states) {
        transaction.execute(
            "UPDATE job_tasks SET status = 'ready', updated_at = ?1 WHERE job_id = ?2 AND task_key = ?3",
            params![now, job_id, task_key],
        )?;
        append_job_event_tx(
            &transaction,
            &job_id,
            Some(&task_key),
            "task.ready",
            json!({ "taskKey": task_key }),
        )?;
    }

    transaction.commit()?;
    Ok(job_id)
}

#[derive(Debug, Clone)]
struct JobIdempotencyRecord {
    job_id: String,
    request_json: String,
}

fn normalize_job_idempotency_key(idempotency_key: Option<&str>) -> Result<Option<String>> {
    let Some(key) = idempotency_key else {
        return Ok(None);
    };
    let key = key.trim();
    if key.is_empty() {
        bail!("Job idempotency key cannot be blank");
    }
    if key.len() > 200 {
        bail!("Job idempotency key is too long");
    }
    Ok(Some(key.to_string()))
}

fn load_job_idempotency_record(
    connection: &Connection,
    template_id: &str,
    template_version: i64,
    origin: &str,
    actor_id: Option<&str>,
    idempotency_key: &str,
) -> Result<Option<JobIdempotencyRecord>> {
    connection
        .query_row(
            "SELECT id, idempotency_request_json
             FROM jobs
             WHERE template_id = ?1
               AND template_version = ?2
               AND origin = ?3
               AND COALESCE(actor_id, '') = COALESCE(?4, '')
               AND idempotency_key = ?5
             ORDER BY created_at, id
             LIMIT 1",
            params![
                template_id,
                template_version,
                origin,
                actor_id,
                idempotency_key,
            ],
            |row| {
                Ok(JobIdempotencyRecord {
                    job_id: row.get(0)?,
                    request_json: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn validate_template_variables(template: &ProcessTemplate, input: &Value) -> Result<()> {
    validate_json_value(&template.variable_schema, input, "template variables")
}

fn compile_plan_snapshot(template: &ProcessTemplate, input: &Value) -> Value {
    json!({
        "schemaVersion": 1,
        "template": {
            "id": template.id,
            "version": template.version,
            "kind": template.kind,
            "capabilityId": template.effective_capability_id(),
            "name": template.name,
        },
        "variableSchema": template.variable_schema,
        "input": input,
        "tasks": template.tasks.iter().map(|task_definition| {
            json!({
                "key": task_definition.key,
                "kind": task_definition.kind,
                "capabilityId": task_definition.effective_capability_id(),
                "label": task_definition.label,
                "required": task_definition.required,
                "dependsOn": task_definition.depends_on,
                "input": task_definition.input,
                "retryPolicy": task_definition.retry_policy,
            })
        }).collect::<Vec<_>>(),
    })
}

fn canonical_json_string(value: &Value) -> String {
    canonical_json_value(value).to_string()
}

fn canonical_json_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonical_json_value).collect()),
        Value::Object(map) => {
            let mut sorted = serde_json::Map::new();
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                if let Some(value) = map.get(key) {
                    sorted.insert(key.clone(), canonical_json_value(value));
                }
            }
            Value::Object(sorted)
        }
        _ => value.clone(),
    }
}

fn validate_job_capabilities(connection: &Connection, template: &ProcessTemplate) -> Result<()> {
    let mut capability_ids = vec![template.effective_capability_id().to_string()];
    capability_ids.extend(
        template
            .tasks
            .iter()
            .map(|task_definition| task_definition.effective_capability_id().to_string()),
    );
    assert_capability_ids_registered(connection, &capability_ids)
}

fn insert_task(
    transaction: &Transaction,
    job_id: &str,
    task_definition: &TaskDefinition,
    now: &str,
) -> Result<()> {
    transaction.execute(
        "INSERT INTO job_tasks (
            id, job_id, task_key, capability_id, task_kind, label, required, status, input_json,
            retry_policy_json, output_json, attempt_count, started_at, completed_at, created_at, updated_at, error_message
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, ?9, NULL, 0, NULL, NULL, ?10, ?10, NULL)",
        params![
            format!("task_{}", Uuid::new_v4()),
            job_id,
            task_definition.key,
            task_definition.effective_capability_id(),
            task_definition.kind,
            task_definition.label,
            if task_definition.required { 1 } else { 0 },
            task_definition.input.to_string(),
            task_definition.retry_policy.to_string(),
            now,
        ],
    )?;
    Ok(())
}

pub fn claim_ready_task(
    connection: &mut Connection,
    job_id: &str,
    worker_id: &str,
    lease_seconds: i64,
    now: DateTime<Utc>,
) -> Result<Option<ClaimedTask>> {
    if worker_id.trim().is_empty() {
        bail!("worker_id is required");
    }
    if lease_seconds <= 0 {
        bail!("lease_seconds must be positive");
    }

    let transaction = connection.transaction()?;
    let job_status = job_status(&transaction, job_id)?;
    if !matches!(job_status.as_deref(), Some("queued") | Some("running")) {
        transaction.commit()?;
        return Ok(None);
    }

    let task_key = next_claimable_task_key(&transaction, job_id)?;
    let Some(task_key) = task_key else {
        transaction.commit()?;
        return Ok(None);
    };

    let claimed_at = now.to_rfc3339();
    let lease_expires_at = (now + Duration::seconds(lease_seconds)).to_rfc3339();
    let updated = transaction.execute(
        "UPDATE job_tasks
         SET status = 'running',
             attempt_count = attempt_count + 1,
             claimed_at = ?1,
             started_at = COALESCE(started_at, ?1),
             lease_owner_id = ?2,
             lease_expires_at = ?3,
             updated_at = ?1,
             error_message = NULL
         WHERE job_id = ?4
           AND task_key = ?5
           AND status = 'ready'
           AND lease_owner_id IS NULL",
        params![claimed_at, worker_id, lease_expires_at, job_id, task_key],
    )?;

    if updated == 0 {
        transaction.commit()?;
        return Ok(None);
    }

    transaction.execute(
        "UPDATE jobs
         SET status = 'running',
             started_at = COALESCE(started_at, ?1),
             current_task_key = ?2,
             updated_at = ?1
         WHERE id = ?3 AND status IN ('queued', 'running')",
        params![claimed_at, task_key, job_id],
    )?;

    let attempt_count = task_attempt_count(&transaction, job_id, &task_key)?;
    append_job_event_tx(
        &transaction,
        job_id,
        Some(&task_key),
        "task.claimed",
        json!({
            "taskKey": task_key,
            "workerId": worker_id,
            "attemptCount": attempt_count,
            "leaseExpiresAt": lease_expires_at,
        }),
    )?;

    transaction.commit()?;
    Ok(Some(ClaimedTask {
        job_id: job_id.to_string(),
        task_key,
        worker_id: worker_id.to_string(),
        attempt_count,
        claimed_at: now,
        lease_expires_at: now + Duration::seconds(lease_seconds),
    }))
}

pub fn complete_leased_task(
    connection: &mut Connection,
    job_id: &str,
    task_key: &str,
    worker_id: &str,
    output: Value,
    now: DateTime<Utc>,
) -> Result<()> {
    complete_leased_task_with_result_envelope(
        connection,
        job_id,
        task_key,
        worker_id,
        legacy_task_result_envelope(job_id, task_key, output),
        now,
    )
}

pub fn complete_leased_task_with_result_envelope(
    connection: &mut Connection,
    job_id: &str,
    task_key: &str,
    worker_id: &str,
    envelope: TaskResultEnvelope,
    now: DateTime<Utc>,
) -> Result<()> {
    let transaction = connection.transaction()?;
    ensure_task_lease_owner(&transaction, job_id, task_key, worker_id)?;
    validate_task_result_envelope(&transaction, &envelope)?;
    let now = now.to_rfc3339();
    let was_required = task_required(&transaction, job_id, task_key)?;
    let output = task_result_envelope_json(&envelope);

    transaction.execute(
        "UPDATE job_tasks
         SET status = 'succeeded',
             output_json = ?1,
             completed_at = ?2,
             updated_at = ?2,
             lease_owner_id = NULL,
             lease_expires_at = NULL,
             error_message = NULL
         WHERE job_id = ?3 AND task_key = ?4",
        params![output.to_string(), now, job_id, task_key],
    )?;

    if was_required {
        transaction.execute(
            "UPDATE jobs
             SET completed_required_task_count = completed_required_task_count + 1,
                 updated_at = ?1
             WHERE id = ?2",
            params![now, job_id],
        )?;
    }

    append_job_event_tx(
        &transaction,
        job_id,
        Some(task_key),
        "task.succeeded",
        json!({
            "taskKey": task_key,
            "result": task_result_event_json(&envelope),
        }),
    )?;
    mark_newly_ready_tasks(&transaction, job_id, &now)?;
    complete_job_if_done(&transaction, job_id, &now)?;
    transaction.commit()?;
    Ok(())
}

fn legacy_task_result_envelope(job_id: &str, task_key: &str, output: Value) -> TaskResultEnvelope {
    TaskResultEnvelope {
        status: "succeeded".to_string(),
        summary: "Task completed with legacy output.".to_string(),
        safe_output: output,
        artifact_refs: Vec::new(),
        evidence_refs: vec![
            format!("job:{job_id}"),
            format!("job_task:{job_id}:{task_key}"),
        ],
        limitations: vec![
            "Legacy completion path; output was wrapped deterministically.".to_string(),
        ],
    }
}

fn task_result_envelope_json(envelope: &TaskResultEnvelope) -> Value {
    json!({
        "schemaVersion": 1,
        "status": envelope.status,
        "summary": envelope.summary,
        "safeOutput": envelope.safe_output,
        "artifactRefs": envelope.artifact_refs,
        "evidenceRefs": envelope.evidence_refs,
        "limitations": envelope.limitations,
    })
}

fn task_result_event_json(envelope: &TaskResultEnvelope) -> Value {
    json!({
        "schemaVersion": 1,
        "status": envelope.status,
        "summary": envelope.summary,
        "artifactRefs": envelope.artifact_refs,
        "evidenceRefs": envelope.evidence_refs,
        "limitations": envelope.limitations,
    })
}

fn validate_task_result_envelope(
    connection: &Connection,
    envelope: &TaskResultEnvelope,
) -> Result<()> {
    if envelope.status != "succeeded" {
        bail!("task result envelope status must be succeeded");
    }
    validate_task_result_text(&envelope.summary, false)?;
    validate_safe_output_value(&envelope.safe_output)?;
    validate_task_result_refs(&envelope.evidence_refs, "evidenceRefs", true)?;
    validate_task_result_refs(&envelope.artifact_refs, "artifactRefs", false)?;
    for artifact_ref in &envelope.artifact_refs {
        if !artifact_exists(connection, artifact_ref)? {
            bail!("task result envelope artifact ref is unknown");
        }
    }
    for limitation in &envelope.limitations {
        validate_task_result_text(limitation, true)?;
    }
    Ok(())
}

fn validate_task_result_refs(refs: &[String], label: &str, required: bool) -> Result<()> {
    if required && refs.is_empty() {
        bail!("task result envelope {label} are required");
    }
    if refs.len() > 50 {
        bail!("task result envelope {label} exceed the supported limit");
    }
    let mut seen = BTreeSet::new();
    for value in refs {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            bail!("task result envelope {label} cannot include blank refs");
        }
        if trimmed.len() > 200 {
            bail!("task result envelope {label} include an oversized ref");
        }
        if trimmed != value {
            bail!("task result envelope {label} refs must already be normalized");
        }
        if !seen.insert(trimmed.to_string()) {
            bail!("task result envelope {label} include duplicate refs");
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("publishing:")
            || lower.starts_with("provider:")
            || lower.starts_with("prompt:")
            || lower.starts_with("policy:")
            || lower.starts_with("analytics:")
            || lower.starts_with("metric:")
        {
            bail!("task result envelope {label} include unsupported claim refs");
        }
    }
    Ok(())
}

fn validate_safe_output_value(value: &Value) -> Result<()> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if is_forbidden_result_key(key) {
                    bail!("task result envelope safe output contains private or internal fields");
                }
                validate_safe_output_value(child)?;
            }
        }
        Value::Array(items) => {
            for child in items {
                validate_safe_output_value(child)?;
            }
        }
        Value::String(text) => validate_task_result_text(text, false)?,
        _ => {}
    }
    Ok(())
}

fn validate_task_result_text(value: &str, allow_empty: bool) -> Result<()> {
    let trimmed = value.trim();
    if !allow_empty && trimmed.is_empty() {
        bail!("task result envelope text is required");
    }
    if trimmed.len() > 2000 {
        bail!("task result envelope text exceeds the supported limit");
    }
    if redaction::contains_sensitive_text(trimmed, &[]) {
        bail!("task result envelope text contains sensitive content");
    }
    Ok(())
}

fn is_forbidden_result_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    [
        "private",
        "secret",
        "prompt",
        "provider",
        "policy",
        "staffrouting",
        "unsupportedclaim",
        "raw",
        "owneronly",
        "privateartifacttext",
    ]
    .iter()
    .any(|forbidden| normalized.contains(forbidden))
}

fn artifact_exists(connection: &Connection, artifact_id: &str) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT COUNT(*) FROM artifacts WHERE id = ?1",
        [artifact_id],
        |row| row.get::<_, i64>(0),
    )? == 1)
}

pub fn fail_leased_task_attempt(
    connection: &mut Connection,
    job_id: &str,
    task_key: &str,
    worker_id: &str,
    error_message: &str,
    now: DateTime<Utc>,
) -> Result<TaskRetryOutcome> {
    let transaction = connection.transaction()?;
    ensure_task_lease_owner(&transaction, job_id, task_key, worker_id)?;
    let outcome = fail_task_attempt_tx(
        &transaction,
        job_id,
        task_key,
        error_message,
        now,
        "task.failed",
    )?;
    transaction.commit()?;
    Ok(outcome)
}

pub fn expire_task_lease_for_retry(
    connection: &mut Connection,
    job_id: &str,
    task_key: &str,
    now: DateTime<Utc>,
    reason: &str,
) -> Result<TaskRetryOutcome> {
    let transaction = connection.transaction()?;
    let lease_expires_at: Option<String> = transaction
        .query_row(
            "SELECT lease_expires_at FROM job_tasks WHERE job_id = ?1 AND task_key = ?2 AND status = 'running'",
            params![job_id, task_key],
            |row| row.get(0),
        )
        .optional()?;
    let Some(lease_expires_at) = lease_expires_at else {
        bail!("task is not leased");
    };
    let lease_expires_at = DateTime::parse_from_rfc3339(&lease_expires_at)?.with_timezone(&Utc);
    if lease_expires_at > now {
        bail!("task lease has not expired");
    }

    let outcome = fail_task_attempt_tx(
        &transaction,
        job_id,
        task_key,
        reason,
        now,
        "task.lease_expired",
    )?;
    transaction.commit()?;
    Ok(outcome)
}

pub fn cancel_job(
    connection: &mut Connection,
    job_id: &str,
    actor_id: Option<&str>,
    reason: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    let transaction = connection.transaction()?;
    let now = now.to_rfc3339();
    let updated = transaction.execute(
        "UPDATE jobs
         SET status = 'canceled',
             completed_at = COALESCE(completed_at, ?1),
             failure_message = ?2,
             current_task_key = NULL,
             updated_at = ?1
         WHERE id = ?3 AND status NOT IN ('succeeded', 'failed', 'canceled')",
        params![now, reason, job_id],
    )?;
    if updated == 0 {
        transaction.commit()?;
        return Ok(());
    }

    let mut statement = transaction.prepare(
        "SELECT task_key FROM job_tasks
         WHERE job_id = ?1 AND status IN ('pending', 'ready', 'running')",
    )?;
    let cancelable_tasks = statement
        .query_map([job_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    transaction.execute(
        "UPDATE job_tasks
         SET status = 'canceled',
             completed_at = COALESCE(completed_at, ?1),
             updated_at = ?1,
             lease_owner_id = NULL,
             lease_expires_at = NULL,
             error_message = ?2
         WHERE job_id = ?3 AND status IN ('pending', 'ready', 'running')",
        params![now, reason, job_id],
    )?;

    append_job_event_tx(
        &transaction,
        job_id,
        None,
        "job.canceled",
        json!({
            "actorId": actor_id,
            "reason": reason,
        }),
    )?;
    for task_key in cancelable_tasks {
        append_job_event_tx(
            &transaction,
            job_id,
            Some(&task_key),
            "task.canceled",
            json!({ "taskKey": task_key, "reason": reason }),
        )?;
    }

    transaction.commit()?;
    Ok(())
}

pub fn append_job_event(
    connection: &Connection,
    job_id: &str,
    task_key: Option<&str>,
    event_type: &str,
    payload: Value,
) -> Result<i64> {
    let sequence = next_event_sequence(connection, job_id)?;
    let event = job_event(event_type, job_id, task_key, sequence, payload.clone());
    connection.execute(
        "INSERT INTO job_events (id, job_id, task_key, sequence, event_type, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            format!("event_{}", Uuid::new_v4()),
            job_id,
            task_key,
            sequence,
            event_type,
            payload.to_string(),
            event.occurred_at,
        ],
    )?;
    append_realtime_event(connection, &event)?;
    insert_diagnostic_log_connection(
        connection,
        job_event_log_entry(job_id, task_key, event_type, payload),
    )?;
    Ok(sequence)
}

fn job_status(connection: &Connection, job_id: &str) -> Result<Option<String>> {
    connection
        .query_row("SELECT status FROM jobs WHERE id = ?1", [job_id], |row| {
            row.get(0)
        })
        .optional()
        .map_err(Into::into)
}

fn next_claimable_task_key(connection: &Connection, job_id: &str) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT task_key FROM job_tasks
             WHERE job_id = ?1 AND status = 'ready' AND lease_owner_id IS NULL
             ORDER BY created_at, task_key
             LIMIT 1",
            [job_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

fn task_attempt_count(connection: &Connection, job_id: &str, task_key: &str) -> Result<i64> {
    Ok(connection.query_row(
        "SELECT attempt_count FROM job_tasks WHERE job_id = ?1 AND task_key = ?2",
        params![job_id, task_key],
        |row| row.get(0),
    )?)
}

fn task_required(connection: &Connection, job_id: &str, task_key: &str) -> Result<bool> {
    let required: i64 = connection.query_row(
        "SELECT required FROM job_tasks WHERE job_id = ?1 AND task_key = ?2",
        params![job_id, task_key],
        |row| row.get(0),
    )?;
    Ok(required != 0)
}

fn ensure_task_lease_owner(
    connection: &Connection,
    job_id: &str,
    task_key: &str,
    worker_id: &str,
) -> Result<()> {
    let owner: Option<String> = connection
        .query_row(
            "SELECT lease_owner_id FROM job_tasks WHERE job_id = ?1 AND task_key = ?2 AND status = 'running'",
            params![job_id, task_key],
            |row| row.get(0),
        )
        .optional()?;
    if owner.as_deref() != Some(worker_id) {
        bail!("task is not leased by worker");
    }
    Ok(())
}

fn fail_task_attempt_tx(
    transaction: &Transaction,
    job_id: &str,
    task_key: &str,
    error_message: &str,
    now: DateTime<Utc>,
    event_type: &str,
) -> Result<TaskRetryOutcome> {
    let attempt_count = task_attempt_count(transaction, job_id, task_key)?;
    let max_attempts = task_max_attempts(transaction, job_id, task_key)?;
    let now = now.to_rfc3339();
    let retrying = attempt_count < max_attempts;
    let next_status = if retrying { "ready" } else { "failed" };

    transaction.execute(
        "UPDATE job_tasks
         SET status = ?1,
             updated_at = ?2,
             lease_owner_id = NULL,
             lease_expires_at = NULL,
             error_message = ?3
         WHERE job_id = ?4 AND task_key = ?5",
        params![next_status, now, error_message, job_id, task_key],
    )?;

    append_job_event_tx(
        transaction,
        job_id,
        Some(task_key),
        event_type,
        json!({
            "taskKey": task_key,
            "attemptCount": attempt_count,
            "maxAttempts": max_attempts,
            "retrying": retrying,
            "reason": error_message,
        }),
    )?;

    if retrying {
        append_job_event_tx(
            transaction,
            job_id,
            Some(task_key),
            "task.ready",
            json!({ "taskKey": task_key, "reason": "retry" }),
        )?;
        Ok(TaskRetryOutcome::Retrying)
    } else {
        if task_required(transaction, job_id, task_key)? {
            transaction.execute(
                "UPDATE jobs
                 SET status = 'failed',
                     failure_message = ?1,
                     completed_at = COALESCE(completed_at, ?2),
                     current_task_key = NULL,
                     updated_at = ?2
                 WHERE id = ?3",
                params![error_message, now, job_id],
            )?;
            append_job_event_tx(
                transaction,
                job_id,
                None,
                "job.failed",
                json!({ "taskKey": task_key, "reason": error_message }),
            )?;
        }
        Ok(TaskRetryOutcome::Exhausted)
    }
}

fn task_max_attempts(connection: &Connection, job_id: &str, task_key: &str) -> Result<i64> {
    let retry_policy_json: String = connection.query_row(
        "SELECT retry_policy_json FROM job_tasks WHERE job_id = ?1 AND task_key = ?2",
        params![job_id, task_key],
        |row| row.get(0),
    )?;
    let retry_policy: Value =
        serde_json::from_str(&retry_policy_json).unwrap_or_else(|_| json!({}));
    Ok(retry_policy
        .get("maxAttempts")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .max(1))
}

fn mark_newly_ready_tasks(transaction: &Transaction, job_id: &str, now: &str) -> Result<()> {
    let tasks = load_task_states(transaction, job_id)?;
    for task_key in ready_task_keys(&tasks) {
        let updated = transaction.execute(
            "UPDATE job_tasks
             SET status = 'ready', updated_at = ?1
             WHERE job_id = ?2 AND task_key = ?3 AND status = 'pending'",
            params![now, job_id, task_key],
        )?;
        if updated == 1 {
            append_job_event_tx(
                transaction,
                job_id,
                Some(&task_key),
                "task.ready",
                json!({ "taskKey": task_key }),
            )?;
        }
    }
    Ok(())
}

fn complete_job_if_done(transaction: &Transaction, job_id: &str, now: &str) -> Result<()> {
    let incomplete_required: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM job_tasks
         WHERE job_id = ?1 AND required = 1 AND status NOT IN ('succeeded', 'skipped')",
        [job_id],
        |row| row.get(0),
    )?;
    if incomplete_required == 0 {
        transaction.execute(
            "UPDATE jobs
             SET status = 'succeeded',
                 completed_at = COALESCE(completed_at, ?1),
                 current_task_key = NULL,
                 updated_at = ?1
             WHERE id = ?2",
            params![now, job_id],
        )?;
        append_job_event_tx(
            transaction,
            job_id,
            None,
            "job.succeeded",
            json!({ "jobId": job_id }),
        )?;
    }
    Ok(())
}

fn load_task_states(connection: &Connection, job_id: &str) -> Result<Vec<TaskState>> {
    let mut statement = connection.prepare(
        "SELECT task_key, required, status FROM job_tasks WHERE job_id = ?1 ORDER BY created_at, task_key",
    )?;
    let mut tasks = statement
        .query_map([job_id], |row| {
            Ok(TaskState {
                key: row.get(0)?,
                required: row.get::<_, i64>(1)? != 0,
                status: row.get(2)?,
                depends_on: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    for task in &mut tasks {
        let mut dependency_statement = connection.prepare(
            "SELECT depends_on_task_key FROM job_task_dependencies
             WHERE job_id = ?1 AND task_key = ?2
             ORDER BY depends_on_task_key",
        )?;
        task.depends_on = dependency_statement
            .query_map(params![job_id, task.key], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
    }
    Ok(tasks)
}

fn append_job_event_tx(
    transaction: &Transaction,
    job_id: &str,
    task_key: Option<&str>,
    event_type: &str,
    payload: Value,
) -> Result<i64> {
    let sequence = next_event_sequence(transaction, job_id)?;
    let event = job_event(event_type, job_id, task_key, sequence, payload.clone());
    transaction.execute(
        "INSERT INTO job_events (id, job_id, task_key, sequence, event_type, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            format!("event_{}", Uuid::new_v4()),
            job_id,
            task_key,
            sequence,
            event_type,
            payload.to_string(),
            event.occurred_at,
        ],
    )?;
    append_realtime_event_tx(transaction, &event)?;
    insert_diagnostic_log_connection(
        transaction,
        job_event_log_entry(job_id, task_key, event_type, payload),
    )?;
    Ok(sequence)
}

fn job_event_log_entry(
    job_id: &str,
    task_key: Option<&str>,
    event_type: &str,
    payload: Value,
) -> NewDiagnosticLogEntry {
    let level = if event_type.contains("failed") || event_type.contains("blocked") {
        "error"
    } else if event_type.contains("waiting") {
        "warn"
    } else {
        "info"
    };
    NewDiagnosticLogEntry {
        job_id: Some(job_id.to_string()),
        task_key: task_key.map(ToString::to_string),
        event_type: Some(event_type.to_string()),
        ..diagnostic_log(level, "job", format!("Job event {event_type}"), payload)
    }
}

fn next_event_sequence(connection: &Connection, job_id: &str) -> Result<i64> {
    let next_sequence = connection.query_row(
        "SELECT COALESCE(MAX(sequence), 0) + 1 FROM job_events WHERE job_id = ?1",
        [job_id],
        |row| row.get(0),
    )?;
    Ok(next_sequence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::schema::init_schema;
    use crate::templates::{seed_builtin_templates, TaskDefinition};
    use rusqlite::Connection;

    fn test_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        seed_builtin_templates(&connection).unwrap();
        connection
    }

    fn test_job(connection: &mut Connection) -> String {
        let template = crate::templates::require_builtin_template("system.health.check").unwrap();
        create_job_from_template(connection, &template, "test", None, json!({})).unwrap()
    }

    fn utc(timestamp: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(timestamp)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn test_task(key: &str, depends_on: &[&str]) -> TaskDefinition {
        TaskDefinition {
            key: key.to_string(),
            capability_id: format!("test.{key}"),
            kind: format!("test.{key}"),
            label: key.to_string(),
            required: true,
            depends_on: depends_on
                .iter()
                .map(|dependency| dependency.to_string())
                .collect(),
            input: json!({}),
            retry_policy: json!({}),
        }
    }

    fn variable_template() -> ProcessTemplate {
        ProcessTemplate {
            id: "test.variable.plan".to_string(),
            capability_id: "system.health.check".to_string(),
            kind: "system.health.check".to_string(),
            name: "Variable Plan".to_string(),
            version: 7,
            description: "exercise compiled plan snapshots".to_string(),
            variable_schema: json!({
                "type": "object",
                "required": ["topic"],
                "properties": {
                    "topic": { "type": "string", "minLength": 1 },
                    "priority": { "enum": ["normal", "urgent"] }
                },
                "additionalProperties": false
            }),
            tasks: vec![
                TaskDefinition {
                    key: "probe".to_string(),
                    capability_id: "system.health.probe".to_string(),
                    kind: "system.health.probe".to_string(),
                    label: "Probe".to_string(),
                    required: true,
                    depends_on: vec![],
                    input: json!({ "from": "template" }),
                    retry_policy: json!({ "maxAttempts": 3 }),
                },
                TaskDefinition {
                    key: "record".to_string(),
                    capability_id: "system.health.record".to_string(),
                    kind: "system.health.record".to_string(),
                    label: "Record".to_string(),
                    required: true,
                    depends_on: vec!["probe".to_string()],
                    input: json!({}),
                    retry_policy: json!({ "maxAttempts": 2 }),
                },
            ],
        }
    }

    fn register_template(connection: &Connection, template: &ProcessTemplate) {
        connection
            .execute(
                "INSERT INTO process_templates (
                    id, capability_id, kind, name, version, description, variable_schema_json,
                    tasks_json, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'now', 'now')",
                params![
                    template.id,
                    template.effective_capability_id(),
                    template.kind,
                    template.name,
                    template.version,
                    template.description,
                    template.variable_schema.to_string(),
                    serde_json::to_string(&template.tasks).unwrap(),
                ],
            )
            .unwrap();
    }

    fn insert_test_artifact(connection: &Connection, artifact_id: &str) {
        connection
            .execute(
                "INSERT INTO artifacts (
                    id, artifact_kind, title, status, visibility_ceiling, summary,
                    evidence_refs_json, provenance_json, content_hash, created_at, updated_at
                 ) VALUES (
                    ?1, 'studio.storyboard', 'Storyboard', 'draft', 'staff',
                    'Safe storyboard summary', '[\"job:test\"]',
                    '{\"generatedBy\":\"kernel.test\"}', 'sha256:test-artifact', 'now', 'now'
                 )",
                [artifact_id],
            )
            .unwrap();
    }

    #[test]
    fn dag_validation_rejects_cycles() {
        let tasks = vec![test_task("a", &["b"]), test_task("b", &["a"])];
        assert!(validate_task_dag(&tasks).is_err());
    }

    #[test]
    fn ready_tasks_respect_dependencies() {
        let tasks = vec![
            TaskState {
                key: "a".to_string(),
                required: true,
                status: "succeeded".to_string(),
                depends_on: vec![],
            },
            TaskState {
                key: "b".to_string(),
                required: true,
                status: "pending".to_string(),
                depends_on: vec!["a".to_string()],
            },
            TaskState {
                key: "c".to_string(),
                required: true,
                status: "pending".to_string(),
                depends_on: vec!["b".to_string()],
            },
        ];

        assert_eq!(ready_task_keys(&tasks), vec!["b".to_string()]);
    }

    #[test]
    fn progress_is_task_count_based_and_reports_elapsed_time() {
        let started_at = DateTime::parse_from_rfc3339("2026-05-07T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let now = DateTime::parse_from_rfc3339("2026-05-07T10:01:30Z")
            .unwrap()
            .with_timezone(&Utc);
        let tasks = vec![
            TaskState {
                key: "a".to_string(),
                required: true,
                status: "succeeded".to_string(),
                depends_on: vec![],
            },
            TaskState {
                key: "b".to_string(),
                required: true,
                status: "running".to_string(),
                depends_on: vec!["a".to_string()],
            },
            TaskState {
                key: "c".to_string(),
                required: true,
                status: "pending".to_string(),
                depends_on: vec!["b".to_string()],
            },
            TaskState {
                key: "d".to_string(),
                required: false,
                status: "succeeded".to_string(),
                depends_on: vec![],
            },
        ];

        let progress = calculate_progress(&tasks, Some(started_at), now);

        assert_eq!(progress.completed_required_tasks, 1);
        assert_eq!(progress.total_required_tasks, 3);
        assert_eq!(progress.percent, 33);
        assert_eq!(progress.current_task_key, Some("b".to_string()));
        assert_eq!(progress.elapsed_seconds, Some(90));
    }

    #[test]
    fn creates_job_from_template_and_records_events() {
        let mut connection = test_connection();
        let job_id = test_job(&mut connection);

        let task_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM job_tasks WHERE job_id = ?1",
                [&job_id],
                |row| row.get(0),
            )
            .unwrap();
        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM job_events WHERE job_id = ?1",
                [&job_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(task_count, 2);
        assert_eq!(event_count, 2);

        let replay_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM realtime_events WHERE job_id = ?1",
                [&job_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(replay_count, 2);
    }

    #[test]
    fn creates_job_with_compiled_plan_snapshot_and_validated_variables() {
        let mut connection = test_connection();
        let template = variable_template();
        register_template(&connection, &template);

        let job_id = create_job_from_template(
            &mut connection,
            &template,
            "test",
            Some("actor_owner"),
            json!({ "topic": "NYC pilot", "priority": "urgent" }),
        )
        .unwrap();

        let compiled_plan_json: String = connection
            .query_row(
                "SELECT compiled_plan_json FROM jobs WHERE id = ?1",
                [&job_id],
                |row| row.get(0),
            )
            .unwrap();
        let compiled_plan: Value = serde_json::from_str(&compiled_plan_json).unwrap();

        assert_eq!(compiled_plan["template"]["id"], "test.variable.plan");
        assert_eq!(compiled_plan["template"]["version"], 7);
        assert_eq!(compiled_plan["input"]["topic"], "NYC pilot");
        assert_eq!(compiled_plan["variableSchema"]["required"][0], "topic");
        assert_eq!(compiled_plan["tasks"][0]["key"], "probe");
        assert_eq!(
            compiled_plan["tasks"][0]["capabilityId"],
            "system.health.probe"
        );
        assert_eq!(compiled_plan["tasks"][0]["required"], true);
        assert_eq!(compiled_plan["tasks"][0]["retryPolicy"]["maxAttempts"], 3);
        assert_eq!(compiled_plan["tasks"][1]["dependsOn"][0], "probe");

        let job_created_payload_json: String = connection
            .query_row(
                "SELECT payload_json FROM job_events
                 WHERE job_id = ?1 AND event_type = 'job.created'",
                [&job_id],
                |row| row.get(0),
            )
            .unwrap();
        let job_created_payload: Value = serde_json::from_str(&job_created_payload_json).unwrap();
        assert_eq!(job_created_payload["compiledPlan"]["taskCount"], 2);
        assert!(job_created_payload.get("input").is_none());
    }

    #[test]
    fn invalid_template_variables_reject_without_partial_job_rows() {
        let mut connection = test_connection();
        let template = variable_template();
        register_template(&connection, &template);

        let before_jobs: i64 = connection
            .query_row("SELECT COUNT(*) FROM jobs", [], |row| row.get(0))
            .unwrap();
        let before_tasks: i64 = connection
            .query_row("SELECT COUNT(*) FROM job_tasks", [], |row| row.get(0))
            .unwrap();

        let error = create_job_from_template(
            &mut connection,
            &template,
            "test",
            None,
            json!({ "topic": "", "extra": "not allowed" }),
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("template variables failed JSON Schema validation"));
        let after_jobs: i64 = connection
            .query_row("SELECT COUNT(*) FROM jobs", [], |row| row.get(0))
            .unwrap();
        let after_tasks: i64 = connection
            .query_row("SELECT COUNT(*) FROM job_tasks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(after_jobs, before_jobs);
        assert_eq!(after_tasks, before_tasks);
    }

    #[test]
    fn legacy_template_without_variable_schema_gets_empty_schema_snapshot() {
        let mut connection = test_connection();
        let mut template =
            crate::templates::require_builtin_template("system.health.check").unwrap();
        template.variable_schema = json!({});

        let job_id = create_job_from_template(
            &mut connection,
            &template,
            "legacy",
            None,
            json!({ "freeform": true }),
        )
        .unwrap();

        let compiled_plan_json: String = connection
            .query_row(
                "SELECT compiled_plan_json FROM jobs WHERE id = ?1",
                [&job_id],
                |row| row.get(0),
            )
            .unwrap();
        let compiled_plan: Value = serde_json::from_str(&compiled_plan_json).unwrap();
        assert_eq!(compiled_plan["variableSchema"], json!({}));
        assert_eq!(compiled_plan["input"]["freeform"], true);
    }

    #[test]
    fn repeated_job_start_with_same_idempotency_key_returns_original_job() {
        let mut connection = test_connection();
        let template = variable_template();
        register_template(&connection, &template);

        let first_job_id = create_job_from_template_with_idempotency(
            &mut connection,
            &template,
            "studio.story",
            Some("actor_owner"),
            json!({ "priority": "urgent", "topic": "NYC pilot" }),
            Some("job-start-1"),
        )
        .unwrap();
        let repeated_job_id = create_job_from_template_with_idempotency(
            &mut connection,
            &template,
            "studio.story",
            Some("actor_owner"),
            json!({ "topic": "NYC pilot", "priority": "urgent" }),
            Some("job-start-1"),
        )
        .unwrap();

        assert_eq!(repeated_job_id, first_job_id);

        let job_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM jobs WHERE idempotency_key = 'job-start-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let task_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM job_tasks WHERE job_id = ?1",
                [&first_job_id],
                |row| row.get(0),
            )
            .unwrap();
        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM job_events WHERE job_id = ?1",
                [&first_job_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(job_count, 1);
        assert_eq!(task_count, 2);
        assert_eq!(event_count, 2);

        let duplicate_index_error = connection
            .execute(
                "INSERT INTO jobs (
                    id, template_id, template_version, capability_id, kind, status, origin,
                    actor_id, input_json, compiled_plan_json, idempotency_key,
                    idempotency_request_json, required_task_count, completed_required_task_count,
                    created_at, updated_at
                 ) VALUES (
                    'job_duplicate_key', ?1, ?2, ?3, ?4, 'queued', 'studio.story',
                    'actor_owner', '{}', '{}', 'job-start-1', '{}', 0, 0, 'now', 'now'
                 )",
                params![
                    template.id,
                    template.version,
                    template.effective_capability_id(),
                    template.kind,
                ],
            )
            .unwrap_err()
            .to_string();
        assert!(duplicate_index_error.contains("UNIQUE constraint failed"));
    }

    #[test]
    fn conflicting_job_start_idempotency_key_rejects_without_mutation() {
        let mut connection = test_connection();
        let template = variable_template();
        register_template(&connection, &template);

        create_job_from_template_with_idempotency(
            &mut connection,
            &template,
            "studio.story",
            Some("actor_owner"),
            json!({ "topic": "NYC pilot", "priority": "normal" }),
            Some("job-start-conflict"),
        )
        .unwrap();

        let before_jobs: i64 = connection
            .query_row("SELECT COUNT(*) FROM jobs", [], |row| row.get(0))
            .unwrap();
        let before_tasks: i64 = connection
            .query_row("SELECT COUNT(*) FROM job_tasks", [], |row| row.get(0))
            .unwrap();
        let before_events: i64 = connection
            .query_row("SELECT COUNT(*) FROM job_events", [], |row| row.get(0))
            .unwrap();

        let error = create_job_from_template_with_idempotency(
            &mut connection,
            &template,
            "studio.story",
            Some("actor_owner"),
            json!({ "topic": "Different private brief", "priority": "normal" }),
            Some("job-start-conflict"),
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("Job idempotency key conflicts with a different request"));
        assert!(!error.contains("Different private brief"));

        let after_jobs: i64 = connection
            .query_row("SELECT COUNT(*) FROM jobs", [], |row| row.get(0))
            .unwrap();
        let after_tasks: i64 = connection
            .query_row("SELECT COUNT(*) FROM job_tasks", [], |row| row.get(0))
            .unwrap();
        let after_events: i64 = connection
            .query_row("SELECT COUNT(*) FROM job_events", [], |row| row.get(0))
            .unwrap();

        assert_eq!(after_jobs, before_jobs);
        assert_eq!(after_tasks, before_tasks);
        assert_eq!(after_events, before_events);
    }

    #[test]
    fn job_starts_without_idempotency_key_preserve_existing_create_behavior() {
        let mut connection = test_connection();
        let template = variable_template();
        register_template(&connection, &template);

        let first_job_id = create_job_from_template(
            &mut connection,
            &template,
            "studio.story",
            Some("actor_owner"),
            json!({ "topic": "NYC pilot", "priority": "normal" }),
        )
        .unwrap();
        let second_job_id = create_job_from_template(
            &mut connection,
            &template,
            "studio.story",
            Some("actor_owner"),
            json!({ "topic": "NYC pilot", "priority": "normal" }),
        )
        .unwrap();

        assert_ne!(first_job_id, second_job_id);
    }

    #[test]
    fn claims_one_ready_task_with_durable_lease_and_rejects_double_claim() {
        let mut connection = test_connection();
        let job_id = test_job(&mut connection);
        let now = utc("2026-05-14T10:00:00Z");

        let claimed = claim_ready_task(&mut connection, &job_id, "worker_a", 60, now)
            .unwrap()
            .expect("ready task should be claimed");

        assert_eq!(claimed.worker_id, "worker_a");
        assert_eq!(claimed.attempt_count, 1);
        assert_eq!(claimed.lease_expires_at, utc("2026-05-14T10:01:00Z"));

        let double_claim = claim_ready_task(
            &mut connection,
            &job_id,
            "worker_b",
            60,
            utc("2026-05-14T10:00:05Z"),
        )
        .unwrap();
        assert!(double_claim.is_none());

        let persisted: (String, String, String, i64) = connection
            .query_row(
                "SELECT status, lease_owner_id, lease_expires_at, attempt_count
                 FROM job_tasks WHERE job_id = ?1 AND task_key = ?2",
                params![job_id, claimed.task_key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(persisted.0, "running");
        assert_eq!(persisted.1, "worker_a");
        assert_eq!(persisted.2, "2026-05-14T10:01:00+00:00");
        assert_eq!(persisted.3, 1);

        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM job_events WHERE job_id = ?1 AND event_type = 'task.claimed'",
                [job_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 1);
    }

    #[test]
    fn completing_a_leased_task_releases_lease_and_marks_dependents_ready() {
        let mut connection = test_connection();
        let job_id = test_job(&mut connection);
        let claimed = claim_ready_task(
            &mut connection,
            &job_id,
            "worker_a",
            60,
            utc("2026-05-14T10:00:00Z"),
        )
        .unwrap()
        .unwrap();

        complete_leased_task(
            &mut connection,
            &job_id,
            &claimed.task_key,
            "worker_a",
            json!({ "ok": true }),
            utc("2026-05-14T10:00:30Z"),
        )
        .unwrap();

        let completed: (String, Option<String>, Option<String>) = connection
            .query_row(
                "SELECT status, lease_owner_id, lease_expires_at
                 FROM job_tasks WHERE job_id = ?1 AND task_key = ?2",
                params![job_id, claimed.task_key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(completed, ("succeeded".to_string(), None, None));

        let ready_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM job_tasks WHERE job_id = ?1 AND status = 'ready'",
                [&job_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(ready_count, 1);
    }

    #[test]
    fn leased_task_completion_persists_result_envelope_and_safe_event() {
        let mut connection = test_connection();
        let job_id = test_job(&mut connection);
        let artifact_id = "artifact_storyboard_1";
        insert_test_artifact(&connection, artifact_id);
        let claimed = claim_ready_task(
            &mut connection,
            &job_id,
            "worker_a",
            60,
            utc("2026-05-14T10:00:00Z"),
        )
        .unwrap()
        .unwrap();

        complete_leased_task_with_result_envelope(
            &mut connection,
            &job_id,
            &claimed.task_key,
            "worker_a",
            TaskResultEnvelope {
                status: "succeeded".to_string(),
                summary: "Storyboard draft created from approved prompt slots.".to_string(),
                safe_output: json!({ "sectionCount": 5, "reviewState": "draft" }),
                artifact_refs: vec![artifact_id.to_string()],
                evidence_refs: vec![
                    format!("job:{job_id}"),
                    format!("job_task:{}:{}", job_id, claimed.task_key),
                    format!("artifact:{artifact_id}"),
                ],
                limitations: vec!["Draft artifact only; no publishing occurred.".to_string()],
            },
            utc("2026-05-14T10:00:30Z"),
        )
        .unwrap();

        let output_json: String = connection
            .query_row(
                "SELECT output_json FROM job_tasks WHERE job_id = ?1 AND task_key = ?2",
                params![job_id, claimed.task_key],
                |row| row.get(0),
            )
            .unwrap();
        let output: Value = serde_json::from_str(&output_json).unwrap();
        assert_eq!(output["schemaVersion"], 1);
        assert_eq!(output["status"], "succeeded");
        assert_eq!(
            output["summary"],
            "Storyboard draft created from approved prompt slots."
        );
        assert_eq!(output["safeOutput"]["sectionCount"], 5);
        assert_eq!(output["artifactRefs"][0], artifact_id);
        assert_eq!(output["evidenceRefs"][2], format!("artifact:{artifact_id}"));
        assert_eq!(
            output["limitations"][0],
            "Draft artifact only; no publishing occurred."
        );

        let event_payload_json: String = connection
            .query_row(
                "SELECT payload_json FROM job_events
                 WHERE job_id = ?1 AND task_key = ?2 AND event_type = 'task.succeeded'",
                params![job_id, claimed.task_key],
                |row| row.get(0),
            )
            .unwrap();
        let event_payload: Value = serde_json::from_str(&event_payload_json).unwrap();
        assert_eq!(event_payload["result"]["summary"], output["summary"]);
        assert_eq!(event_payload["result"]["artifactRefs"][0], artifact_id);
        assert!(event_payload["result"].get("safeOutput").is_none());
    }

    #[test]
    fn malformed_result_envelope_rejects_without_completing_task() {
        let mut connection = test_connection();
        let job_id = test_job(&mut connection);
        let claimed = claim_ready_task(
            &mut connection,
            &job_id,
            "worker_a",
            60,
            utc("2026-05-14T10:00:00Z"),
        )
        .unwrap()
        .unwrap();

        let error = complete_leased_task_with_result_envelope(
            &mut connection,
            &job_id,
            &claimed.task_key,
            "worker_a",
            TaskResultEnvelope {
                status: "succeeded".to_string(),
                summary: "Unsafe result".to_string(),
                safe_output: json!({
                    "providerInternals": "sk-live-secret",
                    "unsupportedClaims": ["published to TikTok"]
                }),
                artifact_refs: vec!["artifact_missing".to_string()],
                evidence_refs: vec!["publishing:fake_metric".to_string()],
                limitations: vec![],
            },
            utc("2026-05-14T10:00:30Z"),
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("task result envelope"));
        assert!(!error.contains("sk-live-secret"));
        assert!(!error.contains("published to TikTok"));

        let state: (String, Option<String>, i64) = connection
            .query_row(
                "SELECT status, output_json, COUNT(*) OVER ()
                 FROM job_tasks WHERE job_id = ?1 AND task_key = ?2",
                params![job_id, claimed.task_key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(state.0, "running");
        assert!(state.1.is_none());

        let event_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM job_events
                 WHERE job_id = ?1 AND task_key = ?2 AND event_type = 'task.succeeded'",
                params![job_id, claimed.task_key],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 0);
    }

    #[test]
    fn legacy_task_output_is_wrapped_in_result_envelope() {
        let mut connection = test_connection();
        let job_id = test_job(&mut connection);
        let claimed = claim_ready_task(
            &mut connection,
            &job_id,
            "worker_a",
            60,
            utc("2026-05-14T10:00:00Z"),
        )
        .unwrap()
        .unwrap();

        complete_leased_task(
            &mut connection,
            &job_id,
            &claimed.task_key,
            "worker_a",
            json!({ "ok": true }),
            utc("2026-05-14T10:00:30Z"),
        )
        .unwrap();

        let output_json: String = connection
            .query_row(
                "SELECT output_json FROM job_tasks WHERE job_id = ?1 AND task_key = ?2",
                params![job_id, claimed.task_key],
                |row| row.get(0),
            )
            .unwrap();
        let output: Value = serde_json::from_str(&output_json).unwrap();
        assert_eq!(output["schemaVersion"], 1);
        assert_eq!(output["status"], "succeeded");
        assert_eq!(output["summary"], "Task completed with legacy output.");
        assert_eq!(output["safeOutput"]["ok"], true);
        assert_eq!(output["artifactRefs"], json!([]));
        assert_eq!(output["evidenceRefs"][0], format!("job:{job_id}"));
    }

    #[test]
    fn failed_and_expired_attempts_retry_until_bounded_attempts_are_exhausted() {
        let mut connection = test_connection();
        let job_id = test_job(&mut connection);
        connection
            .execute(
                "UPDATE job_tasks SET retry_policy_json = '{\"maxAttempts\":2}' WHERE job_id = ?1",
                [&job_id],
            )
            .unwrap();

        let first = claim_ready_task(
            &mut connection,
            &job_id,
            "worker_a",
            30,
            utc("2026-05-14T10:00:00Z"),
        )
        .unwrap()
        .unwrap();
        let outcome = fail_leased_task_attempt(
            &mut connection,
            &job_id,
            &first.task_key,
            "worker_a",
            "deterministic failure",
            utc("2026-05-14T10:00:10Z"),
        )
        .unwrap();
        assert_eq!(outcome, TaskRetryOutcome::Retrying);

        let second = claim_ready_task(
            &mut connection,
            &job_id,
            "worker_b",
            30,
            utc("2026-05-14T10:00:15Z"),
        )
        .unwrap()
        .unwrap();
        assert_eq!(second.task_key, first.task_key);
        assert_eq!(second.attempt_count, 2);

        let outcome = expire_task_lease_for_retry(
            &mut connection,
            &job_id,
            &second.task_key,
            utc("2026-05-14T10:01:00Z"),
            "lease expired",
        )
        .unwrap();
        assert_eq!(outcome, TaskRetryOutcome::Exhausted);

        let final_state: (String, i64) = connection
            .query_row(
                "SELECT status, attempt_count FROM job_tasks WHERE job_id = ?1 AND task_key = ?2",
                params![job_id, first.task_key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(final_state, ("failed".to_string(), 2));

        let job_status: String = connection
            .query_row("SELECT status FROM jobs WHERE id = ?1", [job_id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(job_status, "failed");
    }

    #[test]
    fn canceling_job_clears_leases_records_events_and_prevents_future_claims() {
        let mut connection = test_connection();
        let job_id = test_job(&mut connection);
        let claimed = claim_ready_task(
            &mut connection,
            &job_id,
            "worker_a",
            60,
            utc("2026-05-14T10:00:00Z"),
        )
        .unwrap()
        .unwrap();

        cancel_job(
            &mut connection,
            &job_id,
            Some("actor_local_owner"),
            "operator canceled",
            utc("2026-05-14T10:00:20Z"),
        )
        .unwrap();

        let next_claim = claim_ready_task(
            &mut connection,
            &job_id,
            "worker_b",
            60,
            utc("2026-05-14T10:00:25Z"),
        )
        .unwrap();
        assert!(next_claim.is_none());

        let task_state: (String, Option<String>, Option<String>) = connection
            .query_row(
                "SELECT status, lease_owner_id, lease_expires_at
                 FROM job_tasks WHERE job_id = ?1 AND task_key = ?2",
                params![job_id, claimed.task_key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(task_state, ("canceled".to_string(), None, None));

        let replay_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM realtime_events WHERE job_id = ?1 AND event_type = 'job.canceled'",
                [job_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(replay_count, 1);
    }
}
