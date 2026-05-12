import AdminHealthPage from "@/app/admin/health/page";
import { type SearchParams } from "@/lib/page-role";

export default async function AdminPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AdminHealthPage({ searchParams });
}
