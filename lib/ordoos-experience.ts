import { type ProductRole } from "@/lib/product-navigation";

export type ThemeId = "ai_swiss" | "high_contrast" | "minimal";
export type Density = "compact" | "normal" | "relaxed";
export type MotionMode = "off" | "restrained" | "expressive" | "cinematic";
export type PerformanceMode = "economy" | "standard" | "enhanced" | "cinematic";
export type TypeScale = "sm" | "md" | "lg" | "xl";
export type ContrastMode = "standard" | "high";
export type EvidenceDetail = "brief" | "standard" | "full" | "owner_cockpit";
export type PrivacyDisplay = "client_safe" | "staff_evidence" | "owner_internals";
export type ColorBlindMode = "none" | "deuteranopia" | "protanopia" | "tritanopia";
export type LocaleId = "en-US";

export interface ExperienceSettings {
  theme: ThemeId;
  density: Density;
  motion: MotionMode;
  typeScale: TypeScale;
  contrast: ContrastMode;
  evidenceDetail: EvidenceDetail;
  privacyDisplay: PrivacyDisplay;
  performanceMode: PerformanceMode;
  locale: LocaleId;
  colorBlindMode: ColorBlindMode;
  localComputeEnabled: boolean;
  gpuVisualsEnabled: boolean;
}

export type ExperienceConstraintReason =
  | "role_unavailable"
  | "capability_unavailable"
  | "reduced_motion"
  | "policy";

export interface ExperienceConstraint {
  setting: keyof ExperienceSettings;
  requestedValue: ExperienceSettings[keyof ExperienceSettings];
  effectiveValue: ExperienceSettings[keyof ExperienceSettings];
  reason: ExperienceConstraintReason;
  messageKey: I18nMessageKey;
}

export interface EffectiveExperienceSettings {
  requested: ExperienceSettings;
  effective: ExperienceSettings;
  constraints: readonly ExperienceConstraint[];
}

export interface ExperienceResolutionContext {
  role: ProductRole;
  capabilities?: readonly string[];
  reducedMotionRequired?: boolean;
  policyDisabledSettings?: readonly (keyof ExperienceSettings)[];
}

export interface SettingsManifestEntry<TValue extends string | boolean> {
  setting: keyof ExperienceSettings;
  defaultValue: TValue;
  values: readonly TValue[];
  permissionSensitive: boolean;
}

export type SettingsManifest = {
  [K in keyof ExperienceSettings]: SettingsManifestEntry<ExperienceSettings[K]>;
};

export type TokenGroup =
  | "surface"
  | "text"
  | "border"
  | "focus"
  | "status"
  | "role"
  | "evidence"
  | "spacing"
  | "radius"
  | "motion";

export type SemanticTokenGroup = Record<string, string>;

export type ThemeTokenManifest = Record<TokenGroup, SemanticTokenGroup>;

export interface ThemeManifest {
  id: ThemeId;
  label: string;
  tokens: ThemeTokenManifest;
}

export interface I18nCatalog {
  locale: LocaleId;
  messages: Record<I18nMessageKey, string>;
}

export type I18nMessageKey =
  | "ordo.shell.center_stage"
  | "ordo.shell.composer"
  | "ordo.shell.evidence_actions"
  | "ordo.shell.active_work"
  | "ordo.shell.experience"
  | "ordo.experience.constraint.role_unavailable"
  | "ordo.experience.constraint.capability_unavailable"
  | "ordo.experience.constraint.reduced_motion"
  | "ordo.experience.constraint.policy";

export interface I18nLookupResult {
  key: string;
  locale: LocaleId;
  value: string;
  missing: boolean;
}

export interface AccessibilityProfile {
  reducedMotion: boolean;
  contrast: ContrastMode;
  typeScale: TypeScale;
  density: Density;
  statusPresentation: "motion_and_text" | "text_only";
  liveRegionVerbosity: "status_summary";
}

export const defaultExperienceSettings: ExperienceSettings = {
  theme: "ai_swiss",
  density: "normal",
  motion: "restrained",
  typeScale: "md",
  contrast: "standard",
  evidenceDetail: "standard",
  privacyDisplay: "client_safe",
  performanceMode: "standard",
  locale: "en-US",
  colorBlindMode: "none",
  localComputeEnabled: false,
  gpuVisualsEnabled: false,
};

