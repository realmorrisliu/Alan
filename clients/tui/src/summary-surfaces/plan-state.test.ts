import { describe, expect, test } from "bun:test";
import { countPlanStatuses, deriveCurrentPlanState } from "./plan-state";
import type { EventEnvelope } from "../types";

function planUpdatedEvent(
  overrides: Partial<EventEnvelope> = {},
): EventEnvelope {
  return {
    event_id: "event-plan",
    sequence: 1,
    session_id: "sess-1",
    turn_id: "turn-1",
    item_id: "item-1",
    timestamp_ms: 1000,
    type: "plan_updated",
    explanation: "Current plan",
    items: [
      { id: "p1", content: "Inspect current state", status: "completed" },
      { id: "p2", content: "Render plan panel", status: "in_progress" },
      { id: "p3", content: "Add tests", status: "pending" },
    ],
    ...overrides,
  };
}

describe("plan state helpers", () => {
  test("counts plan statuses", () => {
    expect(
      countPlanStatuses([
        { id: "p1", content: "One", status: "pending" },
        { id: "p2", content: "Two", status: "in_progress" },
        { id: "p3", content: "Three", status: "completed" },
        { id: "p4", content: "Four", status: "completed" },
      ]),
    ).toEqual({
      pending: 1,
      in_progress: 1,
      completed: 2,
    });
  });

  test("derives the latest plan state for the active session", () => {
    const state = deriveCurrentPlanState(
      [
        planUpdatedEvent({
          event_id: "event-plan-1",
          explanation: "Old plan",
          items: [{ id: "p1", content: "Old", status: "pending" }],
        }),
        planUpdatedEvent({
          event_id: "event-plan-2",
          explanation: "Latest plan",
          items: [
            { id: "p2", content: "Now", status: "in_progress" },
            { id: "p3", content: "Later", status: "pending" },
          ],
          timestamp_ms: 2000,
        }),
      ],
      "sess-1",
    );

    expect(state).toEqual({
      explanation: "Latest plan",
      items: [
        { id: "p2", content: "Now", status: "in_progress" },
        { id: "p3", content: "Later", status: "pending" },
      ],
      statusCounts: {
        pending: 1,
        in_progress: 1,
        completed: 0,
      },
      lastUpdatedEventId: "event-plan-2",
      lastUpdatedAt: 2000,
    });
  });

  test("ignores plan events from other sessions", () => {
    const state = deriveCurrentPlanState(
      [
        planUpdatedEvent({ session_id: "sess-2" }),
        planUpdatedEvent({ session_id: "sess-1", event_id: "event-plan-2" }),
      ],
      "sess-1",
    );

    expect(state?.lastUpdatedEventId).toBe("event-plan-2");
  });

  test("clears the plan after rollback for the active session", () => {
    const state = deriveCurrentPlanState(
      [
        planUpdatedEvent(),
        {
          event_id: "event-rollback",
          sequence: 2,
          session_id: "sess-1",
          turn_id: "turn-1",
          item_id: "item-2",
          timestamp_ms: 2000,
          type: "session_rolled_back",
          turns: 1,
        },
      ],
      "sess-1",
    );

    expect(state).toBeNull();
  });

  test("returns null when no current session exists", () => {
    expect(deriveCurrentPlanState([planUpdatedEvent()], null)).toBeNull();
  });
});
