import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StudioTemplatesPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "studio",
    itemId: "templates",
    eyebrow: "Studio",
    title: "Templates",
    brief: ["Templates define repeatable production formats.", "A narrated 30-second concept video, article, brief, or deck can become a governed artifact request type.", "Templates should declare inputs, evidence needs, and output constraints.", "Template authoring is deferred."],
  });
}
