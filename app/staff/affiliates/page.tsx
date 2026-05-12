import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StaffAffiliatesPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "staff",
    itemId: "affiliates",
    eyebrow: "Staff",
    title: "Affiliates",
    brief: ["Staff can inspect affiliate connections, referrals, and success evidence.", "Rewards must cite referral entry points, outcomes, and scoped grants.", "Affiliate visibility must not leak unrelated customer or admin internals.", "Reward policy automation is deferred."],
  });
}
