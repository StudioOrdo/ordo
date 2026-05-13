export type MemberRoomId = "ordo" | "activity" | "offers" | "capabilities" | "requests";

export type MemberWorkItemKind = "message" | "offer" | "capability" | "request";

export type MemberWorkItemStatus = "unread" | "waiting_on_you" | "waiting_on_ordo" | "candidate" | "active" | "done";

export type MemberActionKind =
  | "view"
  | "reply"
  | "approve"
  | "request_changes"
  | "mark_read"
  | "open_source"
  | "respond"
  | "review_terms"
  | "accept"
  | "decline"
  | "request_meeting";

export interface MemberRoom {
  id: MemberRoomId;
  label: string;
  href: string;
  eyebrow: string;
  description: string;
  brief: string;
  quietState: string;
}

export interface MemberAction {
  kind: MemberActionKind;
  label: string;
  href?: string;
  primary?: boolean;
}

export interface MemberEvidenceRef {
  id: string;
  label: string;
  kind: "conversation" | "entry_point" | "offer" | "capability" | "request" | "referral" | "artifact" | "policy" | "person";
}

export interface MemberTimelineEvent {
  at: string;
  title: string;
  summary: string;
}

export interface MemberSummaryBand {
  whatHappened: string;
  whyItMatters: string;
  recommendedAction: string;
}

export interface MemberWorkItem {
  id: string;
  rooms: readonly MemberRoomId[];
  primaryRoom: MemberRoomId;
  kind: MemberWorkItemKind;
  title: string;
  summary: string;
  status: MemberWorkItemStatus;
  readState: "read" | "unread";
  source: string;
  occurredAt: string;
  actions: readonly MemberAction[];
  evidenceRefs: readonly MemberEvidenceRef[];
  summaryBand: MemberSummaryBand;
  timeline: readonly MemberTimelineEvent[];
  relatedObjects?: readonly string[];
  conversationPreview?: readonly {
    speaker: string;
    body: string;
    tone?: "member" | "ordo" | "safe_status" | "offer" | "request";
    title?: string;
    meta?: string;
    actions?: readonly MemberAction[];
  }[];
}

export const memberRooms: readonly MemberRoom[] = [
  {
    id: "ordo",
    label: "Ordo",
    href: "/my/chat",
    eyebrow: "Ordo",
    description: "One relationship conversation with Studio Ordo.",
    brief: "Primary conversation",
    quietState: "ready",
  },
  {
    id: "activity",
    label: "Activity",
    href: "/my/activity",
    eyebrow: "Attention",
    description: "Unread changes and actions from your Studio Ordo relationship.",
    brief: "Trial feedback is waiting",
    quietState: "quiet",
  },
  {
    id: "offers",
    label: "Offers",
    href: "/my/offers",
    eyebrow: "Offers",
    description: "What Studio Ordo has offered you to accept, book, join, or try.",
    brief: "Hosted trial accepted",
    quietState: "trial accepted",
  },
  {
    id: "requests",
    label: "Requests",
    href: "/my/requests",
    eyebrow: "Request queue",
    description: "What Studio Ordo needs you to answer, approve, or complete.",
    brief: "Trial feedback is waiting",
    quietState: "quiet",
  },
  {
    id: "capabilities",
    label: "Capabilities",
    href: "/my/capabilities",
    eyebrow: "Capabilities",
    description: "What you can use because an offer was accepted or enabled.",
    brief: "Hosted trial active",
    quietState: "trial active",
  },
];

