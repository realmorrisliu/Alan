import React from "react";
import { Box, Text } from "ink";
import type { EventEnvelope } from "./types";

export interface MessageListProps {
  events: EventEnvelope[];
  maxRows?: number;
  scrollOffset?: number;
  onRowCountChange?: (count: number) => void;
}

function buildRows(events: EventEnvelope[]) {
  const rows: React.ReactNode[] = [];

  let textBuffer = "";
  let textKey = "";
  let thinkingBuffer = "";
  let thinkingKey = "";

  const flushText = () => {
    if (!textBuffer) return;
    rows.push(
      <Box key={`text:${textKey}`}>
        <Text color="blue" bold>
          Alan:{" "}
        </Text>
        <Text>{textBuffer}</Text>
      </Box>,
    );
    textBuffer = "";
    textKey = "";
  };

  const flushThinking = () => {
    if (!thinkingBuffer) return;
    rows.push(
      <Box key={`thinking:${thinkingKey}`}>
        <Text color="cyan">Thinking: </Text>
        <Text color="cyan" italic>
          {thinkingBuffer}
        </Text>
      </Box>,
    );
    thinkingBuffer = "";
    thinkingKey = "";
  };

  for (const envelope of events) {
    if (!envelope.type) {
      continue;
    }

    if (envelope.type === "text_delta") {
      if (envelope.chunk) {
        if (!textKey) textKey = envelope.event_id;
        textBuffer += envelope.chunk;
      }
      if (envelope.is_final) {
        flushText();
      }
      continue;
    }

    if (envelope.type === "thinking_delta") {
      if (envelope.chunk) {
        if (!thinkingKey) thinkingKey = envelope.event_id;
        thinkingBuffer += envelope.chunk;
      }
      if (envelope.is_final) {
        flushThinking();
      }
      continue;
    }

    flushThinking();
    flushText();

    switch (envelope.type) {
      case "turn_started":
      case "turn_completed":
        rows.push(
          <Box key={envelope.event_id} marginY={1}>
            <Text color="gray">───────────────────────────────</Text>
          </Box>,
        );
        break;

      case "tool_call_started":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="yellow">Tool: {envelope.tool_name || "unknown"}</Text>
          </Box>,
        );
        break;

      case "tool_call_completed": {
        const success = envelope.success ?? true;
        rows.push(
          <Box key={envelope.event_id}>
            <Text color={success ? "green" : "red"}>
              {success ? "Success" : "Failed"}:{" "}
              {envelope.tool_name || "unknown"}
            </Text>
          </Box>,
        );
        break;
      }

      case "task_completed":
        rows.push(
          <Box key={envelope.event_id} marginY={1} flexDirection="column">
            <Text color="green" bold>
              Task completed
            </Text>
            {envelope.summary ? (
              <Text color="green">{envelope.summary}</Text>
            ) : null}
          </Box>,
        );
        break;

      case "yield":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="yellow">
              Pending {envelope.kind || "yield"}: {envelope.request_id || "-"}
            </Text>
          </Box>,
        );
        break;

      case "context_compacted":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="gray">Context compacted.</Text>
          </Box>,
        );
        break;

      case "plan_updated":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="cyan">
              Plan updated ({envelope.items?.length || 0} items)
            </Text>
          </Box>,
        );
        break;

      case "session_rolled_back":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="yellow">
              Rolled back {envelope.num_turns ?? 0} turn(s)
            </Text>
          </Box>,
        );
        break;

      case "stream_lagged":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="yellow">
              Event stream lagged (skipped: {envelope.skipped ?? 0})
            </Text>
          </Box>,
        );
        break;

      case "skills_loaded":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="cyan">
              Skills loaded: {(envelope.skill_ids || []).join(", ") || "none"}
            </Text>
          </Box>,
        );
        break;

      case "dynamic_tools_registered":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="cyan">
              Dynamic tools: {(envelope.tool_names || []).join(", ") || "none"}
            </Text>
          </Box>,
        );
        break;

      case "error":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="red">
              {envelope.recoverable ? "Warning" : "Error"}:{" "}
              {envelope.message || "Unknown error"}
            </Text>
          </Box>,
        );
        break;

      case "session_created":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="cyan">
              [System] Session created: {envelope.message?.slice(0, 8)}...
            </Text>
          </Box>,
        );
        break;

      case "system_message":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="cyan">[System] {envelope.message}</Text>
          </Box>,
        );
        break;

      case "system_error":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="red">[System] Error: {envelope.message}</Text>
          </Box>,
        );
        break;

      case "system_warning":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="yellow">[System] Warning: {envelope.message}</Text>
          </Box>,
        );
        break;

      case "user_message":
        rows.push(
          <Box key={envelope.event_id}>
            <Text color="green" bold>
              You:{" "}
            </Text>
            <Text>{envelope.message}</Text>
          </Box>,
        );
        break;

      default:
        break;
    }
  }

  flushThinking();
  flushText();

  return rows;
}

export function MessageList({
  events,
  maxRows,
  scrollOffset = 0,
  onRowCountChange,
}: MessageListProps) {
  const rows = buildRows(events);
  onRowCountChange?.(rows.length);

  let visibleRows = rows;
  if (maxRows && maxRows > 0) {
    const clampedOffset = Math.max(0, scrollOffset);
    const end = Math.max(rows.length - clampedOffset, 0);
    const start = Math.max(0, end - maxRows);
    visibleRows = rows.slice(start, end);
  }

  return (
    <Box flexDirection="column" width="100%">
      {visibleRows}
    </Box>
  );
}
