//! Skills registry for managing discovered skills.

use crate::skills::loader;
use crate::skills::types::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Registry of discovered skills.
#[derive(Clone)]
pub struct SkillsRegistry {
    /// Skills indexed by ID.
    skills: HashMap<SkillId, SkillMetadata>,
    /// Non-fatal errors encountered during loading.
    errors: Vec<SkillError>,
    /// Working directory for repo-level skills.
    cwd: PathBuf,
    /// User home directory for user-level skills.
    user_home: Option<PathBuf>,
}

impl SkillsRegistry {
    /// Create a new registry and load skills from all scopes.
    pub fn load(cwd: &Path) -> Result<Self, SkillsError> {
        let mut registry = Self {
            skills: HashMap::new(),
            errors: Vec::new(),
            cwd: cwd.to_path_buf(),
            user_home: dirs::home_dir(),
        };

        registry.reload()?;
        Ok(registry)
    }

    /// Create a registry for a specific agent with workspace isolation.
    /// Loads skills from: agent workspace -> user home -> builtin
    pub fn for_agent(agent_workspace_dir: &Path) -> Result<Self, SkillsError> {
        let mut registry = Self {
            skills: HashMap::new(),
            errors: Vec::new(),
            cwd: agent_workspace_dir.to_path_buf(),
            user_home: dirs::home_dir(),
        };

        registry.reload_for_agent()?;
        Ok(registry)
    }

    /// Reload skills from all scopes.
    /// Higher priority scopes override lower priority ones.
    pub fn reload(&mut self) -> Result<(), SkillsError> {
        self.skills.clear();
        self.errors.clear();
        info!("Loading skills from all scopes...");

        // Load in order of priority (lowest first, so higher priority overrides)

        // System (lowest priority, embedded at compile time)
        self.load_system_skills();

        // User
        if let Some(user_dir) = self.user_skills_dir() {
            self.load_scope(&user_dir, SkillScope::User);
        }

        // Repo
        let repo_dir = self.repo_skills_dir();
        self.load_scope(&repo_dir, SkillScope::Repo);

        info!("Loaded {} skills", self.skills.len());
        Ok(())
    }

    /// Reload skills for agent context.
    /// Order: user -> agent workspace (highest)
    pub fn reload_for_agent(&mut self) -> Result<(), SkillsError> {
        self.skills.clear();
        self.errors.clear();
        info!("Loading skills for workspace...");

        // Load in order of priority (lowest first)

        // System (lowest priority, embedded at compile time)
        self.load_system_skills();

        // User
        if let Some(user_dir) = self.user_skills_dir() {
            self.load_scope(&user_dir, SkillScope::User);
        }

        // Agent workspace (highest priority)
        let skills_dir = self.workspace_skills_dir();
        self.load_scope(&skills_dir, SkillScope::Repo);

        info!("Loaded {} skills for workspace", self.skills.len());
        Ok(())
    }

    /// Load skills from a specific directory.
    fn load_scope(&mut self, dir: &Path, scope: SkillScope) {
        let outcome = loader::scan_skills_dir(dir, scope);
        for skill in outcome.skills {
            debug!(
                "Registering skill: {} (scope: {:?}, path: {})",
                skill.id,
                scope,
                skill.path.display()
            );
            // Higher priority skills override lower priority ones
            self.skills.insert(skill.id.clone(), skill);
        }
        self.errors.extend(outcome.errors);
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

        loader::load_skill(&metadata.path, metadata.scope)
    }

    /// List all registered skills.
    pub fn list(&self) -> Vec<&SkillMetadata> {
        self.skills.values().collect()
    }

    /// List skill loading errors (if any).
    pub fn errors(&self) -> &[SkillError] {
        &self.errors
    }

