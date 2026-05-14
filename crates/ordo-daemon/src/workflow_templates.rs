use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

use crate::json_contracts::validate_json_value;

const MAX_FANOUT_ITEMS: i64 = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowTemplateDefinition {
    pub template_id: String,
    pub version: i64,
    pub name: String,
    pub pack_id: String,
    pub visibility_ceiling: String,
    pub idempotency_strategy: String,
    pub input_schema: Value,
    pub variables: Vec<WorkflowVariable>,
    pub tasks: Vec<WorkflowTaskBinding>,
    #[serde(default)]
    pub fanout_groups: Vec<WorkflowFanoutGroup>,
    #[serde(default)]
    pub approval_gates: Vec<WorkflowApprovalGate>,
    #[serde(default)]
    pub provider_requirements: Vec<WorkflowProviderRequirement>,
    #[serde(default)]
    pub deterministic_mocks: Vec<WorkflowDeterministicMock>,
    #[serde(default)]
    pub audit_events: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowVariable {
    pub key: String,
    pub value_type: String,
    pub source_kind: String,
    #[serde(default)]
    pub source_ref: Option<String>,
    pub visibility: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowTaskBinding {
    pub key: String,
    pub method: String,
    #[serde(default)]
    pub input: Value,
    #[serde(default)]
    pub retry_policy: Value,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub visibility: String,
    #[serde(default)]
    pub fanout: Option<String>,
    #[serde(default)]
    pub provider_requirement: Option<String>,
    #[serde(default)]
    pub output_artifact_kind: Option<String>,
    #[serde(default)]
    pub sensitive_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowFanoutGroup {
    pub key: String,
    pub collection_variable: String,
    pub item_variable: String,
    pub max_items: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowApprovalGate {
    pub key: String,
    pub action: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowProviderRequirement {
    pub key: String,
    pub capability: String,
    pub mode: String,
    pub egress: String,
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDeterministicMock {
    pub key: String,
    pub capability: String,
    pub fixture_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowCompilation {
    pub id: String,
    pub template_id: String,
    pub template_version: i64,
    pub idempotency_key: String,
    pub input_hash: String,
    pub safe_compiled_plan: Value,
}

pub fn built_in_workflow_templates() -> Vec<WorkflowTemplateDefinition> {
    vec![
        zodiac_image_set_template(),
        article_with_image_template(),
        story_scrollytelling_homepage_template(),
    ]
}

pub fn seed_builtin_workflow_templates(connection: &Connection) -> Result<()> {
    for template in built_in_workflow_templates() {
        upsert_workflow_template(connection, &template)?;
    }
    Ok(())
}

pub fn upsert_workflow_template(
    connection: &Connection,
    template: &WorkflowTemplateDefinition,
) -> Result<()> {
    validate_workflow_template(template)?;
    let now = Utc::now().to_rfc3339();
    let row_id = workflow_template_row_id(&template.template_id, template.version);
    connection.execute(
        "INSERT INTO workflow_templates (
            id, template_id, version, name, pack_id, status, visibility_ceiling,
            idempotency_strategy, definition_json, input_schema_json, variable_schema_json, task_bindings_json,
            fanout_groups_json, approval_gates_json, provider_requirements_json,
            deterministic_mocks_json, audit_events_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?17)
         ON CONFLICT(template_id, version) DO UPDATE SET
            name = excluded.name,
            pack_id = excluded.pack_id,
            status = excluded.status,
            visibility_ceiling = excluded.visibility_ceiling,
            idempotency_strategy = excluded.idempotency_strategy,
            definition_json = excluded.definition_json,
            input_schema_json = excluded.input_schema_json,
            variable_schema_json = excluded.variable_schema_json,
            task_bindings_json = excluded.task_bindings_json,
            fanout_groups_json = excluded.fanout_groups_json,
            approval_gates_json = excluded.approval_gates_json,
            provider_requirements_json = excluded.provider_requirements_json,
            deterministic_mocks_json = excluded.deterministic_mocks_json,
            audit_events_json = excluded.audit_events_json,
            updated_at = excluded.updated_at",
        params![
            row_id,
            template.template_id,
            template.version,
            template.name,
            template.pack_id,
            template.visibility_ceiling,
            template.idempotency_strategy,
            serde_json::to_string(template)?,
            template.input_schema.to_string(),
            serde_json::to_string(&template.variables)?,
            serde_json::to_string(&template.tasks)?,
            serde_json::to_string(&template.fanout_groups)?,
            serde_json::to_string(&template.approval_gates)?,
            serde_json::to_string(&template.provider_requirements)?,
            serde_json::to_string(&template.deterministic_mocks)?,
            serde_json::to_string(&template.audit_events)?,
            now,
        ],
    )?;
    Ok(())
}

pub fn compile_workflow_template(
    connection: &mut Connection,
    template_id: &str,
    template_version: i64,
    input: Value,
    idempotency_key: &str,
) -> Result<WorkflowCompilation> {
    let idempotency_key = normalize_idempotency_key(idempotency_key)?;
    let template = load_workflow_template(connection, template_id, template_version)?;
    validate_json_value(&template.input_schema, &input, "workflow template input")?;
    validate_workflow_template(&template)?;

    let input_hash = content_hash(&canonical_json_string(&input));
    let safe_compiled_plan = compile_safe_plan(&template, &input)?;
    let transaction = connection.transaction()?;
    if let Some(existing) = load_compilation_by_idempotency(
        &transaction,
        template_id,
        template_version,
        &idempotency_key,
    )? {
        if existing.input_hash == input_hash {
            transaction.commit()?;
            return Ok(existing);
        }
        bail!("Workflow template idempotency key conflicts with a different input");
    }

    let now = Utc::now().to_rfc3339();
    let compilation_id = format!("workflow_compilation_{}", Uuid::new_v4());
    transaction.execute(
        "INSERT INTO workflow_template_compilations (
            id, template_id, template_version, idempotency_key, input_hash,
            safe_compiled_plan_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            compilation_id,
            template_id,
            template_version,
            idempotency_key,
            input_hash,
            safe_compiled_plan.to_string(),
            now,
        ],
    )?;
    transaction.commit()?;

    Ok(WorkflowCompilation {
        id: compilation_id,
        template_id: template_id.to_string(),
        template_version,
        idempotency_key,
        input_hash,
        safe_compiled_plan,
    })
}

pub fn load_workflow_template(
    connection: &Connection,
    template_id: &str,
    template_version: i64,
) -> Result<WorkflowTemplateDefinition> {
    let definition_json: String = connection
        .query_row(
            "SELECT definition_json FROM workflow_templates
             WHERE template_id = ?1 AND version = ?2 AND status = 'active'",
            params![template_id, template_version],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| {
            anyhow::anyhow!("Unknown workflow template: {template_id}@{template_version}")
        })?;
    Ok(serde_json::from_str(&definition_json)?)
}

pub fn validate_workflow_template(template: &WorkflowTemplateDefinition) -> Result<()> {
    if template.template_id.trim().is_empty() {
        bail!("Workflow template id cannot be blank");
    }
    if template.version <= 0 {
        bail!("Workflow template version must be positive");
    }
    if template.idempotency_strategy != "required_idempotency_key" {
        bail!("Workflow template must require an explicit idempotency key");
    }
    validate_visibility(&template.visibility_ceiling, "template visibility")?;

    let mut variable_keys = BTreeSet::new();
    for variable in &template.variables {
        validate_variable(variable)?;
        if !variable_keys.insert(variable.key.clone()) {
            bail!("Duplicate workflow variable key: {}", variable.key);
        }
    }

    let mut fanout_keys = BTreeSet::new();
    for fanout in &template.fanout_groups {
        if !fanout_keys.insert(fanout.key.clone()) {
            bail!("Duplicate workflow fanout key: {}", fanout.key);
        }
        if !variable_keys.contains(&fanout.collection_variable) {
            bail!(
                "Workflow fanout {} references undeclared collection variable {}",
                fanout.key,
                fanout.collection_variable
            );
        }
        if fanout.max_items <= 0 || fanout.max_items > MAX_FANOUT_ITEMS {
            bail!(
                "Workflow fanout {} must be bounded between 1 and {MAX_FANOUT_ITEMS} items",
                fanout.key
            );
        }
    }

    let approval_actions: BTreeSet<String> = template
        .approval_gates
        .iter()
        .filter(|gate| gate.required)
        .map(|gate| gate.action.clone())
        .collect();
    let provider_keys: BTreeSet<String> = template
        .provider_requirements
        .iter()
        .map(|requirement| requirement.key.clone())
        .collect();
    let mock_keys: BTreeSet<String> = template
        .deterministic_mocks
        .iter()
        .map(|mock| mock.key.clone())
        .collect();

    for provider in &template.provider_requirements {
        validate_provider_requirement(provider)?;
        if !mock_keys.contains(&provider.key) {
            bail!(
                "Workflow provider requirement {} is missing deterministic mock fixture",
                provider.key
            );
        }
    }

    let mut task_keys = BTreeSet::new();
    for task in &template.tasks {
        validate_task_binding(task, &fanout_keys, &provider_keys, &approval_actions)?;
        if !task_keys.insert(task.key.clone()) {
            bail!("Duplicate workflow task key: {}", task.key);
        }
    }
    for task in &template.tasks {
        for dependency in &task.depends_on {
            if !task_keys.contains(dependency) {
                bail!(
                    "Workflow task {} depends on missing task {}",
                    task.key,
                    dependency
                );
            }
        }
    }

    Ok(())
}

fn validate_variable(variable: &WorkflowVariable) -> Result<()> {
    if variable.key.trim().is_empty() {
        bail!("Workflow variable key cannot be blank");
    }
    validate_visibility(&variable.visibility, "workflow variable visibility")?;
    match variable.source_kind.as_str() {
        "input" | "artifact" | "graph_method" | "prior_task_output" | "pack_config"
        | "canonical_method" => {}
        "sql" | "generic_context" | "prompt_only" => bail!(
            "Workflow variable {} uses unsafe source kind {}",
            variable.key,
            variable.source_kind
        ),
        other => bail!(
            "Workflow variable {} uses unsupported source kind {other}",
            variable.key
        ),
    }
    if let Some(source_ref) = variable.source_ref.as_deref() {
        let normalized = source_ref.to_ascii_lowercase();
        if normalized.contains("select ")
            || normalized.contains(" from ")
            || normalized == "get_context"
            || normalized == "query_sql"
        {
            bail!(
                "Workflow variable {} uses unsafe source reference",
                variable.key
            );
        }
    }
    Ok(())
}

fn validate_provider_requirement(provider: &WorkflowProviderRequirement) -> Result<()> {
    validate_visibility(&provider.visibility, "workflow provider visibility")?;
    if provider.mode != "deterministic_mock" {
        bail!(
            "Workflow provider requirement {} must use deterministic_mock mode",
            provider.key
        );
    }
    if provider.egress != "none" {
        bail!(
            "Workflow provider requirement {} declares hidden provider egress",
            provider.key
        );
    }
    Ok(())
}

fn validate_task_binding(
    task: &WorkflowTaskBinding,
    fanout_keys: &BTreeSet<String>,
    provider_keys: &BTreeSet<String>,
    approval_actions: &BTreeSet<String>,
) -> Result<()> {
    if task.key.trim().is_empty() {
        bail!("Workflow task key cannot be blank");
    }
    validate_visibility(&task.visibility, "workflow task visibility")?;
    if !task.method.contains('.') {
        bail!(
            "Workflow task {} must bind a product-shaped method",
            task.key
        );
    }
    if matches!(
        task.method.as_str(),
        "query_sql" | "get_context" | "run_tool" | "tool.run"
    ) {
        bail!("Workflow task {} uses a generic unsafe method", task.key);
    }
    if let Some(fanout_key) = task.fanout.as_deref() {
        if !fanout_keys.contains(fanout_key) {
            bail!(
                "Workflow task {} references missing fanout group {}",
                task.key,
                fanout_key
            );
        }
    }
    if let Some(provider_key) = task.provider_requirement.as_deref() {
        if !provider_keys.contains(provider_key) {
            bail!(
                "Workflow task {} references missing provider requirement {}",
                task.key,
                provider_key
            );
        }
    }
    if let Some(action) = task.sensitive_action.as_deref() {
        if !approval_actions.contains(action) {
            bail!(
                "Workflow task {} requires missing approval gate for sensitive action {}",
                task.key,
                action
            );
        }
    }
    Ok(())
}

fn compile_safe_plan(template: &WorkflowTemplateDefinition, input: &Value) -> Result<Value> {
    let variables = resolve_variables(template, input)?;
    let fanouts = resolve_fanouts(template, input)?;
    let tasks = expand_tasks(template, &variables, &fanouts)?;
    Ok(json!({
        "schemaVersion": 1,
        "template": {
            "id": template.template_id,
            "version": template.version,
            "name": template.name,
            "packId": template.pack_id,
            "visibilityCeiling": template.visibility_ceiling,
            "idempotencyStrategy": template.idempotency_strategy,
        },
        "inputSchema": template.input_schema,
        "variables": variables,
        "fanoutGroups": fanouts,
        "tasks": tasks,
        "approvalGates": template.approval_gates,
        "providerRequirements": template.provider_requirements,
        "deterministicMocks": template.deterministic_mocks,
        "auditEvents": template.audit_events,
        "boundaries": {
            "canonicalTablesOwnTruth": true,
            "eventsOwnAuditReplay": true,
            "workflowDefinesReusablePlanOnly": true,
            "defaultValidationRequiresLiveProviders": false,
        }
    }))
}

fn resolve_variables(
    template: &WorkflowTemplateDefinition,
    input: &Value,
) -> Result<BTreeMap<String, Value>> {
    let mut variables = BTreeMap::new();
    for variable in &template.variables {
        let value = match variable.source_kind.as_str() {
            "input" => {
                let source_ref = variable.source_ref.as_deref().unwrap_or(&variable.key);
                input.get(source_ref).ok_or_else(|| {
                    anyhow::anyhow!("Workflow input is missing variable source {source_ref}")
                })?
            }
            _ => {
                variables.insert(
                    variable.key.clone(),
                    json!({
                        "sourceKind": variable.source_kind,
                        "sourceRef": variable.source_ref,
                        "visibility": variable.visibility,
                        "evidenceRefs": variable.evidence_refs,
                        "status": "requiresRuntimeResolution",
                    }),
                );
                continue;
            }
        };
        variables.insert(
            variable.key.clone(),
            safe_variable_value(variable, value.clone()),
        );
    }
    Ok(variables)
}

fn safe_variable_value(variable: &WorkflowVariable, value: Value) -> Value {
    if is_public_safe_visibility(&variable.visibility) {
        json!({
            "value": value,
            "visibility": variable.visibility,
            "sourceKind": variable.source_kind,
            "evidenceRefs": variable.evidence_refs,
        })
    } else {
        json!({
            "privateValueHash": content_hash(&canonical_json_string(&value)),
            "visibility": variable.visibility,
            "sourceKind": variable.source_kind,
            "evidenceRefs": variable.evidence_refs,
        })
    }
}

fn safe_fanout_item_value(variable: &WorkflowVariable, value: Value) -> Value {
    if is_public_safe_visibility(&variable.visibility) {
        value
    } else {
        json!({
            "privateValueHash": content_hash(&canonical_json_string(&value)),
            "visibility": variable.visibility,
        })
    }
}

fn safe_fanout_item_key(
    variable: &WorkflowVariable,
    index: usize,
    value: &Value,
    internal_item_key: &str,
) -> String {
    if is_public_safe_visibility(&variable.visibility) {
        internal_item_key.to_string()
    } else {
        let hash = content_hash(&canonical_json_string(value));
        let suffix = hash.trim_start_matches("sha256:");
        let short_hash = suffix.get(0..12).unwrap_or(suffix);
        format!("item-{index}-{short_hash}")
    }
}

fn resolve_fanouts(
    template: &WorkflowTemplateDefinition,
    input: &Value,
) -> Result<BTreeMap<String, Value>> {
    let variable_by_key: BTreeMap<&str, &WorkflowVariable> = template
        .variables
        .iter()
        .map(|variable| (variable.key.as_str(), variable))
        .collect();
    let mut fanouts = BTreeMap::new();
    for fanout in &template.fanout_groups {
        let variable = variable_by_key
            .get(fanout.collection_variable.as_str())
            .ok_or_else(|| {
                anyhow::anyhow!("Missing fanout variable {}", fanout.collection_variable)
            })?;
        let source_ref = variable.source_ref.as_deref().unwrap_or(&variable.key);
        let items = input
            .get(source_ref)
            .and_then(Value::as_array)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Workflow fanout {} source {} must be an array",
                    fanout.key,
                    source_ref
                )
            })?;
        if items.len() as i64 > fanout.max_items {
            bail!(
                "Workflow fanout {} received {} items, above max {}",
                fanout.key,
                items.len(),
                fanout.max_items
            );
        }

        let mut item_keys = BTreeSet::new();
        let mut safe_items = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let internal_item_key = stable_item_key(index, item);
            if !item_keys.insert(internal_item_key.clone()) {
                bail!(
                    "Workflow fanout {} has duplicate item key {internal_item_key}",
                    fanout.key
                );
            }
            let item_key = safe_fanout_item_key(variable, index, item, &internal_item_key);
            safe_items.push(json!({
                "itemKey": item_key,
                "idempotencyKey": format!("{}:{}", fanout.key, item_key),
                "value": safe_fanout_item_value(variable, item.clone()),
                "visibility": variable.visibility,
            }));
        }
        fanouts.insert(
            fanout.key.clone(),
            json!({
                "collectionVariable": fanout.collection_variable,
                "itemVariable": fanout.item_variable,
                "maxItems": fanout.max_items,
                "items": safe_items,
            }),
        );
    }
    Ok(fanouts)
}

fn expand_tasks(
    template: &WorkflowTemplateDefinition,
    variables: &BTreeMap<String, Value>,
    fanouts: &BTreeMap<String, Value>,
) -> Result<Vec<Value>> {
    let mut tasks = Vec::new();
    for task in &template.tasks {
        if let Some(fanout_key) = task.fanout.as_deref() {
            let fanout = fanouts
                .get(fanout_key)
                .ok_or_else(|| anyhow::anyhow!("Missing fanout group {fanout_key}"))?;
            let items = fanout["items"].as_array().ok_or_else(|| {
                anyhow::anyhow!("Fanout group {fanout_key} items must be an array")
            })?;
            for item in items {
                let item_key = item["itemKey"].as_str().unwrap_or("item");
                tasks.push(json!({
                    "key": format!("{}[{item_key}]", task.key),
                    "baseKey": task.key,
                    "method": task.method,
                    "dependsOn": task.depends_on,
                    "visibility": task.visibility,
                    "fanout": fanout_key,
                    "fanoutItemKey": item_key,
                    "providerRequirement": task.provider_requirement,
                    "outputArtifactKind": task.output_artifact_kind,
                    "retryPolicy": task.retry_policy,
                    "input": resolve_binding_value(&task.input, variables, Some(item))?,
                }));
            }
        } else {
            tasks.push(json!({
                "key": task.key,
                "method": task.method,
                "dependsOn": task.depends_on,
                "visibility": task.visibility,
                "providerRequirement": task.provider_requirement,
                "outputArtifactKind": task.output_artifact_kind,
                "retryPolicy": task.retry_policy,
                "input": resolve_binding_value(&task.input, variables, None)?,
            }));
        }
    }
    Ok(tasks)
}

fn resolve_binding_value(
    value: &Value,
    variables: &BTreeMap<String, Value>,
    fanout_item: Option<&Value>,
) -> Result<Value> {
    match value {
        Value::Array(items) => Ok(Value::Array(
            items
                .iter()
                .map(|item| resolve_binding_value(item, variables, fanout_item))
                .collect::<Result<Vec<_>>>()?,
        )),
        Value::Object(map) => {
            if let Some(variable_key) = map.get("$var").and_then(Value::as_str) {
                let variable = variables.get(variable_key).ok_or_else(|| {
                    anyhow::anyhow!("Task binding references undeclared variable {variable_key}")
                })?;
                return Ok(variable.clone());
            }
            if let Some(item_field) = map.get("$fanoutItem").and_then(Value::as_str) {
                let item = fanout_item.ok_or_else(|| {
                    anyhow::anyhow!("Task binding references fanout item outside fanout")
                })?;
                return item
                    .get(item_field)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Fanout item is missing field {item_field}"));
            }

            let mut resolved = Map::new();
            for (key, item) in map {
                resolved.insert(
                    key.clone(),
                    resolve_binding_value(item, variables, fanout_item)?,
                );
            }
            Ok(Value::Object(resolved))
        }
        _ => Ok(value.clone()),
    }
}

fn load_compilation_by_idempotency(
    connection: &Connection,
    template_id: &str,
    template_version: i64,
    idempotency_key: &str,
) -> Result<Option<WorkflowCompilation>> {
    connection
        .query_row(
            "SELECT id, template_id, template_version, idempotency_key, input_hash, safe_compiled_plan_json
             FROM workflow_template_compilations
             WHERE template_id = ?1 AND template_version = ?2 AND idempotency_key = ?3
             LIMIT 1",
            params![template_id, template_version, idempotency_key],
            |row| {
                let plan_json: String = row.get(5)?;
                let safe_compiled_plan: Value = serde_json::from_str(&plan_json).map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(err),
                    )
                })?;
                Ok(WorkflowCompilation {
                    id: row.get(0)?,
                    template_id: row.get(1)?,
                    template_version: row.get(2)?,
                    idempotency_key: row.get(3)?,
                    input_hash: row.get(4)?,
                    safe_compiled_plan,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn validate_visibility(value: &str, label: &str) -> Result<()> {
    match value {
        "public" | "authenticated" | "staff" | "owner" | "private" => Ok(()),
        _ => bail!("{label} must be public, authenticated, staff, owner, or private"),
    }
}

fn is_public_safe_visibility(value: &str) -> bool {
    matches!(value, "public" | "authenticated")
}

fn normalize_idempotency_key(idempotency_key: &str) -> Result<String> {
    let key = idempotency_key.trim();
    if key.is_empty() {
        bail!("Workflow template idempotency key cannot be blank");
    }
    if key.len() > 200 {
        bail!("Workflow template idempotency key is too long");
    }
    Ok(key.to_string())
}

fn workflow_template_row_id(template_id: &str, version: i64) -> String {
    format!("workflow_template:{}:{}", template_id, version)
}

fn stable_item_key(index: usize, item: &Value) -> String {
    item.as_str()
        .map(slugify)
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| format!("item-{index}"))
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

fn content_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn canonical_json_string(value: &Value) -> String {
    canonical_json_value(value).to_string()
}

fn canonical_json_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonical_json_value).collect()),
        Value::Object(map) => {
            let mut sorted = Map::new();
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

fn zodiac_image_set_template() -> WorkflowTemplateDefinition {
    WorkflowTemplateDefinition {
        template_id: "story.zodiac_image_set".to_string(),
        version: 1,
        name: "Zodiac Image Set".to_string(),
        pack_id: "studio.story".to_string(),
        visibility_ceiling: "staff".to_string(),
        idempotency_strategy: "required_idempotency_key".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["subjects", "visualStyle", "outputCountPerSubject"],
            "additionalProperties": false,
            "properties": {
                "subjects": {
                    "type": "array",
                    "maxItems": 12,
                    "items": { "type": "string", "minLength": 1 }
                },
                "visualStyle": { "type": "string", "minLength": 1 },
                "outputCountPerSubject": { "type": "integer", "minimum": 1, "maximum": 4 }
            }
        }),
        variables: vec![
            input_variable("subjects", "array", "subjects", "staff"),
            input_variable("visualStyle", "string", "visualStyle", "staff"),
            input_variable(
                "outputCountPerSubject",
                "integer",
                "outputCountPerSubject",
                "staff",
            ),
        ],
        fanout_groups: vec![WorkflowFanoutGroup {
            key: "subject".to_string(),
            collection_variable: "subjects".to_string(),
            item_variable: "subject".to_string(),
            max_items: 12,
        }],
        tasks: vec![
            WorkflowTaskBinding {
                key: "image.brief".to_string(),
                method: "story.createImageBrief".to_string(),
                input: json!({
                    "subject": { "$fanoutItem": "value" },
                    "visualStyle": { "$var": "visualStyle" }
                }),
                retry_policy: json!({ "maxAttempts": 2 }),
                depends_on: vec![],
                visibility: "staff".to_string(),
                fanout: Some("subject".to_string()),
                provider_requirement: None,
                output_artifact_kind: Some("image_brief".to_string()),
                sensitive_action: None,
            },
            WorkflowTaskBinding {
                key: "image.generate".to_string(),
                method: "image.generateVariants".to_string(),
                input: json!({
                    "subject": { "$fanoutItem": "value" },
                    "count": { "$var": "outputCountPerSubject" }
                }),
                retry_policy: json!({ "maxAttempts": 2 }),
                depends_on: vec!["image.brief".to_string()],
                visibility: "staff".to_string(),
                fanout: Some("subject".to_string()),
                provider_requirement: Some("image.mock".to_string()),
                output_artifact_kind: Some("generated_image".to_string()),
                sensitive_action: None,
            },
        ],
        approval_gates: vec![],
        provider_requirements: vec![mock_provider("image.mock", "image.generateVariants")],
        deterministic_mocks: vec![WorkflowDeterministicMock {
            key: "image.mock".to_string(),
            capability: "image.generateVariants".to_string(),
            fixture_ref: "fixtures/story/zodiac-image-set.json".to_string(),
        }],
        audit_events: vec!["workflow.template.compiled".to_string()],
    }
}

