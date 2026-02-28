/**
 * Type definitions for Alan TUI
 * Keep these aligned with `alan_protocol` and daemon routes.
 */

// Event types from protocol + client-side synthesized events.
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
  | 'session_created'
  | 'system_message'
  | 'system_error'
  | 'system_warning'
  | 'user_message';

export type YieldKind = 'confirmation' | 'structured_input' | 'dynamic_tool_call';

export interface PlanItem {
  id: string;
  content: string;
  status: 'pending' | 'in_progress' | 'completed';
}

export interface Event {
  type: EventType;
  chunk?: string;
  is_final?: boolean;
  request_id?: string;
  kind?: YieldKind;
  payload?: unknown;
  call_id?: string;
  tool_name?: string;
  arguments?: Record<string, unknown>;
  result?: unknown;
  success?: boolean;
  message?: string;
  recoverable?: boolean;
  summary?: string;
  results?: unknown;
  explanation?: string;
  items?: PlanItem[];
  num_turns?: number;
  removed_messages?: number;
  skill_ids?: string[];
  auto_selected?: boolean;
  tool_names?: string[];
  skipped?: number;
  replay_from_event_id?: string | null;
}

/**
 * Server envelope shape is flattened (`#[serde(flatten)]` in Rust).
 */
export interface EventEnvelope extends Event {
  event_id: string;
  sequence: number;
  session_id: string;
  submission_id?: string;
  turn_id: string;
  item_id: string;
  timestamp_ms: number;
}

export interface ContentTextPart {
  type: 'text';
  text: string;
}

export interface ContentStructuredPart {
  type: 'structured';
  data: unknown;
}

export type ContentPart = ContentTextPart | ContentStructuredPart;

export interface TurnContext {
  workspace_id?: string;
  domain?: string;
}

export interface DynamicToolSpec {
  name: string;
  description: string;
  parameters: unknown;
  capability?: 'read' | 'write' | 'network';
}

export type Op =
  | { type: 'turn'; parts: ContentPart[]; context?: TurnContext }
  | { type: 'input'; parts: ContentPart[] }
  | { type: 'resume'; request_id: string; content: ContentPart[] }
  | { type: 'interrupt' }
  | { type: 'register_dynamic_tools'; tools: DynamicToolSpec[] }
  | { type: 'compact' }
  | { type: 'rollback'; num_turns: number };

// Session types (match agentd API)
export interface SessionListItem {
  session_id: string;
  workspace_id: string;
  active: boolean;
  approval_policy: string;
  sandbox_mode: string;
}

export interface SessionListResponse {
  sessions: SessionListItem[];
}

export interface SessionReadResponse {
  session_id: string;
  workspace_id: string;
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

export interface DaemonStatus {
  state: 'stopped' | 'starting' | 'running' | 'error';
  pid?: number;
  url: string;
  error?: string;
}

export interface ClientEvents {
  connected: () => void;
  disconnected: () => void;
  error: (error: Error) => void;
  event: (envelope: EventEnvelope) => void;
  session_created: (sessionId: string) => void;
}
