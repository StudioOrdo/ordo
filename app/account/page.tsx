import { ProductSurfacePage } from "@/components/product-surface-page";
import { type SearchParams } from "@/lib/page-role";
import { type ProductRole } from "@/lib/product-navigation";

export default async function AccountPage({ searchParams }: { searchParams?: SearchParams }) {
  return await ProductSurfacePage({
    searchParams,
    topItemId: "account",
    eyebrow: "Account",
    title: "Account",
    brief: [
      "Account exposes role-specific tools without moving non-staff users into the appliance.",
      "Client/member tools focus on conversations, offers, deliverables, requests, and settings.",
      "Open the relevant role tool or return to Chat.",
      "Account is the boundary between public participation and role-specific work.",
      "The current role is simulated through the route for implementation validation until auth lands.",
      "Hosted identity, OAuth, and production account management are outside this slice.",
    ],
    accountTools,
  });
}

function accountTools(role: ProductRole): readonly string[] {
  if (role === "affiliate") {
    return ["Affiliate dashboard", "Referral links", "QR card", "Referred leads", "Outcome / commission status", "Approved materials", "Settings"];
  }
  if (role === "staff" || role === "manager" || role === "owner" || role === "admin") {
    return ["Open System", "My profile", "Preferences", "Sign out"];
  }
  return ["My conversations", "My offers", "My deliverables", "My requests", "Settings"];
}
