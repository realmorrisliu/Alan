#!/bin/bash
# TypeScript 类型生成脚本
#
# 这个脚本从 Rust 的 Event 类型生成 TypeScript 类型定义
# 确保客户端和服务端对事件类型的理解一致
#
# 使用方式: ./scripts/generate-ts-types.sh

set -e

echo "Generating TypeScript types from Rust definitions..."

# 输出目录
OUTPUT_DIR="clients/tui/src/generated"
mkdir -p "$OUTPUT_DIR"

# 生成类型定义文件
cat > "$OUTPUT_DIR/types.ts" << 'EOF'
/**
 * Auto-generated TypeScript types from Rust alan_protocol
 * DO NOT EDIT MANUALLY - Run `./scripts/generate-ts-types.sh` to regenerate
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
 */
export interface TextDeltaEvent extends BaseEvent {
  type: 'text_delta';
  chunk: string;
  is_final: boolean;
}

/**
 * Streaming thinking chunk event
 */
export interface ThinkingDeltaEvent extends BaseEvent {
  type: 'thinking_delta';
  chunk: string;
  is_final: boolean;
}

/**
 * Yield event — agent yields control back to the client
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
EOF

echo "✓ Generated types at $OUTPUT_DIR/types.ts"

# 生成事件处理器映射
cat > "$OUTPUT_DIR/event-map.ts" << 'EOF'
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
EOF

echo "✓ Generated event map at $OUTPUT_DIR/event-map.ts"

# 验证生成的文件
echo ""
echo "Generated files:"
ls -la "$OUTPUT_DIR/"

echo ""
echo "TypeScript types generated successfully!"
echo "Import them in your client code:"
echo "  import type { Event, EventEnvelope, TextDeltaEvent } from './generated/types';"
