/**
 * Auto-generated event handler type map
 * Use this to ensure all event types are handled
 */

import type {
  TurnStartedEvent,
  TurnCompletedEvent,
  TextDeltaEvent,
  ThinkingDeltaEvent,
  YieldEvent,
  ToolCallStartedEvent,
  ToolCallCompletedEvent,
  TaskCompletedEvent,
  ErrorEvent,
} from './types';

/**
 * Event handler interface - ensures all event types are handled
 * Implement this interface in your event handler to ensure completeness
 */
export interface EventHandlerMap {
  turn_started: (event: TurnStartedEvent) => void;
  turn_completed: (event: TurnCompletedEvent) => void;
  text_delta: (event: TextDeltaEvent) => void;
  thinking_delta: (event: ThinkingDeltaEvent) => void;
  yield: (event: YieldEvent) => void;
  tool_call_started: (event: ToolCallStartedEvent) => void;
  tool_call_completed: (event: ToolCallCompletedEvent) => void;
  task_completed: (event: TaskCompletedEvent) => void;
  error: (event: ErrorEvent) => void;
}

/**
 * List of all event types that should be displayed to the user
 * Use this to validate your UI handles all visible events
 */
export const USER_VISIBLE_EVENT_TYPES = [
  'text_delta',
  'thinking_delta',
  'yield',
  'tool_call_started',
  'tool_call_completed',
  'task_completed',
  'error',
] as const;

/**
 * List of all message-related event types
 * Ensure your UI subscribes to all of these
 */
export const MESSAGE_EVENT_TYPES = [
  'text_delta',
] as const;
