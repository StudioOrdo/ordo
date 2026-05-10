import { expect, test } from "@playwright/test";

import { defaultExperienceSettings, type ExperienceSettings } from "@/lib/ordoos-experience";
import {
  anonymousExperiencePreferences,
  EXPERIENCE_PREFERENCES_SCHEMA_VERSION,
  loadExperiencePreferences,
  MemoryExperiencePreferenceStore,
  parseStoredExperiencePreferenceRecord,
  sanitizeRequestedExperienceSettings,
  saveExperiencePreferences,
  serializeExperiencePreferenceRecord,
} from "@/lib/ordoos-experience-preferences";

const accessibleRequested: ExperienceSettings = {
  ...defaultExperienceSettings,
  theme: "high_contrast",
  density: "relaxed",
  motion: "off",
  typeScale: "lg",
  contrast: "high",
  colorBlindMode: "deuteranopia",
  locale: "en-US",
  performanceMode: "economy",
};

test.describe("OrdoOS experience preference persistence", () => {
  test("user preferences save and load deterministically", async () => {
    const store = new MemoryExperiencePreferenceStore();

    const saved = await saveExperiencePreferences(
      store,
      "actor_client",
      accessibleRequested,
      "2026-05-10T00:00:00Z",
    );
    const loaded = await loadExperiencePreferences(store, "actor_client", { role: "client" });

    expect(saved.schemaVersion).toBe(EXPERIENCE_PREFERENCES_SCHEMA_VERSION);
    expect(loaded.errors).toEqual([]);
    expect(loaded.record).toEqual(saved);
    expect(loaded.requested).toEqual(accessibleRequested);
    expect(loaded.effective.effective).toMatchObject({
      theme: "high_contrast",
      density: "relaxed",
      motion: "off",
      typeScale: "lg",
      contrast: "high",
      colorBlindMode: "deuteranopia",
      locale: "en-US",
      performanceMode: "economy",
    });
  });

  test("client-safe readback re-resolves owner-only requested settings", async () => {
    const store = new MemoryExperiencePreferenceStore();
    await saveExperiencePreferences(
      store,
      "actor_client",
      {
        ...defaultExperienceSettings,
        evidenceDetail: "owner_cockpit",
        privacyDisplay: "owner_internals",
      },
      "2026-05-10T00:00:00Z",
    );

    const loaded = await loadExperiencePreferences(store, "actor_client", { role: "client" });

    expect(loaded.requested.evidenceDetail).toBe("owner_cockpit");
    expect(loaded.requested.privacyDisplay).toBe("owner_internals");
    expect(loaded.effective.effective.evidenceDetail).toBe("standard");
    expect(loaded.effective.effective.privacyDisplay).toBe("client_safe");
    expect(loaded.effective.constraints).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ setting: "evidenceDetail", reason: "role_unavailable" }),
        expect.objectContaining({ setting: "privacyDisplay", reason: "role_unavailable" }),
      ]),
    );
  });

  test("anonymous defaults work without persisted account preferences", () => {
    const loaded = anonymousExperiencePreferences({ role: "anonymous" });

    expect(loaded.actorId).toBeNull();
    expect(loaded.record).toBeNull();
    expect(loaded.errors).toEqual([]);
    expect(loaded.requested).toEqual(defaultExperienceSettings);
    expect(loaded.effective.effective.privacyDisplay).toBe("client_safe");
  });

  test("malformed stored values fall back safely with explicit errors", () => {
    const parsed = parseStoredExperiencePreferenceRecord("actor_client", {
      actorId: "actor_client",
      schemaVersion: EXPERIENCE_PREFERENCES_SCHEMA_VERSION,
      requested: {
        theme: "unknown-theme",
        typeScale: "huge",
        colorBlindMode: "deuteranopia",
        unsupportedSetting: "value",
      },
      updatedAt: "2026-05-10T00:00:00Z",
    });

    expect(parsed.record).toBeNull();
    expect(parsed.requested.theme).toBe(defaultExperienceSettings.theme);
    expect(parsed.requested.typeScale).toBe(defaultExperienceSettings.typeScale);
    expect(parsed.requested.colorBlindMode).toBe("deuteranopia");
    expect(parsed.errors.map((error) => error.kind)).toEqual([
      "user_input_invalid",
      "user_input_invalid",
      "user_input_invalid",
    ]);
  });

  test("backend-style requestedSettingsJson records can be loaded", () => {
    const parsed = parseStoredExperiencePreferenceRecord("actor_client", {
      actorId: "actor_client",
      schemaVersion: EXPERIENCE_PREFERENCES_SCHEMA_VERSION,
      requestedSettingsJson: JSON.stringify(accessibleRequested),
      updatedAt: "2026-05-10T00:00:00Z",
    });

    expect(parsed.errors).toEqual([]);
    expect(parsed.record?.requested).toEqual(accessibleRequested);
  });

  test("serialization preserves canonical setting order", () => {
    const serialized = serializeExperiencePreferenceRecord({
      actorId: "actor_client",
      schemaVersion: EXPERIENCE_PREFERENCES_SCHEMA_VERSION,
      requested: accessibleRequested,
      updatedAt: "2026-05-10T00:00:00Z",
    });

    expect(Object.keys(serialized.requested)).toEqual([
      "theme",
      "density",
      "motion",
      "typeScale",
      "contrast",
      "evidenceDetail",
      "privacyDisplay",
      "performanceMode",
      "locale",
      "colorBlindMode",
      "localComputeEnabled",
      "gpuVisualsEnabled",
    ]);
  });

  test("sanitizer rejects unknown settings without storing private preference data", () => {
    const sanitized = sanitizeRequestedExperienceSettings({
      ...accessibleRequested,
      rawPrompt: "do not store this",
    });

    expect(sanitized.errors).toEqual([
      expect.objectContaining({
        kind: "user_input_invalid",
        message: "Unknown experience preference setting rawPrompt.",
      }),
    ]);
  });
});
