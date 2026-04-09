import React from "react";
import { Box, Text } from "ink";
import type { CurrentRuntimeSummary } from "./runtime-state.js";
import { SummarySurfacePanel } from "./shared.js";

function toolStatusColor(
  status: NonNullable<CurrentRuntimeSummary["recentTool"]>["status"],
): string {
  if (status === "failed") {
    return "red";
  }
  if (status === "completed") {
    return "green";
  }
  return "yellow";
}

export interface RuntimeSurfaceProps {
  summary: CurrentRuntimeSummary;
}

export function RuntimeSurface({ summary }: RuntimeSurfaceProps) {
  return (
    <SummarySurfacePanel title="Runtime">
      <Text>{summary.headline}</Text>
      <Text color="gray">state={summary.shellRunStatus}</Text>
      {summary.activeTool ? (
        <Text color="yellow">Tool: {summary.activeTool.name} running</Text>
      ) : summary.recentTool ? (
        <Text color={toolStatusColor(summary.recentTool.status)}>
          Recent tool: {summary.recentTool.name} {summary.recentTool.status}
          {summary.recentTool.resultPreview
            ? ` | ${summary.recentTool.resultPreview}`
            : ""}
        </Text>
      ) : null}
      {summary.recoverableError ? (
        <Box flexDirection="column">
          <Text color="yellow">Recoverable error</Text>
          <Text color="yellow">{summary.recoverableError.message}</Text>
        </Box>
      ) : null}
      <Text color="gray">{summary.guidance}</Text>
    </SummarySurfacePanel>
  );
}
