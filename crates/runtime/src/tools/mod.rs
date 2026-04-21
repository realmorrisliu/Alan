//! Tool registry and execution infrastructure.
//!
//! This module provides the `Tool` trait, `ToolRegistry`, `ToolContext`,
//! and `Sandbox` abstractions. Concrete tool implementations live in the
//! `alan-tools` crate.

mod context;
mod registry;
mod sandbox;

pub use context::ToolContext;
pub use registry::{Tool, ToolLocality, ToolRegistry, ToolResult};
pub use sandbox::{ExecResult, Sandbox};
