import { StudioWorkPage } from "@/components/studio-work-page";
import { type SearchParams } from "@/lib/page-role";

export const dynamic = "force-dynamic";

export default async function StudioFactoryJobsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await StudioWorkPage({
    searchParams,
    currentItemId: "factory-jobs",
    roomKind: "runs",
    title: "Factory Jobs",
    description: "Durable Studio production runs projected from the daemon work-item spine.",
  });
}
