export type MockActivitySpace = "my-ordo" | "staff" | "studio" | "owner" | "admin";

export type MockActivityStatus =
  | "unread"
  | "waiting_on_you"
  | "waiting_on_ordo"
  | "active"
  | "ready"
  | "scheduled"
  | "candidate"
  | "done"
  | "blocked";

export type MockActivityPriority = "low" | "normal" | "high";

export interface MockOrdoEvent {
  id: string;
  space: MockActivitySpace;
  rooms: readonly string[];
  primaryRoom: string;
  kind:
    | "message"
    | "offer"
    | "capability"
    | "request"
    | "referral"
    | "job"
    | "artifact"
    | "feedback"
    | "review"
    | "handoff"
    | "pipeline"
    | "revenue"
    | "system";
  title: string;
  summary: string;
  status: MockActivityStatus;
  priority: MockActivityPriority;
  action: string;
  secondaryAction?: string;
  evidenceRefs: readonly string[];
  sourceLabel: string;
  occurredAt: string;
}

export const mockOrdoEvents: readonly MockOrdoEvent[] = [
  {
    id: "evt_user_chat_path_fit",
    space: "my-ordo",
    rooms: ["chat", "activity", "offers"],
    primaryRoom: "chat",
    kind: "message",
    title: "Ava asked which path fits",
    summary: "A meetup QR visitor asked whether to start with a strategic consultation or the hosted 30-day trial.",
    status: "unread",
    priority: "high",
    action: "Reply",
    secondaryAction: "Compare paths",
    evidenceRefs: ["conversation:ava-thompson", "entry:meetup-qr", "offer:starter-trial"],
    sourceLabel: "Ordo",
    occurredAt: "2026-05-10T09:44:00-04:00",
  },
  {
    id: "evt_user_offer_path_selection",
    space: "my-ordo",
    rooms: ["offers", "activity"],
    primaryRoom: "offers",
    kind: "offer",
    title: "Choose the right Studio Ordo path",
    summary: "Select consultation, 30-day trial, affiliate partner, or training access from the meetup QR intake.",
    status: "waiting_on_you",
    priority: "high",
    action: "Open offers",
    evidenceRefs: ["entry:meetup-qr", "visitor_session:qr-2026-05-10", "offer_set:meetup-paths"],
    sourceLabel: "Meetup QR",
    occurredAt: "2026-05-10T09:46:00-04:00",
  },
  {
    id: "evt_user_request_qr_proof",
    space: "my-ordo",
    rooms: ["asks", "activity", "packs"],
    primaryRoom: "asks",
    kind: "request",
    title: "Approve QR card proof",
    summary: "Studio Ordo needs approval before the generated QR proof is used for printed cards or event material.",
    status: "waiting_on_you",
    priority: "high",
    action: "Approve",
    secondaryAction: "Request changes",
    evidenceRefs: ["artifact:qr-card-proof", "trial:30-day", "conversation:ava-thompson"],
    sourceLabel: "QR card proof",
    occurredAt: "2026-05-10T10:02:00-04:00",
  },
  {
    id: "evt_user_feedback_request",
    space: "my-ordo",
    rooms: ["asks", "activity"],
    primaryRoom: "asks",
    kind: "feedback",
    title: "Share private feedback",
    summary: "A private follow-up asks how the trial setup felt. It is not public proof without consent and approval.",
    status: "waiting_on_you",
    priority: "normal",
    action: "Respond",
    evidenceRefs: ["trial:30-day", "feedback_request:trial-day-4", "return_link:simulated-review-request"],
    sourceLabel: "Trial follow-up",
    occurredAt: "2026-05-10T10:14:00-04:00",
  },
  {
    id: "evt_user_affiliate_terms",
    space: "my-ordo",
    rooms: ["affiliate", "activity"],
    primaryRoom: "affiliate",
    kind: "referral",
    title: "Affiliate terms are needed",
    summary: "Tracked links, QR codes, outcomes, and rewards stay locked until affiliate terms are accepted.",
    status: "waiting_on_you",
    priority: "normal",
    action: "Review terms",
    evidenceRefs: ["affiliate_candidate:ava-thompson", "policy:affiliate-terms-required"],
    sourceLabel: "Referral path",
    occurredAt: "2026-05-10T10:20:00-04:00",
  },
  {
    id: "evt_user_referral_link_locked",
    space: "my-ordo",
    rooms: ["affiliate"],
    primaryRoom: "affiliate",
    kind: "referral",
    title: "Referral link is ready to unlock",
    summary: "The affiliate path can create a tracked link after terms are accepted. Until then, the link is a candidate, not a shareable asset.",
    status: "blocked",
    priority: "normal",
    action: "Review terms",
    evidenceRefs: ["affiliate_candidate:ava-thompson", "entry_template:affiliate-link", "policy:affiliate-terms-required"],
    sourceLabel: "Affiliate setup",
    occurredAt: "2026-05-10T10:23:00-04:00",
  },
  {
    id: "evt_user_referral_qr_candidate",
    space: "my-ordo",
    rooms: ["affiliate"],
    primaryRoom: "affiliate",
    kind: "artifact",
    title: "Affiliate QR kit is a candidate",
    summary: "A QR kit can be generated for meetups after affiliate terms are accepted and the daemon validates the artifact identity.",
    status: "candidate",
    priority: "low",
    action: "Preview kit",
    evidenceRefs: ["artifact_candidate:affiliate-qr-kit", "capability:affiliate", "entry_template:affiliate-qr"],
    sourceLabel: "Referral kit",
    occurredAt: "2026-05-10T10:25:00-04:00",
  },
  {
    id: "evt_user_referral_outcome_model",
    space: "my-ordo",
    rooms: ["affiliate"],
    primaryRoom: "affiliate",
    kind: "referral",
    title: "Referral credit needs outcome evidence",
    summary: "Successful credit will cite the referral entry point, visitor session, accepted offer, and outcome before any reward is shown.",
    status: "ready",
    priority: "low",
    action: "View evidence",
    evidenceRefs: ["entry_point:affiliate-referral", "visitor_session:pending", "outcome:accepted-offer"],
    sourceLabel: "Reward model",
    occurredAt: "2026-05-10T10:28:00-04:00",
  },
  {
    id: "evt_user_pack_trial",
    space: "my-ordo",
    rooms: ["packs"],
    primaryRoom: "packs",
    kind: "capability",
    title: "Open an accepted capability",
    summary: "The 30-day Ordo trial capability will expose hosted setup, reset or extension requests, QR tracking, and the relationship conversation once accepted.",
    status: "candidate",
    priority: "normal",
    action: "Open capability",
    evidenceRefs: ["offer:starter-trial", "trial:30-day", "capability:trial"],
    sourceLabel: "30-day Ordo trial",
    occurredAt: "2026-05-10T10:31:00-04:00",
  },
  {
    id: "evt_user_consultation_capability",
    space: "my-ordo",
    rooms: ["packs"],
    primaryRoom: "packs",
    kind: "capability",
    title: "Strategic consultation can unlock owner access",
    summary: "A consultation capability can include scheduling, prep questions, context review, and a decision brief after the call.",
    status: "candidate",
    priority: "normal",
    action: "View consultation",
    evidenceRefs: ["offer:strategic-consultation", "calendar:consultation", "capability:consultation"],
    sourceLabel: "Strategic consultation",
    occurredAt: "2026-05-10T10:34:00-04:00",
  },
  {
    id: "evt_user_training_pack",
    space: "my-ordo",
    rooms: ["packs", "offers"],
    primaryRoom: "packs",
    kind: "capability",
    title: "Training access can unlock tutoring",
    summary: "A student path can include tutoring, assignment feedback, resources, progress help, and capability-enabled tools.",
    status: "active",
    priority: "normal",
    action: "View training",
    evidenceRefs: ["offer:training-access", "knowledge:agentic-os-lecture", "capability:training"],
    sourceLabel: "Training access",
    occurredAt: "2026-05-10T10:36:00-04:00",
  },
  {
    id: "evt_staff_handoff_trial_reset",
    space: "staff",
    rooms: ["handoffs", "conversations", "requests", "members"],
    primaryRoom: "handoffs",
    kind: "handoff",
    title: "Maya asked to talk to Keith live",
    summary: "A meetup QR visitor is ready to start the 30-day trial and asked for Keith while he is online.",
    status: "unread",
    priority: "high",
    action: "Claim handoff",
    evidenceRefs: ["conversation:maya-patel", "entry:meetup-qr", "offer:30-day-trial"],
    sourceLabel: "Staff handoff",
    occurredAt: "2026-05-10T10:40:00-04:00",
  },
  {
    id: "evt_staff_consult_reminder",
    space: "staff",
    rooms: ["conversations", "requests", "members"],
    primaryRoom: "connections",
    kind: "request",
    title: "Marcus needs consultation prep",
    summary: "A strategic consultation is scheduled tomorrow; send a prep reminder and ask for business context.",
    status: "scheduled",
    priority: "normal",
    action: "Send reminder",
    evidenceRefs: ["connection:marcus-chen", "offer:strategic-consultation", "calendar:consultation"],
    sourceLabel: "Consultation",
    occurredAt: "2026-05-10T10:45:00-04:00",
  },
  {
    id: "evt_staff_review_consent",
    space: "staff",
    rooms: ["reviews", "requests", "members"],
    primaryRoom: "reviews",
    kind: "review",
    title: "Student quote needs consent",
    summary: "A training student praised assignment feedback. Keep it private until consent and approval are recorded.",
    status: "waiting_on_you",
    priority: "normal",
    action: "Ask consent",
    evidenceRefs: ["feedback:student-assignment", "review_candidate:training-quote"],
    sourceLabel: "Review moderation",
    occurredAt: "2026-05-10T10:52:00-04:00",
  },
  {
    id: "evt_admin_conversation_handoff",
    space: "admin",
    rooms: ["conversations", "events", "system"],
    primaryRoom: "conversations",
    kind: "handoff",
    title: "Maya Patel meetup handoff is visible system-wide",
    summary: "Admin can inspect the QR entry, durable conversation, handoff state, and replay cursor without exposing provider payloads or private policy internals.",
    status: "unread",
    priority: "high",
    action: "Inspect replay",
    evidenceRefs: ["conversation:maya-patel", "handoff:keith-live", "cursor:staff-142"],
    sourceLabel: "Conversation replay",
    occurredAt: "2026-05-10T10:55:00-04:00",
  },
  {
    id: "evt_admin_conversation_review",
    space: "admin",
    rooms: ["conversations", "events"],
    primaryRoom: "conversations",
    kind: "review",
    title: "Ava Thompson review-return thread",
    summary: "Review-return conversations remain auditable with consent and approval state separated from private feedback.",
    status: "waiting_on_you",
    priority: "normal",
    action: "Review boundary",
    evidenceRefs: ["conversation:ava-thompson", "feedback:trial-day-4", "review_candidate:ava"],
    sourceLabel: "Review lifecycle",
    occurredAt: "2026-05-10T10:57:00-04:00",
  },
  {
    id: "evt_studio_lecture_job",
    space: "studio",
    rooms: ["knowledge", "factory-jobs", "artifacts", "media", "content-pillars", "publications"],
    primaryRoom: "factory-jobs",
    kind: "job",
    title: "Turn raw lecture into training module",
    summary: "Produce transcript, cleaned lesson, article, quiz, short video candidates, and student resources from the lecture source.",
    status: "active",
    priority: "high",
    action: "Open job",
    secondaryAction: "Review source",
    evidenceRefs: ["media:raw-agentic-os-lecture", "knowledge:agentic-os", "offer:training-access"],
    sourceLabel: "Factory job",
    occurredAt: "2026-05-10T11:03:00-04:00",
  },
  {
    id: "evt_studio_qr_artifact",
    space: "studio",
    rooms: ["factory-jobs", "artifacts", "media", "templates"],
    primaryRoom: "artifacts",
    kind: "artifact",
    title: "Meetup QR card is review-ready",
    summary: "The QR card cites campaign, offer, and entry-point evidence before it can be used at the next event.",
    status: "ready",
    priority: "normal",
    action: "Review artifact",
    evidenceRefs: ["artifact:meetup-qr-card", "entry_point:meetup-qr", "template:qr-card"],
    sourceLabel: "Artifact",
    occurredAt: "2026-05-10T11:08:00-04:00",
  },
  {
    id: "evt_owner_meetup_channel",
    space: "owner",
    rooms: ["overview", "marketing", "offers", "affiliates", "reports"],
    primaryRoom: "overview",
    kind: "pipeline",
    title: "Meetup QR is the strongest current channel",
    summary: "12 scans, four fit conversations, two trial requests, one consultation, and one affiliate candidate came from the last event.",
    status: "unread",
    priority: "high",
    action: "Inspect channel",
    evidenceRefs: ["campaign:may-meetup", "entry_point:meetup-qr", "report:live-journey"],
    sourceLabel: "Marketing",
    occurredAt: "2026-05-10T11:14:00-04:00",
  },
  {
    id: "evt_owner_trial_extension_policy",
    space: "owner",
    rooms: ["overview", "revenue", "offers", "reports"],
    primaryRoom: "overview",
    kind: "revenue",
    title: "Hosted trial extension policy needs a rule",
    summary: "Setup delays should not silently consume a user's trial. Decide the owner-approved extension rule before more hosted trials start.",
    status: "waiting_on_you",
    priority: "high",
    action: "Decide policy",
    evidenceRefs: ["trial:nora-patel", "handoff:trial-reset-question", "policy:trial-extension-draft"],
    sourceLabel: "Revenue",
    occurredAt: "2026-05-10T11:20:00-04:00",
  },
  {
    id: "evt_admin_replay_gap",
    space: "admin",
    rooms: ["system", "health", "events", "logs"],
    primaryRoom: "events",
    kind: "system",
    title: "Activity projector replay check",
    summary: "Reconnect should resume by cursor; duplicate events must remain idempotent and gaps must surface explicit errors.",
    status: "active",
    priority: "normal",
    action: "Inspect replay",
    evidenceRefs: ["daemon_event:activity-projector", "cursor:141", "read_model:user-activity"],
    sourceLabel: "Event stream",
    occurredAt: "2026-05-10T11:25:00-04:00",
  },
  {
    id: "evt_admin_provider_guard",
    space: "admin",
    rooms: ["providers", "settings", "logs"],
    primaryRoom: "providers",
    kind: "system",
    title: "Live provider calls remain guarded",
    summary: "Real LLM or email behavior must stay behind explicit owner-approved guards, spend caps, and redaction boundaries.",
    status: "ready",
    priority: "normal",
    action: "Review guards",
    evidenceRefs: ["setting:live-llm-guards", "provider:openai-compatible", "policy:egress"],
    sourceLabel: "Provider guard",
    occurredAt: "2026-05-10T11:29:00-04:00",
  },
];

