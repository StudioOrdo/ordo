import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function AdminEventsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "admin",
    itemId: "events",
    eyebrow: "System View",
    title: "Events",
    brief: ["Events are persisted appliance evidence.", "System View can inspect sequence, cursor, and replay state.", "Client/staff surfaces receive projected read models, not raw event internals.", "Full event table reuse is deferred."],
  });
}
