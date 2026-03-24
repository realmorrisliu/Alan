use crate::skills::BUILTIN_PACKAGE_ASSETS;
use crate::skills::loader;
use crate::skills::types::{
    CapabilityPackage, PackageMount, PackageMountMode, PortableSkill, ResolvedCapabilityView,
    ScopedPackageDir, SkillContentSource, SkillScope,
};
use std::collections::HashMap;
use std::path::Path;

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
                view.packages.push(CapabilityPackage {
                    id: format!("skill:{}", skill.id),
                    scope: package_dir.scope,
                    root_dir: skill.path.parent().map(Path::to_path_buf),
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
        upsert_mounts(&mut self.mounts, overrides);
    }

    pub fn refresh(&self) -> Self {
        let mut refreshed = Self::from_package_dirs(self.package_dirs.clone());
        refreshed.mounts = self.mounts.clone();
        refreshed
    }
}

fn default_mounts_for_packages(packages: &[CapabilityPackage]) -> Vec<PackageMount> {
    let mut mounts = Vec::new();
    let mut index_by_package_id = HashMap::new();

    for package in packages {
        let mode = match package.scope {
            SkillScope::Repo | SkillScope::User => PackageMountMode::Discoverable,
            SkillScope::System => continue,
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

fn upsert_mounts(mounts: &mut Vec<PackageMount>, overrides: &[PackageMount]) {
    let mut index_by_package_id: HashMap<String, usize> = mounts
        .iter()
        .enumerate()
        .map(|(index, mount)| (mount.package_id.clone(), index))
        .collect();

    for override_mount in overrides {
        if let Some(index) = index_by_package_id.get(&override_mount.package_id).copied() {
            mounts[index] = override_mount.clone();
        } else {
            index_by_package_id.insert(override_mount.package_id.clone(), mounts.len());
            mounts.push(override_mount.clone());
        }
    }
}

fn builtin_capability_packages() -> Vec<CapabilityPackage> {
    BUILTIN_PACKAGE_ASSETS
        .iter()
        .map(|asset| CapabilityPackage {
            id: asset.package_id.to_string(),
            scope: SkillScope::System,
            root_dir: None,
            portable_skills: vec![PortableSkill {
                path: builtin_skill_path(asset.skill_label),
                source: SkillContentSource::Embedded(asset.content),
            }],
        })
        .collect()
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
}
