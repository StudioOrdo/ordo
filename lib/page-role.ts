import { resolveProductRole, type ProductRole } from "@/lib/product-navigation";

export type SearchParams = Promise<Record<string, string | string[] | undefined>>;

export async function roleFromSearchParams(searchParams?: SearchParams): Promise<ProductRole> {
  const params = searchParams ? await searchParams : {};
  return resolveProductRole(params.role);
}
