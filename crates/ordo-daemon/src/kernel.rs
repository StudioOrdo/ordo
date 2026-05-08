use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Transaction};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

use crate::capabilities::assert_capability_ids_registered;
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
    validate_task_dag(&template.tasks)?;
    validate_job_capabilities(connection, template)?;

    let transaction = connection.transaction()?;
    let now = Utc::now().to_rfc3339();
    let job_id = format!("job_{}", Uuid::new_v4());
    let required_task_count = template
        .tasks
        .iter()
        .filter(|task_definition| task_definition.required)
        .count();

    transaction.execute(
        "INSERT INTO jobs (
            id, template_id, template_version, capability_id, kind, status, origin, actor_id, input_json,
            current_task_key, required_task_count, completed_required_task_count,
            started_at, completed_at, created_at, updated_at, failure_message
         ) VALUES (?1, ?2, ?3, ?4, ?5, 'queued', ?6, ?7, ?8, NULL, ?9, 0, NULL, NULL, ?10, ?10, NULL)",
        params![
            job_id,
            template.id,
            template.version,
            template.effective_capability_id(),
            template.kind,
            origin,
            actor_id,
            input.to_string(),
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
            output_json, attempt_count, started_at, completed_at, created_at, updated_at, error_message
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, NULL, 0, NULL, NULL, ?9, ?9, NULL)",
        params![
            format!("task_{}", Uuid::new_v4()),
            job_id,
            task_definition.key,
            task_definition.effective_capability_id(),
            task_definition.kind,
            task_definition.label,
            if task_definition.required { 1 } else { 0 },
            task_definition.input.to_string(),
            now,
        ],
    )?;
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
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(sequence)
}

fn append_job_event_tx(
    transaction: &Transaction,
    job_id: &str,
    task_key: Option<&str>,
    event_type: &str,
    payload: Value,
) -> Result<i64> {
    let sequence = next_event_sequence(transaction, job_id)?;
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
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(sequence)
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
        }
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
        let mut connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        seed_builtin_templates(&connection).unwrap();
        let template = crate::templates::require_builtin_template("system.health.check").unwrap();

        let job_id =
            create_job_from_template(&mut connection, &template, "test", None, json!({})).unwrap();

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
    }
}
