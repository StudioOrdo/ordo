import MyAffiliatePage from "@/app/my/affiliate/page";
import { type SearchParams } from "@/lib/page-role";

export default async function MyReferralsPage({ searchParams }: { searchParams?: SearchParams }) {
  return await MyAffiliatePage({ searchParams });
}
