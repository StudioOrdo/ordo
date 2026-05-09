import { ProductSurfacePage } from "@/components/product-surface-page";
import { type SearchParams } from "@/lib/page-role";

export default async function AsksPage({ searchParams }: { searchParams?: SearchParams }) {
  return await ProductSurfacePage({
    searchParams,
    topItemId: "asks",
    eyebrow: "Asks",
    title: "Asks",
    brief: [
      "Asks describe ways to refer, provide, sell to, or support Studio Ordo.",
      "Future ask briefs will measure responses, referrals, and useful outcomes.",
      "Respond to an ask or continue the relationship conversation.",
      "Asks are measurable business instruments connected to people, artifacts, and outcomes.",
      "Future evidence comes from entry points, visitor sessions, conversations, and referral records.",
      "Affiliate payout automation and external analytics are outside this slice.",
    ],
  });
}
