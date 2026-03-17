//! Protocol definitions for the Alan agent.
//!
//! This crate defines the `Op` (user operations) and `Event` (system events)
//! types that form the communication protocol between the agent core and
//! various frontend interfaces (CLI, REST API, WebSocket).

mod adaptive;
mod compaction;
mod content;
mod event;
mod op;

pub use adaptive::{
    AdaptiveForm, AdaptivePresentationHint, AdaptiveYieldCapabilities, ClientCapabilities,
    ConfirmationYieldPayload, CustomYieldPayload, DynamicToolYieldPayload, StructuredInputKind,
    StructuredInputOption, StructuredInputQuestion, StructuredInputYieldPayload,
};
pub use compaction::{
    AppliedCompactionOutcome, CompactionAttemptSnapshot, CompactionMode, CompactionOutcome,
    CompactionReason, CompactionRequestMetadata, CompactionResult, CompactionSkipReason,
    CompactionTrigger, FailedCompactionOutcome, SkippedCompactionOutcome,
};
pub use content::{ContentPart, parts_to_text};
pub use event::{Event, EventEnvelope, ToolDecisionAudit, YieldKind};
pub use op::{
    DynamicToolSpec, GovernanceConfig, GovernanceProfile, InputMode, Op, PlanItem, PlanItemStatus,
    Submission, ToolCapability, TurnContext,
};
