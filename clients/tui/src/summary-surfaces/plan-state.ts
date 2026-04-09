import type { EventEnvelope, PlanItem, PlanSnapshot } from "../types.js";

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

function shouldClearCurrentPlan(event: EventEnvelope): boolean {
  return (
    event.type === "session_rolled_back" ||
    (event.type === "turn_completed" &&
      event.summary === "Task cancelled by user")
  );
}

function buildCurrentPlanState(snapshot: PlanSnapshot): CurrentPlanState | null {
  if (!snapshot.items.length) {
    return null;
  }

  return {
    explanation: snapshot.explanation?.trim() || undefined,
    items: snapshot.items,
    statusCounts: countPlanStatuses(snapshot.items),
    lastUpdatedEventId: snapshot.last_updated_event_id,
    lastUpdatedAt: snapshot.last_updated_at,
  };
}

export function hydrateCurrentPlanState(
  snapshot: PlanSnapshot | null | undefined,
): CurrentPlanState | null {
  return snapshot ? buildCurrentPlanState(snapshot) : null;
}

export function reduceCurrentPlanState(
  state: CurrentPlanState | null,
  event: EventEnvelope,
  currentSessionId: string | null,
): CurrentPlanState | null {
  if (!currentSessionId || event.session_id !== currentSessionId) {
    return state;
  }

  if (shouldClearCurrentPlan(event)) {
    return null;
  }

  if (event.type !== "plan_updated" || !event.items?.length) {
    return state;
  }

  return buildCurrentPlanState({
    explanation: event.explanation,
    items: event.items,
    last_updated_event_id: event.event_id,
    last_updated_at: event.timestamp_ms,
  });
}

export function deriveCurrentPlanState(
  events: EventEnvelope[],
  currentSessionId: string | null,
): CurrentPlanState | null {
  let state: CurrentPlanState | null = null;

  for (const event of events) {
    state = reduceCurrentPlanState(state, event, currentSessionId);
  }

  return state;
}
