/**
 * Type definitions for Alan TUI
 * Mirrors the protocol crate's types
 */

// Event types from protocol
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

export interface Event {
  type: EventType;
  // Common fields
  message?: string;
  content?: string;
  chunk?: string;
  is_final?: boolean;
  // Tool call fields
  call_id?: string;
  tool_name?: string;
  arguments?: Record<string, unknown>;
  result?: unknown;
  success?: boolean;
  // Confirmation fields
  checkpoint_id?: string;
  checkpoint_type?: string;
  summary?: string;
  details?: unknown;
  options?: string[];
  // Error fields
  recoverable?: boolean;
  // Task fields
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
  | 'start_task'
  | 'confirm'
  | 'user_input'
  | 'structured_user_input'
  | 'register_dynamic_tools'
  | 'dynamic_tool_result'
  | 'compact'
  | 'rollback'
  | 'cancel';

export interface Op {
  type: OpType;
  // StartTask fields
  agent_id?: string;
  domain?: string;
  input?: string;
  attachments?: string[];
  // Confirm fields
  checkpoint_id?: string;
  choice?: 'approve' | 'modify' | 'reject';
  modifications?: string;
  // UserInput fields
  content?: string;
  // StructuredUserInput
  request_id?: string;
  answers?: StructuredInputAnswer[];
  // Dynamic tools
  tools?: DynamicToolSpec[];
  call_id?: string;
  result?: unknown;
  // Rollback
  num_turns?: number;
}

export interface StructuredInputAnswer {
  question_id: string;
  value: string;
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

// Event emitter types for client
export interface ClientEvents {
  connected: () => void;
  disconnected: () => void;
  error: (error: Error) => void;
  event: (envelope: EventEnvelope) => void;
  session_created: (sessionId: string) => void;
}
