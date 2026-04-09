import React from "react";
import { Box, Text } from "ink";
import type { CurrentPlanState } from "./plan-state.js";
import { SummarySurfacePanel } from "./shared.js";

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

function planItemColor(
  status: CurrentPlanState["items"][number]["status"],
): string {
  if (status === "completed") {
    return "green";
  }
  if (status === "in_progress") {
    return "cyan";
  }
  return "gray";
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

export interface PlanSurfaceProps {
  plan: CurrentPlanState;
}

export function planItemRowKey(
  item: CurrentPlanState["items"][number],
  index: number,
): string {
  return `${item.id}:${index}`;
}

export function PlanSurface({ plan }: PlanSurfaceProps) {
  return (
    <SummarySurfacePanel title="Plan">
      {plan.explanation ? <Text>{plan.explanation}</Text> : null}
      <Text color="gray">{planStatusSummary(plan)}</Text>
      {plan.items.map((item, index) => (
        <Box key={planItemRowKey(item, index)}>
          <Text color={planItemColor(item.status)}>
            {item.status === "in_progress" ? ">" : " "}{" "}
            {planItemMarker(item.status)} {item.content}
          </Text>
        </Box>
      ))}
    </SummarySurfacePanel>
  );
}
