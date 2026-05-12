import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StudioContentPillarsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "studio",
    itemId: "content-pillars",
    eyebrow: "Studio",
    title: "Content Pillars",
    brief: ["Content pillars organize reusable themes from the knowledgebase.", "They help Ordo produce consistent articles, shorts, briefs, and offer support material.", "Pillars must cite durable knowledge evidence.", "Pillar authoring is deferred."],
  });
}
