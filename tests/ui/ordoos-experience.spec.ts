import { expect, test } from "@playwright/test";

import {
  accessibilityProfile,
  defaultCatalog,
  defaultExperienceSettings,
  experienceAttributes,
  lookupMessage,
  requiredTokenGroups,
  resolveEffectiveExperienceSettings,
  themeManifestById,
  themeManifests,
  themeTokensToCssVariables,
  validateThemeManifest,
  type ExperienceSettings,
} from "@/lib/ordoos-experience";

const ownerRequested: ExperienceSettings = {
  ...defaultExperienceSettings,
  evidenceDetail: "owner_cockpit",
  privacyDisplay: "owner_internals",
  motion: "cinematic",
  performanceMode: "cinematic",
  gpuVisualsEnabled: true,
};

test.describe("OrdoOS experience substrate", () => {
  test("requested settings resolve deterministically into effective settings", () => {
    const first = resolveEffectiveExperienceSettings(ownerRequested, {
      role: "owner",
      capabilities: ["experience.performance.cinematic", "experience.gpu_visuals"],
    });
    const second = resolveEffectiveExperienceSettings(ownerRequested, {
      role: "owner",
      capabilities: ["experience.performance.cinematic", "experience.gpu_visuals"],
    });

    expect(first).toEqual(second);
    expect(first.constraints).toEqual([]);
    expect(first.effective.evidenceDetail).toBe("owner_cockpit");
    expect(first.effective.privacyDisplay).toBe("owner_internals");
  });

  test("client role cannot enable owner-only evidence or privacy internals", () => {
    const resolved = resolveEffectiveExperienceSettings(ownerRequested, { role: "client" });

    expect(resolved.effective.evidenceDetail).toBe("standard");
    expect(resolved.effective.privacyDisplay).toBe("client_safe");
    expect(resolved.constraints).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          setting: "evidenceDetail",
          requestedValue: "owner_cockpit",
          effectiveValue: "standard",
          reason: "role_unavailable",
        }),
        expect.objectContaining({
          setting: "privacyDisplay",
          requestedValue: "owner_internals",
          effectiveValue: "client_safe",
          reason: "role_unavailable",
        }),
      ]),
    );
  });

  test("unavailable performance and GPU settings produce explicit constraints", () => {
    const resolved = resolveEffectiveExperienceSettings(ownerRequested, {
      role: "owner",
      capabilities: [],
    });

    expect(resolved.effective.performanceMode).toBe("enhanced");
    expect(resolved.effective.gpuVisualsEnabled).toBe(false);
    expect(resolved.constraints).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ setting: "performanceMode", reason: "capability_unavailable" }),
        expect.objectContaining({ setting: "gpuVisualsEnabled", reason: "capability_unavailable" }),
      ]),
    );
  });

  test("reduced motion constrains motion while preserving status semantics", () => {
    const resolved = resolveEffectiveExperienceSettings(ownerRequested, {
      role: "owner",
      capabilities: ["experience.performance.cinematic", "experience.gpu_visuals"],
      reducedMotionRequired: true,
    });
    const profile = accessibilityProfile(resolved);

    expect(resolved.effective.motion).toBe("off");
    expect(resolved.effective.gpuVisualsEnabled).toBe(false);
    expect(profile.reducedMotion).toBe(true);
    expect(profile.statusPresentation).toBe("text_only");
    expect(profile.liveRegionVerbosity).toBe("status_summary");
  });

  test("required token groups exist for every implemented theme", () => {
    expect(requiredTokenGroups).toEqual([
      "surface",
      "text",
      "border",
      "focus",
      "status",
      "role",
      "evidence",
      "spacing",
      "radius",
      "motion",
    ]);
    for (const theme of themeManifests) {
      expect(validateThemeManifest(theme)).toEqual([]);
    }
  });

  test("token runtime emits semantic CSS variables", () => {
    const variables = themeTokensToCssVariables(themeManifestById("ai_swiss"));

    expect(variables["--ordo-surface-base"]).toBe("#f6f8f6");
    expect(variables["--ordo-text-primary"]).toBe("#191816");
    expect(variables["--ordo-status-danger"]).toBe("#a7392f");
    expect(variables["--ordo-role-owner"]).toBe("#4f46e5");
    expect(variables["--ordo-motion-duration-base"]).toBe("180ms");
  });

  test("experience attributes expose settings without component-local styling", () => {
    const resolved = resolveEffectiveExperienceSettings(defaultExperienceSettings, { role: "client" });

    expect(experienceAttributes(resolved)).toEqual({
      "data-theme": "ai_swiss",
      "data-density": "normal",
      "data-motion": "restrained",
      "data-type-scale": "md",
      "data-contrast": "standard",
      lang: "en-US",
    });
  });

  test("i18n lookup is deterministic and reports missing keys explicitly", () => {
    const found = lookupMessage(defaultCatalog, "ordo.shell.composer");
    const missing = lookupMessage(defaultCatalog, "ordo.missing.key");

    expect(found).toEqual({
      key: "ordo.shell.composer",
      locale: "en-US",
      value: "Composer",
      missing: false,
    });
    expect(missing).toEqual({
      key: "ordo.missing.key",
      locale: "en-US",
      value: "[[missing:ordo.missing.key]]",
      missing: true,
    });
  });

  test("accessibility profile records contrast, type scale, and density", () => {
    const resolved = resolveEffectiveExperienceSettings(
      {
        ...defaultExperienceSettings,
        contrast: "high",
        typeScale: "lg",
        density: "relaxed",
      },
      { role: "client" },
    );
    const profile = accessibilityProfile(resolved);

    expect(profile.contrast).toBe("high");
    expect(profile.typeScale).toBe("lg");
    expect(profile.density).toBe("relaxed");
    expect(profile.statusPresentation).toBe("motion_and_text");
  });
});
