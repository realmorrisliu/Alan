//! Skills registry for managing discovered skills.

use crate::skills::loader;
use crate::skills::types::*;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, warn};

/// Registry of discovered skills.
#[derive(Clone, Default)]
pub struct SkillsRegistry {
    /// Skills indexed by ID.
    skills: HashMap<SkillId, SkillMetadata>,
    /// Non-fatal errors encountered during loading.
    errors: Vec<SkillError>,
    /// Filesystem paths whose metadata determines whether the registry is stale.
    tracked_paths: Vec<PathBuf>,
}

impl SkillsRegistry {
    pub fn load_capability_view(
        capability_view: &ResolvedCapabilityView,
    ) -> Result<Self, SkillsError> {
        let mut registry = Self::default();
        registry.reload_capability_view(capability_view);
        Ok(registry)
    }

    #[cfg(test)]
    pub(crate) fn load_package_dirs(
        package_dirs: &[ScopedPackageDir],
    ) -> Result<Self, SkillsError> {
        let mut capability_view =
            ResolvedCapabilityView::from_package_dirs(package_dirs.to_vec()).with_default_mounts();
        capability_view.apply_mount_overrides(&crate::skills::default_builtin_package_mounts());
        Self::load_capability_view(&capability_view)
    }

    /// Get a skill's metadata by ID.
    pub fn get(&self, id: &SkillId) -> Option<&SkillMetadata> {
        self.skills.get(id)
    }

    /// Load full skill content by ID.
    pub fn load_skill(&self, id: &SkillId) -> Result<Skill, SkillsError> {
        let metadata = self
            .skills
            .get(id)
            .ok_or_else(|| SkillsError::NotFound(id.clone()))?;

        let mut skill = match &metadata.source {
            SkillContentSource::File(path) => loader::load_skill(path, metadata.scope)?,
            SkillContentSource::Embedded(content) => loader::load_skill_from_content(
                content,
                &metadata.path,
                metadata.scope,
                metadata.source.clone(),
                metadata.package_id.clone(),
            )?,
        };
        skill.metadata.package_id = metadata.package_id.clone();
        skill.metadata.source = metadata.source.clone();
        skill.metadata.mount_mode = metadata.mount_mode;
        Ok(skill)
    }

    /// List all registered skills.
    pub fn list(&self) -> Vec<&SkillMetadata> {
        self.skills.values().collect()
    }

    /// List skill loading errors (if any).
    pub fn errors(&self) -> &[SkillError] {
        &self.errors
    }

    /// Return filesystem paths whose metadata determines whether the registry is stale.
    pub fn tracked_paths(&self) -> &[PathBuf] {
        &self.tracked_paths
    }

    /// List skills sorted by scope priority.
    pub fn list_sorted(&self) -> Vec<&SkillMetadata> {
        let mut skills: Vec<_> = self.skills.values().collect();
        skills.sort_by(|left, right| {
            left.scope
                .priority()
                .cmp(&right.scope.priority())
                .then_with(|| left.id.cmp(&right.id))
        });
        skills
    }

    /// Find skills matching a query (simple keyword matching).
    pub fn find_matches(&self, query: &str) -> Vec<&SkillMetadata> {
        let query_lower = query.to_lowercase();
        let keywords: Vec<_> = query_lower.split_whitespace().collect();

        self.skills
            .values()
            .filter(|skill| {
                let desc_lower = skill.description.to_lowercase();
                let name_lower = skill.name.to_lowercase();
                let tags_lower: Vec<String> =
                    skill.tags.iter().map(|tag| tag.to_lowercase()).collect();

                keywords.iter().any(|keyword| {
                    name_lower.contains(keyword)
                        || desc_lower.contains(keyword)
                        || tags_lower
                            .iter()
                            .any(|tag| tag.contains(keyword) || keyword.contains(tag))
                })
            })
            .collect()
    }

    /// Check if a skill exists.
    pub fn has(&self, id: &SkillId) -> bool {
        self.skills.contains_key(id)
    }

