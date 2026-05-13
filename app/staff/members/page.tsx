import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StaffMembersPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "staff",
    itemId: "members",
    eyebrow: "Support",
    title: "Members",
    brief: [
      "Support Members lists customer, student, affiliate, and prospect relationships.",
      "Each member should expose a safe stage, current request, recent conversation, and next action.",
      "Staff sees support context; customer-safe projections remain separate.",
      "Live relationship read models are deferred until the mock shell shape is accepted.",
    ],
  });
}