export const memberWorkItems: readonly MemberWorkItem[] = [
  {
    id: "trial-conversation",
    rooms: ["ordo"],
    primaryRoom: "ordo",
    kind: "message",
    title: "Hosted trial conversation",
    summary: "You came in from the meetup QR, accepted the hosted 30-day trial, and can ask for Keith from this same thread.",
    status: "done",
    readState: "read",
    source: "Founder Meetup Intro QR",
    occurredAt: "May 10, 9:52 AM",
    actions: [
      { kind: "reply", label: "Message Ordo", href: "/my/chat", primary: true },
      { kind: "request_meeting", label: "Request Keith", href: "/my/chat?notice=request-keith" },
    ],
    evidenceRefs: [
      { id: "conversation:your-ordo-thread", label: "Your conversation", kind: "conversation" },
      { id: "entry:founder-meetup-intro-qr", label: "Founder Meetup Intro QR", kind: "entry_point" },
      { id: "offer:hosted-30-day-trial", label: "Hosted 30-day trial", kind: "offer" },
      { id: "capability:hosted-trial", label: "Hosted trial", kind: "capability" },
    ],
    summaryBand: {
      whatHappened: "You accepted the hosted 30-day trial after entering through the meetup QR.",
      whyItMatters: "Your trial now centers on using Ordo and giving private feedback, not choosing a sales path.",
      recommendedAction: "Message Ordo",
    },
    timeline: [
      { at: "May 10, 9:38 AM", title: "QR visit recorded", summary: "You entered through the Founder Meetup Intro QR." },
      { at: "May 10, 9:46 AM", title: "Keith handoff requested", summary: "You asked whether you could talk to Keith before starting." },
      { at: "May 10, 9:52 AM", title: "Trial accepted", summary: "The hosted 30-day trial became your active path." },
    ],
    relatedObjects: ["Founder Meetup Intro QR", "Hosted 30-day Ordo trial", "Private trial feedback"],
    conversationPreview: [
      {
        speaker: "Ordo",
        body: "You came in from Keith's meetup QR. I can help you choose a consultation, hosted 30-day trial, training access, or the affiliate path from this one thread.",
        tone: "ordo",
      },
      {
        speaker: "Ordo",
        title: "Hosted 30-day trial",
        meta: "Offer",
        body: "Use Ordo for 30 days around your real follow-up process. You can ask setup questions, request Keith, and give private feedback before deciding what to do next.",
        tone: "offer",
        actions: [
          { kind: "accept", label: "Accept trial", primary: true },
          { kind: "decline", label: "Decline" },
        ],
      },
      {
        speaker: "You",
        body: "I accepted the 30-day trial. I want to try it with my real follow-up process before deciding what to do next.",
        tone: "member",
      },
      {
        speaker: "Ordo",
        title: "Share private feedback",
        meta: "Request",
        body: "When you have tried the setup, answer a short private feedback request here. Nothing becomes a public review or testimonial unless you explicitly approve it.",
        tone: "request",
        actions: [{ kind: "reply", label: "Give feedback", href: "/my/chat?notice=private-feedback-request", primary: true }],
      },
      {
        speaker: "Ordo",
        title: "Keith can take over here",
        meta: "Handoff",
        body: "If you want to talk with Keith, request a handoff in this conversation. You will only see the safe status; internal routing, staff notes, and provider details stay hidden.",
        tone: "safe_status",
        actions: [{ kind: "request_meeting", label: "Request Keith", href: "/my/chat?notice=request-keith", primary: true }],
      },
    ],
  },
  {
    id: "private-feedback-request",
    rooms: ["activity", "requests"],
    primaryRoom: "requests",
    kind: "request",
    title: "Complete private feedback request",
    summary: "Tell Studio Ordo what worked and what was confusing in the hosted trial. Nothing becomes public without your consent.",
    status: "waiting_on_you",
    readState: "unread",
    source: "Hosted 30-day trial",
    occurredAt: "May 10, 10:14 AM",
    actions: [
      { kind: "reply", label: "Give feedback", href: "/my/chat?notice=private-feedback-request", primary: true },
      { kind: "mark_read", label: "Mark read" },
    ],
    evidenceRefs: [
      { id: "request:trial-feedback", label: "Private trial feedback", kind: "request" },
      { id: "capability:hosted-trial", label: "Hosted trial", kind: "capability" },
      { id: "policy:review-consent-required", label: "Review consent required", kind: "policy" },
    ],
    summaryBand: {
      whatHappened: "Studio Ordo asked you for one private response about the hosted trial setup.",
      whyItMatters: "Your feedback helps improve the trial, but it stays private unless you approve public use later.",
      recommendedAction: "Give feedback",
    },
    timeline: [
      { at: "May 10, 9:38 AM", title: "QR visit recorded", summary: "You entered through the Founder Meetup Intro QR." },
      { at: "May 10, 9:52 AM", title: "Trial accepted", summary: "You accepted the hosted 30-day Ordo trial." },
      { at: "May 10, 10:14 AM", title: "Feedback requested", summary: "Studio Ordo asked for private feedback on the trial setup." },
    ],
    relatedObjects: ["Private trial feedback", "Hosted 30-day trial", "Review consent policy"],
  },
  {
    id: "hosted-trial-accepted-offer",
    rooms: ["offers"],
    primaryRoom: "offers",
    kind: "offer",
    title: "30 days left in your hosted trial",
    summary: "Your hosted trial is active. Use Ordo for setup questions, feedback, and meeting requests before deciding what to do next.",
    status: "active",
    readState: "read",
    source: "Founder Meetup Intro QR",
    occurredAt: "May 10, 9:52 AM",
    actions: [
      { kind: "reply", label: "Message Ordo", href: "/my/chat?notice=trial-question", primary: true },
      { kind: "request_meeting", label: "Request meeting", href: "/my/chat?notice=request-meeting" },
      { kind: "view", label: "View receipt" },
    ],
    evidenceRefs: [
      { id: "entry:founder-meetup-intro-qr", label: "Founder Meetup Intro QR", kind: "entry_point" },
      { id: "offer:hosted-30-day-trial", label: "Hosted 30-day trial", kind: "offer" },
      { id: "conversation:your-ordo-thread", label: "Your conversation", kind: "conversation" },
    ],
    summaryBand: {
      whatHappened: "Your hosted 30-day trial is active after the meetup QR signup.",
      whyItMatters: "Use the trial with real setup questions, private feedback, and meeting requests while time remains.",
      recommendedAction: "Use the trial",
    },
    timeline: [
      { at: "May 10, 9:38 AM", title: "Entry source known", summary: "You arrived through the Founder Meetup Intro QR." },
      { at: "May 10, 9:44 AM", title: "Offer explained", summary: "Ordo explained the hosted 30-day trial in the relationship conversation." },
      { at: "May 10, 9:52 AM", title: "Offer accepted", summary: "You accepted the hosted trial." },
      { at: "Now", title: "Trial active", summary: "You have 30 days left to use Ordo and ask questions." },
    ],
    relatedObjects: ["Hosted 30-day Ordo trial", "Founder Meetup Intro QR"],
  },
  {
    id: "strategic-consultation-option",
    rooms: ["offers"],
    primaryRoom: "offers",
    kind: "offer",
    title: "Strategic consultation remains available",
    summary: "You can still book a focused call with Keith if you want direction during the trial.",
    status: "candidate",
    readState: "read",
    source: "Consultation path",
    occurredAt: "May 10, 10:00 AM",
    actions: [
      { kind: "view", label: "View consultation", primary: true },
      { kind: "reply", label: "Message Ordo", href: "/my/chat?notice=strategic-consultation" },
    ],
    evidenceRefs: [
      { id: "offer:strategic-consultation", label: "Strategic consultation", kind: "offer" },
      { id: "conversation:your-ordo-thread", label: "Your conversation", kind: "conversation" },
    ],
    summaryBand: {
      whatHappened: "A strategic consultation remains available as a later offer.",
      whyItMatters: "You should not lose access to a relevant next step after accepting the trial.",
      recommendedAction: "View consultation",
    },
    timeline: [
      { at: "May 10, 9:44 AM", title: "Consultation discussed", summary: "You compared the consultation with the hosted trial." },
      { at: "Later", title: "Still available", summary: "The consultation can be booked if the trial creates a strategic question." },
    ],
    relatedObjects: ["Strategic consultation", "Hosted 30-day trial"],
  },
  {
    id: "hosted-trial-capability",
    rooms: ["capabilities"],
    primaryRoom: "capabilities",
    kind: "capability",
    title: "Hosted 30-day trial is active",
    summary: "You can use the hosted trial, ask Ordo questions, and request setup help from the same relationship thread.",
    status: "active",
    readState: "read",
    source: "Hosted 30-day trial",
    occurredAt: "May 10, 9:52 AM",
    actions: [
      { kind: "view", label: "Open trial", primary: true },
      { kind: "reply", label: "Message Ordo" },
    ],
    evidenceRefs: [
      { id: "offer:hosted-30-day-trial", label: "Hosted 30-day trial", kind: "offer" },
      { id: "capability:hosted-trial", label: "Hosted trial", kind: "capability" },
      { id: "conversation:your-ordo-thread", label: "Your conversation", kind: "conversation" },
    ],
    summaryBand: {
      whatHappened: "The hosted 30-day trial is active for you.",
      whyItMatters: "Capabilities show what you can actually use now, not every possible future offer.",
      recommendedAction: "Open trial",
    },
    timeline: [
      { at: "May 10, 9:52 AM", title: "Trial accepted", summary: "You accepted the hosted 30-day trial." },
      { at: "May 10, 9:54 AM", title: "Trial enabled", summary: "Ordo enabled the trial capability for you." },
      { at: "Now", title: "Feedback pending", summary: "Your only current action is private trial feedback." },
    ],
    relatedObjects: ["Hosted trial", "Private trial feedback", "Relationship conversation"],
  },
  {
    id: "trial-feedback-capability",
    rooms: ["capabilities"],
    primaryRoom: "capabilities",
    kind: "capability",
    title: "Private feedback stays private",
    summary: "You can answer trial feedback without creating a public review or testimonial.",
    status: "active",
    readState: "read",
    source: "Review consent policy",
    occurredAt: "May 10, 10:10 AM",
    actions: [
      { kind: "view", label: "View policy", primary: true },
      { kind: "respond", label: "Respond to request" },
    ],
    evidenceRefs: [
      { id: "policy:review-consent-required", label: "Review consent required", kind: "policy" },
      { id: "request:trial-feedback", label: "Private trial feedback", kind: "request" },
    ],
    summaryBand: {
      whatHappened: "Feedback is available as a private capability.",
      whyItMatters: "Studio Ordo can improve the trial without treating your private feedback as public proof.",
      recommendedAction: "Respond to request",
    },
    timeline: [
      { at: "May 10, 10:10 AM", title: "Consent boundary applied", summary: "Private feedback was kept separate from public review proof." },
      { at: "Now", title: "Feedback request open", summary: "You can respond privately when ready." },
    ],
    relatedObjects: ["Review consent policy", "Private feedback request"],
  },
  {
    id: "referral-capability-candidate",
    rooms: ["capabilities"],
    primaryRoom: "capabilities",
    kind: "capability",
    title: "Referral tools can be enabled later",
    summary: "Referral links and QR codes stay locked until you accept referral terms.",
    status: "candidate",
    readState: "read",
    source: "Referral capability",
    occurredAt: "May 10, 10:25 AM",
    actions: [
      { kind: "review_terms", label: "Review terms", primary: true },
      { kind: "view", label: "Preview referral kit" },
    ],
    evidenceRefs: [
      { id: "referral-candidate:your-account", label: "Your referral path", kind: "referral" },
      { id: "policy:affiliate-terms-required", label: "Affiliate terms required", kind: "policy" },
    ],
    summaryBand: {
      whatHappened: "Referral tools are available as a candidate capability, but not active.",
      whyItMatters: "Referral credit must be governed before links, QR codes, or reward status become shareable.",
      recommendedAction: "Review terms",
    },
    timeline: [
      { at: "May 10, 10:25 AM", title: "Referral candidate recorded", summary: "A referral capability can be enabled later." },
      { at: "Before sharing", title: "Terms required", summary: "You must accept terms before referral links or QR codes become active." },
    ],
    relatedObjects: ["Referral terms", "Tracked referral link", "Referral QR kit"],
  },
];

