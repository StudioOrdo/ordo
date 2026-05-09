import { ClientConversationBrief, StaffConversationQueues } from "@/components/conversation-foundation";
import { ProductShell } from "@/components/product-shell";
import { roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { isStaffRole } from "@/lib/product-navigation";

export default async function ChatPage({ searchParams }: { searchParams?: SearchParams }) {
  const role = await roleFromSearchParams(searchParams);

  return (
    <ProductShell role={role} currentTopItemId="chat" currentStaffItemId="conversations">
      {isStaffRole(role) ? <StaffConversationQueues role={role} /> : <ClientConversationBrief />}
    </ProductShell>
  );
}
