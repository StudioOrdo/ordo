import { PublicSurfaceDeck, type PublicHomeMode } from "@/components/public-surface-deck";
import { roleFromSearchParams, type SearchParams } from "@/lib/page-role";

export default async function HomePage({ searchParams }: { searchParams?: SearchParams }) {
  const role = await roleFromSearchParams(searchParams);
  const params = searchParams ? await searchParams : {};
  const configuredHomeMode = resolveHomeMode(params.home);

  return <PublicSurfaceDeck role={role} configuredHomeMode={configuredHomeMode} />;
}

function resolveHomeMode(rawMode: string | string[] | undefined): PublicHomeMode {
  const mode = Array.isArray(rawMode) ? rawMode[0] : rawMode;
  return mode === "chat" ? "chat" : "story";
}
