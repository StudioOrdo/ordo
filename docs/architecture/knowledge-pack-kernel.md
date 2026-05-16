# Knowledge Pack Kernel

Status: future direction after the OrdoStudio NYC pilot foundations

The current 0.1.9 work should stay focused on the first-user OrdoStudio loop:
founder intake, Story Pack workflow compilation, protected preview, publish
approval, content analytics, generated-content memory review, tracked entry
handoff, offers, trials, referrals, and rewards.

Knowledge Packs are the next larger product shape behind that loop. They
capture the idea that useful knowledge in the AI economy is not just generated
text or a folder of files. It is curated, provenance-backed, human-reviewed
graph knowledge that can teach, validate, and power downstream work.

```text
raw source material
-> source manifest
-> rights and provenance review
-> extraction and normalization
-> spans, claims, entities, aliases, and candidate edges
-> HITL review and consensus
-> promoted graph records
-> versioned Knowledge Pack
-> Story, Studio, Growth, Support, Knowledge, and chat workflows
```

Knowledge Packs are not a near-term replacement for the NYC pilot. They are a
future pack family that should reuse the same Ordo spine: capabilities, jobs,
requests, artifacts, events, graph candidates, review decisions, projections,
Access, Growth, and audit.

## Product Thesis

AI makes raw content cheap. The valuable layer becomes:

- where the knowledge came from;
- whether the source can be reused;
- what claims the source supports;
- which entities were resolved;
- which aliases are accepted;
- which edges were reviewed;
- who approved or rejected the claim;
- what confidence threshold was met;
- where the knowledge was used;
- what feedback or outcomes corrected it.

The paid asset is not an embedding index. The paid asset is a reusable graph
package with provenance and human approval.

Examples:

- `1920s Bauhaus Knowledge Pack`;
- `NASA Apollo Visual Pack`;
- `UAP Declassified Records Pack`;
- `Local Restaurant Launch Pack`;
- `AI Policy Timeline Pack`;
- `Public Domain Space Visuals Pack`;
- `Founder Story Positioning Pack`.

## Boundary Model

Knowledge Packs should sit beside workflow packs, not outside Ordo's trust
model.

- Source connectors acquire or reference source material through registered
  capabilities.
- Source manifests record locator, title, creator, rights assertion, source
  kind, content hash when available, and provenance.
- Artifacts preserve extracted files, normalized text, OCR output, images,
  thumbnails, transcripts, citation packets, review packets, and export bundles.
- Graph candidates propose entities, aliases, claims, and edges.
- Human-in-the-loop requests collect review, disambiguation, rights, and
  promotion decisions.
- Consensus policy decides whether a candidate can be promoted.
- Graph records explain confirmed relationships and point back to sources,
  spans, artifacts, reviewers, events, and policy.
- Projections expose the pack safely to Knowledge, Studio, Growth, Support,
  Member View, and public surfaces according to Access and visibility rules.

Vectors may assist retrieval, clustering, and duplicate detection, but they do
not own truth.

## Knowledge Pack Manifest

A Knowledge Pack manifest should be explicit and versioned:

```json
{
  "id": "knowledge.bauhaus.1920s",
  "name": "1920s Bauhaus Knowledge Pack",
  "version": "0.1.0",
  "status": "draft",
  "sourceManifests": [],
  "rightsRules": [],
  "artifactKinds": [],
  "claimSchemas": [],
  "graphNodeKinds": [],
  "graphEdgeKinds": [],
  "canonicalEdgeVocabulary": [],
  "aliasRules": [],
  "consensusPolicy": {
    "defaultThreshold": "internal_research",
    "reviewerWeights": [],
    "promotionRules": []
  },
  "importCompatibility": [],
  "exportCompatibility": [],
  "visibilityCeiling": "staff",
  "licenseSummary": "mixed_review_required",
  "reviewHistory": [],
  "fixtures": []
}
```

The manifest cannot grant hidden authority. It declares what the pack contains
and what workflows may do with it. Core still owns policy, Access, graph
promotion, artifact visibility, publication, egress, and audit.

## Source Acquisition

The long-term source strategy should prefer protocol-level connectors before
one-off scrapers:

- IIIF for museum, manuscript, map, and book images;
- OAI-PMH for library, archive, arXiv, and open-access metadata harvesting;
- CKAN for data portals;
- MediaWiki/Wikibase for Wikimedia Commons, Wikidata, and related knowledge
  bases;
- trusted source-specific adapters for sources such as Smithsonian, Library of
  Congress, NARA, Internet Archive, NASA, The Met, and similar archives.

Each connector should produce source manifests and acquisition artifacts, not
unreviewed product truth.

## Rights And Provenance

Every acquired or referenced item needs rights and provenance metadata:

- source kind and locator;
- stable source ID when available;
- title, creator, publisher, date, collection, and canonical URL;
- license or rights claim;
- reuse class such as `cc0`, `public_domain`, `us_government`, `open_license`,
  `unknown`, or `restricted`;
- attribution requirement;
- acquisition method and timestamp;
- file hash or source metadata hash when available;
- limitations and warnings.

Unknown or mixed rights should block public derivative use until review.

## HITL Consensus Racetrack

Knowledge curation needs a human-in-the-loop racetrack. Candidates move through
review lanes until they meet, miss, or dispute a configurable confidence
threshold.

```text
candidate extracted
-> machine evidence gathered
-> review packet prepared
-> reviewer decision collected
-> consensus score updated
-> threshold met, disputed, rejected, or needs more evidence
-> promotion, rejection, or hold event
```

