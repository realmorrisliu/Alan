//! Workspace Resolver - path resolution layer that maps workspace identifiers to paths.
//!
//! Resolution priority:
//! 1. Alias from the CLI registry
//! 2. Short ID (6 chars) from the CLI registry
//! 3. Identifier interpreted as a path (if valid)
//! 4. Default workspace path

use crate::registry::{WorkspaceRegistry, generate_workspace_id};
use anyhow::{Context, Result, ensure};
use std::{
    ffi::{OsStr, OsString},
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};
use tracing::{debug, warn};

/// Workspace resolution result
#[derive(Debug, Clone)]
pub struct ResolvedWorkspace {
    /// Workspace ID (short hash, 6 chars)
    #[allow(dead_code)]
    pub id: String,
    /// Canonical absolute path
    pub path: PathBuf,
    /// Workspace state directory (`.alan`)
    pub alan_dir: PathBuf,
    /// Optional alias
    #[allow(dead_code)]
    pub alias: Option<String>,
    /// Whether this workspace is registered in the registry
    #[allow(dead_code)]
    pub registered: bool,
}

/// Workspace path resolver
#[derive(Debug, Clone)]
pub struct WorkspaceResolver {
    registry: WorkspaceRegistry,
    default_workspace_dir: PathBuf,
}

impl WorkspaceResolver {
    /// Create a new resolver and load the CLI registry
    pub fn new() -> Result<Self> {
        let registry = WorkspaceRegistry::load()?;
        let default_workspace_dir = Self::default_workspace_dir()?;

        Ok(Self {
            registry,
            default_workspace_dir,
        })
    }

    /// Create with an explicit registry and default workspace directory.
    #[allow(dead_code)]
    pub fn with_registry(registry: WorkspaceRegistry, default_dir: PathBuf) -> Self {
        Self {
            registry,
            default_workspace_dir: default_dir,
        }
    }

    /// Get the default workspace directory (`~/.alan/`)
    fn default_workspace_dir() -> Result<PathBuf> {
        alan_runtime::AlanHomePaths::detect()
            .map(|paths| paths.alan_home_dir)
            .context("Cannot determine home directory")
    }

    /// Resolve a workspace identifier to a path
    ///
    /// Supported identifier formats:
    /// - Registry alias (for example, `"my-project"`)
    /// - Short ID (for example, `"a1b2c3"`)
    /// - Absolute path (for example, `"/home/user/projects/myapp"`)
    /// - Relative path (relative to the current working directory)
    /// - `None` (returns the default workspace)
    pub fn resolve(&self, identifier: Option<&str>) -> Result<ResolvedWorkspace> {
        // `None` means "use the default workspace".
        if identifier.is_none() {
            return self.default_workspace();
        }
        let identifier = identifier.unwrap();

        // 1. Try resolving from the registry (alias or short ID).
        if let Some(entry) = self.registry.find(identifier) {
            let (workspace_path, workspace_alan_dir) =
                self.normalize_workspace_path_and_alan_dir(&entry.path);
            debug!(%identifier, path = %entry.path.display(), "Resolved workspace from registry");
            return Ok(ResolvedWorkspace {
                id: entry.id.clone(),
                path: workspace_path,
                alan_dir: workspace_alan_dir,
                alias: Some(entry.alias.clone()),
                registered: true,
            });
        }

        // 2. Try resolving it as a path.
        let path = Path::new(identifier);
        let canonical = Self::canonicalize_path(path)?;
        let (workspace_path, workspace_alan_dir) =
            self.normalize_workspace_path_and_alan_dir(&canonical);

        // Check whether the path contains a `.alan` directory (initialized workspace).
        if !self.is_valid_workspace(&workspace_path) {
            warn!(
                path = %workspace_path.display(),
                "Path is not a valid workspace (missing workspace state directory)"
            );
        }

        // Generate an ID using the same algorithm as the registry.
        let id = generate_workspace_id(&workspace_path);

        // Check whether this path is actually in the registry (path match).
        let registered = self.registry.find(&id).is_some();

        Ok(ResolvedWorkspace {
            id,
            path: workspace_path,
            alan_dir: workspace_alan_dir,
            alias: None,
            registered,
        })
    }

