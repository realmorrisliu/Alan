import type { PendingYield } from "../adaptive-surfaces/yield-state.js";
import type { EventEnvelope } from "../types.js";

export type ShellRunStatus =
  | "starting"
  | "ready"
  | "running"
  | "yielded"
  | "error";

export interface RuntimeToolState {
  callId?: string;
  name: string;
  status: "running" | "completed" | "failed";
  resultPreview?: string | null;
}

export interface RecoverableErrorState {
  message: string;
  eventId: string;
  timestampMs: number;
}

export interface CurrentRuntimeSummary {
  headline: string;
  shellRunStatus: ShellRunStatus;
  activeTool: RuntimeToolState | null;
  recentTool: RuntimeToolState | null;
  recoverableError: RecoverableErrorState | null;
  guidance: string;
}

export interface DeriveRuntimeSummaryInput {
  events: EventEnvelope[];
  currentSessionId: string | null;
  shellRunStatus: ShellRunStatus;
  pendingYield: PendingYield | null;
}

function yieldLabel(pendingYield: PendingYield): string {
  if (pendingYield.kind === "confirmation") {
    return "confirmation";
  }
  if (pendingYield.kind === "structured_input") {
    return "structured input";
  }
  if (pendingYield.kind === "dynamic_tool") {
    return "dynamic tool";
  }
  return "custom action";
}

function buildHeadline(
  shellRunStatus: ShellRunStatus,
  pendingYield: PendingYield | null,
  activeTool: RuntimeToolState | null,
  recoverableError: RecoverableErrorState | null,
): string {
  if (pendingYield) {
    return `Waiting on ${yieldLabel(pendingYield)}`;
  }
  if (activeTool) {
    return `Running ${activeTool.name}`;
  }
  if (recoverableError && shellRunStatus === "error") {
    return "Recoverable issue";
  }
  if (shellRunStatus === "starting") {
    return "Starting runtime";
  }
  if (shellRunStatus === "running") {
    return "Runtime active";
  }
  if (shellRunStatus === "yielded") {
    return "Waiting on input";
  }
  if (shellRunStatus === "error") {
    return "Runtime error";
  }
  return "Ready";
}

function buildGuidance(
  pendingYield: PendingYield | null,
  recoverableError: RecoverableErrorState | null,
): string {
  if (pendingYield?.kind === "confirmation") {
    return "Next: resolve in the Action panel or /approve /reject /modify";
  }
  if (pendingYield?.kind === "structured_input") {
    return "Next: answer in the Action panel or /answers <json-array>";
  }
  if (pendingYield) {
    return "Next: use the Action panel or /resume <json>";
  }
  if (recoverableError) {
    return "Next: inspect the warning, correct the input if needed, then retry or continue.";
  }
  return "Next: send a request or use /help.";
}

export function deriveCurrentRuntimeSummary({
  events,
  currentSessionId,
  shellRunStatus,
  pendingYield,
}: DeriveRuntimeSummaryInput): CurrentRuntimeSummary {
  let activeTool: RuntimeToolState | null = null;
  let recentTool: RuntimeToolState | null = null;
  let recoverableError: RecoverableErrorState | null = null;

  if (currentSessionId) {
    const toolNamesByCallId = new Map<string, string>();

    for (const event of events) {
      if (event.session_id !== currentSessionId) {
        continue;
      }

      if (event.type === "turn_started") {
        recoverableError = null;
        continue;
      }

      if (event.type === "turn_completed") {
        activeTool = null;
        recoverableError = null;
        continue;
      }

      if (event.type === "tool_call_started") {
        const callId = event.id || event.call_id;
        const name = event.name || event.tool_name || "unknown";

        if (callId) {
          toolNamesByCallId.set(callId, name);
        }

        activeTool = {
          callId,
          name,
          status: "running",
        };
        recentTool = activeTool;
        continue;
      }

      if (event.type === "tool_call_completed") {
        const callId = event.id || event.call_id;
        const name: string =
          (callId ? toolNamesByCallId.get(callId) : undefined) ||
          event.name ||
          event.tool_name ||
          activeTool?.name ||
          "unknown";
        const status = event.success === false ? "failed" : "completed";

        recentTool = {
          callId,
          name,
          status,
          resultPreview: event.result_preview,
        };

        if (!callId || activeTool?.callId === callId || activeTool?.name === name) {
          activeTool = null;
        }
        continue;
      }

      if (event.type === "error" && event.recoverable && event.message) {
        recoverableError = {
          message: event.message,
          eventId: event.event_id,
          timestampMs: event.timestamp_ms,
        };
      }
    }
  }

  return {
    headline: buildHeadline(
      shellRunStatus,
      pendingYield,
      activeTool,
      recoverableError,
    ),
    shellRunStatus,
    activeTool,
    recentTool,
    recoverableError,
    guidance: buildGuidance(pendingYield, recoverableError),
  };
}