export const experienceSettingsManifest: SettingsManifest = {
  theme: {
    setting: "theme",
    defaultValue: "ai_swiss",
    values: ["ai_swiss", "high_contrast", "minimal"],
    permissionSensitive: false,
  },
  density: {
    setting: "density",
    defaultValue: "normal",
    values: ["compact", "normal", "relaxed"],
    permissionSensitive: false,
  },
  motion: {
    setting: "motion",
    defaultValue: "restrained",
    values: ["off", "restrained", "expressive", "cinematic"],
    permissionSensitive: false,
  },
  typeScale: {
    setting: "typeScale",
    defaultValue: "md",
    values: ["sm", "md", "lg", "xl"],
    permissionSensitive: false,
  },
  contrast: {
    setting: "contrast",
    defaultValue: "standard",
    values: ["standard", "high"],
    permissionSensitive: false,
  },
  evidenceDetail: {
    setting: "evidenceDetail",
    defaultValue: "standard",
    values: ["brief", "standard", "full", "owner_cockpit"],
    permissionSensitive: true,
  },
  privacyDisplay: {
    setting: "privacyDisplay",
    defaultValue: "client_safe",
    values: ["client_safe", "staff_evidence", "owner_internals"],
    permissionSensitive: true,
  },
  performanceMode: {
    setting: "performanceMode",
    defaultValue: "standard",
    values: ["economy", "standard", "enhanced", "cinematic"],
    permissionSensitive: false,
  },
  locale: {
    setting: "locale",
    defaultValue: "en-US",
    values: ["en-US"],
    permissionSensitive: false,
  },
  colorBlindMode: {
    setting: "colorBlindMode",
    defaultValue: "none",
    values: ["none", "deuteranopia", "protanopia", "tritanopia"],
    permissionSensitive: false,
  },
  localComputeEnabled: {
    setting: "localComputeEnabled",
    defaultValue: false,
    values: [false, true],
    permissionSensitive: false,
  },
  gpuVisualsEnabled: {
    setting: "gpuVisualsEnabled",
    defaultValue: false,
    values: [false, true],
    permissionSensitive: false,
  },
};

export const requiredTokenGroups: readonly TokenGroup[] = [
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
];

export const themeManifests: readonly ThemeManifest[] = [
  {
    id: "ai_swiss",
    label: "AI Swiss",
    tokens: {
      surface: { base: "#f6f8f6", panel: "#ffffff", muted: "#edf2ef" },
      text: { primary: "#191816", muted: "#636a64", subtle: "#858d86" },
      border: { default: "#d8ded8", strong: "#b9c4bd" },
      focus: { ring: "#0f766e" },
      status: { ok: "#166534", warn: "#9a5b10", danger: "#a7392f", neutral: "#636a64" },
      role: { client: "#0f766e", staff: "#315b7d", owner: "#4f46e5" },
      evidence: { durable: "#166534", candidate: "#9a5b10", denied: "#a7392f" },
      spacing: { xs: "4px", sm: "8px", md: "12px", lg: "18px", xl: "28px" },
      radius: { sm: "4px", md: "8px", lg: "12px" },
      motion: { durationFast: "120ms", durationBase: "180ms", easing: "cubic-bezier(0.2, 0, 0, 1)" },
    },
  },
  {
    id: "high_contrast",
    label: "High Contrast",
    tokens: {
      surface: { base: "#ffffff", panel: "#ffffff", muted: "#f0f0f0" },
      text: { primary: "#000000", muted: "#222222", subtle: "#444444" },
      border: { default: "#000000", strong: "#000000" },
      focus: { ring: "#005fcc" },
      status: { ok: "#005a1f", warn: "#7a3d00", danger: "#9b0000", neutral: "#000000" },
      role: { client: "#005fcc", staff: "#6b2d00", owner: "#4b0082" },
      evidence: { durable: "#005a1f", candidate: "#7a3d00", denied: "#9b0000" },
      spacing: { xs: "4px", sm: "8px", md: "12px", lg: "18px", xl: "28px" },
      radius: { sm: "2px", md: "4px", lg: "6px" },
      motion: { durationFast: "0ms", durationBase: "0ms", easing: "linear" },
    },
  },
  {
    id: "minimal",
    label: "Minimal",
    tokens: {
      surface: { base: "#fafafa", panel: "#ffffff", muted: "#eeeeee" },
      text: { primary: "#1a1a1a", muted: "#666666", subtle: "#888888" },
      border: { default: "#dddddd", strong: "#bbbbbb" },
      focus: { ring: "#111111" },
      status: { ok: "#2f6f3e", warn: "#875f1d", danger: "#944039", neutral: "#666666" },
      role: { client: "#555555", staff: "#333333", owner: "#111111" },
      evidence: { durable: "#2f6f3e", candidate: "#875f1d", denied: "#944039" },
      spacing: { xs: "4px", sm: "8px", md: "12px", lg: "16px", xl: "24px" },
      radius: { sm: "0px", md: "2px", lg: "4px" },
      motion: { durationFast: "90ms", durationBase: "140ms", easing: "ease-out" },
    },
  },
];

