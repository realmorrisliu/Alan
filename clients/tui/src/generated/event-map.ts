/**
 * Auto-generated event handler type map
 * Use this to ensure all event types are handled
 */

import type { EventEnvelope } from "./types";

export interface EventHandlerMap {
  turn_started: (event: EventEnvelope) => void;
  turn_completed: (event: EventEnvelope) => void;
  text_delta: (event: EventEnvelope) => void;
  thinking_delta: (event: EventEnvelope) => void;
  tool_call_started: (event: EventEnvelope) => void;
  tool_call_completed: (event: EventEnvelope) => void;
  plan_updated: (event: EventEnvelope) => void;
  session_rolled_back: (event: EventEnvelope) => void;
  yield: (event: EventEnvelope) => void;
  error: (event: EventEnvelope) => void;
  warning: (event: EventEnvelope) => void;
}

export const USER_VISIBLE_EVENT_TYPES = [
  "text_delta",
  "thinking_delta",
  "yield",
  "tool_call_started",
  "tool_call_completed",
  "plan_updated",
  "session_rolled_back",
  "warning",
  "error",
] as const;

export const MESSAGE_EVENT_TYPES = ["text_delta"] as const;
