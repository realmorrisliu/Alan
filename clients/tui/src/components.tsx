import React from "react";
import { Box, Text } from "ink";
import type { EventEnvelope, ToolDecisionAudit } from "./types";
import {
  confirmationSummary,
  normalizeYieldKind,
  structuredQuestions,
  structuredTitle,
} from "./yield";

export interface MessageListProps {
  events: EventEnvelope[];
}

function formatTime(timestampMs: number | undefined): string {
  if (!timestampMs) return "--:--:--";

  const date = new Date(timestampMs);
  const hh = String(date.getHours()).padStart(2, "0");
  const mm = String(date.getMinutes()).padStart(2, "0");
  const ss = String(date.getSeconds()).padStart(2, "0");
  return `${hh}:${mm}:${ss}`;
}

function shortId(value: string | undefined, fallback = "-"): string {
  if (!value) return fallback;
  return value.slice(0, 8);
}

function auditDetail(audit: ToolDecisionAudit | undefined): string | null {
  if (!audit) return null;

  const ruleText = audit.rule_id ? `, rule=${audit.rule_id}` : "";
  const reasonText = audit.reason ? `, reason=${audit.reason}` : "";
  return `${audit.action}/${audit.capability} @ ${audit.policy_source}${ruleText}${reasonText}`;
}

function eventRow(
  key: string,
  timestampMs: number | undefined,
  label: string,
  labelColor: string,
  body: string,
  bodyColor = "white",
  detail?: string | null,
  detailColor = "gray",
) {
  return (
    <Box key={key} flexDirection="column" width="100%">
      <Box width="100%">
        <Text color="gray">[{formatTime(timestampMs)}] </Text>
        <Text color={labelColor} bold>
          {label}
        </Text>
        <Text color={bodyColor}> {body}</Text>
      </Box>
      {detail ? (
        <Box paddingLeft={2} width="100%">
          <Text color={detailColor}>{detail}</Text>
        </Box>
      ) : null}
    </Box>
  );
}

