//! Protocol definitions for the Alan agent.
//!
//! This crate defines the `Op` (user operations) and `Event` (system events)
//! types that form the communication protocol between the agent core and
//! various frontend interfaces (CLI, REST API, WebSocket).

mod event;
mod op;

pub use event::{Event, EventEnvelope, YieldKind};
pub use op::{
    ApprovalPolicy, ConfirmChoice, DynamicToolSpec, Op, PlanItem, PlanItemStatus, SandboxMode,
    StructuredInputAnswer, StructuredInputOption, StructuredInputQuestion, Submission,
    ToolCapability, TurnContext,
};
