import { ClientConversationBrief, StaffConversationQueues } from "@/components/conversation-foundation";
import { ProductShell } from "@/components/product-shell";
import { PublicSurfaceDeck, type PublicHomeMode } from "@/components/public-surface-deck";
import { mobileStepFromSearchParams, railModeFromSearchParams, roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { publicEntryContextFromSearchParams } from "@/lib/public-entry-context";
import { isStaffRole } from "@/lib/product-navigation";

export default async function ChatPage({ searchParams }: { searchParams?: SearchParams }) {
  const role = await roleFromSearchParams(searchParams);
  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams);
  const params = searchParams ? await searchParams : {};
  const configuredHomeMode = resolveHomeMode(params.home);
  const entryContext = publicEntryContextFromSearchParams(params);

  if (role === "anonymous") {
    return <PublicSurfaceDeck role={role} configuredHomeMode={configuredHomeMode} surfaceMode="chat" entryContext={entryContext} />;
  }

  return (
    <ProductShell
      role={role}
      appSpaceId={isStaffRole(role) ? "staff" : "my-ordo"}
      currentItemId={isStaffRole(role) ? "conversations" : "chat"}
      collapseSectionRail={!isStaffRole(role)}
      railMode={railMode}
      mobileStep={mobileStep}
    >
      {isStaffRole(role) ? <StaffConversationQueues role={role} /> : <ClientConversationBrief />}
    </ProductShell>
  );
}

function resolveHomeMode(rawMode: string | string[] | undefined): PublicHomeMode {
  const mode = Array.isArray(rawMode) ? rawMode[0] : rawMode;
  return mode === "chat" ? "chat" : "story";
}