function buildRows(events: EventEnvelope[]) {
  const rows: React.ReactNode[] = [];
  const toolNamesByCallId = new Map<string, string>();
  let turnIndex = 0;

  let textBuffer = "";
  let textKey = "";
  let textTimestamp: number | undefined;

  let thinkingBuffer = "";
  let thinkingKey = "";
  let thinkingTimestamp: number | undefined;

  const flushText = () => {
    if (!textBuffer) return;

    rows.push(
      eventRow(
        `text:${textKey}`,
        textTimestamp,
        "Alan",
        "blue",
        textBuffer,
        "white",
      ),
    );

    textBuffer = "";
    textKey = "";
    textTimestamp = undefined;
  };

  const flushThinking = () => {
    if (!thinkingBuffer) return;

    rows.push(
      eventRow(
        `thinking:${thinkingKey}`,
        thinkingTimestamp,
        "Reasoning",
        "cyan",
        thinkingBuffer,
        "cyan",
      ),
    );

    thinkingBuffer = "";
    thinkingKey = "";
    thinkingTimestamp = undefined;
  };

  for (const envelope of events) {
    if (!envelope.type) {
      continue;
    }

    if (envelope.type === "text_delta") {
      if (envelope.chunk) {
        if (!textKey) {
          textKey = envelope.event_id;
          textTimestamp = envelope.timestamp_ms;
        }
        textBuffer += envelope.chunk;
      }
      if (envelope.is_final) {
        flushText();
      }
      continue;
    }

    if (envelope.type === "thinking_delta") {
      if (envelope.chunk) {
        if (!thinkingKey) {
          thinkingKey = envelope.event_id;
          thinkingTimestamp = envelope.timestamp_ms;
        }
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
      case "turn_started": {
        turnIndex += 1;
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            `Turn ${turnIndex}`,
            "gray",
            "started",
            "gray",
            `turn_id=${envelope.turn_id || "-"}`,
          ),
        );
        break;
      }

      case "turn_completed": {
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            `Turn ${Math.max(turnIndex, 1)}`,
            "green",
            "completed",
            "green",
            envelope.summary ?? null,
            "green",
          ),
        );
        break;
      }

      case "tool_call_started": {
        const callId = envelope.id || envelope.call_id;
        const toolName = envelope.name || envelope.tool_name || "unknown";

        if (callId && toolName !== "unknown") {
          toolNamesByCallId.set(callId, toolName);
        }

        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "Tool",
            "yellow",
            `${toolName} running`,
            "yellow",
            auditDetail(envelope.audit),
          ),
        );
        break;
      }

      case "tool_call_completed": {
        const callId = envelope.id || envelope.call_id;
        const toolName =
          (callId ? toolNamesByCallId.get(callId) : undefined) ||
          envelope.name ||
          envelope.tool_name ||
          "unknown";

        const preview = envelope.result_preview;
        const success = envelope.success;
        const status =
          success === false
            ? "failed"
            : success === true
              ? "success"
              : preview?.trim().toLowerCase().startsWith("error:")
                ? "failed"
              : "completed";
        const color =
          status === "failed"
            ? "red"
            : status === "success"
              ? "green"
              : "green";

        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "Tool",
            color,
            `${toolName} ${status}`,
            color,
            preview || auditDetail(envelope.audit),
          ),
        );
        break;
      }

      case "yield": {
        const yieldKind = normalizeYieldKind(envelope.kind);

        if (yieldKind === "confirmation") {
          rows.push(
            eventRow(
              envelope.event_id,
              envelope.timestamp_ms,
              "Yield",
              "yellow",
              `confirmation (${shortId(envelope.request_id)})`,
              "yellow",
              confirmationSummary(envelope.payload) ||
                "Use /approve, /reject, or /modify <text>",
            ),
          );
          break;
        }

        if (yieldKind === "structured_input") {
          const title = structuredTitle(envelope.payload);
          const questionCount = structuredQuestions(envelope.payload).length;

          rows.push(
            eventRow(
              envelope.event_id,
              envelope.timestamp_ms,
              "Yield",
              "yellow",
              `structured input (${questionCount} question${questionCount === 1 ? "" : "s"})`,
              "yellow",
              title || "Use the Action panel or /answers <json-array>",
            ),
          );
          break;
        }

        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "Yield",
            "yellow",
            `${yieldKind || "pending"} (${shortId(envelope.request_id)})`,
            "yellow",
            "Use /resume <json> to continue.",
          ),
        );
        break;
      }

      case "error":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            envelope.recoverable ? "Warning" : "Error",
            envelope.recoverable ? "yellow" : "red",
            envelope.message || "Unknown error",
            envelope.recoverable ? "yellow" : "red",
          ),
        );
        break;

      case "warning":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "Warning",
            "yellow",
            envelope.message || "Warning",
            "yellow",
          ),
        );
        break;

      case "task_completed":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "Task",
            "green",
            "completed",
            "green",
            envelope.summary || null,
            "green",
          ),
        );
        break;

      case "context_compacted":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "Context",
            "gray",
            "compacted",
            "gray",
          ),
        );
        break;

      case "plan_updated":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "Plan",
            "cyan",
            `updated (${envelope.items?.length || 0} item${
              (envelope.items?.length || 0) === 1 ? "" : "s"
            })`,
            "cyan",
            envelope.explanation || null,
          ),
        );
        break;

      case "session_rolled_back":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "Session",
            "yellow",
            `rolled back ${envelope.turns ?? 0} turn(s)`,
            "yellow",
          ),
        );
        break;

      case "stream_lagged":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "Warning",
            "yellow",
            `stream lagged (skipped ${envelope.skipped ?? 0})`,
            "yellow",
          ),
        );
        break;

      case "skills_loaded":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "Skills",
            "cyan",
            `loaded ${(envelope.skill_ids || []).join(", ") || "none"}`,
            "cyan",
          ),
        );
        break;

      case "dynamic_tools_registered":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "Tools",
            "cyan",
            `dynamic ${(envelope.tool_names || []).join(", ") || "none"}`,
            "cyan",
          ),
        );
        break;

      case "session_created":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "System",
            "cyan",
            `session created (${shortId(envelope.message)})`,
            "cyan",
          ),
        );
        break;

      case "system_message":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "System",
            "cyan",
            envelope.message || "",
            "cyan",
          ),
        );
        break;

      case "system_error":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "System",
            "red",
            envelope.message || "",
            "red",
          ),
        );
        break;

      case "system_warning":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "System",
            "yellow",
            envelope.message || "",
            "yellow",
          ),
        );
        break;

      case "user_message":
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "You",
            "green",
            envelope.message || "",
            "white",
          ),
        );
        break;

      default:
        rows.push(
          eventRow(
            envelope.event_id,
            envelope.timestamp_ms,
            "Event",
            "gray",
            envelope.type,
            "gray",
          ),
        );
        break;
    }
  }

  flushThinking();
  flushText();

  return rows;
}

export function MessageList({ events }: MessageListProps) {
  const rows = buildRows(events);

  return (
    <Box flexDirection="column" width="100%">
      {rows}
    </Box>
  );
}
