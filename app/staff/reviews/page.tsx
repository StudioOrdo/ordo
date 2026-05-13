import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StaffReviewsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "staff",
    itemId: "reviews",
    eyebrow: "Staff",
    title: "Reviews",
    brief: ["Reviews require consent and approval before publication.", "Moderation evidence should be staff/admin inspectable and client-safe when summarized.", "No fake reviews or unsupported proof belongs here.", "Publication UI is deferred."],
  });
}
