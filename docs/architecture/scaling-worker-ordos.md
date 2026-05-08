# Scaling With Worker Ordos

Status: Future direction

This document describes a scaling direction, not shipped behavior.

The principle is:

```text
One Ordo owns truth. Many Ordos perform work. Artifacts flow back.
```

## Problem

Some Ordo use cases may have thousands of users but still fit the appliance
model. A course might have 5,000 students. A creator might have a large audience.
A CPA practice might have many client document workflows. These do not require
Ordo to become a single large centralized SaaS database.

The scaling problem is usually concurrent work:

- tutoring sessions;
- retrieval and answer generation;
- document review;
- analytics jobs;
- issue triage;
- content generation.

The canonical truth can stay small and governed while execution fans out.

## Home Ordo

The Home Ordo owns canonical state:

- official corpus and public content;
- RBAC, roster, clients, or audience membership;
- owner/admin dashboards;
- global analytics and briefs;
- offers and published artifacts;
- final student, client, or customer records;
- policy and capability definitions.

The Home Ordo authorizes work, assigns work, receives artifacts, and derives
canonical briefs. It should not have to run every heavy interactive job itself.

## Worker Ordos

Worker Ordos perform bounded execution:

- student tutor conversations;
- retrieval over approved corpus slices;
- assignment or document feedback;
- session summarization;
- misconception or issue detection;
- diagnostic/support work;
- batch analytics.

Workers return artifacts. They should not directly rewrite canonical Home Ordo
truth.

## Router / Gateway

A router can assign jobs or sessions by:

- cohort;
- course section;
- user hash;
- current load;
- assignment/module;
- privacy boundary;
- customer or institution boundary.

The user should still experience one Ordo. Routing is an execution detail.

## Example: 5,000-Student Course

1. Student signs into the course Home Ordo.
2. Home Ordo checks enrollment and content access.
3. Student starts a tutoring session or assignment review.
4. Home Ordo creates a scoped job request.
5. Router assigns the request to a Worker Ordo.
6. Worker Ordo runs the bounded chat/retrieval/feedback job.
7. Worker returns artifacts with provenance, evidence, limitations, and scope.
8. Home Ordo stores canonical student-facing and instructor-facing artifacts.
9. Scheduled Home Ordo briefs summarize cohort patterns for the instructor.

Instructor briefs might answer:

- What concepts are students missing?
- Which lecture sections are causing confusion?
- Which students need support?
- What content should be revised?

## Artifact Rule

Workers should return structured artifacts, not uncontrolled state mutations.

A worker result should identify:

- Home Ordo;
- Worker Ordo;
- job id;
- actor or cohort scope;
- allowed corpus or resource scope;
- output summary;
- evidence;
- limitations;
- created timestamp.

The Home Ordo validates the artifact before accepting it into canonical state.

## Privacy And Access

This direction depends on RBAC and resource-scoped retrieval. Workers should
receive only the actor context and corpus slices needed for their job.

Before this ships, Ordo needs:

- durable actors, roles, and memberships;
- resource visibility;
- per-user private records;
- access-aware retrieval;
- operator-visible audit trails.

## Generalization

The same pattern can support:

- CPA firms: workers prepare document summaries and draft packets while the
  firm Ordo owns client truth and final review.
- Creators: workers handle audience chats and content analysis while the creator
  Ordo owns brand, offers, and published artifacts.
- Agencies: workers run campaign, research, or reporting jobs while the agency
  Ordo owns clients and approvals.
- Support: trial Ordos prepare local issue reports; a maintainer Ordo receives
  approved support artifacts in the future.

## Relationship To A2A

Worker Ordos are related to future Agent2Agent work, but this repo does not
currently implement A2A networking or Worker Ordo orchestration.

The first likely A2A wedge is support issue reports:

```text
trial Ordo -> approved issue artifact -> maintainer Ordo -> project brief
```

That should come after local reports, RBAC, artifact contracts, and
operator-confirmed egress are in place.
