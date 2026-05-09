use anyhow::{ensure, Result};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

use crate::business::{list_business_facts_connection, BusinessFactViewer};

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
}
