#!/bin/bash
# TypeScript 类型生成脚本
#
# 从 Rust alan_protocol 生成前端消费的类型定义。

set -e

echo "Generating TypeScript types from Rust definitions..."

OUTPUT_DIR="clients/tui/src/generated"
mkdir -p "$OUTPUT_DIR"

cat > "$OUTPUT_DIR/types.ts" << 'TYPES_EOF'
/**
 * Auto-generated TypeScript types from Rust alan_protocol
 * DO NOT EDIT MANUALLY - Run `./scripts/generate-ts-types.sh` to regenerate
 */

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

export type YieldKind = 'confirmation' | 'structured_input' | 'dynamic_tool';

export interface PlanItem {
  id: string;
  content: string;
  status: 'pending' | 'in_progress' | 'completed';
}

// Flattened event fields from Rust `Event` enum.
export interface Event {
  type: EventType;
  chunk?: string;
  is_final?: boolean;
  request_id?: string;
  kind?: YieldKind;
  payload?: unknown;
  call_id?: string;
  tool_name?: string;
  arguments?: unknown;
  result?: unknown;
  success?: boolean;
  summary?: string;
  results?: unknown;
  explanation?: string;
  items?: PlanItem[];
  turns?: number;
  removed_messages?: number;
  skipped?: number;
  replay_from_event_id?: string | null;
  message?: string;
  recoverable?: boolean;
  skill_ids?: string[];
  auto_selected?: boolean;
  tool_names?: string[];
}

// Rust EventEnvelope uses #[serde(flatten)] for event payload.
export interface EventEnvelope extends Event {
  event_id: string;
  sequence: number;
  session_id: string;
  submission_id?: string;
  turn_id: string;
  item_id: string;
  timestamp_ms: number;
}
TYPES_EOF

echo "✓ Generated types at $OUTPUT_DIR/types.ts"

cat > "$OUTPUT_DIR/event-map.ts" << 'MAP_EOF'
/**
 * Auto-generated event handler type map
 * Use this to ensure all event types are handled
 */

import type { EventEnvelope } from './types';

export interface EventHandlerMap {
  turn_started: (event: EventEnvelope) => void;
  turn_completed: (event: EventEnvelope) => void;
  text_delta: (event: EventEnvelope) => void;
  thinking_delta: (event: EventEnvelope) => void;
  yield: (event: EventEnvelope) => void;
  tool_call_started: (event: EventEnvelope) => void;
  tool_call_completed: (event: EventEnvelope) => void;
  task_completed: (event: EventEnvelope) => void;
  context_compacted: (event: EventEnvelope) => void;
  plan_updated: (event: EventEnvelope) => void;
  session_rolled_back: (event: EventEnvelope) => void;
  stream_lagged: (event: EventEnvelope) => void;
  error: (event: EventEnvelope) => void;
  skills_loaded: (event: EventEnvelope) => void;
  dynamic_tools_registered: (event: EventEnvelope) => void;
}

export const USER_VISIBLE_EVENT_TYPES = [
  'text_delta',
  'thinking_delta',
  'yield',
  'tool_call_started',
  'tool_call_completed',
  'task_completed',
  'error',
] as const;

export const MESSAGE_EVENT_TYPES = ['text_delta'] as const;
MAP_EOF

echo "✓ Generated event map at $OUTPUT_DIR/event-map.ts"

echo ""
echo "Generated files:"
ls -la "$OUTPUT_DIR/"

echo ""
echo "TypeScript types generated successfully!"
