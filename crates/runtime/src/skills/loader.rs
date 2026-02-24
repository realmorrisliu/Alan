//! Skill loading from filesystem.

use crate::skills::types::*;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use tracing::{debug, error, warn};

/// Load skill metadata from a SKILL.md file.
pub fn load_skill_metadata(path: &Path, scope: SkillScope) -> Result<SkillMetadata, SkillsError> {
    let content = std::fs::read_to_string(path)?;
    parse_skill_metadata(&content, path, scope)
}

/// Parse skill metadata from content.
pub fn parse_skill_metadata(
    content: &str,
    path: &Path,
    scope: SkillScope,
) -> Result<SkillMetadata, SkillsError> {
    let (frontmatter_str, _) =
        extract_frontmatter(content).ok_or(SkillsError::MissingFrontmatter)?;

    let frontmatter: SkillFrontmatter = serde_yaml::from_str(&frontmatter_str)?;

    // Validate metadata fields
    validate_skill_metadata(
        &frontmatter.name,
        &frontmatter.description,
        frontmatter.metadata.short_description.as_deref(),
    )?;

    // Validate capabilities
    validate_capabilities(&frontmatter.capabilities)?;

    let id = name_to_id(&frontmatter.name);

    Ok(SkillMetadata {
        id,
        name: frontmatter.name,
        description: frontmatter.description,
        short_description: frontmatter.metadata.short_description,
        path: path.to_path_buf(),
        scope,
        tags: frontmatter.metadata.tags,
        capabilities: Some(frontmatter.capabilities),
    })
}

/// Load full skill content (metadata + body).
pub fn load_skill(path: &Path, scope: SkillScope) -> Result<Skill, SkillsError> {
    let content = std::fs::read_to_string(path)?;

    let (frontmatter_str, body) =
        extract_frontmatter(&content).ok_or(SkillsError::MissingFrontmatter)?;

    let frontmatter: SkillFrontmatter = serde_yaml::from_str(&frontmatter_str)?;

    // Validate metadata fields
    validate_skill_metadata(
        &frontmatter.name,
        &frontmatter.description,
        frontmatter.metadata.short_description.as_deref(),
    )?;

    // Validate capabilities
    validate_capabilities(&frontmatter.capabilities)?;

    let id = name_to_id(&frontmatter.name);

    let metadata = SkillMetadata {
        id,
        name: frontmatter.name.clone(),
        description: frontmatter.description.clone(),
        short_description: frontmatter.metadata.short_description.clone(),
        path: path.to_path_buf(),
        scope,
        tags: frontmatter.metadata.tags.clone(),
        capabilities: Some(frontmatter.capabilities.clone()),
    };

    Ok(Skill {
        metadata,
        content: body,
        frontmatter,
    })
}

const MAX_SCAN_DEPTH: usize = 6;
const MAX_SKILLS_DIRS_PER_ROOT: usize = 2000;

