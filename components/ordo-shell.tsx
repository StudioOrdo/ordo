import type { ReactNode } from "react";

import { composeOrdoShell, isSlotEnabled, type OrdoShellComposition } from "@/lib/ordoos-shell";
import { type ProductRole } from "@/lib/product-navigation";

interface OrdoShellProps {
  role: ProductRole;
  surfaceId: string;
  centerStage: ReactNode;
  composer?: ReactNode;
  evidenceActionRail?: ReactNode;
  activeWorkStrip?: ReactNode;
  experienceMenu?: ReactNode;
}

export function OrdoShell({
  role,
  surfaceId,
  centerStage,
  composer,
  evidenceActionRail,
  activeWorkStrip,
  experienceMenu,
}: OrdoShellProps) {
  const composition = composeOrdoShell(role, surfaceId);

  return (
    <section className="ordo-shell" data-role={role} data-surface={composition.surface.kind} aria-label="Ordo operating surface">
      <header className="ordo-shell-top">
        <div>
          <span className="eyebrow">OrdoOS</span>
          <h1>{composition.surface.label}</h1>
        </div>
        <ShellSlot composition={composition} slot="experience_menu">
          {experienceMenu ?? <span>{composition.role}</span>}
        </ShellSlot>
      </header>

      <div className="ordo-shell-body">
        <ShellSlot composition={composition} slot="center_stage">
          {centerStage}
        </ShellSlot>

        <aside className="ordo-shell-rail" aria-label="Evidence and actions">
          <ShellSlot composition={composition} slot="evidence_action_rail">
            {evidenceActionRail ?? <span>No evidence selected.</span>}
          </ShellSlot>
        </aside>
      </div>

      <ShellSlot composition={composition} slot="active_work_strip">
        {activeWorkStrip ?? <span>Idle</span>}
      </ShellSlot>

      <ShellSlot composition={composition} slot="composer">
        {composer ?? <span>Composer ready.</span>}
      </ShellSlot>
    </section>
  );
}

function ShellSlot({
  composition,
  slot,
  children,
}: {
  composition: OrdoShellComposition;
  slot: Parameters<typeof isSlotEnabled>[1];
  children: ReactNode;
}) {
  if (!isSlotEnabled(composition, slot)) {
    return null;
  }

  return (
    <section className="ordo-shell-slot" data-slot={slot} aria-label={slot.replaceAll("_", " ")}>
      {children}
    </section>
  );
}
