import { ProductSurfacePage } from "@/components/product-surface-page";
import { type SearchParams } from "@/lib/page-role";

export default async function LatestPage({ searchParams }: { searchParams?: SearchParams }) {
  return await ProductSurfacePage({
    searchParams,
    topItemId: "latest",
    eyebrow: "Latest",
    title: "Latest",
    brief: [
      "Latest will collect public and member-safe updates.",
      "Published artifacts, offer changes, asks, and briefs can appear here when evidence supports them.",
      "Use Latest to understand what changed before continuing to Chat.",
      "The surface keeps public/member updates separate from appliance events and diagnostic logs.",
      "Future items will derive from published read models, not private staff or owner-only facts.",
      "This slice does not build feed production or external distribution.",
    ],
  });
}
