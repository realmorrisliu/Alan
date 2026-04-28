use crate::paths::AlanHomePaths;
use std::{
    ffi::OsStr,
    path::{Component, Path, PathBuf},
};

pub const DEFAULT_AGENT_NAME: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRootKind {
    GlobalDefault,
    WorkspaceDefault,
    GlobalNamed(String),
    WorkspaceNamed(String),
    LaunchRoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRootPaths {
    pub kind: AgentRootKind,
    pub root_dir: PathBuf,
    pub config_path: PathBuf,
    pub persona_dir: PathBuf,
    pub skills_dir: PathBuf,
    pub policy_path: PathBuf,
}

impl AgentRootPaths {
    pub fn new(kind: AgentRootKind, root_dir: PathBuf) -> Self {
        Self {
            kind,
            config_path: root_dir.join("agent.toml"),
            persona_dir: root_dir.join("persona"),
            skills_dir: root_dir.join("skills"),
            policy_path: root_dir.join("policy.yaml"),
            root_dir,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolvedAgentRoots {
    roots: Vec<AgentRootPaths>,
}

impl ResolvedAgentRoots {
    pub fn for_workspace(workspace_root: Option<&Path>, agent_name: Option<&str>) -> Self {
        Self::with_home_paths(AlanHomePaths::detect(), workspace_root, agent_name)
    }

    pub(crate) fn with_home_paths(
        home_paths: Option<AlanHomePaths>,
        workspace_root: Option<&Path>,
        agent_name: Option<&str>,
    ) -> Self {
        let mut roots = Vec::new();

        if let Some(home_paths) = home_paths {
            roots.push(AgentRootPaths::new(
                AgentRootKind::GlobalDefault,
                home_paths.global_agent_root_dir,
            ));
            if let Some(workspace_root) = workspace_root {
                roots.push(AgentRootPaths::new(
                    AgentRootKind::WorkspaceDefault,
                    workspace_agent_root_dir(workspace_root),
                ));
            }
            if let Some(name) = normalize_named_agent_name(agent_name) {
                roots.push(AgentRootPaths::new(
                    AgentRootKind::GlobalNamed(name.to_string()),
                    home_paths.global_named_agents_dir.join(name),
                ));
                if let Some(workspace_root) = workspace_root {
                    roots.push(AgentRootPaths::new(
                        AgentRootKind::WorkspaceNamed(name.to_string()),
                        workspace_named_agent_root_dir(workspace_root, name),
                    ));
                }
            }
        } else if let Some(workspace_root) = workspace_root {
            roots.push(AgentRootPaths::new(
                AgentRootKind::WorkspaceDefault,
                workspace_agent_root_dir(workspace_root),
            ));
            if let Some(name) = normalize_named_agent_name(agent_name) {
                roots.push(AgentRootPaths::new(
                    AgentRootKind::WorkspaceNamed(name.to_string()),
                    workspace_named_agent_root_dir(workspace_root, name),
                ));
            }
        }

        Self { roots }
    }

    pub fn roots(&self) -> &[AgentRootPaths] {
        &self.roots
    }

    pub fn with_appended_root(mut self, root: AgentRootPaths) -> Self {
        self.roots.push(root);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    pub fn config_paths(&self) -> Vec<PathBuf> {
        self.roots
            .iter()
            .map(|root| root.config_path.clone())
            .collect()
    }

    pub fn persona_dirs(&self) -> Vec<PathBuf> {
        self.roots
            .iter()
            .map(|root| root.persona_dir.clone())
            .collect()
    }

    pub fn skills_dirs(&self) -> Vec<PathBuf> {
        self.roots
            .iter()
            .map(|root| root.skills_dir.clone())
            .collect()
    }

    pub fn highest_precedence_policy_path(&self) -> Option<PathBuf> {
        self.roots
            .iter()
            .rev()
            .map(|root| root.policy_path.clone())
            .find(|path| path.exists())
    }

    pub fn writable_root_dir(&self) -> Option<PathBuf> {
        self.roots.last().map(|root| root.root_dir.clone())
    }

    pub fn writable_persona_dir(&self) -> Option<PathBuf> {
        self.roots
            .iter()
            .rev()
            .find(|root| {
                matches!(
                    root.kind,
                    AgentRootKind::LaunchRoot
                        | AgentRootKind::WorkspaceDefault
                        | AgentRootKind::WorkspaceNamed(_)
                )
            })
            .map(|root| root.persona_dir.clone())
    }
}

pub fn workspace_alan_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".alan")
}

pub fn workspace_sessions_dir(workspace_root: &Path) -> PathBuf {
    workspace_sessions_dir_from_alan_dir(&workspace_alan_dir(workspace_root))
}

pub fn workspace_sessions_dir_from_alan_dir(workspace_alan_dir: &Path) -> PathBuf {
    workspace_alan_dir.join("sessions")
}

pub fn workspace_memory_dir(workspace_root: &Path) -> PathBuf {
    workspace_memory_dir_from_alan_dir(&workspace_alan_dir(workspace_root))
}

pub fn workspace_memory_dir_from_alan_dir(workspace_alan_dir: &Path) -> PathBuf {
    workspace_alan_dir.join("memory")
}

pub fn workspace_agent_root_dir(workspace_root: &Path) -> PathBuf {
    workspace_agent_root_dir_from_alan_dir(&workspace_alan_dir(workspace_root))
}

pub fn workspace_agent_root_dir_from_alan_dir(workspace_alan_dir: &Path) -> PathBuf {
    workspace_alan_dir.join("agents").join(DEFAULT_AGENT_NAME)
}

pub fn workspace_persona_dir(workspace_root: &Path) -> PathBuf {
    workspace_persona_dir_from_alan_dir(&workspace_alan_dir(workspace_root))
}

pub fn workspace_persona_dir_from_alan_dir(workspace_alan_dir: &Path) -> PathBuf {
    workspace_agent_root_dir_from_alan_dir(workspace_alan_dir).join("persona")
}

pub fn workspace_skills_dir(workspace_root: &Path) -> PathBuf {
    workspace_skills_dir_from_alan_dir(&workspace_alan_dir(workspace_root))
}

pub fn workspace_skills_dir_from_alan_dir(workspace_alan_dir: &Path) -> PathBuf {
    workspace_agent_root_dir_from_alan_dir(workspace_alan_dir).join("skills")
}

pub fn workspace_named_agents_dir(workspace_root: &Path) -> PathBuf {
    workspace_alan_dir(workspace_root).join("agents")
}

pub fn workspace_public_skills_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".agents").join("skills")
}

pub fn workspace_named_agent_root_dir(workspace_root: &Path, agent_name: &str) -> PathBuf {
    workspace_named_agents_dir(workspace_root).join(agent_name)
}

pub fn normalize_agent_name(agent_name: Option<&str>) -> Option<&str> {
    agent_name.and_then(|name| {
        let trimmed = name.trim();
        if trimmed.is_empty() || !is_single_path_component(trimmed) {
            None
        } else {
            Some(trimmed)
        }
    })
}

pub fn normalize_named_agent_name(agent_name: Option<&str>) -> Option<&str> {
    normalize_agent_name(agent_name).and_then(|name| {
        if name == DEFAULT_AGENT_NAME {
            None
        } else {
            Some(name)
        }
    })
}

fn is_single_path_component(name: &str) -> bool {
    let mut components = Path::new(name).components();
    matches!(components.next(), Some(Component::Normal(component)) if component == OsStr::new(name))
        && components.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn resolve_workspace_default_roots_in_overlay_order() {
        let workspace_root = Path::new("/tmp/demo-workspace");
        let roots = ResolvedAgentRoots::with_home_paths(
            Some(AlanHomePaths::from_home_dir(Path::new("/tmp/demo-home"))),
            Some(workspace_root),
            None,
        );

        assert_eq!(roots.roots().len(), 2);
        assert!(matches!(
            roots.roots()[0].kind,
            AgentRootKind::GlobalDefault
        ));
        assert_eq!(
            roots.roots()[0].root_dir,
            Path::new("/tmp/demo-home/.alan/agents/default")
        );
        assert!(matches!(
            roots.roots()[1].kind,
            AgentRootKind::WorkspaceDefault
        ));
        assert_eq!(
            roots.roots()[1].root_dir,
            workspace_root.join(".alan").join("agents").join("default")
        );
    }

    #[test]
    fn resolve_named_agent_roots_after_base_roots() {
        let workspace_root = Path::new("/tmp/demo-workspace");
        let roots = ResolvedAgentRoots::with_home_paths(
            Some(AlanHomePaths::from_home_dir(Path::new("/tmp/demo-home"))),
            Some(workspace_root),
            Some("coder"),
        );

        assert_eq!(roots.roots().len(), 4);
        assert!(matches!(
            roots.roots()[0].kind,
            AgentRootKind::GlobalDefault
        ));
        assert!(matches!(
            roots.roots()[1].kind,
            AgentRootKind::WorkspaceDefault
        ));
        assert!(
            matches!(roots.roots()[2].kind, AgentRootKind::GlobalNamed(ref name) if name == "coder")
        );
        assert!(
            matches!(roots.roots()[3].kind, AgentRootKind::WorkspaceNamed(ref name) if name == "coder")
        );
        assert_eq!(
            roots.roots()[2].root_dir,
            Path::new("/tmp/demo-home/.alan/agents/coder")
        );
        assert_eq!(
            roots.roots()[3].root_dir,
            workspace_root.join(".alan").join("agents").join("coder")
        );
    }

    #[test]
    fn highest_precedence_policy_path_prefers_workspace_named_root() {
        let workspace_root = tempfile::TempDir::new().unwrap();
        let roots = ResolvedAgentRoots::with_home_paths(
            Some(AlanHomePaths::from_home_dir(Path::new("/tmp/demo-home"))),
            Some(workspace_root.path()),
            Some("coder"),
        );
        std::fs::create_dir_all(roots.roots()[0].root_dir.clone()).unwrap();
        std::fs::write(&roots.roots()[0].policy_path, "rules: []\n").unwrap();
        std::fs::create_dir_all(roots.roots()[3].root_dir.clone()).unwrap();
        std::fs::write(&roots.roots()[3].policy_path, "rules: []\n").unwrap();

        assert_eq!(
            roots.highest_precedence_policy_path(),
            Some(roots.roots()[3].policy_path.clone())
        );
    }

    #[test]
    fn writable_persona_dir_prefers_workspace_root() {
        let workspace_root = Path::new("/tmp/demo-workspace");
        let roots = ResolvedAgentRoots::with_home_paths(
            Some(AlanHomePaths::from_home_dir(Path::new("/tmp/demo-home"))),
            Some(workspace_root),
            None,
        );

        assert_eq!(
            roots.writable_persona_dir(),
            Some(
                workspace_root
                    .join(".alan")
                    .join("agents")
                    .join("default")
                    .join("persona")
            )
        );
    }

    #[test]
    fn explicit_default_agent_name_uses_default_roots_only() {
        let workspace_root = Path::new("/tmp/demo-workspace");
        let roots = ResolvedAgentRoots::with_home_paths(
            Some(AlanHomePaths::from_home_dir(Path::new("/tmp/demo-home"))),
            Some(workspace_root),
            Some(DEFAULT_AGENT_NAME),
        );

        assert_eq!(roots.roots().len(), 2);
        assert!(matches!(
            roots.roots()[0].kind,
            AgentRootKind::GlobalDefault
        ));
        assert!(matches!(
            roots.roots()[1].kind,
            AgentRootKind::WorkspaceDefault
        ));
        assert!(roots.roots().iter().all(|root| !matches!(
            root.kind,
            AgentRootKind::GlobalNamed(_) | AgentRootKind::WorkspaceNamed(_)
        )));
    }

    #[test]
    fn writable_root_dir_prefers_highest_precedence_root() {
        let workspace_root = Path::new("/tmp/demo-workspace");
        let roots = ResolvedAgentRoots::with_home_paths(
            Some(AlanHomePaths::from_home_dir(Path::new("/tmp/demo-home"))),
            Some(workspace_root),
            Some("coder"),
        );

        assert_eq!(
            roots.writable_root_dir(),
            Some(workspace_root.join(".alan").join("agents").join("coder"))
        );
    }

    #[test]
    fn named_agent_roots_ignore_path_traversal_names() {
        let workspace_root = Path::new("/tmp/demo-workspace");
        let roots = ResolvedAgentRoots::with_home_paths(
            Some(AlanHomePaths::from_home_dir(Path::new("/tmp/demo-home"))),
            Some(workspace_root),
            Some("../coder"),
        );

        assert_eq!(roots.roots().len(), 2);
        assert!(matches!(
            roots.roots()[0].kind,
            AgentRootKind::GlobalDefault
        ));
        assert!(matches!(
            roots.roots()[1].kind,
            AgentRootKind::WorkspaceDefault
        ));
    }

    #[test]
    fn named_agent_roots_keep_single_component_names() {
        let workspace_root = Path::new("/tmp/demo-workspace");
        let roots = ResolvedAgentRoots::with_home_paths(
            Some(AlanHomePaths::from_home_dir(Path::new("/tmp/demo-home"))),
            Some(workspace_root),
            Some("coder.v2"),
        );

        assert_eq!(roots.roots().len(), 4);
        assert_eq!(
            roots.roots()[2].root_dir,
            Path::new("/tmp/demo-home/.alan/agents/coder.v2")
        );
        assert_eq!(
            roots.roots()[3].root_dir,
            workspace_root.join(".alan").join("agents").join("coder.v2")
        );
    }

    #[test]
    fn workspace_layout_helpers_share_the_same_canonical_layout() {
        let workspace_root = Path::new("/tmp/demo-workspace");
        let alan_dir = workspace_alan_dir(workspace_root);

        assert_eq!(
            workspace_sessions_dir(workspace_root),
            alan_dir.join("sessions")
        );
        assert_eq!(
            workspace_sessions_dir_from_alan_dir(&alan_dir),
            alan_dir.join("sessions")
        );
        assert_eq!(
            workspace_memory_dir(workspace_root),
            alan_dir.join("memory")
        );
        assert_eq!(
            workspace_memory_dir_from_alan_dir(&alan_dir),
            alan_dir.join("memory")
        );
        assert_eq!(
            workspace_agent_root_dir(workspace_root),
            alan_dir.join("agents").join("default")
        );
        assert_eq!(
            workspace_agent_root_dir_from_alan_dir(&alan_dir),
            alan_dir.join("agents").join("default")
        );
        assert_eq!(
            workspace_persona_dir(workspace_root),
            alan_dir.join("agents/default/persona")
        );
        assert_eq!(
            workspace_persona_dir_from_alan_dir(&alan_dir),
            alan_dir.join("agents/default/persona")
        );
        assert_eq!(
            workspace_skills_dir(workspace_root),
            alan_dir.join("agents/default/skills")
        );
        assert_eq!(
            workspace_skills_dir_from_alan_dir(&alan_dir),
            alan_dir.join("agents/default/skills")
        );
        assert_eq!(
            workspace_named_agents_dir(workspace_root),
            alan_dir.join("agents")
        );
    }
}