fn article_with_image_template() -> WorkflowTemplateDefinition {
    WorkflowTemplateDefinition {
        template_id: "content.article_with_image".to_string(),
        version: 1,
        name: "Article With Image".to_string(),
        pack_id: "studio.story".to_string(),
        visibility_ceiling: "staff".to_string(),
        idempotency_strategy: "required_idempotency_key".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["topic", "audience"],
            "additionalProperties": false,
            "properties": {
                "topic": { "type": "string", "minLength": 1 },
                "audience": { "type": "string", "minLength": 1 }
            }
        }),
        variables: vec![
            input_variable("topic", "string", "topic", "authenticated"),
            input_variable("audience", "string", "audience", "staff"),
        ],
        tasks: vec![
            WorkflowTaskBinding {
                key: "article.draft".to_string(),
                method: "content.draftArticle".to_string(),
                input: json!({ "topic": { "$var": "topic" }, "audience": { "$var": "audience" } }),
                retry_policy: json!({ "maxAttempts": 2 }),
                depends_on: vec![],
                visibility: "staff".to_string(),
                fanout: None,
                provider_requirement: Some("llm.mock".to_string()),
                output_artifact_kind: Some("article_draft".to_string()),
                sensitive_action: None,
            },
            WorkflowTaskBinding {
                key: "image.brief".to_string(),
                method: "story.createImageBrief".to_string(),
                input: json!({ "topic": { "$var": "topic" } }),
                retry_policy: json!({ "maxAttempts": 2 }),
                depends_on: vec!["article.draft".to_string()],
                visibility: "staff".to_string(),
                fanout: None,
                provider_requirement: None,
                output_artifact_kind: Some("image_brief".to_string()),
                sensitive_action: None,
            },
        ],
        fanout_groups: vec![],
        approval_gates: vec![],
        provider_requirements: vec![mock_provider("llm.mock", "content.draftArticle")],
        deterministic_mocks: vec![WorkflowDeterministicMock {
            key: "llm.mock".to_string(),
            capability: "content.draftArticle".to_string(),
            fixture_ref: "fixtures/content/article-with-image.json".to_string(),
        }],
        audit_events: vec!["workflow.template.compiled".to_string()],
    }
}