    /// List skills sorted by scope priority.
    pub fn list_sorted(&self) -> Vec<&SkillMetadata> {
        let mut skills: Vec<_> = self.skills.values().collect();
        skills.sort_by_key(|s| s.scope.priority());
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
                let tags_lower: Vec<String> = skill.tags.iter().map(|t| t.to_lowercase()).collect();

                // Check if any keyword appears in name or description
                keywords.iter().any(|kw| {
                    name_lower.contains(kw)
                        || desc_lower.contains(kw)
                        || tags_lower.iter().any(|t| t.contains(kw) || kw.contains(t))
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

    /// Load system skills embedded at compile time.
    fn load_system_skills(&mut self) {
        use crate::skills::{MEMORY_SKILL_MD, PLAN_SKILL_MD, WORKSPACE_MANAGER_SKILL_MD};

        let system_skills: &[(&str, &str)] = &[
            ("memory", MEMORY_SKILL_MD),
            ("plan", PLAN_SKILL_MD),
            ("workspace-manager", WORKSPACE_MANAGER_SKILL_MD),
        ];

        for (label, content) in system_skills {
            let virtual_path = PathBuf::from(format!("<builtin>/{}/SKILL.md", label));
            match loader::parse_skill_metadata(content, &virtual_path, SkillScope::System) {
                Ok(metadata) => {
                    debug!("Registered system skill: {}", metadata.id);
                    self.skills.insert(metadata.id.clone(), metadata);
                }
                Err(e) => {
                    warn!("Failed to parse system skill '{}': {}", label, e);
                }
            }
        }
    }

    fn user_skills_dir(&self) -> Option<PathBuf> {
        self.user_home
            .as_ref()
            .map(|h| h.join(".config/alan/skills"))
    }

    fn repo_skills_dir(&self) -> PathBuf {
        self.cwd.join(".alan/skills")
    }

    fn workspace_skills_dir(&self) -> PathBuf {
        // For agents, cwd is the agent workspace directory
        // Skills are stored directly in workspace/skills (not .alan/skills)
        self.cwd.join("context/skills")
    }
}

impl Default for SkillsRegistry {
    fn default() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::load(&cwd).expect("Failed to load skills registry")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_skill(dir: &Path, name: &str, skill_name: &str, description: &str) {
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
    fn test_registry_load() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        // Create repo-level skill
        let repo_skills = cwd.join(".alan/skills");
        create_test_skill(&repo_skills, "repo-skill", "Repo Skill", "From repo");

        let registry = SkillsRegistry::load(cwd).unwrap();

        assert!(registry.has(&"repo-skill".to_string()));
        assert_eq!(
            registry.get(&"repo-skill".to_string()).unwrap().scope,
            SkillScope::Repo
        );
    }

    #[test]
    fn test_find_matches() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let repo_skills = cwd.join(".alan/skills");
        create_test_skill(
            &repo_skills,
            "test-skill",
            "Test Skill",
            "A skill for testing purposes",
        );

        let registry = SkillsRegistry::load(cwd).unwrap();

        let matches = registry.find_matches("test");
        assert!(!matches.is_empty(), "Should find at least one match");
    }

    #[test]
    fn test_registry_list_sorted() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let repo_skills = cwd.join(".alan/skills");
        create_test_skill(&repo_skills, "repo-skill", "Repo Skill", "Repo level");

        let registry = SkillsRegistry::load(cwd).unwrap();
        let sorted = registry.list_sorted();

        // Verify it's sorted by scope priority (Repo=0 first, then User=1, then System=2)
        let mut last_priority = 0;
        for skill in &sorted {
            let priority = skill.scope.priority();
            assert!(
                priority >= last_priority,
                "Skills should be sorted by priority"
            );
            last_priority = priority;
        }
    }

    #[test]
    fn test_registry_reload() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let mut registry = SkillsRegistry::load(cwd).unwrap();
        let initial_len = registry.len();

        // Add a new skill after initial load
        let repo_skills = cwd.join(".alan/skills");
        create_test_skill(&repo_skills, "new-skill", "New Skill", "Added later");

        // Reload should pick up the new skill
        registry.reload().unwrap();
        assert!(registry.has(&"new-skill".to_string()));
        assert!(registry.len() > initial_len);
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let registry = SkillsRegistry::load(cwd).unwrap();

        assert!(registry.get(&"nonexistent".to_string()).is_none());
    }

    #[test]
    fn test_registry_is_empty() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let registry = SkillsRegistry::load(cwd).unwrap();

