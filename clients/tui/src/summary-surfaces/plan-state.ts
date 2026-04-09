import type { EventEnvelope, PlanItem } from "../types.js";

export interface PlanStatusCounts {
  pending: number;
  in_progress: number;
  completed: number;
}

export interface CurrentPlanState {
  explanation?: string;
  items: PlanItem[];
  statusCounts: PlanStatusCounts;
  lastUpdatedEventId: string;
  lastUpdatedAt: number;
}

function emptyPlanStatusCounts(): PlanStatusCounts {
  return {
    pending: 0,
    in_progress: 0,
    completed: 0,
  };
}

export function countPlanStatuses(items: PlanItem[]): PlanStatusCounts {
  const counts = emptyPlanStatusCounts();

  for (const item of items) {
    counts[item.status] += 1;
  }

  return counts;
}

export function deriveCurrentPlanState(
  events: EventEnvelope[],
  currentSessionId: string | null,
): CurrentPlanState | null {
  if (!currentSessionId) {
    return null;
  }

  let state: CurrentPlanState | null = null;

  for (const event of events) {
    if (event.session_id !== currentSessionId) {
      continue;
    }

    if (
      event.type === "session_rolled_back" ||
      (event.type === "turn_completed" &&
        event.summary === "Task cancelled by user")
    ) {
      state = null;
      continue;
    }

    if (event.type !== "plan_updated" || !event.items?.length) {
      continue;
    }

    state = {
      explanation: event.explanation?.trim() || undefined,
      items: event.items,
      statusCounts: countPlanStatuses(event.items),
      lastUpdatedEventId: event.event_id,
      lastUpdatedAt: event.timestamp_ms,
    };
  }

  return state;
}
