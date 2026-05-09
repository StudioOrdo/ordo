# Approved Support Packet Handoff MVP

Status: backend foundation ready for PR; egress transport not built

## Why It Matters

Support should start from an operator-reviewed packet, not hidden telemetry.
This is the first A2A-shaped workflow without full peer networking.

## MVP Scope

- Reuse local issue reports as support packets.
- Add an approval step for sending a bounded packet to Studio Ordo Support.
- Send only approved packet content.
- Record sent packet metadata, destination, receipt, and outcome.
- Keep local report/export behavior available without egress.

## Backend Foundation

- Support packet drafts are prepared from local issue reports and preview the
	exact bounded packet content before any future egress transport exists.
- Drafts persist payload hash, destination metadata, `approvalRequired`, and
	`externalDelivery: false` evidence.
- Approval records `approved_local_only` with local receipt evidence and still
	records `deliveryState: not_sent`.
- No daemon route sends packets to Studio Ordo Support or any external system in
	this slice.

## Durable Product Nouns

- Support Packet
- Egress Approval
- Support Receipt
- Connection Event

## Acceptance Criteria

- No packet leaves without explicit operator approval.
- The sent payload is exactly the reviewed packet or a bounded derivative.
- Receipt is persisted and visible.
- Failure to send leaves local state understandable.
- Approval alone does not send; delivery remains future explicit work.

## Non-Goals

- General A2A networking.
- Automatic telemetry.
- Support chat.
- Network delivery implementation.

## Validation

- Tests proving approval is required.
- Payload redaction tests.
- Runtime proof against a mock support endpoint.
