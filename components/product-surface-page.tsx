import { ProductShell } from "@/components/product-shell";
import { mobileStepFromSearchParams, railModeFromSearchParams, roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { type ProductAppSpace, type ProductRole } from "@/lib/product-navigation";

interface Props {
  searchParams?: SearchParams;
  appSpaceId?: ProductAppSpace;
  itemId: string;
  title: string;
  eyebrow: string;
  brief: readonly string[];
  surfaceBrief?: SurfaceBriefFixture;
  accountTools?: (role: ProductRole) => readonly string[];
}

interface SurfaceBriefFixture {
  title: string;
  generatedAt: string;
  refreshStatus: "queued" | "running" | "failed" | "idle";
  body: string;
  evidenceRefs: readonly string[];
  limitations: readonly string[];
}

export async function ProductSurfacePage({ searchParams, appSpaceId = "site", itemId, title, eyebrow, brief, surfaceBrief, accountTools }: Props) {
  const role = await roleFromSearchParams(searchParams);
  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams);
  const params = searchParams ? await searchParams : {};
  const entryPointSlug = firstQueryValue(params.entryPointSlug);
  const visitorSessionId = firstQueryValue(params.visitorSessionId);
  const tools = accountTools?.(role) ?? [];

  return (
    <ProductShell role={role} appSpaceId={appSpaceId} currentItemId={itemId} railMode={railMode} mobileStep={mobileStep}>
      {itemId === "offers" && (entryPointSlug || visitorSessionId) ? (
        <section className="surface-brief-panel" aria-label="Tracked entry context">
          <div className="brief-heading-row">
            <div>
              <span className="eyebrow">Tracked entry context</span>
              <h2 className="panel-title">This offer view has visitor-session evidence.</h2>
            </div>
            <span className="status-pill">Public-safe</span>
          </div>
          <p>
            Ordo can use the recorded entry and session as attribution evidence for fit checks and later outcomes. It does not
            grant rewards or access from a scan alone.
          </p>
          <div className="brief-grid">
            <div className="brief-block">
              <span>Entry</span>
              <p>{entryPointSlug ?? "Unknown public entry"}</p>
            </div>
            <div className="brief-block">
              <span>Visitor session</span>
              <p>{visitorSessionId ? "Recorded" : "Pending"}</p>
            </div>
          </div>
        </section>
      ) : null}
      {surfaceBrief ? <SurfaceBriefPanel brief={surfaceBrief} /> : null}
      <section className="brief-panel narrative-brief">
        <span className="eyebrow">{eyebrow}</span>
        <h2 className="panel-title">{title}</h2>
        <div className="brief-grid">
          {brief.map((text, index) => (
            <div key={text} className="brief-block">
              <span>{briefLabels[index] ?? "Evidence"}</span>
              <p>{text}</p>
            </div>
          ))}
        </div>
      </section>
      {tools.length > 0 ? (
        <section className="plain-panel">
          <h3 className="panel-title">Account Tools</h3>
          <ul className="tool-list">
            {tools.map((tool) => (
              <li key={tool}>{tool}</li>
            ))}
          </ul>
        </section>
      ) : null}
    </ProductShell>
  );
}

function SurfaceBriefPanel({ brief }: { brief: SurfaceBriefFixture }) {
  return (
    <section className="surface-brief-panel" aria-label="Latest completed surface brief">
      <div className="brief-heading-row">
        <div>
          <span className="eyebrow">Latest completed brief</span>
          <h2 className="panel-title">{brief.title}</h2>
        </div>
        <span className="status-pill">Refresh {brief.refreshStatus}</span>
      </div>
      <p>{brief.body}</p>
      <div className="brief-grid">
        <div className="brief-block">
          <span>Evidence</span>
          <p>{brief.evidenceRefs.join(", ")}</p>
        </div>
        <div className="brief-block">
          <span>Limitations</span>
          <p>{brief.limitations.join(" ")}</p>
        </div>
      </div>
      <small>Generated {brief.generatedAt}. The surface remains available while refresh runs.</small>
    </section>
  );
}

const briefLabels = ["What is happening", "What changed", "What to do next", "Why it matters", "Evidence", "Limitations"];

function firstQueryValue(value: string | string[] | undefined): string | undefined {
  return Array.isArray(value) ? value[0] : value;
}