fn story_scrollytelling_homepage_template() -> WorkflowTemplateDefinition {
    WorkflowTemplateDefinition {
        template_id: "studio.story.scrollytelling_homepage".to_string(),
        version: 1,
        name: "Story Pack Scrollytelling Homepage".to_string(),
        pack_id: "studio.story".to_string(),
        visibility_ceiling: "staff".to_string(),
        idempotency_strategy: "required_idempotency_key".to_string(),
        input_schema: json!({
            "type": "object",
            "required": ["founderProfile", "businessPositioning", "sections", "publishMode"],
            "additionalProperties": false,
            "properties": {
                "founderProfile": { "type": "string", "minLength": 1 },
                "businessPositioning": { "type": "string", "minLength": 1 },
                "sections": {
                    "type": "array",
                    "maxItems": 12,
                    "items": { "type": "string", "minLength": 1 }
                },
                "publishMode": { "enum": ["manual", "scheduled"] }
            }
        }),
        variables: vec![
            input_variable("founderProfile", "string", "founderProfile", "private"),
            input_variable(
                "businessPositioning",
                "string",
                "businessPositioning",
                "staff",
            ),
            input_variable("sections", "array", "sections", "staff"),
            input_variable("publishMode", "string", "publishMode", "staff"),
        ],
        fanout_groups: vec![WorkflowFanoutGroup {
            key: "section".to_string(),
            collection_variable: "sections".to_string(),
            item_variable: "section".to_string(),
            max_items: 12,
        }],
        tasks: vec![
            WorkflowTaskBinding {
                key: "deck.create".to_string(),
                method: "homepage.createNarrativeDeck".to_string(),
                input: json!({
                    "businessPositioning": { "$var": "businessPositioning" },
                    "founderProfile": { "$var": "founderProfile" }
                }),
                retry_policy: json!({ "maxAttempts": 2 }),
                depends_on: vec![],
                visibility: "staff".to_string(),
                fanout: None,
                provider_requirement: Some("llm.mock".to_string()),
                output_artifact_kind: Some("narrative_deck".to_string()),
                sensitive_action: None,
            },
            WorkflowTaskBinding {
                key: "section.image_brief".to_string(),
                method: "story.createImageBriefs".to_string(),
                input: json!({ "section": { "$fanoutItem": "value" } }),
                retry_policy: json!({ "maxAttempts": 2 }),
                depends_on: vec!["deck.create".to_string()],
                visibility: "staff".to_string(),
                fanout: Some("section".to_string()),
                provider_requirement: None,
                output_artifact_kind: Some("image_brief".to_string()),
                sensitive_action: None,
            },
            WorkflowTaskBinding {
                key: "publish.approval".to_string(),
                method: "publish.requestApproval".to_string(),
                input: json!({ "publishMode": { "$var": "publishMode" } }),
                retry_policy: json!({ "maxAttempts": 1 }),
                depends_on: vec!["section.image_brief".to_string()],
                visibility: "staff".to_string(),
                fanout: None,
                provider_requirement: None,
                output_artifact_kind: Some("approval_request".to_string()),
                sensitive_action: Some("publish".to_string()),
            },
        ],
        approval_gates: vec![WorkflowApprovalGate {
            key: "manual_publish_approval".to_string(),
            action: "publish".to_string(),
            required: true,
        }],
        provider_requirements: vec![mock_provider("llm.mock", "homepage.createNarrativeDeck")],
        deterministic_mocks: vec![WorkflowDeterministicMock {
            key: "llm.mock".to_string(),
            capability: "homepage.createNarrativeDeck".to_string(),
            fixture_ref: "fixtures/story/scrollytelling-homepage.json".to_string(),
        }],
        audit_events: vec![
            "workflow.template.compiled".to_string(),
            "approval.requested".to_string(),
        ],
    }
}

