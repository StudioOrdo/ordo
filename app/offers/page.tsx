import { ProductSurfacePage } from "@/components/product-surface-page";
import { type SearchParams } from "@/lib/page-role";

export default async function OffersPage({ searchParams }: { searchParams?: SearchParams }) {
  return await ProductSurfacePage({
    searchParams,
    topItemId: "offers",
    eyebrow: "Offers",
    title: "Offers",
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
