/**
 * Auto-generated TypeScript types from Rust alan_protocol
 * DO NOT EDIT MANUALLY - Run `cargo run --bin generate-types` to regenerate
 */

// Event type discriminant
export type EventType =
  | 'turn_started'
  | 'turn_completed'
  | 'thinking'
  | 'thinking_complete'
  | 'reasoning_delta'
  | 'message_delta'
  | 'message_delta_chunk'
  | 'confirmation_required'
  | 'structured_user_input_requested'
  | 'tool_call_started'
  | 'tool_call_completed'
  | 'task_completed'
  | 'context_compacted'
  | 'plan_updated'
  | 'session_rolled_back'
  | 'stream_lagged'
  | 'error'
  | 'skills_loaded'
  | 'dynamic_tools_registered'
  | 'dynamic_tool_call_requested';

// Base event interface
export interface BaseEvent {
  type: EventType;
}

// Individual event types
export interface TurnStartedEvent extends BaseEvent {
  type: 'turn_started';
}

export interface TurnCompletedEvent extends BaseEvent {
  type: 'turn_completed';
}

export interface ThinkingEvent extends BaseEvent {
  type: 'thinking';
  message: string;
}

export interface ThinkingCompleteEvent extends BaseEvent {
  type: 'thinking_complete';
}

export interface ReasoningDeltaEvent extends BaseEvent {
  type: 'reasoning_delta';
  chunk: string;
  is_final: boolean;
}

/**
 * Complete message content event
 * This is the primary event for displaying assistant messages
 */
export interface MessageDeltaEvent extends BaseEvent {
  type: 'message_delta';
  content: string;
}

/**
 * Streaming message chunk event
 * Used for typing effect; may be followed by MessageDelta
 */
export interface MessageDeltaChunkEvent extends BaseEvent {
  type: 'message_delta_chunk';
  chunk: string;
  is_final: boolean;
}

export interface ToolCallStartedEvent extends BaseEvent {
  type: 'tool_call_started';
  call_id: string;
  tool_name: string;
  arguments: unknown;
}

export interface ToolCallCompletedEvent extends BaseEvent {
  type: 'tool_call_completed';
  call_id: string;
  tool_name: string;
  result: unknown;
  success: boolean;
}

export interface TaskCompletedEvent extends BaseEvent {
  type: 'task_completed';
  summary: string;
  results: unknown;
}

export interface ErrorEvent extends BaseEvent {
  type: 'error';
  message: string;
  recoverable: boolean;
}

// Union type of all events
export type Event =
  | TurnStartedEvent
  | TurnCompletedEvent
  | ThinkingEvent
  | ThinkingCompleteEvent
  | ReasoningDeltaEvent
  | MessageDeltaEvent
  | MessageDeltaChunkEvent
  | ToolCallStartedEvent
  | ToolCallCompletedEvent
  | TaskCompletedEvent
  | ErrorEvent;

// Event envelope
export interface EventEnvelope {
  event_id: string;
  sequence: number;
  session_id: string;
  submission_id?: string;
  turn_id: string;
  item_id: string;
  timestamp_ms: number;
  event: Event;
}

// Event type guards
export const isMessageEvent = (event: Event): event is MessageDeltaEvent | MessageDeltaChunkEvent => {
  return event.type === 'message_delta' || event.type === 'message_delta_chunk';
};

export const isToolCallEvent = (event: Event): event is ToolCallStartedEvent | ToolCallCompletedEvent => {
  return event.type === 'tool_call_started' || event.type === 'tool_call_completed';
};
