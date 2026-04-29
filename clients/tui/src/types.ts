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
  | "session_rolled_back"
  | "error";

// Legacy/compat runtime events that may appear in historical logs.
export type LegacyCompatEventType =
  | "task_completed"
  | "context_compacted"
  | "plan_updated"
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

export interface PlanSnapshot {
  explanation?: string;
  items: PlanItem[];
  last_updated_event_id: string;
  last_updated_at: number;
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
export type ProviderId =
  | "chatgpt"
  | "google_gemini_generate_content"
  | "openai_responses"
  | "openai_chat_completions"
  | "openai_chat_completions_compatible"
  | "anthropic_messages";
export type CredentialKind =
  | "managed_oauth"
  | "secret_string"
  | "ambient_cloud_auth";
export type ConnectionCredentialStatusKind =
  | "missing"
  | "available"
  | "pending"
  | "expired"
  | "error";

export type Op =
  | { type: "turn"; parts: ContentPart[]; context?: TurnContext }
  | { type: "input"; parts: ContentPart[] }
  | { type: "resume"; request_id: string; content: ContentPart[] }
  | { type: "interrupt" }
  | { type: "register_dynamic_tools"; tools: DynamicToolSpec[] }
  | { type: "set_client_capabilities"; capabilities: ClientCapabilities }
  | { type: "compact_with_options"; focus?: string }
  | { type: "rollback"; turns: number };

// Session types (match daemon API)
export interface SessionListItem {
  session_id: string;
  workspace_id: string;
  active: boolean;
  agent_name?: string;
  profile_id?: string;
  provider?: ProviderId;
  resolved_model: string;
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
  agent_name?: string;
  profile_id?: string;
  provider?: ProviderId;
  resolved_model: string;
  governance: GovernanceConfig;
  streaming_mode: StreamingMode;
  partial_stream_recovery_mode: PartialStreamRecoveryMode;
  rollout_path?: string;
  latest_plan_snapshot?: PlanSnapshot;
  messages: unknown[];
}

export type ChildRunStatus =
  | "starting"
  | "running"
  | "blocked"
  | "completed"
  | "failed"
  | "timed_out"
  | "terminating"
  | "terminated"
  | "cancelled";

export type ChildRunTerminationMode = "graceful" | "forceful";

export interface ChildRunTerminationRequest {
  actor: string;
  reason: string;
  mode: ChildRunTerminationMode;
  requested_at_ms: number;
}

export interface ChildRunRecord {
  id: string;
  parent_session_id: string;
  child_session_id: string;
  workspace_root?: string;
  rollout_path?: string;
  launch_target?: string;
  status: ChildRunStatus;
  created_at_ms: number;
  updated_at_ms: number;
  latest_heartbeat_at_ms?: number;
  latest_progress_at_ms?: number;
  latest_event_kind?: string;
  latest_status_summary?: string;
  warnings?: string[];
  error_message?: string;
  termination?: ChildRunTerminationRequest;
}

export interface ChildRunListResponse {
  child_runs: ChildRunRecord[];
}

export interface ChildRunResponse {
  child_run: ChildRunRecord;
}

export interface TerminateChildRunRequest {
  reason?: string;
  mode?: ChildRunTerminationMode;
  actor?: string;
}

export interface CreateSessionRequest {
  workspace_dir?: string;
  agent_name?: string;
  profile_id?: string;
  governance?: GovernanceConfig;
  streaming_mode?: StreamingMode;
  partial_stream_recovery_mode?: PartialStreamRecoveryMode;
}

export interface CreateSessionResponse {
  session_id: string;
  websocket_url: string;
  events_url: string;
  submit_url: string;
  agent_name?: string;
  profile_id?: string;
  provider?: ProviderId;
  resolved_model: string;
  governance: GovernanceConfig;
  streaming_mode: StreamingMode;
  partial_stream_recovery_mode: PartialStreamRecoveryMode;
}

export interface ProviderDescriptor {
  provider_id: ProviderId;
  display_name: string;
  credential_kind: CredentialKind;
  supports_browser_login: boolean;
  supports_device_login: boolean;
  supports_secret_entry: boolean;
  supports_logout: boolean;
  supports_test: boolean;
  required_settings: string[];
  optional_settings: string[];
  default_settings: Record<string, string>;
}

export interface ConnectionCatalogResponse {
  providers: ProviderDescriptor[];
}

export interface ConnectionProfileSummary {
  profile_id: string;
  label?: string;
  provider: ProviderId;
  credential_id?: string;
  settings: Record<string, string>;
  credential_status: ConnectionCredentialStatusKind;
  is_default: boolean;
  source: string;
  created_at: string;
  updated_at: string;
}

export type ConnectionPinScope = "global" | "workspace";

export type ConnectionSelectionSource =
  | "none"
  | "default_profile"
  | "global_pin"
  | "workspace_pin";

export interface ConnectionPinState {
  scope: ConnectionPinScope;
  config_path: string;
  profile_id: string;
}

export interface ConnectionCurrentState {
  workspace_dir?: string;
  global_pin?: ConnectionPinState;
  workspace_pin?: ConnectionPinState;
  default_profile?: string;
  effective_profile?: string;
  effective_source: ConnectionSelectionSource;
}

export interface ConnectionListResponse {
  default_profile?: string;
  profiles: ConnectionProfileSummary[];
}

export interface ConnectionCredentialStatus {
  profile_id: string;
  credential_id?: string;
  credential_kind: CredentialKind;
  status: ConnectionCredentialStatusKind;
  last_checked_at?: string;
  detail?: {
    account_email?: string;
    account_plan?: string;
    message?: string;
  };
}

export interface CreateConnectionRequest {
  profile_id: string;
  label?: string;
  provider: ProviderId;
  credential_id?: string;
  settings?: Record<string, string>;
  activate?: boolean;
}

export interface UpdateConnectionRequest {
  label?: string;
  credential_id?: string;
  settings?: Record<string, string>;
}

export interface SetConnectionDefaultRequest {
  profile_id: string;
  workspace_dir?: string;
}

export interface ClearConnectionDefaultRequest {
  workspace_dir?: string;
}

export interface PinConnectionRequest {
  profile_id: string;
  scope: ConnectionPinScope;
  workspace_dir?: string;
}

export interface UnpinConnectionRequest {
  scope?: ConnectionPinScope;
  workspace_dir?: string;
}

export interface StartConnectionBrowserLoginRequest {
  timeout_secs?: number;
}

export interface StartConnectionBrowserLoginResponse {
  login_id: string;
  auth_url: string;
  redirect_uri: string;
  created_at: string;
  expires_at: string;
}

export interface StartConnectionDeviceLoginResponse {
  login_id: string;
  verification_url: string;
  user_code: string;
  interval_secs: number;
  created_at: string;
  expires_at: string;
}

export interface ConnectionLoginSuccessResponse {
  account_id: string;
  email?: string;
  plan_type?: string;
  snapshot?: unknown;
}

export interface ConnectionLogoutResponse {
  removed: boolean;
  snapshot?: unknown;
}

export interface ConnectionTestResponse {
  profile_id: string;
  ok: boolean;
  provider: ProviderId;
  resolved_model: string;
  message: string;
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
