import { StudioWorkPage } from "@/components/studio-work-page";
import { type SearchParams } from "@/lib/page-role";

export const dynamic = "force-dynamic";

export default async function StudioArtifactsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await StudioWorkPage({
    searchParams,
    currentItemId: "artifacts",
    roomKind: "artifacts",
    title: "Artifacts",
    description: "Durable artifact review state projected from daemon artifact work items.",
  });
}
