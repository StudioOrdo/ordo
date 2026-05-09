export type ConversationQueueId = "my-handoffs" | "team-queue" | "all-conversations";

export type HandoffStatus =
  | "suggested"
  | "requested"
  | "accepted"
  | "declined"
  | "assigned"
  | "in_progress"
  | "returned_to_agent"
  | "closed";

export type ConversationMode =
  | "agent_led"
  | "human_led_active"
  | "human_led_idle"
  | "assistive_private"
  | "needs_handoff"
  | "returned_to_agent";

export type CandidateState = "proposed" | "confirmed" | "rejected" | "superseded";

export interface ConversationQueue {
  id: ConversationQueueId;
  label: string;
  description: string;
  defaultFor: readonly string[];
}

export interface HandoffBrief {
  reason: string;
  urgency: "low" | "medium" | "high" | "urgent";
  status: HandoffStatus;
  assignedTo: string;
  requiredCapability: string;
  allowedContext: readonly string[];
  evidenceSummary: string;
  suggestedReply: string;
  riskOrConstraint: string;
}

export interface ConversationEpisode {
  id: string;
  title: string;
  kind: string;
  status: CandidateState;
  confidence: number;
  evidenceRefs: readonly string[];
  provenance: string;
}

export interface ConversationQueueRow {
  id: string;
  conversationLabel: string;
  connectionLabel: string;
  queueId: ConversationQueueId;
  whyHere: string;
  lastMeaningfulChange: string;
  unreadCount: number;
  actionCount: number;
  mode: ConversationMode;
  handoff: HandoffBrief;
  episode: ConversationEpisode;
}

export const conversationQueues: readonly ConversationQueue[] = [
  {
    id: "my-handoffs",
    label: "My Handoffs",
    description: "Assigned work that needs this staff member to act.",
    defaultFor: ["staff"],
  },
  {
    id: "team-queue",
    label: "Team Queue",
    description: "Handoffs and review work shared by the operating team.",
    defaultFor: ["manager", "owner"],
  },
  {
    id: "all-conversations",
    label: "All Conversations",
    description: "Authorized administrative view across conversation work.",
    defaultFor: ["admin"],
  },
];

export const sampleConversationRows: readonly ConversationQueueRow[] = [
  {
    id: "conv_ava",
    conversationLabel: "Your conversation with Studio Ordo",
    connectionLabel: "Ava Thompson",
    queueId: "my-handoffs",
    whyHere: "Ava asked about QR card pricing and is ready for a human answer.",
    lastMeaningfulChange: "Pricing question in the Starter trial conversation.",
    unreadCount: 2,
    actionCount: 1,
    mode: "human_led_active",
    handoff: {
      reason: "Pricing and purchase-intent question",
      urgency: "high",
      status: "assigned",
      assignedTo: "Keith",
      requiredCapability: "conversation.handoff.manage",
      allowedContext: ["Starter offer", "QR card artifact", "recent conversation window"],
      evidenceSummary: "Ava viewed the Starter offer and asked whether metal QR cards are included.",
      suggestedReply: "Confirm the trial scope, name the metal card add-on, and ask whether she wants the first card proof.",
      riskOrConstraint: "Do not promise custom card pricing until the current offer terms are confirmed.",
    },
    episode: {
      id: "episode_qr_card_pricing",
      title: "Metal QR card request",
      kind: "pricing-question",
      status: "proposed",
      confidence: 0.82,
      evidenceRefs: ["message_ava_14", "offer_view_starter_3", "artifact_qr_card_1"],
      provenance: "conversation.episode.extract",
    },
  },
  {
    id: "conv_marcus",
    conversationLabel: "Your conversation with Studio Ordo",
    connectionLabel: "Marcus Reed",
    queueId: "team-queue",
    whyHere: "Marcus offered a referral and the relationship outcome needs qualification.",
    lastMeaningfulChange: "Referral offered after reviewing the local-business beta ask.",
    unreadCount: 0,
    actionCount: 1,
    mode: "needs_handoff",
    handoff: {
      reason: "Referral offered",
      urgency: "medium",
      status: "requested",
      assignedTo: "Team Queue",
      requiredCapability: "referral.qualify",
      allowedContext: ["local-business beta ask", "referral message", "relationship summary"],
      evidenceSummary: "Marcus named a possible beta customer after viewing the ask.",
      suggestedReply: "Thank Marcus, ask for permission to mention him, and request the best intro path.",
      riskOrConstraint: "Do not record the referred person as confirmed until consent and contact details exist.",
    },
    episode: {
      id: "episode_referral_offer",
      title: "Local-business beta referral",
      kind: "referral-offered",
      status: "proposed",
      confidence: 0.76,
      evidenceRefs: ["message_marcus_7", "ask_view_beta_2"],
      provenance: "conversation.tags.update",
    },
  },
];

export function defaultQueueForRole(role: string): ConversationQueueId {
  if (role === "admin") {
    return "all-conversations";
  }
  if (role === "manager" || role === "owner") {
    return "team-queue";
  }
  return "my-handoffs";
}

export function canAccessQueue(role: string, queueId: ConversationQueueId): boolean {
  if (queueId === "my-handoffs") {
    return role === "staff" || role === "manager" || role === "owner" || role === "admin";
  }
  if (queueId === "team-queue") {
    return role === "manager" || role === "owner" || role === "admin";
  }
  return role === "owner" || role === "admin";
}

export function queueRowsForRole(role: string): readonly ConversationQueueRow[] {
  const defaultQueue = defaultQueueForRole(role);
  if (defaultQueue === "all-conversations") {
    return sampleConversationRows;
  }
  return sampleConversationRows.filter((row) => row.queueId === defaultQueue || defaultQueue === "team-queue");
}
