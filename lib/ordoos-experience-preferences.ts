import {
  defaultExperienceSettings,
  experienceSettingsManifest,
  resolveEffectiveExperienceSettings,
  type EffectiveExperienceSettings,
  type ExperienceResolutionContext,
  type ExperienceSettings,
} from "@/lib/ordoos-experience";
import { createUiError, type UiError } from "@/lib/ordoos-frontend-contracts";

export const EXPERIENCE_PREFERENCES_SCHEMA_VERSION = "ordo.experience_preferences.v1";

export interface ExperiencePreferenceRecord {
  actorId: string;
  schemaVersion: typeof EXPERIENCE_PREFERENCES_SCHEMA_VERSION;
  requested: ExperienceSettings;
  updatedAt: string;
}

export interface ExperiencePreferenceLoadResult {
  actorId: string | null;
  record: ExperiencePreferenceRecord | null;
  requested: ExperienceSettings;
  effective: EffectiveExperienceSettings;
  errors: readonly UiError[];
}

export interface ExperiencePreferenceStorePort {
  load(actorId: string): Promise<unknown | null>;
  save(record: ExperiencePreferenceRecord): Promise<void>;
}

export interface SerializedExperiencePreferenceRecord {
  actorId: string;
  schemaVersion: string;
  requested: ExperienceSettings;
  updatedAt: string;
}

type PartialRequestedSettings = Partial<Record<keyof ExperienceSettings, unknown>>;

export class MemoryExperiencePreferenceStore implements ExperiencePreferenceStorePort {
  private readonly records = new Map<string, SerializedExperiencePreferenceRecord>();

  async load(actorId: string): Promise<unknown | null> {
    return this.records.get(actorId) ?? null;
  }

  async save(record: ExperiencePreferenceRecord): Promise<void> {
    this.records.set(record.actorId, serializeExperiencePreferenceRecord(record));
  }
}

export function anonymousExperiencePreferences(
  context: ExperienceResolutionContext,
): ExperiencePreferenceLoadResult {
  const requested = { ...defaultExperienceSettings };
  return {
    actorId: null,
    record: null,
    requested,
    effective: resolveEffectiveExperienceSettings(requested, context),
    errors: [],
  };
}

export async function loadExperiencePreferences(
  store: ExperiencePreferenceStorePort,
  actorId: string | null | undefined,
  context: ExperienceResolutionContext,
): Promise<ExperiencePreferenceLoadResult> {
  if (!actorId) {
    return anonymousExperiencePreferences(context);
  }

  const rawRecord = await store.load(actorId);
  if (!rawRecord) {
    const requested = { ...defaultExperienceSettings };
    return {
      actorId,
      record: null,
      requested,
      effective: resolveEffectiveExperienceSettings(requested, context),
      errors: [],
    };
  }

  const parsed = parseStoredExperiencePreferenceRecord(actorId, rawRecord);
  return {
    actorId,
    record: parsed.record,
    requested: parsed.requested,
    effective: resolveEffectiveExperienceSettings(parsed.requested, context),
    errors: parsed.errors,
  };
}

export async function saveExperiencePreferences(
  store: ExperiencePreferenceStorePort,
  actorId: string,
  requested: ExperienceSettings,
  updatedAt: string,
): Promise<ExperiencePreferenceRecord> {
  const sanitized = sanitizeRequestedExperienceSettings(requested);
  if (sanitized.errors.length > 0) {
    throw new Error(sanitized.errors.map((error) => error.message).join("; "));
  }

  const record: ExperiencePreferenceRecord = {
    actorId,
    schemaVersion: EXPERIENCE_PREFERENCES_SCHEMA_VERSION,
    requested: sanitized.requested,
    updatedAt,
  };
  await store.save(record);
  return record;
}