export const defaultCatalog: I18nCatalog = {
  locale: "en-US",
  messages: {
    "ordo.shell.center_stage": "Center stage",
    "ordo.shell.composer": "Composer",
    "ordo.shell.evidence_actions": "Evidence and actions",
    "ordo.shell.active_work": "Active work",
    "ordo.shell.experience": "Experience",
    "ordo.experience.constraint.role_unavailable": "Unavailable for this role.",
    "ordo.experience.constraint.capability_unavailable": "Capability is not available.",
    "ordo.experience.constraint.reduced_motion": "Reduced motion is required.",
    "ordo.experience.constraint.policy": "Disabled by policy.",
  },
};

export function resolveEffectiveExperienceSettings(
  requested: ExperienceSettings,
  context: ExperienceResolutionContext,
): EffectiveExperienceSettings {
  const effective: ExperienceSettings = { ...requested };
  const constraints: ExperienceConstraint[] = [];
  const capabilities = new Set(context.capabilities ?? []);
  const policyDisabled = new Set(context.policyDisabledSettings ?? []);

  if (!roleCanUseEvidenceDetail(context.role, requested.evidenceDetail)) {
    effective.evidenceDetail = maxEvidenceDetailForRole(context.role);
    constraints.push(constraint("evidenceDetail", requested.evidenceDetail, effective.evidenceDetail, "role_unavailable"));
  }

  if (!roleCanUsePrivacyDisplay(context.role, requested.privacyDisplay)) {
    effective.privacyDisplay = maxPrivacyDisplayForRole(context.role);
    constraints.push(constraint("privacyDisplay", requested.privacyDisplay, effective.privacyDisplay, "role_unavailable"));
  }

  if (context.reducedMotionRequired && requested.motion !== "off") {
    effective.motion = "off";
    constraints.push(constraint("motion", requested.motion, "off", "reduced_motion"));
  }

  if (requested.performanceMode === "cinematic" && !capabilities.has("experience.performance.cinematic")) {
    effective.performanceMode = "enhanced";
    constraints.push(constraint("performanceMode", requested.performanceMode, "enhanced", "capability_unavailable"));
  }

  if (requested.gpuVisualsEnabled && !capabilities.has("experience.gpu_visuals")) {
    effective.gpuVisualsEnabled = false;
    constraints.push(constraint("gpuVisualsEnabled", true, false, "capability_unavailable"));
  }

  if (effective.motion === "off" && effective.gpuVisualsEnabled) {
    effective.gpuVisualsEnabled = false;
    constraints.push(constraint("gpuVisualsEnabled", true, false, "reduced_motion"));
  }

  for (const setting of policyDisabled) {
    const defaultValue = experienceSettingsManifest[setting].defaultValue;
    if (effective[setting] !== defaultValue) {
      const requestedValue = effective[setting];
      (effective as Record<keyof ExperienceSettings, ExperienceSettings[keyof ExperienceSettings]>)[setting] = defaultValue;
      constraints.push(constraint(setting, requestedValue, defaultValue, "policy"));
    }
  }

  return { requested, effective, constraints };
}

