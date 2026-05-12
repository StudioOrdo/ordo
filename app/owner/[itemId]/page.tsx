import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";
import { appSpaceById } from "@/lib/product-navigation";

type Params = Promise<{ itemId: string }>;

export default async function OwnerItemPage({ params, searchParams }: { params: Params; searchParams?: SearchParams }) {
  const { itemId } = await params;
  const item = appSpaceById("owner").items.find((candidate) => candidate.id === itemId) ?? appSpaceById("owner").items[0];

  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "owner",
    itemId: item.id,
    eyebrow: "Business",
    title: item.label,
    brief: [
      "Business is owner governance: performance, marketing, revenue, affiliates, reports, and decisions.",
      "It is separate from Staff customer operations, Studio production, and System appliance governance.",
      "Every owner decision should cite durable business evidence.",
      "Live owner analytics are deferred until the mock shell proves the shape.",
    ],
  });
}
