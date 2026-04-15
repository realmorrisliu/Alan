use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::prompts::{
    MEMORY_INBOX_DIRNAME, MEMORY_TOPICS_DIRNAME, MEMORY_USER_FILENAME, WORKSPACE_MEMORY_FILENAME,
    ensure_workspace_memory_layout_at,
};
use crate::session::Session;
use crate::tape::Message;

const DEFAULT_PROMOTED_FACTS_HEADER: &str = "## Promoted Facts";
const DEFAULT_TOPIC_SUMMARY: &str = "Promoted from inbox entries.";
const DEFAULT_EVIDENCE_ITEM: &str = "No evidence recorded.";

#[derive(Debug, Clone)]
pub(crate) struct InboxEntryDraft {
    pub kind: &'static str,
    pub target: String,
    pub confidence: &'static str,
    pub observation: String,
    pub evidence: Vec<String>,
    pub promotion_rationale: String,
    pub source_sessions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromotionOutcome {
    pub inbox_path: PathBuf,
    pub target_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct InboxEntryFrontmatter {
    id: String,
    kind: String,
    status: String,
    target: String,
    confidence: String,
    created_at: String,
    updated_at: String,
    source_sessions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InboxEntryDocument {
    frontmatter: InboxEntryFrontmatter,
    observation: String,
    evidence: Vec<String>,
    promotion_rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TopicPageFrontmatter {
    title: String,
    aliases: Vec<String>,
    tags: Vec<String>,
    entities: Vec<String>,
    updated_at: String,
    source_sessions: Vec<String>,
}

pub(crate) async fn stage_inbox_entry(
    memory_dir: &Path,
    draft: InboxEntryDraft,
    now: DateTime<Utc>,
) -> Result<PathBuf> {
    ensure_workspace_memory_layout_at(memory_dir).with_context(|| {
        format!(
            "failed to ensure workspace memory layout before staging inbox entry at {}",
            memory_dir.display()
        )
    })?;

    let id = format!("inbox-{}", uuid::Uuid::new_v4().simple());
    let path = inbox_entry_path(memory_dir, now, &id);
    let document = InboxEntryDocument {
        frontmatter: InboxEntryFrontmatter {
            id,
            kind: draft.kind.to_string(),
            status: "observed".to_string(),
            target: draft.target,
            confidence: draft.confidence.to_string(),
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
            source_sessions: dedup_strings(draft.source_sessions),
        },
        observation: draft.observation.trim().to_string(),
        evidence: normalize_items(draft.evidence),
        promotion_rationale: draft.promotion_rationale.trim().to_string(),
    };

    write_text_file(&path, &render_inbox_entry(&document)).await?;
    Ok(path)
}

pub(crate) async fn promote_inbox_entry(
    memory_dir: &Path,
    inbox_path: &Path,
    now: DateTime<Utc>,
) -> Result<PromotionOutcome> {
    ensure_workspace_memory_layout_at(memory_dir).with_context(|| {
        format!(
            "failed to ensure workspace memory layout before promoting inbox entry at {}",
            memory_dir.display()
        )
    })?;

    let raw = tokio::fs::read_to_string(inbox_path)
        .await
        .with_context(|| format!("read inbox entry {}", inbox_path.display()))?;
    let mut document = parse_inbox_entry(&raw)
        .with_context(|| format!("parse inbox entry {}", inbox_path.display()))?;
    let target_path = resolve_target_path(memory_dir, &document.frontmatter.target)?;
    let promoted_from = format_relative_memory_path(memory_dir, inbox_path);
    let promoted_stamp = now.format("%F").to_string();
    let promoted_line = format!(
        "- [{}] {} (promoted from {})",
        promoted_stamp,
        document.observation.trim(),
        promoted_from
    );

    match document.frontmatter.target.as_str() {
        MEMORY_USER_FILENAME | WORKSPACE_MEMORY_FILENAME => {
            let existing = read_text_file_or_default(&target_path).await?;
            if !existing.contains(document.observation.trim()) {
                let updated = append_markdown_section_item(
                    &existing,
                    DEFAULT_PROMOTED_FACTS_HEADER,
                    &promoted_line,
                );
                write_text_file(&target_path, &updated).await?;
            }
        }
        target if is_topic_target(target) => {
            let existing = read_text_file_or_default(&target_path).await?;
            let title = slug_to_title(topic_slug_from_target(target)?);
            let mut topic = ensure_topic_page_frontmatter(
                &existing,
                &title,
                now,
                &document.frontmatter.source_sessions,
            )?;
            if !topic.contains(document.observation.trim()) {
                topic = append_markdown_section_item(&topic, "## Stable Facts", &promoted_line);
                for evidence in document
                    .evidence
                    .iter()
                    .filter(|value| !value.trim().is_empty())
                {
                    topic = append_markdown_section_item(
                        &topic,
                        "## References",
                        &format!("- {evidence}"),
                    );
                }
                topic = append_markdown_section_item(
                    &topic,
                    "## References",
                    &format!("- Source inbox entry: {promoted_from}"),
                );
            }
            write_text_file(&target_path, &topic).await?;

            let memory_path = memory_dir.join(WORKSPACE_MEMORY_FILENAME);
            let memory_content = read_text_file_or_default(&memory_path).await?;
            let topic_index_line = format!(
                "- {} -> topics/{}.md",
                topic_slug_from_target(target)?,
                topic_slug_from_target(target)?
            );
            let updated_memory =
                append_markdown_section_item(&memory_content, "## Topic Index", &topic_index_line);
            write_text_file(&memory_path, &updated_memory).await?;
        }
        other => bail!("unsupported inbox promotion target: {other}"),
    }

    document.frontmatter.status = "confirmed".to_string();
    document.frontmatter.updated_at = now.to_rfc3339();
    write_text_file(inbox_path, &render_inbox_entry(&document)).await?;

    Ok(PromotionOutcome {
        inbox_path: inbox_path.to_path_buf(),
        target_path,
    })
}

pub(crate) async fn capture_confirmed_turn_memory(
    memory_dir: Option<&Path>,
    session: &Session,
) -> Result<()> {
    let Some(memory_dir) = memory_dir else {
        return Ok(());
    };

    let drafts = derive_confirmed_memory_drafts(session);
    if drafts.is_empty() {
        return Ok(());
    }

    let now = Utc::now();
    for draft in drafts {
        let inbox_path = stage_inbox_entry(memory_dir, draft, now).await?;
        promote_inbox_entry(memory_dir, &inbox_path, now).await?;
    }

    Ok(())
}

fn parse_inbox_entry(content: &str) -> Result<InboxEntryDocument> {
    let (frontmatter, body) = split_frontmatter(content)?;
    let frontmatter: InboxEntryFrontmatter =
        serde_yaml::from_str(frontmatter).context("parse inbox frontmatter")?;

    let observation = extract_markdown_section(body, "## Observation")
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    let evidence = extract_markdown_section(body, "## Evidence")
        .map(parse_markdown_list)
        .unwrap_or_default();
    let promotion_rationale = extract_markdown_section(body, "## Promotion Rationale")
        .map(str::trim)
        .unwrap_or_default()
        .to_string();

    Ok(InboxEntryDocument {
        frontmatter,
        observation,
        evidence,
        promotion_rationale,
    })
}

fn render_inbox_entry(document: &InboxEntryDocument) -> String {
    let frontmatter = render_yaml_without_leading_delimiter(&document.frontmatter)
        .expect("serialize inbox frontmatter");
    let evidence = if document.evidence.is_empty() {
        format!("- {DEFAULT_EVIDENCE_ITEM}")
    } else {
        document
            .evidence
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "---\n{frontmatter}---\n\n## Observation\n{}\n\n## Evidence\n{}\n\n## Promotion Rationale\n{}\n",
        document.observation.trim(),
        evidence,
        document.promotion_rationale.trim()
    )
}

fn resolve_target_path(memory_dir: &Path, target: &str) -> Result<PathBuf> {
    match target {
        MEMORY_USER_FILENAME => Ok(memory_dir.join(MEMORY_USER_FILENAME)),
        WORKSPACE_MEMORY_FILENAME => Ok(memory_dir.join(WORKSPACE_MEMORY_FILENAME)),
        _ if is_topic_target(target) => Ok(memory_dir.join(target)),
        _ => bail!("unsupported inbox target path: {target}"),
    }
}

fn ensure_topic_page_frontmatter(
    content: &str,
    title: &str,
    now: DateTime<Utc>,
    source_sessions: &[String],
) -> Result<String> {
    let existing_body = if content.trim().is_empty() {
        default_topic_body(title)
    } else if let Ok((_, body)) = split_frontmatter(content) {
        body.trim().to_string()
    } else {
        content.trim().to_string()
    };

    let frontmatter = if let Ok((yaml, _)) = split_frontmatter(content) {
        let mut parsed: TopicPageFrontmatter =
            serde_yaml::from_str(yaml).context("parse topic page frontmatter")?;
        parsed.updated_at = now.to_rfc3339();
        parsed.source_sessions = merge_source_sessions(parsed.source_sessions, source_sessions);
        parsed
    } else {
        TopicPageFrontmatter {
            title: title.to_string(),
            aliases: Vec::new(),
            tags: Vec::new(),
            entities: Vec::new(),
            updated_at: now.to_rfc3339(),
            source_sessions: dedup_strings(source_sessions.to_vec()),
        }
    };

    let frontmatter = render_yaml_without_leading_delimiter(&frontmatter)
        .context("serialize topic page frontmatter")?;
    Ok(format!(
        "---\n{frontmatter}---\n\n{}\n",
        existing_body.trim()
    ))
}

fn default_topic_body(title: &str) -> String {
    format!(
        "# {title}\n\n## Summary\n{DEFAULT_TOPIC_SUMMARY}\n\n## Stable Facts\n\n## Key Decisions\n\n## Open Questions\n\n## References\n"
    )
}

fn append_markdown_section_item(content: &str, heading: &str, item: &str) -> String {
    if item.trim().is_empty() || content.contains(item) {
        return ensure_trailing_newline(content);
    }

    let normalized = ensure_trailing_newline(content);
    if let Some(start) = normalized.find(heading) {
        let search_start = start + heading.len();
        let section_tail = &normalized[search_start..];
        let next_section_offset = section_tail
            .find("\n## ")
            .map(|offset| search_start + offset);
        let insertion_at = next_section_offset.unwrap_or(normalized.len());
        let mut updated = String::with_capacity(normalized.len() + item.len() + 4);
        updated.push_str(&normalized[..insertion_at]);
        if !updated.ends_with("\n\n") {
            if !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push('\n');
        }
        updated.push_str(item.trim());
        updated.push('\n');
        if insertion_at < normalized.len() && !normalized[insertion_at..].starts_with('\n') {
            updated.push('\n');
        }
        updated.push_str(&normalized[insertion_at..]);
        return updated;
    }

    let mut updated = normalized.trim_end().to_string();
    if !updated.is_empty() {
        updated.push_str("\n\n");
    }
    updated.push_str(heading);
    updated.push_str("\n\n");
    updated.push_str(item.trim());
    updated.push('\n');
    updated
}

fn extract_markdown_section<'a>(content: &'a str, heading: &str) -> Option<&'a str> {
    let start = content.find(heading)?;
    let body_start = start + heading.len();
    let section_tail = &content[body_start..];
    let next_section_offset = section_tail.find("\n## ").unwrap_or(section_tail.len());
    Some(section_tail[..next_section_offset].trim())
}

fn split_frontmatter(content: &str) -> Result<(&str, &str)> {
    let trimmed = content.trim_start();
    let remainder = trimmed
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow!("missing frontmatter delimiter"))?;
    let (frontmatter, body) = remainder
        .split_once("\n---\n")
        .ok_or_else(|| anyhow!("missing closing frontmatter delimiter"))?;
    Ok((frontmatter, body))
}

fn render_yaml_without_leading_delimiter<T: Serialize>(value: &T) -> Result<String> {
    let rendered = serde_yaml::to_string(value).context("render yaml")?;
    Ok(rendered
        .strip_prefix("---\n")
        .unwrap_or(rendered.as_str())
        .to_string())
}

fn parse_markdown_list(section: &str) -> Vec<String> {
    section
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("- "))
        .map(|line| line.trim_start_matches("- ").trim().to_string())
        .filter(|line| !line.is_empty() && line != DEFAULT_EVIDENCE_ITEM)
        .collect()
}

