# Mediated Chat MVP

Status: not built

## Why It Matters

The owner and visitor should be able to communicate through Ordo without losing
policy, context, evidence, or the ability to pause and resume.

## MVP Scope

- Create conversation records for mediated owner/visitor chat.
- Attach participant identities, visibility boundary, source session, and
  handoff envelope.
- Route messages through Ordo-owned storage and policy checks.
- Let owner pause, continue, or close a conversation.
- Produce a briefable transcript summary.

## Durable Product Nouns

- Conversation
- Participant
- Message
- Transcript Summary
- Conversation State

## Acceptance Criteria

- Conversation history is durable and scoped.
- Messages do not expose private content outside policy.
- Owner can close or pause conversation.
- Handoff source remains linked to the conversation.

## Non-Goals

- Voice or video.
- External chat platform integration.
- Multi-operator customer support suite.

## Validation

- Schema and policy tests.
- Message state tests.
- UI smoke once chat surface exists.
