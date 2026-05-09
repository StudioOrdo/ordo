import { conversationQueues, queueRowsForRole, type ConversationQueueRow } from "@/lib/conversation-product";
import { type ProductRole } from "@/lib/product-navigation";

export function ClientConversationBrief() {
  return (
    <section className="brief-panel narrative-brief" aria-labelledby="client-conversation-title">
      <span className="eyebrow">Chat</span>
      <h2 id="client-conversation-title" className="panel-title">
        Your conversation with Studio Ordo
      </h2>
      <div className="brief-grid">
        <BriefBlock title="What is happening" text="This is the single relationship conversation for your work with Studio Ordo." />
        <BriefBlock title="What changed" text="Internal episodes, handoffs, and staff notes stay behind the scenes unless they become useful deliverables." />
        <BriefBlock title="What to do next" text="Ask a question, review your latest deliverable, or continue the offer conversation already in progress." />
        <BriefBlock title="Why it matters" text="Context stays together so you do not have to choose between support tickets, sales threads, and project notes." />
        <BriefBlock title="Evidence" text="Future messages, offers, asks, artifacts, and handoff receipts will cite durable Ordo evidence." />
        <BriefBlock title="Limitations" text="Realtime messaging is planned in the 0.1.3 implementation arc; this surface is the product contract." />
      </div>
    </section>
  );
}

export function StaffConversationQueues({ role }: { role: ProductRole }) {
  const rows = queueRowsForRole(role);

  return (
    <div className="work-surface">
      <aside className="record-list" aria-label="Conversation queues">
        <span className="eyebrow">Conversations</span>
        <h2>Queues</h2>
        <div className="queue-tabs" role="list" aria-label="Queue availability">
          {conversationQueues.map((queue) => (
            <span key={queue.id} className="queue-pill" data-queue={queue.id}>
              {queue.label}
            </span>
          ))}
        </div>
        <div className="queue-list">
          {rows.map((row) => (
            <QueueRow key={row.id} row={row} />
          ))}
        </div>
      </aside>

      <section className="brief-panel narrative-brief" aria-labelledby="handoff-brief-title">
        <span className="eyebrow">Handoff Brief</span>
        <h2 id="handoff-brief-title" className="panel-title">
          {rows[0]?.connectionLabel ?? "No active handoff"}
        </h2>
        {rows[0] ? <HandoffBrief row={rows[0]} /> : <p className="muted">No queue item is selected.</p>}
      </section>
    </div>
  );
}

function QueueRow({ row }: { row: ConversationQueueRow }) {
  return (
    <article className="queue-row">
      <div>
        <strong>{row.connectionLabel}</strong>
        <span>{row.whyHere}</span>
      </div>
      <dl>
        <div>
          <dt>Urgency</dt>
          <dd>{row.handoff.urgency}</dd>
        </div>
        <div>
          <dt>Status</dt>
          <dd>{row.handoff.status.replaceAll("_", " ")}</dd>
        </div>
        <div>
          <dt>Actions</dt>
          <dd>{row.actionCount}</dd>
        </div>
      </dl>
    </article>
  );
}

function HandoffBrief({ row }: { row: ConversationQueueRow }) {
  return (
    <>
      <div className="brief-grid">
        <BriefBlock title="Why this is here" text={row.whyHere} />
        <BriefBlock title="What changed" text={row.lastMeaningfulChange} />
        <BriefBlock title="Suggested reply" text={row.handoff.suggestedReply} />
        <BriefBlock title="Risk or constraint" text={row.handoff.riskOrConstraint} />
        <BriefBlock title="Evidence" text={row.handoff.evidenceSummary} />
        <BriefBlock title="Allowed context" text={row.handoff.allowedContext.join(", ")} />
      </div>
      <section className="plain-panel compact-panel" aria-labelledby="episode-title">
        <span className="eyebrow">Episode Candidate</span>
        <h3 id="episode-title" className="panel-title">
          {row.episode.title}
        </h3>
        <p className="muted">
          {row.episode.kind} · {row.episode.status} · confidence {Math.round(row.episode.confidence * 100)}%
        </p>
        <p className="muted">Evidence: {row.episode.evidenceRefs.join(", ")}</p>
      </section>
    </>
  );
}

function BriefBlock({ title, text }: { title: string; text: string }) {
  return (
    <div className="brief-block">
      <span>{title}</span>
      <p>{text}</p>
    </div>
  );
}
