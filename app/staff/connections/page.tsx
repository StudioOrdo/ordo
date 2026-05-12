import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StaffConnectionsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "staff",
    itemId: "connections",
    eyebrow: "Staff",
    title: "Connections",
    brief: ["Connections are relationship rooms for people, companies, affiliates, and other Ordo systems.", "Stages should make business value clear: discovery, considering, trial, customer, advocate, and dormant.", "Private staff notes and Ordo assistance stay staff-only.", "Full live connection cards are deferred."],
  });
}
