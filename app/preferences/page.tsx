import { AppPlaceholderPage } from "@/components/app-placeholder-page";
import { type SearchParams } from "@/lib/page-role";

export default async function PreferencesPage({ searchParams }: { searchParams?: SearchParams }) {
  return await AppPlaceholderPage({
    searchParams,
    appSpaceId: "my-ordo",
    itemId: "preferences",
    eyebrow: "Preferences",
    title: "Experience Preferences",
    brief: [
      "Preferences are user/account experience settings, not a path to privileged internals.",
      "Font size, contrast, motion, color-blind mode, density, theme, locale, and performance mode persist as requested settings.",
      "Effective settings are resolved by role and capability before rendering.",
      "The final settings controls are deferred; this page proves the navigation home.",
    ],
    facts: [
      { label: "Stored value", value: "Requested settings only." },
      { label: "Rendered value", value: "Role-constrained effective settings." },
      { label: "Accessibility", value: "Font size, contrast, reduced motion, and color-blind modes are first-class." },
    ],
  });
}
