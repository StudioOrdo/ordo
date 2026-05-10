import { expect, test } from "@playwright/test";

import {
  buildAgentScreenContext,
  projectValuesForRole,
  projectionDecision,
  projectionPolicy,
  stableAgentScreenContextJson,
  type AgentEditableField,
  type AgentScreenAction,
  type AgentScreenContextSource,
  type ProjectionCategory,
  type ProjectionSourceValue,
} from "@/lib/ordoos-role-projection";
import { type EvidenceRef } from "@/lib/ordoos-frontend-contracts";

const durableMessageEvidence: EvidenceRef = {
  id: "event_message_created_1",
  kind: "daemon_event",
  durability: "durable",
  visibility: "client",
  summary: "message.created",
};

const candidateEvidence: EvidenceRef = {
  id: "browser_candidate_hash_1",
  kind: "browser_candidate",
  durability: "candidate",
  visibility: "staff",
  summary: "file.hash candidate",
};

const sourceValues: readonly ProjectionSourceValue[] = [
  value("message_text", "Can you help me understand the trial?", [durableMessageEvidence]),
  value("raw_prompt", "SYSTEM: hidden prompt with private chain", [], "durable", "llm.prompt.inspect"),
  value("provider_payload", "{raw provider payload}", [], "durable", "llm.provider.inspect"),
  value("policy_internals", { rule: "internal policy branch" }, [], "durable", "policy.inspect"),
  value("privacy_placeholder_map", { ORDO_PRIVATE_EMAIL_1: "person@example.com" }, [], "durable", "privacy.map.inspect"),
  value("staff_notes", "Staff-only renewal risk note"),
  value("confidence_internals", { score: 0.82, reason: "internal scorer" }, [], "durable", "analysis.inspect"),
  value("token_ledger", { promptTokens: 111, outputTokens: 22 }, [], "durable", "llm.accounting.inspect"),
  value("staff_routing_details", { handoffId: "handoff_1", assignedTo: "actor_staff" }),
  value("accounting_evidence", { estimatedTokens: 133 }, [], "durable", "llm.accounting.inspect"),
  value("artifact_metadata", { artifactId: "artifact_1", title: "Trial proof" }, [], "durable", undefined, "client_1"),
  value("browser_candidate_output", { hash: "abc123" }, [candidateEvidence], "candidate", "browser.candidate.inspect"),
  value("durable_daemon_evidence", { eventId: "event_message_created_1" }, [durableMessageEvidence], "durable", undefined, "client_1"),
];

const editField: AgentEditableField = {
  fieldId: "message_reply_body",
  label: "Reply body",
  kind: "markdown",
  currentValue: "Draft reply",
  target: {
    kind: "command",
    commandKind: "message.submit",
    objectId: "conversation_1",
    path: "/draft/body",
  },
  validation: {
    required: true,
    maxLength: 4000,
  },
};

const action: AgentScreenAction = {
  actionId: "submit_reply",
  label: "Submit reply",
  kind: "command",
  commandKind: "message.submit",
  targetObjectId: "conversation_1",
  evidenceRefs: [durableMessageEvidence],
  constraints: ["Preserve user agency.", "Do not invent offer terms."],
};

const screenSource: AgentScreenContextSource = {
  screenId: "chat.relationship",
  route: "/chat",
  surfaceKind: "relationship_conversation",
  generatedAt: "2026-05-10T00:00:00Z",
  constraints: ["No DOM scraping.", "Only typed command proposals."],
  actions: [action],
  objects: [
    {
      objectId: "conversation_1",
      kind: "conversation",
      title: "Your conversation with Studio Ordo",
      summary: "Trial conversation context.",
      visibility: "client",
      values: sourceValues,
      editableFields: [editField],
      actions: [action],
    },
  ],
};

