import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StudioFactoryJobsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "studio",
    itemId: "factory-jobs",
    eyebrow: "Studio",
    title: "Factory Jobs",
    brief: ["Factory jobs produce artifacts from knowledge and requests.", "Stages should stream through pub/sub and expose progress without fake certainty.", "Examples include short videos, articles, decks, QR cards, and briefs.", "Job creation UI is deferred."],
  });
}
