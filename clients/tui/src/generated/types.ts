/**
 * Auto-generated TypeScript types from Rust alan_protocol
 * DO NOT EDIT MANUALLY - Run `./scripts/generate-ts-types.sh` to regenerate
 */

export type EventType =
  | "turn_started"
  | "turn_completed"
  | "text_delta"
  | "thinking_delta"
  | "yield"
  | "tool_call_started"
  | "tool_call_completed"
  | "task_completed"
  | "context_compacted"
  | "plan_updated"
  | "session_rolled_back"
  | "stream_lagged"
  | "error"
  | "skills_loaded"
  | "dynamic_tools_registered";

export type YieldKind = "confirmation" | "structured_input" | "dynamic_tool";

export interface PlanItem {
  id: string;
  content: string;
  status: "pending" | "in_progress" | "completed";
}

// Flattened event fields from Rust `Event` enum.
export interface Event {
  type: EventType;
  chunk?: string;
  is_final?: boolean;
  request_id?: string;
  kind?: YieldKind;
  payload?: unknown;
  // New tool event fields
  id?: string;
  name?: string;
  result_preview?: string | null;
  // Legacy tool event fields kept for backward compatibility
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
