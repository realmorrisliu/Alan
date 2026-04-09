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

export interface CurrentRuntimeState {
  activeTool: RuntimeToolState | null;
  recentTool: RuntimeToolState | null;
  recoverableError: RecoverableErrorState | null;
  toolNamesByCallId: Record<string, string>;
}

export interface CurrentRuntimeSummary {
  headline: string;
  shellRunStatus: ShellRunStatus;
  activeTool: RuntimeToolState | null;
  recentTool: RuntimeToolState | null;
  recoverableError: RecoverableErrorState | null;
  guidance: string;
}

export interface BuildRuntimeSummaryInput {
  state: CurrentRuntimeState;
  shellRunStatus: ShellRunStatus;
  pendingYield: PendingYield | null;
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
  if (recoverableError) {
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
  if (recoverableError) {
    return "Next: inspect the warning, correct the input if needed, then retry or continue.";
  }
  if (shellRunStatus === "error") {
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
  toolNamesByCallId: Record<string, string>,
  activeTool: RuntimeToolState | null,
  recentTool: RuntimeToolState | null,
): string {
  const callId = event.id || event.call_id;

  return (
    (callId ? toolNamesByCallId[callId] : undefined) ||
    event.name ||
    event.tool_name ||
    (callId && activeTool?.callId === callId ? activeTool.name : undefined) ||
    (callId && recentTool?.callId === callId ? recentTool.name : undefined) ||
    activeTool?.name ||
    "unknown"
  );
}

export function createCurrentRuntimeState(): CurrentRuntimeState {
  return {
    activeTool: null,
    recentTool: null,
    recoverableError: null,
    toolNamesByCallId: {},
  };
}

function resetTurnScopedRuntimeState(
  state: CurrentRuntimeState,
  resultPreview: string | null,
  clearRecoverableError = false,
): CurrentRuntimeState {
  return {
    ...state,
    activeTool: null,
    recentTool:
      normalizeIncompleteTool(state.activeTool, resultPreview) ??
      normalizeIncompleteTool(state.recentTool, resultPreview) ??
      state.recentTool,
    recoverableError: clearRecoverableError ? null : state.recoverableError,
    toolNamesByCallId: {},
  };
}

export function reduceCurrentRuntimeState(
  state: CurrentRuntimeState,
  event: EventEnvelope,
  currentSessionId: string | null,
): CurrentRuntimeState {
  if (!currentSessionId || event.session_id !== currentSessionId) {
    return state;
  }

  if (event.type === "turn_started") {
    return {
      ...resetTurnScopedRuntimeState(
        state,
        "turn restarted before tool completion",
      ),
      recoverableError: null,
    };
  }

  if (event.type === "turn_completed") {
    return resetTurnScopedRuntimeState(
      state,
      event.summary === "Task cancelled by user"
        ? "cancelled by user"
        : "turn ended before tool completion",
    );
  }

  if (event.type === "task_completed") {
    return resetTurnScopedRuntimeState(
      state,
      "task completed before tool completion",
      true,
    );
  }

  if (event.type === "session_rolled_back") {
    return createCurrentRuntimeState();
  }

  if (event.type === "tool_call_started") {
    const callId = event.id || event.call_id;
    const name = event.name || event.tool_name || "unknown";
    const activeTool: RuntimeToolState = {
      callId,
      name,
      status: "running",
    };

    return {
      ...state,
      activeTool,
      recentTool: activeTool,
      toolNamesByCallId: callId
        ? {
            ...state.toolNamesByCallId,
            [callId]: name,
          }
        : state.toolNamesByCallId,
    };
  }

  if (event.type === "tool_call_completed") {
    const callId = event.id || event.call_id;
    const name = resolveCompletionToolName(
      event,
      state.toolNamesByCallId,
      state.activeTool,
      state.recentTool,
    );
    const status = completionFallbackStatus(event);
    const toolNamesByCallId = { ...state.toolNamesByCallId };

    if (callId) {
      delete toolNamesByCallId[callId];
    }

    return {
      ...state,
      activeTool:
        !callId ||
        state.activeTool?.callId === callId ||
        state.activeTool?.name === name
          ? null
          : state.activeTool,
      recentTool: {
        callId,
        name,
        status,
        resultPreview: event.result_preview,
      },
      toolNamesByCallId,
    };
  }

  if (event.type === "error") {
    return {
      ...state,
      recoverableError:
        event.recoverable && event.message
          ? {
              message: event.message,
              eventId: event.event_id,
              timestampMs: event.timestamp_ms,
            }
          : null,
    };
  }

  return state;
}

export function deriveCurrentRuntimeState(
  events: EventEnvelope[],
  currentSessionId: string | null,
): CurrentRuntimeState {
  let state = createCurrentRuntimeState();

  for (const event of events) {
    state = reduceCurrentRuntimeState(state, event, currentSessionId);
  }

  return state;
}

export function buildCurrentRuntimeSummary({
  state,
  shellRunStatus,
  pendingYield,
}: BuildRuntimeSummaryInput): CurrentRuntimeSummary {
  const summaryState =
    shellRunStatus === "error"
      ? resetTurnScopedRuntimeState(
          state,
          state.recoverableError?.message ?? "runtime error",
        )
      : state;

  return {
    headline: buildHeadline(
      shellRunStatus,
      pendingYield,
      summaryState.activeTool,
      summaryState.recoverableError,
    ),
    shellRunStatus,
    activeTool: summaryState.activeTool,
    recentTool: summaryState.recentTool,
    recoverableError: summaryState.recoverableError,
    guidance: buildGuidance(
      shellRunStatus,
      pendingYield,
      summaryState.recoverableError,
    ),
  };
}

export function deriveCurrentRuntimeSummary({
  events,
  currentSessionId,
  shellRunStatus,
  pendingYield,
}: DeriveRuntimeSummaryInput): CurrentRuntimeSummary {
  return buildCurrentRuntimeSummary({
    state: deriveCurrentRuntimeState(events, currentSessionId),
    shellRunStatus,
    pendingYield,
  });
}
