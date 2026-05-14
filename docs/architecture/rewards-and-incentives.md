# Rewards And Incentives

Status: target architecture direction as of 2026-05-13

Current canon:

- [Current Product Canon](../business/current-product-canon.md)
- [Target Architecture Plan](target-architecture-plan.md)
- [Tracked Entry Points And Visitor Sessions](tracked-entry-points.md)
- [Offers And Trial Lifecycle](offers-and-trials.md)

Rewards should be a reusable Growth primitive, not a hardcoded OrdoStudio trial
promotion.

The generic loop is:

```text
tracked action
-> qualifying rule
-> reward event
-> reward ledger entry
-> benefit grant
-> access or balance update
-> expiration, reversal, or renewal
```

Growth decides what was earned. Access enforces what the user can use.

## Product Uses

The same subsystem should support:

- qualified referral grants seven extra hosted days;
- accepted feedback grants extra hosted days;
- community QA earns usage credits;
- affiliate referrals earn credit, rewards, or commission evidence;
- onboarding completion unlocks a pack;
- publishing milestones grant render minutes;
- course participation unlocks lesson access;
- future community leaderboard and prize programs.

The first OrdoStudio pilot policy should stay simple:

```text
base hosted trial: 30 days
active pilot capacity: 10 hosted trial spots
qualified referral: +7 hosted days
accepted feedback: +N hosted days, policy-defined
pilot extension cap: policy-defined
```

## Durable Objects

Target durable objects:

- `reward_program`: named incentive program with owner, status, visibility,
  terms, start/end windows, and abuse policy.
- `reward_rule`: qualification rule bound to events such as referral,
  feedback, offer acceptance, trial activation, artifact publication, or
  verified outcome.
- `reward_event`: candidate earning event with evidence refs and current
  qualification state.
- `reward_ledger_entry`: append-only reward accounting row.
- `benefit_grant`: granted benefit such as hosted days, render minutes, pack
  access, consulting credit, usage quota, or leaderboard points.
- `benefit_balance`: derived balance by actor, access record, offer, or hosted
  instance.
- `qualification_review`: optional human or automated review of feedback,
  referral quality, fraud, spam, duplicate accounts, or policy exceptions.
- `leaderboard_entry`: opt-in public or community-scoped projection, never the
  source of truth.

The ledger is canonical. Balances and leaderboard rows are projections.

## Reward States

Reward events should support:

- `pending`
- `qualified`
- `granted`
- `rejected`
- `reversed`
- `expired`
- `capped`
- `needs_review`

Benefits should support:

- `active`
- `scheduled`
- `consumed`
- `expired`
- `revoked`
- `reversed`

Do not silently mutate counters. Record the event, decision, grant, and any
reversal.

## Evidence Rules

No reward without evidence.

Referral credit should require a qualified downstream event, not just a scan.
For the pilot, the first clean qualification event is trial activation. Later
programs may use conversion, payment, verified attendance, accepted review, or
manual approval.

Feedback credit should require useful feedback, not any text submission. The
qualification record should cite the feedback submission, reviewer or automated
rubric, accepted tags, and granted benefit.

Each reward ledger entry should cite:

- actor or connection receiving the reward;
- reward program and rule;
- source event, such as tracked entry point, visitor session, offer acceptance,
  trial, feedback submission, artifact, publication, or outcome;
- policy decision;
- benefit grant id when granted;
- expiration, cap, or reversal reason when applicable.

## Access Boundary

Rewards should grant benefits through Access. They should not bypass
entitlement, hosted-instance, capability, usage-limit, or pack policy.

Examples:

- seven referral days extend a hosted trial access grant;
- accepted feedback grants hosted days up to the pilot cap;
- leaderboard points do not grant private data access;
- prize eligibility creates a review item before any external reward is sent.

Access remains the enforcement layer. Growth remains the evidence and reward
decision layer.

## Leaderboards And Prizes

Leaderboards should be opt-in and pseudonymous by default. A leaderboard is a
projection of reward ledger evidence, not the ledger itself.

Prize programs add operational and legal risk. Treat prizes as a later program
type with explicit terms, eligibility rules, abuse review, location limits,
tax/reporting notes where relevant, and manual approval before fulfillment.

## Abuse And Reversal

The system should expect abuse:

- self-referrals;
- duplicate accounts;
- referral spam;
- low-value feedback spam;
- fake scans;
- scripted signup attempts;
- leaderboard manipulation.

Every reward program needs a cap, review path, and reversal mechanism.

The default posture is:

```text
reward is pending until qualified
benefit is granted only after qualification
benefit can be reversed when evidence is invalidated
```

## Read Models

Useful projections:

- Growth reward program summary.
- Member reward balance.
- Referral activity and qualified conversions.
- Feedback credit status.
- Hosted trial expiration and extension timeline.
- Community leaderboard, opt-in only.
- Owner abuse/review queue.

These projections should be rebuilt from reward events, ledger entries, benefit
grants, and access state.

## Non-Goals

- No cash payout automation in the first version.
- No public leaderboard by default.
- No reward for scans alone.
- No hidden tracking beyond tracked-entry-point policy.
- No reward grants that bypass Access.
- No prize fulfillment without explicit owner approval.
