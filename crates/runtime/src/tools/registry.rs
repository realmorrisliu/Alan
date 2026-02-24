//! Tool registry for managing and executing tools.

use super::context::ToolContext;
use anyhow::Result;
use jsonschema::{Draft, JSONSchema};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::debug;

use crate::config::Config;
use crate::llm::ToolDefinition;

/// Result type for tool execution
pub type ToolResult = Pin<Box<dyn Future<Output = Result<Value>> + Send>>;

/// A tool that can be executed by the agent
pub trait Tool: Send + Sync {
    /// Get the tool's name
    fn name(&self) -> &str;

    /// Get the tool's description
    fn description(&self) -> &str;

    /// Get the JSON Schema for the tool's parameters
    fn parameters_schema(&self) -> Value;

    /// Execute the tool with the given arguments and context
    fn execute(&self, arguments: Value, ctx: &ToolContext) -> ToolResult;

    /// Coarse capability classification used by runtime approval/sandbox policy.
    fn capability(&self, _arguments: &Value) -> alan_protocol::ToolCapability {
        alan_protocol::ToolCapability::Read
    }

    /// Get the recommended timeout for this tool in seconds.
    fn timeout_secs(&self) -> usize {
        30
    }
}

/// Registry for managing tools
#[derive(Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    config: Arc<Config>,
    schema_cache: Arc<std::sync::Mutex<HashMap<String, Arc<JSONSchema>>>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        let config = Arc::new(Config::from_env());
        Self::with_config(config)
    }

    pub fn with_config(config: Arc<Config>) -> Self {
        Self {
            tools: HashMap::new(),
            config,
            schema_cache: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Register a tool
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let name = tool.name().to_string();
        debug!(%name, "Registering tool");
        self.schema_cache
            .lock()
            .expect("schema cache mutex poisoned")
            .remove(&name);
        self.tools.insert(name, Arc::new(tool));
    }

    /// Register a boxed tool (for dynamic tools)
    pub fn register_boxed(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        debug!(%name, "Registering boxed tool");
        self.schema_cache
            .lock()
            .expect("schema cache mutex poisoned")
            .remove(&name);
        self.tools.insert(name, Arc::from(tool));
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Check if a tool exists
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Resolve a tool's coarse capability classification.
    pub fn capability_for_tool(
        &self,
        name: &str,
        arguments: &Value,
    ) -> Option<alan_protocol::ToolCapability> {
        self.get(name).map(|tool| tool.capability(arguments))
    }

    /// Get all registered tool names
    pub fn list_tools(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    /// Get tool definitions for LLM function calling
    pub fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: tool.parameters_schema(),
            })
            .collect()
    }

    /// Execute a tool by name with the given context
    pub async fn execute_with_context(
        &self,
        name: &str,
        arguments: Value,
        ctx: &ToolContext,
    ) -> Result<Value> {
        if let Some(tool) = self.get(name) {
            debug!(%name, "Executing tool");
            self.validate_tool_args(tool.as_ref(), &arguments)?;

            // Use tool-specific timeout
            let timeout_secs = if self.config.tool_timeout_secs != 30 {
                self.config.tool_timeout_secs
            } else {
                tool.timeout_secs()
            };

            if timeout_secs == 0 {
                tool.execute(arguments, ctx).await
            } else {
                let timeout = std::time::Duration::from_secs(timeout_secs as u64);
                match tokio::time::timeout(timeout, tool.execute(arguments, ctx)).await {
                    Ok(result) => result,
                    Err(_) => anyhow::bail!("Tool execution timed out after {}s", timeout_secs),
                }
            }
        } else {
            anyhow::bail!("Tool not found: {}", name)
        }
    }

    /// Execute a tool by name (backward compatible, uses default context)
    /// Note: This creates a default ToolContext. Prefer execute_with_context for production use.
    pub async fn execute(&self, name: &str, arguments: Value) -> Result<Value> {
        // Create a default context - this is a simplification for backward compatibility
        let ctx = ToolContext::new(
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            std::env::temp_dir(),
            self.config.clone(),
        );
        self.execute_with_context(name, arguments, &ctx).await
    }

    fn validate_tool_args(&self, tool: &dyn Tool, arguments: &Value) -> Result<()> {
        let tool_name = tool.name().to_string();
        let compiled = {
            let mut cache = self
                .schema_cache
                .lock()
                .expect("schema cache mutex poisoned");
            if let Some(schema) = cache.get(&tool_name) {
                Arc::clone(schema)
            } else {
                let schema = tool.parameters_schema();
                let compiled = Arc::new(
                    JSONSchema::options()
                        .with_draft(Draft::Draft7)
                        .compile(&schema)
                        .map_err(|e| {
                            anyhow::anyhow!("Invalid tool schema for {}: {}", tool.name(), e)
                        })?,
                );
                cache.insert(tool_name.clone(), Arc::clone(&compiled));
                compiled
            }
        };

        if let Err(errors) = compiled.validate(arguments) {
            let details: Vec<String> = errors
                .map(|err| {
                    let path = err.instance_path.to_string();
                    let path = if path.is_empty() {
                        "/".to_string()
                    } else {
                        path
                    };
                    format!("{}: {}", path, err)
                })
                .collect();
            anyhow::bail!(
                "Tool arguments validation failed for {}: {}",
                tool.name(),
                details.join("; ")
            );
        }

        Ok(())
    }

    /// Validate that required tools are available
    pub fn validate_required_tools(&self, required: &[String]) -> Result<Vec<String>> {
        let mut missing = Vec::new();
        for tool in required {
            if !self.has(tool) {
                missing.push(tool.clone());
            }
        }
        Ok(missing)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestTool;

    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }

        fn description(&self) -> &str {
            "A test tool"
        }

        fn parameters_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                }
            })
        }

        fn execute(&self, arguments: Value, _ctx: &ToolContext) -> ToolResult {
            Box::pin(async move {
                let input = arguments["input"].as_str().unwrap_or("default");
                Ok(serde_json::json!({"result": input}))
            })
        }
    }

    fn test_ctx() -> ToolContext {
        ToolContext::new(
            PathBuf::from("/workspace"),
            PathBuf::from("/tmp"),
            Arc::new(Config::default()),
        )
    }

    #[test]
    fn test_tool_registry_new() {
        let registry = ToolRegistry::new();
        assert!(registry.list_tools().is_empty());
    }

    #[test]
    fn test_tool_registry_register() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);
        assert!(registry.has("test_tool"));
        assert_eq!(registry.list_tools().len(), 1);
    }

    #[tokio::test]
    async fn test_tool_registry_execute() {
        let mut registry = ToolRegistry::new();
        registry.register(TestTool);

        let ctx = test_ctx();
        let args = serde_json::json!({"input": "hello"});
        let result = registry.execute_with_context("test_tool", args, &ctx).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap()["result"], "hello");
    }

    #[tokio::test]
    async fn test_tool_registry_execute_nonexistent() {
        let registry = ToolRegistry::new();
        let ctx = test_ctx();
        let args = serde_json::json!({});

        let result = registry.execute_with_context("nonexistent", args, &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    struct SlowTestTool {
        delay_ms: u64,
    }

    impl Tool for SlowTestTool {
        fn name(&self) -> &str {
            "slow_tool"
        }
        fn description(&self) -> &str {
            "A slow tool"
        }
        fn parameters_schema(&self) -> Value {
            serde_json::json!({"type": "object"})
        }
        fn execute(&self, _args: Value, _ctx: &ToolContext) -> ToolResult {
            let delay = self.delay_ms;
            Box::pin(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                Ok(serde_json::json!({"status": "done"}))
            })
        }
    }

    #[tokio::test]
    async fn test_tool_timeout() {
        let config = Arc::new(Config {
            tool_timeout_secs: 1,
            ..Default::default()
        });
        let mut registry = ToolRegistry::with_config(config);
        registry.register(SlowTestTool { delay_ms: 2000 });

        let ctx = test_ctx();
        let result = registry.execute_with_context("slow_tool", serde_json::json!({}), &ctx).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    struct CountingTool {
        calls: Arc<AtomicUsize>,
    }

    impl Tool for CountingTool {
        fn name(&self) -> &str {
            "counting_tool"
        }
        fn description(&self) -> &str {
            "Counts schema calls"
        }
        fn parameters_schema(&self) -> Value {
            self.calls.fetch_add(1, Ordering::SeqCst);
            serde_json::json!({"type": "object"})
        }
        fn execute(&self, _args: Value, _ctx: &ToolContext) -> ToolResult {
            Box::pin(async move { Ok(serde_json::json!({})) })
        }
    }

    #[tokio::test]
    async fn test_schema_caching() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut registry = ToolRegistry::new();
        registry.register(CountingTool {
            calls: Arc::clone(&calls),
        });

        let ctx = test_ctx();
        registry
            .execute_with_context("counting_tool", serde_json::json!({}), &ctx)
            .await
            .unwrap();
        registry
            .execute_with_context("counting_tool", serde_json::json!({}), &ctx)
            .await
            .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
