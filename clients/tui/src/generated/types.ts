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
  | "yield"
  | "warning"
  | "error";

export type YieldKind =
  | "confirmation"
  | "structured_input"
  | "dynamic_tool"
  | (string & {})
  | { custom: string };

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
