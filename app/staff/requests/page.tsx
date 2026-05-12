import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function StaffRequestsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "staff",
    itemId: "requests",
    eyebrow: "Support",
    title: "Requests",
    brief: [
      "Support Requests collect customer asks that need a human answer, approval, or escalation.",
      "They must stay tied to the primary relationship conversation and durable evidence.",
      "Staff-only routing and policy details must not leak into customer screens.",
      "Live request queues are deferred until the mock shell shape is accepted.",
    ],
  });
}