    /// Resolve and ensure the workspace directory exists
    ///
    /// If the path is not initialized (missing `.alan`), create the directory structure.
    pub fn resolve_or_create(&self, identifier: Option<&str>) -> Result<ResolvedWorkspace> {
        let resolved = self.resolve(identifier)?;

        // Ensure workspace state structure exists and is complete.
        if !resolved.alan_dir.exists() {
            debug!(path = %resolved.path.display(), "Creating workspace directory structure");
        }
        self.create_workspace_structure(&resolved)?;

        let workspace_path = if resolved.path == self.default_workspace_dir {
            std::fs::canonicalize(&self.default_workspace_dir).with_context(|| {
                format!(
                    "Failed to canonicalize default workspace: {}",
                    self.default_workspace_dir.display()
                )
            })?
        } else {
            std::fs::canonicalize(Self::normalize_creation_path(&resolved.path)).with_context(
                || {
                    format!(
                        "Failed to canonicalize workspace: {}",
                        resolved.path.display()
                    )
                },
            )?
        };
        let alan_dir = if resolved.alan_dir == self.default_workspace_dir {
            workspace_path.clone()
        } else {
            std::fs::canonicalize(workspace_path.join(".alan")).with_context(|| {
                format!(
                    "Failed to canonicalize workspace state directory: {}",
                    resolved.alan_dir.display()
                )
            })?
        };

        Ok(ResolvedWorkspace {
            path: workspace_path,
            alan_dir,
            ..resolved
        })
    }

    /// Get the default workspace
    pub fn default_workspace(&self) -> Result<ResolvedWorkspace> {
        let path = self.default_workspace_dir.clone();

        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }

        let id = generate_workspace_id(&path);

