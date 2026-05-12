import { resolveProductRole, type ProductRole } from "@/lib/product-navigation";

export type SearchParams = Promise<Record<string, string | string[] | undefined>>;
export type ProductRailMode = "expanded" | "collapsed";
export type ProductMobileStep = "rooms" | "evidence" | "content";

export async function roleFromSearchParams(searchParams?: SearchParams): Promise<ProductRole> {
  const params = searchParams ? await searchParams : {};
  return resolveProductRole(params.role);
}

export async function railModeFromSearchParams(searchParams?: SearchParams): Promise<ProductRailMode> {
  const params = searchParams ? await searchParams : {};
  return resolveRailMode(params.rail);
}

export async function mobileStepFromSearchParams(searchParams?: SearchParams): Promise<ProductMobileStep> {
  const params = searchParams ? await searchParams : {};
  return resolveMobileStep(params.mobile);
}

export async function selectedItemIndexFromSearchParams(searchParams?: SearchParams): Promise<number> {
  const params = searchParams ? await searchParams : {};
  return resolveSelectedItemIndex(params.item);
}

function resolveRailMode(rawMode: string | string[] | undefined): ProductRailMode {
  const mode = Array.isArray(rawMode) ? rawMode[0] : rawMode;
  return mode === "collapsed" ? "collapsed" : "expanded";
}

function resolveMobileStep(rawStep: string | string[] | undefined): ProductMobileStep {
  const step = Array.isArray(rawStep) ? rawStep[0] : rawStep;
  return step === "evidence" || step === "content" ? step : "rooms";
}

function resolveSelectedItemIndex(rawItem: string | string[] | undefined): number {
  const item = Array.isArray(rawItem) ? rawItem[0] : rawItem;
  const parsed = item ? Number.parseInt(item, 10) : 0;
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : 0;
}
