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

export type ConversationMessageAuthor = "client" | "staff" | "assistant";
export type ConversationMessageState = "pending" | "persisted" | "read" | "failed" | "deleted";

export interface ConversationReaction {
  key: string;
  label: string;
  count: number;
  selectedByViewer: boolean;
}

export interface ConversationMessage {
  id: string;
  clientId?: string;
  author: ConversationMessageAuthor;
  authorLabel: string;
  body: string;
  occurredAt: string;
  sequence: number;
  state: ConversationMessageState;
  edited: boolean;
  canEdit: boolean;
  canUndo: boolean;
  receiptLabel: string;
  reactions: readonly ConversationReaction[];
}

export interface ArtifactBriefCard {
  id: string;
  artifactKind: string;
  systemLabel: string;
  clientLabel: string;
  value: string;
  use: string;
  nextAction: string;
  producingJob: string;
  provenance: string;
  storageHealth: string;
  visibility: "staff" | "client";
  evidenceRefs: readonly string[];
}

export interface EthicalPersuasionGuidance {
  slotId: "ethical_business_persuasion";
  slotVersion: "v1";
  useCase: string;
  principles: readonly string[];
  staffReasoning: string;
  clientSafeSuggestion: string;
  evidenceRefs: readonly string[];
  sourceRefs: readonly string[];
}

