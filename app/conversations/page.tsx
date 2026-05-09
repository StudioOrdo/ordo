import { StaffConversationQueues } from "@/components/conversation-foundation";
import { ProductShell } from "@/components/product-shell";
import { roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { isStaffRole } from "@/lib/product-navigation";

export default async function ConversationsPage({ searchParams }: { searchParams?: SearchParams }) {
  const role = await roleFromSearchParams(searchParams);

  return (
    <ProductShell role={role} currentTopItemId="chat" currentStaffItemId="conversations">
      {isStaffRole(role) ? (
        <StaffConversationQueues role={role} />
      ) : (
        <section className="brief-panel narrative-brief">
          <span className="eyebrow">Chat</span>
          <h2 className="panel-title">Your conversation with Studio Ordo</h2>
          <p className="muted">Conversation queues are staff work surfaces. Clients keep one relationship conversation.</p>
        </section>
      )}
    </ProductShell>
  );
}
