/**
 * Type definitions for Alan TUI
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
  | 'dynamic_tools_registered'
  // Client-side synthesized events
  | 'session_created'
  | 'system_message'
  | 'system_error'
  | 'system_warning'
  | 'user_message';

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

/**
 * EventEnvelope from server
 * 
 * Note: In Rust, EventEnvelope uses #[serde(flatten)] for the event field,
 * so all Event fields are merged at the root level, not nested under an "event" key.
 */
export interface EventEnvelope extends Event {
  event_id: string;
  sequence: number;
  session_id: string;
  submission_id?: string;
  turn_id: string;
  item_id: string;
  timestamp_ms: number;
  // Event fields are flattened here (type, content, message, etc.)
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

// Session types (匹配 agentd 的 API)
export interface SessionListItem {
  session_id: string;
  agent_id: string;
  active: boolean;
  approval_policy: string;
  sandbox_mode: string;
}

export interface SessionListResponse {
  sessions: SessionListItem[];
}

export interface SessionReadResponse {
  session_id: string;
  agent_id: string;
  active: boolean;
  approval_policy: string;
  sandbox_mode: string;
  rollout_path?: string;
  messages: unknown[];
}

export interface CreateSessionRequest {
  workspace_dir?: string;
  approval_policy?: string;
  sandbox_mode?: string;
}

export interface CreateSessionResponse {
  session_id: string;
  websocket_url: string;
  events_url: string;
  submit_url: string;
  approval_policy: string;
  sandbox_mode: string;
}

// Daemon status
export interface DaemonStatus {
  state: 'stopped' | 'starting' | 'running' | 'error';
  pid?: number;
  url: string;
  error?: string;
}

// Event emitter types for client
export interface ClientEvents {
  connected: () => void;
  disconnected: () => void;
  error: (error: Error) => void;
  event: (envelope: EventEnvelope) => void;
  session_created: (sessionId: string) => void;
}
