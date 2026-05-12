import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StaffHandoffsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "staff",
    itemId: "handoffs",
    eyebrow: "Staff",
    title: "Handoffs",
    brief: ["Handoffs are durable staff/customer interaction controls.", "Staff can accept, delegate, return to agent, or close where policy allows.", "Customer screens must not expose staff routing or private support mechanics.", "Final gateway command UI is deferred."],
  });
}
