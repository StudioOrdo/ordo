import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StudioKnowledgePage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "studio",
    itemId: "knowledge",
    eyebrow: "Studio",
    title: "Knowledge",
    brief: ["Studio starts with managed knowledge: business facts, source material, transcripts, and proof.", "Knowledge becomes the source for content pillars, jobs, artifacts, and publications.", "Private and unpublished knowledge remains role-safe.", "Knowledge editing workflows are deferred."],
  });
}
