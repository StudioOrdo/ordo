import Link from "next/link";

import { ProductShell } from "@/components/product-shell";
import { type ProductRole, roleHref } from "@/lib/product-navigation";

interface PublicFeedProps {
  role: ProductRole;
}

export function PublicFeed({ role }: PublicFeedProps) {
  return (
    <ProductShell role={role} appSpaceId="site" currentItemId="feed">
      <section className="feed-shell" aria-label="Studio Ordo public feed">
        {feedItems.map((item, index) => (
          <article key={item.id} className={`feed-card feed-card-${item.kind}`}>
            <div className="feed-index">{String(index + 1).padStart(2, "0")}</div>
            <div className="feed-media" aria-hidden="true">
              <span>{item.visual}</span>
            </div>
            <div className="feed-copy">
              <span className="eyebrow">{item.kicker}</span>
              <h1>{item.title}</h1>
              <p>{item.body}</p>
              <div className="feed-proof">
                {item.proof.map((proof) => (
                  <span key={proof}>{proof}</span>
                ))}
              </div>
              <div className="hero-actions">
                {item.actions.map((action) => (
                  <Link key={action.href} href={roleHref(action.href, role)} className={action.primary ? "primary-action" : "secondary-action"}>
                    {action.label}
                  </Link>
                ))}
              </div>
            </div>
          </article>
        ))}
      </section>
    </ProductShell>
  );
}

const feedItems = [
  {
    id: "ordo-opening",
    kind: "about",
    kicker: "Studio Ordo",
    title: "A business operating surface that starts with conversation.",
    body: "Meet someone, share a QR code, keep the relationship alive, and let Ordo turn evidence into useful next actions.",
    visual: "SO",
    proof: ["Ordo-first", "Evidence-backed", "Role-safe"],
    actions: [
      { label: "Start chat", href: "/chat", primary: true },
      { label: "About Ordo", href: "/about", primary: false },
    ],
  },
  {
    id: "offer-trial",
    kind: "offer",
    kicker: "Offer",
    title: "Try OrdoStudio for 30 days.",
    body: "A focused trial for solopreneurs who need customer conversations, offers, referrals, and content production to move together.",
    visual: "30",
    proof: ["No fake urgency", "Trial evidence", "Plain-language fit check"],
    actions: [
      { label: "Ask if it fits", href: "/chat", primary: true },
      { label: "See offer context", href: "/offers", primary: false },
    ],
  },
  {
    id: "ask-affiliate",
    kind: "ask",
    kicker: "Ask",
    title: "Send one good person back to us.",
    body: "Affiliates and friends can use tracked links and QR codes so useful introductions get clear attribution.",
    visual: "QR",
    proof: ["Tracked entry", "Referral evidence", "Reward-ready"],
    actions: [
      { label: "Open affiliate path", href: "/my/affiliate", primary: true },
      { label: "Talk to Ordo", href: "/chat", primary: false },
    ],
  },
  {
    id: "factory-output",
    kind: "factory",
    kicker: "Factory",
    title: "Knowledge turns into artifacts.",
    body: "Articles, short videos, briefs, QR cards, media, and offer support material should come from the knowledgebase and production jobs.",
    visual: "FX",
    proof: ["Knowledge source", "Job stages", "Published artifact"],
    actions: [
      { label: "Open Studio", href: "/studio/knowledge", primary: true },
      { label: "View chat", href: "/chat", primary: false },
    ],
  },
  {
    id: "customer-loop",
    kind: "review",
    kicker: "Loop",
    title: "Feedback becomes private intelligence before it becomes public proof.",
    body: "Review requests stay governed. Consent, approval, and publication boundaries keep the public story clean.",
    visual: "RV",
    proof: ["Consent first", "Approval required", "No fake proof"],
    actions: [
      { label: "Return to chat", href: "/chat", primary: true },
      { label: "Latest proof", href: "/feed", primary: false },
    ],
  },
] as const;
