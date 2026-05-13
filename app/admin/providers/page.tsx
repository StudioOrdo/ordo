import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function AdminProvidersPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "admin",
    itemId: "providers",
    eyebrow: "System View",
    title: "Providers",
    brief: ["Provider and model configuration is governed system setup.", "No provider secrets or raw payloads belong in UI fixtures.", "Live provider behavior stays behind explicit guards.", "Provider settings UI is deferred."],
  });
}
