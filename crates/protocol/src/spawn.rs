use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Explicit launch target for a child agent instance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SpawnTarget {
    /// Launch from a resolved on-disk agent root directory.
    ResolvedAgentRoot { root_dir: PathBuf },
}

/// Shared parent-side handle that may be explicitly bound into a child launch.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SpawnHandle {
    Workspace,
    Artifacts,
    Memory,
    Plan,
    ConversationSnapshot,
    ToolResults,
    ApprovalScope,
}

/// Launch inputs supplied for a child runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SpawnLaunchInputs {
    pub task: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_root: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<PathBuf>,
}

/// First-version tool profile override for a child launch.
///
/// Alan does not have stable named host profiles yet, so the initial contract
/// models a profile override as an explicit tool allowlist.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SpawnToolProfileOverride {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
}

impl SpawnToolProfileOverride {
    pub fn is_empty(&self) -> bool {
        self.allowed_tools.is_empty()
    }
}

/// Runtime overrides applied to the child runtime at launch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SpawnRuntimeOverrides {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_profile: Option<SpawnToolProfileOverride>,
}

impl SpawnRuntimeOverrides {
    pub fn is_empty(&self) -> bool {
        self.model.is_none()
            && self.policy_path.is_none()
            && self
                .tool_profile
                .as_ref()
                .is_none_or(SpawnToolProfileOverride::is_empty)
    }
}

/// Explicit child-agent launch contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpawnSpec {
    pub target: SpawnTarget,
    pub launch: SpawnLaunchInputs,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handles: Vec<SpawnHandle>,
    #[serde(default, skip_serializing_if = "SpawnRuntimeOverrides::is_empty")]
    pub runtime_overrides: SpawnRuntimeOverrides,
}

impl SpawnSpec {
    pub fn has_handle(&self, handle: SpawnHandle) -> bool {
        self.handles.contains(&handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_spec_round_trips_with_handles_and_overrides() {
        let spec = SpawnSpec {
            target: SpawnTarget::ResolvedAgentRoot {
                root_dir: PathBuf::from("/tmp/child"),
            },
            launch: SpawnLaunchInputs {
                task: "Review the repository".to_string(),
                cwd: Some(PathBuf::from("/tmp/workspace")),
                workspace_root: Some(PathBuf::from("/tmp/workspace")),
                timeout_secs: Some(120),
                budget_tokens: Some(2048),
                output_dir: Some(PathBuf::from("/tmp/workspace/out")),
            },
            handles: vec![
                SpawnHandle::Workspace,
                SpawnHandle::ConversationSnapshot,
                SpawnHandle::ToolResults,
            ],
            runtime_overrides: SpawnRuntimeOverrides {
                model: Some("gpt-5.4".to_string()),
                policy_path: Some(".alan/agent/policy.yaml".to_string()),
                tool_profile: Some(SpawnToolProfileOverride {
                    allowed_tools: vec!["read_file".to_string(), "grep".to_string()],
                }),
            },
        };

        let value = serde_json::to_value(&spec).unwrap();
        assert_eq!(value["target"]["kind"], "resolved_agent_root");
        assert_eq!(value["handles"][0], "workspace");
        assert_eq!(value["runtime_overrides"]["model"], "gpt-5.4");

        let parsed: SpawnSpec = serde_json::from_value(value).unwrap();
        assert!(parsed.has_handle(SpawnHandle::Workspace));
        assert!(parsed.has_handle(SpawnHandle::ConversationSnapshot));
        assert_eq!(
            parsed.runtime_overrides.tool_profile.unwrap().allowed_tools,
            vec!["read_file".to_string(), "grep".to_string()]
        );
    }
}
