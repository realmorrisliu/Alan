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
  if (recoverableError && shellRunStatus === "error") {
    return "Recoverable issue";
  }
  if (shellRunStatus === "error") {
    return "Runtime error";
  }
  if (pendingYield) {
    return `Waiting on ${yieldLabel(pendingYield)}`;
  }
  if (activeTool) {
    return `Running ${activeTool.name}`;
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
  return "Ready";
}

function buildGuidance(
  shellRunStatus: ShellRunStatus,
  pendingYield: PendingYield | null,
  recoverableError: RecoverableErrorState | null,
): string {
  if (shellRunStatus === "error") {
    if (recoverableError) {
      return "Next: inspect the warning, correct the input if needed, then retry or continue.";
    }
    return "Next: inspect the latest error and reconnect or retry the session.";
  }
  if (pendingYield?.kind === "confirmation") {
    return "Next: resolve in the Action panel or /approve /reject /modify";
  }
  if (pendingYield?.kind === "structured_input") {
    return "Next: answer in the Action panel or /answers <json-array>";
  }
  if (pendingYield) {
    return "Next: use the Action panel or /resume <json>";
  }
  return "Next: send a request or use /help.";
}

function normalizeIncompleteTool(
  tool: RuntimeToolState | null,
  resultPreview: string | null,
): RuntimeToolState | null {
  if (!tool || tool.status !== "running") {
    return tool;
  }

  return {
    ...tool,
    status: "failed",
    resultPreview: tool.resultPreview ?? resultPreview,
  };
}

function completionFallbackStatus(
  event: EventEnvelope,
): RuntimeToolState["status"] {
  if (event.success === false) {
    return "failed";
  }
  if (event.success === true) {
    return "completed";
  }

  const preview = event.result_preview?.trim().toLowerCase();
  if (preview?.startsWith("error:")) {
    return "failed";
  }

  return "completed";
}

function resolveCompletionToolName(
  event: EventEnvelope,
  toolNamesByCallId: Map<string, string>,
  activeTool: RuntimeToolState | null,
  recentTool: RuntimeToolState | null,
): string {
  const callId = event.id || event.call_id;

  return (
    (callId ? toolNamesByCallId.get(callId) : undefined) ||
    event.name ||
    event.tool_name ||
    (callId && activeTool?.callId === callId ? activeTool.name : undefined) ||
    (callId && recentTool?.callId === callId ? recentTool.name : undefined) ||
    activeTool?.name ||
    "unknown"
  );
}

function resetTurnScopedRuntimeState(
  activeTool: RuntimeToolState | null,
  recentTool: RuntimeToolState | null,
  resultPreview: string | null,
): {
  activeTool: RuntimeToolState | null;
  recentTool: RuntimeToolState | null;
} {
  return {
    activeTool: null,
    recentTool:
      normalizeIncompleteTool(activeTool, resultPreview) ??
      normalizeIncompleteTool(recentTool, resultPreview) ??
      recentTool,
  };
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
        ({ activeTool, recentTool } = resetTurnScopedRuntimeState(
          activeTool,
          recentTool,
          "turn restarted before tool completion",
        ));
        recoverableError = null;
        continue;
      }

      if (event.type === "turn_completed") {
        ({ activeTool, recentTool } = resetTurnScopedRuntimeState(
          activeTool,
          recentTool,
          event.summary === "Task cancelled by user"
            ? "cancelled by user"
            : "turn ended before tool completion",
        ));
        continue;
      }

      if (event.type === "task_completed") {
        ({ activeTool, recentTool } = resetTurnScopedRuntimeState(
          activeTool,
          recentTool,
          "task completed before tool completion",
        ));
        recoverableError = null;
        continue;
      }

      if (event.type === "session_rolled_back") {
        activeTool = null;
        recentTool = null;
        recoverableError = null;
        toolNamesByCallId.clear();
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
        const name = resolveCompletionToolName(
          event,
          toolNamesByCallId,
          activeTool,
          recentTool,
        );
        const status = completionFallbackStatus(event);

        recentTool = {
          callId,
          name,
          status,
          resultPreview: event.result_preview,
        };

        if (
          !callId ||
          activeTool?.callId === callId ||
          activeTool?.name === name
        ) {
          activeTool = null;
        }
        continue;
      }

      if (event.type === "error") {
        if (event.recoverable && event.message) {
          recoverableError = {
            message: event.message,
            eventId: event.event_id,
            timestampMs: event.timestamp_ms,
          };
        } else {
          recoverableError = null;
        }
      }
    }
  }

  if (shellRunStatus === "error") {
    ({ activeTool, recentTool } = resetTurnScopedRuntimeState(
      activeTool,
      recentTool,
      recoverableError?.message ?? "runtime error",
    ));
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
    guidance: buildGuidance(shellRunStatus, pendingYield, recoverableError),
  };
}
