import { describe, expect, test } from "bun:test";
import {
  countPlanStatuses,
  deriveCurrentPlanState,
  hydrateCurrentPlanState,
  reduceCurrentPlanState,
} from "./plan-state";
import type { EventEnvelope, PlanSnapshot } from "../types";

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

  test("clears the plan after an interrupt cancels the active task", () => {
    const state = deriveCurrentPlanState(
      [
        planUpdatedEvent(),
        {
          event_id: "event-cancelled",
          sequence: 2,
          session_id: "sess-1",
          turn_id: "turn-1",
          item_id: "item-2",
          timestamp_ms: 2000,
          type: "turn_completed",
          summary: "Task cancelled by user",
        },
      ],
      "sess-1",
    );

    expect(state).toBeNull();
  });

  test("keeps the plan after a normal turn completion", () => {
    const state = deriveCurrentPlanState(
      [
        planUpdatedEvent(),
        {
          event_id: "event-turn-complete",
          sequence: 2,
          session_id: "sess-1",
          turn_id: "turn-1",
          item_id: "item-2",
          timestamp_ms: 2000,
          type: "turn_completed",
          summary: "Task completed",
        },
      ],
      "sess-1",
    );

    expect(state?.lastUpdatedEventId).toBe("event-plan");
  });

  test("returns null when no current session exists", () => {
    expect(deriveCurrentPlanState([planUpdatedEvent()], null)).toBeNull();
  });

  test("hydrates state from a persisted session snapshot", () => {
    const snapshot: PlanSnapshot = {
      explanation: "Hydrated plan",
      items: [{ id: "p1", content: "Resume session", status: "in_progress" }],
      last_updated_event_id: "evt_42",
      last_updated_at: 4200,
    };

    expect(hydrateCurrentPlanState(snapshot)).toEqual({
      explanation: "Hydrated plan",
      items: [{ id: "p1", content: "Resume session", status: "in_progress" }],
      statusCounts: {
        pending: 0,
        in_progress: 1,
        completed: 0,
      },
      lastUpdatedEventId: "evt_42",
      lastUpdatedAt: 4200,
    });
  });

  test("preserves the current plan across unrelated events", () => {
    const initial = deriveCurrentPlanState([planUpdatedEvent()], "sess-1");
    const state = reduceCurrentPlanState(
      initial,
      {
        event_id: "event-warning",
        sequence: 2,
        session_id: "sess-1",
        turn_id: "turn-1",
        item_id: "item-2",
        timestamp_ms: 2000,
        type: "warning",
        message: "Unrelated warning",
      },
      "sess-1",
    );

    expect(state?.lastUpdatedEventId).toBe("event-plan");
  });
});
