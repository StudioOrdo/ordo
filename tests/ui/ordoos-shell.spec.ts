import { expect, test } from "@playwright/test";

import {
  allShellSlots,
  composeOrdoShell,
  isSlotEnabled,
  resolveOrdoSurface,
  resolveRootSurfaceForRole,
} from "@/lib/ordoos-shell";

test.describe("OrdoOS shell substrate", () => {
  test("surface registry resolves known surfaces deterministically", () => {
    expect(resolveOrdoSurface("chat", "client")).toMatchObject({
      fallback: false,
      surface: { kind: "chat_reference", route: "/chat" },
    });
    expect(resolveOrdoSurface("system", "owner")).toMatchObject({
      fallback: false,
      surface: { kind: "system", route: "/" },
    });
    expect(resolveOrdoSurface("staff-cockpit", "staff")).toMatchObject({
      fallback: false,
      surface: { kind: "staff_cockpit", route: "/conversations" },
    });
  });

  test("unknown and denied surfaces resolve to explicit placeholder fallback", () => {
    expect(resolveOrdoSurface("missing", "client")).toMatchObject({
      fallback: true,
      deniedReason: "unknown_surface",
      surface: { kind: "placeholder" },
    });
    expect(resolveOrdoSurface("system", "client")).toMatchObject({
      fallback: true,
      deniedReason: "role_not_allowed",
      surface: { kind: "placeholder" },
    });
  });

  test("root routing rule is chat-first except owner/system routes", () => {
    expect(resolveRootSurfaceForRole("anonymous").kind).toBe("chat_reference");
    expect(resolveRootSurfaceForRole("client").kind).toBe("chat_reference");
    expect(resolveRootSurfaceForRole("member").kind).toBe("chat_reference");
    expect(resolveRootSurfaceForRole("affiliate").kind).toBe("chat_reference");
    expect(resolveRootSurfaceForRole("staff").kind).toBe("chat_reference");
    expect(resolveRootSurfaceForRole("manager").kind).toBe("chat_reference");
    expect(resolveRootSurfaceForRole("owner").kind).toBe("system");
    expect(resolveRootSurfaceForRole("admin").kind).toBe("system");
  });

  test("public and client shell composition does not expose staff or system navigation", () => {
    for (const role of ["anonymous", "client", "member", "affiliate"] as const) {
      const composition = composeOrdoShell(role, "chat");

      expect(composition.showStaffNavigation).toBe(false);
      expect(composition.showSystemNavigation).toBe(false);
      expect(isSlotEnabled(composition, "center_stage")).toBe(true);
      expect(isSlotEnabled(composition, "composer")).toBe(true);
      expect(isSlotEnabled(composition, "evidence_action_rail")).toBe(true);
      expect(isSlotEnabled(composition, "active_work_strip")).toBe(true);
      expect(isSlotEnabled(composition, "experience_menu")).toBe(true);
    }
  });

  test("staff and admin shell composition exposes role-appropriate operating slots", () => {
    const staff = composeOrdoShell("staff", "staff-cockpit");
    const admin = composeOrdoShell("admin", "staff-cockpit");

    expect(staff.showStaffNavigation).toBe(true);
    expect(staff.showSystemNavigation).toBe(false);
    expect(admin.showStaffNavigation).toBe(true);
    expect(admin.showSystemNavigation).toBe(true);
    expect(isSlotEnabled(staff, "composer")).toBe(true);
    expect(isSlotEnabled(admin, "evidence_action_rail")).toBe(true);
  });

  test("owner system shell exposes system surface without composer by default", () => {
    const owner = composeOrdoShell("owner", "system");

    expect(owner.surface.kind).toBe("system");
    expect(owner.showSystemNavigation).toBe(true);
    expect(isSlotEnabled(owner, "center_stage")).toBe(true);
    expect(isSlotEnabled(owner, "evidence_action_rail")).toBe(true);
    expect(isSlotEnabled(owner, "active_work_strip")).toBe(true);
    expect(isSlotEnabled(owner, "experience_menu")).toBe(true);
    expect(isSlotEnabled(owner, "composer")).toBe(false);
  });

  test("composer remains present while active work strip reports background work", () => {
    const composition = composeOrdoShell("client", "chat");
    const enabledSlots = composition.slots.filter((slot) => slot.enabled).map((slot) => slot.kind);

    expect(enabledSlots).toEqual(allShellSlots());
    expect(isSlotEnabled(composition, "composer")).toBe(true);
    expect(isSlotEnabled(composition, "active_work_strip")).toBe(true);
  });

  test("shell slot output is deterministic and role-safe", () => {
    const first = composeOrdoShell("client", "chat");
    const second = composeOrdoShell("client", "chat");

    expect(first).toEqual(second);
    expect(JSON.stringify(first)).not.toContain("system");
    expect(JSON.stringify(first)).not.toContain("staff_cockpit");
  });
});