export function parseStoredExperiencePreferenceRecord(
  actorId: string,
  rawRecord: unknown,
): { record: ExperiencePreferenceRecord | null; requested: ExperienceSettings; errors: readonly UiError[] } {
  const errors: UiError[] = [];

  if (!isRecord(rawRecord)) {
    errors.push(createUiError("user_input_invalid", "Stored experience preferences are not an object."));
    const requested = { ...defaultExperienceSettings };
    return { record: null, requested, errors };
  }

  const schemaVersion = typeof rawRecord.schemaVersion === "string" ? rawRecord.schemaVersion : null;
  if (schemaVersion !== EXPERIENCE_PREFERENCES_SCHEMA_VERSION) {
    errors.push(createUiError("user_input_invalid", "Stored experience preference schema version is unsupported."));
  }

  const rawRequested = readRequestedPayload(rawRecord);
  const sanitized = sanitizeRequestedExperienceSettings(rawRequested);
  errors.push(...sanitized.errors);

  const updatedAt = typeof rawRecord.updatedAt === "string" && rawRecord.updatedAt ? rawRecord.updatedAt : "unknown";
  const record: ExperiencePreferenceRecord | null =
    errors.length === 0
      ? {
          actorId,
          schemaVersion: EXPERIENCE_PREFERENCES_SCHEMA_VERSION,
          requested: sanitized.requested,
          updatedAt,
        }
      : null;

  return {
    record,
    requested: sanitized.requested,
    errors,
  };
}

export function sanitizeRequestedExperienceSettings(rawSettings: unknown): {
  requested: ExperienceSettings;
  errors: readonly UiError[];
} {
  const errors: UiError[] = [];
  const requested: ExperienceSettings = { ...defaultExperienceSettings };

  if (!isRecord(rawSettings)) {
    errors.push(createUiError("user_input_invalid", "Requested experience settings must be an object."));
    return { requested, errors };
  }

  for (const key of Object.keys(experienceSettingsManifest) as (keyof ExperienceSettings)[]) {
    if (!(key in rawSettings)) {
      continue;
    }
    const value = (rawSettings as PartialRequestedSettings)[key];
    const manifest = experienceSettingsManifest[key];
    if ((manifest.values as readonly unknown[]).includes(value)) {
      requested[key] = value as never;
    } else {
      errors.push(createUiError("user_input_invalid", `Unknown experience setting value for ${key}.`));
    }
  }

  const knownKeys = new Set(Object.keys(experienceSettingsManifest));
  for (const key of Object.keys(rawSettings).sort()) {
    if (!knownKeys.has(key)) {
      errors.push(createUiError("user_input_invalid", `Unknown experience preference setting ${key}.`));
    }
  }

  return { requested, errors };
}

export function serializeExperiencePreferenceRecord(
  record: ExperiencePreferenceRecord,
): SerializedExperiencePreferenceRecord {
  return {
    actorId: record.actorId,
    schemaVersion: record.schemaVersion,
    requested: canonicalizeRequestedSettings(record.requested),
    updatedAt: record.updatedAt,
  };
}

function canonicalizeRequestedSettings(settings: ExperienceSettings): ExperienceSettings {
  return {
    theme: settings.theme,
    density: settings.density,
    motion: settings.motion,
    typeScale: settings.typeScale,
    contrast: settings.contrast,
    evidenceDetail: settings.evidenceDetail,
    privacyDisplay: settings.privacyDisplay,
    performanceMode: settings.performanceMode,
    locale: settings.locale,
    colorBlindMode: settings.colorBlindMode,
    localComputeEnabled: settings.localComputeEnabled,
    gpuVisualsEnabled: settings.gpuVisualsEnabled,
  };
}

function readRequestedPayload(rawRecord: Record<string, unknown>): unknown {
  if ("requested" in rawRecord) {
    return rawRecord.requested;
  }
  if (typeof rawRecord.requestedSettingsJson === "string") {
    try {
      return JSON.parse(rawRecord.requestedSettingsJson);
    } catch {
      return null;
    }
  }
  return null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
