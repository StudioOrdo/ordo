import { ProductShell } from "@/components/product-shell";
import { roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { type ProductRole } from "@/lib/product-navigation";

interface Props {
  searchParams?: SearchParams;
  topItemId: string;
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

export async function ProductSurfacePage({ searchParams, topItemId, title, eyebrow, brief, surfaceBrief, accountTools }: Props) {
  const role = await roleFromSearchParams(searchParams);
  const tools = accountTools?.(role) ?? [];

  return (
    <ProductShell role={role} currentTopItemId={topItemId}>
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
