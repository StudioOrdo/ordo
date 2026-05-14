# Studio Ordo Hosted Appliance MVP

Status: active landing target, not fully implemented

Studio Ordo is the hosted control plane for AGPL Ordo appliances. The near-term
MVP is not to finish every public Ordo product surface. The MVP is to prove that
Studio Ordo can invite, provision, supervise, brief, back up, and gracefully
close hosted trial Ordos.

## Product Shape

Studio Ordo remains the public business, support, and control-plane site. A
trial user's own hosted Ordo can start as an under-construction public site
while Ordo interviews the owner, creates first artifacts, and prepares the
business surface.

The first loop is:

```text
meet Keith
-> scan QR
-> ask Ordo for a trial
-> accept if capacity exists, otherwise join waitlist
-> provision a hosted Ordo appliance
-> onboard while the trial site says under construction
-> create conversation rollups and Growth briefs
-> ask for feedback or referrals
-> extend, convert, or close out
-> email final backup and return invitation
-> decommission only after export evidence exists
```

## What Exists Now

- The AGPL appliance repository and one-image Docker runtime exist.
- The Rust daemon owns SQLite migrations, jobs, events, artifacts, backups,
  restore preflight, scheduler foundations, policy, resource grants, offers,
  trials, hosted trial slot capacity, waitlist entries, and reset guards.
- Offer acceptance can create a trial, hosted trial slot, and scoped
  `hosted_trial/use` grant when capacity exists.
- If active hosted capacity is full, acceptance records waitlist evidence.
- Backup jobs produce manifest/checksum-backed artifacts.
- Referral records, business outcomes, and attribution evidence exist as
  backend primitives.
- Conversation, LLM gateway, eval, artifact, and brief foundations exist.

## MVP Next

The hosted appliance MVP adds the missing operational bridge around the existing
backend primitives:

- hosted Ordo instance records;
- Docker/Traefik provisioning and route assignment;
- per-trial data and media volumes;
- commissioning and decommissioning process templates;
- transactional email with notification attempts and receipts;
- reminder schedules for onboarding, feedback, expiration, and backup;
- scheduled conversation rollups into Growth artifacts;
- trial closeout that generates a final backup and return invitation;
- owner/staff management surfaces for capacity, slots, waitlist, and lifecycle
  evidence.

## Platform Later

Later platform features should build on the MVP rather than replace it:

- reward ledger, benefit grants, and usage quotas for offers and asks;
- real affiliate dashboard and payout or credit approval flow;
- governed A2A networking between Ordos;
- Studio Ordo Prime as the support, feedback, directory, and premium job node;
- premium media tools such as story-to-video, voice, editing, and production
  jobs funded by credits or tokens;
- object storage for larger media workloads;
- directory and discovery of opted-in Ordos.

## Ideal Product Shape

Ordo is a local-first business appliance for one-person businesses. Studio Ordo
adds managed hosting, a support network, premium production capabilities, and a
directory layer without turning the appliance into lock-in.

The strategic promise is:

```text
AGPL appliance ownership
+ managed convenience
+ portable backups
+ governed network
+ premium production tools
= more operating leverage without surrendering business memory
```

Studio Ordo should win by being the easiest and most useful place to run,
support, and grow an Ordo, not by making it hard to leave.