import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function AccountPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "my-ordo",
    itemId: "account",
    eyebrow: "Account",
    title: "Account",
    brief: [
      "Account exposes identity, access, and security state without moving users into staff, owner, or system shells.",
      "User tools focus on Ordo, activity, offers, capabilities, requests, and preferences.",
      "Account actions should cite explicit auth and access evidence when they become live.",
      "Hosted identity, OAuth, password reset, and production account management remain deferred in this mockup.",
    ],
  });
}
