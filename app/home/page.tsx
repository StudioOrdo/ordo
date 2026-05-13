import { ProductSurfacePage } from "@/components/product-surface-page";
import { type SearchParams } from "@/lib/page-role";

export default async function HomeSurfacePage({ searchParams }: { searchParams?: SearchParams }) {
  return await ProductSurfacePage({
    searchParams,
    itemId: "home",
    eyebrow: "Home",
    title: "Studio Ordo",
    brief: [
      "Home is the public and member entry point for the business story.",
      "Authenticated users can choose Ordo-first as their default landing behavior.",
      "Review the current business context, then continue to Ordo, Offers, Requests, or Latest.",
      "Home stays outside the staff/admin appliance so visitors can understand the business without system internals.",
      "Future copy comes from public read models and published artifacts.",
      "This page is a shell contract; production Home content is not implemented in this slice.",
    ],
  });
}
