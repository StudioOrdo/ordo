# Deferred Member Experience Components

Status: preserved for later product pass

The member Ordo room now prioritizes real daemon-backed chat through `/chat/ws`.
The earlier product-depth components are still in the repository and should be
revisited after live provider chat works reliably end to end.

Preserved components and fixtures:

- `MemberConversationPreview`, `MemberSummaryBand`, timelines, evidence refs,
  and action rows in `components/member-ordo-surface.tsx`.
- Member room and work-item fixtures in `lib/member-ordo-mock.ts`.
- Handoff-oriented placeholder/product surfaces in `components/app-placeholder-page.tsx`
  and `components/ordo-chat-prototype.tsx`.
- Backend handoff foundation tracked in `docs/backlog/handoff-inbox.md`.
- Offer/trial state foundation tracked in `docs/backlog/offer-trial-state.md`.

Return to these after provider chat is stable:

- Reintroduce handoff cards from durable daemon read models instead of mock
  conversation preview messages.
- Connect hosted trial/offer cards to real offer acceptance and trial state.
- Add owner/admin repair and diagnostic controls as governed daemon methods,
  not arbitrary SQL or direct provider calls from the browser.