Consensus policy should be configurable by pack and use case:

- private exploratory pack: lower threshold;
- internal research pack: medium threshold;
- public education/content pack: higher threshold;
- legal, medical, policy, or financial pack: very high threshold with stronger
  reviewer requirements and source requirements.

Consensus inputs can include:

- reviewer count;
- reviewer role, weight, or reputation;
- source quality;
- agreement or disagreement;
- evidence strength;
- conflict history;
- recency;
- rights and visibility status;
- authority links;
- outcome or correction history.

Review requests should be first-class Ordo requests. Examples:

- resolve entity conflict;
- confirm alias;
- approve claim;
- reject unsupported claim;
- verify source rights;
- mark OCR span unreliable;
- request more evidence;
- approve graph promotion.

## Entity Resolution

Entity resolution should be candidate-first and reviewable. The system may
propose that two mentions refer to the same person, artwork, organization,
place, publication, event, concept, or source, but promotion requires evidence
and policy.

Possible entity conflict actions:

- merge entities;
- keep separate;
- mark alias;
- mark unresolved;
- reject candidate;
- request more evidence;
- create new local entity;
- link to external authority.

Each action should preserve reviewer, evidence refs, source spans, and reason.

## Canonical Edge Vocabulary

A Knowledge Pack should avoid generic `RELATED_TO` graphs. Edges should encode
why one node connects to another.

Wikipedia-style links and archive metadata can seed candidate edges, but the
edge verb is the knowledge product. Links become useful when Ordo maps them to
canonical relationship kinds with evidence.

Example canonical edge verbs:

```text
PERSON TAUGHT_AT SCHOOL
PERSON STUDIED_AT SCHOOL
PERSON MEMBER_OF MOVEMENT
ARTWORK CREATED_BY PERSON
ARTWORK EXHIBITED_AT EXHIBITION
OBJECT PART_OF COLLECTION
DOCUMENT MENTIONS ENTITY
CLAIM SUPPORTED_BY SOURCE_SPAN
CLAIM CONTRADICTED_BY SOURCE_SPAN
EVENT OCCURRED_AT PLACE
WORK INFLUENCED_BY WORK
PUBLICATION PUBLISHED_BY ORGANIZATION
ALIAS REFERS_TO ENTITY
```

Wikipedia-derived links, Wikidata statements, archive metadata, citations, OCR
spans, and LLM extraction may all propose edges. None of those should become
confirmed graph truth without deterministic validation or review policy.

## Bauhaus Example

A `1920s Bauhaus Knowledge Pack` could include:

- source manifests for museum records, books, images, essays, catalogs, and
  archive pages;
- rights metadata for public-domain, CC0, restricted, and unknown assets;
- entities for artists, teachers, students, workshops, schools, buildings,
  objects, exhibitions, publications, movements, and places;
- aliases such as alternate spellings, translated names, object model names,
  and institutional names;
- candidate and confirmed edges such as `TAUGHT_AT`, `STUDIED_AT`,
  `CREATED_BY`, `EXHIBITED_AT`, `LOCATED_IN`, `INFLUENCED_BY`, and
  `PART_OF_COLLECTION`;
- claims with source spans and citation packets;
- unresolved or disputed attributions;
- human review decisions and consensus thresholds;
- public-safe and internal-only projections.

Downstream workflows could use the pack to produce:

- lesson plans;
- scrollytelling pages;
- YouTube scripts;
- visual asset packs;
- lecture decks;
- timelines;
- quizzes or games;
- article series;
- design inspiration boards;
- source-backed image briefs.

## Import And Export

Ordo should eventually import and export Knowledge Packs as graph packages, not
loose graph dumps.

Import flow:

```text
import package
-> validate schema, hashes, and signatures when available
-> inspect rights and visibility
-> map node and edge kinds
-> detect entity and alias conflicts
-> create candidates and review requests
-> promote only approved or trusted-policy records
-> preserve import provenance
```

Export flow:

```text
select pack or scope
-> verify actor permissions
-> redact private fields
-> include source manifests and artifact manifest
-> include claims, source spans, graph records, and review history
-> include rights and visibility rules
-> create signed or hashed bundle
```

Imported records become candidates first unless a policy explicitly trusts the
source, schema, signature, reviewer set, and pack status.

## Ask, Offer, Access, Growth

Knowledge Packs fit Ordo's Ask/Offer loop:

- Offer: unlock or sell a curated knowledge pack, source pack, workflow, or
  review service.
- Ask: request review for entity conflicts, rights checks, claims, aliases,
  citations, or graph promotion.
- Access: grant use of a pack, workflow, source collection, artifact set, or
  export capability.
- Growth: measure which packs, sources, claims, and workflows produced content,
  conversions, referrals, corrections, or business outcomes.
- Rewards: reward useful curation, corrections, reviews, source contributions,
  or maintained packs only when durable evidence qualifies the contribution.

For the NYC pilot, this should remain future direction. The immediate value is
that Story Pack and content analytics should preserve evidence, approval, and
learning in a way that can later grow into Knowledge Pack production.

## Validation Expectations

Future Knowledge Pack work must test:

- manifest validation;
- source manifest parsing;
- rights classification and public-use blocking;
- artifact hash/provenance preservation;
- entity conflict request creation;
- alias resolution decisions;
- canonical edge vocabulary validation;
- candidate-first import;
- consensus threshold behavior;
- graph promotion denial when evidence or review is insufficient;
- public/member redaction;
- export redaction and package integrity;
- deterministic fixtures without live network or live providers by default.
