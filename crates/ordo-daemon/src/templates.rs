use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::capabilities::assert_capability_ids_registered;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDefinition {
    pub key: String,
    #[serde(default)]
    pub capability_id: String,
    pub kind: String,
    pub label: String,
    pub required: bool,
    pub depends_on: Vec<String>,
    pub input: Value,
    #[serde(default = "empty_json_object")]
    pub retry_policy: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessTemplate {
    pub id: String,
    #[serde(default)]
    pub capability_id: String,
    pub kind: String,
    pub name: String,
    pub version: i64,
    pub description: String,
    #[serde(default = "empty_json_object")]
    pub variable_schema: Value,
    pub tasks: Vec<TaskDefinition>,
}

impl ProcessTemplate {
    pub fn effective_capability_id(&self) -> &str {
        if self.capability_id.is_empty() {
            &self.kind
        } else {
            &self.capability_id
        }
    }

    pub fn task(&self, key: &str) -> Option<&TaskDefinition> {
        self.tasks
            .iter()
            .find(|task_definition| task_definition.key == key)
    }
}

impl TaskDefinition {
    pub fn effective_capability_id(&self) -> &str {
        if self.capability_id.is_empty() {
            &self.kind
        } else {
            &self.capability_id
        }
    }
}

pub fn built_in_templates() -> Vec<ProcessTemplate> {
    vec![
        system_health_check_template(),
        system_brief_template(),
        surface_brief_template(),
        backup_create_template(),
        restore_execute_template(),
        issue_report_prepare_template(),
        promo_video_package_template(),
    ]
}

pub fn find_builtin_template(template_id: &str) -> Option<ProcessTemplate> {
    built_in_templates()
        .into_iter()
        .find(|template| template.id == template_id)
}

pub fn find_builtin_template_version(template_id: &str, version: i64) -> Option<ProcessTemplate> {
    built_in_templates()
        .into_iter()
        .find(|template| template.id == template_id && template.version == version)
}

pub fn seed_builtin_templates(connection: &Connection) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    for template in built_in_templates() {
        validate_template_capabilities(connection, &template)?;
        connection.execute(
            "INSERT INTO process_templates (
                id, capability_id, kind, name, version, description, variable_schema_json,
                tasks_json, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
             ON CONFLICT(id, version) DO UPDATE SET
                capability_id = excluded.capability_id,
                kind = excluded.kind,
                name = excluded.name,
                description = excluded.description,
                variable_schema_json = excluded.variable_schema_json,
                tasks_json = excluded.tasks_json,
                updated_at = excluded.updated_at",
            params![
                template.id,
                template.effective_capability_id(),
                template.kind,
                template.name,
                template.version,
                template.description,
                template.variable_schema.to_string(),
                serde_json::to_string(&template.tasks)?,
                now,
            ],
        )?;
    }

    Ok(())
}

fn validate_template_capabilities(
    connection: &Connection,
    template: &ProcessTemplate,
) -> Result<()> {
    let mut capability_ids = vec![template.effective_capability_id().to_string()];
    capability_ids.extend(
        template
            .tasks
            .iter()
            .map(|task_definition| task_definition.effective_capability_id().to_string()),
    );
    assert_capability_ids_registered(connection, &capability_ids)
}

