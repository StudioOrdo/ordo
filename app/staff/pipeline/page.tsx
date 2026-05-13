import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StaffPipelinePage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "staff",
    itemId: "pipeline",
    eyebrow: "Staff",
    title: "Pipeline",
    brief: ["Pipeline names where relationships are in the business journey.", "Visitor, discovery, considering, trial, customer, and advocate should be simple and evidence-backed.", "Affiliate is a role or capability, not a sales stage.", "Revenue analytics and forecasting are deferred."],
  });
}
