import StaffConversationsPage from "@/app/staff/conversations/page";
import { type SearchParams } from "@/lib/page-role";

export default async function StaffPage({ searchParams }: { searchParams?: SearchParams }) {
  return await StaffConversationsPage({ searchParams });
}
