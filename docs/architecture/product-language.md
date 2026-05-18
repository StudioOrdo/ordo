# Product Language

Status: required guidance for member-facing UI and demo copy

Ordo's internal architecture needs precise terms. The product should not expose
most of those terms to normal members.

A busy non-technical person should be able to use Ordo without learning how
Ordo is built. The UI should sound human, calm, and concrete.

## Language Rule

Use architecture words in code, schemas, tests, and implementation docs.

Use plain words in the product.

The member-facing UI should answer:

```text
What is this?
Why am I seeing it?
What do I do?
Is it safe?
What happens next?
Can I undo or ask for changes?
What evidence or receipt exists?
```

If a screen requires the user to understand Ordo internals before taking the
next step, the screen is not ready.

## Internal Terms And Product Words

| Internal term | Product language |
| --- | --- |
| Capability | What you can do |
| Capability grant | Access you have |
| Pack | What Ordo is using |
| Product pack | Work area or service |
| Workflow | Plan |
| Workflow compilation | Plan check |
| Workflow state | What Ordo is working on |
| DAG | Steps |
| Job | Work run |
| Task | Step |
| Artifact | Draft, file, result, or receipt |
| Event | What happened |
| Evidence ref | Source or receipt |
| Graph | Connections |
| Graph truth | Confirmed connection |
| Candidate | Needs review |
| Promotion | Save as trusted |
| Memory promotion readiness | Ready for review |
| Generated-content memory candidate | Something Ordo may remember |
| DecisionQueueItem / WorkItem | Behind-the-scenes routing |
| Handoff queue projection | People waiting for help |
| Provider egress | Send outside Ordo |
| Policy | Safety rules |
| Provider | AI service |
| Prompt | Instructions to AI |
| Route contract | Page safety rule |
| Degraded | Not fully ready |

These translations are not exact schema replacements. They are copy guidance.
The underlying implementation should keep precise names.

## Preferred Member Vocabulary

Use these words often:

```text
For you
Needs your attention
Requests
Offers
People
Drafts
Ready to review
Waiting on you
Waiting on Ordo
Done
Receipts
Sources
Why this matters
Safe to approve
Not ready yet
Ask for changes
View receipt
```

Use these only when the user needs more detail:

```text
source
review
approval
access
history
connection
private
public
staff-only
```

Avoid these in member-facing first screens:

```text
daemon
DAG
artifact
capability
pack
candidate
promotion
graph truth
evidence refs
provider egress
route contract
decision queue
workflow compilation
memory readiness packet
```

If one of these terms must appear for an operator, place it below a plain
explanation and label it as technical detail.

## Examples

### Story Intake

Do not say:

```text
Workflow compilation evidence is available.
```

Say:

```text
Story plan is ready.
Ordo checked the pieces needed to build this story. Nothing has been published.
```

Technical detail can appear below:

```text
Plan check: studio.story.scrollytelling_homepage
```

### Studio Preview

Do not say:

```text
Compiled DAG has missing inputs.
```

Say:

```text
Some pieces are missing.
Add the missing details before Ordo can prepare the draft.
```

### Support

Do not say:

```text
Handoff queue projection contains claimable work items.
```

Say:

```text
People are waiting for help.
Claim one when you are ready to respond.
```

### Memory

Do not say:

```text
Memory promotion readiness packet created.
```

Say:

```text
Ready for you to review.
Ordo found something it may remember later. Nothing has been saved to memory yet.
```

### Knowledge

Do not say:

```text
Candidate graph edges require promotion.
```

Say:

```text
These connections still need review.
Ordo will not treat them as trusted until someone approves them.
```

### Provider / AI

Do not say:

```text
Provider egress policy blocked this call.
```

Say:

```text
Ordo did not send this outside the appliance.
Check the safety settings before using an outside AI service.
```

## Tone

Use direct, ordinary language:

- "Ordo checked..."
- "Nothing has been published."
- "This still needs review."
- "You can approve it or ask for changes."
- "Visitors cannot see staff notes."
- "This is a draft."
- "This is ready to review."

Avoid clinical or grandiose language:

- "governed decision state";
- "projection transition";
- "canonical truth object";
- "provenance-aware semantic promotion";
- "workflow orchestration surface";
- "human-in-the-loop queue" on member pages.

## Trust Copy

Trust copy should be clear without sounding scary.

Good:

```text
Nothing has been published.
Nothing has been saved to memory.
Visitors cannot see staff notes.
Ordo used these sources.
This still needs a person.
```

Bad:

```text
No graph truth mutation occurred.
No canonical memory mutation occurred.
Provider internals were excluded from the projection.
```

Keep technical proof available for operators, but do not lead with it.

## Screen Test

Before shipping a member-facing surface, ask:

1. Can a non-technical person tell what this is in five seconds?
2. Is there one obvious next action?
3. Is it clear whether this is draft, review, ready, or done?
4. Is it clear whether clicking will publish, send, save, or only inspect?
5. Are staff-only, private, prompt, provider, and policy details hidden?
6. Are sources or receipts available without dominating the screen?
7. Does the copy avoid architecture terms unless the user opens details?

If the answer is no, fix the language before adding more architecture.