export function themeManifestById(theme: ThemeId): ThemeManifest {
  return themeManifests.find((manifest) => manifest.id === theme) ?? themeManifests[0]!;
}

export function themeTokensToCssVariables(theme: ThemeManifest): Record<string, string> {
  const variables: Record<string, string> = {};
  for (const group of requiredTokenGroups) {
    for (const [name, value] of Object.entries(theme.tokens[group])) {
      variables[`--ordo-${kebab(group)}-${kebab(name)}`] = value;
    }
  }
  return variables;
}

export function experienceAttributes(settings: EffectiveExperienceSettings): Record<string, string> {
  return {
    "data-theme": settings.effective.theme,
    "data-density": settings.effective.density,
    "data-motion": settings.effective.motion,
    "data-type-scale": settings.effective.typeScale,
    "data-contrast": settings.effective.contrast,
    lang: settings.effective.locale,
  };
}

export function lookupMessage(
  catalog: I18nCatalog,
  key: string,
  params: Record<string, string | number> = {},
): I18nLookupResult {
  const template = catalog.messages[key as I18nMessageKey];
  if (!template) {
    return {
      key,
      locale: catalog.locale,
      value: `[[missing:${key}]]`,
      missing: true,
    };
  }
  return {
    key,
    locale: catalog.locale,
    value: template.replace(/\{([a-zA-Z0-9_]+)\}/g, (_, name: string) => String(params[name] ?? `{${name}}`)),
    missing: false,
  };
}

export function accessibilityProfile(settings: EffectiveExperienceSettings): AccessibilityProfile {
  return {
    reducedMotion: settings.effective.motion === "off",
    contrast: settings.effective.contrast,
    typeScale: settings.effective.typeScale,
    density: settings.effective.density,
    statusPresentation: settings.effective.motion === "off" ? "text_only" : "motion_and_text",
    liveRegionVerbosity: "status_summary",
  };
}

export function validateThemeManifest(theme: ThemeManifest): readonly string[] {
  const missing: string[] = [];
  for (const group of requiredTokenGroups) {
    if (!theme.tokens[group] || Object.keys(theme.tokens[group]).length === 0) {
      missing.push(group);
    }
  }
  return missing;
}

function roleCanUseEvidenceDetail(role: ProductRole, value: EvidenceDetail): boolean {
  if (value === "owner_cockpit") {
    return role === "owner" || role === "admin";
  }
  if (value === "full") {
    return role === "staff" || role === "manager" || role === "owner" || role === "admin";
  }
  return true;
}

function maxEvidenceDetailForRole(role: ProductRole): EvidenceDetail {
  if (role === "owner" || role === "admin") {
    return "owner_cockpit";
  }
  if (role === "staff" || role === "manager") {
    return "full";
  }
  return "standard";
}

function roleCanUsePrivacyDisplay(role: ProductRole, value: PrivacyDisplay): boolean {
  if (value === "owner_internals") {
    return role === "owner" || role === "admin";
  }
  if (value === "staff_evidence") {
    return role === "staff" || role === "manager" || role === "owner" || role === "admin";
  }
  return true;
}

function maxPrivacyDisplayForRole(role: ProductRole): PrivacyDisplay {
  if (role === "owner" || role === "admin") {
    return "owner_internals";
  }
  if (role === "staff" || role === "manager") {
    return "staff_evidence";
  }
  return "client_safe";
}

function constraint<K extends keyof ExperienceSettings>(
  setting: K,
  requestedValue: ExperienceSettings[K],
  effectiveValue: ExperienceSettings[K],
  reason: ExperienceConstraintReason,
): ExperienceConstraint {
  return {
    setting,
    requestedValue,
    effectiveValue,
    reason,
    messageKey: `ordo.experience.constraint.${reason}`,
  };
}

function kebab(value: string): string {
  return value.replace(/[A-Z]/g, (match) => `-${match.toLowerCase()}`).replaceAll("_", "-");
}
