import MyAsksPage from "@/app/my/asks/page";
import { type SearchParams } from "@/lib/page-role";

export default async function MyRequestsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await MyAsksPage({ searchParams });
}