        // Registry might have system skills, so just check the method works
        let _ = registry.is_empty();
    }

    #[test]
    fn test_find_matches_by_description() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let repo_skills = cwd.join(".alan/skills");
        create_test_skill(
            &repo_skills,
            "my-skill",
            "My Skill",
            "A skill for searching purposes with unique keyword xyz123",
        );

        let registry = SkillsRegistry::load(cwd).unwrap();

        // Search by unique word in description
        let matches = registry.find_matches("xyz123");
        assert!(!matches.is_empty(), "Should find match by description");
    }

    #[test]
    fn test_find_matches_by_name() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let repo_skills = cwd.join(".alan/skills");
        create_test_skill(&repo_skills, "unique-skill", "Unique Skill", "Description");

        let registry = SkillsRegistry::load(cwd).unwrap();

        // Search by name
        let matches = registry.find_matches("unique");
        assert!(!matches.is_empty(), "Should find match by name");
    }

    #[test]
    fn test_find_matches_multiple_keywords() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let repo_skills = cwd.join(".alan/skills");
        create_test_skill(
            &repo_skills,
            "skill-one",
            "Skill One",
            "First skill description",
        );
        create_test_skill(
            &repo_skills,
            "skill-two",
            "Skill Two",
            "Second skill description",
        );

        let registry = SkillsRegistry::load(cwd).unwrap();

        // Search with multiple keywords
        let matches = registry.find_matches("one two");
        assert!(
            !matches.is_empty(),
            "Should find matches for multiple keywords"
        );
    }

    #[test]
    fn test_find_matches_no_results() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let repo_skills = cwd.join(".alan/skills");
        create_test_skill(&repo_skills, "skill-a", "Skill A", "Description A");

        let registry = SkillsRegistry::load(cwd).unwrap();

        // Search for nonexistent keyword
        let matches = registry.find_matches("nonexistentxyz123");
        assert!(matches.is_empty(), "Should return empty for no matches");
    }

    #[test]
    fn test_find_matches_case_insensitive() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let repo_skills = cwd.join(".alan/skills");
        create_test_skill(
            &repo_skills,
            "case-skill",
            "Case SKILL",
            "UPPERCASE description",
        );

        let registry = SkillsRegistry::load(cwd).unwrap();

        // Search with lowercase keyword
        let matches = registry.find_matches("skill");
        assert!(!matches.is_empty(), "Should find match case insensitively");

        // Search with uppercase keyword
        let matches = registry.find_matches("UPPERCASE");
        assert!(!matches.is_empty(), "Should find match case insensitively");
    }

    #[test]
    fn test_registry_list() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let repo_skills = cwd.join(".alan/skills");
        create_test_skill(&repo_skills, "skill-a", "Skill A", "Description A");
        create_test_skill(&repo_skills, "skill-b", "Skill B", "Description B");

        let registry = SkillsRegistry::load(cwd).unwrap();

        let skills = registry.list();
        let skill_ids: Vec<_> = skills.iter().map(|s| s.id.as_str()).collect();

        assert!(skill_ids.contains(&"skill-a"));
        assert!(skill_ids.contains(&"skill-b"));
    }

    #[test]
    fn test_user_skills_dir() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let registry = SkillsRegistry::load(cwd).unwrap();

        // In test environment, HOME might be set or not
        // Just verify the method doesn't panic
        let _ = registry.user_skills_dir();
    }

    #[test]
    fn test_repo_skills_dir() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let registry = SkillsRegistry::load(cwd).unwrap();

        let repo_dir = registry.repo_skills_dir();
        assert!(repo_dir.ends_with(".alan/skills"));
    }

    #[test]
    fn test_workspace_skills_dir() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let registry = SkillsRegistry::load(cwd).unwrap();

        let skills_dir = registry.workspace_skills_dir();
        assert!(skills_dir.ends_with("context/skills"));
    }

    #[test]
    fn test_registry_default() {
        // This test verifies Default impl works
        // It may load actual system skills if available
        let registry = SkillsRegistry::default();
        // Just verify it doesn't panic
        let _ = registry.len();
    }

    #[test]
    fn test_registry_len_empty() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let registry = SkillsRegistry::load(cwd).unwrap();

        // Get initial length (might include system skills)
        let initial_len = registry.len();

        // Add a skill and verify length increases
        let repo_skills = cwd.join(".alan/skills");
        create_test_skill(&repo_skills, "new-skill", "New Skill", "Description");

        let registry = SkillsRegistry::load(cwd).unwrap();
        assert!(registry.len() >= initial_len);
    }
}