/// Scan a directory for skills (recursive).
pub fn scan_skills_dir(dir: &Path, scope: SkillScope) -> SkillLoadOutcome {
    let mut outcome = SkillLoadOutcome::default();

    if !dir.exists() {
        debug!("Skills directory does not exist: {}", dir.display());
        return outcome;
    }

    if !dir.is_dir() {
        warn!("Skills path is not a directory: {}", dir.display());
        return outcome;
    }

    let follow_symlinks = matches!(scope, SkillScope::Repo | SkillScope::User);
    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::from([(dir.to_path_buf(), 0)]);
    let mut visited_dirs: HashSet<PathBuf> = HashSet::new();
    let mut seen_skills: HashSet<PathBuf> = HashSet::new();
    visited_dirs.insert(dir.to_path_buf());
    let mut truncated = false;

    while let Some((current, depth)) = queue.pop_front() {
        if depth > MAX_SCAN_DEPTH {
            continue;
        }
        if visited_dirs.len() >= MAX_SKILLS_DIRS_PER_ROOT {
            truncated = true;
            break;
        }

        let entries = match std::fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(e) => {
                error!(
                    "Failed to read skills directory {}: {}",
                    current.display(),
                    e
                );
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => continue,
            };

            if file_name.starts_with('.') {
                continue;
            }

            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };

            if file_type.is_symlink() {
                if !follow_symlinks {
                    continue;
                }

                let metadata = match std::fs::metadata(&path) {
                    Ok(metadata) => metadata,
                    Err(e) => {
                        error!(
                            "Failed to stat skills entry {} (symlink): {}",
                            path.display(),
                            e
                        );
                        continue;
                    }
                };

                if metadata.is_dir() {
                    let resolved = match std::fs::canonicalize(&path) {
                        Ok(resolved) => resolved,
                        Err(_) => continue,
                    };
                    if visited_dirs.insert(resolved.clone()) {
                        queue.push_back((resolved, depth + 1));
                    }
                }
                continue;
            }

            if file_type.is_dir() {
                let resolved = match std::fs::canonicalize(&path) {
                    Ok(resolved) => resolved,
                    Err(_) => continue,
                };
                if visited_dirs.insert(resolved.clone()) {
                    queue.push_back((resolved, depth + 1));
                }
                continue;
            }

            if file_type.is_file() && file_name == "SKILL.md" {
                let resolved = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
                if !seen_skills.insert(resolved.clone()) {
                    continue;
                }

                match load_skill_metadata(&resolved, scope) {
                    Ok(metadata) => {
                        debug!("Loaded skill: {} from {}", metadata.id, resolved.display());
                        outcome.skills.push(metadata);
                    }
                    Err(e) => {
                        error!("Failed to load skill from {}: {}", resolved.display(), e);
                        outcome.errors.push(SkillError {
                            path: resolved.clone(),
                            message: e.to_string(),
                        });
                    }
                }
            }
        }
    }

    if truncated {
        warn!(
            "Skills scan truncated after {} directories (root: {})",
            MAX_SKILLS_DIRS_PER_ROOT,
            dir.display()
        );
    }

    outcome
}

/// Get the user skills directory.
pub fn user_skills_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".config/alan/skills"))
}

/// Get the repo skills directory for a given cwd.
pub fn repo_skills_dir(cwd: &Path) -> PathBuf {
    cwd.join(".alan/skills")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_load_skill_metadata() {
        let content = r#"---
name: Test Skill
description: A test skill for testing
metadata:
  short-description: Short desc
  tags:
    - test
    - demo
---

# Test Skill Body

This is the body content.
"#;

        let metadata =
            parse_skill_metadata(content, Path::new("/tmp/test/SKILL.md"), SkillScope::User)
                .unwrap();

        assert_eq!(metadata.id, "test-skill");
        assert_eq!(metadata.name, "Test Skill");
        assert_eq!(metadata.description, "A test skill for testing");
        assert_eq!(metadata.short_description, Some("Short desc".to_string()));
        assert_eq!(metadata.tags, vec!["test", "demo"]);
        assert_eq!(metadata.scope, SkillScope::User);
    }

    #[test]
    fn test_scan_skills_dir() {
        let temp = TempDir::new().unwrap();
        let skills_dir = temp.path().join("skills");

        // Create a valid skill
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

        // Create a directory without SKILL.md (should be ignored)
        std::fs::create_dir(skills_dir.join("invalid")).unwrap();

        // Create a hidden directory (should be ignored)
        std::fs::create_dir(skills_dir.join(".hidden")).unwrap();

        let outcome = scan_skills_dir(&skills_dir, SkillScope::User);
        assert_eq!(outcome.skills.len(), 1);
        assert_eq!(outcome.skills[0].id, "test-skill");
    }

    #[test]
    fn test_load_full_skill() {
        let temp = TempDir::new().unwrap();
        let skill_md = temp.path().join("SKILL.md");

        let content = r#"---
name: Full Test
description: Testing full load
---

# Body

Content here.
"#;

        std::fs::write(&skill_md, content).unwrap();

        let skill = load_skill(&skill_md, SkillScope::Repo).unwrap();
        assert_eq!(skill.metadata.id, "full-test");
        assert!(skill.content.contains("# Body"));
    }

    #[test]
    fn test_user_skills_dir() {
        let user_dir = user_skills_dir();
        // Just verify it returns Some path ending with .config/alan/skills
        if let Some(dir) = user_dir {
            let path_str = dir.to_string_lossy();
            assert!(path_str.ends_with(".config/alan/skills"));
        }
    }

    #[test]
    fn test_repo_skills_dir() {
        let temp = TempDir::new().unwrap();
        let repo_dir = repo_skills_dir(temp.path());
        let path_str = repo_dir.to_string_lossy();
        assert!(path_str.ends_with(".alan/skills"));
    }
}
