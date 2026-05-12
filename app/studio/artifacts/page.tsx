import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StudioArtifactsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "studio",
    itemId: "artifacts",
    eyebrow: "Studio",
    title: "Artifacts",
    brief: ["Artifacts are the durable outputs of the factory.", "They should know their source knowledge, job, owner, review state, and publication targets.", "Browser-generated output is candidate until daemon validation.", "The artifact library UI is deferred."],
  });
}
