import { ProductSurfacePage } from "@/components/product-surface-page";
import { type SearchParams } from "@/lib/page-role";

export default async function AsksPage({ searchParams }: { searchParams?: SearchParams }) {
  return await ProductSurfacePage({
    searchParams,
    topItemId: "asks",
    eyebrow: "Asks",
    title: "Asks",
    surfaceBrief: {
      title: "Ask surface brief",
      generatedAt: "2026-05-09T09:50:00Z",
      refreshStatus: "queued",
      body: "Asks are ready for evidence-backed responses and referral outcomes, but the ask model remains intentionally lightweight.",
      evidenceRefs: ["ask_beta", "referral_records_v24"],
      limitations: ["No affiliate payout automation or external analytics are included."],
    },
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
