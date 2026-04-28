import React from "react";
import { Box, Text } from "ink";
import type { CurrentPlanState } from "./plan-state.js";
import type {
  CurrentRuntimeSummary,
  RuntimeToolState,
} from "./runtime-state.js";

function planItemMarker(
  status: CurrentPlanState["items"][number]["status"],
): string {
  if (status === "completed") {
    return "[x]";
  }
  if (status === "in_progress") {
    return "[~]";
  }
  return "[ ]";
}

function planItemPrefix(
  status: CurrentPlanState["items"][number]["status"],
): string {
  return status === "in_progress" ? "> " : "";
}

function planStatusSummary(plan: CurrentPlanState): string {
  const segments: string[] = [];

  if (plan.statusCounts.in_progress > 0) {
    segments.push(`${plan.statusCounts.in_progress} active`);
  }
  if (plan.statusCounts.pending > 0) {
    segments.push(`${plan.statusCounts.pending} pending`);
  }
  if (plan.statusCounts.completed > 0) {
    segments.push(`${plan.statusCounts.completed} completed`);
  }

  return segments.join(" | ");
}

function prioritizedPlanItem(
  plan: CurrentPlanState,
): CurrentPlanState["items"][number] | null {
  return (
    plan.items.find((item) => item.status === "in_progress") ??
    plan.items.find((item) => item.status === "pending") ??
    plan.items.at(-1) ??
    null
  );
}

function runtimeToolSummary(tool: RuntimeToolState): string {
  const preview = tool.resultPreview ? ` | ${tool.resultPreview}` : "";
  return `tool=${tool.name} ${tool.status}${preview}`;
}

function compactGuidance(guidance: string): string {
  return guidance.replace(/^Next:\s*/i, "");
}

export function buildPlanHudSummary(
  plan: CurrentPlanState | null,
): string | null {
  if (!plan || plan.items.length === 0) {
    return null;
  }

  const statusSummary = planStatusSummary(plan);
  const focusItem = prioritizedPlanItem(plan);
  const focusSummary = focusItem
    ? `${planItemPrefix(focusItem.status)}${planItemMarker(focusItem.status)} ${
        focusItem.content
      }`
    : null;

  return `Plan: ${[statusSummary, focusSummary].filter(Boolean).join(" | ")}`;
}

export function buildRuntimeHudSummary(summary: CurrentRuntimeSummary): string {
  const segments = [summary.headline, `state=${summary.shellRunStatus}`];

  if (summary.activeTool) {
    segments.push(runtimeToolSummary(summary.activeTool));
  } else if (summary.recentTool) {
    segments.push(runtimeToolSummary(summary.recentTool));
  }

  if (summary.recoverableError) {
    segments.push(summary.recoverableError.message);
  }

  segments.push(compactGuidance(summary.guidance));

  return `Runtime: ${segments.join(" | ")}`;
}

export interface SummaryHudProps {
  plan: CurrentPlanState | null;
  runtimeSummary: CurrentRuntimeSummary;
  footerHint: string;
  pending: boolean;
}

export function SummaryHud({
  plan,
  runtimeSummary,
  footerHint,
  pending,
}: SummaryHudProps) {
  const planSummary = buildPlanHudSummary(plan);
  const runtimeSummaryText = buildRuntimeHudSummary(runtimeSummary);
  const statusColor =
    pending || runtimeSummary.recoverableError ? "yellow" : "gray";

  return (
    <Box flexDirection="column" paddingX={1} marginTop={1}>
      {planSummary ? <Text color="cyan">{planSummary}</Text> : null}
      <Text color={statusColor}>{runtimeSummaryText}</Text>
      <Text color="gray">{footerHint}</Text>
    </Box>
  );
}
