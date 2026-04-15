use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::tape::{ContentPart, parts_to_text};

const MAX_FILE_RECALL_CHARS: usize = 1_200;
const MAX_RECALL_FILES: usize = 4;
const MAX_CANDIDATE_SCAN_FILES: usize = 64;

#[derive(Debug, Clone)]
struct RecallCandidate {
    path: PathBuf,
    score: usize,
}

pub(crate) fn build_turn_recall_bundle(
    memory_dir: Option<&Path>,
    user_input: Option<&[ContentPart]>,
) -> Option<String> {
    let memory_dir = memory_dir?;
    if !memory_dir.exists() {
        return None;
    }

    let query = user_input
        .map(parts_to_text)
        .unwrap_or_default()
        .trim()
        .to_string();
    if query.is_empty() {
        return None;
    }

    let query_lower = query.to_lowercase();
    let query_tokens = tokenize_query(&query_lower);
    let identity_query = is_identity_query(&query_lower);
    let continuity_query = is_continuity_query(&query_lower);
    let workspace_query = is_workspace_query(&query_lower);
    let recent_query = is_recent_query(&query_lower);

    if !identity_query
        && !continuity_query
        && !workspace_query
        && !recent_query
        && query_tokens.is_empty()
    {
        return None;
    }

    let mut selected_paths = Vec::new();
    if identity_query {
        selected_paths.push(memory_dir.join("USER.md"));
    }
    if workspace_query {
        selected_paths.push(memory_dir.join("MEMORY.md"));
    }
    if continuity_query {
        selected_paths.push(memory_dir.join("handoffs").join("LATEST.md"));
    }

    let mut scored_candidates = Vec::new();
    if continuity_query || recent_query {
        scored_candidates.extend(score_candidate_files(
            collect_markdown_files_recursive(
                &memory_dir.join("sessions"),
                MAX_CANDIDATE_SCAN_FILES,
            ),
            &query_tokens,
        ));
        scored_candidates.extend(score_candidate_files(
            collect_markdown_files(&memory_dir.join("daily"), MAX_CANDIDATE_SCAN_FILES),
            &query_tokens,
        ));
    }
    if workspace_query || !query_tokens.is_empty() {
        scored_candidates.extend(score_candidate_files(
            collect_markdown_files(&memory_dir.join("topics"), MAX_CANDIDATE_SCAN_FILES),
            &query_tokens,
        ));
    }

    scored_candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.path.cmp(&left.path))
    });
    for candidate in scored_candidates
        .into_iter()
        .filter(|candidate| candidate.score > 0)
        .take(MAX_RECALL_FILES)
    {
        selected_paths.push(candidate.path);
    }

    let mut seen = BTreeSet::new();
    let sections: Vec<String> = selected_paths
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .filter_map(|path| {
            let content = fs::read_to_string(&path).ok()?;
            let trimmed = content.trim();
            if trimmed.is_empty() {
                return None;
            }
            let relative_path = path
                .strip_prefix(memory_dir)
                .map(|value| format!(".alan/memory/{}", value.display()))
                .unwrap_or_else(|_| path.display().to_string());
            Some(format!(
                "### {relative_path}\n{}\n",
                truncate_chars(trimmed, MAX_FILE_RECALL_CHARS)
            ))
        })
        .collect();

    if sections.is_empty() {
        return None;
    }

    Some(format!(
        "## Runtime Recall Bundle\n\
Selected turn-relevant pure-text memory based on the current user request. Treat this as runtime-routed recall, not raw speculative search.\n\n{}",
        sections.join("\n")
    ))
}

fn is_identity_query(query: &str) -> bool {
    [
        "who am i",
        "my name",
        "my preference",
        "my preferences",
        "favorite",
        "prefer",
    ]
    .iter()
    .any(|needle| query.contains(needle))
}

fn is_continuity_query(query: &str) -> bool {
    [
        "continue",
        "resume",
        "last time",
        "previous session",
        "earlier",
        "before",
        "where did we leave off",
        "what were we doing",
    ]
    .iter()
    .any(|needle| query.contains(needle))
}

fn is_workspace_query(query: &str) -> bool {
    [
        "architecture",
        "decision",
        "constraint",
        "project context",
        "why did we",
        "workspace",
    ]
    .iter()
    .any(|needle| query.contains(needle))
}

fn is_recent_query(query: &str) -> bool {
    ["today", "yesterday", "recent", "latest", "just now"]
        .iter()
        .any(|needle| query.contains(needle))
}

