use anyhow::{ensure, Result};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

use crate::business::{list_business_facts_connection, BusinessFactViewer};
use crate::security::redaction;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSurfacesResponse {
    pub about: AboutReadModel,
    pub offers: OffersReadModel,
    pub asks: AsksReadModel,
    pub feed: FeedReadModel,
    pub readiness: Vec<PublicSurfaceReadiness>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AboutReadModel {
    pub fields: Vec<PublicSurfaceField>,
    pub readiness: PublicSurfaceReadiness,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OffersReadModel {
    pub items: Vec<PublicSurfaceItem>,
    pub readiness: PublicSurfaceReadiness,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AsksReadModel {
    pub items: Vec<PublicSurfaceItem>,
    pub readiness: PublicSurfaceReadiness,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedReadModel {
    pub items: Vec<PublicSurfaceItem>,
    pub readiness: PublicSurfaceReadiness,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSurfaceField {
    pub key: String,
    pub value: Value,
    pub evidence: PublicSurfaceEvidence,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSurfaceItem {
    pub item_id: String,
    pub fields: Vec<PublicSurfaceField>,
    pub readiness: PublicSurfaceReadiness,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicSurfaceEvidence {
    pub fact_id: String,
    pub fact_key: String,
    pub source_kind: String,
    pub source_label: Option<String>,
    pub source_uri: Option<String>,
    pub provenance: Value,
    pub published_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicSurfaceReadiness {
    pub surface: String,
    pub ready: bool,
    pub fact_count: usize,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductSurfaceContract {
    pub home_about: HomeAboutNarrativeReadModel,
    pub offer_intents: Vec<BusinessIntentObject>,
    pub ask_intents: Vec<BusinessIntentObject>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomepageStoryDeckResponse {
    pub profile: HomepageStoryProfile,
    pub deck: HomepageNarrativeDeck,
    pub readiness: PublicSurfaceReadiness,
    pub refresh: HomepageStoryRefreshContract,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomepageStoryProfile {
    pub positioning: String,
    pub audience: Option<String>,
    pub primary_cta: Option<HomepageStoryCta>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomepageNarrativeDeck {
    pub deck_id: String,
    pub version: i64,
    pub surface: String,
    pub slides: Vec<HomepageNarrativeSlide>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomepageNarrativeSlide {
    pub slide_id: String,
    pub section_id: String,
    pub order: i64,
    pub title: String,
    pub body: String,
    pub copy_slots: Vec<HomepageStoryCopySlot>,
    pub cta_refs: Vec<HomepageStoryCta>,
    pub evidence_refs: Vec<String>,
    pub limitations: Vec<String>,
    pub motion_profile: String,
    pub reduced_motion_fallback: String,
    pub image_brief_method: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomepageStoryCopySlot {
    pub slot: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomepageStoryCta {
    pub label: String,
    pub href: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomepageStoryRefreshContract {
    pub manual_refresh_supported: bool,
    pub scheduled_refresh_supported: bool,
    pub image_brief_method: String,
    pub live_provider_required: bool,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeAboutNarrativeReadModel {
    pub billboards: Vec<HomeAboutBillboard>,
    pub readiness: PublicSurfaceReadiness,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeAboutBillboard {
    pub billboard_id: String,
    pub status: String,
    pub headline: String,
    pub body: String,
    pub reduced_motion_fallback: String,
    pub links: Vec<String>,
    pub evidence: Vec<PublicSurfaceEvidence>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessIntentObject {
    pub intent_id: String,
    pub intent_kind: String,
    pub human_readable: String,
    pub machine_readable: Value,
    pub evidence: Vec<PublicSurfaceEvidence>,
    pub readiness: PublicSurfaceReadiness,
}

pub fn public_surfaces(db_path: &Path) -> Result<PublicSurfacesResponse> {
    let connection = Connection::open(db_path)?;
    public_surfaces_connection(&connection)
}

pub fn public_surfaces_connection(connection: &Connection) -> Result<PublicSurfacesResponse> {
    let public_facts =
        list_business_facts_connection(connection, BusinessFactViewer::Public)?.facts;

    public_surfaces_from_public_facts(&public_facts)
}

pub fn public_product_surface_contract_connection(
    connection: &Connection,
) -> Result<ProductSurfaceContract> {
    let public_facts =
        list_business_facts_connection(connection, BusinessFactViewer::Public)?.facts;
    let home_about = home_about_narrative_from_public_facts(&public_facts);
    let surfaces = public_surfaces_from_public_facts(&public_facts)?;
    let offer_intents = business_intents_from_items("offer", &surfaces.offers.items)?;
    let ask_intents = business_intents_from_items("ask", &surfaces.asks.items)?;

    Ok(ProductSurfaceContract {
        home_about,
        offer_intents,
        ask_intents,
    })
}

pub fn public_product_surface_contract(db_path: &Path) -> Result<ProductSurfaceContract> {
    let connection = Connection::open(db_path)?;
    public_product_surface_contract_connection(&connection)
}

pub fn homepage_story_deck(db_path: &Path) -> Result<HomepageStoryDeckResponse> {
    let connection = Connection::open(db_path)?;
    homepage_story_deck_connection(&connection)
}

pub fn homepage_story_deck_connection(
    connection: &Connection,
) -> Result<HomepageStoryDeckResponse> {
    let public_facts =
        list_business_facts_connection(connection, BusinessFactViewer::Public)?.facts;
    let profile_fields = fields_for_prefix(&public_facts, "homepage.profile.");
    let slide_items = grouped_items_for_prefixes(&public_facts, &["homepage.slides."]);
    let evidence_refs = homepage_evidence_refs(connection, &public_facts)?;
    let profile = homepage_story_profile_from_fields(&profile_fields);
    let mut slides = slide_items
        .iter()
        .filter_map(homepage_slide_from_item)
        .collect::<Vec<_>>();
    slides.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.slide_id.cmp(&right.slide_id))
    });

    let mut deck_evidence_refs = stable_unique(
        slides
            .iter()
            .flat_map(|slide| slide.evidence_refs.clone())
            .chain(profile.evidence_refs.clone())
            .chain(evidence_refs)
            .collect(),
    );
    deck_evidence_refs.sort();

    let mut missing = Vec::new();
    if profile.positioning.trim().is_empty() {
        missing.push("published public homepage profile positioning".to_string());
    }
    if slides.is_empty() {
        missing.push("published public homepage slide facts".to_string());
    }
    let ready = missing.is_empty();
    let mut limitations = vec![
        "Deck structure is deterministic and owned by Ordo; AI may only add governed color later."
            .to_string(),
        "Live image generation, live publishing, and analytics claims are not part of this projection."
            .to_string(),
    ];
    if slides.is_empty() {
        limitations.push(
            "No published public homepage slides were found, so Ordo returns readiness gaps instead of invented copy."
                .to_string(),
        );
    }
    if deck_evidence_refs.is_empty() {
        limitations.push(
            "No public-safe artifacts, offers, tracked entry points, or completed briefs were available as supplemental evidence."
                .to_string(),
        );
    }

    Ok(HomepageStoryDeckResponse {
        profile,
        deck: HomepageNarrativeDeck {
            deck_id: "homepage.story.v1".to_string(),
            version: 1,
            surface: "homepage".to_string(),
            slides,
            evidence_refs: deck_evidence_refs,
            limitations: limitations.clone(),
        },
        readiness: PublicSurfaceReadiness {
            surface: "homepage.story".to_string(),
            ready,
            fact_count: public_facts
                .iter()
                .filter(|fact| fact.fact_key.starts_with("homepage."))
                .count(),
            missing,
        },
        refresh: HomepageStoryRefreshContract {
            manual_refresh_supported: true,
            scheduled_refresh_supported: true,
            image_brief_method: "homepage.prepare_image_briefs".to_string(),
            live_provider_required: false,
            limitations: vec![
                "Refresh support is a contract extension point; this function performs no scheduling or publication."
                    .to_string(),
                "Image brief preparation is metadata-only here and does not call a live provider."
                    .to_string(),
            ],
        },
    })
}

pub fn public_surfaces_from_public_facts(
    public_facts: &[crate::business::BusinessFactView],
) -> Result<PublicSurfacesResponse> {
    let about_fields = fields_for_prefix(public_facts, "about.");
    let offer_items = grouped_items_for_prefixes(public_facts, &["offers.", "offer."]);
    let ask_items = grouped_items_for_prefixes(public_facts, &["asks.", "ask.", "wants.", "want."]);
    let feed_items = grouped_items_for_prefixes(public_facts, &["feed."]);

    let about_readiness = readiness("about", about_fields.len());
    let offers_readiness = readiness("offers", offer_items.len());
    let asks_readiness = readiness("asks", ask_items.len());
    let feed_readiness = readiness("feed", feed_items.len());

    Ok(PublicSurfacesResponse {
        about: AboutReadModel {
            fields: about_fields,
            readiness: about_readiness.clone(),
        },
        offers: OffersReadModel {
            items: offer_items,
            readiness: offers_readiness.clone(),
        },
        asks: AsksReadModel {
            items: ask_items,
            readiness: asks_readiness.clone(),
        },
        feed: FeedReadModel {
            items: feed_items,
            readiness: feed_readiness.clone(),
        },
        readiness: vec![
            about_readiness,
            offers_readiness,
            asks_readiness,
            feed_readiness,
        ],
    })
}

pub fn public_about(db_path: &Path) -> Result<AboutReadModel> {
    Ok(public_surfaces(db_path)?.about)
}

pub fn public_offers(db_path: &Path) -> Result<OffersReadModel> {
    Ok(public_surfaces(db_path)?.offers)
}

pub fn public_asks(db_path: &Path) -> Result<AsksReadModel> {
    Ok(public_surfaces(db_path)?.asks)
}

pub fn public_feed(db_path: &Path) -> Result<FeedReadModel> {
    Ok(public_surfaces(db_path)?.feed)
}

fn home_about_narrative_from_public_facts(
    facts: &[crate::business::BusinessFactView],
) -> HomeAboutNarrativeReadModel {
    let billboard_items =
        grouped_items_for_prefixes(facts, &["about.billboards.", "home.billboards."]);
    let billboards = billboard_items
        .iter()
        .filter_map(home_about_billboard_from_item)
        .collect::<Vec<_>>();
    HomeAboutNarrativeReadModel {
        readiness: PublicSurfaceReadiness {
            surface: "home_about".to_string(),
            ready: !billboards.is_empty(),
            fact_count: billboards.len(),
            missing: if billboards.is_empty() {
                vec!["published public home/about billboard facts".to_string()]
            } else {
                Vec::new()
            },
        },
        billboards,
    }
}

fn home_about_billboard_from_item(item: &PublicSurfaceItem) -> Option<HomeAboutBillboard> {
    let status = string_field(item, "status").unwrap_or_else(|| "dynamic".to_string());
    if matches!(status.as_str(), "draft" | "retired") {
        return None;
    }
    Some(HomeAboutBillboard {
        billboard_id: item.item_id.clone(),
        status,
        headline: string_field(item, "headline").unwrap_or_default(),
        body: string_field(item, "body").unwrap_or_default(),
        reduced_motion_fallback: string_field(item, "reducedMotionFallback")
            .or_else(|| string_field(item, "reduced_motion_fallback"))
            .unwrap_or_default(),
        links: links_field(item),
        evidence: item
            .fields
            .iter()
            .map(|field| field.evidence.clone())
            .collect(),
    })
}

fn business_intents_from_items(
    intent_kind: &str,
    items: &[PublicSurfaceItem],
) -> Result<Vec<BusinessIntentObject>> {
    items
        .iter()
        .map(|item| business_intent_from_item(intent_kind, item))
        .collect()
}

fn business_intent_from_item(
    intent_kind: &str,
    item: &PublicSurfaceItem,
) -> Result<BusinessIntentObject> {
    ensure!(
        string_field(item, "title").is_some(),
        "{intent_kind} intent requires a public title"
    );
    ensure!(
        string_field(item, "summary").is_some(),
        "{intent_kind} intent requires a public summary"
    );
    for field in &item.fields {
        ensure!(
            !unsupported_persuasion_claim(field),
            "{intent_kind} intent contains unsupported public persuasion proof: {}",
            field.key
        );
    }

    let mut machine_readable = Map::new();
    for field in &item.fields {
        machine_readable.insert(field.key.clone(), field.value.clone());
    }
    machine_readable.insert(
        "intentKind".to_string(),
        Value::String(intent_kind.to_string()),
    );
    machine_readable.insert(
        "a2aStatus".to_string(),
        Value::String("future_contract".to_string()),
    );
    machine_readable.insert(
        "decisionBoundary".to_string(),
        Value::String("human_or_policy_decides_what_becomes_real".to_string()),
    );

    Ok(BusinessIntentObject {
        intent_id: item.item_id.clone(),
        intent_kind: intent_kind.to_string(),
        human_readable: format!(
            "{}: {}",
            string_field(item, "title").unwrap_or_default(),
            string_field(item, "summary").unwrap_or_default()
        ),
        machine_readable: Value::Object(machine_readable),
        evidence: item
            .fields
            .iter()
            .map(|field| field.evidence.clone())
            .collect(),
        readiness: PublicSurfaceReadiness {
            surface: format!("{intent_kind}.{}", item.item_id),
            ready: true,
            fact_count: item.fields.len(),
            missing: Vec::new(),
        },
    })
}

fn homepage_story_profile_from_fields(fields: &[PublicSurfaceField]) -> HomepageStoryProfile {
    let positioning = fields
        .iter()
        .find(|field| field.key == "positioning")
        .and_then(|field| safe_public_string(&field.value))
        .unwrap_or_default();
    let audience = fields
        .iter()
        .find(|field| field.key == "audience")
        .and_then(|field| safe_public_string(&field.value));
    let primary_cta = homepage_cta_from_fields(fields, "primaryCta");
    let evidence_refs = evidence_refs_for_fields(fields);
    let mut limitations = Vec::new();
    if positioning.is_empty() {
        limitations.push("Missing published public positioning fact.".to_string());
    }
    if primary_cta.is_none() {
        limitations.push("Missing public primary CTA fact.".to_string());
    }

    HomepageStoryProfile {
        positioning,
        audience,
        primary_cta,
        evidence_refs,
        limitations,
    }
}

fn homepage_slide_from_item(item: &PublicSurfaceItem) -> Option<HomepageNarrativeSlide> {
    if item.fields.iter().any(unsupported_persuasion_claim) {
        return None;
    }

    let title = safe_string_field(item, "title").unwrap_or_default();
    let body = safe_string_field(item, "body").unwrap_or_default();
    if title.trim().is_empty() && body.trim().is_empty() {
        return None;
    }
    let section_id = safe_string_field(item, "sectionId")
        .or_else(|| safe_string_field(item, "section_id"))
        .unwrap_or_else(|| item.item_id.clone());
    let order = item
        .fields
        .iter()
        .find(|field| field.key == "order")
        .and_then(|field| field.value.as_i64())
        .unwrap_or(1000);
    let motion_profile = safe_string_field(item, "motionProfile")
        .or_else(|| safe_string_field(item, "motion_profile"))
        .filter(|motion| {
            matches!(
                motion.as_str(),
                "reduced" | "restrained" | "expressive" | "cinematic"
            )
        })
        .unwrap_or_else(|| "restrained".to_string());
    let reduced_motion_fallback = safe_string_field(item, "reducedMotionFallback")
        .or_else(|| safe_string_field(item, "reduced_motion_fallback"))
        .unwrap_or_else(|| {
            if body.is_empty() {
                title.clone()
            } else {
                body.clone()
            }
        });
    let mut limitations = Vec::new();
    if reduced_motion_fallback.trim().is_empty() {
        limitations.push("Reduced-motion fallback was missing.".to_string());
    }
    if title.trim().is_empty() || body.trim().is_empty() {
        limitations.push("Slide has incomplete public copy slots.".to_string());
    }

    Some(HomepageNarrativeSlide {
        slide_id: item.item_id.clone(),
        section_id,
        order,
        title,
        body,
        copy_slots: homepage_copy_slots(item),
        cta_refs: homepage_cta_from_fields(&item.fields, "")
            .into_iter()
            .collect(),
        evidence_refs: evidence_refs_for_fields(&item.fields),
        limitations,
        motion_profile,
        reduced_motion_fallback,
        image_brief_method: Some("homepage.prepare_image_briefs".to_string()),
    })
}

fn homepage_copy_slots(item: &PublicSurfaceItem) -> Vec<HomepageStoryCopySlot> {
    item.fields
        .iter()
        .filter(|field| {
            !matches!(
                field.key.as_str(),
                "body"
                    | "ctaHref"
                    | "ctaLabel"
                    | "imageBriefPrompt"
                    | "motionProfile"
                    | "order"
                    | "reducedMotionFallback"
                    | "sectionId"
                    | "title"
            ) && !unsafe_public_story_field(&field.key)
                && !unsupported_persuasion_claim(field)
        })
        .map(|field| HomepageStoryCopySlot {
            slot: field.key.clone(),
            value: redaction::sanitize_json_strings(field.value.clone()),
        })
        .collect()
}

fn homepage_cta_from_fields(
    fields: &[PublicSurfaceField],
    prefix: &str,
) -> Option<HomepageStoryCta> {
    let label_key = if prefix.is_empty() {
        "ctaLabel".to_string()
    } else {
        format!("{prefix}.label")
    };
    let href_key = if prefix.is_empty() {
        "ctaHref".to_string()
    } else {
        format!("{prefix}.href")
    };
    let label = fields
        .iter()
        .find(|field| field.key == label_key)
        .and_then(|field| safe_public_string(&field.value))?;
    let href = fields
        .iter()
        .find(|field| field.key == href_key)
        .and_then(|field| safe_public_href(&field.value))?;
    Some(HomepageStoryCta {
        label,
        href,
        evidence_refs: evidence_refs_for_fields(
            &fields
                .iter()
                .filter(|field| field.key == label_key || field.key == href_key)
                .cloned()
                .collect::<Vec<_>>(),
        ),
    })
}

fn homepage_evidence_refs(
    connection: &Connection,
    public_facts: &[crate::business::BusinessFactView],
) -> Result<Vec<String>> {
    let mut refs = public_facts
        .iter()
        .filter(|fact| fact.fact_key.starts_with("homepage."))
        .map(|fact| format!("business_fact:{}", fact.id))
        .collect::<Vec<_>>();
    refs.extend(public_offer_refs(public_facts));
    refs.extend(public_artifact_refs(connection)?);
    refs.extend(public_completed_surface_brief_refs(connection)?);
    refs.extend(public_tracked_entry_refs(connection)?);
    Ok(stable_unique(refs))
}

fn public_offer_refs(public_facts: &[crate::business::BusinessFactView]) -> Vec<String> {
    grouped_items_for_prefixes(public_facts, &["offers.", "offer."])
        .into_iter()
        .filter(|item| {
            string_field(item, "title").is_some() && string_field(item, "summary").is_some()
        })
        .map(|item| format!("offer:{}", item.item_id))
        .collect()
}

fn public_artifact_refs(connection: &Connection) -> Result<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT id, summary
         FROM artifacts
         WHERE visibility_ceiling = 'public'
           AND status IN ('published', 'ready', 'available')
           AND (health_status IS NULL OR health_status != 'missing')
         ORDER BY updated_at DESC, id ASC
         LIMIT 12",
    )?;
    let refs = statement
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let summary: String = row.get(1)?;
            Ok((id, summary))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter(|(_, summary)| !redaction::contains_sensitive_text(summary, &[]))
        .map(|(id, _)| format!("artifact:{id}"))
        .collect();
    Ok(refs)
}

fn public_completed_surface_brief_refs(connection: &Connection) -> Result<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT sb.id
         FROM surface_briefs sb
         JOIN artifacts a ON a.id = sb.artifact_id
         WHERE sb.status = 'completed'
           AND a.visibility_ceiling = 'public'
           AND a.status IN ('published', 'ready', 'available')
         ORDER BY sb.generated_at DESC, sb.id ASC
         LIMIT 6",
    )?;
    let refs = statement
        .query_map([], |row| {
            let id: String = row.get(0)?;
            Ok(format!("surface_brief:{id}"))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(refs)
}

fn public_tracked_entry_refs(connection: &Connection) -> Result<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT id
         FROM tracked_entry_points
         WHERE status = 'active'
           AND destination_surface IN ('about', 'offers', 'asks', 'feed')
         ORDER BY updated_at DESC, id ASC
         LIMIT 12",
    )?;
    let refs = statement
        .query_map([], |row| {
            let id: String = row.get(0)?;
            Ok(format!("tracked_entry_point:{id}"))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(refs)
}

fn unsupported_persuasion_claim(field: &PublicSurfaceField) -> bool {
    let key = field.key.to_ascii_lowercase();
    let guarded = [
        "scarcity",
        "urgency",
        "socialproof",
        "social_proof",
        "authority",
        "metric",
        "review",
    ];
    let guarded_key = guarded.iter().any(|guard| key.contains(guard));
    if !guarded_key {
        return false;
    }
    field
        .value
        .as_object()
        .and_then(|object| object.get("evidenceRefs"))
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
}

fn string_field(item: &PublicSurfaceItem, key: &str) -> Option<String> {
    item.fields
        .iter()
        .find(|field| field.key == key)
        .and_then(|field| field.value.as_str().map(ToString::to_string))
}

fn safe_string_field(item: &PublicSurfaceItem, key: &str) -> Option<String> {
    item.fields
        .iter()
        .find(|field| field.key == key)
        .and_then(|field| safe_public_string(&field.value))
}

fn safe_public_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(redaction::redact_public_text)
        .filter(|text| !text.trim().is_empty())
}

fn safe_public_href(value: &Value) -> Option<String> {
    let href = safe_public_string(value)?;
    if href.starts_with('/')
        || href.starts_with('#')
        || href.starts_with("https://")
        || href.starts_with("mailto:")
    {
        Some(href)
    } else {
        None
    }
}

fn evidence_refs_for_fields(fields: &[PublicSurfaceField]) -> Vec<String> {
    stable_unique(
        fields
            .iter()
            .map(|field| format!("business_fact:{}", field.evidence.fact_id))
            .collect(),
    )
}

fn stable_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn unsafe_public_story_field(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    [
        "compiledplanprivate",
        "owneronly",
        "policyinternal",
        "privateartifact",
        "privatepayload",
        "promptinternal",
        "providerinternal",
        "providersecret",
        "rawpolicy",
        "secret",
        "staffrouting",
        "taskprivate",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn links_field(item: &PublicSurfaceItem) -> Vec<String> {
    item.fields
        .iter()
        .find(|field| field.key == "links")
        .and_then(|field| field.value.as_array())
        .map(|links| {
            links
                .iter()
                .filter_map(|link| link.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn fields_for_prefix(
    facts: &[crate::business::BusinessFactView],
    prefix: &str,
) -> Vec<PublicSurfaceField> {
    facts
        .iter()
        .filter_map(|fact| {
            fact.fact_key
                .strip_prefix(prefix)
                .filter(|key| !key.is_empty())
                .map(|key| field_from_fact(key, fact))
        })
        .collect()
}

fn grouped_items_for_prefixes(
    facts: &[crate::business::BusinessFactView],
    prefixes: &[&str],
) -> Vec<PublicSurfaceItem> {
    let mut grouped: BTreeMap<String, Vec<PublicSurfaceField>> = BTreeMap::new();
    for fact in facts {
        let Some(stripped) = strip_any_prefix(&fact.fact_key, prefixes) else {
            continue;
        };
        let (item_id, field_key) = split_item_key(stripped);
        grouped
            .entry(item_id)
            .or_default()
            .push(field_from_fact(&field_key, fact));
    }

    grouped
        .into_iter()
        .map(|(item_id, fields)| PublicSurfaceItem {
            readiness: readiness(&item_id, fields.len()),
            item_id,
            fields,
        })
        .collect()
}

fn strip_any_prefix<'a>(value: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    prefixes
        .iter()
        .find_map(|prefix| value.strip_prefix(prefix))
        .filter(|stripped| !stripped.is_empty())
}

fn split_item_key(value: &str) -> (String, String) {
    let mut parts = value.splitn(2, '.');
    let item_id = parts.next().unwrap_or("item").to_string();
    let field_key = parts.next().unwrap_or("value").to_string();
    (item_id, field_key)
}

fn field_from_fact(key: &str, fact: &crate::business::BusinessFactView) -> PublicSurfaceField {
    PublicSurfaceField {
        key: key.to_string(),
        value: fact.value.clone(),
        evidence: PublicSurfaceEvidence {
            fact_id: fact.id.clone(),
            fact_key: fact.fact_key.clone(),
            source_kind: fact.source_kind.clone(),
            source_label: fact.source_label.clone(),
            source_uri: fact.source_uri.clone(),
            provenance: fact.provenance.clone(),
            published_at: fact.published_at.clone(),
            updated_at: fact.updated_at.clone(),
        },
    }
}

fn readiness(surface: &str, fact_count: usize) -> PublicSurfaceReadiness {
    PublicSurfaceReadiness {
        surface: surface.to_string(),
        ready: fact_count > 0,
        fact_count,
        missing: if fact_count > 0 {
            Vec::new()
        } else {
            vec!["published public facts".to_string()]
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::business::{
        create_business_fact, BusinessFactVisibility, BusinessFactWriteRequest, PublicationState,
    };
    use crate::policy::LOCAL_OWNER_ACTOR_ID;
    use crate::schema::init_database;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn public_surfaces_exclude_non_public_and_unpublished_facts() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        for (fact_key, visibility, publication_state) in [
            (
                "about.tagline",
                BusinessFactVisibility::Public,
                PublicationState::Published,
            ),
            (
                "about.draft",
                BusinessFactVisibility::Public,
                PublicationState::Draft,
            ),
            (
                "about.archived",
                BusinessFactVisibility::Public,
                PublicationState::Archived,
            ),
            (
                "about.revoked",
                BusinessFactVisibility::Public,
                PublicationState::Revoked,
            ),
            (
                "about.authenticated",
                BusinessFactVisibility::Authenticated,
                PublicationState::Published,
            ),
            (
                "about.staff",
                BusinessFactVisibility::Staff,
                PublicationState::Published,
            ),
            (
                "about.owner",
                BusinessFactVisibility::Owner,
                PublicationState::Published,
            ),
        ] {
            insert_fact(
                &db_path,
                fact_key,
                visibility,
                publication_state,
                json!(fact_key),
            );
        }

        let surfaces = public_surfaces(&db_path).unwrap();

        assert_eq!(surfaces.about.fields.len(), 1);
        assert_eq!(surfaces.about.fields[0].key, "tagline");
        assert_eq!(surfaces.about.fields[0].value, json!("about.tagline"));
        assert!(surfaces.about.readiness.ready);
    }

    #[test]
    fn public_surfaces_group_offers_asks_and_feed_items() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        insert_public_fact(&db_path, "offers.consulting.title", json!("Consulting"));
        insert_public_fact(&db_path, "offers.consulting.summary", json!("Focused help"));
        insert_public_fact(&db_path, "asks.partners.title", json!("Partners"));
        insert_public_fact(&db_path, "feed.launch.title", json!("Launch note"));

        let surfaces = public_surfaces(&db_path).unwrap();

        assert_eq!(surfaces.offers.items.len(), 1);
        assert_eq!(surfaces.offers.items[0].item_id, "consulting");
        assert_eq!(surfaces.offers.items[0].fields.len(), 2);
        assert_eq!(surfaces.asks.items[0].item_id, "partners");
        assert_eq!(surfaces.feed.items[0].item_id, "launch");
    }

    #[test]
    fn public_surfaces_return_explicit_missing_readiness() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        let surfaces = public_surfaces(&db_path).unwrap();

        assert!(!surfaces.about.readiness.ready);
        assert_eq!(surfaces.about.readiness.fact_count, 0);
        assert_eq!(
            surfaces.about.readiness.missing,
            vec!["published public facts".to_string()]
        );
        assert_eq!(surfaces.readiness.len(), 4);
        assert!(surfaces.readiness.iter().all(|readiness| !readiness.ready));
    }

    #[test]
    fn product_surface_contract_builds_home_about_and_intents_from_public_facts() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        insert_public_fact(&db_path, "about.billboards.hero.status", json!("published"));
        insert_public_fact(
            &db_path,
            "about.billboards.hero.headline",
            json!("Proof-backed client operations"),
        );
        insert_public_fact(
            &db_path,
            "about.billboards.hero.body",
            json!("Ordo turns relationship evidence into a usable next action."),
        );
        insert_public_fact(
            &db_path,
            "about.billboards.hero.reducedMotionFallback",
            json!("Static proof-backed narrative with the same claims."),
        );
        insert_public_fact(
            &db_path,
            "about.billboards.hero.links",
            json!(["/offers/starter", "/asks/referrals", "/chat"]),
        );
        insert_public_fact(&db_path, "offers.starter.title", json!("Starter Sprint"));
        insert_public_fact(
            &db_path,
            "offers.starter.summary",
            json!("A focused implementation sprint."),
        );
        insert_public_fact(
            &db_path,
            "offers.starter.terms",
            json!({"approvalRequired": true, "startPath": "/chat"}),
        );
        insert_public_fact(&db_path, "asks.referrals.title", json!("Referral fit"));
        insert_public_fact(
            &db_path,
            "asks.referrals.summary",
            json!("Introduce teams that need proof-backed operations."),
        );

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let contract = public_product_surface_contract_connection(&connection).unwrap();

        assert_eq!(contract.home_about.billboards.len(), 1);
        assert_eq!(
            contract.home_about.billboards[0].reduced_motion_fallback,
            "Static proof-backed narrative with the same claims."
        );
        assert_eq!(contract.offer_intents.len(), 1);
        assert_eq!(contract.offer_intents[0].intent_kind, "offer");
        assert_eq!(
            contract.offer_intents[0].machine_readable["decisionBoundary"],
            "human_or_policy_decides_what_becomes_real"
        );
        assert_eq!(contract.ask_intents.len(), 1);
    }

    #[test]
    fn product_surface_contract_rejects_unsupported_public_persuasion_proof() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        insert_public_fact(&db_path, "offers.rush.title", json!("Rush Offer"));
        insert_public_fact(&db_path, "offers.rush.summary", json!("Act now."));
        insert_public_fact(&db_path, "offers.rush.scarcity", json!("Only two spots."));

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let error = public_product_surface_contract_connection(&connection).unwrap_err();

        assert!(error
            .to_string()
            .contains("unsupported public persuasion proof"));
    }

    #[test]
    fn homepage_story_deck_projects_public_story_inputs_in_stable_order() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        insert_public_fact(
            &db_path,
            "homepage.profile.positioning",
            json!("A practical answer to enshittification."),
        );
        insert_public_fact(
            &db_path,
            "homepage.profile.primaryCta.label",
            json!("Start with Ordo"),
        );
        insert_public_fact(&db_path, "homepage.profile.primaryCta.href", json!("/chat"));
        insert_public_fact(&db_path, "homepage.slides.proof.order", json!(20));
        insert_public_fact(&db_path, "homepage.slides.proof.sectionId", json!("proof"));
        insert_public_fact(
            &db_path,
            "homepage.slides.proof.title",
            json!("Evidence beats platform fog"),
        );
        insert_public_fact(
            &db_path,
            "homepage.slides.proof.body",
            json!("Ordo keeps the operating record local and reviewable."),
        );
        insert_public_fact(
            &db_path,
            "homepage.slides.proof.reducedMotionFallback",
            json!("A static evidence-led story."),
        );
        insert_public_fact(
            &db_path,
            "homepage.slides.proof.ctaLabel",
            json!("See the pilot"),
        );
        insert_public_fact(&db_path, "homepage.slides.proof.ctaHref", json!("/e/nyc"));
        insert_public_fact(&db_path, "homepage.slides.hero.order", json!(10));
        insert_public_fact(&db_path, "homepage.slides.hero.title", json!("Studio Ordo"));
        insert_public_fact(
            &db_path,
            "homepage.slides.hero.body",
            json!("A local-first AI operating appliance for solopreneurs."),
        );
        insert_public_fact(
            &db_path,
            "homepage.slides.hero.motionProfile",
            json!("cinematic"),
        );
        insert_public_fact(&db_path, "offers.trial.title", json!("30-day hosted trial"));
        insert_public_fact(
            &db_path,
            "offers.trial.summary",
            json!("Try Ordo with clear experimental limits."),
        );

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        insert_public_artifact(
            &connection,
            "artifact_storyboard",
            "Public storyboard proof",
        );
        insert_home_entry_point(&connection, "entry_nyc", "nyc", "NYC meetup QR");

        let deck = homepage_story_deck_connection(&connection).unwrap();

        assert!(deck.readiness.ready);
        assert_eq!(deck.deck.deck_id, "homepage.story.v1");
        assert_eq!(
            deck.profile.positioning,
            "A practical answer to enshittification."
        );
        assert_eq!(deck.deck.slides.len(), 2);
        assert_eq!(deck.deck.slides[0].slide_id, "hero");
        assert_eq!(deck.deck.slides[1].slide_id, "proof");
        assert_eq!(deck.deck.slides[1].cta_refs[0].href, "/e/nyc");
        assert!(deck
            .deck
            .slides
            .iter()
            .all(|slide| slide.image_brief_method.as_deref()
                == Some("homepage.prepare_image_briefs")));
        assert!(deck
            .deck
            .evidence_refs
            .iter()
            .any(|reference| reference == "artifact:artifact_storyboard"));
        assert!(deck
            .deck
            .evidence_refs
            .iter()
            .any(|reference| reference == "tracked_entry_point:entry_nyc"));
        assert!(deck
            .deck
            .evidence_refs
            .iter()
            .any(|reference| reference.starts_with("offer:trial")));

        let rebuilt = homepage_story_deck_connection(&connection).unwrap();
        assert_eq!(
            serde_json::to_value(&deck).unwrap(),
            serde_json::to_value(&rebuilt).unwrap()
        );
    }

    #[test]
    fn homepage_story_deck_excludes_private_internal_and_unsupported_inputs() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();

        insert_public_fact(
            &db_path,
            "homepage.profile.positioning",
            json!("Local-first public story."),
        );
        insert_public_fact(&db_path, "homepage.slides.hero.order", json!(1));
        insert_public_fact(&db_path, "homepage.slides.hero.title", json!("Safe title"));
        insert_public_fact(
            &db_path,
            "homepage.slides.hero.body",
            json!("Call us at 555-555-5555 with sk-live-secret."),
        );
        insert_public_fact(
            &db_path,
            "homepage.slides.hero.promptInternal",
            json!("never show this prompt note"),
        );
        insert_public_fact(
            &db_path,
            "homepage.slides.hero.providerSecret",
            json!("provider secret should not appear"),
        );
        insert_fact(
            &db_path,
            "homepage.slides.private.title",
            BusinessFactVisibility::Owner,
            PublicationState::Published,
            json!("owner-only slide"),
        );
        insert_fact(
            &db_path,
            "homepage.slides.draft.title",
            BusinessFactVisibility::Public,
            PublicationState::Draft,
            json!("draft slide"),
        );
        let connection = rusqlite::Connection::open(&db_path).unwrap();
        insert_private_artifact(
            &connection,
            "artifact_private",
            "Project Orchid private artifact",
        );

        let deck = homepage_story_deck_connection(&connection).unwrap();
        let serialized = serde_json::to_string(&deck).unwrap();

        assert!(deck.readiness.ready);
        assert_eq!(deck.deck.slides.len(), 1);
        assert!(!serialized.contains("owner-only slide"));
        assert!(!serialized.contains("draft slide"));
        assert!(!serialized.contains("promptInternal"));
        assert!(!serialized.contains("providerSecret"));
        assert!(!serialized.contains("never show this prompt note"));
        assert!(!serialized.contains("provider secret should not appear"));
        assert!(!serialized.contains("Project Orchid"));
        assert!(!serialized.contains("555-555-5555"));
        assert!(!serialized.contains("sk-live-secret"));
        assert!(serialized.contains("[REDACTED_PHONE]"));
        assert!(serialized.contains("[REDACTED_SECRET]"));
    }

    #[test]
    fn homepage_story_deck_reports_readiness_gaps_without_inventing_copy() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        init_database(&db_path).unwrap();
        insert_public_fact(
            &db_path,
            "homepage.profile.positioning",
            json!("Public story exists but has no slides yet."),
        );

        let connection = rusqlite::Connection::open(&db_path).unwrap();
        let deck = homepage_story_deck_connection(&connection).unwrap();

        assert!(!deck.readiness.ready);
        assert!(deck.deck.slides.is_empty());
        assert!(deck
            .readiness
            .missing
            .contains(&"published public homepage slide facts".to_string()));
        assert!(deck
            .deck
            .limitations
            .iter()
            .any(|limitation| { limitation.contains("No published public homepage slides") }));
        assert_eq!(
            deck.profile.positioning,
            "Public story exists but has no slides yet."
        );
    }

    fn insert_public_fact(db_path: &Path, fact_key: &str, value: serde_json::Value) {
        insert_fact(
            db_path,
            fact_key,
            BusinessFactVisibility::Public,
            PublicationState::Published,
            value,
        );
    }

    fn insert_fact(
        db_path: &Path,
        fact_key: &str,
        visibility: BusinessFactVisibility,
        publication_state: PublicationState,
        value: serde_json::Value,
    ) {
        create_business_fact(
            db_path,
            BusinessFactWriteRequest {
                subject_type: None,
                subject_id: None,
                fact_key: fact_key.to_string(),
                value,
                source_kind: Some("operator".to_string()),
                source_label: Some("Public surface test".to_string()),
                source_uri: None,
                provenance: Some(json!({ "test": true })),
                visibility: Some(visibility),
                publication_state: Some(publication_state),
            },
            Some(LOCAL_OWNER_ACTOR_ID),
        )
        .unwrap();
    }

    fn insert_public_artifact(connection: &rusqlite::Connection, id: &str, summary: &str) {
        connection
            .execute(
                "INSERT INTO artifacts (
                    id, artifact_kind, title, status, visibility_ceiling, summary,
                    source_kind, source_id, evidence_refs_json, provenance_json, content_hash,
                    storage_uri, health_status, created_at, updated_at
                 ) VALUES (?1, 'homepage.storyboard', 'Public storyboard', 'published',
                    'public', ?2, 'homepage', 'story', '[\"business_fact:homepage\"]',
                    '{\"test\":true}', 'sha256:public-story', NULL, 'available', 'now', 'now')",
                rusqlite::params![id, summary],
            )
            .unwrap();
    }

    fn insert_private_artifact(connection: &rusqlite::Connection, id: &str, summary: &str) {
        connection
            .execute(
                "INSERT INTO artifacts (
                    id, artifact_kind, title, status, visibility_ceiling, summary,
                    source_kind, source_id, evidence_refs_json, provenance_json, content_hash,
                    storage_uri, health_status, created_at, updated_at
                 ) VALUES (?1, 'homepage.storyboard', 'Private storyboard', 'published',
                    'owner', ?2, 'homepage', 'story', '[\"business_fact:homepage\"]',
                    '{\"test\":true}', 'sha256:private-story', NULL, 'available', 'now', 'now')",
                rusqlite::params![id, summary],
            )
            .unwrap();
    }

    fn insert_home_entry_point(
        connection: &rusqlite::Connection,
        id: &str,
        slug: &str,
        label: &str,
    ) {
        connection
            .execute(
                "INSERT INTO tracked_entry_points (
                    id, slug, label, status, source_kind, source_label, destination_surface,
                    destination_id, public_path, qr_payload_json, attribution_json, metadata_json,
                    created_at, updated_at
                 ) VALUES (?1, ?2, ?3, 'active', 'event', 'NYC meetup', 'about',
                    NULL, ?4, '{\"kind\":\"ordo.tracked_entry_point\"}', '{}', '{}', 'now', 'now')",
                rusqlite::params![id, slug, label, format!("/e/{slug}")],
            )
            .unwrap();
    }
}
