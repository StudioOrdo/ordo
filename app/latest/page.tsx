import { ProductSurfacePage } from "@/components/product-surface-page";
import { type SearchParams } from "@/lib/page-role";

export default async function LatestPage({ searchParams }: { searchParams?: SearchParams }) {
  return await ProductSurfacePage({
    searchParams,
    itemId: "latest",
    eyebrow: "Latest",
    title: "Latest",
    surfaceBrief: {
      title: "Latest surface brief",
      generatedAt: "2026-05-09T09:50:00Z",
      refreshStatus: "idle",
      body: "Latest can show published artifacts, offer changes, asks, and briefs when those items have public-safe evidence.",
      evidenceRefs: ["artifact_deliverables_v25", "surface_brief_jobs_v26"],
      limitations: ["Feed production and external distribution are still out of scope."],
    },
    brief: [
      "Latest will collect public and user-safe updates.",
      "Published artifacts, offer changes, asks, and briefs can appear here when evidence supports them.",
      "Use Latest to understand what changed before continuing to Ordo.",
      "The surface keeps public/member updates separate from appliance events and diagnostic logs.",
      "Future items will derive from published read models, not private staff or owner-only facts.",
      "This slice does not build feed production or external distribution.",
    ],
  });
}
