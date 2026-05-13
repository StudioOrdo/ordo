import MyChatPage from "@/app/my/chat/page";
import { type SearchParams } from "@/lib/page-role";

export default async function MyOrdoPage({ searchParams }: { searchParams?: SearchParams }) {
  return await MyChatPage({ searchParams });
}
