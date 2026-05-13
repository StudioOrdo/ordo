import { StudioWorkPage } from "@/components/studio-work-page";
import { type SearchParams } from "@/lib/page-role";

export const dynamic = "force-dynamic";

export default async function StudioPage({ searchParams }: { searchParams?: SearchParams }) {
  return await StudioWorkPage({
    searchParams,
    currentItemId: "factory-jobs",
    title: "Studio Production",
    description: "Durable production runs, artifacts, evidence, and backed review actions.",
  });
}