export interface ConversationDetail {
  conversationId: string;
  participantId: string;
  title: string;
  subtitle: string;
  mode: ConversationMode;
  connectionLabel: string;
  narrativeBrief: {
    whatIsHappening: string;
    whatChanged: string;
    nextStep: string;
    whyItMatters: string;
    evidence: string;
    limitation: string;
  };
  messages: readonly ConversationMessage[];
  artifactCards: readonly ArtifactBriefCard[];
  persuasionGuidance: EthicalPersuasionGuidance | null;
  typing: readonly string[];
  presence: readonly string[];
  unreadFromSequence: number;
  lastReadSequence: number;
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

export const sampleConversationDetails: Record<string, ConversationDetail> = {
  conv_ava: {
    conversationId: "conv_ava",
    participantId: "participant_keith",
    title: "Your conversation with Studio Ordo",
    subtitle: "Ava Thompson · Starter offer and QR card proof",
    mode: "human_led_active",
    connectionLabel: "Ava Thompson",
    unreadFromSequence: 3,
    lastReadSequence: 2,
    typing: ["Ava is typing"],
    presence: ["Keith here", "Ava available now"],
    narrativeBrief: {
      whatIsHappening: "Ava is evaluating the Starter offer and asked whether a metal QR card is included.",
      whatChanged: "The conversation moved from general interest to a purchase-intent pricing question.",
      nextStep: "Confirm the trial scope, name the metal card add-on, and ask whether she wants the first proof.",
      whyItMatters: "A precise answer can turn the offer view into a concrete deliverable without inventing terms.",
      evidence: "message_ava_14, offer_view_starter_3, artifact_qr_card_1",
      limitation: "Custom card pricing must be confirmed before a final quote is promised.",
    },
    messages: [
      {
        id: "message_ava_12",
        author: "assistant",
        authorLabel: "Ordo Assistant",
        body: "The Starter trial covers the first public profile, the offer page, and a digital QR proof.",
        occurredAt: "9:42 AM",
        sequence: 1,
        state: "read",
        edited: false,
        canEdit: false,
        canUndo: false,
        receiptLabel: "Read",
        reactions: [],
      },
      {
        id: "message_ava_13",
        author: "client",
        authorLabel: "Ava",
        body: "That helps. I care most about the card people can scan at events.",
        occurredAt: "9:44 AM",
        sequence: 2,
        state: "read",
        edited: false,
        canEdit: false,
        canUndo: false,
        receiptLabel: "Read",
        reactions: [{ key: "useful", label: "Useful", count: 1, selectedByViewer: false }],
      },
      {
        id: "message_ava_14",
        author: "client",
        authorLabel: "Ava",
        body: "Are metal QR cards included, or is that a separate add-on?",
        occurredAt: "9:46 AM",
        sequence: 3,
        state: "persisted",
        edited: false,
        canEdit: false,
        canUndo: false,
        receiptLabel: "Delivered",
        reactions: [],
      },
      {
        id: "message_keith_15",
        author: "staff",
        authorLabel: "Keith",
        body: "Metal cards are a separate add-on. The trial includes the digital proof so you can approve the destination before we produce anything physical.",
        occurredAt: "9:48 AM",
        sequence: 4,
        state: "persisted",
        edited: false,
        canEdit: true,
        canUndo: true,
        receiptLabel: "Delivered",
        reactions: [{ key: "clear", label: "Clear", count: 1, selectedByViewer: true }],
      },
    ],
    artifactCards: [
      {
        id: "artifact_qr_card_1",
        artifactKind: "offer.material",
        systemLabel: "Artifact: Starter QR card proof",
        clientLabel: "Deliverable: QR card proof",
        value: "Turns Ava's pricing question into a concrete proof she can review.",
        use: "Used by the Starter offer conversation and future offer outcome attribution.",
        nextAction: "Confirm the offer scope, then decide whether to publish as a client deliverable.",
        producingJob: "artifacts.brief.generate",
        provenance: "offer_view_starter_3, message_ava_14",
        storageHealth: "available",
        visibility: "client",
        evidenceRefs: ["offer_view_starter_3", "message_ava_14"],
      },
    ],
    persuasionGuidance: {
      slotId: "ethical_business_persuasion",
      slotVersion: "v1",
      useCase: "reply_draft",
      principles: ["reciprocity"],
      staffReasoning: "Use reciprocity because offer_view_starter_3 shows Ava already received value in the digital proof explanation.",
      clientSafeSuggestion: "You can review the digital proof first, then decide whether the card add-on is useful.",
      evidenceRefs: ["offer_view_starter_3", "message_ava_14"],
      sourceRefs: ["artifact_qr_card_1"],
    },
  },
  conv_marcus: {
    conversationId: "conv_marcus",
    participantId: "participant_keith",
    title: "Your conversation with Studio Ordo",
    subtitle: "Marcus Reed · Local-business beta referral",
    mode: "needs_handoff",
    connectionLabel: "Marcus Reed",
    unreadFromSequence: 0,
    lastReadSequence: 3,
    typing: [],
    presence: ["Marcus replies soon"],
    narrativeBrief: {
      whatIsHappening: "Marcus offered a referral after reading the local-business beta ask.",
      whatChanged: "The conversation now needs consent and a clean introduction path before attribution.",
      nextStep: "Thank Marcus, ask permission to mention him, and request the best way to introduce the business.",
      whyItMatters: "Referral attribution should be evidence-backed before it becomes an outcome candidate.",
      evidence: "message_marcus_7, ask_view_beta_2",
      limitation: "Do not record the referred person as confirmed without consent and contact details.",
    },
    messages: [
      {
        id: "message_marcus_5",
        author: "assistant",
        authorLabel: "Ordo Assistant",
        body: "The beta ask is looking for local operators who already have customer attention but need a clearer public surface.",
        occurredAt: "Yesterday",
        sequence: 1,
        state: "read",
        edited: false,
        canEdit: false,
        canUndo: false,
        receiptLabel: "Read",
        reactions: [],
      },
      {
        id: "message_marcus_7",
        author: "client",
        authorLabel: "Marcus",
        body: "I know a shop owner who might be perfect for this. Want an intro?",
        occurredAt: "Today",
        sequence: 2,
        state: "persisted",
        edited: false,
        canEdit: false,
        canUndo: false,
        receiptLabel: "Delivered",
        reactions: [{ key: "lead", label: "Lead", count: 1, selectedByViewer: false }],
      },
    ],
    artifactCards: [
      {
        id: "artifact_beta_ask_1",
        artifactKind: "ask.material",
        systemLabel: "Artifact: Local-business beta ask",
        clientLabel: "Deliverable: Beta intro brief",
        value: "Captures the ask Marcus responded to before referral qualification.",
        use: "Used by the team queue and referral outcome evidence.",
        nextAction: "Confirm permission to mention Marcus before treating the referral as qualified.",
        producingJob: "artifact.usage.summarize",
        provenance: "ask_view_beta_2, message_marcus_7",
        storageHealth: "available",
        visibility: "staff",
        evidenceRefs: ["ask_view_beta_2", "message_marcus_7"],
      },
    ],
    persuasionGuidance: {
      slotId: "ethical_business_persuasion",
      slotVersion: "v1",
      useCase: "reply_draft",
      principles: ["commitment_consistency", "unity"],
      staffReasoning: "Use commitment/consistency because message_marcus_7 shows Marcus offered a referral; use unity only around the shared local-business beta mission.",
      clientSafeSuggestion: "Thanks for thinking of someone who may fit the beta. If you are comfortable, tell me the best way to make the introduction.",
      evidenceRefs: ["message_marcus_7", "ask_view_beta_2"],
      sourceRefs: ["conversation_conv_marcus"],
    },
  },
};

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
