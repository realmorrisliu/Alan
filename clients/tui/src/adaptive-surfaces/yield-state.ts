import { normalizeYieldKind } from "../yield.js";

export type PendingYieldKind =
  | "confirmation"
  | "structured_input"
  | "dynamic_tool"
  | "custom";

export interface PendingYield {
  requestId: string;
  kind: PendingYieldKind;
  payload: unknown;
}

export function parsePendingYieldKind(kind: unknown): PendingYieldKind {
  const normalized = normalizeYieldKind(kind as never);
  if (normalized === "confirmation") return "confirmation";
  if (normalized === "structured_input") return "structured_input";
  if (normalized === "dynamic_tool") return "dynamic_tool";
  return "custom";
}
