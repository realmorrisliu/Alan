use crate::skills::loader;
use crate::skills::types::{
    CapabilityPackage, CapabilityPackageExports, CapabilityPackageResources, PackageMount,
    PackageMountMode, PortableSkill, ResolvedCapabilityView, ScopedPackageDir, SkillContentSource,
    SkillScope,
};
use crate::skills::{BUILTIN_PACKAGE_ASSETS, merge_package_mounts};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

impl ResolvedCapabilityView {
    pub fn from_package_dirs(package_dirs: Vec<ScopedPackageDir>) -> Self {
        let mut view = Self {
            package_dirs,
            ..Self::default()
        };

        view.packages.extend(builtin_capability_packages());

        let package_dirs = view.package_dirs.clone();
        for package_dir in package_dirs {
            let outcome = loader::scan_skills_dir(&package_dir.path, package_dir.scope);
            view.errors.extend(outcome.errors);
            view.tracked_paths.extend(outcome.tracked_paths);

            for skill in outcome.skills {
                let root_dir = skill.path.parent().map(Path::to_path_buf);
                view.packages.push(CapabilityPackage {
                    id: format!("skill:{}", skill.id),
                    scope: package_dir.scope,
                    exports: package_exports_for_root_dir(root_dir.as_deref()),
                    root_dir,
                    portable_skills: vec![PortableSkill {
                        path: skill.path.clone(),
                        source: SkillContentSource::File(skill.path),
                    }],
                });
            }
        }

        view.tracked_paths.sort();
        view.tracked_paths.dedup();
        view
    }

    pub fn with_default_mounts(mut self) -> Self {
        self.mounts = default_mounts_for_packages(&self.packages);
        self
    }

    pub fn apply_mount_overrides(&mut self, overrides: &[PackageMount]) {
        self.mounts = merge_package_mounts(&self.mounts, overrides);
    }

    pub fn refresh(&self) -> Self {
        let mut refreshed =
            Self::from_package_dirs(self.package_dirs.clone()).with_default_mounts();
        refreshed.mounts = merge_package_mounts(&refreshed.mounts, &self.mounts);
        refreshed
    }
}

fn default_mounts_for_packages(packages: &[CapabilityPackage]) -> Vec<PackageMount> {
    let mut mounts = Vec::new();
    let mut index_by_package_id = HashMap::new();

    for package in packages {
        let mode = match package.scope {
            SkillScope::Repo | SkillScope::User => PackageMountMode::Discoverable,
            SkillScope::Builtin => continue,
        };
        let mount = PackageMount {
            package_id: package.id.clone(),
            mode,
        };

        if let Some(index) = index_by_package_id.get(&mount.package_id).copied() {
            mounts[index] = mount;
        } else {
            index_by_package_id.insert(mount.package_id.clone(), mounts.len());
            mounts.push(mount);
        }
    }

    mounts
}

fn builtin_capability_packages() -> Vec<CapabilityPackage> {
    BUILTIN_PACKAGE_ASSETS
        .iter()
        .map(|asset| CapabilityPackage {
            id: asset.package_id.to_string(),
            scope: SkillScope::Builtin,
            exports: CapabilityPackageExports::default(),
            root_dir: None,
            portable_skills: vec![PortableSkill {
                path: builtin_skill_path(asset.skill_label),
                source: SkillContentSource::Embedded(asset.content),
            }],
        })
        .collect()
}

fn package_exports_for_root_dir(root_dir: Option<&Path>) -> CapabilityPackageExports {
    let Some(root_dir) = root_dir else {
        return CapabilityPackageExports::default();
    };

    CapabilityPackageExports {
        child_agent_roots: child_agent_roots(root_dir),
        resources: CapabilityPackageResources {
            scripts_dir: existing_dir(root_dir.join("scripts")),
            references_dir: existing_dir(root_dir.join("references")),
            assets_dir: existing_dir(root_dir.join("assets")),
            viewers_dir: existing_dir(root_dir.join("viewers")),
        },
    }
}

fn child_agent_roots(root_dir: &Path) -> Vec<PathBuf> {
    let agents_dir = root_dir.join("agents");
    let Ok(entries) = std::fs::read_dir(&agents_dir) else {
        return Vec::new();
    };

    let mut roots: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    roots.sort();
    roots
}

fn existing_dir(path: PathBuf) -> Option<PathBuf> {
    path.is_dir().then_some(path)
}