fn normalize_items(items: Vec<String>) -> Vec<String> {
    dedup_strings(
        items
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

fn merge_source_sessions(mut existing: Vec<String>, additional: &[String]) -> Vec<String> {
    existing.extend(additional.iter().cloned());
    dedup_strings(existing)
}

fn dedup_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn derive_confirmed_memory_drafts(session: &Session) -> Vec<InboxEntryDraft> {
    let Some(user_text) = session
        .tape
        .messages()
        .iter()
        .rev()
        .find(|message| message.is_user())
        .map(Message::text_content)
    else {
        return Vec::new();
    };

    let normalized = normalize_whitespace(&user_text);
    let lowered = normalized.to_lowercase();
    let mut drafts = Vec::new();

    if let Some(name) = extract_fact_after_prefix(&normalized, &lowered, &["my name is "]) {
        drafts.push(InboxEntryDraft {
            kind: "user_identity",
            target: MEMORY_USER_FILENAME.to_string(),
            confidence: "high",
            observation: format!("Name: {name}"),
            evidence: vec![normalized.clone()],
            promotion_rationale: "Direct user-stated stable identity detail.".to_string(),
            source_sessions: vec![session.id.clone()],
        });
    }

    if let Some(preference) =
        extract_fact_after_prefix(&normalized, &lowered, &["i prefer ", "my preferred "])
    {
        drafts.push(InboxEntryDraft {
            kind: "user_preference",
            target: MEMORY_USER_FILENAME.to_string(),
            confidence: "high",
            observation: format!("Preference: {preference}"),
            evidence: vec![normalized.clone()],
            promotion_rationale: "Direct user-stated stable preference.".to_string(),
            source_sessions: vec![session.id.clone()],
        });
    }

    if let Some(favorite) = extract_favorite_fact(&normalized, &lowered) {
        drafts.push(InboxEntryDraft {
            kind: "user_preference",
            target: MEMORY_USER_FILENAME.to_string(),
            confidence: "high",
            observation: favorite,
            evidence: vec![normalized.clone()],
            promotion_rationale: "Direct user-stated favorite/preference detail.".to_string(),
            source_sessions: vec![session.id.clone()],
        });
    }

    if let Some(constraint) = extract_fact_after_prefix(
        &normalized,
        &lowered,
        &[
            "remember this constraint: ",
            "remember the constraint: ",
            "the constraint is ",
        ],
    ) {
        drafts.push(InboxEntryDraft {
            kind: "workspace_fact",
            target: WORKSPACE_MEMORY_FILENAME.to_string(),
            confidence: "high",
            observation: format!("Constraint: {constraint}"),
            evidence: vec![normalized.clone()],
            promotion_rationale: "Direct user-stated durable workspace constraint.".to_string(),
            source_sessions: vec![session.id.clone()],
        });
    }

    if let Some(rule) = extract_fact_after_prefix(
        &normalized,
        &lowered,
        &[
            "remember this rule: ",
            "remember the rule: ",
            "the rule is ",
            "workflow rule: ",
        ],
    ) {
        drafts.push(InboxEntryDraft {
            kind: "workflow_rule",
            target: WORKSPACE_MEMORY_FILENAME.to_string(),
            confidence: "high",
            observation: format!("Workflow rule: {rule}"),
            evidence: vec![normalized.clone()],
            promotion_rationale: "Direct user-stated durable workflow rule.".to_string(),
            source_sessions: vec![session.id.clone()],
        });
    }

    drafts
}

fn extract_fact_after_prefix(original: &str, lowered: &str, prefixes: &[&str]) -> Option<String> {
    prefixes.iter().find_map(|prefix| {
        lowered.find(prefix).and_then(|start| {
            let start = start + prefix.len();
            let extracted = extract_until_sentence_boundary(&original[start..]);
            (!extracted.is_empty()).then_some(extracted)
        })
    })
}

fn extract_favorite_fact(original: &str, lowered: &str) -> Option<String> {
    let favorite_start = lowered.find("my favorite ")?;
    let original_tail = &original[favorite_start + "my favorite ".len()..];
    let lowered_tail = &lowered[favorite_start + "my favorite ".len()..];
    let is_pos = lowered_tail.find(" is ")?;
    let subject = extract_until_sentence_boundary(&original_tail[..is_pos]);
    let value = extract_until_sentence_boundary(&original_tail[is_pos + " is ".len()..]);
    if subject.is_empty() || value.is_empty() {
        return None;
    }
    Some(format!(
        "Favorite {}: {}",
        subject.trim_end_matches('s'),
        value
    ))
}

fn extract_until_sentence_boundary(text: &str) -> String {
    let trimmed = text.trim();
    let boundary = trimmed.find(['.', '\n', '!', '?']).unwrap_or(trimmed.len());
    trimmed[..boundary]
        .trim()
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .trim_end_matches(',')
        .to_string()
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_topic_target(target: &str) -> bool {
    target.starts_with(&format!("{MEMORY_TOPICS_DIRNAME}/")) && target.ends_with(".md")
}

fn topic_slug_from_target(target: &str) -> Result<&str> {
    target
        .strip_prefix(&format!("{MEMORY_TOPICS_DIRNAME}/"))
        .and_then(|value| value.strip_suffix(".md"))
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("invalid topic target: {target}"))
}

fn slug_to_title(slug: &str) -> String {
    slug.split('-')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => {
                    format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn inbox_entry_path(memory_dir: &Path, now: DateTime<Utc>, id: &str) -> PathBuf {
    memory_dir.join(MEMORY_INBOX_DIRNAME).join(format!(
        "{:04}/{:02}/{:02}/{}.md",
        now.year(),
        now.month(),
        now.day(),
        id
    ))
}

fn format_relative_memory_path(memory_dir: &Path, path: &Path) -> String {
    path.strip_prefix(memory_dir)
        .map(|relative| format!(".alan/memory/{}", relative.display()))
        .unwrap_or_else(|_| path.display().to_string())
}

fn ensure_trailing_newline(content: &str) -> String {
    let mut normalized = content.trim_end().to_string();
    if !normalized.is_empty() {
        normalized.push('\n');
    }
    normalized
}

async fn read_text_file_or_default(path: &Path) -> Result<String> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => Ok(content),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(err).with_context(|| format!("read {}", path.display())),
    }
}

async fn write_text_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("create directory {}", parent.display()))?;
    }
    let mut file = tokio::fs::File::create(path)
        .await
        .with_context(|| format!("create {}", path.display()))?;
    file.write_all(content.as_bytes())
        .await
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn stage_inbox_entry_writes_expected_observed_entry() {
        let temp = TempDir::new().unwrap();
        let memory_dir = temp.path().join(".alan/memory");
        let now = DateTime::parse_from_rfc3339("2026-04-15T10:30:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let inbox_path = stage_inbox_entry(
            &memory_dir,
            InboxEntryDraft {
                kind: "workspace_fact",
                target: WORKSPACE_MEMORY_FILENAME.to_string(),
                confidence: "medium",
                observation: "The recall router should stay lexical-only.".to_string(),
                evidence: vec!["Observed in session summary.".to_string()],
                promotion_rationale: "Useful, but not yet confirmed as stable memory.".to_string(),
                source_sessions: vec!["sess-123".to_string()],
            },
            now,
        )
        .await
        .unwrap();

        let stored = tokio::fs::read_to_string(&inbox_path).await.unwrap();
        assert!(stored.contains("status: observed"));
        assert!(stored.contains("target: MEMORY.md"));
        assert!(stored.contains("## Observation"));
        assert!(stored.contains("lexical-only"));
    }

    #[tokio::test]
    async fn promote_inbox_entry_updates_memory_file_and_marks_confirmed() {
        let temp = TempDir::new().unwrap();
        let memory_dir = temp.path().join(".alan/memory");
        ensure_workspace_memory_layout_at(&memory_dir).unwrap();
        let now = DateTime::parse_from_rfc3339("2026-04-15T10:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let inbox_path = stage_inbox_entry(
            &memory_dir,
            InboxEntryDraft {
                kind: "workspace_fact",
                target: WORKSPACE_MEMORY_FILENAME.to_string(),
                confidence: "high",
                observation: "Keep memory recall lexical and file-backed.".to_string(),
                evidence: vec!["Repeated in design notes.".to_string()],
                promotion_rationale: "Confirmed by the user.".to_string(),
                source_sessions: vec!["sess-456".to_string()],
            },
            now,
        )
        .await
        .unwrap();

        let outcome = promote_inbox_entry(&memory_dir, &inbox_path, now)
            .await
            .unwrap();

        assert_eq!(
            outcome.target_path,
            memory_dir.join(WORKSPACE_MEMORY_FILENAME)
        );
        let memory_file = tokio::fs::read_to_string(memory_dir.join(WORKSPACE_MEMORY_FILENAME))
            .await
            .unwrap();
        assert!(memory_file.contains("## Promoted Facts"));
        assert!(memory_file.contains("lexical and file-backed"));

        let updated_inbox = tokio::fs::read_to_string(inbox_path).await.unwrap();
        assert!(updated_inbox.contains("status: confirmed"));
    }

    #[tokio::test]
    async fn promote_topic_entry_creates_topic_page_and_memory_index() {
        let temp = TempDir::new().unwrap();
        let memory_dir = temp.path().join(".alan/memory");
        ensure_workspace_memory_layout_at(&memory_dir).unwrap();
        let now = DateTime::parse_from_rfc3339("2026-04-15T10:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let inbox_path = stage_inbox_entry(
            &memory_dir,
            InboxEntryDraft {
                kind: "topic_fact",
                target: "topics/memory-router.md".to_string(),
                confidence: "medium",
                observation: "Topic pages are the overflow surface for recurring memory facts."
                    .to_string(),
                evidence: vec!["Repeated across multiple sessions.".to_string()],
                promotion_rationale: "Recurring enough to deserve a topic page.".to_string(),
                source_sessions: vec!["sess-789".to_string()],
            },
            now,
        )
        .await
        .unwrap();

        let outcome = promote_inbox_entry(&memory_dir, &inbox_path, now)
            .await
            .unwrap();

        let topic_path = memory_dir.join("topics/memory-router.md");
        assert_eq!(outcome.target_path, topic_path);

        let topic_page = tokio::fs::read_to_string(memory_dir.join("topics/memory-router.md"))
            .await
            .unwrap();
        assert!(topic_page.contains("title: Memory Router"));
        assert!(topic_page.contains("## Stable Facts"));
        assert!(topic_page.contains("overflow surface"));

        let memory_file = tokio::fs::read_to_string(memory_dir.join(WORKSPACE_MEMORY_FILENAME))
            .await
            .unwrap();
        assert!(memory_file.contains("## Topic Index"));
        assert!(memory_file.contains("memory-router -> topics/memory-router.md"));
    }

    #[tokio::test]
    async fn capture_confirmed_turn_memory_promotes_explicit_user_fact() {
        let temp = TempDir::new().unwrap();
        let memory_dir = temp.path().join(".alan/memory");
        ensure_workspace_memory_layout_at(&memory_dir).unwrap();

        let mut session = Session::new();
        session.id = "sess-confirm".to_string();
        session.add_user_message("My favorite editor is Helix.");

        capture_confirmed_turn_memory(Some(&memory_dir), &session)
            .await
            .unwrap();

        let user_memory = tokio::fs::read_to_string(memory_dir.join(MEMORY_USER_FILENAME))
            .await
            .unwrap();
        assert!(user_memory.contains("Favorite editor: Helix"));

        let inbox_root = memory_dir.join(MEMORY_INBOX_DIRNAME);
        let inbox_entries = collect_markdown_files_recursively(&inbox_root);
        assert!(!inbox_entries.is_empty());
    }

    fn collect_markdown_files_recursively(dir: &Path) -> Vec<PathBuf> {
        let mut collected = Vec::new();
        collect_markdown_files_recursively_inner(dir, &mut collected);
        collected
    }

    fn collect_markdown_files_recursively_inner(dir: &Path, collected: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                collect_markdown_files_recursively_inner(&path, collected);
            } else if path
                .extension()
                .is_some_and(|extension| extension == std::ffi::OsStr::new("md"))
            {
                collected.push(path);
            }
        }
    }
}
