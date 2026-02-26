//! Workspace Resolver — 路径解析层，统一处理 workspace 标识到路径的映射。
//!
//! 解析优先级：
//! 1. CLI Registry 中的 alias
//! 2. CLI Registry 中的短 ID (6位)
//! 3. 直接作为路径解析（如果有效）
//! 4. 默认 workspace 路径

use crate::registry::{WorkspaceRegistry, generate_workspace_id};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Workspace 解析结果
#[derive(Debug, Clone)]
pub struct ResolvedWorkspace {
    /// Workspace ID (短哈希，6位)
    #[allow(dead_code)]
    pub id: String,
    /// 规范化后的绝对路径
    pub path: PathBuf,
    /// 可选的别名
    #[allow(dead_code)]
    pub alias: Option<String>,
    /// 是否已注册在 registry 中
    #[allow(dead_code)]
    pub registered: bool,
}

/// Workspace 路径解析器
#[derive(Debug, Clone)]
pub struct WorkspaceResolver {
    registry: WorkspaceRegistry,
    default_workspace_dir: PathBuf,
}

impl WorkspaceResolver {
    /// 创建新的解析器，加载 CLI Registry
    pub fn new() -> Result<Self> {
        let registry = WorkspaceRegistry::load()?;
        let default_workspace_dir = Self::default_workspace_dir()?;

        Ok(Self {
            registry,
            default_workspace_dir,
        })
    }

    /// 使用指定的 registry 创建（用于测试）
    #[cfg(test)]
    pub fn with_registry(registry: WorkspaceRegistry, default_dir: PathBuf) -> Self {
        Self {
            registry,
            default_workspace_dir: default_dir,
        }
    }