fn input_variable(
    key: &str,
    value_type: &str,
    source_ref: &str,
    visibility: &str,
) -> WorkflowVariable {
    WorkflowVariable {
        key: key.to_string(),
        value_type: value_type.to_string(),
        source_kind: "input".to_string(),
        source_ref: Some(source_ref.to_string()),
        visibility: visibility.to_string(),
        evidence_refs: vec![],
    }
}

fn mock_provider(key: &str, capability: &str) -> WorkflowProviderRequirement {
    WorkflowProviderRequirement {
        key: key.to_string(),
        capability: capability.to_string(),
        mode: "deterministic_mock".to_string(),
        egress: "none".to_string(),
        visibility: "staff".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;

    fn setup_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_workflow_templates(&connection).unwrap();
        connection
    }

    #[test]
    fn seeds_builtin_workflow_template_fixtures() {
        let connection = setup_connection();
        for template_id in [
            "story.zodiac_image_set",
            "content.article_with_image",
            "studio.story.scrollytelling_homepage",
        ] {
            load_workflow_template(&connection, template_id, 1).unwrap();
        }
    }

    #[test]
    fn compiles_story_template_with_typed_variables_bounded_fanout_and_approval_gate() {
        let mut connection = setup_connection();

        let compilation = compile_workflow_template(
            &mut connection,
            "studio.story.scrollytelling_homepage",
            1,
            json!({
                "founderProfile": "private founder story",
                "businessPositioning": "answer enshittification with owned local tools",
                "sections": ["private-origin-section", "private-method-section", "private-offer-section"],
                "publishMode": "manual"
            }),
            "story-homepage-1",
        )
        .unwrap();

        assert_eq!(
            compilation.safe_compiled_plan["template"]["id"],
            "studio.story.scrollytelling_homepage"
        );
        assert_eq!(
            compilation.safe_compiled_plan["variables"]["founderProfile"]["privateValueHash"]
                .as_str()
                .unwrap()
                .starts_with("sha256:"),
            true
        );
        let safe_plan_json = compilation.safe_compiled_plan.to_string();
        assert!(!safe_plan_json.contains("private-origin-section"));
        assert!(!safe_plan_json.contains("private-method-section"));
        assert!(!safe_plan_json.contains("private-offer-section"));
        assert!(!safe_plan_json.contains("private founder story"));
        assert_eq!(
            compilation.safe_compiled_plan["approvalGates"][0]["action"],
            "publish"
        );
        assert_eq!(
            compilation.safe_compiled_plan["boundaries"]["defaultValidationRequiresLiveProviders"],
            false
        );

        let expanded_tasks = compilation.safe_compiled_plan["tasks"].as_array().unwrap();
        assert_eq!(expanded_tasks.len(), 5);
        assert!(expanded_tasks.iter().any(|task| task["key"]
            .as_str()
            .unwrap()
            .starts_with("section.image_brief[item-0-")));
        assert!(expanded_tasks
            .iter()
            .filter_map(|task| task["input"]["section"]["privateValueHash"].as_str())
            .any(|hash| hash.starts_with("sha256:")));
    }

    #[test]
    fn rejects_unsafe_sources_hidden_egress_unbounded_fanout_and_missing_approval_gate() {
        let base = story_scrollytelling_homepage_template();
        let mut bad_source = base.clone();
        bad_source.variables[0].source_kind = "sql".to_string();
        let error = validate_workflow_template(&bad_source)
            .unwrap_err()
            .to_string();
        assert!(error.contains("unsafe source kind sql"));

        let mut hidden_egress = base.clone();
        hidden_egress.provider_requirements[0].egress = "network".to_string();
        let error = validate_workflow_template(&hidden_egress)
            .unwrap_err()
            .to_string();
        assert!(error.contains("hidden provider egress"));

        let mut unbounded = base.clone();
        unbounded.fanout_groups[0].max_items = 0;
        let error = validate_workflow_template(&unbounded)
            .unwrap_err()
            .to_string();
        assert!(error.contains("must be bounded"));

        let mut missing_gate = base;
        missing_gate.approval_gates.clear();
        let error = validate_workflow_template(&missing_gate)
            .unwrap_err()
            .to_string();
        assert!(error.contains("missing approval gate"));
    }

    #[test]
    fn idempotency_returns_existing_compilation_and_rejects_conflicting_input() {
        let mut connection = setup_connection();
        let first = compile_workflow_template(
            &mut connection,
            "content.article_with_image",
            1,
            json!({ "topic": "aliens", "audience": "curious small business owners" }),
            "article-image-1",
        )
        .unwrap();
        let repeated = compile_workflow_template(
            &mut connection,
            "content.article_with_image",
            1,
            json!({ "audience": "curious small business owners", "topic": "aliens" }),
            "article-image-1",
        )
        .unwrap();
        assert_eq!(first.id, repeated.id);
        assert_eq!(first.input_hash, repeated.input_hash);

        let error = compile_workflow_template(
            &mut connection,
            "content.article_with_image",
            1,
            json!({ "topic": "robots", "audience": "curious small business owners" }),
            "article-image-1",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("idempotency key conflicts"));

        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM workflow_template_compilations WHERE idempotency_key = 'article-image-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn invalid_input_rejects_without_partial_compilation_row() {
        let mut connection = setup_connection();

        let error = compile_workflow_template(
            &mut connection,
            "content.article_with_image",
            1,
            json!({ "topic": "aliens" }),
            "article-image-invalid",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("workflow template input failed JSON Schema validation"));

        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM workflow_template_compilations WHERE idempotency_key = 'article-image-invalid'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn handles_empty_fanout_and_rejects_duplicate_item_keys_deterministically() {
        let mut connection = setup_connection();
        let empty = compile_workflow_template(
            &mut connection,
            "story.zodiac_image_set",
            1,
            json!({
                "subjects": [],
                "visualStyle": "cinematic editorial",
                "outputCountPerSubject": 1
            }),
            "zodiac-empty",
        )
        .unwrap();
        let tasks = empty.safe_compiled_plan["tasks"].as_array().unwrap();
        assert!(tasks.is_empty());

        let error = compile_workflow_template(
            &mut connection,
            "story.zodiac_image_set",
            1,
            json!({
                "subjects": ["Aries", "aries"],
                "visualStyle": "cinematic editorial",
                "outputCountPerSubject": 1
            }),
            "zodiac-dupe",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("duplicate item key aries"));
    }
}
