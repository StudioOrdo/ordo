import { MemberOrdoSurface } from "@/components/member-ordo-surface";
import { type SearchParams } from "@/lib/page-role";

export default async function MyOffersPage({ searchParams }: { searchParams?: SearchParams }) {
  return await MemberOrdoSurface({ searchParams, roomId: "offers" });
}