    /// 获取默认 workspace 目录 (~/.alan/workspace/)
    fn default_workspace_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot determine home directory")?;
        Ok(home.join(".alan").join("workspace"))
    }

    /// 解析 workspace 标识符到路径
    ///
    /// 支持的标识符格式：
    /// - Registry alias (如 "my-project")
    /// - 短 ID (如 "a1b2c3")
    /// - 绝对路径 (如 "/home/user/projects/myapp")
    /// - 相对路径 (相对于当前工作目录)
    /// - None (返回默认 workspace)
    pub fn resolve(&self, identifier: Option<&str>) -> Result<ResolvedWorkspace> {
        // None 表示使用默认 workspace
        if identifier.is_none() {
            return self.default_workspace();
        }
        let identifier = identifier.unwrap();

        // 1. 尝试从 Registry 解析 (alias 或短 ID)
        if let Some(entry) = self.registry.find(identifier) {
            debug!(%identifier, path = %entry.path.display(), "Resolved workspace from registry");
            return Ok(ResolvedWorkspace {
                id: entry.id.clone(),
                path: entry.path.clone(),
                alias: Some(entry.alias.clone()),
                registered: true,
            });
        }

        // 2. 尝试作为路径解析
        let path = Path::new(identifier);
        let canonical = Self::canonicalize_path(path)?;

        // 检查路径是否包含 .alan 目录（已初始化的 workspace）
        if !self.is_valid_workspace(&canonical) {
            warn!(path = %canonical.display(), "Path is not a valid workspace (no .alan directory)");
        }

        // 生成 ID（与 registry 相同的算法）
        let id = generate_workspace_id(&canonical);

        // 检查这个路径是否实际上在 registry 中（通过路径匹配）
        let registered = self.registry.find(&id).is_some();

        Ok(ResolvedWorkspace {
            id,
            path: canonical,
            alias: None,
            registered,
        })
    }

    /// 解析并确保 workspace 目录存在
    ///
    /// 如果路径未初始化（没有 .alan），会自动创建目录结构
    pub fn resolve_or_create(&self, identifier: Option<&str>) -> Result<ResolvedWorkspace> {
        let resolved = self.resolve(identifier)?;

        // 确保 .alan 目录存在
        if !resolved.path.join(".alan").exists() {
            debug!(path = %resolved.path.display(), "Creating workspace directory structure");
            Self::create_workspace_structure(&resolved.path)?;
        }

        Ok(resolved)
    }

    /// 获取默认 workspace
    pub fn default_workspace(&self) -> Result<ResolvedWorkspace> {
        let path = self.default_workspace_dir.clone();

        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }

        let id = generate_workspace_id(&path);

        Ok(ResolvedWorkspace {
            id,
            path,
            alias: Some("default".to_string()),
            registered: false,
        })
    }

    /// 获取 workspace 具体子目录 (例如 log, memory 等)
    #[allow(dead_code)]
    pub fn workspace_sessions_dir(&self, workspace_path: &Path) -> PathBuf {
        workspace_path.join(".alan").join("sessions")
    }

    /// 获取 workspace 的 memory 目录
    #[allow(dead_code)]
    pub fn workspace_memory_dir(&self, workspace_path: &Path) -> PathBuf {
        workspace_path.join(".alan").join("memory")
    }

    /// 获取 workspace 的 context 目录
    #[allow(dead_code)]
    pub fn workspace_context_dir(&self, workspace_path: &Path) -> PathBuf {
        workspace_path.join(".alan").join("context")
    }

    /// 检查路径是否为有效的 workspace（包含 .alan 目录）
    pub fn is_valid_workspace(&self, path: &Path) -> bool {
        path.join(".alan").is_dir()
    }

    /// 规范化路径
    fn canonicalize_path(path: &Path) -> Result<PathBuf> {
        if path.exists() {
            // 路径存在，规范化它
            std::fs::canonicalize(path)
                .with_context(|| format!("Failed to canonicalize path: {}", path.display()))
        } else {
            // 路径不存在，检查是否是相对路径
            if path.is_relative() {
                let cwd = std::env::current_dir()?;
                let absolute = cwd.join(path);
                if absolute.exists() {
                    std::fs::canonicalize(&absolute).with_context(|| {
                        format!("Failed to canonicalize path: {}", absolute.display())
                    })
                } else {
                    // 路径不存在，但返回绝对路径（可能后续会创建）
                    Ok(absolute)
                }
            } else {
                // 绝对路径但不存在
                Ok(path.to_path_buf())
            }
        }
    }

    /// 创建 workspace 目录结构
    fn create_workspace_structure(path: &Path) -> Result<()> {
        let alan_dir = path.join(".alan");
        std::fs::create_dir_all(alan_dir.join("context").join("skills"))?;
        std::fs::create_dir_all(alan_dir.join("sessions"))?;
        std::fs::create_dir_all(alan_dir.join("memory"))?;

        // 创建空的 MEMORY.md
        std::fs::write(alan_dir.join("memory").join("MEMORY.md"), "# Memory\n")?;

        debug!(path = %path.display(), "Created workspace directory structure");
        Ok(())
    }

    /// 刷新 registry (如果在外部被修改)
    #[allow(dead_code)]
    pub fn refresh_registry(&mut self) -> Result<()> {
        self.registry = WorkspaceRegistry::load()?;
        Ok(())
    }

    /// 列出所有已注册的 workspaces
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
        assert_eq!(resolved.alias, Some("default".to_string()));
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

        // 目录不存在时应该创建
        let resolved = resolver
            .resolve_or_create(Some(new_workspace.to_str().unwrap()))
            .unwrap();

        assert!(resolved.path.join(".alan").exists());
        assert!(resolved.path.join(".alan/sessions").exists());
        assert!(resolved.path.join(".alan/memory/MEMORY.md").exists());
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

        // 多次生成应该相同
        let id1 = generate_workspace_id(&path);
        let id2 = generate_workspace_id(&path);
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 6);

        // 不同路径不同 ID
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
            resolver.workspace_context_dir(&workspace),
            workspace.join(".alan/context")
        );
    }
}
