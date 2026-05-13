import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StaffConversationsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "staff",
    itemId: "conversations",
    eyebrow: "Staff",
    title: "Conversations",
    brief: [
      "Staff Conversations should list handed-off customer threads in the evidence column.",
      "Selecting one thread should load only that thread in the main content area.",
      "Customer-visible continuity stays separate from staff-only routing and policy mechanics.",
      "Live daemon-backed queues are deferred until the mock shell shape is accepted.",
    ],
  });
}
