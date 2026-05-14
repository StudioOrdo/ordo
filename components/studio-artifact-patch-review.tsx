import { StudioArtifactPatchAcceptForm } from "@/components/studio-artifact-patch-actions";
import { statusClass } from "@/components/system-panels";
import type { StudioArtifactPatchSnapshot } from "@/lib/daemon-client";

export function StudioArtifactPatchReviewPanel({ snapshot }: { snapshot: StudioArtifactPatchSnapshot }) {
  const degraded = Boolean(snapshot.degradedReason);

  return (
    <section className="plain-panel table-shell">
      <div className="meta-row">
        <h3 className="panel-title">Artifact Patch Review</h3>
        <span className={statusClass(degraded ? "error" : snapshot.proposals.length > 0 ? "ready" : "empty")}>
          {degraded ? "degraded" : snapshot.proposals.length > 0 ? "pending" : "empty"}
        </span>
      </div>
      {snapshot.degradedReason ? <p className="brief-body">{snapshot.degradedReason}</p> : null}
      {!snapshot.degradedReason && snapshot.proposals.length === 0 ? (
        <p className="brief-body">No pending governed text artifact patch proposals are available.</p>
      ) : null}
      {snapshot.proposals.length > 0 ? (
        <table className="data-table">
          <thead>
            <tr>
              <th>Proposal</th>
              <th>Review</th>
              <th>Evidence</th>
              <th>Preview</th>
              <th>Accept</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.proposals.map((proposal) => (
              <tr key={proposal.id}>
                <td>
                  <strong>{proposal.sourceArtifactTitle}</strong>
                  <span className="table-subtle">
                    {proposal.sourceArtifactKind}:{proposal.sourceArtifactId}
                  </span>
                  <span className="table-subtle">Version {proposal.sourceVersionId}</span>
                  <span className="table-subtle">Patch {proposal.id}</span>
                </td>
                <td>
                  <span className={statusClass(proposal.reviewState)}>{proposal.reviewState}</span>
                  <span className="table-subtle">{proposal.sourceArtifactVisibility}</span>
                  <span className="table-subtle">
                    {proposal.preview.addedLines} added / {proposal.preview.removedLines} removed / {proposal.preview.hunks} hunk(s)
                  </span>
                  <span className="table-subtle">Reject/defer unavailable</span>
                </td>
                <td>{proposal.evidenceRefs.length > 0 ? proposal.evidenceRefs.join(", ") : "none"}</td>
                <td>
                  <pre className="table-subtle">{proposal.boundedPatchPreview}</pre>
                  {proposal.previewTruncated ? <span className="table-subtle">Preview truncated</span> : null}
                </td>
                <td>
                  <StudioArtifactPatchAcceptForm proposalId={proposal.id} disabled={proposal.reviewState !== "proposed"} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
    </section>
  );
}