export function eventsForRoom(space: MockActivitySpace, room: string): readonly MockOrdoEvent[] {
  return mockOrdoEvents
    .filter((event) => event.space === space && event.rooms.includes(room))
    .sort((a, b) => a.occurredAt.localeCompare(b.occurredAt));
}

export function unreadCountForRoom(space: MockActivitySpace, room: string): number {
  return eventsForRoom(space, room).filter((event) => event.status === "unread" || event.status === "waiting_on_you").length;
}

export function activeCountForRoom(space: MockActivitySpace, room: string): number {
  return eventsForRoom(space, room).filter((event) => event.status !== "done").length;
}

export function stateLabelForRoom(space: MockActivitySpace, room: string): string | undefined {
  const unreadCount = unreadCountForRoom(space, room);
  if (unreadCount > 0) {
    return unreadCount === 1 ? "1 waiting" : `${unreadCount} waiting`;
  }

  const events = eventsForRoom(space, room);
  if (!events.length) {
    return undefined;
  }

  const activeCount = activeCountForRoom(space, room);
  if (activeCount > 1) {
    return `${activeCount} active`;
  }

  return statusLabel(events[0].status);
}

export function statusLabel(status: MockActivityStatus): string {
  switch (status) {
    case "waiting_on_you":
      return "waiting on you";
    case "waiting_on_ordo":
      return "waiting on Studio Ordo";
    case "unread":
      return "unread";
    case "active":
      return "active";
    case "ready":
      return "ready";
    case "scheduled":
      return "scheduled";
    case "candidate":
      return "candidate";
    case "done":
      return "done";
    case "blocked":
      return "blocked";
  }
}
