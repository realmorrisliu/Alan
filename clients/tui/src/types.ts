/**
 * Type definitions for Alan TUI.
 *
 * Protocol truth source: crates/protocol/src/event.rs and crates/protocol/src/op.rs
 * This file keeps protocol events explicit and isolates local synthetic events.
 */

import type { ClientCapabilities, YieldKind } from "./generated/types";

export type {
  AdaptiveForm,
  AdaptivePresentationHint,
  AdaptiveYieldCapabilities,
  ClientCapabilities,
  ConfirmationYieldPayload,
  CustomYieldPayload,
  DynamicToolYieldPayload,
  StructuredInputKind as ProtocolStructuredInputKind,
  StructuredInputOption as ProtocolStructuredInputOption,
  StructuredInputQuestion as ProtocolStructuredInputQuestion,
  StructuredInputYieldPayload,
  YieldKind,
} from "./generated/types";

export type ProtocolEventType =
  | "turn_started"
  | "turn_completed"
  | "text_delta"
  | "thinking_delta"
  | "warning"
  | "yield"
  | "tool_call_started"
  | "tool_call_completed"
  | "error";

// Legacy/compat runtime events that may appear in historical logs.
export type LegacyCompatEventType =
  | "task_completed"
  | "context_compacted"
  | "plan_updated"
  | "session_rolled_back"
  | "stream_lagged"
  | "skills_loaded"
  | "dynamic_tools_registered";

// Events synthesized by the TUI client itself.
export type LocalClientEventType =
  | "session_created"
  | "system_message"
  | "system_error"
  | "system_warning"
  | "user_message";

export type EventType =
  | ProtocolEventType
  | LegacyCompatEventType
  | LocalClientEventType;

export interface ToolDecisionAudit {
  policy_source: string;
  rule_id?: string;
  action: "allow" | "deny" | "escalate" | string;
  reason?: string;
  capability: "read" | "write" | "network" | "unknown" | string;
  sandbox_backend: string;
}

export interface PlanItem {
  id: string;
  content: string;
  status: "pending" | "in_progress" | "completed";
}

export interface Event {
  type: EventType;
  chunk?: string;
  is_final?: boolean;

  // Yield fields
  request_id?: string;
  kind?: YieldKind;
  payload?: unknown;

  // Tool lifecycle fields (current protocol)
  id?: string;
  name?: string;
  result_preview?: string | null;
  audit?: ToolDecisionAudit;

  // Legacy tool fields kept for compatibility
  call_id?: string;
  tool_name?: string;
  arguments?: Record<string, unknown>;
  result?: unknown;
  success?: boolean;

  // Common metadata
  message?: string;
  recoverable?: boolean;
  summary?: string;

  // Legacy/compat fields
  results?: unknown;
  explanation?: string;
  items?: PlanItem[];
  turns?: number;
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
  type: "text";
  text: string;
}

export interface ContentStructuredPart {
  type: "structured";
  data: unknown;
}

export type ContentPart = ContentTextPart | ContentStructuredPart;

export interface TurnContext {
  workspace_id?: string;
}

export interface DynamicToolSpec {
  name: string;
  description: string;
  parameters: unknown;
  capability?: "read" | "write" | "network";
}

export interface GovernanceConfig {
  profile: "autonomous" | "conservative";
  policy_path?: string;
}

export type StreamingMode = "auto" | "on" | "off";
export type PartialStreamRecoveryMode = "continue_once" | "off";

export type Op =
  | { type: "turn"; parts: ContentPart[]; context?: TurnContext }
  | { type: "input"; parts: ContentPart[] }
  | { type: "resume"; request_id: string; content: ContentPart[] }
  | { type: "interrupt" }
  | { type: "register_dynamic_tools"; tools: DynamicToolSpec[] }
  | { type: "set_client_capabilities"; capabilities: ClientCapabilities }
  | { type: "compact" }
  | { type: "rollback"; turns: number };

// Session types (match daemon API)
export interface SessionListItem {
  session_id: string;
  workspace_id: string;
  active: boolean;
  governance: GovernanceConfig;
  streaming_mode: StreamingMode;
  partial_stream_recovery_mode: PartialStreamRecoveryMode;
}

export interface SessionListResponse {
  sessions: SessionListItem[];
}

export interface SessionReadResponse {
  session_id: string;
  workspace_id: string;
  active: boolean;
  governance: GovernanceConfig;
  streaming_mode: StreamingMode;
  partial_stream_recovery_mode: PartialStreamRecoveryMode;
  rollout_path?: string;
  messages: unknown[];
}

export interface CreateSessionRequest {
  workspace_dir?: string;
  governance?: GovernanceConfig;
  streaming_mode?: StreamingMode;
  partial_stream_recovery_mode?: PartialStreamRecoveryMode;
}

export interface CreateSessionResponse {
  session_id: string;
  websocket_url: string;
  events_url: string;
  submit_url: string;
  governance: GovernanceConfig;
  streaming_mode: StreamingMode;
  partial_stream_recovery_mode: PartialStreamRecoveryMode;
}

export interface DaemonStatus {
  state: "stopped" | "starting" | "running" | "error";
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
