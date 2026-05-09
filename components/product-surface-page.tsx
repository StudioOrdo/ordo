import { ProductShell } from "@/components/product-shell";
import { roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { type ProductRole } from "@/lib/product-navigation";

interface Props {
  searchParams?: SearchParams;
  topItemId: string;
  title: string;
  eyebrow: string;
  brief: readonly string[];
  accountTools?: (role: ProductRole) => readonly string[];
}

export async function ProductSurfacePage({ searchParams, topItemId, title, eyebrow, brief, accountTools }: Props) {
  const role = await roleFromSearchParams(searchParams);
  const tools = accountTools?.(role) ?? [];

  return (
    <ProductShell role={role} currentTopItemId={topItemId}>
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

const briefLabels = ["What is happening", "What changed", "What to do next", "Why it matters", "Evidence", "Limitations"];