pub fn require_builtin_template(template_id: &str) -> Result<ProcessTemplate> {
    find_builtin_template(template_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown built-in process template: {template_id}"))
}

pub fn require_builtin_template_version(
    template_id: &str,
    version: i64,
) -> Result<ProcessTemplate> {
    find_builtin_template_version(template_id, version).ok_or_else(|| {
        anyhow::anyhow!("Unknown built-in process template version: {template_id}@{version}")
    })
}

fn empty_json_object() -> Value {
    json!({})
}

fn task(key: &str, kind: &str, label: &str, depends_on: &[&str]) -> TaskDefinition {
    TaskDefinition {
        key: key.to_string(),
        capability_id: kind.to_string(),
        kind: kind.to_string(),
        label: label.to_string(),
        required: true,
        depends_on: depends_on
            .iter()
            .map(|dependency| dependency.to_string())
            .collect(),
        input: json!({}),
        retry_policy: json!({}),
    }
}

fn system_health_check_template() -> ProcessTemplate {
    ProcessTemplate {
        id: "system.health.check".to_string(),
        capability_id: "system.health.check".to_string(),
        kind: "system.health.check".to_string(),
        name: "System Health Check".to_string(),
        version: 1,
        description: "Capture basic appliance health evidence.".to_string(),
        variable_schema: json!({}),
        tasks: vec![
            task(
                "health.probe",
                "system.health.probe",
                "Probe appliance health",
                &[],
            ),
            task(
                "health.record",
                "system.health.record",
                "Record health evidence",
                &["health.probe"],
            ),
        ],
    }
}

fn system_brief_template() -> ProcessTemplate {
    ProcessTemplate {
        id: "brief.system.generate".to_string(),
        capability_id: "brief.system.generate".to_string(),
        kind: "brief.system.generate".to_string(),
        name: "Generate System Brief".to_string(),
        version: 1,
        description: "Write the durable System Brief from appliance evidence.".to_string(),
        variable_schema: json!({}),
        tasks: vec![
            task(
                "scope.validate",
                "brief.scope.validate",
                "Validate brief scope",
                &[],
            ),
            task(
                "evidence.collect",
                "brief.evidence.collect",
                "Collect system evidence",
                &["scope.validate"],
            ),
            task(
                "evidence.manifest",
                "brief.evidence.manifest",
                "Build evidence manifest",
                &["evidence.collect"],
            ),
            task(
                "draft.generate",
                "brief.draft.generate",
                "Generate brief draft",
                &["evidence.manifest"],
            ),
            task(
                "claims.validate",
                "brief.claims.validate",
                "Validate claims against evidence",
                &["draft.generate"],
            ),
            task(
                "artifact.save",
                "brief.artifact.save",
                "Save brief artifact",
                &["claims.validate"],
            ),
        ],
    }
}

fn surface_brief_template() -> ProcessTemplate {
    ProcessTemplate {
        id: "surface.brief.generate".to_string(),
        capability_id: "surface.brief.generate".to_string(),
        kind: "surface.brief.generate".to_string(),
        name: "Generate Surface Brief".to_string(),
        version: 1,
        description:
            "Write an evidence-backed surface brief artifact without blocking the surface."
                .to_string(),
        variable_schema: json!({}),
        tasks: vec![
            task(
                "scope.validate",
                "brief.scope.validate",
                "Validate surface brief scope",
                &[],
            ),
            task(
                "evidence.collect",
                "brief.evidence.collect",
                "Collect surface evidence",
                &["scope.validate"],
            ),
            task(
                "draft.generate",
                "brief.draft.generate",
                "Generate deterministic surface brief",
                &["evidence.collect"],
            ),
            task(
                "claims.validate",
                "brief.claims.validate",
                "Validate surface brief claims",
                &["draft.generate"],
            ),
            task(
                "artifact.save",
                "artifacts.brief.generate",
                "Save surface brief artifact",
                &["claims.validate"],
            ),
        ],
    }
}

fn backup_create_template() -> ProcessTemplate {
    ProcessTemplate {
        id: "backup.create".to_string(),
        capability_id: "backup.create".to_string(),
        kind: "backup.create".to_string(),
        name: "Create Backup".to_string(),
        version: 1,
        description: "Create a backup artifact and integrity manifest.".to_string(),
        variable_schema: json!({}),
        tasks: vec![
            task(
                "boundary.check",
                "backup.boundary.check",
                "Check data boundary",
                &[],
            ),
            task(
                "lock.acquire",
                "backup.lock.acquire",
                "Acquire backup lock",
                &["boundary.check"],
            ),
            task(
                "sqlite.snapshot",
                "backup.sqlite.snapshot",
                "Snapshot SQLite",
                &["lock.acquire"],
            ),
            task(
                "files.scan",
                "backup.files.scan",
                "Scan files",
                &["lock.acquire"],
            ),
            task(
                "archive.write",
                "backup.archive.write",
                "Write archive",
                &["sqlite.snapshot", "files.scan"],
            ),
            task(
                "manifest.write",
                "backup.manifest.write",
                "Write manifest",
                &["archive.write"],
            ),
            task(
                "integrity.verify",
                "backup.integrity.verify",
                "Verify integrity",
                &["manifest.write"],
            ),
            task(
                "backup.record",
                "backup.record",
                "Record backup",
                &["integrity.verify"],
            ),
        ],
    }
}

fn restore_execute_template() -> ProcessTemplate {
    ProcessTemplate {
        id: "restore.execute".to_string(),
        capability_id: "restore.execute".to_string(),
        kind: "restore.execute".to_string(),
        name: "Execute Restore".to_string(),
        version: 1,
        description: "Restore from an appliance backup with confirmation and safety backup."
            .to_string(),
        variable_schema: json!({}),
        tasks: vec![
            task(
                "request.validate",
                "restore.request.validate",
                "Validate restore request",
                &[],
            ),
            task(
                "archive.verify",
                "restore.archive.verify",
                "Verify backup archive",
                &["request.validate"],
            ),
            task(
                "confirmation.require",
                "restore.confirmation.require",
                "Require confirmation",
                &["request.validate"],
            ),
            task(
                "safety.backup",
                "restore.safety.backup",
                "Create safety backup",
                &["confirmation.require"],
            ),
            task(
                "lock.acquire",
                "restore.lock.acquire",
                "Acquire restore lock",
                &["archive.verify", "safety.backup"],
            ),
            task(
                "sqlite.restore",
                "restore.sqlite.restore",
                "Restore SQLite",
                &["lock.acquire"],
            ),
            task(
                "files.restore",
                "restore.files.restore",
                "Restore files",
                &["lock.acquire"],
            ),
            task(
                "state.verify",
                "restore.state.verify",
                "Verify restored state",
                &["sqlite.restore", "files.restore"],
            ),
            task(
                "app.restart",
                "system.next.restart",
                "Restart app if needed",
                &["state.verify"],
            ),
            task(
                "restore.record",
                "restore.record",
                "Record restore",
                &["app.restart"],
            ),
        ],
    }
}

fn issue_report_prepare_template() -> ProcessTemplate {
    ProcessTemplate {
        id: "issue.report.prepare".to_string(),
        capability_id: "issue.report.prepare".to_string(),
        kind: "issue.report.prepare".to_string(),
        name: "Prepare Issue Report".to_string(),
        version: 1,
        description: "Prepare a local issue report with redacted diagnostics and evidence."
            .to_string(),
        variable_schema: json!({}),
        tasks: vec![
            task(
                "scope.validate",
                "issue.scope.validate",
                "Validate issue scope",
                &[],
            ),
            task(
                "narrative.capture",
                "issue.narrative.capture",
                "Capture issue narrative",
                &[],
            ),
            task(
                "diagnostics.collect",
                "issue.diagnostics.collect",
                "Collect diagnostics",
                &["scope.validate"],
            ),
            task(
                "events.collect",
                "issue.events.collect",
                "Collect recent events",
                &["scope.validate"],
            ),
            task(
                "jobs.collect",
                "issue.jobs.collect",
                "Collect recent jobs",
                &["scope.validate"],
            ),
            task(
                "redactions.apply",
                "issue.redactions.apply",
                "Apply redactions",
                &["diagnostics.collect", "events.collect", "jobs.collect"],
            ),
            task(
                "draft.generate",
                "issue.draft.generate",
                "Generate report draft",
                &["narrative.capture", "redactions.apply"],
            ),
            task(
                "artifact.save",
                "issue.artifact.save",
                "Save report artifact",
                &["draft.generate"],
            ),
        ],
    }
}

fn promo_video_package_template() -> ProcessTemplate {
    ProcessTemplate {
        id: "studio.promo_video.package".to_string(),
        capability_id: "studio.promo_video.package".to_string(),
        kind: "studio.promo_video.package".to_string(),
        name: "Stage Promo Video Package".to_string(),
        version: 1,
        description: "Create a deterministic staged package for a 10-30 second vertical promo without external publishing.".to_string(),
        variable_schema: json!({}),
        tasks: vec![
            task(
                "brief.validate",
                "studio.promo_video.brief.validate",
                "Validate promo brief and package boundaries",
                &[],
            ),
            task(
                "script.draft",
                "studio.promo_video.script.draft",
                "Draft deterministic short promo script",
                &["brief.validate"],
            ),
            task(
                "media.plan",
                "studio.promo_video.media.plan",
                "Plan prompt-only vertical media beats",
                &["script.draft"],
            ),
            task(
                "captions.prepare",
                "studio.promo_video.captions.prepare",
                "Prepare caption cues",
                &["script.draft", "media.plan"],
            ),
            task(
                "package.stage",
                "studio.promo_video.package.stage",
                "Stage local promo package artifact",
                &["captions.prepare"],
            ),
        ],
    }
}

pub fn assert_template_exists(template_id: &str) -> Result<()> {
    if find_builtin_template(template_id).is_none() {
        bail!("Unknown process template: {template_id}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::schema::init_schema;

    #[test]
    fn includes_phase_one_templates() {
        let template_ids: Vec<String> = built_in_templates()
            .into_iter()
            .map(|template| template.id)
            .collect();

        assert!(template_ids.contains(&"system.health.check".to_string()));
        assert!(template_ids.contains(&"brief.system.generate".to_string()));
        assert!(template_ids.contains(&"surface.brief.generate".to_string()));
        assert!(template_ids.contains(&"backup.create".to_string()));
        assert!(template_ids.contains(&"restore.execute".to_string()));
        assert!(template_ids.contains(&"issue.report.prepare".to_string()));
        assert!(template_ids.contains(&"studio.promo_video.package".to_string()));
    }

    #[test]
    fn built_in_templates_reference_registered_capabilities() {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();

        seed_builtin_templates(&connection).unwrap();
    }
}
