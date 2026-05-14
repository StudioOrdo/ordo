import { ProductShell } from "@/components/product-shell";
import { PageTitle, statusClass } from "@/components/system-panels";
import { getProviderSnapshot, getSystemSnapshot, type ProviderConfigView } from "@/lib/daemon-client";
import { mobileStepFromSearchParams, railModeFromSearchParams, roleFromSearchParams, type SearchParams } from "@/lib/page-role";
import { isAdminRole, type ProductRole } from "@/lib/product-navigation";

export const dynamic = "force-dynamic";

export default async function AdminProvidersPage({ searchParams }: { searchParams?: SearchParams }) {
  const requestedRole = await roleFromSearchParams(searchParams);
  const railMode = await railModeFromSearchParams(searchParams);
  const mobileStep = await mobileStepFromSearchParams(searchParams);
  const role: ProductRole = isAdminRole(requestedRole) ? requestedRole : "owner";
  const [systemSnapshot, providerSnapshot] = await Promise.all([getSystemSnapshot(), getProviderSnapshot()]);
  const readiness = providerSnapshot.readiness;
  const guardStatus = readiness?.liveInvocationEnabled ? "live" : "blocked";

  return (
    <ProductShell role={role} appSpaceId="admin" currentItemId="providers" railMode={railMode} mobileStep={mobileStep}>
      <PageTitle
        eyebrow="System View"
        title="Providers"
        description="Redacted provider readiness for local model configuration and live-call guards."
      />

      <section className="plain-panel">
        <h3 className="panel-title">Readiness</h3>
        {providerSnapshot.degradedReason ? <p className="brief-body">{providerSnapshot.degradedReason}</p> : null}
        <div className="data-row">
          <span className="label">Configured mode</span>
          <span className="value">{readiness?.configuredProviderMode ?? "unavailable"}</span>
        </div>
        <div className="data-row">
          <span className="label">Requested provider</span>
          <span className="value">{readiness?.requestedProviderId ?? "deterministic local"}</span>
        </div>
        <div className="data-row">
          <span className="label">Default provider</span>
          <span className="value">{readiness?.defaultProviderId ?? "none"}</span>
        </div>
        <div className="data-row">
          <span className="label">Credentials</span>
          <span className="value">
            <span className={statusClass(readiness?.credentialsPresent ? "ok" : "warn")}>{readiness?.credentialSource ?? "unavailable"}</span>
          </span>
        </div>
        <div className="data-row">
          <span className="label">Live invocation</span>
          <span className="value">
            <span className={statusClass(guardStatus)}>{guardStatus}</span> {readiness?.liveInvocationGuard ?? "daemon unavailable"}
          </span>
        </div>
        <div className="data-row">
          <span className="label">OpenAI resolver</span>
          <span className="value">
            <span className={statusClass(readiness?.openai.readyForGuardedSmoke ? "ok" : "warn")}>{readiness?.openai.decision ?? "unavailable"}</span>
          </span>
        </div>
      </section>

      <section className="plain-panel table-shell">
        <h3 className="panel-title">Provider Catalog</h3>
        <table className="data-table">
          <thead>
            <tr>
              <th>Provider</th>
              <th>Status</th>
              <th>Credential</th>
              <th>Model</th>
              <th>Model Options</th>
            </tr>
          </thead>
          <tbody>
            {providerSnapshot.providers.length === 0 ? (
              <tr>
                <td colSpan={5} className="table-empty">
                  No provider read model is available.
                </td>
              </tr>
            ) : (
              providerSnapshot.providers.map((provider) => <ProviderRow key={provider.providerId} provider={provider} />)
            )}
          </tbody>
        </table>
      </section>

      <section className="plain-panel">
        <h3 className="panel-title">Evidence</h3>
        <ul className="evidence-list">
          <li>
            <span className="label">Snapshot</span>
            <span className="value">{providerSnapshot.createdAt}</span>
          </li>
          <li>
            <span className="label">Daemon</span>
            <span className="value">{providerSnapshot.daemonUrl}</span>
          </li>
          <li>
            <span className="label">System Health</span>
            <span className="value">{systemSnapshot.health?.status ?? "unavailable"}</span>
          </li>
        </ul>
      </section>
    </ProductShell>
  );
}

function ProviderRow({ provider }: { provider: ProviderConfigView }) {
  const status = provider.enabled ? "enabled" : "disabled";
  const modelOptions = provider.availableModels.map((model) => model.id).join(", ") || "custom local";

  return (
    <tr>
      <td>
        {provider.providerName}
        <div className="muted">{provider.providerId}</div>
      </td>
      <td>
        <span className={statusClass(provider.enabled ? "ok" : "warn")}>{status}</span>
        {provider.defaultProvider ? <span className="status-pill">default</span> : null}
      </td>
      <td>
        <span className={statusClass(provider.apiKey.configured || provider.providerId === "local" ? "ok" : "warn")}>
          {provider.providerId === "local" ? "not required" : provider.apiKey.source}
        </span>
      </td>
      <td>{provider.model ?? "none"}</td>
      <td>{modelOptions}</td>
    </tr>
  );
}
