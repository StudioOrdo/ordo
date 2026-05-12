import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function AdminConversationsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "admin",
    itemId: "conversations",
    eyebrow: "System View",
    title: "Conversations",
    brief: [
      "Admin Conversations should show all system conversation streams through role-safe projections.",
      "Selecting one stream should load only that stream in the main content area.",
      "Provider payloads, prompts, private policy mechanics, and privacy maps stay withheld.",
      "Live all-system replay is deferred until the mock shell shape is accepted.",
    ],
  });
}
