#!/bin/bash
# TypeScript type generation script
#
# Generates frontend-consumable type definitions from Rust `alan_protocol`.

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
  | "turn_started"
  | "turn_completed"
  | "text_delta"
  | "thinking_delta"
  | "tool_call_started"
  | "tool_call_completed"
  | "plan_updated"
  | "session_rolled_back"
  | "yield"
  | "warning"
  | "error";

export type YieldKind =
  | "confirmation"
  | "structured_input"
  | "dynamic_tool"
  | (string & {})
  | { custom: string };

export type AdaptivePresentationHint =
  | "radio"
  | "toggle"
  | "searchable"
  | "multiline"
  | "compact"
  | "dangerous";

export type StructuredInputKind =
  | "text"
  | "boolean"
  | "number"
  | "integer"
  | "single_select"
  | "multi_select";

export interface StructuredInputOption {
  value: string;
  label: string;
  description?: string;
}

export interface StructuredInputQuestion {
  id: string;
  label: string;
  prompt: string;
  kind?: StructuredInputKind;
  required?: boolean;
  placeholder?: string;
  help_text?: string;
  default?: string;
  defaults?: string[];
  min_selected?: number;
  max_selected?: number;
  options?: StructuredInputOption[];
  presentation_hints?: AdaptivePresentationHint[];
}

export interface AdaptiveForm {
  fields?: StructuredInputQuestion[];
}

export interface ConfirmationYieldPayload {
  checkpoint_type: string;
  summary: string;
  details?: unknown;
  options?: string[];
  default_option?: string;
  presentation_hints?: AdaptivePresentationHint[];
}

export interface StructuredInputYieldPayload {
  title: string;
  prompt?: string;
  questions?: StructuredInputQuestion[];
}

export interface DynamicToolYieldPayload {
  tool_name: string;
  arguments: unknown;
  title: string;
  prompt?: string;
  form?: AdaptiveForm;
}

export interface CustomYieldPayload {
  title?: string;
  prompt?: string;
  details?: unknown;
  form?: AdaptiveForm;
}

export interface AdaptiveYieldCapabilities {
  rich_confirmation?: boolean;
  structured_input?: boolean;
  schema_driven_forms?: boolean;
  presentation_hints?: boolean;
}

export interface ClientCapabilities {
  adaptive_yields?: AdaptiveYieldCapabilities;
}

export interface ToolDecisionAudit {
  policy_source: string;
  rule_id?: string;
  action: "allow" | "deny" | "escalate" | string;
  reason?: string;
  capability: "read" | "write" | "network" | "unknown" | string;
  sandbox_backend: string;
}

// Flattened event fields from Rust `Event` enum.
export interface Event {
  type: EventType;
  summary?: string;
  chunk?: string;
  is_final?: boolean;
  id?: string;
  name?: string;
  success?: boolean;
  result_preview?: string | null;
  audit?: ToolDecisionAudit;
  request_id?: string;
  kind?: YieldKind;
  payload?: unknown;
  message?: string;
  recoverable?: boolean;
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
MAP_EOF

echo "✓ Generated event map at $OUTPUT_DIR/event-map.ts"

echo ""
echo "Generated files:"
ls -la "$OUTPUT_DIR/"

echo ""
echo "TypeScript types generated successfully!"
