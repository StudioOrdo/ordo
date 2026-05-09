import { ProductSurfacePage } from "@/components/product-surface-page";
import { type SearchParams } from "@/lib/page-role";

export default async function OffersPage({ searchParams }: { searchParams?: SearchParams }) {
  return await ProductSurfacePage({
    searchParams,
    topItemId: "offers",
    eyebrow: "Offers",
    title: "Offers",
    surfaceBrief: {
      title: "Offer surface brief",
      generatedAt: "2026-05-09T09:50:00Z",
      refreshStatus: "running",
      body: "The Starter offer has public read-model evidence and outcome attribution can now cite offer, visitor session, and entry point ids.",
      evidenceRefs: ["offer_starter", "business_outcome_attribution_v24"],
      limitations: ["No payment processing or external CRM evidence is included."],
    },
    brief: [
      "Offers describe ways to buy from Studio Ordo.",
      "Future offer briefs will connect views, conversations, referrals, artifacts, and outcomes.",
      "Choose an offer or ask Ordo what fits your situation.",
      "Offers are measurable business instruments, not static marketing cards.",
      "Current backend public offer read models exist; this shell preserves the product route.",
      "Payments, hosted trial orchestration, and external follow-up are outside this slice.",
    ],
  });
}
