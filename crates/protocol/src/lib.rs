//! Protocol definitions for the Alan agent.
//!
//! This crate defines the `Op` (user operations) and `Event` (system events)
//! types that form the communication protocol between the agent core and
//! various frontend interfaces (CLI, REST API, WebSocket).

mod content;
mod event;
mod op;

pub use content::{ContentPart, parts_to_text};
pub use event::{Event, EventEnvelope, YieldKind};
pub use op::{
    ApprovalPolicy, DynamicToolSpec, Op, PlanItem, PlanItemStatus, SandboxMode,
    StructuredInputOption, StructuredInputQuestion, Submission, ToolCapability, TurnContext,
};
