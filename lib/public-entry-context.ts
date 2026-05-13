export interface PublicEntryContext {
  entryPointSlug?: string;
  visitorSessionId?: string;
}

export function publicEntryContextFromSearchParams(
  params: Record<string, string | string[] | undefined>,
): PublicEntryContext {
  return {
    entryPointSlug: stablePublicIdentifier(firstQueryValue(params.entryPointSlug)),
    visitorSessionId: stablePublicIdentifier(firstQueryValue(params.visitorSessionId)),
  };
}

export function hasPublicEntryContext(context: PublicEntryContext): boolean {
  return Boolean(context.entryPointSlug || context.visitorSessionId);
}

function firstQueryValue(value: string | string[] | undefined): string | undefined {
  return Array.isArray(value) ? value[0] : value;
}

function stablePublicIdentifier(value: string | undefined): string | undefined {
  const normalized = value?.trim();
  if (!normalized || normalized.length > 120) {
    return undefined;
  }
  return /^[A-Za-z0-9_.-]+$/.test(normalized) ? normalized : undefined;
}
