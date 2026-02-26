/**
 * Auto-generated TypeScript types from Rust alan_protocol
 * DO NOT EDIT MANUALLY - Run `cargo run --bin generate-types` to regenerate
 */

// Event type discriminant
export type EventType =
  | 'turn_started'
  | 'turn_completed'
  | 'text_delta'
  | 'thinking_delta'
  | 'yield'
  | 'tool_call_started'
  | 'tool_call_completed'
  | 'task_completed'
  | 'context_compacted'
  | 'plan_updated'
  | 'session_rolled_back'
  | 'stream_lagged'
  | 'error'
  | 'skills_loaded'
  | 'dynamic_tools_registered';

export type YieldKind = 'confirmation' | 'structured_input' | 'dynamic_tool_call';

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

/**
 * Streaming text chunk event
 * Replaces MessageDelta and MessageDeltaChunk
 */
export interface TextDeltaEvent extends BaseEvent {
  type: 'text_delta';
  chunk: string;
  is_final: boolean;
}

/**
 * Streaming thinking chunk event
 * Replaces Thinking, ThinkingComplete, and ReasoningDelta
 */
export interface ThinkingDeltaEvent extends BaseEvent {
  type: 'thinking_delta';
  chunk: string;
  is_final: boolean;
}

/**
 * Yield event — agent yields control back to the client
 * Replaces ConfirmationRequired, StructuredUserInputRequested, DynamicToolCallRequested
 */
export interface YieldEvent extends BaseEvent {
  type: 'yield';
  request_id: string;
  kind: YieldKind;
  payload: unknown;
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
  | TextDeltaEvent
  | ThinkingDeltaEvent
  | YieldEvent
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
export const isTextEvent = (event: Event): event is TextDeltaEvent => {
  return event.type === 'text_delta';
};

export const isThinkingEvent = (event: Event): event is ThinkingDeltaEvent => {
  return event.type === 'thinking_delta';
};

export const isToolCallEvent = (event: Event): event is ToolCallStartedEvent | ToolCallCompletedEvent => {
  return event.type === 'tool_call_started' || event.type === 'tool_call_completed';
};

export const isYieldEvent = (event: Event): event is YieldEvent => {
  return event.type === 'yield';
};
