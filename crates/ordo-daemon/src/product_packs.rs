use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;

use crate::capabilities::load_capability;
use crate::events::{append_realtime_event_tx, system_event};
use crate::json_contracts::validate_json_schema_document;
use crate::policy::{
    provenance_metadata, ActorContext, ActorKind, PolicyAction, ResourceClassification,
    ResourceKind, ResourceRef,
};
use crate::schema::db::ConnectionExt;
use crate::templates::require_builtin_template_version;
use crate::workflow_templates::load_workflow_template;

pub const PRODUCT_PACK_STATUS_ENABLED: &str = "enabled";
pub const PRODUCT_PACK_STATUS_DISABLED: &str = "disabled";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackInstallRequest {
    pub manifest: ProductPackManifest,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    #[serde(default)]
    pub provenance: Value,
    #[serde(default)]
    pub capability_bindings: Vec<ProductPackCapabilityBinding>,
    #[serde(default)]
    pub job_template_bindings: Vec<ProductPackJobTemplateBinding>,
    #[serde(default)]
    pub workflow_template_bindings: Vec<ProductPackWorkflowTemplateBinding>,
    #[serde(default)]
    pub request_templates: Vec<ProductPackRequestTemplateBinding>,
    #[serde(default)]
    pub artifact_contracts: Vec<ProductPackArtifactContractBinding>,
    #[serde(default)]
    pub graph_node_kinds: Vec<ProductPackGraphNodeKindBinding>,
    #[serde(default)]
    pub graph_edge_kinds: Vec<ProductPackGraphEdgeKindBinding>,
    #[serde(default)]
    pub projection_surfaces: Vec<ProductPackProjectionSurfaceBinding>,
    #[serde(default)]
    pub llm_method_bindings: Vec<ProductPackLlmMethodBinding>,
    #[serde(default)]
    pub visibility: Value,
    #[serde(default)]
    pub access: Value,
    #[serde(default)]
    pub growth: Value,
    #[serde(default)]
    pub limits: Value,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackCapabilityBinding {
    pub key: String,
    pub capability_id: String,
    #[serde(default)]
    pub visibility: Value,
    #[serde(default)]
    pub limits: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackJobTemplateBinding {
    pub key: String,
    pub template_id: String,
    pub template_version: i64,
    #[serde(default)]
    pub visibility: Value,
    #[serde(default)]
    pub limits: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackWorkflowTemplateBinding {
    pub key: String,
    pub template_id: String,
    pub template_version: i64,
    #[serde(default)]
    pub visibility: Value,
    #[serde(default)]
    pub limits: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackRequestTemplateBinding {
    pub key: String,
    pub title: String,
    pub capability_id: String,
    #[serde(default)]
    pub contract: Value,
    #[serde(default)]
    pub visibility: Value,
    #[serde(default)]
    pub limits: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackArtifactContractBinding {
    pub key: String,
    pub artifact_kind: String,
    #[serde(default)]
    pub contract: Value,
    #[serde(default)]
    pub visibility: Value,
    #[serde(default)]
    pub limits: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackGraphNodeKindBinding {
    pub key: String,
    pub node_kind: String,
    #[serde(default)]
    pub contract: Value,
    #[serde(default)]
    pub visibility: Value,
    #[serde(default)]
    pub limits: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackGraphEdgeKindBinding {
    pub key: String,
    pub edge_kind: String,
    #[serde(default)]
    pub contract: Value,
    #[serde(default)]
    pub visibility: Value,
    #[serde(default)]
    pub limits: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackProjectionSurfaceBinding {
    pub key: String,
    pub surface: String,
    #[serde(default)]
    pub contract: Value,
    #[serde(default)]
    pub visibility: Value,
    #[serde(default)]
    pub limits: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackLlmMethodBinding {
    pub key: String,
    pub method_name: String,
    #[serde(default)]
    pub contract: Value,
    #[serde(default)]
    pub visibility: Value,
    #[serde(default)]
    pub limits: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackListResponse {
    pub packs: Vec<ProductPackView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackResponse {
    pub pack: ProductPackView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackView {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: String,
    pub manifest: Value,
    pub validation: Value,
    pub provenance: Value,
    pub bindings: Vec<ProductPackBindingView>,
    pub created_by_actor_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackBindingView {
    pub id: String,
    pub pack_id: String,
    pub binding_kind: String,
    pub binding_key: String,
    pub capability_id: Option<String>,
    pub template_id: Option<String>,
    pub template_version: Option<i64>,
    pub artifact_kind: Option<String>,
    pub contract: Value,
    pub visibility: Value,
    pub access: Value,
    pub growth: Value,
    pub limits: Value,
    pub status: String,
    pub disabled_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPackMemberSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: String,
    pub binding_count: usize,
    pub available_binding_kinds: Vec<String>,
}

pub fn list_product_packs(_db_path: &Path) -> Result<ProductPackListResponse> {
    let connection = Connection::open(_db_path)?;
    let mut statement = connection.prepare(
        "SELECT id, name, version, status, manifest_json, validation_json, provenance_json,
                created_by_actor_id, created_at, updated_at
         FROM product_packs ORDER BY updated_at DESC, id ASC",
    )?;
    let rows = statement.query_map([], product_pack_from_row)?;
    let mut packs = Vec::new();
    for row in rows {
        let record = row?;
        let bindings = load_pack_bindings(&connection, &record.id)?;
        packs.push(record.into_view(bindings));
    }
    Ok(ProductPackListResponse { packs })
}

pub fn read_product_pack(db_path: &Path, pack_id: &str) -> Result<ProductPackResponse> {
    let connection = Connection::open(db_path)?;
    let record = require_product_pack(&connection, pack_id)?;
    let bindings = load_pack_bindings(&connection, pack_id)?;
    Ok(ProductPackResponse {
        pack: record.into_view(bindings),
    })
}

pub fn read_product_pack_member_summary(
    db_path: &Path,
    pack_id: &str,
) -> Result<ProductPackMemberSummary> {
    let response = read_product_pack(db_path, pack_id)?;
    let mut binding_kinds = BTreeSet::new();
    for binding in &response.pack.bindings {
        if binding.status == PRODUCT_PACK_STATUS_ENABLED {
            binding_kinds.insert(binding.binding_kind.clone());
        }
    }
    Ok(ProductPackMemberSummary {
        id: response.pack.id,
        name: response.pack.name,
        version: response.pack.version,
        status: response.pack.status,
        binding_count: response.pack.bindings.len(),
        available_binding_kinds: binding_kinds.into_iter().collect(),
    })
}

pub fn install_product_pack(
    db_path: &Path,
    request: ProductPackInstallRequest,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<ProductPackResponse> {
    let mut connection = Connection::open(db_path)?;
    let validation = validate_product_pack_manifest(&connection, &request.manifest)?;
    let transaction = connection.transaction()?;
    let now = Utc::now().to_rfc3339();
    let provenance = product_pack_provenance(&request.manifest.id, origin, actor_id);
    let manifest_json = serde_json::to_string(&request.manifest)?;
    let validation_json = validation.to_string();
    let provenance_json = provenance.to_string();
    transaction.execute(
        "INSERT INTO product_packs (
            id, name, version, status, manifest_json, validation_json, provenance_json,
            created_by_actor_id, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            version = excluded.version,
            status = excluded.status,
            manifest_json = excluded.manifest_json,
            validation_json = excluded.validation_json,
            provenance_json = excluded.provenance_json,
            created_by_actor_id = excluded.created_by_actor_id,
            updated_at = excluded.updated_at",
        params![
            request.manifest.id,
            request.manifest.name,
            request.manifest.version,
            PRODUCT_PACK_STATUS_ENABLED,
            manifest_json,
            validation_json,
            provenance_json,
            actor_id,
            now,
        ],
    )?;
    let version_id = format!("{}@{}", request.manifest.id, request.manifest.version);
    transaction.execute(
        "INSERT INTO product_pack_versions (
            id, pack_id, version, manifest_json, validation_json, provenance_json,
            installed_by_actor_id, installed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(pack_id, version) DO UPDATE SET
            manifest_json = excluded.manifest_json,
            validation_json = excluded.validation_json,
            provenance_json = excluded.provenance_json,
            installed_by_actor_id = excluded.installed_by_actor_id,
            installed_at = excluded.installed_at",
        params![
            version_id,
            request.manifest.id,
            request.manifest.version,
            manifest_json,
            validation_json,
            provenance_json,
            actor_id,
            now,
        ],
    )?;
    transaction.execute(
        "DELETE FROM product_pack_bindings WHERE pack_id = ?1",
        [request.manifest.id.as_str()],
    )?;
    insert_manifest_bindings(&transaction, &request.manifest, &now)?;
    append_realtime_event_tx(
        &transaction,
        &system_event(
            "product_pack.installed",
            json!({
                "packId": request.manifest.id,
                "version": request.manifest.version,
                "bindingCount": validation["bindingCount"],
                "status": PRODUCT_PACK_STATUS_ENABLED,
            }),
        ),
    )?;
    transaction.commit()?;
    read_product_pack(db_path, &request.manifest.id)
}

pub fn story_pack_manifest() -> ProductPackManifest {
    ProductPackManifest {
        id: "studio.story".to_string(),
        name: "Studio Story Pack".to_string(),
        version: "0.1.0".to_string(),
        description: Some(
            "Internal governed workflow declarations for scrollytelling homepage production."
                .to_string(),
        ),
        provenance: json!({
            "source": "docs/architecture/pack-kernel.md",
            "reviewedBy": "owner",
            "reason": "Internal Story Pack manifest for scrollytelling homepage workflow"
        }),
        capability_bindings: vec![
            ProductPackCapabilityBinding {
                key: "surface_brief_generate".to_string(),
                capability_id: "surface.brief.generate".to_string(),
                visibility: json!({ "surface": "studio", "member": false }),
                limits: json!({ "deterministicOnly": true }),
            },
            ProductPackCapabilityBinding {
                key: "artifact_brief_generate".to_string(),
                capability_id: "artifacts.brief.generate".to_string(),
                visibility: json!({ "surface": "studio", "member": false }),
                limits: json!({ "deterministicOnly": true }),
            },
            ProductPackCapabilityBinding {
                key: "promo_video_package".to_string(),
                capability_id: "studio.promo_video.package".to_string(),
                visibility: json!({ "surface": "studio", "member": false }),
                limits: json!({
                    "contractOnly": true,
                    "externalReleaseAllowed": false,
                    "liveProviderDefault": false
                }),
            },
        ],
        job_template_bindings: vec![
            ProductPackJobTemplateBinding {
                key: "surface_brief_refresh".to_string(),
                template_id: "surface.brief.generate".to_string(),
                template_version: 1,
                visibility: json!({ "surface": "studio" }),
                limits: json!({ "maxQueued": 1 }),
            },
            ProductPackJobTemplateBinding {
                key: "video_storyboard_placeholder".to_string(),
                template_id: "studio.promo_video.package".to_string(),
                template_version: 1,
                visibility: json!({ "surface": "studio" }),
                limits: json!({
                    "rendersVideo": false,
                    "externalReleaseAllowed": false,
                    "liveProviderDefault": false
                }),
            },
        ],
        workflow_template_bindings: vec![ProductPackWorkflowTemplateBinding {
            key: "scrollytelling_homepage".to_string(),
            template_id: "studio.story.scrollytelling_homepage".to_string(),
            template_version: 1,
            visibility: json!({ "surface": "studio", "member": false }),
            limits: json!({
                "typedVariables": true,
                "boundedFanout": true,
                "approvalRequired": true,
                "defaultValidationRequiresLiveProviders": false
            }),
        }],
        request_templates: vec![
            ProductPackRequestTemplateBinding {
                key: "founder_intake".to_string(),
                title: "Founder story intake".to_string(),
                capability_id: "surface.brief.generate".to_string(),
                contract: json_schema_object(&[
                    "founderProfile",
                    "businessPositioning",
                    "audience",
                ]),
                visibility: json!({ "studio": true, "member": false }),
                limits: json!({ "storesPrivateInputs": false }),
            },
            ProductPackRequestTemplateBinding {
                key: "refresh_proposal".to_string(),
                title: "Homepage story refresh proposal".to_string(),
                capability_id: "surface.brief.generate".to_string(),
                contract: json_schema_object(&["deckId", "reason"]),
                visibility: json!({ "studio": true, "member": false }),
                limits: json!({ "requiresApproval": true }),
            },
        ],
        artifact_contracts: vec![
            artifact_contract("story_profile", "story.profile"),
            artifact_contract("narrative_deck", "story.narrative_deck"),
            artifact_contract("image_brief", "story.image_brief"),
            artifact_contract(
                "image_provider_request_envelope",
                "story.image_provider_request_envelope",
            ),
            artifact_contract(
                "generated_image_candidate",
                "story.generated_image_candidate",
            ),
            artifact_contract("homepage_version", "story.homepage_version"),
            artifact_contract("refresh_proposal", "story.refresh_proposal"),
            artifact_contract("video_storyboard", "story.video_storyboard"),
        ],
        graph_node_kinds: vec![
            graph_node_contract("story_profile", "story_profile"),
            graph_node_contract("homepage_section", "homepage_section"),
            graph_node_contract("claim", "claim"),
            graph_node_contract("artifact", "artifact"),
            graph_node_contract("pack", "pack"),
        ],
        graph_edge_kinds: vec![
            graph_edge_contract("contains_claim", "CONTAINS_CLAIM"),
            graph_edge_contract("derived_from", "DERIVED_FROM"),
            graph_edge_contract("appears_in", "APPEARS_IN"),
            graph_edge_contract("produced_from_input", "PRODUCED_FROM_INPUT"),
        ],
        projection_surfaces: vec![
            projection_surface("studio_story", "studio.story"),
            projection_surface("growth_story", "growth.story"),
            projection_surface("public_homepage_story", "public.homepage_story"),
            projection_surface("systems_pack_review", "systems.pack_review"),
        ],
        llm_method_bindings: vec![
            llm_method_binding("pack_inspect_manifest", "pack.inspect_manifest"),
            llm_method_binding("workflow_resolve_variables", "workflow.resolveVariables"),
            llm_method_binding(
                "homepage_prepare_image_briefs",
                "homepage.prepare_image_briefs",
            ),
            llm_method_binding("image_review_against_brief", "image.reviewAgainstBrief"),
            llm_method_binding("claim_validate_public_claim", "claim.validate_public_claim"),
            llm_method_binding(
                "graph_get_resource_neighborhood",
                "graph.get_resource_neighborhood",
            ),
        ],
        visibility: json!({
            "owner": true,
            "staff": true,
            "memberSummary": true,
            "publicSurface": false
        }),
        access: json!({ "requiredAccessKind": "owner_or_staff" }),
        growth: json!({
            "evidenceKinds": [
                "content_event",
                "publication_evidence",
                "feedback_response",
                "business_outcome"
            ],
            "recordsAnalyticsTruth": false
        }),
        limits: json!({
            "maxConcurrentRuns": 1,
            "defaultValidationRequiresLiveProviders": false,
            "externalReleaseAllowed": false
        }),
        metadata: json!({
            "status": "internal",
            "coreOwnsTrust": true,
            "packOwnsWorkflow": true,
            "workflowDeclaresOnly": true,
            "deferred": [
                "scrollytelling_runtime",
                "live_image_generation",
                "external_marketplace",
                "publication_automation",
                "external_analytics"
            ]
        }),
    }
}

pub fn install_story_pack_manifest(
    db_path: &Path,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<ProductPackResponse> {
    install_product_pack(
        db_path,
        ProductPackInstallRequest {
            manifest: story_pack_manifest(),
        },
        origin,
        actor_id,
    )
}

pub fn disable_product_pack(
    db_path: &Path,
    pack_id: &str,
    origin: &str,
    actor_id: Option<&str>,
) -> Result<ProductPackResponse> {
    let mut connection = Connection::open(db_path)?;
    require_product_pack(&connection, pack_id)?;
    let transaction = connection.transaction()?;
    let now = Utc::now().to_rfc3339();
    let provenance = product_pack_provenance(pack_id, origin, actor_id);
    transaction.execute(
        "UPDATE product_packs
         SET status = ?2, provenance_json = ?3, updated_at = ?4
         WHERE id = ?1",
        params![
            pack_id,
            PRODUCT_PACK_STATUS_DISABLED,
            provenance.to_string(),
            now
        ],
    )?;
    transaction.execute(
        "UPDATE product_pack_bindings
         SET status = ?2, disabled_at = ?3, updated_at = ?3
         WHERE pack_id = ?1",
        params![pack_id, PRODUCT_PACK_STATUS_DISABLED, now],
    )?;
    append_realtime_event_tx(
        &transaction,
        &system_event(
            "product_pack.disabled",
            json!({
                "packId": pack_id,
                "status": PRODUCT_PACK_STATUS_DISABLED,
            }),
        ),
    )?;
    transaction.commit()?;
    read_product_pack(db_path, pack_id)
}

struct ProductPackRecord {
    id: String,
    name: String,
    version: String,
    status: String,
    manifest: Value,
    validation: Value,
    provenance: Value,
    created_by_actor_id: Option<String>,
    created_at: String,
    updated_at: String,
}

fn validate_product_pack_manifest(
    connection: &Connection,
    manifest: &ProductPackManifest,
) -> Result<Value> {
    require_identifier(&manifest.id, "Product pack id")?;
    require_non_empty(&manifest.name, "Product pack name")?;
    require_identifier(&manifest.version, "Product pack version")?;
    require_manifest_provenance(&manifest.provenance)?;
    reject_hidden_authority(&serde_json::to_value(manifest)?)?;

    let binding_count = manifest.capability_bindings.len()
        + manifest.job_template_bindings.len()
        + manifest.workflow_template_bindings.len()
        + manifest.request_templates.len()
        + manifest.artifact_contracts.len()
        + manifest.graph_node_kinds.len()
        + manifest.graph_edge_kinds.len()
        + manifest.projection_surfaces.len()
        + manifest.llm_method_bindings.len();
    if binding_count == 0 {
        bail!("Product pack manifest requires at least one governed binding");
    }

    let mut keys = BTreeSet::new();
    for binding in &manifest.capability_bindings {
        require_binding_key(&mut keys, "capability", &binding.key)?;
        require_identifier(&binding.capability_id, "Capability binding id")?;
        load_capability(connection, &binding.capability_id)?
            .ok_or_else(|| anyhow::anyhow!("Unknown capability: {}", binding.capability_id))?;
    }
    for binding in &manifest.job_template_bindings {
        require_binding_key(&mut keys, "job_template", &binding.key)?;
        require_identifier(&binding.template_id, "Job template id")?;
        require_builtin_template_version(&binding.template_id, binding.template_version)?;
    }
    for binding in &manifest.workflow_template_bindings {
        require_binding_key(&mut keys, "workflow_template", &binding.key)?;
        require_identifier(&binding.template_id, "Workflow template id")?;
        load_workflow_template(connection, &binding.template_id, binding.template_version)?;
    }
    for binding in &manifest.request_templates {
        require_binding_key(&mut keys, "request_template", &binding.key)?;
        require_non_empty(&binding.title, "Request template title")?;
        require_identifier(&binding.capability_id, "Request template capability id")?;
        load_capability(connection, &binding.capability_id)?
            .ok_or_else(|| anyhow::anyhow!("Unknown capability: {}", binding.capability_id))?;
        validate_contract(&binding.contract, "request template contract")?;
    }
    for binding in &manifest.artifact_contracts {
        require_binding_key(&mut keys, "artifact_contract", &binding.key)?;
        require_identifier(&binding.artifact_kind, "Artifact kind")?;
        validate_contract(&binding.contract, "artifact contract")?;
    }
    for binding in &manifest.graph_node_kinds {
        require_binding_key(&mut keys, "graph_node_kind", &binding.key)?;
        require_identifier(&binding.node_kind, "Graph node kind")?;
        require_declared_graph_node_kind(&binding.node_kind)?;
        validate_contract(&binding.contract, "graph node contract")?;
    }
    for binding in &manifest.graph_edge_kinds {
        require_binding_key(&mut keys, "graph_edge_kind", &binding.key)?;
        require_identifier(&binding.edge_kind, "Graph edge kind")?;
        require_declared_graph_edge_kind(&binding.edge_kind)?;
        validate_contract(&binding.contract, "graph edge contract")?;
    }
    for binding in &manifest.projection_surfaces {
        require_binding_key(&mut keys, "projection_surface", &binding.key)?;
        require_identifier(&binding.surface, "Projection surface")?;
        validate_contract(&binding.contract, "projection surface contract")?;
    }
    for binding in &manifest.llm_method_bindings {
        require_binding_key(&mut keys, "llm_method", &binding.key)?;
        require_method_name(&binding.method_name)?;
        require_llm_method_contract(connection, &binding.method_name)?;
        validate_contract(&binding.contract, "LLM method binding contract")?;
    }

    Ok(json!({
        "status": "accepted",
        "bindingCount": binding_count,
        "capabilityBindingCount": manifest.capability_bindings.len(),
        "jobTemplateBindingCount": manifest.job_template_bindings.len(),
        "workflowTemplateBindingCount": manifest.workflow_template_bindings.len(),
        "requestTemplateCount": manifest.request_templates.len(),
        "artifactContractCount": manifest.artifact_contracts.len(),
        "graphNodeKindCount": manifest.graph_node_kinds.len(),
        "graphEdgeKindCount": manifest.graph_edge_kinds.len(),
        "projectionSurfaceCount": manifest.projection_surfaces.len(),
        "llmMethodBindingCount": manifest.llm_method_bindings.len(),
        "unsupportedBehavior": [
            "pack_execution",
            "mcp_export_grants",
            "marketplace",
            "billing",
            "remote_registry",
            "benefit_grants",
            "public_publishing",
            "live_provider_default"
        ],
    }))
}

fn insert_manifest_bindings(
    transaction: &rusqlite::Transaction<'_>,
    manifest: &ProductPackManifest,
    now: &str,
) -> Result<()> {
    for binding in &manifest.capability_bindings {
        insert_binding(
            transaction,
            manifest,
            BindingInsert {
                kind: "capability",
                key: &binding.key,
                capability_id: Some(&binding.capability_id),
                template_id: None,
                template_version: None,
                artifact_kind: None,
                contract: &json!({}),
                visibility: &binding.visibility,
                limits: &binding.limits,
            },
            now,
        )?;
    }
    for binding in &manifest.job_template_bindings {
        insert_binding(
            transaction,
            manifest,
            BindingInsert {
                kind: "job_template",
                key: &binding.key,
                capability_id: None,
                template_id: Some(&binding.template_id),
                template_version: Some(binding.template_version),
                artifact_kind: None,
                contract: &json!({}),
                visibility: &binding.visibility,
                limits: &binding.limits,
            },
            now,
        )?;
    }
    for binding in &manifest.workflow_template_bindings {
        insert_binding(
            transaction,
            manifest,
            BindingInsert {
                kind: "workflow_template",
                key: &binding.key,
                capability_id: None,
                template_id: None,
                template_version: None,
                artifact_kind: Some(&binding.template_id),
                contract: &json!({}),
                visibility: &binding.visibility,
                limits: &binding.limits,
            },
            now,
        )?;
    }
    for binding in &manifest.request_templates {
        insert_binding(
            transaction,
            manifest,
            BindingInsert {
                kind: "request_template",
                key: &binding.key,
                capability_id: Some(&binding.capability_id),
                template_id: None,
                template_version: None,
                artifact_kind: None,
                contract: &binding.contract,
                visibility: &binding.visibility,
                limits: &binding.limits,
            },
            now,
        )?;
    }
    for binding in &manifest.artifact_contracts {
        insert_binding(
            transaction,
            manifest,
            BindingInsert {
                kind: "artifact_contract",
                key: &binding.key,
                capability_id: None,
                template_id: None,
                template_version: None,
                artifact_kind: Some(&binding.artifact_kind),
                contract: &binding.contract,
                visibility: &binding.visibility,
                limits: &binding.limits,
            },
            now,
        )?;
    }
    for binding in &manifest.graph_node_kinds {
        insert_binding(
            transaction,
            manifest,
            BindingInsert {
                kind: "graph_node_kind",
                key: &binding.key,
                capability_id: None,
                template_id: None,
                template_version: None,
                artifact_kind: Some(&binding.node_kind),
                contract: &binding.contract,
                visibility: &binding.visibility,
                limits: &binding.limits,
            },
            now,
        )?;
    }
    for binding in &manifest.graph_edge_kinds {
        insert_binding(
            transaction,
            manifest,
            BindingInsert {
                kind: "graph_edge_kind",
                key: &binding.key,
                capability_id: None,
                template_id: None,
                template_version: None,
                artifact_kind: Some(&binding.edge_kind),
                contract: &binding.contract,
                visibility: &binding.visibility,
                limits: &binding.limits,
            },
            now,
        )?;
    }
    for binding in &manifest.projection_surfaces {
        insert_binding(
            transaction,
            manifest,
            BindingInsert {
                kind: "projection_surface",
                key: &binding.key,
                capability_id: None,
                template_id: None,
                template_version: None,
                artifact_kind: Some(&binding.surface),
                contract: &binding.contract,
                visibility: &binding.visibility,
                limits: &binding.limits,
            },
            now,
        )?;
    }
    for binding in &manifest.llm_method_bindings {
        insert_binding(
            transaction,
            manifest,
            BindingInsert {
                kind: "llm_method",
                key: &binding.key,
                capability_id: None,
                template_id: None,
                template_version: None,
                artifact_kind: Some(&binding.method_name),
                contract: &binding.contract,
                visibility: &binding.visibility,
                limits: &binding.limits,
            },
            now,
        )?;
    }
    Ok(())
}

struct BindingInsert<'a> {
    kind: &'a str,
    key: &'a str,
    capability_id: Option<&'a str>,
    template_id: Option<&'a str>,
    template_version: Option<i64>,
    artifact_kind: Option<&'a str>,
    contract: &'a Value,
    visibility: &'a Value,
    limits: &'a Value,
}

fn insert_binding(
    transaction: &rusqlite::Transaction<'_>,
    manifest: &ProductPackManifest,
    binding: BindingInsert<'_>,
    now: &str,
) -> Result<()> {
    transaction.execute(
        "INSERT INTO product_pack_bindings (
            id, pack_id, binding_kind, binding_key, capability_id, template_id,
            template_version, artifact_kind, contract_json, visibility_json, access_json,
            growth_json, limits_json, status, disabled_at, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, NULL, ?15, ?15)",
        params![
            format!("{}:{}:{}", manifest.id, binding.kind, binding.key),
            manifest.id,
            binding.kind,
            binding.key,
            binding.capability_id,
            binding.template_id,
            binding.template_version,
            binding.artifact_kind,
            binding.contract.to_string(),
            binding.visibility.to_string(),
            manifest.access.to_string(),
            manifest.growth.to_string(),
            binding.limits.to_string(),
            PRODUCT_PACK_STATUS_ENABLED,
            now,
        ],
    )?;
    Ok(())
}

fn validate_contract(contract: &Value, label: &str) -> Result<()> {
    if contract.is_null() || contract == &json!({}) {
        return Ok(());
    }
    validate_json_schema_document(contract, label)
}

fn require_llm_method_contract(connection: &Connection, method_name: &str) -> Result<()> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM llm_method_contracts
             WHERE name = ?1 AND execution_status = 'contract_only'
             LIMIT 1",
            [method_name],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if exists {
        Ok(())
    } else {
        bail!("Unknown LLM method contract: {method_name}")
    }
}

fn require_manifest_provenance(provenance: &Value) -> Result<()> {
    if provenance
        .as_object()
        .is_some_and(|object| !object.is_empty())
    {
        Ok(())
    } else {
        bail!("Product pack manifest requires provenance metadata")
    }
}

fn require_binding_key(keys: &mut BTreeSet<String>, kind: &str, key: &str) -> Result<()> {
    require_identifier(key, "Binding key")?;
    let composite = format!("{kind}:{key}");
    if !keys.insert(composite) {
        bail!("Duplicate product pack binding key: {kind}:{key}");
    }
    Ok(())
}

fn reject_hidden_authority(value: &Value) -> Result<()> {
    reject_hidden_authority_at(value, "manifest")
}

fn reject_hidden_authority_at(value: &Value, path: &str) -> Result<()> {
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                if is_hidden_authority_key(key) {
                    bail!("unsupported hidden-authority field in {path}: {key}");
                }
                reject_hidden_authority_at(nested, path)?;
            }
        }
        Value::Array(values) => {
            for nested in values {
                reject_hidden_authority_at(nested, path)?;
            }
        }
        Value::String(text) => {
            if looks_like_executable_reference(text) {
                bail!("unsupported executable reference in {path}");
            }
        }
        _ => {}
    }
    Ok(())
}

fn is_hidden_authority_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    [
        "secret",
        "password",
        "token",
        "apikey",
        "providerinternal",
        "promptinternal",
        "rawpolicy",
        "staffrouting",
        "mcpexport",
        "packexecution",
        "executable",
        "command",
        "shell",
        "oauth",
        "payment",
        "marketplace",
        "remoteregistry",
        "arbitraryexecution",
        "externalpublishing",
        "externalpublish",
        "publicpublishing",
        "provideregress",
        "wipereset",
        "rewardaccessmutation",
        "benefitgrant",
        "rewardledger",
        "accessgrant",
        "hostedtimeextension",
    ]
    .iter()
    .any(|fragment| normalized.contains(fragment))
}

fn looks_like_executable_reference(text: &str) -> bool {
    let normalized = text.to_ascii_lowercase();
    normalized.starts_with("file://")
        || normalized.starts_with("./")
        || normalized.starts_with("../")
        || normalized.ends_with(".sh")
        || normalized.ends_with(".exe")
        || normalized.contains("/bin/")
}

fn require_identifier(value: &str, label: &str) -> Result<()> {
    let normalized = require_non_empty(value, label)?;
    if normalized
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        Ok(())
    } else {
        bail!("{label} may only contain ASCII letters, numbers, dots, underscores, or hyphens")
    }
}

fn require_method_name(value: &str) -> Result<()> {
    let normalized = require_non_empty(value, "LLM method name")?;
    let Some((family, method)) = normalized.split_once('.') else {
        bail!("LLM method name must use product-shaped family.method form");
    };
    require_identifier(family, "LLM method family")?;
    require_identifier(method, "LLM method method")?;
    if matches!(
        normalized.as_str(),
        "query_sql" | "search_database" | "get_context" | "run_tool" | "update_record"
    ) || method.eq_ignore_ascii_case("run_tool")
    {
        bail!("LLM method name must not grant generic authority");
    }
    Ok(())
}

fn require_declared_graph_node_kind(node_kind: &str) -> Result<()> {
    if declared_graph_node_kinds().contains(&node_kind) {
        Ok(())
    } else {
        bail!("Unknown graph node kind: {node_kind}")
    }
}

fn require_declared_graph_edge_kind(edge_kind: &str) -> Result<()> {
    if declared_graph_edge_kinds().contains(&edge_kind) {
        Ok(())
    } else {
        bail!("Unknown graph edge kind: {edge_kind}")
    }
}

fn declared_graph_node_kinds() -> &'static [&'static str] {
    &[
        "actor",
        "connection",
        "conversation",
        "conversation_message",
        "visitor_session",
        "tracked_entry_point",
        "offer",
        "offer_acceptance",
        "trial",
        "request",
        "handoff",
        "artifact",
        "job",
        "job_task",
        "event",
        "claim",
        "homepage_section",
        "story_profile",
        "reward_program",
        "reward_event",
        "benefit_grant",
        "business_outcome",
        "pack",
        "capability",
        "corpus_item",
    ]
}

fn declared_graph_edge_kinds() -> &'static [&'static str] {
    &[
        "AUTHORED",
        "MENTIONS",
        "SUPPORTS",
        "CONTRADICTS",
        "DERIVED_FROM",
        "PRODUCED",
        "USES",
        "REQUESTED",
        "APPROVED",
        "REJECTED",
        "GRANTED",
        "REVOKED",
        "REFERRED",
        "ATTRIBUTED_TO",
        "ACCEPTED",
        "TRIGGERED",
        "HANDED_OFF_TO",
        "REQUIRES",
        "DEPENDS_ON",
        "INSTALLED",
        "EMITTED",
        "APPEARS_IN",
        "CONTAINS_CLAIM",
        "PUBLISHED_TO",
        "INFLUENCED",
        "REVISED_BY_FEEDBACK",
        "PRODUCED_FROM_INPUT",
    ]
}

fn require_non_empty(value: &str, label: &str) -> Result<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        bail!("{label} is required");
    }
    Ok(normalized)
}

fn json_schema_object(required_fields: &[&str]) -> Value {
    let mut properties = serde_json::Map::new();
    for field in required_fields {
        properties.insert(
            (*field).to_string(),
            json!({ "type": "string", "minLength": 1 }),
        );
    }
    json!({
        "type": "object",
        "required": required_fields,
        "additionalProperties": false,
        "properties": properties
    })
}

fn artifact_contract(key: &str, artifact_kind: &str) -> ProductPackArtifactContractBinding {
    ProductPackArtifactContractBinding {
        key: key.to_string(),
        artifact_kind: artifact_kind.to_string(),
        contract: json_schema_object(&["artifactId", "evidenceRef"]),
        visibility: json!({
            "studio": true,
            "member": false,
            "public": false,
            "publicDerivativeRequiresApproval": true
        }),
        limits: json!({
            "defaultValidationRequiresLiveProviders": false,
            "storesPrivateTextInSummary": false
        }),
    }
}

fn graph_node_contract(key: &str, node_kind: &str) -> ProductPackGraphNodeKindBinding {
    ProductPackGraphNodeKindBinding {
        key: key.to_string(),
        node_kind: node_kind.to_string(),
        contract: json_schema_object(&["resourceKind", "resourceId", "evidenceRef"]),
        visibility: json!({ "maxCeiling": "staff", "publicRequiresApproval": true }),
        limits: json!({ "candidateFirst": true, "confirmedRequiresApproval": true }),
    }
}

fn graph_edge_contract(key: &str, edge_kind: &str) -> ProductPackGraphEdgeKindBinding {
    ProductPackGraphEdgeKindBinding {
        key: key.to_string(),
        edge_kind: edge_kind.to_string(),
        contract: json_schema_object(&["sourceRef", "targetRef", "evidenceRef"]),
        visibility: json!({ "maxCeiling": "staff", "publicRequiresApproval": true }),
        limits: json!({ "candidateFirst": true, "confirmedRequiresApproval": true }),
    }
}

fn projection_surface(key: &str, surface: &str) -> ProductPackProjectionSurfaceBinding {
    ProductPackProjectionSurfaceBinding {
        key: key.to_string(),
        surface: surface.to_string(),
        contract: json_schema_object(&["surfaceId", "evidenceRef"]),
        visibility: json!({
            "roleSafe": true,
            "memberSummary": surface.starts_with("public.") || surface.starts_with("growth.")
        }),
        limits: json!({
            "redactsRoutingDetails": true,
            "redactsProviderDetails": true,
            "redactsPromptDetails": true,
            "omitsGraphCertainty": true
        }),
    }
}

fn llm_method_binding(key: &str, method_name: &str) -> ProductPackLlmMethodBinding {
    ProductPackLlmMethodBinding {
        key: key.to_string(),
        method_name: method_name.to_string(),
        contract: json_schema_object(&["evidenceRef"]),
        visibility: json!({ "maxCeiling": "staff", "memberSafeOutputRequired": true }),
        limits: json!({
            "readOnly": true,
            "defaultValidationRequiresLiveProviders": false,
            "requiresEvidenceRefs": true,
            "requiresLimitations": true
        }),
    }
}

fn require_product_pack(connection: &Connection, pack_id: &str) -> Result<ProductPackRecord> {
    connection
        .query_row(
            "SELECT id, name, version, status, manifest_json, validation_json, provenance_json,
                    created_by_actor_id, created_at, updated_at
             FROM product_packs WHERE id = ?1",
            [pack_id],
            product_pack_from_row,
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("Product pack was not found: {pack_id}"))
}

fn product_pack_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProductPackRecord> {
    let manifest_json: String = row.get(4)?;
    let validation_json: String = row.get(5)?;
    let provenance_json: String = row.get(6)?;
    Ok(ProductPackRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        version: row.get(2)?,
        status: row.get(3)?,
        manifest: serde_json::from_str(&manifest_json).unwrap_or_else(|_| json!({})),
        validation: serde_json::from_str(&validation_json).unwrap_or_else(|_| json!({})),
        provenance: serde_json::from_str(&provenance_json).unwrap_or_else(|_| json!({})),
        created_by_actor_id: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn load_pack_bindings(
    connection: &Connection,
    pack_id: &str,
) -> Result<Vec<ProductPackBindingView>> {
    connection.query_many(
        "SELECT id, pack_id, binding_kind, binding_key, capability_id, template_id,
                template_version, artifact_kind, contract_json, visibility_json, access_json,
                growth_json, limits_json, status, disabled_at, created_at, updated_at
         FROM product_pack_bindings WHERE pack_id = ?1 ORDER BY binding_kind ASC, binding_key ASC",
        [pack_id],
        product_pack_binding_from_row,
    )
}

fn product_pack_binding_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProductPackBindingView> {
    let contract_json: String = row.get(8)?;
    let visibility_json: String = row.get(9)?;
    let access_json: String = row.get(10)?;
    let growth_json: String = row.get(11)?;
    let limits_json: String = row.get(12)?;
    Ok(ProductPackBindingView {
        id: row.get(0)?,
        pack_id: row.get(1)?,
        binding_kind: row.get(2)?,
        binding_key: row.get(3)?,
        capability_id: row.get(4)?,
        template_id: row.get(5)?,
        template_version: row.get(6)?,
        artifact_kind: row.get(7)?,
        contract: serde_json::from_str(&contract_json).unwrap_or_else(|_| json!({})),
        visibility: serde_json::from_str(&visibility_json).unwrap_or_else(|_| json!({})),
        access: serde_json::from_str(&access_json).unwrap_or_else(|_| json!({})),
        growth: serde_json::from_str(&growth_json).unwrap_or_else(|_| json!({})),
        limits: serde_json::from_str(&limits_json).unwrap_or_else(|_| json!({})),
        status: row.get(13)?,
        disabled_at: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

impl ProductPackRecord {
    fn into_view(self, bindings: Vec<ProductPackBindingView>) -> ProductPackView {
        ProductPackView {
            id: self.id,
            name: self.name,
            version: self.version,
            status: self.status,
            manifest: self.manifest,
            validation: self.validation,
            provenance: self.provenance,
            bindings,
            created_by_actor_id: self.created_by_actor_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn product_pack_provenance(pack_id: &str, origin: &str, actor_id: Option<&str>) -> Value {
    provenance_metadata(
        actor_context_for_origin(origin, actor_id),
        PolicyAction::Validate,
        ResourceRef::new(ResourceKind::ProductPack, pack_id),
        Some("product_packs.write"),
        ResourceClassification::local_operations_ready_for_review(),
    )
}

fn actor_context_for_origin(origin: &str, actor_id: Option<&str>) -> ActorContext {
    let kind = match origin {
        "mcp" => ActorKind::McpClient,
        "scheduler" => ActorKind::Scheduler,
        "system" => ActorKind::System,
        _ => ActorKind::BrowserOperator,
    };
    ActorContext::new(kind, origin, actor_id.map(ToString::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::LOCAL_OWNER_ACTOR_ID;
    use rusqlite::Connection;
    use serde_json::json;

    fn valid_manifest() -> ProductPackManifest {
        ProductPackManifest {
            id: "product_pack.nyc.promo_ops".to_string(),
            name: "NYC Promo Ops".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Governed pilot work for offer follow-up.".to_string()),
            provenance: json!({
                "source": "local_fixture",
                "reviewedBy": "owner",
                "reason": "TDD fixture for product pack manifest spine"
            }),
            capability_bindings: vec![ProductPackCapabilityBinding {
                key: "system_status".to_string(),
                capability_id: "system.status.read".to_string(),
                visibility: json!({ "surface": "systems", "member": false }),
                limits: json!({ "maxRunsPerDay": 4 }),
            }],
            job_template_bindings: vec![ProductPackJobTemplateBinding {
                key: "health_check".to_string(),
                template_id: "system.health.check".to_string(),
                template_version: 1,
                visibility: json!({ "surface": "systems" }),
                limits: json!({ "maxQueued": 1 }),
            }],
            workflow_template_bindings: vec![],
            request_templates: vec![ProductPackRequestTemplateBinding {
                key: "strategy_request".to_string(),
                title: "Strategy session request".to_string(),
                capability_id: "system.status.read".to_string(),
                contract: json!({
                    "type": "object",
                    "required": ["summary"],
                    "additionalProperties": false,
                    "properties": { "summary": { "type": "string" } }
                }),
                visibility: json!({ "member": true, "staff": true }),
                limits: json!({ "maxOpen": 1 }),
            }],
            artifact_contracts: vec![ProductPackArtifactContractBinding {
                key: "promo_brief".to_string(),
                artifact_kind: "promo_brief".to_string(),
                contract: json!({
                    "type": "object",
                    "required": ["title"],
                    "additionalProperties": false,
                    "properties": { "title": { "type": "string" } }
                }),
                visibility: json!({ "studio": true, "member": false }),
                limits: json!({ "maxVersions": 3 }),
            }],
            graph_node_kinds: vec![],
            graph_edge_kinds: vec![],
            projection_surfaces: vec![],
            llm_method_bindings: vec![],
            visibility: json!({ "owner": true, "memberSummary": true }),
            access: json!({ "requiredAccessKind": "hosted_trial" }),
            growth: json!({ "evidenceKinds": ["offer_acceptance", "feedback_review"] }),
            limits: json!({ "maxConcurrentRuns": 2 }),
            metadata: json!({ "deferred": ["pack_execution", "marketplace", "billing"] }),
        }
    }

    fn setup_db() -> (tempfile::TempDir, std::path::PathBuf) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        crate::schema::init_database(&db_path).unwrap();
        (temp_dir, db_path)
    }

    fn count_rows(connection: &Connection, table_name: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table_name}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    #[test]
    fn valid_manifest_installs_lists_reads_and_disables_pack() {
        let (_temp_dir, db_path) = setup_db();

        let installed = install_product_pack(
            &db_path,
            ProductPackInstallRequest {
                manifest: valid_manifest(),
            },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        assert_eq!(installed.pack.id, "product_pack.nyc.promo_ops");
        assert_eq!(installed.pack.status, PRODUCT_PACK_STATUS_ENABLED);
        assert_eq!(installed.pack.bindings.len(), 4);
        assert_eq!(
            installed.pack.provenance["resource"]["kind"],
            "product_pack"
        );
        assert_eq!(installed.pack.validation["status"], "accepted");

        let listed = list_product_packs(&db_path).unwrap();
        assert_eq!(listed.packs.len(), 1);
        assert_eq!(listed.packs[0].bindings.len(), 4);

        let read = read_product_pack(&db_path, "product_pack.nyc.promo_ops").unwrap();
        assert_eq!(read.pack.manifest["id"], "product_pack.nyc.promo_ops");

        let disabled =
            disable_product_pack(&db_path, "product_pack.nyc.promo_ops", "test", None).unwrap();
        assert_eq!(disabled.pack.status, PRODUCT_PACK_STATUS_DISABLED);
        assert!(disabled
            .pack
            .bindings
            .iter()
            .all(|binding| binding.status == PRODUCT_PACK_STATUS_DISABLED));

        let connection = Connection::open(&db_path).unwrap();
        assert_eq!(count_rows(&connection, "product_pack_versions"), 1);
        assert_eq!(count_rows(&connection, "product_pack_bindings"), 4);
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM realtime_events WHERE event_type IN ('product_pack.installed', 'product_pack.disabled')",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            2
        );
    }

    #[test]
    fn rejects_unknown_capability_and_template_without_partial_install() {
        let (_temp_dir, db_path) = setup_db();
        let mut unknown_capability = valid_manifest();
        unknown_capability.capability_bindings[0].capability_id = "shell.run".to_string();

        let error = install_product_pack(
            &db_path,
            ProductPackInstallRequest {
                manifest: unknown_capability,
            },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap_err();
        assert!(error.to_string().contains("Unknown capability"));

        let mut unknown_template = valid_manifest();
        unknown_template.job_template_bindings[0].template_id = "shell.run".to_string();
        let error = install_product_pack(
            &db_path,
            ProductPackInstallRequest {
                manifest: unknown_template,
            },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("Unknown built-in process template"));

        let connection = Connection::open(&db_path).unwrap();
        assert_eq!(count_rows(&connection, "product_packs"), 0);
        assert_eq!(count_rows(&connection, "product_pack_bindings"), 0);
    }

    #[test]
    fn rejects_hidden_authority_and_secret_shaped_manifest_without_leaking_secret() {
        let (_temp_dir, db_path) = setup_db();
        let mut manifest = valid_manifest();
        manifest.metadata = json!({
            "providerSecret": "sk-do-not-leak",
            "mcpExportGrant": true,
            "packExecution": "enabled"
        });

        let error = install_product_pack(
            &db_path,
            ProductPackInstallRequest { manifest },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("unsupported hidden-authority field"));
        assert!(!message.contains("sk-do-not-leak"));

        let connection = Connection::open(&db_path).unwrap();
        assert_eq!(count_rows(&connection, "product_packs"), 0);
    }

    #[test]
    fn reinstalling_same_version_is_idempotent_and_preserves_version_evidence() {
        let (_temp_dir, db_path) = setup_db();
        let request = ProductPackInstallRequest {
            manifest: valid_manifest(),
        };
        install_product_pack(
            &db_path,
            request.clone(),
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        install_product_pack(&db_path, request, "test", Some(LOCAL_OWNER_ACTOR_ID)).unwrap();

        let connection = Connection::open(&db_path).unwrap();
        assert_eq!(count_rows(&connection, "product_packs"), 1);
        assert_eq!(count_rows(&connection, "product_pack_versions"), 1);
        assert_eq!(count_rows(&connection, "product_pack_bindings"), 4);
    }

    #[test]
    fn member_summary_withholds_owner_validation_and_manifest_internals() {
        let (_temp_dir, db_path) = setup_db();
        install_product_pack(
            &db_path,
            ProductPackInstallRequest {
                manifest: valid_manifest(),
            },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();

        let summary =
            read_product_pack_member_summary(&db_path, "product_pack.nyc.promo_ops").unwrap();
        let json = serde_json::to_string(&summary).unwrap();
        assert_eq!(summary.binding_count, 4);
        assert!(summary
            .available_binding_kinds
            .contains(&"capability".to_string()));
        assert!(!json.contains("manifest"));
        assert!(!json.contains("validation"));
        assert!(!json.contains("provenance"));
        assert!(!json.contains("provider"));
        assert!(!json.contains("prompt"));
        assert!(!json.contains("secret"));
        assert!(!json.contains("rawPolicy"));
        assert!(!json.contains("staffRouting"));
    }

    #[test]
    fn story_pack_manifest_installs_declared_workflow_without_hidden_authority() {
        let (_temp_dir, db_path) = setup_db();

        let installed =
            install_story_pack_manifest(&db_path, "test", Some(LOCAL_OWNER_ACTOR_ID)).unwrap();

        assert_eq!(installed.pack.id, "studio.story");
        assert_eq!(installed.pack.status, PRODUCT_PACK_STATUS_ENABLED);
        assert_eq!(
            installed.pack.validation["workflowTemplateBindingCount"],
            json!(1)
        );
        assert_eq!(installed.pack.validation["graphNodeKindCount"], json!(5));
        assert_eq!(installed.pack.validation["graphEdgeKindCount"], json!(4));
        assert_eq!(
            installed.pack.validation["projectionSurfaceCount"],
            json!(4)
        );
        assert_eq!(installed.pack.validation["llmMethodBindingCount"], json!(6));
        assert_eq!(
            installed.pack.manifest["metadata"]["workflowDeclaresOnly"],
            json!(true)
        );
        assert_eq!(
            installed.pack.manifest["limits"]["defaultValidationRequiresLiveProviders"],
            json!(false)
        );
        assert!(installed.pack.bindings.iter().any(|binding| {
            binding.binding_kind == "workflow_template"
                && binding.artifact_kind.as_deref() == Some("studio.story.scrollytelling_homepage")
        }));
        assert!(installed.pack.bindings.iter().any(|binding| {
            binding.binding_kind == "artifact_contract"
                && binding.artifact_kind.as_deref() == Some("story.image_brief")
        }));
        assert!(installed.pack.bindings.iter().any(|binding| {
            binding.binding_kind == "llm_method"
                && binding.artifact_kind.as_deref() == Some("homepage.prepare_image_briefs")
        }));
        assert!(installed.pack.bindings.iter().any(|binding| {
            binding.binding_kind == "projection_surface"
                && binding.artifact_kind.as_deref() == Some("public.homepage_story")
        }));

        let manifest_json = installed.pack.manifest.to_string();
        assert!(!manifest_json.contains("providerSecret"));
        assert!(!manifest_json.contains("promptInternal"));
        assert!(!manifest_json.contains("rawPolicy"));
        assert!(!manifest_json.contains("staffRouting"));
        assert!(!manifest_json.contains("accessGrant"));
        assert!(!manifest_json.contains("rewardLedger"));
    }

    #[test]
    fn story_pack_rejects_undeclared_method_and_preserves_member_safe_summary() {
        let (_temp_dir, db_path) = setup_db();
        let mut manifest = story_pack_manifest();
        manifest.llm_method_bindings[0].method_name = "run_tool".to_string();

        let error = install_product_pack(
            &db_path,
            ProductPackInstallRequest { manifest },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap_err();
        assert!(error.to_string().contains("LLM method name"));

        let mut unsafe_manifest = story_pack_manifest();
        unsafe_manifest.metadata = json!({
            "publicPublishing": true,
            "providerEgress": "unreviewed",
            "wipeReset": true,
            "arbitraryExecution": true
        });
        let error = install_product_pack(
            &db_path,
            ProductPackInstallRequest {
                manifest: unsafe_manifest,
            },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("unsupported hidden-authority field"));

        let mut unknown_graph_node = story_pack_manifest();
        unknown_graph_node.graph_node_kinds[0].node_kind = "provider_internal".to_string();
        let error = install_product_pack(
            &db_path,
            ProductPackInstallRequest {
                manifest: unknown_graph_node,
            },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap_err();
        assert!(error.to_string().contains("Unknown graph node kind"));

        let mut unknown_graph_edge = story_pack_manifest();
        unknown_graph_edge.graph_edge_kinds[0].edge_kind = "LEAKS_TO".to_string();
        let error = install_product_pack(
            &db_path,
            ProductPackInstallRequest {
                manifest: unknown_graph_edge,
            },
            "test",
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap_err();
        assert!(error.to_string().contains("Unknown graph edge kind"));

        install_story_pack_manifest(&db_path, "test", Some(LOCAL_OWNER_ACTOR_ID)).unwrap();
        let summary = read_product_pack_member_summary(&db_path, "studio.story").unwrap();
        let summary_json = serde_json::to_string(&summary).unwrap();
        assert_eq!(summary.id, "studio.story");
        assert!(summary
            .available_binding_kinds
            .contains(&"workflow_template".to_string()));
        assert!(summary
            .available_binding_kinds
            .contains(&"llm_method".to_string()));
        assert!(!summary_json.contains("validation"));
        assert!(!summary_json.contains("manifest"));
        assert!(!summary_json.contains("provider"));
        assert!(!summary_json.contains("prompt"));
        assert!(!summary_json.contains("graph certainty"));
        assert!(!summary_json.contains("private artifact"));
    }
}
