use anyhow::{ensure, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::conversation_analysis::{load_job, ConversationAnalysisJobView};
use crate::conversations::{append_conversation_event, ConversationMessageView};
use crate::events::RealtimeEvent;

pub const KNOWLEDGE_GRAPH_SCHEMA_VERSION: &str = "knowledge_graph.candidates.v1";
pub const CONFIRMED_GRAPH_SCHEMA_VERSION: &str = "graph.confirmed.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeGraphCandidateTarget {
    Node,
    Edge,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGraphNodeCandidateView {
    pub id: String,
    pub job_id: String,
    pub conversation_id: String,
    pub segment_id: Option<String>,
    pub source_analysis_candidate_id: Option<String>,
    pub node_kind: String,
    pub label: String,
    pub candidate_state: String,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub source_event_refs: Vec<String>,
    pub content_hash: String,
    pub visibility: String,
    pub created_at: String,
    pub updated_at: String,
    pub state_changed_at: Option<String>,
    pub state_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGraphEdgeCandidateView {
    pub id: String,
    pub job_id: String,
    pub conversation_id: String,
    pub segment_id: Option<String>,
    pub source_analysis_candidate_id: Option<String>,
    pub source_node_candidate_id: String,
    pub target_node_candidate_id: String,
    pub relationship_kind: String,
    pub label: String,
    pub candidate_state: String,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub source_event_refs: Vec<String>,
    pub content_hash: String,
    pub visibility: String,
    pub created_at: String,
    pub updated_at: String,
    pub state_changed_at: Option<String>,
    pub state_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGraphCandidateList {
    pub nodes: Vec<KnowledgeGraphNodeCandidateView>,
    pub edges: Vec<KnowledgeGraphEdgeCandidateView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeView {
    pub id: String,
    pub node_kind: String,
    pub resource_kind: String,
    pub resource_id: String,
    pub label: String,
    pub status: String,
    pub visibility_ceiling: String,
    pub content_hash: String,
    pub provenance: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdgeView {
    pub id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub relationship_kind: String,
    pub status: String,
    pub confidence: f64,
    pub visibility_ceiling: String,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub content_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNeighborhood {
    pub nodes: Vec<GraphNodeView>,
    pub edges: Vec<GraphEdgeView>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphPromotionOutcome {
    pub promotion_id: String,
    pub node: Option<GraphNodeView>,
    pub edge: Option<GraphEdgeView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphViewer {
    pub actor_id: Option<String>,
    pub visibility: String,
}

#[derive(Debug, Clone)]
pub struct KnowledgeGraphNodeCandidateInput {
    pub source_analysis_candidate_id: Option<String>,
    pub node_kind: String,
    pub label: String,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub source_event_refs: Vec<String>,
    pub visibility: String,
}

#[derive(Debug, Clone)]
pub struct KnowledgeGraphEdgeCandidateInput {
    pub source_analysis_candidate_id: Option<String>,
    pub source_node_candidate_id: String,
    pub target_node_candidate_id: String,
    pub relationship_kind: String,
    pub label: String,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
    pub provenance: Value,
    pub source_event_refs: Vec<String>,
    pub visibility: String,
}

pub fn extract_graph_candidates_for_analysis_job(
    connection: &Connection,
    job_id: &str,
) -> Result<(KnowledgeGraphCandidateList, Vec<RealtimeEvent>)> {
    let job = load_job(connection, job_id)?;
    let message_id = job
        .source_message_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("knowledge graph extraction requires source message"))?;
    let message = load_message(connection, message_id)?;
    let evidence_refs = vec![evidence_ref(&message)];
    let source_event_refs = message
        .event_cursor
        .map(|cursor| vec![format!("realtime_event:{cursor}")])
        .unwrap_or_default();
    let provenance = json!({
        "schemaVersion": KNOWLEDGE_GRAPH_SCHEMA_VERSION,
        "generator": "deterministic.local",
        "analysisJobId": job.id,
        "sourceMessageId": message.id,
        "sourceEventCursor": message.event_cursor,
    });

    let labels = extract_entity_labels(&message.body_markdown);
    let mut events = Vec::new();
    let mut nodes = Vec::new();
    for label in labels {
        let inserted = propose_node_candidate(
            connection,
            &job,
            KnowledgeGraphNodeCandidateInput {
                source_analysis_candidate_id: None,
                node_kind: classify_node_kind(&label).to_string(),
                label,
                confidence: 0.62,
                evidence_refs: evidence_refs.clone(),
                provenance: provenance.clone(),
                source_event_refs: source_event_refs.clone(),
                visibility: "staff_private".to_string(),
            },
        )?;
        if let Some(event) = inserted.event {
            events.push(event);
        }
        nodes.push(inserted.candidate);
    }

    let mut edges = Vec::new();
    if nodes.len() >= 2 {
        let relationship_kind = relationship_kind_for_text(&message.body_markdown);
        let edge = propose_edge_candidate(
            connection,
            &job,
            KnowledgeGraphEdgeCandidateInput {
                source_analysis_candidate_id: None,
                source_node_candidate_id: nodes[0].id.clone(),
                target_node_candidate_id: nodes[1].id.clone(),
                relationship_kind: relationship_kind.to_string(),
                label: relationship_kind.replace('_', " "),
                confidence: 0.58,
                evidence_refs,
                provenance,
                source_event_refs,
                visibility: "staff_private".to_string(),
            },
        )?;
        if let Some(event) = edge.event {
            events.push(event);
        }
        edges.push(edge.candidate);
    }

    Ok((KnowledgeGraphCandidateList { nodes, edges }, events))
}

pub struct ProposedNodeCandidate {
    pub candidate: KnowledgeGraphNodeCandidateView,
    pub event: Option<RealtimeEvent>,
}

pub fn propose_node_candidate(
    connection: &Connection,
    job: &ConversationAnalysisJobView,
    input: KnowledgeGraphNodeCandidateInput,
) -> Result<ProposedNodeCandidate> {
    validate_common_candidate_input(
        &input.label,
        input.confidence,
        &input.evidence_refs,
        &input.provenance,
    )?;
    let label = sanitize_text(&input.label);
    ensure!(!label.trim().is_empty(), "node label is required");
    let provenance = sanitize_json(input.provenance);
    let content_hash = stable_hash(&format!(
        "node|{}|{label}|{}|{}",
        input.node_kind,
        json!(input.evidence_refs),
        provenance
    ));
    let id = stable_candidate_id("knowledge_graph_node_candidate", &content_hash);
    let now = Utc::now().to_rfc3339();
    let inserted = connection.execute(
        "INSERT OR IGNORE INTO knowledge_graph_node_candidates (
            id, job_id, conversation_id, segment_id, source_analysis_candidate_id,
            node_kind, label, candidate_state, confidence, evidence_refs_json,
            provenance_json, source_event_refs_json, content_hash, visibility, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'proposed', ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
        params![
            id,
            job.id,
            job.conversation_id,
            job.segment_id,
            input.source_analysis_candidate_id,
            input.node_kind,
            label,
            input.confidence,
            json!(input.evidence_refs).to_string(),
            provenance.to_string(),
            json!(input.source_event_refs).to_string(),
            content_hash,
            input.visibility,
            now,
        ],
    )? == 1;
    let candidate = load_node_candidate(connection, &id)?;
    let event = if inserted {
        Some(append_conversation_event(
            connection,
            &candidate.conversation_id,
            candidate.segment_id.as_deref(),
            None,
            "knowledge_graph.node_candidate.created",
            json!({
                "candidateId": candidate.id,
                "candidateState": candidate.candidate_state,
                "nodeKind": candidate.node_kind,
                "label": candidate.label,
                "evidenceRefs": candidate.evidence_refs,
                "contentHash": candidate.content_hash,
            }),
            None,
        )?)
    } else {
        None
    };
    Ok(ProposedNodeCandidate { candidate, event })
}

pub struct ProposedEdgeCandidate {
    pub candidate: KnowledgeGraphEdgeCandidateView,
    pub event: Option<RealtimeEvent>,
}

pub fn propose_edge_candidate(
    connection: &Connection,
    job: &ConversationAnalysisJobView,
    input: KnowledgeGraphEdgeCandidateInput,
) -> Result<ProposedEdgeCandidate> {
    validate_common_candidate_input(
        &input.label,
        input.confidence,
        &input.evidence_refs,
        &input.provenance,
    )?;
    ensure!(
        input.source_node_candidate_id != input.target_node_candidate_id,
        "edge candidates require distinct source and target nodes"
    );
    let label = sanitize_text(&input.label);
    let provenance = sanitize_json(input.provenance);
    let content_hash = stable_hash(&format!(
        "edge|{}|{}|{}|{label}|{}|{}",
        input.relationship_kind,
        input.source_node_candidate_id,
        input.target_node_candidate_id,
        json!(input.evidence_refs),
        provenance
    ));
    let id = stable_candidate_id("knowledge_graph_edge_candidate", &content_hash);
    let now = Utc::now().to_rfc3339();
    let inserted = connection.execute(
        "INSERT OR IGNORE INTO knowledge_graph_edge_candidates (
            id, job_id, conversation_id, segment_id, source_analysis_candidate_id,
            source_node_candidate_id, target_node_candidate_id, relationship_kind, label,
            candidate_state, confidence, evidence_refs_json, provenance_json,
            source_event_refs_json, content_hash, visibility, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'proposed', ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)",
        params![
            id,
            job.id,
            job.conversation_id,
            job.segment_id,
            input.source_analysis_candidate_id,
            input.source_node_candidate_id,
            input.target_node_candidate_id,
            input.relationship_kind,
            label,
            input.confidence,
            json!(input.evidence_refs).to_string(),
            provenance.to_string(),
            json!(input.source_event_refs).to_string(),
            content_hash,
            input.visibility,
            now,
        ],
    )? == 1;
    let candidate = load_edge_candidate(connection, &id)?;
    let event = if inserted {
        Some(append_conversation_event(
            connection,
            &candidate.conversation_id,
            candidate.segment_id.as_deref(),
            None,
            "knowledge_graph.edge_candidate.created",
            json!({
                "candidateId": candidate.id,
                "candidateState": candidate.candidate_state,
                "relationshipKind": candidate.relationship_kind,
                "sourceNodeCandidateId": candidate.source_node_candidate_id,
                "targetNodeCandidateId": candidate.target_node_candidate_id,
                "evidenceRefs": candidate.evidence_refs,
                "contentHash": candidate.content_hash,
            }),
            None,
        )?)
    } else {
        None
    };
    Ok(ProposedEdgeCandidate { candidate, event })
}

pub fn transition_graph_candidate(
    connection: &Connection,
    target: KnowledgeGraphCandidateTarget,
    candidate_id: &str,
    new_state: &str,
    reason: &str,
) -> Result<RealtimeEvent> {
    ensure!(
        matches!(new_state, "confirmed" | "rejected" | "superseded"),
        "unsupported graph candidate state"
    );
    ensure!(
        !reason.trim().is_empty(),
        "state transition reason is required"
    );
    let now = Utc::now().to_rfc3339();
    match target {
        KnowledgeGraphCandidateTarget::Node => {
            let candidate = load_node_candidate(connection, candidate_id)?;
            connection.execute(
                "UPDATE knowledge_graph_node_candidates
                 SET candidate_state = ?2, state_changed_at = ?3, state_reason = ?4, updated_at = ?3
                 WHERE id = ?1",
                params![candidate_id, new_state, now, sanitize_text(reason)],
            )?;
            append_state_transition_event(
                connection,
                &candidate.conversation_id,
                candidate.segment_id.as_deref(),
                candidate_id,
                "node",
                new_state,
                reason,
            )
        }
        KnowledgeGraphCandidateTarget::Edge => {
            let candidate = load_edge_candidate(connection, candidate_id)?;
            connection.execute(
                "UPDATE knowledge_graph_edge_candidates
                 SET candidate_state = ?2, state_changed_at = ?3, state_reason = ?4, updated_at = ?3
                 WHERE id = ?1",
                params![candidate_id, new_state, now, sanitize_text(reason)],
            )?;
            append_state_transition_event(
                connection,
                &candidate.conversation_id,
                candidate.segment_id.as_deref(),
                candidate_id,
                "edge",
                new_state,
                reason,
            )
        }
    }
}

pub fn promote_node_candidate(
    connection: &Connection,
    candidate_id: &str,
    actor_id: Option<&str>,
    reason: &str,
) -> Result<GraphPromotionOutcome> {
    ensure!(!reason.trim().is_empty(), "promotion reason is required");
    let candidate = load_node_candidate(connection, candidate_id)?;
    ensure!(
        matches!(candidate.candidate_state.as_str(), "proposed" | "confirmed"),
        "only proposed or confirmed node candidates can be promoted"
    );
    let now = Utc::now().to_rfc3339();
    let node_id = stable_candidate_id(
        "graph_node",
        &stable_hash(&format!(
            "knowledge_graph_node_candidate|{}|{}",
            candidate.id, candidate.content_hash
        )),
    );
    let provenance = json!({
        "schemaVersion": CONFIRMED_GRAPH_SCHEMA_VERSION,
        "source": "knowledge_graph_node_candidate",
        "candidateId": candidate.id,
        "jobId": candidate.job_id,
        "conversationId": candidate.conversation_id,
        "sourceAnalysisCandidateId": candidate.source_analysis_candidate_id,
        "candidateProvenance": candidate.provenance,
    });
    connection.execute(
        "INSERT OR IGNORE INTO graph_nodes (
            id, node_kind, resource_kind, resource_id, label, status, visibility_ceiling,
            content_hash, provenance_json, created_at, updated_at
         ) VALUES (?1, ?2, 'knowledge_graph_node_candidate', ?3, ?4, 'confirmed', ?5, ?6, ?7, ?8, ?8)",
        params![
            node_id,
            candidate.node_kind,
            candidate.id,
            candidate.label,
            normalize_graph_visibility(&candidate.visibility),
            candidate.content_hash,
            provenance.to_string(),
            now,
        ],
    )?;
    connection.execute(
        "UPDATE knowledge_graph_node_candidates
         SET candidate_state = 'confirmed', state_changed_at = COALESCE(state_changed_at, ?2),
             state_reason = COALESCE(state_reason, ?3), updated_at = ?2
         WHERE id = ?1",
        params![candidate.id, now, sanitize_text(reason)],
    )?;
    let promotion_id = record_candidate_promotion(
        connection,
        "node",
        &candidate.id,
        Some(&node_id),
        None,
        actor_id,
        reason,
        &candidate.evidence_refs,
        &provenance,
        &now,
    )?;
    append_promotion_event(
        connection,
        &candidate.conversation_id,
        candidate.segment_id.as_deref(),
        "node",
        &candidate.id,
        Some(&node_id),
        None,
        &candidate.evidence_refs,
    )?;
    Ok(GraphPromotionOutcome {
        promotion_id,
        node: Some(load_graph_node(connection, &node_id)?),
        edge: None,
    })
}

pub fn promote_edge_candidate(
    connection: &Connection,
    candidate_id: &str,
    actor_id: Option<&str>,
    reason: &str,
) -> Result<GraphPromotionOutcome> {
    ensure!(!reason.trim().is_empty(), "promotion reason is required");
    let candidate = load_edge_candidate(connection, candidate_id)?;
    ensure!(
        matches!(candidate.candidate_state.as_str(), "proposed" | "confirmed"),
        "only proposed or confirmed edge candidates can be promoted"
    );
    let source = promote_node_candidate(
        connection,
        &candidate.source_node_candidate_id,
        actor_id,
        "Promoted as edge endpoint.",
    )?
    .node
    .ok_or_else(|| anyhow::anyhow!("source node promotion did not return a graph node"))?;
    let target = promote_node_candidate(
        connection,
        &candidate.target_node_candidate_id,
        actor_id,
        "Promoted as edge endpoint.",
    )?
    .node
    .ok_or_else(|| anyhow::anyhow!("target node promotion did not return a graph node"))?;
    let now = Utc::now().to_rfc3339();
    let edge_hash = stable_hash(&format!(
        "knowledge_graph_edge_candidate|{}|{}|{}|{}",
        candidate.id, source.id, target.id, candidate.content_hash
    ));
    let edge_id = stable_candidate_id("graph_edge", &edge_hash);
    let provenance = json!({
        "schemaVersion": CONFIRMED_GRAPH_SCHEMA_VERSION,
        "source": "knowledge_graph_edge_candidate",
        "candidateId": candidate.id,
        "jobId": candidate.job_id,
        "conversationId": candidate.conversation_id,
        "sourceNodeCandidateId": candidate.source_node_candidate_id,
        "targetNodeCandidateId": candidate.target_node_candidate_id,
        "candidateProvenance": candidate.provenance,
    });
    connection.execute(
        "INSERT OR IGNORE INTO graph_edges (
            id, source_node_id, target_node_id, relationship_kind, status, confidence,
            visibility_ceiling, evidence_refs_json, provenance_json, content_hash,
            created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, 'confirmed', ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![
            edge_id,
            source.id,
            target.id,
            candidate.relationship_kind,
            candidate.confidence,
            normalize_graph_visibility(&candidate.visibility),
            json!(candidate.evidence_refs).to_string(),
            provenance.to_string(),
            edge_hash,
            now,
        ],
    )?;
    for evidence_ref in &candidate.evidence_refs {
        let evidence_id = stable_candidate_id(
            "graph_edge_evidence",
            &stable_hash(&format!("{edge_id}|{evidence_ref}")),
        );
        connection.execute(
            "INSERT OR IGNORE INTO graph_edge_evidence (
                id, edge_id, evidence_kind, evidence_ref, summary, created_at
             ) VALUES (?1, ?2, 'candidate_evidence', ?3, ?4, ?5)",
            params![
                evidence_id,
                edge_id,
                evidence_ref,
                "Evidence retained from knowledge graph edge candidate",
                now,
            ],
        )?;
    }
    connection.execute(
        "UPDATE knowledge_graph_edge_candidates
         SET candidate_state = 'confirmed', state_changed_at = COALESCE(state_changed_at, ?2),
             state_reason = COALESCE(state_reason, ?3), updated_at = ?2
         WHERE id = ?1",
        params![candidate.id, now, sanitize_text(reason)],
    )?;
    let promotion_id = record_candidate_promotion(
        connection,
        "edge",
        &candidate.id,
        None,
        Some(&edge_id),
        actor_id,
        reason,
        &candidate.evidence_refs,
        &provenance,
        &now,
    )?;
    append_promotion_event(
        connection,
        &candidate.conversation_id,
        candidate.segment_id.as_deref(),
        "edge",
        &candidate.id,
        None,
        Some(&edge_id),
        &candidate.evidence_refs,
    )?;
    Ok(GraphPromotionOutcome {
        promotion_id,
        node: None,
        edge: Some(load_graph_edge(connection, &edge_id)?),
    })
}

pub fn graph_nodes_for_resource(
    connection: &Connection,
    resource_kind: &str,
    resource_id: &str,
    viewer: &GraphViewer,
) -> Result<Vec<GraphNodeView>> {
    let mut statement = connection.prepare(
        "SELECT id, node_kind, resource_kind, resource_id, label, status, visibility_ceiling,
                content_hash, provenance_json, created_at, updated_at
         FROM graph_nodes
         WHERE resource_kind = ?1 AND resource_id = ?2 AND status = 'confirmed'
         ORDER BY created_at ASC, id ASC",
    )?;
    let nodes = statement
        .query_map(params![resource_kind, resource_id], graph_node_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter(|node| viewer_can_see(viewer, &node.visibility_ceiling))
        .collect::<Vec<_>>();
    audit_graph_query(
        connection,
        "graph.get_resource_nodes",
        viewer,
        &json!({"resourceKind": resource_kind, "resourceId": resource_id}),
        &json!({"nodeIds": nodes.iter().map(|node| &node.id).collect::<Vec<_>>()}),
    )?;
    Ok(nodes)
}

pub fn graph_one_hop_neighborhood(
    connection: &Connection,
    node_id: &str,
    viewer: &GraphViewer,
) -> Result<GraphNeighborhood> {
    let Some(root) = load_graph_node_optional(connection, node_id)? else {
        return Ok(GraphNeighborhood {
            nodes: vec![],
            edges: vec![],
            limitations: vec!["Graph node was not found.".to_string()],
        });
    };
    if !viewer_can_see(viewer, &root.visibility_ceiling) {
        audit_graph_query(
            connection,
            "graph.get_resource_neighborhood",
            viewer,
            &json!({"nodeId": node_id}),
            &json!({"denied": true}),
        )?;
        return Ok(GraphNeighborhood {
            nodes: vec![],
            edges: vec![],
            limitations: vec!["Graph neighborhood is not visible to this viewer.".to_string()],
        });
    }
    let mut edge_statement = connection.prepare(
        "SELECT id, source_node_id, target_node_id, relationship_kind, status, confidence,
                visibility_ceiling, evidence_refs_json, provenance_json, content_hash,
                created_at, updated_at
         FROM graph_edges
         WHERE status = 'confirmed' AND (source_node_id = ?1 OR target_node_id = ?1)
         ORDER BY created_at ASC, id ASC",
    )?;
    let edges = edge_statement
        .query_map([node_id], graph_edge_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter(|edge| viewer_can_see(viewer, &edge.visibility_ceiling))
        .collect::<Vec<_>>();
    let mut nodes = vec![root];
    for edge in &edges {
        for related_id in [&edge.source_node_id, &edge.target_node_id] {
            if nodes.iter().any(|node| node.id == *related_id) {
                continue;
            }
            if let Some(node) = load_graph_node_optional(connection, related_id)? {
                if viewer_can_see(viewer, &node.visibility_ceiling) {
                    nodes.push(node);
                }
            }
        }
    }
    audit_graph_query(
        connection,
        "graph.get_resource_neighborhood",
        viewer,
        &json!({"nodeId": node_id}),
        &json!({
            "nodeIds": nodes.iter().map(|node| &node.id).collect::<Vec<_>>(),
            "edgeIds": edges.iter().map(|edge| &edge.id).collect::<Vec<_>>(),
        }),
    )?;
    Ok(GraphNeighborhood {
        nodes,
        edges,
        limitations: vec!["One-hop confirmed graph traversal only.".to_string()],
    })
}

pub fn list_graph_candidates(
    connection: &Connection,
    conversation_id: &str,
    candidate_state: Option<&str>,
) -> Result<KnowledgeGraphCandidateList> {
    Ok(KnowledgeGraphCandidateList {
        nodes: list_node_candidates(connection, conversation_id, candidate_state)?,
        edges: list_edge_candidates(connection, conversation_id, candidate_state)?,
    })
}

pub fn load_node_candidate(
    connection: &Connection,
    candidate_id: &str,
) -> Result<KnowledgeGraphNodeCandidateView> {
    connection
        .query_row(
            "SELECT id, job_id, conversation_id, segment_id, source_analysis_candidate_id,
                    node_kind, label, candidate_state, confidence, evidence_refs_json,
                    provenance_json, source_event_refs_json, content_hash, visibility,
                    created_at, updated_at, state_changed_at, state_reason
             FROM knowledge_graph_node_candidates
             WHERE id = ?1",
            [candidate_id],
            node_from_row,
        )
        .map_err(Into::into)
}

pub fn load_edge_candidate(
    connection: &Connection,
    candidate_id: &str,
) -> Result<KnowledgeGraphEdgeCandidateView> {
    connection
        .query_row(
            "SELECT id, job_id, conversation_id, segment_id, source_analysis_candidate_id,
                    source_node_candidate_id, target_node_candidate_id, relationship_kind,
                    label, candidate_state, confidence, evidence_refs_json, provenance_json,
                    source_event_refs_json, content_hash, visibility, created_at, updated_at,
                    state_changed_at, state_reason
             FROM knowledge_graph_edge_candidates
             WHERE id = ?1",
            [candidate_id],
            edge_from_row,
        )
        .map_err(Into::into)
}

fn append_state_transition_event(
    connection: &Connection,
    conversation_id: &str,
    segment_id: Option<&str>,
    candidate_id: &str,
    candidate_target: &str,
    new_state: &str,
    reason: &str,
) -> Result<RealtimeEvent> {
    append_conversation_event(
        connection,
        conversation_id,
        segment_id,
        None,
        &format!("knowledge_graph.candidate.{new_state}"),
        json!({
            "candidateId": candidate_id,
            "candidateTarget": candidate_target,
            "candidateState": new_state,
            "reason": sanitize_text(reason),
        }),
        None,
    )
}

fn validate_common_candidate_input(
    label: &str,
    confidence: f64,
    evidence_refs: &[String],
    provenance: &Value,
) -> Result<()> {
    ensure!(!label.trim().is_empty(), "candidate label is required");
    ensure!(
        (0.0..=1.0).contains(&confidence),
        "candidate confidence must be 0.0..=1.0"
    );
    ensure!(
        !evidence_refs.is_empty(),
        "graph candidate evidence refs are required"
    );
    ensure!(
        !provenance
            .as_object()
            .map(|object| object.is_empty())
            .unwrap_or(true),
        "graph candidate provenance is required"
    );
    Ok(())
}

fn load_message(connection: &Connection, message_id: &str) -> Result<ConversationMessageView> {
    connection
        .query_row(
            "SELECT id, conversation_id, segment_id, participant_id, message_kind, status,
                    body_markdown, visibility, client_message_id, sequence, event_cursor,
                    undo_expires_at, undo_cancelled_at, created_at, edited_at, deleted_at
             FROM conversation_messages
             WHERE id = ?1",
            [message_id],
            |row| {
                Ok(ConversationMessageView {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    segment_id: row.get(2)?,
                    participant_id: row.get(3)?,
                    message_kind: row.get(4)?,
                    status: row.get(5)?,
                    body_markdown: row.get(6)?,
                    visibility: row.get(7)?,
                    client_message_id: row.get(8)?,
                    sequence: row.get(9)?,
                    event_cursor: row.get(10)?,
                    undo_expires_at: row.get(11)?,
                    undo_cancelled_at: row.get(12)?,
                    created_at: row.get(13)?,
                    edited_at: row.get(14)?,
                    deleted_at: row.get(15)?,
                })
            },
        )
        .map_err(Into::into)
}

fn list_node_candidates(
    connection: &Connection,
    conversation_id: &str,
    candidate_state: Option<&str>,
) -> Result<Vec<KnowledgeGraphNodeCandidateView>> {
    let mut statement = if candidate_state.is_some() {
        connection.prepare(
            "SELECT id, job_id, conversation_id, segment_id, source_analysis_candidate_id,
                    node_kind, label, candidate_state, confidence, evidence_refs_json,
                    provenance_json, source_event_refs_json, content_hash, visibility,
                    created_at, updated_at, state_changed_at, state_reason
             FROM knowledge_graph_node_candidates
             WHERE conversation_id = ?1 AND candidate_state = ?2
             ORDER BY created_at ASC",
        )?
    } else {
        connection.prepare(
            "SELECT id, job_id, conversation_id, segment_id, source_analysis_candidate_id,
                    node_kind, label, candidate_state, confidence, evidence_refs_json,
                    provenance_json, source_event_refs_json, content_hash, visibility,
                    created_at, updated_at, state_changed_at, state_reason
             FROM knowledge_graph_node_candidates
             WHERE conversation_id = ?1
             ORDER BY created_at ASC",
        )?
    };
    let rows = if let Some(state) = candidate_state {
        statement.query_map(params![conversation_id, state], node_from_row)?
    } else {
        statement.query_map(params![conversation_id], node_from_row)?
    };
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn list_edge_candidates(
    connection: &Connection,
    conversation_id: &str,
    candidate_state: Option<&str>,
) -> Result<Vec<KnowledgeGraphEdgeCandidateView>> {
    let mut statement = if candidate_state.is_some() {
        connection.prepare(
            "SELECT id, job_id, conversation_id, segment_id, source_analysis_candidate_id,
                    source_node_candidate_id, target_node_candidate_id, relationship_kind,
                    label, candidate_state, confidence, evidence_refs_json, provenance_json,
                    source_event_refs_json, content_hash, visibility, created_at, updated_at,
                    state_changed_at, state_reason
             FROM knowledge_graph_edge_candidates
             WHERE conversation_id = ?1 AND candidate_state = ?2
             ORDER BY created_at ASC",
        )?
    } else {
        connection.prepare(
            "SELECT id, job_id, conversation_id, segment_id, source_analysis_candidate_id,
                    source_node_candidate_id, target_node_candidate_id, relationship_kind,
                    label, candidate_state, confidence, evidence_refs_json, provenance_json,
                    source_event_refs_json, content_hash, visibility, created_at, updated_at,
                    state_changed_at, state_reason
             FROM knowledge_graph_edge_candidates
             WHERE conversation_id = ?1
             ORDER BY created_at ASC",
        )?
    };
    let rows = if let Some(state) = candidate_state {
        statement.query_map(params![conversation_id, state], edge_from_row)?
    } else {
        statement.query_map(params![conversation_id], edge_from_row)?
    };
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn node_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeGraphNodeCandidateView> {
    Ok(KnowledgeGraphNodeCandidateView {
        id: row.get(0)?,
        job_id: row.get(1)?,
        conversation_id: row.get(2)?,
        segment_id: row.get(3)?,
        source_analysis_candidate_id: row.get(4)?,
        node_kind: row.get(5)?,
        label: row.get(6)?,
        candidate_state: row.get(7)?,
        confidence: row.get(8)?,
        evidence_refs: json_string_array(row.get(9)?),
        provenance: json_object(row.get(10)?),
        source_event_refs: json_string_array(row.get(11)?),
        content_hash: row.get(12)?,
        visibility: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
        state_changed_at: row.get(16)?,
        state_reason: row.get(17)?,
    })
}

fn edge_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeGraphEdgeCandidateView> {
    Ok(KnowledgeGraphEdgeCandidateView {
        id: row.get(0)?,
        job_id: row.get(1)?,
        conversation_id: row.get(2)?,
        segment_id: row.get(3)?,
        source_analysis_candidate_id: row.get(4)?,
        source_node_candidate_id: row.get(5)?,
        target_node_candidate_id: row.get(6)?,
        relationship_kind: row.get(7)?,
        label: row.get(8)?,
        candidate_state: row.get(9)?,
        confidence: row.get(10)?,
        evidence_refs: json_string_array(row.get(11)?),
        provenance: json_object(row.get(12)?),
        source_event_refs: json_string_array(row.get(13)?),
        content_hash: row.get(14)?,
        visibility: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
        state_changed_at: row.get(18)?,
        state_reason: row.get(19)?,
    })
}

fn load_graph_node(connection: &Connection, node_id: &str) -> Result<GraphNodeView> {
    load_graph_node_optional(connection, node_id)?
        .ok_or_else(|| anyhow::anyhow!("graph node not found: {node_id}"))
}

fn load_graph_node_optional(
    connection: &Connection,
    node_id: &str,
) -> Result<Option<GraphNodeView>> {
    connection
        .query_row(
            "SELECT id, node_kind, resource_kind, resource_id, label, status, visibility_ceiling,
                    content_hash, provenance_json, created_at, updated_at
             FROM graph_nodes
             WHERE id = ?1",
            [node_id],
            graph_node_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn load_graph_edge(connection: &Connection, edge_id: &str) -> Result<GraphEdgeView> {
    connection
        .query_row(
            "SELECT id, source_node_id, target_node_id, relationship_kind, status, confidence,
                    visibility_ceiling, evidence_refs_json, provenance_json, content_hash,
                    created_at, updated_at
             FROM graph_edges
             WHERE id = ?1",
            [edge_id],
            graph_edge_from_row,
        )
        .map_err(Into::into)
}

fn graph_node_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GraphNodeView> {
    Ok(GraphNodeView {
        id: row.get(0)?,
        node_kind: row.get(1)?,
        resource_kind: row.get(2)?,
        resource_id: row.get(3)?,
        label: row.get(4)?,
        status: row.get(5)?,
        visibility_ceiling: row.get(6)?,
        content_hash: row.get(7)?,
        provenance: json_object(row.get(8)?),
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn graph_edge_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GraphEdgeView> {
    Ok(GraphEdgeView {
        id: row.get(0)?,
        source_node_id: row.get(1)?,
        target_node_id: row.get(2)?,
        relationship_kind: row.get(3)?,
        status: row.get(4)?,
        confidence: row.get(5)?,
        visibility_ceiling: row.get(6)?,
        evidence_refs: json_string_array(row.get(7)?),
        provenance: json_object(row.get(8)?),
        content_hash: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn record_candidate_promotion(
    connection: &Connection,
    candidate_kind: &str,
    candidate_id: &str,
    graph_node_id: Option<&str>,
    graph_edge_id: Option<&str>,
    actor_id: Option<&str>,
    reason: &str,
    evidence_refs: &[String],
    provenance: &Value,
    now: &str,
) -> Result<String> {
    let promotion_id = stable_candidate_id(
        "graph_candidate_promotion",
        &stable_hash(&format!("{candidate_kind}|{candidate_id}|confirmed")),
    );
    connection.execute(
        "INSERT OR IGNORE INTO graph_candidate_promotions (
            id, candidate_kind, candidate_id, graph_node_id, graph_edge_id, decision,
            reason, actor_id, evidence_refs_json, provenance_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, 'confirmed', ?6, ?7, ?8, ?9, ?10)",
        params![
            promotion_id,
            candidate_kind,
            candidate_id,
            graph_node_id,
            graph_edge_id,
            sanitize_text(reason),
            actor_id,
            json!(evidence_refs).to_string(),
            provenance.to_string(),
            now,
        ],
    )?;
    Ok(promotion_id)
}

fn append_promotion_event(
    connection: &Connection,
    conversation_id: &str,
    segment_id: Option<&str>,
    candidate_kind: &str,
    candidate_id: &str,
    graph_node_id: Option<&str>,
    graph_edge_id: Option<&str>,
    evidence_refs: &[String],
) -> Result<Option<RealtimeEvent>> {
    let already_logged: i64 = connection.query_row(
        "SELECT COUNT(*) FROM conversation_events
         WHERE conversation_id = ?1 AND event_type = 'knowledge_graph.candidate.promoted'
           AND payload_json LIKE ?2",
        params![conversation_id, format!("%{candidate_id}%")],
        |row| row.get(0),
    )?;
    if already_logged > 0 {
        return Ok(None);
    }
    Ok(Some(append_conversation_event(
        connection,
        conversation_id,
        segment_id,
        None,
        "knowledge_graph.candidate.promoted",
        json!({
            "candidateKind": candidate_kind,
            "candidateId": candidate_id,
            "graphNodeId": graph_node_id,
            "graphEdgeId": graph_edge_id,
            "evidenceRefs": evidence_refs,
        }),
        None,
    )?))
}

fn audit_graph_query(
    connection: &Connection,
    method_name: &str,
    viewer: &GraphViewer,
    input: &Value,
    output: &Value,
) -> Result<()> {
    let input_hash = stable_hash(&canonical_json_string(input));
    let output_hash = stable_hash(&canonical_json_string(output));
    let id = stable_candidate_id(
        "graph_query_audit",
        &stable_hash(&format!("{method_name}|{input_hash}|{output_hash}")),
    );
    connection.execute(
        "INSERT OR IGNORE INTO graph_query_audit (
            id, method_name, viewer_context_json, input_hash, output_hash,
            policy_decision_id, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6)",
        params![
            id,
            method_name,
            json!({
                "actorId": viewer.actor_id,
                "visibility": viewer.visibility,
            })
            .to_string(),
            input_hash,
            output_hash,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

fn canonical_json_string(value: &Value) -> String {
    canonical_json_value(value).to_string()
}

fn canonical_json_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonical_json_value).collect()),
        Value::Object(map) => {
            let mut sorted = serde_json::Map::new();
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                if let Some(value) = map.get(key) {
                    sorted.insert(key.clone(), canonical_json_value(value));
                }
            }
            Value::Object(sorted)
        }
        other => other.clone(),
    }
}

fn normalize_graph_visibility(visibility: &str) -> String {
    match visibility {
        "public" => "public".to_string(),
        "authenticated" | "member" | "participants" => "member".to_string(),
        "staff" | "staff_private" => "staff".to_string(),
        "owner" | "owner_private" => "owner".to_string(),
        _ => "staff".to_string(),
    }
}

fn viewer_can_see(viewer: &GraphViewer, visibility_ceiling: &str) -> bool {
    visibility_rank(&viewer.visibility) >= visibility_rank(visibility_ceiling)
}

fn visibility_rank(visibility: &str) -> u8 {
    match normalize_graph_visibility(visibility).as_str() {
        "public" => 0,
        "member" => 1,
        "staff" => 2,
        "owner" => 3,
        _ => 2,
    }
}

fn extract_entity_labels(text: &str) -> Vec<String> {
    let mut labels = Vec::new();
    for raw in text.split_whitespace() {
        let token = raw.trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '-' && ch != '_');
        if token.len() < 3 || token.contains('@') || looks_like_api_key(token) {
            continue;
        }
        let Some(first) = token.chars().next() else {
            continue;
        };
        if first.is_uppercase() && !is_common_sentence_word(token) {
            let label = sanitize_text(token);
            if !labels.contains(&label) {
                labels.push(label);
            }
        }
        if labels.len() >= 4 {
            break;
        }
    }
    labels
}

fn classify_node_kind(label: &str) -> &'static str {
    let lower = label.to_ascii_lowercase();
    if lower.ends_with("inc") || lower.ends_with("llc") || lower.ends_with("studio") {
        "organization"
    } else {
        "mentioned_entity"
    }
}

fn relationship_kind_for_text(text: &str) -> &'static str {
    let lower = text.to_ascii_lowercase();
    if lower.contains("interested in") {
        "interested_in"
    } else if lower.contains("works with") || lower.contains("working with") {
        "works_with"
    } else if lower.contains("referred") {
        "referred_by"
    } else if lower.contains("please") || lower.contains("need ") || lower.contains("request") {
        "requested"
    } else {
        "mentioned_with"
    }
}

fn is_common_sentence_word(token: &str) -> bool {
    matches!(
        token,
        "Can" | "Could" | "Please" | "Need" | "What" | "When" | "Where" | "How" | "The" | "This"
    )
}

fn evidence_ref(message: &ConversationMessageView) -> String {
    format!("conversation_message:{}", message.id)
}

fn sanitize_json(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(sanitize_text(&text)),
        Value::Array(values) => Value::Array(values.into_iter().map(sanitize_json).collect()),
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (key, sanitize_json(value)))
                .collect(),
        ),
        other => other,
    }
}

fn sanitize_text(text: &str) -> String {
    let mut sanitized = Vec::new();
    let mut skip_next_bearer = false;
    for token in text.split_whitespace() {
        if skip_next_bearer {
            sanitized.push("[REDACTED_TOKEN]".to_string());
            skip_next_bearer = false;
            continue;
        }
        if token.eq_ignore_ascii_case("bearer") {
            sanitized.push("Bearer".to_string());
            skip_next_bearer = true;
            continue;
        }
        let trimmed = token.trim_matches(|ch: char| {
            !ch.is_alphanumeric() && ch != '@' && ch != '-' && ch != '_' && ch != '.'
        });
        if looks_like_email(trimmed) {
            sanitized.push(token.replace(trimmed, "[REDACTED_EMAIL]"));
        } else if looks_like_api_key(trimmed) {
            sanitized.push(token.replace(trimmed, "[REDACTED_SECRET]"));
        } else {
            sanitized.push(token.to_string());
        }
    }
    sanitized.join(" ")
}

fn looks_like_email(token: &str) -> bool {
    token.contains('@') && token.contains('.') && token.len() >= 6
}

fn looks_like_api_key(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    (lower.starts_with("sk-") || lower.starts_with("tok_") || lower.starts_with("key_"))
        && token.len() >= 10
}

fn json_string_array(raw: String) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
}

fn json_object(raw: String) -> Value {
    serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}))
}

fn stable_candidate_id(prefix: &str, content_hash: &str) -> String {
    let suffix = content_hash.strip_prefix("sha256:").unwrap_or(content_hash);
    format!("{prefix}_{}", &suffix[..24.min(suffix.len())])
}

fn stable_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::seed_builtin_capabilities;
    use crate::conversation_analysis::{
        queue_analysis_for_message, run_deterministic_analysis_for_job,
    };
    use crate::conversations::{
        create_conversation_message, create_conversation_participant,
        find_or_create_canonical_conversation, CanonicalConversationRequest,
        ConversationMessageCreateRequest, ConversationParticipantCreateRequest,
    };
    use crate::schema::init_schema;

    fn test_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        init_schema(&connection).unwrap();
        seed_builtin_capabilities(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO actors (id, actor_kind, display_name, status, metadata_json, created_at, updated_at)
                 VALUES ('actor_staff', 'staff', 'Staff', 'active', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO connections (
                    id, connection_type, display_name, status, identity_json, scope_json, metadata_json, created_at, updated_at
                 ) VALUES ('connection_1', 'client', 'Client', 'active', '{}', '{}', '{}', 'now', 'now')",
                [],
            )
            .unwrap();
        connection
    }

    fn create_message(connection: &Connection, body: &str) -> ConversationMessageView {
        let conversation = find_or_create_canonical_conversation(
            connection,
            &CanonicalConversationRequest {
                surface: "client_portal".to_string(),
                subject_kind: "connection".to_string(),
                subject_id: "connection_1".to_string(),
                connection_id: Some("connection_1".to_string()),
                visitor_session_id: None,
                created_by_actor_id: Some("actor_staff".to_string()),
            },
        )
        .unwrap();
        let participant = create_conversation_participant(
            connection,
            &ConversationParticipantCreateRequest {
                conversation_id: conversation.id.clone(),
                participant_kind: "connection".to_string(),
                actor_id: None,
                connection_id: Some("connection_1".to_string()),
                visitor_session_id: None,
                display_name: "Client".to_string(),
                role: "client".to_string(),
            },
        )
        .unwrap();
        create_conversation_message(
            connection,
            &ConversationMessageCreateRequest {
                conversation_id: conversation.id,
                segment_id: None,
                participant_id: participant.id,
                message_kind: "human".to_string(),
                body_markdown: body.to_string(),
                visibility: "participants".to_string(),
                client_message_id: "client_msg_graph".to_string(),
                reply_to_message_id: None,
                undo_expires_at: None,
            },
        )
        .unwrap()
    }

    fn completed_analysis_job(connection: &Connection, body: &str) -> ConversationAnalysisJobView {
        let message = create_message(connection, body);
        let job = queue_analysis_for_message(connection, &message)
            .unwrap()
            .unwrap();
        run_deterministic_analysis_for_job(connection, &job.id).unwrap();
        load_job(connection, &job.id).unwrap()
    }

    #[test]
    fn graph_candidates_require_evidence_and_provenance() {
        let connection = test_connection();
        let job = completed_analysis_job(&connection, "Ada from Acme is interested in Ordo.");

        let missing_evidence = propose_node_candidate(
            &connection,
            &job,
            KnowledgeGraphNodeCandidateInput {
                source_analysis_candidate_id: None,
                node_kind: "person".to_string(),
                label: "Ada".to_string(),
                confidence: 0.8,
                evidence_refs: vec![],
                provenance: json!({"generator": "test"}),
                source_event_refs: vec![],
                visibility: "staff_private".to_string(),
            },
        );
        assert!(missing_evidence.is_err());

        let missing_provenance = propose_node_candidate(
            &connection,
            &job,
            KnowledgeGraphNodeCandidateInput {
                source_analysis_candidate_id: None,
                node_kind: "person".to_string(),
                label: "Ada".to_string(),
                confidence: 0.8,
                evidence_refs: vec!["conversation_message:message_1".to_string()],
                provenance: json!({}),
                source_event_refs: vec![],
                visibility: "staff_private".to_string(),
            },
        );
        assert!(missing_provenance.is_err());
    }

    #[test]
    fn deterministic_extraction_creates_proposed_nodes_edges_and_is_idempotent() {
        let connection = test_connection();
        let job = completed_analysis_job(
            &connection,
            "Ada from Acme is interested in Ordo premium chat.",
        );

        let (first, first_events) =
            extract_graph_candidates_for_analysis_job(&connection, &job.id).unwrap();
        let (second, second_events) =
            extract_graph_candidates_for_analysis_job(&connection, &job.id).unwrap();

        assert!(first.nodes.len() >= 2);
        assert_eq!(first.edges.len(), 1);
        assert!(first.nodes.iter().all(|node| {
            node.candidate_state == "proposed"
                && !node.evidence_refs.is_empty()
                && !node.provenance.as_object().unwrap().is_empty()
                && node.content_hash.starts_with("sha256:")
        }));
        assert_eq!(first.nodes, second.nodes);
        assert_eq!(first.edges, second.edges);
        assert!(first_events
            .iter()
            .any(|event| event.event_type == "knowledge_graph.node_candidate.created"));
        assert!(first_events
            .iter()
            .any(|event| event.event_type == "knowledge_graph.edge_candidate.created"));
        assert!(second_events.is_empty());
    }

    #[test]
    fn graph_candidate_lifecycle_and_listing_are_durable() {
        let connection = test_connection();
        let job = completed_analysis_job(&connection, "Ada works with Acme on Ordo.");
        let (list, _events) =
            extract_graph_candidates_for_analysis_job(&connection, &job.id).unwrap();
        let node_id = &list.nodes[0].id;
        let edge_id = &list.edges[0].id;

        let confirmed = transition_graph_candidate(
            &connection,
            KnowledgeGraphCandidateTarget::Node,
            node_id,
            "confirmed",
            "Staff verified from durable conversation evidence.",
        )
        .unwrap();
        let rejected = transition_graph_candidate(
            &connection,
            KnowledgeGraphCandidateTarget::Edge,
            edge_id,
            "rejected",
            "Relationship was too ambiguous.",
        )
        .unwrap();

        assert_eq!(confirmed.event_type, "knowledge_graph.candidate.confirmed");
        assert_eq!(rejected.event_type, "knowledge_graph.candidate.rejected");
        let confirmed_nodes =
            list_graph_candidates(&connection, &job.conversation_id, Some("confirmed")).unwrap();
        assert_eq!(confirmed_nodes.nodes.len(), 1);
        assert_eq!(confirmed_nodes.edges.len(), 0);
        assert_eq!(
            load_edge_candidate(&connection, edge_id)
                .unwrap()
                .candidate_state,
            "rejected"
        );
    }

    #[test]
    fn confirmed_graph_promotion_is_idempotent_and_retains_evidence() {
        let connection = test_connection();
        let job = completed_analysis_job(&connection, "Ada works with Acme on Ordo.");
        let (list, _events) =
            extract_graph_candidates_for_analysis_job(&connection, &job.id).unwrap();

        let first = promote_edge_candidate(
            &connection,
            &list.edges[0].id,
            Some("actor_staff"),
            "Staff verified relationship from durable conversation evidence.",
        )
        .unwrap();
        let second = promote_edge_candidate(
            &connection,
            &list.edges[0].id,
            Some("actor_staff"),
            "Staff verified relationship from durable conversation evidence.",
        )
        .unwrap();

        assert_eq!(first, second);
        let edge = first.edge.unwrap();
        assert_eq!(edge.status, "confirmed");
        assert!(!edge.evidence_refs.is_empty());
        assert_eq!(
            load_edge_candidate(&connection, &list.edges[0].id)
                .unwrap()
                .candidate_state,
            "confirmed"
        );
        let promotion_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM graph_candidate_promotions WHERE candidate_kind = 'edge' AND candidate_id = ?1",
                [&list.edges[0].id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(promotion_count, 1);
        let edge_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM graph_edges", [], |row| row.get(0))
            .unwrap();
        assert_eq!(edge_count, 1);
        let evidence_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM graph_edge_evidence WHERE edge_id = ?1",
                [&edge.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(evidence_count >= 1);
    }

    #[test]
    fn confirmed_graph_traversal_is_visibility_filtered() {
        let connection = test_connection();
        let job = completed_analysis_job(&connection, "Ada works with Acme on Ordo.");
        let (list, _events) =
            extract_graph_candidates_for_analysis_job(&connection, &job.id).unwrap();
        let edge = promote_edge_candidate(
            &connection,
            &list.edges[0].id,
            Some("actor_staff"),
            "Staff verified relationship from durable conversation evidence.",
        )
        .unwrap()
        .edge
        .unwrap();

        let staff_viewer = GraphViewer {
            actor_id: Some("actor_staff".to_string()),
            visibility: "staff".to_string(),
        };
        let public_viewer = GraphViewer {
            actor_id: None,
            visibility: "public".to_string(),
        };

        let staff_neighborhood =
            graph_one_hop_neighborhood(&connection, &edge.source_node_id, &staff_viewer).unwrap();
        assert_eq!(staff_neighborhood.edges.len(), 1);
        assert!(staff_neighborhood.nodes.len() >= 2);

        let public_neighborhood =
            graph_one_hop_neighborhood(&connection, &edge.source_node_id, &public_viewer).unwrap();
        assert!(public_neighborhood.edges.is_empty());
        assert!(public_neighborhood.nodes.is_empty());
        assert!(public_neighborhood
            .limitations
            .iter()
            .any(|limitation| limitation.contains("not visible")));

        let staff_nodes = graph_nodes_for_resource(
            &connection,
            "knowledge_graph_node_candidate",
            &list.nodes[0].id,
            &staff_viewer,
        )
        .unwrap();
        let public_nodes = graph_nodes_for_resource(
            &connection,
            "knowledge_graph_node_candidate",
            &list.nodes[0].id,
            &public_viewer,
        )
        .unwrap();
        assert_eq!(staff_nodes.len(), 1);
        assert!(public_nodes.is_empty());
        let audit_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM graph_query_audit", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(audit_count >= 3);
    }

    #[test]
    fn graph_candidates_do_not_store_sensitive_fixture_text() {
        let connection = test_connection();
        let job = completed_analysis_job(
            &connection,
            "Ada from Acme emailed ada@example.com with Bearer tok_abcdef123456 about Ordo.",
        );

        extract_graph_candidates_for_analysis_job(&connection, &job.id).unwrap();

        for raw in ["ada@example.com", "tok_abcdef123456"] {
            for (table, columns) in [
                (
                    "knowledge_graph_node_candidates",
                    "label || provenance_json || evidence_refs_json",
                ),
                (
                    "knowledge_graph_edge_candidates",
                    "label || provenance_json || evidence_refs_json",
                ),
                ("conversation_events", "payload_json"),
                ("realtime_events", "payload_json"),
            ] {
                let leaked_count: i64 = connection
                    .query_row(
                        &format!("SELECT COUNT(*) FROM {table} WHERE {columns} LIKE ?1"),
                        [format!("%{raw}%")],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(leaked_count, 0, "{table} leaked {raw}");
            }
        }
    }
}
