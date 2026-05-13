import StudioKnowledgePage from "@/app/studio/knowledge/page";
import { type SearchParams } from "@/lib/page-role";

export default async function StudioPage({ searchParams }: { searchParams?: SearchParams }) {
  return await StudioKnowledgePage({ searchParams });
}
