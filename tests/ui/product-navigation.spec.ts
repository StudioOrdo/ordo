import { expect, test } from "@playwright/test";

import {
  accessibleAppSpaces,
  appSpaceById,
  appSpaceLabel,
  canAccessAppSpace,
  defaultProductUiNamingSettings,
  productRoles,
  roleFamilyForRole,
  roleFamilyLabel,
  roleHref,
  roleLabel,
  siteRailItems,
} from "@/lib/product-navigation";

test.describe("product app-space navigation", () => {
  test("public users only receive the public site app space", () => {
    expect(accessibleAppSpaces("anonymous").map((space) => space.id)).toEqual(["site"]);
    expect(canAccessAppSpace("anonymous", "my-ordo")).toBe(false);
    expect(canAccessAppSpace("anonymous", "staff")).toBe(false);
    expect(canAccessAppSpace("anonymous", "studio")).toBe(false);
    expect(canAccessAppSpace("anonymous", "owner")).toBe(false);
    expect(canAccessAppSpace("anonymous", "admin")).toBe(false);
  });

  test("authenticated customer roles receive Ordo without support, business, or system workspaces", () => {
    for (const role of ["client", "member", "affiliate"] as const) {
      expect(accessibleAppSpaces(role).map((space) => space.id)).toEqual(["site", "my-ordo"]);
      expect(appSpaceById("site").label).toBe("Site");
      expect(appSpaceById("my-ordo").label).toBe("Ordo");
      expect(canAccessAppSpace(role, "staff")).toBe(false);
      expect(canAccessAppSpace(role, "studio")).toBe(false);
      expect(canAccessAppSpace(role, "owner")).toBe(false);
      expect(canAccessAppSpace(role, "admin")).toBe(false);
    }
  });

  test("staff roles receive support without studio, business, or system governance", () => {
    for (const role of ["staff"] as const) {
      expect(accessibleAppSpaces(role).map((space) => space.id)).toEqual(["site", "my-ordo", "staff"]);
      expect(canAccessAppSpace(role, "studio")).toBe(false);
      expect(canAccessAppSpace(role, "owner")).toBe(false);
      expect(canAccessAppSpace(role, "admin")).toBe(false);
    }
  });

  test("studio operator roles receive studio without support, business, or system governance", () => {
    for (const role of ["studio"] as const) {
      expect(accessibleAppSpaces(role).map((space) => space.id)).toEqual(["site", "my-ordo", "studio"]);
      expect(canAccessAppSpace(role, "staff")).toBe(false);
      expect(canAccessAppSpace(role, "owner")).toBe(false);
      expect(canAccessAppSpace(role, "admin")).toBe(false);
    }
  });

  test("manager roles can cross support and studio without business or system governance", () => {
    for (const role of ["manager"] as const) {
      expect(accessibleAppSpaces(role).map((space) => space.id)).toEqual(["site", "my-ordo", "staff", "studio"]);
      expect(canAccessAppSpace(role, "owner")).toBe(false);
      expect(canAccessAppSpace(role, "admin")).toBe(false);
    }
  });

  test("owner and admin roles can reach all app spaces", () => {
    for (const role of ["owner", "admin"] as const) {
      expect(accessibleAppSpaces(role).map((space) => space.id)).toEqual(["site", "my-ordo", "staff", "studio", "owner", "admin"]);
    }
  });

  test("app spaces expose deterministic left rails", () => {
    expect(siteRailItems.map((item) => item.id)).toEqual(["feed", "chat", "about"]);
    expect(appSpaceById("my-ordo").items.map((item) => item.id)).toEqual(["ordo", "activity", "offers", "requests", "capabilities"]);
    expect(appSpaceById("staff").items.map((item) => item.id)).toEqual(["handoffs", "conversations", "requests", "reviews", "members"]);
    expect(appSpaceById("studio").items.map((item) => item.id)).toEqual([
      "knowledge",
      "factory-jobs",
      "artifacts",
      "publications",
      "templates",
    ]);
    expect(appSpaceById("owner").items.map((item) => item.id)).toEqual([
      "overview",
      "marketing",
      "revenue",
      "offers",
      "affiliates",
      "reports",
    ]);
    expect(appSpaceById("admin").items.map((item) => item.id)).toEqual([
      "health",
      "events",
      "access",
      "providers",
      "hosted-trials",
      "backup",
      "settings",
    ]);
  });

  test("role switch links preserve the current app route", () => {
    expect(roleHref("/studio/knowledge", "staff")).toBe("/studio/knowledge?role=staff");
    expect(roleHref("/studio/knowledge", "anonymous")).toBe("/studio/knowledge");
    expect(productRoles).toContain("affiliate");
    expect(productRoles).toContain("studio");
  });

  test("role and shell labels are customizable without changing permission ids", () => {
    const settings = {
      ...defaultProductUiNamingSettings,
      roleFamilyAliases: {
        ...defaultProductUiNamingSettings.roleFamilyAliases,
        guest: "Visitor",
        authenticated: "Customer",
      },
      roleAliases: {
        ...defaultProductUiNamingSettings.roleAliases,
        client: "Student",
      },
      shellAliases: {
        ...defaultProductUiNamingSettings.shellAliases,
        "my-ordo": "Customer Room",
      },
    };

    expect(roleFamilyForRole("client")).toBe("authenticated");
    expect(roleFamilyLabel("client", settings)).toBe("Customer");
    expect(roleLabel("client", settings)).toBe("Student");
    expect(appSpaceLabel("my-ordo", settings)).toBe("Customer Room");
    expect(canAccessAppSpace("client", "my-ordo")).toBe(true);
    expect(canAccessAppSpace("client", "admin")).toBe(false);
  });
});
