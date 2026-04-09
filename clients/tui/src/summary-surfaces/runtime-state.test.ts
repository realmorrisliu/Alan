import { describe, expect, test } from "bun:test";
import { deriveCurrentRuntimeSummary, type ShellRunStatus } from "./runtime-state";
import type { PendingYield } from "../adaptive-surfaces/yield-state";
import type { EventEnvelope } from "../types";

function baseEvent(overrides: Partial<EventEnvelope>): EventEnvelope {
  return {
    event_id: "event-1",
    sequence: 1,
    session_id: "sess-1",
    turn_id: "turn-1",
    item_id: "item-1",
    timestamp_ms: 1000,
    type: "turn_started",
    ...overrides,
  };
}

function deriveSummary(
  events: EventEnvelope[],
  shellRunStatus: ShellRunStatus,
  pendingYield: PendingYield | null = null,
) {
  return deriveCurrentRuntimeSummary({
    events,
    currentSessionId: "sess-1",
    shellRunStatus,
    pendingYield,
  });
}

describe("runtime state helpers", () => {
  test("tracks the currently running tool", () => {
    const summary = deriveSummary(
      [
        baseEvent({ type: "tool_call_started", id: "call-1", name: "bash" }),
      ],
      "running",
    );

    expect(summary.headline).toBe("Running bash");
    expect(summary.activeTool).toEqual({
      callId: "call-1",
      name: "bash",
      status: "running",
    });
  });

  test("keeps the most recent tool after completion", () => {
    const summary = deriveSummary(
      [
        baseEvent({ type: "tool_call_started", id: "call-1", name: "bash" }),
        baseEvent({
          event_id: "event-2",
          sequence: 2,
          type: "tool_call_completed",
          id: "call-1",
          result_preview: "ok",
        }),
      ],
      "ready",
    );

    expect(summary.activeTool).toBeNull();
    expect(summary.recentTool).toEqual({
      callId: "call-1",
      name: "bash",
      status: "completed",
      resultPreview: "ok",
    });
  });

  test("surfaces recoverable errors with guidance", () => {
    const summary = deriveSummary(
      [
        baseEvent({
          event_id: "event-error",
          type: "error",
          message: "Need different input",
          recoverable: true,
        }),
      ],
      "error",
    );

    expect(summary.headline).toBe("Recoverable issue");
    expect(summary.recoverableError?.message).toBe("Need different input");
    expect(summary.guidance).toContain("correct the input");
  });

  test("prefers pending-yield guidance over generic runtime state", () => {
    const summary = deriveSummary([], "yielded", {
      requestId: "req-1",
      kind: "confirmation",
      payload: {},
    });

    expect(summary.headline).toBe("Waiting on confirmation");
    expect(summary.guidance).toContain("/approve");
  });

  test("ignores events from other sessions", () => {
    const summary = deriveCurrentRuntimeSummary({
      events: [
        baseEvent({
          session_id: "sess-2",
          type: "tool_call_started",
          id: "call-1",
          name: "bash",
        }),
      ],
      currentSessionId: "sess-1",
      shellRunStatus: "ready",
      pendingYield: null,
    });

    expect(summary.activeTool).toBeNull();
    expect(summary.recentTool).toBeNull();
  });

  test("clears stale recoverable errors when a new turn starts", () => {
    const summary = deriveSummary(
      [
        baseEvent({
          event_id: "event-error",
          type: "error",
          message: "Need different input",
          recoverable: true,
        }),
        baseEvent({
          event_id: "event-turn-2",
          sequence: 2,
          turn_id: "turn-2",
          type: "turn_started",
        }),
      ],
      "running",
    );

    expect(summary.recoverableError).toBeNull();
  });
});