fn builtin_skill_path(label: &str) -> std::path::PathBuf {
    format!("<builtin>/{label}/SKILL.md").into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn resolved_capability_view_includes_builtin_packages() {
        let view = ResolvedCapabilityView::from_package_dirs(Vec::new()).with_default_mounts();
        let package_ids: Vec<_> = view
            .packages
            .iter()
            .map(|package| package.id.as_str())
            .collect();

        assert!(package_ids.contains(&"builtin:alan-memory"));
        assert!(package_ids.contains(&"builtin:alan-plan"));
        assert!(package_ids.contains(&"builtin:alan-workspace-manager"));
        assert!(
            !view
                .mounts
                .iter()
                .any(|mount| mount.package_id == "builtin:alan-plan")
        );
    }

    #[test]
    fn resolved_capability_view_discovers_single_skill_packages_from_overlay_dirs() {
        let temp = TempDir::new().unwrap();
        let skills_dir = temp.path().join("skills");
        let skill_dir = skills_dir.join("test-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(
            file,
            r#"---
name: Test Skill
description: A test skill
---

Body
"#
        )
        .unwrap();

        let view = ResolvedCapabilityView::from_package_dirs(vec![ScopedPackageDir {
            path: skills_dir,
            scope: SkillScope::Repo,
        }])
        .with_default_mounts();

        assert!(
            view.packages
                .iter()
                .any(|package| package.id == "skill:test-skill")
        );
        assert!(view.mounts.iter().any(|mount| {
            mount.package_id == "skill:test-skill" && mount.mode == PackageMountMode::Discoverable
        }));
    }

    #[test]
    fn resolved_capability_view_discovers_package_exports_from_skill_root() {
        let temp = TempDir::new().unwrap();
        let skills_dir = temp.path().join("skills");
        let skill_dir = skills_dir.join("test-skill");
        std::fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        std::fs::create_dir_all(skill_dir.join("references")).unwrap();
        std::fs::create_dir_all(skill_dir.join("assets")).unwrap();
        std::fs::create_dir_all(skill_dir.join("viewers")).unwrap();
        std::fs::create_dir_all(skill_dir.join("agents/reviewer")).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: Test Skill
description: A test skill
---

Body
"#,
        )
        .unwrap();

        let view = ResolvedCapabilityView::from_package_dirs(vec![ScopedPackageDir {
            path: skills_dir,
            scope: SkillScope::Repo,
        }])
        .with_default_mounts();
        let package = view
            .packages
            .iter()
            .find(|package| package.id == "skill:test-skill")
            .unwrap();
        let canonical_skill_dir = std::fs::canonicalize(&skill_dir).unwrap();

        assert_eq!(package.exports.child_agent_roots.len(), 1);
        assert_eq!(
            package
                .exports
                .resources
                .scripts_dir
                .as_deref()
                .and_then(|path| std::fs::canonicalize(path).ok())
                .as_deref(),
            Some(canonical_skill_dir.join("scripts").as_path())
        );
        assert_eq!(
            package
                .exports
                .resources
                .references_dir
                .as_deref()
                .and_then(|path| std::fs::canonicalize(path).ok())
                .as_deref(),
            Some(canonical_skill_dir.join("references").as_path())
        );
        assert_eq!(
            package
                .exports
                .resources
                .assets_dir
                .as_deref()
                .and_then(|path| std::fs::canonicalize(path).ok())
                .as_deref(),
            Some(canonical_skill_dir.join("assets").as_path())
        );
        assert_eq!(
            package
                .exports
                .resources
                .viewers_dir
                .as_deref()
                .and_then(|path| std::fs::canonicalize(path).ok())
                .as_deref(),
            Some(canonical_skill_dir.join("viewers").as_path())
        );
    }

    #[test]
    fn resolved_capability_view_does_not_register_child_agent_skills_in_parent_scan() {
        let temp = TempDir::new().unwrap();
        let skills_dir = temp.path().join("skills");
        let skill_dir = skills_dir.join("parent-skill");
        std::fs::create_dir_all(skill_dir.join("agents/reviewer/skills/child-only")).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: Parent Skill
description: Parent-visible skill
---

Body
"#,
        )
        .unwrap();
        std::fs::write(
            skill_dir.join("agents/reviewer/skills/child-only/SKILL.md"),
            r#"---
name: Child Only
description: Child-agent-only skill
---

Body
"#,
        )
        .unwrap();

        let view = ResolvedCapabilityView::from_package_dirs(vec![ScopedPackageDir {
            path: skills_dir,
            scope: SkillScope::Repo,
        }])
        .with_default_mounts();
        let package_ids: Vec<_> = view
            .packages
            .iter()
            .map(|package| package.id.as_str())
            .collect();
        let parent_package = view
            .packages
            .iter()
            .find(|package| package.id == "skill:parent-skill")
            .unwrap();

        assert!(package_ids.contains(&"skill:parent-skill"));
        assert!(!package_ids.contains(&"skill:child-only"));
        assert_eq!(parent_package.exports.child_agent_roots.len(), 1);
    }

    #[test]
    fn refresh_preserves_explicit_mount_overrides() {
        let mut view = ResolvedCapabilityView::from_package_dirs(Vec::new()).with_default_mounts();
        view.apply_mount_overrides(&[PackageMount {
            package_id: "builtin:alan-plan".to_string(),
            mode: PackageMountMode::ExplicitOnly,
        }]);

        let refreshed = view.refresh();
        let mount = refreshed
            .mounts
            .iter()
            .find(|mount| mount.package_id == "builtin:alan-plan")
            .unwrap();

        assert_eq!(mount.mode, PackageMountMode::ExplicitOnly);
    }

    #[test]
    fn refresh_adds_default_mounts_for_newly_discovered_packages() {
        let temp = TempDir::new().unwrap();
        let skills_dir = temp.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let mut view = ResolvedCapabilityView::from_package_dirs(vec![ScopedPackageDir {
            path: skills_dir.clone(),
            scope: SkillScope::Repo,
        }])
        .with_default_mounts();
        view.apply_mount_overrides(&[PackageMount {
            package_id: "builtin:alan-plan".to_string(),
            mode: PackageMountMode::ExplicitOnly,
        }]);

        let skill_dir = skills_dir.join("new-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            r#"---
name: New Skill
description: Freshly added
---

Body
"#,
        )
        .unwrap();

        let refreshed = view.refresh();

        assert!(refreshed.mounts.iter().any(|mount| {
            mount.package_id == "skill:new-skill" && mount.mode == PackageMountMode::Discoverable
        }));
        assert!(refreshed.mounts.iter().any(|mount| {
            mount.package_id == "builtin:alan-plan" && mount.mode == PackageMountMode::ExplicitOnly
        }));
    }
}
