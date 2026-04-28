import { describe, expect, test } from "bun:test";
import { buildPlanHudSummary, buildRuntimeHudSummary } from "./hud-surface";
import type { CurrentPlanState } from "./plan-state";
import type { CurrentRuntimeSummary } from "./runtime-state";

function plan(overrides: Partial<CurrentPlanState> = {}): CurrentPlanState {
  return {
    items: [
      {
        id: "inspect",
        content: "Inspect current TUI layout",
        status: "completed",
      },
      {
        id: "hud",
        content: "Move status into the input HUD",
        status: "in_progress",
      },
      { id: "verify", content: "Run focused tests", status: "pending" },
    ],
    statusCounts: {
      completed: 1,
      in_progress: 1,
      pending: 1,
    },
    lastUpdatedEventId: "evt_1",
    lastUpdatedAt: 1000,
    ...overrides,
  };
}

function runtime(
  overrides: Partial<CurrentRuntimeSummary> = {},
): CurrentRuntimeSummary {
  return {
    headline: "Running bash",
    shellRunStatus: "running",
    activeTool: {
      callId: "call-1",
      name: "bash",
      status: "running",
    },
    recentTool: null,
    recoverableError: null,
    guidance: "Next: send a request or use /help.",
    ...overrides,
  };
}

describe("summary HUD helpers", () => {
  test("summarizes the active plan in one compact line", () => {
    expect(buildPlanHudSummary(plan())).toBe(
      "Plan: 1 active | 1 pending | 1 completed | > [~] Move status into the input HUD",
    );
  });

  test("uses the next pending plan item when nothing is active", () => {
    expect(
      buildPlanHudSummary(
        plan({
          items: [
            { id: "verify", content: "Run focused tests", status: "pending" },
          ],
          statusCounts: {
            completed: 0,
            in_progress: 0,
            pending: 1,
          },
        }),
      ),
    ).toBe("Plan: 1 pending | [ ] Run focused tests");
  });

  test("summarizes runtime state in one compact line", () => {
    expect(buildRuntimeHudSummary(runtime())).toBe(
      "Runtime: Running bash | state=running | tool=bash running | send a request or use /help.",
    );
  });

  test("includes active child-run count when available", () => {
    expect(buildRuntimeHudSummary(runtime(), 2)).toBe(
      "Runtime: Running bash | state=running | children=2 active | tool=bash running | send a request or use /help.",
    );
  });
});
