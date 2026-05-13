import { ProductShell } from "@/components/product-shell";
import { PageTitle, statusClass } from "@/components/system-panels";
import {
  getOfferBuilderSnapshot,
  type OfferBuilderOffer,
  type OfferBuilderReference,
} from "@/lib/daemon-client";
import { mobileStepFromSearchParams, railModeFromSearchParams, roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { isAdminRole, type ProductRole } from "@/lib/product-navigation";
import { notFound } from "next/navigation";

export const dynamic = "force-dynamic";

export default async function OwnerOffersPage({ searchParams }: { searchParams?: SearchParams }) {
  const requestedRole = await roleFromSearchParams(searchParams);
  if (!isAdminRole(requestedRole)) {
    notFound();
  }
  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams);
  const role: ProductRole = requestedRole;
  const snapshot = await getOfferBuilderSnapshot();
  const activeOffer = snapshot.offers[0] ?? null;
  const degraded = Boolean(snapshot.degradedReason);

  return (
    <ProductShell role={role} appSpaceId="owner" currentItemId="offers" railMode={railMode} mobileStep={mobileStep}>
      <PageTitle
        eyebrow="Business"
        title="Offer Builder"
        description="Pilot offer terms, publication readiness, supported primitives, and explicit deferrals."
      />

      <section className="brief-panel">
        <div className="meta-row">
          <span>As of {snapshot.generatedAt ?? snapshot.createdAt}</span>
          <span className={statusClass(degraded ? "error" : activeOffer?.validation.state ?? "empty")}>
            {degraded ? "degraded" : activeOffer?.validation.state ?? "empty"}
          </span>
        </div>
        <ul className="brief-list">
          {summaryLines(snapshot.offers, degraded).map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      </section>

      {snapshot.degradedReason ? (
        <section className="plain-panel">
          <h3 className="panel-title">State</h3>
          <p className="brief-body">{snapshot.degradedReason}</p>
        </section>
      ) : null}

      <section className="plain-panel table-shell">
        <h3 className="panel-title">Pilot Offers</h3>
        <table className="data-table">
          <thead>
            <tr>
              <th>Offer</th>
              <th>Publication</th>
              <th>Trial</th>
              <th>Readiness</th>
              <th>Public Preview</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.offers.length === 0 ? (
              <tr>
                <td colSpan={5} className="table-empty">
                  No durable offers are available.
                </td>
              </tr>
            ) : (
              snapshot.offers.map((offer) => <OfferRow key={offer.offer.id} item={offer} />)
            )}
          </tbody>
        </table>
      </section>

      {activeOffer ? (
        <>
          <ReferencePanel title="Supported References" references={activeOffer.validation.supportedReferences} />
          <ReferencePanel title="Deferred References" references={activeOffer.validation.deferredReferences} />
          <section className="plain-panel">
            <h3 className="panel-title">Validation</h3>
            {activeOffer.validation.blockers.length > 0 ? (
              activeOffer.validation.blockers.map((blocker) => (
                <div key={blocker} className="data-row">
                  <span className="label">Blocker</span>
                  <span className="value">{blocker}</span>
                </div>
              ))
            ) : (
              <div className="data-row">
                <span className="label">Blockers</span>
                <span className="value">none</span>
              </div>
            )}
            {activeOffer.validation.warnings.map((warning) => (
              <div key={warning} className="data-row">
                <span className="label">Warning</span>
                <span className="value">{warning}</span>
              </div>
            ))}
          </section>
        </>
      ) : null}
    </ProductShell>
  );
}

function OfferRow({ item }: { item: OfferBuilderOffer }) {
  return (
    <tr>
      <td>
        <strong>{item.offer.title}</strong>
        <span className="table-subtle">{item.offer.slug}</span>
      </td>
      <td>
        <span className={statusClass(item.offer.publicationState)}>{item.offer.publicationState}</span>
        <span className="table-subtle">
          {item.offer.visibility} / {item.offer.status}
        </span>
      </td>
      <td>
        {item.offer.trialDays} days
        <span className="table-subtle">{termsVersion(item.offer.terms)}</span>
      </td>
      <td>
        <span className={statusClass(item.validation.state)}>{item.validation.state}</span>
        <span className="table-subtle">{item.validation.publishable ? "publishable" : "not publishable"}</span>
      </td>
      <td>{item.publicPreview ? item.publicPreview.summary : "not public"}</td>
    </tr>
  );
}

function ReferencePanel({ title, references }: { title: string; references: OfferBuilderReference[] }) {
  return (
    <section className="plain-panel table-shell">
      <h3 className="panel-title">{title}</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>Reference</th>
            <th>Status</th>
            <th>Evidence</th>
          </tr>
        </thead>
        <tbody>
          {references.map((reference) => (
            <tr key={reference.key}>
              <td>
                <strong>{reference.label}</strong>
                <span className="table-subtle">{reference.detail}</span>
              </td>
              <td>
                <span className={statusClass(reference.status)}>{reference.status}</span>
                {reference.blockedBy ? <span className="table-subtle">{reference.blockedBy}</span> : null}
              </td>
              <td>{reference.evidenceRefs.length > 0 ? reference.evidenceRefs.join(", ") : "not yet available"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

function summaryLines(offers: OfferBuilderOffer[], degraded: boolean): string[] {
  if (degraded) {
    return ["Offer Builder is degraded because the daemon snapshot is unavailable."];
  }
  if (offers.length === 0) {
    return ["No pilot offer has been created yet."];
  }
  const publishable = offers.filter((offer) => offer.validation.publishable).length;
  const blocked = offers.filter((offer) => offer.validation.state === "blocked").length;
  return [
    `${offers.length} durable offer(s) are available for owner review.`,
    `${publishable} offer(s) are publishable from current daemon evidence.`,
    `${blocked} offer(s) are blocked by validation.`,
    "Rewards and product/workforce pack bindings remain explicit deferrals until their ledgers exist.",
  ];
}

function termsVersion(terms: Record<string, unknown>): string {
  return typeof terms.termsVersion === "string" ? terms.termsVersion : "current terms";
}