fn tokenize_query(query: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "about", "after", "again", "been", "from", "have", "into", "just", "that", "the", "their",
        "them", "then", "they", "this", "what", "when", "where", "which", "while", "with", "would",
        "your", "were",
    ];

    query
        .split(|ch: char| !ch.is_alphanumeric())
        .map(str::trim)
        .filter(|token| token.len() >= 3)
        .filter(|token| !STOP_WORDS.contains(token))
        .map(ToString::to_string)
        .collect()
}

fn collect_markdown_files(dir: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension == std::ffi::OsStr::new("md"))
        })
        .collect();
    files.sort();
    files.reverse();
    files.truncate(max_files);
    files
}

fn collect_markdown_files_recursive(dir: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut collected = Vec::new();
    collect_markdown_files_recursive_inner(dir, &mut collected, max_files);
    collected.sort();
    collected.reverse();
    collected.truncate(max_files);
    collected
}

fn collect_markdown_files_recursive_inner(
    dir: &Path,
    collected: &mut Vec<PathBuf>,
    max_files: usize,
) {
    if collected.len() >= max_files {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files_recursive_inner(&path, collected, max_files);
        } else if path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == std::ffi::OsStr::new("md"))
        {
            collected.push(path);
            if collected.len() >= max_files {
                return;
            }
        }
    }
}

fn score_candidate_files(paths: Vec<PathBuf>, query_tokens: &[String]) -> Vec<RecallCandidate> {
    paths
        .into_iter()
        .filter_map(|path| {
            let content = fs::read_to_string(&path).ok()?;
            let score = lexical_overlap_score(&path, &content, query_tokens);
            Some(RecallCandidate { path, score })
        })
        .collect()
}

fn lexical_overlap_score(path: &Path, content: &str, query_tokens: &[String]) -> usize {
    if query_tokens.is_empty() {
        return 0;
    }
    let path_text = path.to_string_lossy().to_lowercase();
    let content_text = content.to_lowercase();
    query_tokens
        .iter()
        .filter(|token| path_text.contains(token.as_str()) || content_text.contains(token.as_str()))
        .count()
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let mut truncated = text
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn identity_query_prefers_user_memory() {
        let temp = TempDir::new().unwrap();
        let memory_dir = temp.path().join(".alan/memory");
        crate::prompts::ensure_workspace_memory_layout_at(&memory_dir).unwrap();
        fs::write(
            memory_dir.join("USER.md"),
            "# User Memory\nName: Morris Liu\n",
        )
        .unwrap();

        let bundle = build_turn_recall_bundle(
            Some(&memory_dir),
            Some(&[crate::tape::ContentPart::text("Who am I?")]),
        )
        .expect("expected recall bundle");

        assert!(bundle.contains(".alan/memory/USER.md"));
        assert!(bundle.contains("Morris Liu"));
    }

    #[test]
    fn continuity_query_picks_handoff_and_session_summary() {
        let temp = TempDir::new().unwrap();
        let memory_dir = temp.path().join(".alan/memory");
        crate::prompts::ensure_workspace_memory_layout_at(&memory_dir).unwrap();
        fs::write(
            memory_dir.join("handoffs/LATEST.md"),
            "# Latest Handoff\nWe were refining the recall router.\n",
        )
        .unwrap();
        fs::create_dir_all(memory_dir.join("sessions/2026/04/15")).unwrap();
        fs::write(
            memory_dir.join("sessions/2026/04/15/sess-1.md"),
            "# Session Summary\nRecall router work in progress.\n",
        )
        .unwrap();

        let bundle = build_turn_recall_bundle(
            Some(&memory_dir),
            Some(&[crate::tape::ContentPart::text(
                "What were we doing in the previous session?",
            )]),
        )
        .expect("expected recall bundle");

        assert!(bundle.contains(".alan/memory/handoffs/LATEST.md"));
        assert!(bundle.contains(".alan/memory/sessions/2026/04/15/sess-1.md"));
    }

    #[test]
    fn workspace_query_can_pick_relevant_topic_page() {
        let temp = TempDir::new().unwrap();
        let memory_dir = temp.path().join(".alan/memory");
        crate::prompts::ensure_workspace_memory_layout_at(&memory_dir).unwrap();
        fs::write(
            memory_dir.join("topics/memory-router.md"),
            "# Memory Router\nArchitecture decision: use lexical recall over pure-text files.\n",
        )
        .unwrap();

        let bundle = build_turn_recall_bundle(
            Some(&memory_dir),
            Some(&[crate::tape::ContentPart::text(
                "What is the architecture decision for the memory router?",
            )]),
        )
        .expect("expected recall bundle");

        assert!(bundle.contains(".alan/memory/topics/memory-router.md"));
        assert!(bundle.contains("lexical recall"));
    }
}
