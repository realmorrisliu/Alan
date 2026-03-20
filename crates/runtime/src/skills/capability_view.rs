use crate::skills::loader;
use crate::skills::types::{
    CapabilityPackage, PackageMount, PortableSkill, ResolvedCapabilityView, ScopedPackageDir,
    SkillContentSource, SkillScope,
};
use crate::skills::{MEMORY_SKILL_MD, PLAN_SKILL_MD, WORKSPACE_MANAGER_SKILL_MD};
use std::path::Path;

impl ResolvedCapabilityView {
    pub fn from_package_dirs(package_dirs: Vec<ScopedPackageDir>) -> Self {
        let mut view = Self {
            package_dirs,
            ..Self::default()
        };

        for package in builtin_capability_packages() {
            view.mounts.push(PackageMount {
                package_id: package.id.clone(),
            });
            view.packages.push(package);
        }

        let package_dirs = view.package_dirs.clone();
        for package_dir in package_dirs {
            let outcome = loader::scan_skills_dir(&package_dir.path, package_dir.scope);
            view.errors.extend(outcome.errors);
            view.tracked_paths.extend(outcome.tracked_paths);

            for skill in outcome.skills {
                let package_id = format!("skill:{}", skill.id);
                view.mounts.push(PackageMount {
                    package_id: package_id.clone(),
                });
                view.packages.push(CapabilityPackage {
                    id: package_id,
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

    pub fn refresh(&self) -> Self {
        Self::from_package_dirs(self.package_dirs.clone())
    }
}

fn builtin_capability_packages() -> Vec<CapabilityPackage> {
    builtin_single_skill_packages()
        .into_iter()
        .map(|(package_id, label, content)| CapabilityPackage {
            id: package_id.to_string(),
            scope: SkillScope::System,
            root_dir: None,
            portable_skills: vec![PortableSkill {
                path: builtin_skill_path(label),
                source: SkillContentSource::Embedded(content),
            }],
        })
        .collect()
}

fn builtin_single_skill_packages() -> [(&'static str, &'static str, &'static str); 3] {
    [
        ("builtin:alan-memory", "memory", MEMORY_SKILL_MD),
        ("builtin:alan-plan", "plan", PLAN_SKILL_MD),
        (
            "builtin:alan-workspace-manager",
            "workspace-manager",
            WORKSPACE_MANAGER_SKILL_MD,
        ),
    ]
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
        let view = ResolvedCapabilityView::from_package_dirs(Vec::new());
        let package_ids: Vec<_> = view
            .packages
            .iter()
            .map(|package| package.id.as_str())
            .collect();

        assert!(package_ids.contains(&"builtin:alan-memory"));
        assert!(package_ids.contains(&"builtin:alan-plan"));
        assert!(package_ids.contains(&"builtin:alan-workspace-manager"));
        assert_eq!(view.mounts.len(), 3);
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
        }]);

        assert!(
            view.packages
                .iter()
                .any(|package| package.id == "skill:test-skill")
        );
        assert!(
            view.mounts
                .iter()
                .any(|mount| mount.package_id == "skill:test-skill")
        );
    }
}
