import OwnerItemPage from "@/app/owner/[itemId]/page";
import { type SearchParams } from "@/lib/page-role";

export default async function OwnerPage({ searchParams }: { searchParams?: SearchParams }) {
  return await OwnerItemPage({ params: Promise.resolve({ itemId: "overview" }), searchParams });
}
