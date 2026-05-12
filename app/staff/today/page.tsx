import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StaffTodayPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "staff",
    itemId: "today",
    eyebrow: "Staff",
    title: "Today",
    brief: ["Staff Today is the relationship and revenue work brief.", "It should prioritize customers, handoffs, trials, feedback, and next actions.", "Every card should be backed by durable conversation or outcome evidence.", "Final live queue ordering is deferred."],
  });
}