    /// Get the number of registered skills.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    fn reload_capability_view(&mut self, capability_view: &ResolvedCapabilityView) {
        self.skills.clear();
        self.errors.clear();
        self.tracked_paths.clear();
        self.apply_capability_view(capability_view.refresh());
    }

    fn apply_capability_view(&mut self, capability_view: ResolvedCapabilityView) {
        self.errors.extend(capability_view.errors);
        self.tracked_paths.extend(capability_view.tracked_paths);
        self.tracked_paths.sort();
        self.tracked_paths.dedup();

        let mount_modes: HashMap<String, PackageMountMode> = capability_view
            .mounts
            .into_iter()
            .map(|mount| (mount.package_id, mount.mode))
            .collect();

        for package in capability_view.packages {
            let Some(mount_mode) = mount_modes.get(&package.id).copied() else {
                continue;
            };
            if !mount_mode.exposes_skills() {
                continue;
            }

            for portable_skill in package.portable_skills {
                match &portable_skill.source {
                    SkillContentSource::File(path) => {
                        match loader::load_skill_metadata(path, package.scope) {
                            Ok(mut metadata) => {
                                metadata.package_id = Some(package.id.clone());
                                metadata.source = portable_skill.source.clone();
                                metadata.mount_mode = mount_mode;
                                debug!(
                                    "Registering skill: {} (package: {}, scope: {:?}, mount_mode: {:?}, path: {})",
                                    metadata.id,
                                    package.id,
                                    package.scope,
                                    mount_mode,
                                    metadata.path.display()
                                );
                                self.skills.insert(metadata.id.clone(), metadata);
                            }
                            Err(err) => {
                                warn!(
                                    path = %path.display(),
                                    package_id = %package.id,
                                    error = %err,
                                    "Failed to load portable skill metadata"
                                );
                                self.errors.push(SkillError {
                                    path: path.to_path_buf(),
                                    message: err.to_string(),
                                });
                            }
                        }
                    }
                    SkillContentSource::Embedded(content) => {
                        match loader::parse_skill_metadata_with_source(
                            content,
                            &portable_skill.path,
                            package.scope,
                            portable_skill.source.clone(),
                            Some(package.id.clone()),
                        ) {
                            Ok(mut metadata) => {
                                metadata.mount_mode = mount_mode;
                                debug!(
                                    "Registering skill: {} (package: {}, scope: {:?}, mount_mode: {:?}, path: {})",
                                    metadata.id,
                                    package.id,
                                    package.scope,
                                    mount_mode,
                                    metadata.path.display()
                                );
                                self.skills.insert(metadata.id.clone(), metadata);
                            }
                            Err(err) => {
                                warn!(
                                    path = %portable_skill.path.display(),
                                    package_id = %package.id,
                                    error = %err,
                                    "Failed to parse embedded portable skill metadata"
                                );
                                self.errors.push(SkillError {
                                    path: portable_skill.path.clone(),
                                    message: err.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_skill(dir: &std::path::Path, name: &str, skill_name: &str, description: &str) {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(
            file,
            r#"---
name: {}
description: {}
---

Body
"#,
            skill_name, description
        )
        .unwrap();
    }

    #[test]
    fn load_package_dirs_registers_discovered_skill() {
        let temp = TempDir::new().unwrap();
        let repo_skills = temp.path().join("skills");
        create_test_skill(&repo_skills, "repo-skill", "Repo Skill", "From repo");

        let registry = SkillsRegistry::load_package_dirs(&[ScopedPackageDir {
            path: repo_skills,
            scope: SkillScope::Repo,
        }])
        .unwrap();

        assert!(registry.has(&"repo-skill".to_string()));
        assert_eq!(
            registry.get(&"repo-skill".to_string()).unwrap().scope,
            SkillScope::Repo
        );
    }

    #[test]
    fn load_capability_view_prefers_later_entries_for_same_skill_id() {
        let temp = TempDir::new().unwrap();
        let global_dir = temp.path().join("global");
        let workspace_dir = temp.path().join("workspace");

        create_test_skill(&global_dir, "shared-skill", "Shared Skill", "From global");
        create_test_skill(
            &workspace_dir,
            "shared-skill",
            "Shared Skill",
            "From workspace",
        );

        let capability_view = ResolvedCapabilityView::from_package_dirs(vec![
            ScopedPackageDir {
                path: global_dir,
                scope: SkillScope::User,
            },
            ScopedPackageDir {
                path: workspace_dir,
                scope: SkillScope::Repo,
            },
        ])
        .with_default_mounts();

        let registry = SkillsRegistry::load_capability_view(&capability_view).unwrap();
        let skill = registry.get(&"shared-skill".to_string()).unwrap();

        assert_eq!(skill.description, "From workspace");
        assert_eq!(skill.scope, SkillScope::Repo);
    }

    #[test]
    fn load_capability_view_respects_mount_modes() {
        let mut capability_view =
            ResolvedCapabilityView::from_package_dirs(Vec::new()).with_default_mounts();
        capability_view.apply_mount_overrides(&crate::skills::default_builtin_package_mounts());
        capability_view.apply_mount_overrides(&[
            PackageMount {
                package_id: "builtin:alan-memory".to_string(),
                mode: PackageMountMode::ExplicitOnly,
            },
            PackageMount {
                package_id: "builtin:alan-plan".to_string(),
                mode: PackageMountMode::Internal,
            },
        ]);

        let registry = SkillsRegistry::load_capability_view(&capability_view).unwrap();
        let memory = registry.get(&"memory".to_string()).unwrap();

        assert_eq!(memory.mount_mode, PackageMountMode::ExplicitOnly);
        assert!(registry.get(&"plan".to_string()).is_none());
        assert!(registry.get(&"workspace-manager".to_string()).is_some());
    }

    #[test]
    fn find_matches_uses_name_description_and_tags() {
        let temp = TempDir::new().unwrap();
        let repo_skills = temp.path().join("skills");
        create_test_skill(
            &repo_skills,
            "test-skill",
            "Test Skill",
            "A skill for testing purposes",
        );

        let registry = SkillsRegistry::load_package_dirs(&[ScopedPackageDir {
            path: repo_skills,
            scope: SkillScope::Repo,
        }])
        .unwrap();

        let matches = registry.find_matches("test");
        assert!(!matches.is_empty(), "Should find at least one match");
    }

    #[test]
    fn list_sorted_is_stable_within_scope() {
        let temp = TempDir::new().unwrap();
        let repo_skills = temp.path().join("skills");
        create_test_skill(&repo_skills, "b-skill", "B Skill", "B");
        create_test_skill(&repo_skills, "a-skill", "A Skill", "A");

        let registry = SkillsRegistry::load_package_dirs(&[ScopedPackageDir {
            path: repo_skills,
            scope: SkillScope::Repo,
        }])
        .unwrap();
        let ids: Vec<_> = registry
            .list_sorted()
            .into_iter()
            .filter(|skill| skill.scope == SkillScope::Repo)
            .map(|skill| skill.id.clone())
            .collect();

        assert_eq!(ids, vec!["a-skill".to_string(), "b-skill".to_string()]);
    }
}