export function memberRoomById(roomId: MemberRoomId): MemberRoom {
  return memberRooms.find((room) => room.id === roomId) ?? memberRooms[0];
}

export function memberItemsForRoom(roomId: MemberRoomId): readonly MemberWorkItem[] {
  if (roomId === "activity") {
    return memberWorkItems.filter((item) => item.readState === "unread" || item.status === "waiting_on_you");
  }
  return [...memberWorkItems.filter((item) => item.rooms.includes(roomId))].sort((a, b) => {
    if (a.primaryRoom === roomId && b.primaryRoom !== roomId) return -1;
    if (a.primaryRoom !== roomId && b.primaryRoom === roomId) return 1;
    return 0;
  });
}

export function memberRoomWaitingCount(roomId: MemberRoomId): number {
  return memberItemsForRoom(roomId).filter((item) => item.readState === "unread" || item.status === "waiting_on_you").length;
}

export function memberWorkspaceWaitingCount(): number {
  return memberWorkItems.filter((item) => item.readState === "unread" || item.status === "waiting_on_you").length;
}

export function selectedMemberItem(roomId: MemberRoomId, selectedIndex: number): MemberWorkItem {
  const items = memberItemsForRoom(roomId);
  return items[Math.max(0, Math.min(selectedIndex, items.length - 1))] ?? memberWorkItems[0];
}

export function memberRoomIdFromPath(path: string): MemberRoomId {
  if (path.includes("activity")) return "activity";
  if (path.includes("offers")) return "offers";
  if (path.includes("capabilities") || path.includes("packs") || path.includes("access") || path.includes("referrals") || path.includes("affiliate")) return "capabilities";
  if (path.includes("requests") || path.includes("asks")) return "requests";
  return "ordo";
}