test.describe("OrdoOS role-safe projections", () => {
  test("projection matrix covers every required category for every role group", () => {
    const requiredCategories: readonly ProjectionCategory[] = [
      "message_text",
      "raw_prompt",
      "provider_payload",
      "policy_internals",
      "privacy_placeholder_map",
      "staff_notes",
      "confidence_internals",
      "token_ledger",
      "staff_routing_details",
      "accounting_evidence",
      "artifact_metadata",
      "browser_candidate_output",
      "durable_daemon_evidence",
    ];

    expect(projectionPolicy.map((entry) => entry.category)).toEqual(requiredCategories);
    for (const entry of projectionPolicy) {
      expect(Object.keys(entry.decisions)).toEqual([
        "public",
        "client",
        "affiliate",
        "staff",
        "manager_admin",
        "owner_system",
      ]);
    }
  });

  test("public and client contexts deny staff, provider, policy, privacy, and token internals", () => {
    for (const role of ["anonymous", "client", "member"] as const) {
      const projected = projectValuesForRole(role, sourceValues, { scopedOwnerId: "client_1" });
      const visibleCategories = projected.visible.map((item) => item.category);
      const deniedCategories = projected.denied.map((item) => item.category);

      expect(visibleCategories).toEqual(["message_text", "artifact_metadata", "durable_daemon_evidence"]);
      expect(deniedCategories).toEqual([
        "raw_prompt",
        "provider_payload",
        "policy_internals",
        "privacy_placeholder_map",
        "staff_notes",
        "confidence_internals",
        "token_ledger",
        "staff_routing_details",
        "accounting_evidence",
        "browser_candidate_output",
      ]);
    }
  });

  test("affiliate projection remains scoped and denies unrelated customer internals", () => {
    const projected = projectValuesForRole("affiliate", sourceValues, { scopedOwnerId: "other_client" });
    const visibleCategories = projected.visible.map((item) => item.category);
    const scopedDenied = projected.denied.filter((item) => item.decision === "scoped").map((item) => item.category);

    expect(visibleCategories).toEqual(["message_text"]);
    expect(scopedDenied).toEqual(["artifact_metadata", "durable_daemon_evidence"]);
    expect(projected.denied.map((item) => item.category)).toContain("staff_notes");
    expect(projected.denied.map((item) => item.category)).toContain("staff_routing_details");
  });

  test("staff can see staff-safe context but not raw provider secrets or privacy maps without gates", () => {
    const projected = projectValuesForRole("staff", sourceValues, { scopedOwnerId: "client_1" });
    const visibleCategories = projected.visible.map((item) => item.category);

    expect(visibleCategories).toEqual([
      "message_text",
      "staff_notes",
      "staff_routing_details",
      "artifact_metadata",
      "durable_daemon_evidence",
    ]);
    expect(projected.denied.map((item) => item.category)).toEqual([
      "raw_prompt",
      "provider_payload",
      "policy_internals",
      "privacy_placeholder_map",
      "confidence_internals",
      "token_ledger",
      "accounting_evidence",
      "browser_candidate_output",
    ]);
  });

  test("owner system context can see owner-scoped operational metadata and candidate labels", () => {
    const projected = projectValuesForRole("owner", sourceValues, {
      scopedOwnerId: "client_1",
      capabilities: ["llm.prompt.inspect", "llm.provider.inspect"],
    });
    const visible = new Set(projected.visible.map((item) => item.category));

    expect(visible.has("raw_prompt")).toBe(true);
    expect(visible.has("provider_payload")).toBe(true);
    expect(visible.has("privacy_placeholder_map")).toBe(true);
    expect(visible.has("token_ledger")).toBe(true);
    expect(visible.has("browser_candidate_output")).toBe(true);
    expect(projected.visible.find((item) => item.category === "browser_candidate_output")?.durability).toBe("candidate");
  });

  test("agent screen context is deterministic structured JSON without DOM or denied values", () => {
    const context = buildAgentScreenContext(screenSource, "client", { scopedOwnerId: "client_1" });
    const serialized = stableAgentScreenContextJson(context);
    const reparsed = JSON.parse(serialized);

    expect(reparsed.schemaVersion).toBe("ordo.agent_screen_context.v1");
    expect(reparsed.route).toBe("/chat");
    expect(reparsed.objects[0].editableFields[0].target.commandKind).toBe("message.submit");
    expect(reparsed.objects[0].actions[0].commandKind).toBe("message.submit");
    expect(serialized).toBe(stableAgentScreenContextJson(context));
    expect(serialized).not.toContain("<div");
    expect(serialized).not.toContain("SYSTEM: hidden prompt");
    expect(serialized).not.toContain("{raw provider payload}");
    expect(serialized).not.toContain("ORDO_PRIVATE_EMAIL_1");
    expect(serialized).not.toContain("Staff-only renewal risk note");
    expect(serialized).not.toContain("promptTokens");
    expect(context.denied.length).toBeGreaterThan(0);
  });

  test("browser candidate output remains candidate until daemon validation", () => {
    const context = buildAgentScreenContext(screenSource, "owner", {
      scopedOwnerId: "client_1",
      capabilities: ["browser.candidate.inspect"],
    });
    const projectedCandidate = context.objects[0]?.values.find((item) => item.category === "browser_candidate_output");

    expect(projectedCandidate?.durability).toBe("candidate");
    expect(context.objects[0]?.durability).toBe("candidate");
  });

  test("unknown projection category fails closed with explicit error", () => {
    expect(projectionDecision("client", "new_unknown_category")).toBe("unknown");
    const projected = projectValuesForRole("client", [
      { category: "new_unknown_category", value: "should not render" },
    ]);

    expect(projected.visible).toEqual([]);
    expect(projected.denied).toHaveLength(1);
    expect(projected.denied[0]?.decision).toBe("unknown");
    expect(projected.denied[0]?.error.kind).toBe("permission_denied");
  });
});

function value(
  category: ProjectionCategory,
  value: unknown,
  evidenceRefs: readonly EvidenceRef[] = [],
  durability: "candidate" | "durable" = "durable",
  requiredCapability?: string,
  scopeOwnerId?: string,
): ProjectionSourceValue {
  return {
    category,
    value,
    evidenceRefs,
    durability,
    requiredCapability,
    scopeOwnerId,
  };
}
