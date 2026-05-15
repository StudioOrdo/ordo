use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingHandoffPacketViewer {
    Owner,
    Staff,
    Member,
    Public,
}

impl OnboardingHandoffPacketViewer {
    fn can_view_staff_context(self) -> bool {
        matches!(self, Self::Owner | Self::Staff)
    }

    fn visibility_label(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Staff => "staff",
            Self::Member => "member",
            Self::Public => "public",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstUserOnboardingHandoffPacketRequest {
    pub viewer: OnboardingHandoffPacketViewer,
    pub visitor_session_id: Option<String>,
    pub handoff_item_id: Option<String>,
    pub conversation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstUserOnboardingHandoffPacket {
    pub schema_version: String,
    pub status: String,
    pub viewer: OnboardingHandoffPacketViewer,
    pub visibility: String,
    pub member_status: OnboardingMemberStatus,
    pub staff_context: Option<OnboardingStaffContext>,
    pub visitor: Option<OnboardingVisitorContext>,
    pub tracked_entry: Option<OnboardingTrackedEntryContext>,
    pub handoff: Option<OnboardingHandoffContext>,
    pub conversation: Option<OnboardingConversationContext>,
    pub product_request: Option<OnboardingProductRequestContext>,
    pub trial: OnboardingEvidenceHint,
    pub referral: OnboardingEvidenceHint,
    pub reward: OnboardingEvidenceHint,
    pub evidence_refs: Vec<String>,
    pub missing: Vec<String>,
    pub limitations: Vec<String>,
    pub live_provider_required: bool,
    pub mutates_canonical_state: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingMemberStatus {
    pub state: String,
    pub summary: String,
    pub next_step: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingStaffContext {
    pub summary: String,
    pub actionable_items: Vec<String>,
    pub missing_prerequisites: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingVisitorContext {
    pub id: String,
    pub status: String,
    pub destination_surface: String,
    pub destination_id: Option<String>,
    pub entry_point_id: String,
    pub entry_point_slug: String,
    pub created_at: String,
    pub last_seen_at: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingTrackedEntryContext {
    pub id: String,
    pub slug: String,
    pub label: String,
    pub source_kind: String,
    pub source_label: Option<String>,
    pub destination_surface: String,
    pub destination_id: Option<String>,
    pub public_path: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingHandoffContext {
    pub id: String,
    pub source_kind: String,
    pub source_id: Option<String>,
    pub delivery_state: String,
    pub owner_decision: Option<String>,
    pub reason: Option<String>,
    pub requested_action: Option<String>,
    pub urgency: Option<String>,
    pub next_action_hint: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingConversationContext {
    pub id: String,
    pub surface: String,
    pub subject_kind: String,
    pub subject_id: String,
    pub status: String,
    pub visibility: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingProductRequestContext {
    pub id: String,
    pub request_kind: String,
    pub title: String,
    pub status: String,
    pub priority: i64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingEvidenceHint {
    pub state: String,
    pub summary: String,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone)]
struct VisitorRecord {
    id: String,
    entry_point_id: String,
    entry_point_slug: String,
    status: String,
    destination_surface: String,
    destination_id: Option<String>,
    created_at: String,
    last_seen_at: String,
}

#[derive(Debug, Clone)]
struct EntryPointRecord {
    id: String,
    slug: String,
    label: String,
    source_kind: String,
    source_label: Option<String>,
    destination_surface: String,
    destination_id: Option<String>,
    public_path: String,
}

#[derive(Debug, Clone)]
struct HandoffRecord {
    id: String,
    source_kind: String,
    source_id: Option<String>,
    request: Value,
    delivery_state: String,
    owner_decision: Option<String>,
    reason: Option<String>,
    requested_action: Option<String>,
    urgency: Option<String>,
    next_action_hint: Option<String>,
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone)]
struct ConversationRecord {
    id: String,
    surface: String,
    subject_kind: String,
    subject_id: String,
    status: String,
    visibility: String,
    visitor_session_id: Option<String>,
}

#[derive(Debug, Clone)]
struct ProductRequestRecord {
    id: String,
    request_kind: String,
    title: String,
    status: String,
    priority: i64,
    evidence_refs: Vec<String>,
}

pub fn prepare_first_user_onboarding_handoff_packet(
    db_path: &Path,
    request: FirstUserOnboardingHandoffPacketRequest,
) -> Result<FirstUserOnboardingHandoffPacket> {
    let connection = Connection::open(db_path)?;
    let can_view_staff_context = request.viewer.can_view_staff_context();
    let handoff = request
        .handoff_item_id
        .as_deref()
        .map(|id| read_handoff(&connection, id))
        .transpose()?
        .flatten();

    let conversation_id = request
        .conversation_id
        .clone()
        .or_else(|| handoff.as_ref().and_then(handoff_conversation_id));
    let conversation = conversation_id
        .as_deref()
        .map(|id| read_conversation(&connection, id))
        .transpose()?
        .flatten();

    let visitor_session_id = request
        .visitor_session_id
        .clone()
        .or_else(|| handoff.as_ref().and_then(handoff_visitor_session_id))
        .or_else(|| {
            conversation
                .as_ref()
                .and_then(|conversation| conversation.visitor_session_id.clone())
        });
    let visitor = visitor_session_id
        .as_deref()
        .map(|id| read_visitor_session(&connection, id))
        .transpose()?
        .flatten();
    let tracked_entry = visitor
        .as_ref()
        .map(|visitor| read_tracked_entry_point(&connection, &visitor.entry_point_id))
        .transpose()?
        .flatten();
    let product_request = find_product_request(&connection, &handoff, &conversation, &visitor)?;
    let mut trial = build_trial_hint(&connection, visitor.as_ref(), tracked_entry.as_ref())?;
    let mut referral = build_referral_hint(&connection, visitor.as_ref())?;
    let mut reward = build_reward_hint(&connection, visitor.as_ref(), &referral, &trial)?;
    if !can_view_staff_context {
        trial.evidence_refs.clear();
        referral.evidence_refs.clear();
        reward.evidence_refs.clear();
    }

    let mut evidence_refs = BTreeSet::new();
    if let Some(visitor) = &visitor {
        evidence_refs.insert(format!("visitor_session:{}", visitor.id));
    }
    if let Some(entry) = &tracked_entry {
        evidence_refs.insert(format!("tracked_entry_point:{}", entry.id));
    }
    if let Some(handoff) = &handoff {
        evidence_refs.insert(format!("handoff_item:{}", handoff.id));
        evidence_refs.extend(handoff.evidence_refs.iter().cloned());
    }
    if let Some(conversation) = &conversation {
        evidence_refs.insert(format!("conversation:{}", conversation.id));
    }
    if let Some(product_request) = &product_request {
        evidence_refs.insert(format!("product_request_spine:{}", product_request.id));
        evidence_refs.extend(product_request.evidence_refs.iter().cloned());
    }
    evidence_refs.extend(trial.evidence_refs.iter().cloned());
    evidence_refs.extend(referral.evidence_refs.iter().cloned());
    evidence_refs.extend(reward.evidence_refs.iter().cloned());

    let mut missing = Vec::new();
    if visitor.is_none() {
        missing.push("visitor_session".to_string());
    }
    if tracked_entry.is_none() {
        missing.push("tracked_entry_point".to_string());
    }
    if handoff.is_none() {
        missing.push("handoff_item".to_string());
    }
    if conversation.is_none() {
        missing.push("conversation".to_string());
    }
    if product_request.is_none() {
        missing.push("product_request_spine".to_string());
    }

    let state = if handoff.is_some() && visitor.is_some() {
        "ready"
    } else {
        "incomplete"
    };
    let evidence_refs = evidence_refs
        .into_iter()
        .filter(|reference| allowed_packet_evidence_ref(reference, request.viewer))
        .collect::<Vec<_>>();
    let member_status_evidence_refs = if can_view_staff_context {
        evidence_refs.as_slice()
    } else {
        &[]
    };
    let member_status = member_status_for(state, &handoff, &trial, member_status_evidence_refs);
    let staff_context = can_view_staff_context.then(|| {
        staff_context_for(
            state,
            visitor.as_ref(),
            tracked_entry.as_ref(),
            handoff.as_ref(),
            conversation.as_ref(),
            product_request.as_ref(),
            &trial,
            &referral,
            &reward,
            &missing,
            &evidence_refs,
        )
    });

    let handoff_context = handoff
        .as_ref()
        .map(|handoff| handoff_context_for(handoff, request.viewer));
    let packet = FirstUserOnboardingHandoffPacket {
        schema_version: "first_user_onboarding_handoff_packet.v1".to_string(),
        status: state.to_string(),
        viewer: request.viewer,
        visibility: request.viewer.visibility_label().to_string(),
        member_status,
        staff_context,
        visitor: if can_view_staff_context {
            visitor.map(VisitorRecord::into_context)
        } else {
            None
        },
        tracked_entry: if can_view_staff_context {
            tracked_entry.map(EntryPointRecord::into_context)
        } else {
            None
        },
        handoff: if can_view_staff_context {
            handoff_context
        } else {
            None
        },
        conversation: if can_view_staff_context {
            conversation.map(ConversationRecord::into_context)
        } else {
            None
        },
        product_request: if can_view_staff_context {
            product_request.map(ProductRequestRecord::into_context)
        } else {
            None
        },
        trial,
        referral,
        reward,
        evidence_refs: if can_view_staff_context {
            evidence_refs
        } else {
            Vec::new()
        },
        missing: if can_view_staff_context {
            missing
        } else {
            Vec::new()
        },
        limitations: vec![
            "Packet generation is read-only and does not grant access, rewards, or trial authority."
                .to_string(),
            "Trial, referral, and reward status is reported only from canonical evidence found in local tables."
                .to_string(),
        ],
        live_provider_required: false,
        mutates_canonical_state: false,
    };
    Ok(packet)
}

fn read_visitor_session(connection: &Connection, id: &str) -> Result<Option<VisitorRecord>> {
    Ok(connection
        .query_row(
            "SELECT id, entry_point_id, entry_point_slug, status, destination_surface,
                    destination_id, created_at, last_seen_at
             FROM visitor_sessions
             WHERE id = ?1",
            params![id],
            |row| {
                Ok(VisitorRecord {
                    id: row.get(0)?,
                    entry_point_id: row.get(1)?,
                    entry_point_slug: row.get(2)?,
                    status: row.get(3)?,
                    destination_surface: row.get(4)?,
                    destination_id: row.get(5)?,
                    created_at: row.get(6)?,
                    last_seen_at: row.get(7)?,
                })
            },
        )
        .optional()?)
}

fn read_tracked_entry_point(connection: &Connection, id: &str) -> Result<Option<EntryPointRecord>> {
    Ok(connection
        .query_row(
            "SELECT id, slug, label, source_kind, source_label, destination_surface,
                    destination_id, public_path
             FROM tracked_entry_points
             WHERE id = ?1",
            params![id],
            |row| {
                Ok(EntryPointRecord {
                    id: row.get(0)?,
                    slug: row.get(1)?,
                    label: row.get(2)?,
                    source_kind: row.get(3)?,
                    source_label: row.get(4)?,
                    destination_surface: row.get(5)?,
                    destination_id: row.get(6)?,
                    public_path: row.get(7)?,
                })
            },
        )
        .optional()?)
}

fn read_handoff(connection: &Connection, id: &str) -> Result<Option<HandoffRecord>> {
    Ok(connection
        .query_row(
            "SELECT id, source_kind, source_id, request_json, delivery_state, owner_decision,
                    reason, requested_action, urgency, next_action_hint, evidence_refs_json
             FROM handoff_inbox_items
             WHERE id = ?1",
            params![id],
            |row| {
                let request_json: String = row.get(3)?;
                let evidence_refs_json: String = row.get(10)?;
                Ok(HandoffRecord {
                    id: row.get(0)?,
                    source_kind: row.get(1)?,
                    source_id: row.get(2)?,
                    request: serde_json::from_str(&request_json).unwrap_or_else(|_| json!({})),
                    delivery_state: row.get(4)?,
                    owner_decision: row.get(5)?,
                    reason: row.get(6)?,
                    requested_action: row.get(7)?,
                    urgency: row.get(8)?,
                    next_action_hint: row.get(9)?,
                    evidence_refs: serde_json::from_str(&evidence_refs_json).unwrap_or_default(),
                })
            },
        )
        .optional()?)
}

fn read_conversation(connection: &Connection, id: &str) -> Result<Option<ConversationRecord>> {
    Ok(connection
        .query_row(
            "SELECT id, surface, subject_kind, subject_id, status, visibility, visitor_session_id
             FROM conversations
             WHERE id = ?1",
            params![id],
            |row| {
                Ok(ConversationRecord {
                    id: row.get(0)?,
                    surface: row.get(1)?,
                    subject_kind: row.get(2)?,
                    subject_id: row.get(3)?,
                    status: row.get(4)?,
                    visibility: row.get(5)?,
                    visitor_session_id: row.get(6)?,
                })
            },
        )
        .optional()?)
}

fn find_product_request(
    connection: &Connection,
    handoff: &Option<HandoffRecord>,
    conversation: &Option<ConversationRecord>,
    visitor: &Option<VisitorRecord>,
) -> Result<Option<ProductRequestRecord>> {
    let mut source_ids = Vec::new();
    if let Some(handoff) = handoff {
        source_ids.push(handoff.id.clone());
        if let Some(source_id) = &handoff.source_id {
            source_ids.push(source_id.clone());
        }
    }
    if let Some(conversation) = conversation {
        source_ids.push(conversation.id.clone());
    }
    if let Some(visitor) = visitor {
        source_ids.push(visitor.id.clone());
    }
    for source_id in source_ids {
        let request = connection
            .query_row(
                "SELECT id, request_kind, title, status, priority, evidence_refs_json
                 FROM product_request_spine
                 WHERE source_id = ?1 OR object_id = ?1
                 ORDER BY priority DESC, updated_at DESC
                 LIMIT 1",
                params![source_id],
                |row| {
                    let evidence_refs_json: String = row.get(5)?;
                    Ok(ProductRequestRecord {
                        id: row.get(0)?,
                        request_kind: row.get(1)?,
                        title: row.get(2)?,
                        status: row.get(3)?,
                        priority: row.get(4)?,
                        evidence_refs: serde_json::from_str(&evidence_refs_json)
                            .unwrap_or_default(),
                    })
                },
            )
            .optional()?;
        if request.is_some() {
            return Ok(request);
        }
    }
    Ok(None)
}

fn build_trial_hint(
    connection: &Connection,
    visitor: Option<&VisitorRecord>,
    tracked_entry: Option<&EntryPointRecord>,
) -> Result<OnboardingEvidenceHint> {
    let Some(visitor) = visitor else {
        return Ok(missing_hint(
            "trial",
            "No visitor session was found, so trial intent cannot be attributed.",
        ));
    };
    let canonical = connection
        .query_row(
            "SELECT oa.id, oa.status, t.id, t.status
             FROM offer_acceptances oa
             LEFT JOIN trials t ON t.acceptance_id = oa.id
             WHERE oa.visitor_session_id = ?1
             ORDER BY oa.accepted_at DESC
             LIMIT 1",
            params![visitor.id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()?;
    if let Some((acceptance_id, acceptance_status, trial_id, trial_status)) = canonical {
        let mut refs = vec![format!("offer_acceptance:{acceptance_id}")];
        if let Some(trial_id) = trial_id {
            refs.push(format!("trial:{trial_id}"));
        }
        return Ok(OnboardingEvidenceHint {
            state: "evidence_found".to_string(),
            summary: format!(
                "Canonical offer acceptance is {acceptance_status}; trial state is {}.",
                trial_status.unwrap_or_else(|| "not_created".to_string())
            ),
            evidence_refs: refs,
            limitations: Vec::new(),
        });
    }
    if visitor.destination_surface == "offer"
        || visitor.destination_surface == "offers"
        || tracked_entry
            .map(|entry| {
                entry.destination_surface == "offer" || entry.destination_surface == "offers"
            })
            .unwrap_or(false)
    {
        return Ok(OnboardingEvidenceHint {
            state: "pending_canonical_evidence".to_string(),
            summary: "Visitor arrived through an offer-oriented entry point, but no canonical offer acceptance or trial record exists.".to_string(),
            evidence_refs: vec![format!("visitor_session:{}", visitor.id)],
            limitations: vec!["Do not treat entry scans or offer-page visits as trial authority.".to_string()],
        });
    }
    Ok(missing_hint(
        "trial",
        "No canonical offer acceptance or trial evidence was found.",
    ))
}

fn build_referral_hint(
    connection: &Connection,
    visitor: Option<&VisitorRecord>,
) -> Result<OnboardingEvidenceHint> {
    let Some(visitor) = visitor else {
        return Ok(missing_hint(
            "referral",
            "No visitor session was found, so referral evidence cannot be attributed.",
        ));
    };
    let referral = connection
        .query_row(
            "SELECT id, status, evidence_refs_json
             FROM referral_records
             WHERE visitor_session_id = ?1
             ORDER BY updated_at DESC
             LIMIT 1",
            params![visitor.id],
            |row| {
                let evidence_refs_json: String = row.get(2)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    serde_json::from_str::<Vec<String>>(&evidence_refs_json).unwrap_or_default(),
                ))
            },
        )
        .optional()?;
    if let Some((id, status, mut refs)) = referral {
        refs.push(format!("referral_record:{id}"));
        refs.sort();
        refs.dedup();
        return Ok(OnboardingEvidenceHint {
            state: "evidence_found".to_string(),
            summary: format!("Canonical referral record is {status}."),
            evidence_refs: refs,
            limitations: Vec::new(),
        });
    }
    Ok(OnboardingEvidenceHint {
        state: "pending_canonical_evidence".to_string(),
        summary: "No canonical referral record was found for this visitor session.".to_string(),
        evidence_refs: vec![format!("visitor_session:{}", visitor.id)],
        limitations: vec![
            "Do not qualify referral rewards from scan/session evidence alone.".to_string(),
        ],
    })
}

fn build_reward_hint(
    connection: &Connection,
    visitor: Option<&VisitorRecord>,
    referral: &OnboardingEvidenceHint,
    trial: &OnboardingEvidenceHint,
) -> Result<OnboardingEvidenceHint> {
    let mut source_refs = Vec::new();
    if let Some(visitor) = visitor {
        source_refs.push(format!("visitor_session:{}", visitor.id));
    }
    source_refs.extend(referral.evidence_refs.iter().cloned());
    source_refs.extend(trial.evidence_refs.iter().cloned());
    source_refs.sort();
    source_refs.dedup();

    for source_ref in &source_refs {
        let Some((source_kind, source_id)) = source_ref.split_once(':') else {
            continue;
        };
        if let Some((event_id, state, refs)) = connection
            .query_row(
                "SELECT id, state, evidence_refs_json
                 FROM reward_events
                 WHERE source_kind = ?1 AND source_id = ?2
                 ORDER BY updated_at DESC
                 LIMIT 1",
                params![source_kind, source_id],
                |row| {
                    let evidence_refs_json: String = row.get(2)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        serde_json::from_str::<Vec<String>>(&evidence_refs_json)
                            .unwrap_or_default(),
                    ))
                },
            )
            .optional()?
        {
            let mut evidence_refs = refs;
            evidence_refs.push(format!("reward_event:{event_id}"));
            evidence_refs.sort();
            evidence_refs.dedup();
            return Ok(OnboardingEvidenceHint {
                state: "evidence_found".to_string(),
                summary: format!("Canonical reward event is {state}."),
                evidence_refs,
                limitations: Vec::new(),
            });
        }
    }
    Ok(OnboardingEvidenceHint {
        state: "pending_canonical_evidence".to_string(),
        summary: "No canonical reward event or benefit grant was found for the available referral/trial evidence.".to_string(),
        evidence_refs: source_refs,
        limitations: vec!["Do not imply reward eligibility or balances without reward ledger evidence.".to_string()],
    })
}

fn missing_hint(kind: &str, summary: &str) -> OnboardingEvidenceHint {
    OnboardingEvidenceHint {
        state: "missing".to_string(),
        summary: summary.to_string(),
        evidence_refs: Vec::new(),
        limitations: vec![format!(
            "{kind} status cannot be inferred without canonical evidence."
        )],
    }
}

fn allowed_packet_evidence_ref(reference: &str, viewer: OnboardingHandoffPacketViewer) -> bool {
    let Some((kind, _id)) = reference.split_once(':') else {
        return false;
    };
    match kind {
        "visitor_session"
        | "tracked_entry_point"
        | "conversation"
        | "offer_acceptance"
        | "trial" => true,
        "handoff_item" | "product_request_spine" | "referral_record" | "reward_event" => {
            viewer.can_view_staff_context()
        }
        _ => false,
    }
}

fn member_status_for(
    state: &str,
    handoff: &Option<HandoffRecord>,
    trial: &OnboardingEvidenceHint,
    evidence_refs: &[String],
) -> OnboardingMemberStatus {
    let (status_state, summary, next_step) = if let Some(handoff) = handoff {
        match handoff.delivery_state.as_str() {
            "delivered" | "resolved" => (
                "handoff_completed",
                "Your onboarding handoff has been reviewed.",
                "Watch for the next approved customer-facing step.",
            ),
            "continue_screening" => (
                "screening_required",
                "Your onboarding request needs one more qualifying step before review.",
                "Answer the next onboarding question in the conversation.",
            ),
            _ => (
                "queued",
                "Your onboarding handoff is queued for review.",
                "The team will review the available onboarding context.",
            ),
        }
    } else if state == "incomplete" {
        (
            "incomplete",
            "Your onboarding packet is missing required context.",
            "Continue from the public entry or conversation so Ordo can attach evidence.",
        )
    } else if trial.state == "pending_canonical_evidence" {
        (
            "interest_captured",
            "Your onboarding interest is captured, but trial status has not been confirmed.",
            "Complete the trial or handoff step before Ordo treats it as active.",
        )
    } else {
        (
            "ready",
            "Your onboarding context is ready for review.",
            "The team can review the evidence-backed packet.",
        )
    };
    OnboardingMemberStatus {
        state: status_state.to_string(),
        summary: summary.to_string(),
        next_step: next_step.to_string(),
        evidence_refs: evidence_refs
            .iter()
            .filter(|reference| {
                reference.starts_with("visitor_session:")
                    || reference.starts_with("tracked_entry_point:")
                    || reference.starts_with("conversation:")
                    || reference.starts_with("offer_acceptance:")
                    || reference.starts_with("trial:")
            })
            .cloned()
            .collect(),
    }
}

fn staff_context_for(
    state: &str,
    visitor: Option<&VisitorRecord>,
    tracked_entry: Option<&EntryPointRecord>,
    handoff: Option<&HandoffRecord>,
    conversation: Option<&ConversationRecord>,
    product_request: Option<&ProductRequestRecord>,
    trial: &OnboardingEvidenceHint,
    referral: &OnboardingEvidenceHint,
    reward: &OnboardingEvidenceHint,
    missing: &[String],
    evidence_refs: &[String],
) -> OnboardingStaffContext {
    let mut actionable_items = Vec::new();
    if let Some(handoff) = handoff {
        actionable_items.push(format!(
            "Review handoff {} in state {}.",
            handoff.id, handoff.delivery_state
        ));
        if let Some(action) = &handoff.requested_action {
            actionable_items.push(action.clone());
        }
    }
    if let Some(product_request) = product_request {
        actionable_items.push(format!(
            "Review product request {} ({}) in state {}.",
            product_request.id, product_request.request_kind, product_request.status
        ));
    }
    if trial.state == "pending_canonical_evidence" {
        actionable_items
            .push("Confirm offer acceptance/trial before claiming trial activation.".to_string());
    }
    if referral.state == "pending_canonical_evidence"
        || reward.state == "pending_canonical_evidence"
    {
        actionable_items.push(
            "Treat referral/reward context as pending until canonical reward evidence exists."
                .to_string(),
        );
    }
    if state == "incomplete" {
        actionable_items
            .push("Collect missing onboarding context before owner review.".to_string());
    }
    let summary = match (visitor, tracked_entry, conversation) {
        (Some(visitor), Some(entry), Some(conversation)) => format!(
            "Visitor {} arrived through {} and is linked to conversation {}.",
            visitor.id, entry.slug, conversation.id
        ),
        (Some(visitor), Some(entry), None) => format!(
            "Visitor {} arrived through {}; no conversation evidence is attached.",
            visitor.id, entry.slug
        ),
        _ => "Onboarding handoff packet is incomplete.".to_string(),
    };
    OnboardingStaffContext {
        summary,
        actionable_items,
        missing_prerequisites: missing.to_vec(),
        evidence_refs: evidence_refs.to_vec(),
    }
}

fn handoff_context_for(
    handoff: &HandoffRecord,
    viewer: OnboardingHandoffPacketViewer,
) -> OnboardingHandoffContext {
    let can_view_staff = viewer.can_view_staff_context();
    OnboardingHandoffContext {
        id: handoff.id.clone(),
        source_kind: handoff.source_kind.clone(),
        source_id: handoff.source_id.clone(),
        delivery_state: handoff.delivery_state.clone(),
        owner_decision: if can_view_staff {
            handoff.owner_decision.clone()
        } else {
            None
        },
        reason: if can_view_staff {
            handoff.reason.clone()
        } else {
            None
        },
        requested_action: if can_view_staff {
            handoff.requested_action.clone()
        } else {
            None
        },
        urgency: if can_view_staff {
            handoff.urgency.clone()
        } else {
            None
        },
        next_action_hint: if can_view_staff {
            handoff.next_action_hint.clone()
        } else {
            None
        },
        evidence_refs: handoff
            .evidence_refs
            .iter()
            .filter(|reference| {
                can_view_staff
                    || reference.starts_with("visitor_session:")
                    || reference.starts_with("conversation:")
                    || reference.starts_with("trial:")
                    || reference.starts_with("offer_acceptance:")
            })
            .cloned()
            .collect(),
    }
}

fn handoff_conversation_id(handoff: &HandoffRecord) -> Option<String> {
    json_string(&handoff.request, "conversationId").or_else(|| {
        (handoff.source_kind == "conversation")
            .then(|| handoff.source_id.clone())
            .flatten()
    })
}

fn handoff_visitor_session_id(handoff: &HandoffRecord) -> Option<String> {
    json_string(&handoff.request, "visitorSessionId").or_else(|| {
        (handoff.source_kind == "visitor_session")
            .then(|| handoff.source_id.clone())
            .flatten()
    })
}

fn json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

impl VisitorRecord {
    fn into_context(self) -> OnboardingVisitorContext {
        let evidence_ref = format!("visitor_session:{}", self.id);
        OnboardingVisitorContext {
            id: self.id,
            status: self.status,
            destination_surface: self.destination_surface,
            destination_id: self.destination_id,
            entry_point_id: self.entry_point_id,
            entry_point_slug: self.entry_point_slug,
            created_at: self.created_at,
            last_seen_at: self.last_seen_at,
            evidence_ref,
        }
    }
}

impl EntryPointRecord {
    fn into_context(self) -> OnboardingTrackedEntryContext {
        let evidence_ref = format!("tracked_entry_point:{}", self.id);
        OnboardingTrackedEntryContext {
            id: self.id,
            slug: self.slug,
            label: self.label,
            source_kind: self.source_kind,
            source_label: self.source_label,
            destination_surface: self.destination_surface,
            destination_id: self.destination_id,
            public_path: self.public_path,
            evidence_ref,
        }
    }
}

impl ConversationRecord {
    fn into_context(self) -> OnboardingConversationContext {
        let evidence_ref = format!("conversation:{}", self.id);
        OnboardingConversationContext {
            id: self.id,
            surface: self.surface,
            subject_kind: self.subject_kind,
            subject_id: self.subject_id,
            status: self.status,
            visibility: self.visibility,
            evidence_ref,
        }
    }
}

impl ProductRequestRecord {
    fn into_context(self) -> OnboardingProductRequestContext {
        OnboardingProductRequestContext {
            id: self.id,
            request_kind: self.request_kind,
            title: self.title,
            status: self.status,
            priority: self.priority,
            evidence_refs: self.evidence_refs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::availability::{
        request_strategy_session_handoff, update_operator_presence, ConnectionTrustLevel,
        InterruptionThreshold, OperatorPresenceStatus, OperatorPresenceWriteRequest,
        StrategySessionHandoffRequest,
    };
    use crate::policy::LOCAL_OWNER_ACTOR_ID;
    use crate::schema::init_database;
    use tempfile::TempDir;

    fn setup_db() -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        let connection = Connection::open(&db_path).unwrap();
        connection
            .execute(
                "INSERT INTO tracked_entry_points (
                    id, slug, label, status, source_kind, source_label, destination_surface,
                    destination_id, public_path, qr_payload_json, attribution_json, metadata_json,
                    created_by_actor_id, created_at, updated_at, archived_at
                 ) VALUES (
                    'entry_story', 'story', 'Story entry', 'active', 'qr', 'NYC flyer',
                    'offers', 'offer_story', '/e/story', '{}', '{}', '{}',
                    NULL, '2026-05-15T10:00:00Z', '2026-05-15T10:00:00Z', NULL
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO visitor_sessions (
                    id, entry_point_id, entry_point_slug, status, destination_surface,
                    destination_id, attribution_json, user_agent_hash, created_at, updated_at,
                    last_seen_at, ended_at
                 ) VALUES (
                    'visitor_session_1', 'entry_story', 'story', 'active', 'offers',
                    'offer_story', '{}', NULL, '2026-05-15T10:01:00Z',
                    '2026-05-15T10:02:00Z', '2026-05-15T10:02:00Z', NULL
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO conversations (
                    id, surface, subject_kind, subject_id, connection_id, visitor_session_id,
                    status, visibility, privacy_scope, current_segment_id, last_meaningful_change,
                    unread_count, action_count, summary_json, metadata_json, created_by_actor_id,
                    created_at, updated_at, closed_at, archived_at
                 ) VALUES (
                    'conversation_1', 'public_entry', 'visitor_session', 'visitor_session_1',
                    NULL, 'visitor_session_1', 'open', 'staff', 'support',
                    NULL, '', 0, 0, '{}', '{}', NULL,
                    '2026-05-15T10:03:00Z', '2026-05-15T10:04:00Z', NULL, NULL
                 )",
                [],
            )
            .unwrap();
        update_operator_presence(
            &db_path,
            OperatorPresenceWriteRequest {
                status: OperatorPresenceStatus::Available,
                threshold: InterruptionThreshold::Open,
                status_message: None,
                metadata: None,
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        (temp_dir, db_path)
    }

    fn insert_canonical_trial(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO offer_acceptances (
                    id, offer_id, offer_slug, offer_title, visitor_session_id, entry_point_id,
                    entry_point_slug, attribution_json, acceptance_context_json, status,
                    accepted_at, created_at, updated_at, idempotency_key, access_grant_id
                 ) VALUES (
                    'acceptance_1', 'offer_story', 'story-offer', 'Story Offer',
                    'visitor_session_1', 'entry_story', 'story', '{}', '{}', 'accepted',
                    '2026-05-15T10:05:00Z', '2026-05-15T10:05:00Z',
                    '2026-05-15T10:05:00Z', 'acceptance-key-1', NULL
                 )",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO trials (
                    id, acceptance_id, offer_id, offer_slug, visitor_session_id, status,
                    started_at, trial_ends_at, converted_at, voided_at, expired_at,
                    follow_up_needed_at, decision_evidence_json, created_at, updated_at
                 ) VALUES (
                    'trial_1', 'acceptance_1', 'offer_story', 'story-offer',
                    'visitor_session_1', 'started', '2026-05-15T10:05:00Z',
                    '2026-06-14T10:05:00Z', NULL, NULL, NULL, NULL, '{}',
                    '2026-05-15T10:05:00Z', '2026-05-15T10:05:00Z'
                 )",
                [],
            )
            .unwrap();
    }

    fn request_handoff(db_path: &Path) -> String {
        let (response, _) = request_strategy_session_handoff(
            db_path,
            StrategySessionHandoffRequest {
                conversation_id: Some("conversation_1".to_string()),
                visitor_session_id: Some("visitor_session_1".to_string()),
                trial_id: None,
                access_grant_id: None,
                connection_id: Some("connection_1".to_string()),
                member_actor_id: Some("actor_member_1".to_string()),
                message_excerpt: Some(
                    "I want Keith to help with the launch. private_note: route to staff"
                        .to_string(),
                ),
                context_summary: Some("Founder wants first-user onboarding.".to_string()),
                urgency: Some("high".to_string()),
                connection_trust: Some(ConnectionTrustLevel::Trusted),
                evaluated_at: Some("2026-05-15T10:06:00Z".to_string()),
                evidence_refs: Some(vec![
                    "visitor_session:visitor_session_1".to_string(),
                    "provider_internal:secret".to_string(),
                ]),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
        response.status.request_id
    }

    fn insert_product_request(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO product_request_spine (
                    id, request_kind, source_kind, source_id, object_kind, object_id,
                    title, summary, status, priority, actor_kind, actor_id, connection_id,
                    visibility, due_at, created_at, updated_at, safe_context_json,
                    evidence_refs_json, actions_json, projected_at
                 ) VALUES (
                    'request_1', 'onboarding_handoff', 'conversation', 'conversation_1',
                    'visitor_session', 'visitor_session_1', 'First-user handoff',
                    'Prepare customer onboarding handoff.', 'open', 20,
                    'visitor', 'visitor_session_1', 'connection_1', 'staff', NULL,
                    '2026-05-15T10:06:00Z', '2026-05-15T10:06:00Z', '{}',
                    '[\"conversation:conversation_1\"]', '[]', '2026-05-15T10:06:00Z'
                 )",
                [],
            )
            .unwrap();
    }

    #[test]
    fn staff_packet_joins_public_entry_trial_conversation_and_handoff_evidence() {
        let (_temp_dir, db_path) = setup_db();
        let connection = Connection::open(&db_path).unwrap();
        insert_canonical_trial(&connection);
        insert_product_request(&connection);
        let handoff_id = request_handoff(&db_path);

        let packet = prepare_first_user_onboarding_handoff_packet(
            &db_path,
            FirstUserOnboardingHandoffPacketRequest {
                viewer: OnboardingHandoffPacketViewer::Staff,
                visitor_session_id: Some("visitor_session_1".to_string()),
                handoff_item_id: Some(handoff_id),
                conversation_id: None,
            },
        )
        .unwrap();

        assert_eq!(packet.status, "ready");
        assert!(packet.staff_context.is_some());
        assert_eq!(packet.visitor.as_ref().unwrap().id, "visitor_session_1");
        assert_eq!(packet.tracked_entry.as_ref().unwrap().slug, "story");
        assert_eq!(packet.conversation.as_ref().unwrap().id, "conversation_1");
        assert_eq!(packet.product_request.as_ref().unwrap().id, "request_1");
        assert_eq!(packet.trial.state, "evidence_found");
        assert!(packet.evidence_refs.contains(&"trial:trial_1".to_string()));
        assert!(packet
            .evidence_refs
            .contains(&"tracked_entry_point:entry_story".to_string()));
        assert_eq!(packet.live_provider_required, false);
        assert_eq!(packet.mutates_canonical_state, false);
    }

    #[test]
    fn missing_context_returns_safe_incomplete_packet_without_fake_attribution() {
        let (_temp_dir, db_path) = setup_db();
        let packet = prepare_first_user_onboarding_handoff_packet(
            &db_path,
            FirstUserOnboardingHandoffPacketRequest {
                viewer: OnboardingHandoffPacketViewer::Staff,
                visitor_session_id: Some("missing_session".to_string()),
                handoff_item_id: Some("missing_handoff".to_string()),
                conversation_id: Some("missing_conversation".to_string()),
            },
        )
        .unwrap();

        assert_eq!(packet.status, "incomplete");
        assert_eq!(packet.member_status.state, "incomplete");
        assert!(packet.evidence_refs.is_empty());
        assert!(packet.missing.contains(&"visitor_session".to_string()));
        assert!(packet.missing.contains(&"handoff_item".to_string()));
        assert_eq!(packet.trial.state, "missing");
        assert_eq!(packet.referral.state, "missing");
        assert_eq!(packet.reward.state, "pending_canonical_evidence");
    }

    #[test]
    fn trial_referral_and_reward_hints_stay_pending_until_canonical_records_exist() {
        let (_temp_dir, db_path) = setup_db();
        let connection = Connection::open(&db_path).unwrap();
        insert_product_request(&connection);
        let handoff_id = request_handoff(&db_path);

        let packet = prepare_first_user_onboarding_handoff_packet(
            &db_path,
            FirstUserOnboardingHandoffPacketRequest {
                viewer: OnboardingHandoffPacketViewer::Staff,
                visitor_session_id: Some("visitor_session_1".to_string()),
                handoff_item_id: Some(handoff_id),
                conversation_id: Some("conversation_1".to_string()),
            },
        )
        .unwrap();

        assert_eq!(packet.trial.state, "pending_canonical_evidence");
        assert!(packet.trial.limitations[0].contains("scan"));
        assert_eq!(packet.referral.state, "pending_canonical_evidence");
        assert!(packet.referral.limitations[0].contains("scan/session"));
        assert_eq!(packet.reward.state, "pending_canonical_evidence");
        assert!(packet.reward.limitations[0].contains("ledger"));
    }

    #[test]
    fn member_packet_hides_staff_routing_private_notes_and_provider_internals() {
        let (_temp_dir, db_path) = setup_db();
        let handoff_id = request_handoff(&db_path);

        let packet = prepare_first_user_onboarding_handoff_packet(
            &db_path,
            FirstUserOnboardingHandoffPacketRequest {
                viewer: OnboardingHandoffPacketViewer::Member,
                visitor_session_id: Some("visitor_session_1".to_string()),
                handoff_item_id: Some(handoff_id),
                conversation_id: Some("conversation_1".to_string()),
            },
        )
        .unwrap();
        let serialized = serde_json::to_string(&packet).unwrap();

        assert!(packet.staff_context.is_none());
        assert!(packet.visitor.is_none());
        assert!(packet.tracked_entry.is_none());
        assert!(packet.handoff.is_none());
        assert!(packet.conversation.is_none());
        assert!(packet.product_request.is_none());
        assert!(packet.evidence_refs.is_empty());
        assert!(packet.missing.is_empty());
        for forbidden in [
            "private_note",
            "route to staff",
            "provider_internal",
            "secret",
            "assigneeActorId",
            "memberActorId",
            "rawPrompt",
            "owner-only",
            "product_request_spine",
            "handoff_item",
            "conversation_1",
            "visitor_session_1",
            "entry_story",
        ] {
            assert!(
                !serialized.contains(forbidden),
                "member packet leaked {forbidden}: {serialized}"
            );
        }
    }

    #[test]
    fn packet_generation_is_read_only_and_deterministic() {
        let (_temp_dir, db_path) = setup_db();
        let handoff_id = request_handoff(&db_path);
        let request = FirstUserOnboardingHandoffPacketRequest {
            viewer: OnboardingHandoffPacketViewer::Staff,
            visitor_session_id: Some("visitor_session_1".to_string()),
            handoff_item_id: Some(handoff_id),
            conversation_id: Some("conversation_1".to_string()),
        };
        let before: i64 = Connection::open(&db_path)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM handoff_events", [], |row| row.get(0))
            .unwrap();

        let first =
            prepare_first_user_onboarding_handoff_packet(&db_path, request.clone()).unwrap();
        let second = prepare_first_user_onboarding_handoff_packet(&db_path, request).unwrap();
        let after: i64 = Connection::open(&db_path)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM handoff_events", [], |row| row.get(0))
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(before, after);
    }
}
