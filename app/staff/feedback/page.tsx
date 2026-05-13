import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StaffFeedbackPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "staff",
    itemId: "feedback",
    eyebrow: "Staff",
    title: "Feedback",
    brief: ["Feedback is private business intelligence.", "Review candidates can be derived from feedback only with provenance and consent boundaries.", "Client-visible surfaces must not expose internal classification.", "Review-return workflow UI is deferred."],
  });
}
