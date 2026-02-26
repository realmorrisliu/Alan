/**
 * Type definitions for Alan Electron
 * Mirrors the protocol crate's types
 */

// Event types from protocol
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

export interface Event {
  type: EventType;
  // TextDelta / ThinkingDelta fields
  chunk?: string;
  is_final?: boolean;
  // Yield fields
  request_id?: string;
  kind?: YieldKind;
  payload?: unknown;
  // Tool call fields
  call_id?: string;
  tool_name?: string;
  arguments?: Record<string, unknown>;
  result?: unknown;
  success?: boolean;
  // Error fields
  message?: string;
  recoverable?: boolean;
  // Task fields
  summary?: string;
  results?: unknown;
  // Plan fields
  explanation?: string;
  items?: PlanItem[];
  // Session rollback
  num_turns?: number;
  removed_messages?: number;
  // Skills
  skill_ids?: string[];
  auto_selected?: boolean;
  // Dynamic tools
  tool_names?: string[];
  // Context compacted
  content?: string;
}

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

export interface PlanItem {
  id: string;
  content: string;
  status: 'pending' | 'in_progress' | 'completed';
}

// Operation types
export type OpType =
  | 'turn'
  | 'input'
  | 'resume'
  | 'interrupt'
  | 'register_dynamic_tools'
  | 'compact'
  | 'rollback';

export interface Op {
  type: OpType;
  // Turn fields
  input?: string;
  context?: Record<string, unknown>;
  // Input fields
  content?: string;
  // Resume fields
  request_id?: string;
  result?: unknown;
  // Dynamic tools
  tools?: DynamicToolSpec[];
  // Rollback
  num_turns?: number;
}

export interface DynamicToolSpec {
  name: string;
  description: string;
  parameters: unknown;
  capability?: 'read' | 'write' | 'network';
}

export interface Submission {
  id: string;
  op: Op;
}

// Session types
export interface Session {
  id: string;
  agent_id: string;
  status: 'active' | 'paused' | 'completed' | 'error';
  created_at: string;
  updated_at: string;
}

export interface CreateSessionRequest {
  approval_policy?: 'on_request' | 'never';
  sandbox_mode?: 'read_only' | 'workspace_write' | 'danger_full_access';
}

export interface CreateSessionResponse {
  id: string;
  agent_id: string;
  status: string;
}