        Ok(ResolvedWorkspace {
            id,
            path,
            alan_dir: self.default_workspace_dir.clone(),
            alias: Some("default".to_string()),
            registered: false,
        })
    }

    /// Get the `.alan` directory for a workspace
    pub fn workspace_alan_dir(&self, workspace_path: &Path) -> PathBuf {
        if workspace_path == self.default_workspace_dir
            || workspace_path
                .file_name()
                .map(|name| name == std::ffi::OsStr::new(".alan"))
                .unwrap_or(false)
        {
            workspace_path.to_path_buf()
        } else {
            alan_runtime::workspace_alan_dir(workspace_path)
        }
    }

    /// Get a specific workspace subdirectory (for example, `sessions`)
    #[allow(dead_code)]
    pub fn workspace_sessions_dir(&self, workspace_path: &Path) -> PathBuf {
        alan_runtime::workspace_sessions_dir_from_alan_dir(&self.workspace_alan_dir(workspace_path))
    }

    /// Get the workspace `memory` directory
    #[allow(dead_code)]
    pub fn workspace_memory_dir(&self, workspace_path: &Path) -> PathBuf {
        alan_runtime::workspace_memory_dir_from_alan_dir(&self.workspace_alan_dir(workspace_path))
    }

    /// Get the workspace `persona` directory
    #[allow(dead_code)]
    pub fn workspace_persona_dir(&self, workspace_path: &Path) -> PathBuf {
        alan_runtime::workspace_persona_dir_from_alan_dir(&self.workspace_alan_dir(workspace_path))
    }

    /// Check whether a path is a valid workspace (contains a workspace state directory)
    pub fn is_valid_workspace(&self, path: &Path) -> bool {
        self.workspace_alan_dir(path).is_dir()
    }

    /// Canonicalize a path
    fn canonicalize_path(path: &Path) -> Result<PathBuf> {
        if path.exists() {
            // Path exists, canonicalize it.
            std::fs::canonicalize(path)
                .with_context(|| format!("Failed to canonicalize path: {}", path.display()))
        } else {
            // Path does not exist, check whether it is relative.
            if path.is_relative() {
                let cwd = std::env::current_dir()?;
                let absolute = cwd.join(path);
                if absolute.exists() {
                    std::fs::canonicalize(&absolute).with_context(|| {
                        format!("Failed to canonicalize path: {}", absolute.display())
                    })
                } else {
                    // Path does not exist, but return absolute path (may be created later).
                    Ok(absolute)
                }
            } else {
                // Absolute path that does not exist.
                Ok(path.to_path_buf())
            }
        }
    }

    /// Create workspace directory structure
    fn create_workspace_structure(&self, resolved: &ResolvedWorkspace) -> Result<()> {
        self.ensure_workspace_state_layout(&resolved.path, &resolved.alan_dir)?;
        let alan_dir = if resolved.alan_dir == self.default_workspace_dir {
            std::fs::create_dir_all(&self.default_workspace_dir)?;
            std::fs::canonicalize(&self.default_workspace_dir).with_context(|| {
                format!(
                    "Failed to canonicalize workspace state directory: {}",
                    self.default_workspace_dir.display()
                )
            })?
        } else {
            let workspace_path = self.ensure_workspace_root_exists(&resolved.path)?;
            let alan_dir = workspace_path.join(".alan");
            match std::fs::create_dir(&alan_dir) {
                Ok(()) => {}
                Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
                Err(err) => {
                    return Err(err).with_context(|| {
                        format!(
                            "Failed to create workspace state directory: {}",
                            alan_dir.display()
                        )
                    });
                }
            }
            let alan_dir = std::fs::canonicalize(&alan_dir).with_context(|| {
                format!(
                    "Failed to canonicalize workspace state directory: {}",
                    alan_dir.display()
                )
            })?;
            self.ensure_workspace_state_layout(&workspace_path, &alan_dir)?;
            alan_dir
        };

        let agent_dir = alan_dir.join("agent");
        let skills_dir = agent_dir.join("skills");
        let sessions_dir = alan_dir.join("sessions");
        let memory_dir = alan_dir.join("memory");
        let persona_dir = agent_dir.join("persona");

        std::fs::create_dir_all(&skills_dir)?;
        std::fs::create_dir_all(&sessions_dir)?;
        std::fs::create_dir_all(&memory_dir)?;
        std::fs::create_dir_all(&persona_dir)?;

        // Create an empty MEMORY.md if it does not exist.
        let memory_file = memory_dir.join("MEMORY.md");
        if !memory_file.exists() {
            std::fs::write(memory_file, "# Memory\n")?;
        }
        alan_runtime::prompts::ensure_workspace_bootstrap_files_at(&persona_dir)?;

        debug!(path = %alan_dir.display(), "Created workspace directory structure");
        Ok(())
    }

    fn ensure_workspace_root_exists(&self, workspace_path: &Path) -> Result<PathBuf> {
        let workspace_path = Self::normalize_creation_path(workspace_path);
        if workspace_path.exists() {
            return std::fs::canonicalize(&workspace_path).with_context(|| {
                format!(
                    "Failed to canonicalize workspace: {}",
                    workspace_path.display()
                )
            });
        }

        let (existing_ancestor, missing_components) =
            Self::split_existing_workspace_ancestor(&workspace_path)?;
        let mut current = std::fs::canonicalize(&existing_ancestor).with_context(|| {
            format!(
                "Failed to canonicalize workspace ancestor: {}",
                existing_ancestor.display()
            )
        })?;

        for component in missing_components {
            current.push(&component);
            match std::fs::create_dir(&current) {
                Ok(()) => {}
                Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
                Err(err) => {
                    return Err(err).with_context(|| {
                        format!(
                            "Failed to create workspace directory: {}",
                            current.display()
                        )
                    });
                }
            }
        }

        Ok(current)
    }

    fn split_existing_workspace_ancestor(
        workspace_path: &Path,
    ) -> Result<(PathBuf, Vec<OsString>)> {
        let mut current = workspace_path;
        let mut missing_components = Vec::new();

        while !current.exists() {
            let component = current.file_name().with_context(|| {
                format!(
                    "Workspace path must have an existing parent: {}",
                    workspace_path.display()
                )
            })?;
            Self::ensure_single_normal_component(component)?;
            missing_components.push(component.to_os_string());
            current = current.parent().with_context(|| {
                format!(
                    "Workspace path must have an existing parent: {}",
                    workspace_path.display()
                )
            })?;
        }

        missing_components.reverse();
        Ok((current.to_path_buf(), missing_components))
    }

    fn ensure_single_normal_component(component: &OsStr) -> Result<()> {
        let mut components = Path::new(component).components();
        ensure!(
            matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none(),
            "Workspace path component must be a single normal component: {}",
            Path::new(component).display()
        );
        Ok(())
    }

    fn normalize_creation_path(path: &Path) -> PathBuf {
        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
                Component::RootDir => normalized.push(component.as_os_str()),
                Component::CurDir => {}
                Component::ParentDir => {
                    normalized.pop();
                }
                Component::Normal(part) => normalized.push(part),
            }
        }
        normalized
    }

    fn ensure_workspace_state_layout(&self, workspace_path: &Path, alan_dir: &Path) -> Result<()> {
        if alan_dir == self.default_workspace_dir {
            ensure!(
                workspace_path == self.default_workspace_dir,
                "Default workspace state directory must resolve to {}",
                self.default_workspace_dir.display()
            );
            return Ok(());
        }

        ensure!(
            alan_dir.file_name() == Some(OsStr::new(".alan")),
            "Workspace state directory must end with .alan: {}",
            alan_dir.display()
        );
        ensure!(
            alan_dir.starts_with(workspace_path),
            "Workspace state directory must stay within workspace root: {}",
            alan_dir.display()
        );
        Ok(())
    }

    fn normalize_workspace_path_and_alan_dir(&self, canonical: &Path) -> (PathBuf, PathBuf) {
        let is_explicit_alan_dir = canonical
            .file_name()
            .map(|name| name == std::ffi::OsStr::new(".alan"))
            .unwrap_or(false);
        if is_explicit_alan_dir
            && canonical != self.default_workspace_dir
            && let Some(parent) = canonical.parent()
        {
            return (parent.to_path_buf(), canonical.to_path_buf());
        }

        (canonical.to_path_buf(), self.workspace_alan_dir(canonical))
    }

    /// Refresh the registry (if modified externally)
    #[allow(dead_code)]
    pub fn refresh_registry(&mut self) -> Result<()> {
        self.registry = WorkspaceRegistry::load()?;
        Ok(())
    }

    /// List all registered workspaces
    #[allow(dead_code)]
    pub fn list_registered(&self) -> &[crate::registry::WorkspaceEntry] {
        self.registry.list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_registry() -> (WorkspaceRegistry, TempDir, String) {
        let temp = TempDir::new().unwrap();
        let workspace_dir = temp.path().join("test-workspace");
        std::fs::create_dir_all(&workspace_dir).unwrap();
        std::fs::create_dir_all(workspace_dir.join(".alan")).unwrap();

        let id = generate_workspace_id(&workspace_dir);
        let entry = crate::registry::WorkspaceEntry {
            id: id.clone(),
            path: workspace_dir.clone(),
            alias: "test-alias".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let registry = WorkspaceRegistry {
            version: 1,
            workspaces: vec![entry],
        };

        (registry, temp, id)
    }

    #[test]
    fn test_resolve_by_alias() {
        let (registry, temp, expected_id) = create_test_registry();
        let default_dir = temp.path().join("default");
        let resolver = WorkspaceResolver::with_registry(registry, default_dir);

        let resolved = resolver.resolve(Some("test-alias")).unwrap();
        assert_eq!(resolved.id, expected_id);
        assert!(resolved.registered);
        assert_eq!(resolved.alias, Some("test-alias".to_string()));
        assert_eq!(resolved.alan_dir, resolved.path.join(".alan"));
    }

    #[test]
    fn test_resolve_by_short_id() {
        let (registry, temp, id) = create_test_registry();
        let default_dir = temp.path().join("default");
        let resolver = WorkspaceResolver::with_registry(registry, default_dir);

        let resolved = resolver.resolve(Some(&id)).unwrap();
        assert_eq!(resolved.id, id);
        assert!(resolved.registered);
    }

    #[test]
    fn test_resolve_by_path() {
        let (registry, temp, expected_id) = create_test_registry();
        let workspace_path = temp.path().join("test-workspace");
        let default_dir = temp.path().join("default");
        let resolver = WorkspaceResolver::with_registry(registry, default_dir);

        let resolved = resolver
            .resolve(Some(workspace_path.to_str().unwrap()))
            .unwrap();
        assert_eq!(resolved.id, expected_id);
    }

    #[test]
    fn test_resolve_unregistered_path() {
        let temp = TempDir::new().unwrap();
        let unregistered = temp.path().join("unregistered");
        std::fs::create_dir_all(&unregistered).unwrap();
        std::fs::create_dir_all(unregistered.join(".alan")).unwrap();

        let registry = WorkspaceRegistry {
            version: 1,
            workspaces: vec![],
        };

        let resolver = WorkspaceResolver::with_registry(registry, temp.path().join("default"));
        let resolved = resolver
            .resolve(Some(unregistered.to_str().unwrap()))
            .unwrap();

        assert!(!resolved.registered);
        assert_eq!(resolved.alias, None);
        assert!(resolver.is_valid_workspace(&resolved.path));
    }

    #[test]
    fn test_resolve_default() {
        let temp = TempDir::new().unwrap();
        let registry = WorkspaceRegistry {
            version: 1,
            workspaces: vec![],
        };

        let default_dir = temp.path().join("default-workspace");
        let resolver = WorkspaceResolver::with_registry(registry, default_dir.clone());

        let resolved = resolver.resolve(None).unwrap();
        assert_eq!(resolved.path, default_dir);
        assert_eq!(resolved.alan_dir, default_dir);
        assert_eq!(resolved.alias, Some("default".to_string()));
    }

    #[test]
    fn test_default_workspace_dir_not_nested_workspace() {
        let default = WorkspaceResolver::default_workspace_dir().unwrap();
        assert_eq!(
            default
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(""),
            ".alan"
        );
        assert!(
            !default.ends_with("workspace"),
            "default workspace dir should not be ~/.alan/workspace"
        );
    }

    #[test]
    fn test_resolve_or_create() {
        let temp = TempDir::new().unwrap();
        let new_workspace = temp.path().join("new-workspace");

        let registry = WorkspaceRegistry {
            version: 1,
            workspaces: vec![],
        };

        let resolver = WorkspaceResolver::with_registry(registry, temp.path().join("default"));

        // Should create directories when they do not exist.
        let resolved = resolver
            .resolve_or_create(Some(new_workspace.to_str().unwrap()))
            .unwrap();

        assert!(resolved.alan_dir.exists());
        assert!(resolved.alan_dir.join("sessions").exists());
        assert!(resolved.alan_dir.join("memory/MEMORY.md").exists());
        assert!(resolved.alan_dir.join("agent/persona/SOUL.md").exists());
    }

    #[test]
    fn test_resolve_or_create_rejects_state_dir_outside_workspace_root() {
        let temp = TempDir::new().unwrap();
        let registry = WorkspaceRegistry {
            version: 1,
            workspaces: vec![],
        };
        let resolver = WorkspaceResolver::with_registry(registry, temp.path().join("default"));
        let resolved = ResolvedWorkspace {
            id: "abc123".to_string(),
            path: temp.path().join("workspace"),
            alan_dir: temp.path().join("outside/.alan"),
            alias: None,
            registered: false,
        };

        let err = resolver.create_workspace_structure(&resolved).unwrap_err();

        assert!(
            err.to_string()
                .contains("Workspace state directory must stay within workspace root")
        );
    }

    #[test]
    fn test_is_valid_workspace() {
        let temp = TempDir::new().unwrap();
        let valid = temp.path().join("valid");
        let invalid = temp.path().join("invalid");

        std::fs::create_dir_all(valid.join(".alan")).unwrap();
        std::fs::create_dir_all(&invalid).unwrap();

        let registry = WorkspaceRegistry {
            version: 1,
            workspaces: vec![],
        };

        let resolver = WorkspaceResolver::with_registry(registry, temp.path().join("default"));

        assert!(resolver.is_valid_workspace(&valid));
        assert!(!resolver.is_valid_workspace(&invalid));
    }

    #[test]
    fn test_workspace_id_generation_consistency() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test-workspace");
        std::fs::create_dir_all(&path).unwrap();

        // Multiple generations should be stable.
        let id1 = generate_workspace_id(&path);
        let id2 = generate_workspace_id(&path);
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 6);

        // Different paths should produce different IDs.
        let other_path = temp.path().join("other-workspace");
        std::fs::create_dir_all(&other_path).unwrap();
        let other_id = generate_workspace_id(&other_path);
        assert_ne!(id1, other_id);
    }

    #[test]
    fn test_workspace_dir_helpers() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().join("workspace");

        let registry = WorkspaceRegistry {
            version: 1,
            workspaces: vec![],
        };

        let resolver = WorkspaceResolver::with_registry(registry, temp.path().join("default"));

        assert_eq!(
            resolver.workspace_sessions_dir(&workspace),
            workspace.join(".alan/sessions")
        );
        assert_eq!(
            resolver.workspace_memory_dir(&workspace),
            workspace.join(".alan/memory")
        );
        assert_eq!(
            resolver.workspace_persona_dir(&workspace),
            workspace.join(".alan/agent/persona")
        );
    }

    #[test]
    fn test_resolve_explicit_alan_path_uses_parent_as_workspace_root() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().join("workspace");
        let alan_dir = workspace.join(".alan");
        std::fs::create_dir_all(&alan_dir).unwrap();

        let registry = WorkspaceRegistry {
            version: 1,
            workspaces: vec![],
        };

        let resolver = WorkspaceResolver::with_registry(registry, temp.path().join("default"));
        let resolved = resolver.resolve(Some(alan_dir.to_str().unwrap())).unwrap();

        assert_eq!(
            std::fs::canonicalize(&resolved.path).unwrap(),
            std::fs::canonicalize(&workspace).unwrap()
        );
        assert_eq!(
            std::fs::canonicalize(&resolved.alan_dir).unwrap(),
            std::fs::canonicalize(&alan_dir).unwrap()
        );
    }

    #[test]
    fn test_resolve_registry_entry_with_alan_path_normalizes_to_parent_root() {
        let temp = TempDir::new().unwrap();
        let workspace = temp.path().join("workspace");
        let alan_dir = workspace.join(".alan");
        std::fs::create_dir_all(&alan_dir).unwrap();

        let entry = crate::registry::WorkspaceEntry {
            id: generate_workspace_id(&workspace),
            path: std::fs::canonicalize(&alan_dir).unwrap(),
            alias: "legacy-alan-path".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        let registry = WorkspaceRegistry {
            version: 1,
            workspaces: vec![entry],
        };

        let resolver = WorkspaceResolver::with_registry(registry, temp.path().join("default"));
        let resolved = resolver.resolve(Some("legacy-alan-path")).unwrap();

        assert_eq!(
            std::fs::canonicalize(&resolved.path).unwrap(),
            std::fs::canonicalize(&workspace).unwrap()
        );
        assert_eq!(
            std::fs::canonicalize(&resolved.alan_dir).unwrap(),
            std::fs::canonicalize(&alan_dir).unwrap()
        );
    }

    #[test]
    fn test_resolve_or_create_normalizes_nonexistent_parent_segments() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("nested").join("..").join("workspace");

        let registry = WorkspaceRegistry {
            version: 1,
            workspaces: vec![],
        };

        let resolver = WorkspaceResolver::with_registry(registry, temp.path().join("default"));
        let resolved = resolver
            .resolve_or_create(Some(target.to_str().unwrap()))
            .unwrap();

        assert_eq!(
            std::fs::canonicalize(&resolved.path).unwrap(),
            std::fs::canonicalize(temp.path().join("workspace")).unwrap()
        );
        assert!(resolved.alan_dir.join("sessions").exists());
    }
}
