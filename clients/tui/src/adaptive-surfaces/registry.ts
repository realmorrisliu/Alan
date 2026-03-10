import { confirmationSurface } from "./confirmation-surface.js";
import { customYieldSurface, dynamicToolSurface } from "./generic-surface.js";
import { structuredInputSurface } from "./structured-input-surface.js";
import type { AdaptiveSurfaceDefinition } from "./types.js";
import type { PendingYield, PendingYieldKind } from "./yield-state.js";

export const adaptiveSurfaceRegistry: Record<
  PendingYieldKind,
  AdaptiveSurfaceDefinition
> = {
  confirmation: confirmationSurface,
  structured_input: structuredInputSurface,
  dynamic_tool: dynamicToolSurface,
  custom: customYieldSurface,
};

export function getAdaptiveSurface(
  pendingYield: PendingYield | null,
): AdaptiveSurfaceDefinition | null {
  return pendingYield ? adaptiveSurfaceRegistry[pendingYield.kind] : null;
}
