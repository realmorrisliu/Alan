//! Simple workspace-only sandbox.
//!
//! This sandbox only enforces that all operations happen within
//! the workspace directory. No OS-level sandboxing (Landlock/Seatbelt).

use anyhow::{Result, anyhow};
use regex::Regex;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

const SANDBOX_BACKEND_WORKSPACE_PATH_GUARD: &str = "workspace_path_guard";
const PROTECTED_SUBPATHS: [&str; 3] = [".git", ".alan", ".agents"];

/// Execution result from sandbox
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Simple workspace-only sandbox
#[derive(Clone)]
pub struct Sandbox {
    workspace_root: PathBuf,
}

impl Sandbox {
    /// Create a new sandbox restricted to the given workspace
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Name of the active sandbox backend.
    pub fn backend_name(&self) -> &'static str {
        Self::backend_name_static()
    }

    /// Name of the built-in workspace path guard backend.
    pub const fn backend_name_static() -> &'static str {
        SANDBOX_BACKEND_WORKSPACE_PATH_GUARD
    }

    /// Check if a path is within the workspace
    pub fn is_in_workspace(&self, path: &Path) -> bool {
        // Try to get absolute path
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };

        // Get canonical workspace (may fail if doesn't exist)
        let canonical_workspace = self
            .canonicalize(&self.workspace_root)
            .unwrap_or_else(|_| dunce::simplified(&self.workspace_root).to_path_buf());

        // For existing paths, use canonical path
        if absolute_path.exists() {
            let canonical_path = self
                .canonicalize(&absolute_path)
                .unwrap_or_else(|_| dunce::simplified(&absolute_path).to_path_buf());
            return canonical_path.starts_with(&canonical_workspace);
        }

        // For new files, check that all existing parent directories are within workspace
        let mut current = absolute_path.parent();
        while let Some(parent) = current {
            if parent.exists() {
                let canonical_parent = self
                    .canonicalize(parent)
                    .unwrap_or_else(|_| dunce::simplified(parent).to_path_buf());
                return canonical_parent.starts_with(&canonical_workspace);
            }
            current = parent.parent();
        }

        // If no parent exists, check if the path itself starts with workspace
        dunce::simplified(&absolute_path)
            .to_string_lossy()
            .starts_with(&canonical_workspace.to_string_lossy().to_string())
    }

    /// Read a file within the workspace
    pub async fn read(&self, path: &Path) -> Result<Vec<u8>> {
        if !self.is_in_workspace(path) {
            return Err(anyhow!(
                "Path outside workspace: {} (workspace: {})",
                path.display(),
                self.workspace_root.display()
            ));
        }

        tokio::fs::read(path)
            .await
            .map_err(|e| anyhow!("Failed to read file: {}", e))
    }

    /// Read file as string
    pub async fn read_string(&self, path: &Path) -> Result<String> {
        let bytes = self.read(path).await?;
        String::from_utf8(bytes).map_err(|e| anyhow!("Invalid UTF-8: {}", e))
    }

    /// Write a file within the workspace
    pub async fn write(&self, path: &Path, content: &[u8]) -> Result<()> {
        if !self.is_in_workspace(path) {
            return Err(anyhow!(
                "Path outside workspace: {} (workspace: {})",
                path.display(),
                self.workspace_root.display()
            ));
        }
        self.ensure_path_not_protected(path, "write")?;

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(path, content)
            .await
            .map_err(|e| anyhow!("Failed to write file: {}", e))
    }

    /// Execute a command within the workspace
    pub async fn exec(&self, cmd: &str, cwd: &Path) -> Result<ExecResult> {
        self.exec_with_timeout_and_capability(cmd, cwd, None, None)
            .await
    }

    /// Execute a command within the workspace with an optional timeout.
    pub async fn exec_with_timeout(
        &self,
        cmd: &str,
        cwd: &Path,
        timeout: Option<Duration>,
    ) -> Result<ExecResult> {
        self.exec_with_timeout_and_capability(cmd, cwd, timeout, None)
            .await
    }

    /// Execute a command within the workspace with path-guard checks.
    pub async fn exec_with_timeout_and_capability(
        &self,
        cmd: &str,
        cwd: &Path,
        timeout: Option<Duration>,
        _capability: Option<alan_protocol::ToolCapability>,
    ) -> Result<ExecResult> {
        if !self.is_in_workspace(cwd) {
            return Err(anyhow!(
                "Working directory outside workspace: {} (workspace: {})",
                cwd.display(),
                self.workspace_root.display()
            ));
        }
        self.ensure_path_not_protected(cwd, "process cwd")?;

        self.validate_shell_features(cmd)?;
        self.validate_command_paths(cmd, cwd)?;

        let mut command = tokio::process::Command::new("sh");
        // Defense in depth: start the shell with pathname expansion disabled.
        command.arg("-f").arg("-c").arg(cmd).current_dir(cwd);
        let output = if let Some(limit) = timeout {
            match tokio::time::timeout(limit, command.output()).await {
                Ok(result) => result.map_err(|e| anyhow!("Failed to execute command: {}", e))?,
                Err(_) => {
                    return Err(anyhow!(
                        "Command execution timed out after {}s",
                        limit.as_secs()
                    ));
                }
            }
        } else {
            command
                .output()
                .await
                .map_err(|e| anyhow!("Failed to execute command: {}", e))?
        };

        Ok(ExecResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    /// List directory contents
    pub async fn list_dir(&self, path: &Path) -> Result<Vec<tokio::fs::DirEntry>> {
        if !self.is_in_workspace(path) {
            return Err(anyhow!(
                "Path outside workspace: {} (workspace: {})",
                path.display(),
                self.workspace_root.display()
            ));
        }

        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = dir.next_entry().await? {
            entries.push(entry);
        }
        Ok(entries)
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        Ok(dunce::canonicalize(path)?)
    }

    fn validate_command_paths(&self, cmd: &str, cwd: &Path) -> Result<()> {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Command cannot be empty"));
        }

        let tokens = shell_word_tokens(trimmed)?;
        let commands = shell_commands(trimmed)?;
        self.validate_nested_command_evaluators(&commands)?;

        for token in tokens {
            for candidate in path_like_subtokens(&token) {
                self.validate_command_path_candidate(candidate, cwd)?;
            }
        }

        let regex = Regex::new(r"/[A-Za-z0-9._/-]+").expect("absolute-path regex is valid");
        for matched in regex.find_iter(trimmed) {
            let start = matched.start();
            if start > 0 {
                let prev = trimmed.as_bytes()[start - 1];
                if prev == b':'
                    || prev == b'.'
                    || prev == b'/'
                    || prev == b'_'
                    || prev == b'-'
                    || prev.is_ascii_alphanumeric()
                {
                    // Skip URL fragments and path segments within relative paths or identifiers.
                    continue;
                }
            }
            let literal = matched.as_str();
            if is_allowed_absolute_command_path(Path::new(literal)) {
                continue;
            }
            if !self.is_in_workspace(Path::new(literal)) {
                return Err(anyhow!(
                    "Command contains absolute path outside workspace: {}",
                    literal
                ));
            }
            self.ensure_path_not_protected(Path::new(literal), "process path reference")?;
        }

        Ok(())
    }

    fn validate_shell_features(&self, cmd: &str) -> Result<()> {
        if contains_shell_expansion(cmd)
            || contains_shell_brace_expansion(cmd)
            || contains_shell_globbing(cmd)
        {
            return Err(anyhow!(
                "Sandbox backend {} rejects shell variable, command, brace, or glob expansion because path references cannot be validated safely",
                self.backend_name()
            ));
        }
        Ok(())
    }

    fn validate_command_path_candidate(&self, token: &str, cwd: &Path) -> Result<()> {
        if token.is_empty() || token.starts_with('-') {
            return Ok(());
        }

        if token.starts_with('~') {
            return Err(anyhow!(
                "Command references HOME paths outside workspace: {}",
                token
            ));
        }

        if token.contains("://") {
            return Ok(());
        }

        if looks_like_path_token(token) || looks_like_bare_protected_subpath_token(token) {
            let candidate = if Path::new(token).is_absolute() {
                PathBuf::from(token)
            } else {
                cwd.join(token)
            };
            if candidate.is_absolute() && is_allowed_absolute_command_path(&candidate) {
                return Ok(());
            }
            if !self.is_in_workspace(&candidate) {
                return Err(anyhow!(
                    "Command references path outside workspace: {}",
                    token
                ));
            }
            self.ensure_path_not_protected(&candidate, "process path reference")?;
        }

        Ok(())
    }

    fn validate_nested_command_evaluators(&self, commands: &[Vec<String>]) -> Result<()> {
        for words in commands {
            let Some(view) = nested_evaluator_view(words) else {
                continue;
            };
            if let Some(display) = view.opaque_wrapper_display.as_deref() {
                return Err(anyhow!(
                    "Sandbox backend {} rejects nested command evaluators like {} because inner paths cannot be validated safely",
                    self.backend_name(),
                    display
                ));
            }
            if is_shell_eval_builtin(view.command) {
                return Err(anyhow!(
                    "Sandbox backend {} rejects nested command evaluators like {} because inner paths cannot be validated safely",
                    self.backend_name(),
                    view.display
                ));
            }
            if let Some(flag) = leading_eval_flag(view.command, view.args) {
                return Err(anyhow!(
                    "Sandbox backend {} rejects nested command evaluators like {} {} because inner paths cannot be validated safely",
                    self.backend_name(),
                    view.display,
                    flag
                ));
            }
        }
        Ok(())
    }

    fn ensure_path_not_protected(&self, path: &Path, action: &str) -> Result<()> {
        if let Some(component) = self.protected_subpath_component(path) {
            return Err(anyhow!(
                "Sandbox backend {} blocks {} under protected subpath {}: {}",
                self.backend_name(),
                action,
                component,
                path.display()
            ));
        }
        Ok(())
    }

    fn protected_subpath_component(&self, path: &Path) -> Option<&'static str> {
        let canonical_workspace = self
            .canonicalize(&self.workspace_root)
            .unwrap_or_else(|_| lexically_normalize_path(&self.workspace_root));
        let resolved_path = self.resolved_path_with_existing_parents(path);
        let relative = resolved_path.strip_prefix(&canonical_workspace).ok()?;
        relative.components().find_map(|component| match component {
            Component::Normal(name) => {
                let candidate = name.to_str()?;
                PROTECTED_SUBPATHS
                    .iter()
                    .copied()
                    .find(|protected| *protected == candidate)
            }
            _ => None,
        })
    }

    fn normalized_path(&self, path: &Path) -> PathBuf {
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };
        if absolute_path.exists() {
            self.canonicalize(&absolute_path)
                .unwrap_or_else(|_| lexically_normalize_path(&absolute_path))
        } else {
            lexically_normalize_path(&absolute_path)
        }
    }

    fn resolved_path_with_existing_parents(&self, path: &Path) -> PathBuf {
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };
        if absolute_path.exists() {
            return self.normalized_path(&absolute_path);
        }

        let mut current = absolute_path.as_path();
        let mut suffix = Vec::<OsString>::new();
        while !current.exists() {
            let Some(name) = current.file_name() else {
                return lexically_normalize_path(&absolute_path);
            };
            suffix.push(name.to_os_string());
            let Some(parent) = current.parent() else {
                return lexically_normalize_path(&absolute_path);
            };
            current = parent;
        }

        let mut resolved = self
            .canonicalize(current)
            .unwrap_or_else(|_| lexically_normalize_path(current));
        for component in suffix.iter().rev() {
            resolved.push(component);
        }
        resolved
    }
}

fn looks_like_path_token(token: &str) -> bool {
    token.starts_with('/')
        || token.starts_with("./")
        || token.starts_with("../")
        || token == "."
        || token == ".."
        || token.contains('/')
}

fn looks_like_bare_protected_subpath_token(token: &str) -> bool {
    PROTECTED_SUBPATHS
        .iter()
        .copied()
        .any(|protected| token.trim_end_matches('/') == protected)
}

fn path_like_subtokens(token: &str) -> Vec<&str> {
    let mut candidates = vec![token];
    if let Some((_, rhs)) = token.rsplit_once('=')
        && !rhs.is_empty()
    {
        candidates.push(rhs);
    }
    candidates
}

fn is_allowed_absolute_command_path(path: &Path) -> bool {
    matches!(
        path.to_str(),
        Some("/dev/null" | "/dev/stdin" | "/dev/stdout" | "/dev/stderr")
    )
}

fn lexically_normalize_path(path: &Path) -> PathBuf {
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

fn contains_shell_expansion(command: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in command.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        if in_double {
            match ch {
                '\\' => escaped = true,
                '"' => in_double = false,
                '$' | '`' => return true,
                _ => {}
            }
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '\'' => in_single = true,
            '"' => in_double = true,
            '$' | '`' => return true,
            _ => {}
        }
    }

    false
}

fn contains_shell_brace_expansion(command: &str) -> bool {
    let chars: Vec<char> = command.chars().collect();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (index, ch) in chars.iter().copied().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }

        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        if in_double {
            match ch {
                '\\' => escaped = true,
                '"' => in_double = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '\'' => in_single = true,
            '"' => in_double = true,
            '{' | '}' if is_brace_expansion_position(&chars, index) => return true,
            _ => {}
        }
    }

    false
}

fn contains_shell_globbing(command: &str) -> bool {
    let chars: Vec<char> = command.chars().collect();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (index, ch) in chars.iter().copied().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }

        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        if in_double {
            match ch {
                '\\' => escaped = true,
                '"' => in_double = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '\'' => in_single = true,
            '"' => in_double = true,
            '*' | '?' => return true,
            '[' if !is_test_bracket_token(&chars, index) => return true,
            _ => {}
        }
    }

    false
}

fn is_test_bracket_token(chars: &[char], index: usize) -> bool {
    let mut end = index;
    while let Some(ch) = chars.get(end) {
        if ch.is_whitespace() || is_shell_separator(*ch) {
            break;
        }
        end += 1;
    }

    match end.saturating_sub(index) {
        1 => chars[index] == '[',
        2 => chars[index] == '[' && chars.get(index + 1).copied() == Some('['),
        _ => false,
    }
}

fn is_brace_expansion_position(chars: &[char], index: usize) -> bool {
    let prev = index.checked_sub(1).and_then(|i| chars.get(i)).copied();
    let next = chars.get(index + 1).copied();
    brace_neighbor_requires_expansion(prev) || brace_neighbor_requires_expansion(next)
}

fn brace_neighbor_requires_expansion(ch: Option<char>) -> bool {
    matches!(ch, Some(value) if !value.is_whitespace() && !is_shell_separator(value))
}

fn is_shell_separator(ch: char) -> bool {
    matches!(ch, ';' | '|' | '&' | '(' | ')' | '<' | '>')
}

fn shell_word_tokens(command: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if in_single {
            if ch == '\'' {
                in_single = false;
            } else {
                current.push(ch);
            }
            continue;
        }

        if in_double {
            match ch {
                '\\' => {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    } else {
                        return Err(anyhow!("Command ends with an incomplete escape sequence"));
                    }
                }
                '"' => in_double = false,
                _ => current.push(ch),
            }
            continue;
        }

        match ch {
            '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                } else {
                    return Err(anyhow!("Command ends with an incomplete escape sequence"));
                }
            }
            '\'' => in_single = true,
            '"' => in_double = true,
            c if c.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            ';' | '(' | ')' | '{' | '}' => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            '&' | '|' | '<' | '>' => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }

                if matches!(chars.peek(), Some(next) if *next == ch) {
                    chars.next();
                }
            }
            _ => current.push(ch),
        }
    }

    if escaped {
        return Err(anyhow!("Command ends with an incomplete escape sequence"));
    }
    if in_single || in_double {
        return Err(anyhow!("Command contains an unterminated quoted string"));
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

fn shell_commands(command: &str) -> Result<Vec<Vec<String>>> {
    let mut commands = Vec::new();
    let mut current_command = Vec::new();
    let mut current_word = String::new();
    let mut chars = command.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if escaped {
            current_word.push(ch);
            escaped = false;
            continue;
        }

        if in_single {
            if ch == '\'' {
                in_single = false;
            } else {
                current_word.push(ch);
            }
            continue;
        }

        if in_double {
            match ch {
                '\\' => {
                    if let Some(next) = chars.next() {
                        current_word.push(next);
                    } else {
                        return Err(anyhow!("Command ends with an incomplete escape sequence"));
                    }
                }
                '"' => in_double = false,
                _ => current_word.push(ch),
            }
            continue;
        }

        match ch {
            '\\' => {
                if let Some(next) = chars.next() {
                    current_word.push(next);
                } else {
                    return Err(anyhow!("Command ends with an incomplete escape sequence"));
                }
            }
            '\'' => in_single = true,
            '"' => in_double = true,
            c if c.is_whitespace() => {
                if !current_word.is_empty() {
                    current_command.push(std::mem::take(&mut current_word));
                }
            }
            ';' | '|' | '&' | '(' | ')' | '{' | '}' => {
                if !current_word.is_empty() {
                    current_command.push(std::mem::take(&mut current_word));
                }
                if !current_command.is_empty() {
                    commands.push(std::mem::take(&mut current_command));
                }
                if matches!(chars.peek(), Some(next) if *next == ch && matches!(ch, '|' | '&')) {
                    chars.next();
                }
            }
            _ => current_word.push(ch),
        }
    }

    if escaped {
        return Err(anyhow!("Command ends with an incomplete escape sequence"));
    }
    if in_single || in_double {
        return Err(anyhow!("Command contains an unterminated quoted string"));
    }
    if !current_word.is_empty() {
        current_command.push(current_word);
    }
    if !current_command.is_empty() {
        commands.push(current_command);
    }

    Ok(commands)
}

fn command_basename(command: &str) -> &str {
    Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(command)
}

struct NestedEvaluatorView<'a> {
    display: String,
    command: &'a str,
    args: &'a [String],
    opaque_wrapper_display: Option<String>,
}

fn nested_evaluator_view(words: &[String]) -> Option<NestedEvaluatorView<'_>> {
    let mut command_index = next_command_offset(words)?;
    let mut display = command_basename(&words[command_index]).to_string();

    loop {
        let command = command_basename(&words[command_index]);
        let args = &words[command_index + 1..];
        let next_offset = if command == "env" {
            if let Some(flag) = env_split_string_flag(args) {
                return Some(NestedEvaluatorView {
                    display: display.clone(),
                    command,
                    args,
                    opaque_wrapper_display: Some(format!("{display} {flag}")),
                });
            }
            env_command_offset(args)
        } else if is_transparent_command_wrapper(command) {
            transparent_wrapper_offset(command, args)
        } else {
            None
        };

        let Some(next_relative_offset) = next_offset else {
            return Some(NestedEvaluatorView {
                display,
                command,
                args,
                opaque_wrapper_display: None,
            });
        };

        command_index += 1 + next_relative_offset;
        display.push(' ');
        display.push_str(command_basename(&words[command_index]));
    }
}

fn next_command_offset(words: &[String]) -> Option<usize> {
    words.iter().position(|word| !is_env_assignment(word))
}

fn env_command_offset(args: &[String]) -> Option<usize> {
    let mut index = 0;
    while let Some(arg) = args.get(index).map(|arg| arg.as_str()) {
        if arg == "--" {
            index += 1;
            break;
        }
        if is_env_assignment(arg) {
            index += 1;
            continue;
        }
        match env_option_behavior(arg) {
            Some(
                EnvOptionBehavior::Passthrough
                | EnvOptionBehavior::InlineValue
                | EnvOptionBehavior::SplitStringInlineValue,
            ) => {
                index += 1;
                continue;
            }
            Some(EnvOptionBehavior::TakesNextArg | EnvOptionBehavior::SplitStringNextArg) => {
                index += 2;
                continue;
            }
            None => {}
        }
        break;
    }

    args.get(index)?;
    Some(index)
}

fn transparent_wrapper_offset(command: &str, args: &[String]) -> Option<usize> {
    match command {
        "command" => command_wrapper_offset(args),
        "exec" => exec_wrapper_offset(args),
        "builtin" => builtin_wrapper_offset(args),
        _ => None,
    }
}

fn command_wrapper_offset(args: &[String]) -> Option<usize> {
    let mut index = 0;
    while let Some(arg) = args.get(index).map(|arg| arg.as_str()) {
        if arg == "--" {
            index += 1;
            break;
        }
        if command_wrapper_is_query_flag(arg) {
            return None;
        }
        if command_wrapper_is_exec_flag(arg) {
            index += 1;
            continue;
        }
        break;
    }

    args.get(index)?;
    Some(index)
}

fn builtin_wrapper_offset(args: &[String]) -> Option<usize> {
    let mut index = 0;
    if let Some(arg) = args.get(index).map(|arg| arg.as_str()) {
        if arg == "--" {
            index += 1;
        } else if builtin_query_flag(arg) {
            return None;
        }
    }

    args.get(index)?;
    Some(index)
}

fn exec_wrapper_offset(args: &[String]) -> Option<usize> {
    let mut index = 0;
    while let Some(arg) = args.get(index).map(|arg| arg.as_str()) {
        if arg == "--" {
            index += 1;
            break;
        }
        if arg == "-a" {
            index += 2;
            continue;
        }
        if has_inline_exec_argv0(arg) || is_exec_wrapper_flag(arg) {
            index += 1;
            continue;
        }
        break;
    }

    args.get(index)?;
    Some(index)
}

fn is_env_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_shell_eval_builtin(command: &str) -> bool {
    matches!(command, "eval" | "." | "source")
}

fn is_transparent_command_wrapper(command: &str) -> bool {
    matches!(command, "command" | "builtin" | "exec")
}

fn env_split_string_flag(args: &[String]) -> Option<&str> {
    let mut index = 0;
    while let Some(arg) = args.get(index).map(|arg| arg.as_str()) {
        if arg == "--" {
            return None;
        }
        if is_env_assignment(arg) {
            index += 1;
            continue;
        }
        match env_option_behavior(arg) {
            Some(
                EnvOptionBehavior::SplitStringInlineValue | EnvOptionBehavior::SplitStringNextArg,
            ) => return Some(arg),
            Some(EnvOptionBehavior::Passthrough | EnvOptionBehavior::InlineValue) => {
                index += 1;
                continue;
            }
            Some(EnvOptionBehavior::TakesNextArg) => {
                index += 2;
                continue;
            }
            None => {}
        }
        break;
    }
    None
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EnvOptionBehavior {
    Passthrough,
    TakesNextArg,
    InlineValue,
    SplitStringNextArg,
    SplitStringInlineValue,
}

fn env_option_behavior(arg: &str) -> Option<EnvOptionBehavior> {
    if matches!(arg, "--ignore-environment" | "--null") {
        return Some(EnvOptionBehavior::Passthrough);
    }
    if arg == "--split-string" {
        return Some(EnvOptionBehavior::SplitStringNextArg);
    }
    if arg.starts_with("--split-string=") {
        return Some(EnvOptionBehavior::SplitStringInlineValue);
    }
    if matches!(arg, "--unset" | "--chdir") {
        return Some(EnvOptionBehavior::TakesNextArg);
    }
    if arg.starts_with("--unset=") || arg.starts_with("--chdir=") {
        return Some(EnvOptionBehavior::InlineValue);
    }
    env_short_option_behavior(arg)
}

fn env_short_option_behavior(arg: &str) -> Option<EnvOptionBehavior> {
    if arg.starts_with("--") {
        return None;
    }
    let rest = arg.strip_prefix('-')?;
    if rest.is_empty() {
        return None;
    }

    let mut saw_passthrough = false;
    for (index, ch) in rest.char_indices() {
        match ch {
            'i' | '0' => saw_passthrough = true,
            'u' | 'C' => {
                return Some(if rest[index + ch.len_utf8()..].is_empty() {
                    EnvOptionBehavior::TakesNextArg
                } else {
                    EnvOptionBehavior::InlineValue
                });
            }
            'S' => {
                return Some(if rest[index + ch.len_utf8()..].is_empty() {
                    EnvOptionBehavior::SplitStringNextArg
                } else {
                    EnvOptionBehavior::SplitStringInlineValue
                });
            }
            _ => return None,
        }
    }

    saw_passthrough.then_some(EnvOptionBehavior::Passthrough)
}

fn command_wrapper_is_exec_flag(arg: &str) -> bool {
    let Some(rest) = arg.strip_prefix('-') else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(|ch| ch == 'p')
}

fn command_wrapper_is_query_flag(arg: &str) -> bool {
    let Some(rest) = arg.strip_prefix('-') else {
        return false;
    };
    !rest.is_empty()
        && rest.chars().all(|ch| matches!(ch, 'p' | 'v' | 'V'))
        && rest.chars().any(|ch| matches!(ch, 'v' | 'V'))
}

fn builtin_query_flag(arg: &str) -> bool {
    arg == "-p"
}

fn is_exec_wrapper_flag(arg: &str) -> bool {
    let Some(rest) = arg.strip_prefix('-') else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(|ch| matches!(ch, 'c' | 'l'))
}

fn has_inline_exec_argv0(arg: &str) -> bool {
    arg.starts_with("-a") && arg.len() > 2
}

fn is_shell_eval_wrapper(command: &str, flag: &str) -> bool {
    matches!(command, "sh" | "bash" | "dash" | "zsh" | "ksh")
        && shell_flag_contains_short_option(flag, 'c')
}

fn is_code_eval_wrapper(command: &str, flag: &str) -> bool {
    match command {
        "python" | "python3" => shell_flag_contains_short_option(flag, 'c'),
        "node" => {
            shell_flag_contains_short_option(flag, 'e')
                || shell_flag_contains_short_option(flag, 'p')
                || flag == "--print"
        }
        "perl" => {
            shell_flag_contains_short_option(flag, 'e')
                || shell_flag_contains_short_option(flag, 'E')
        }
        "ruby" | "lua" => shell_flag_contains_short_option(flag, 'e'),
        "php" => shell_flag_contains_short_option(flag, 'r'),
        _ => false,
    }
}

fn leading_eval_flag<'a>(command: &str, args: &'a [String]) -> Option<&'a str> {
    match command {
        "sh" | "bash" | "dash" | "zsh" | "ksh" => scan_leading_args(
            args,
            |arg| is_shell_eval_wrapper("sh", arg),
            shell_wrapper_advance,
        ),
        "python" | "python3" => scan_leading_args(
            args,
            |arg| is_code_eval_wrapper("python3", arg),
            python_wrapper_advance,
        ),
        "node" => scan_leading_args(
            args,
            |arg| is_code_eval_wrapper("node", arg),
            node_wrapper_advance,
        ),
        "perl" => scan_leading_args(
            args,
            |arg| is_code_eval_wrapper("perl", arg),
            perl_wrapper_advance,
        ),
        "ruby" => scan_leading_args(
            args,
            |arg| is_code_eval_wrapper("ruby", arg),
            ruby_wrapper_advance,
        ),
        "lua" => scan_leading_args(
            args,
            |arg| is_code_eval_wrapper("lua", arg),
            lua_wrapper_advance,
        ),
        "php" => scan_leading_args(
            args,
            |arg| is_code_eval_wrapper("php", arg),
            php_wrapper_advance,
        ),
        _ => None,
    }
}

fn scan_leading_args<F, G>(args: &[String], matches_eval: F, advance: G) -> Option<&str>
where
    F: Fn(&str) -> bool,
    G: Fn(&str) -> Option<usize>,
{
    let mut index = 0;
    while let Some(arg) = args.get(index).map(|arg| arg.as_str()) {
        if arg == "--" {
            break;
        }
        if matches_eval(arg) {
            return Some(arg);
        }
        index += advance(arg)?;
    }
    None
}

fn shell_wrapper_advance(arg: &str) -> Option<usize> {
    if exact_or_inline_option_with_value(
        arg,
        &["-o", "+o", "-O", "+O"],
        &["--rcfile", "--init-file"],
    ) {
        Some(if has_attached_option_value(arg) { 1 } else { 2 })
    } else if arg.starts_with('-') || arg.starts_with('+') {
        Some(1)
    } else {
        None
    }
}

fn python_wrapper_advance(arg: &str) -> Option<usize> {
    if exact_or_inline_option_with_value(arg, &["-W", "-X"], &["--check-hash-based-pycs"]) {
        Some(if has_attached_option_value(arg) { 1 } else { 2 })
    } else if matches!(arg, "-m" | "--module" | "-") {
        None
    } else if arg.starts_with('-') {
        Some(1)
    } else {
        None
    }
}

fn node_wrapper_advance(arg: &str) -> Option<usize> {
    if exact_or_inline_option_with_value(
        arg,
        &["-r", "-C"],
        &[
            "--require",
            "--loader",
            "--experimental-loader",
            "--import",
            "--watch-path",
            "--conditions",
            "--input-type",
            "--inspect",
            "--inspect-brk",
            "--inspect-port",
            "--openssl-config",
            "--redirect-warnings",
            "--trace-event-categories",
            "--trace-event-file-pattern",
            "--diagnostic-dir",
            "--icu-data-dir",
            "--title",
        ],
    ) {
        Some(if has_attached_option_value(arg) { 1 } else { 2 })
    } else if arg.starts_with('-') {
        Some(1)
    } else {
        None
    }
}

fn perl_wrapper_advance(arg: &str) -> Option<usize> {
    if exact_or_inline_option_with_value(arg, &["-I", "-M", "-m"], &[]) {
        Some(if has_attached_option_value(arg) { 1 } else { 2 })
    } else if arg.starts_with('-') {
        Some(1)
    } else {
        None
    }
}

fn ruby_wrapper_advance(arg: &str) -> Option<usize> {
    if exact_or_inline_option_with_value(
        arg,
        &["-C", "-E", "-F", "-I", "-r"],
        &["--enable", "--disable", "--encoding"],
    ) {
        Some(if has_attached_option_value(arg) { 1 } else { 2 })
    } else if arg.starts_with('-') {
        Some(1)
    } else {
        None
    }
}

fn lua_wrapper_advance(arg: &str) -> Option<usize> {
    if exact_or_inline_option_with_value(arg, &["-l"], &[]) {
        Some(if has_attached_option_value(arg) { 1 } else { 2 })
    } else if arg.starts_with('-') {
        Some(1)
    } else {
        None
    }
}

fn php_wrapper_advance(arg: &str) -> Option<usize> {
    if exact_or_inline_option_with_value(arg, &["-c", "-d", "-z"], &["--define"]) {
        Some(if has_attached_option_value(arg) { 1 } else { 2 })
    } else if matches!(arg, "-f" | "--file") {
        None
    } else if arg.starts_with('-') {
        Some(1)
    } else {
        None
    }
}

fn exact_or_inline_option_with_value(arg: &str, short: &[&str], long: &[&str]) -> bool {
    short
        .iter()
        .any(|flag| arg == *flag || arg.starts_with(flag))
        || long
            .iter()
            .any(|flag| arg == *flag || arg.starts_with(&format!("{flag}=")))
}

fn has_attached_option_value(arg: &str) -> bool {
    arg.contains('=') || (arg.starts_with('-') && !arg.starts_with("--") && arg.len() > 2)
}

fn shell_flag_contains_short_option(flag: &str, option: char) -> bool {
    if let Some(rest) = flag.strip_prefix("--") {
        return matches!(
            (rest, option),
            ("command", 'c') | ("eval", 'e') | ("run", 'r')
        );
    }

    flag.starts_with('-') && flag.chars().skip(1).any(|ch| ch == option)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_sandbox_read_write() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        // Write a file
        let file_path = temp.path().join("test.txt");
        sandbox.write(&file_path, b"hello world").await.unwrap();

        // Read it back
        let content = sandbox.read_string(&file_path).await.unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_sandbox_blocks_outside_workspace() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        // Try to read outside workspace
        let outside_path = PathBuf::from("/etc/passwd");
        let result = sandbox.read(&outside_path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sandbox_exec() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox.exec("echo hello", temp.path()).await.unwrap();
        assert_eq!(result.stdout.trim(), "hello");
        assert_eq!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_outside_workspace_path_reference() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox.exec("cat /etc/passwd", temp.path()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sandbox_exec_allows_workspace_relative_paths() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let file = temp.path().join("in_workspace.txt");
        tokio::fs::write(&file, "ok").await.unwrap();

        let result = sandbox.exec("cat ./in_workspace.txt", temp.path()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().stdout.trim(), "ok");
    }

    #[tokio::test]
    async fn test_sandbox_exec_allows_dev_null_redirection() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox.exec("echo ok > /dev/null", temp.path()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().exit_code, 0);
    }

    #[tokio::test]
    async fn test_sandbox_blocks_write_to_protected_subpath() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected = temp.path().join(".git/config");
        tokio::fs::create_dir_all(protected.parent().unwrap())
            .await
            .unwrap();

        let result = sandbox.write(&protected, b"[core]\n").await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("protected subpath .git")
        );
    }

    #[tokio::test]
    async fn test_sandbox_allows_read_from_protected_subpath() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected = temp.path().join(".alan/policy.yaml");
        tokio::fs::create_dir_all(protected.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&protected, "rules: []\n").await.unwrap();

        let result = sandbox.read_string(&protected).await;
        assert_eq!(result.unwrap(), "rules: []\n");
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_mutating_command_for_protected_path() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected = temp.path().join(".alan/config.toml");
        tokio::fs::create_dir_all(protected.parent().unwrap())
            .await
            .unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "touch .alan/config.toml",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("protected subpath .alan")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_read_only_command_for_protected_path() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected = temp.path().join(".git/HEAD");
        tokio::fs::create_dir_all(protected.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&protected, "ref: refs/heads/main\n")
            .await
            .unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "cat .git/HEAD",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Read),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("protected subpath .git")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_mutating_cwd_inside_protected_subpath() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected_dir = temp.path().join(".agents");
        tokio::fs::create_dir_all(&protected_dir).await.unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "touch state.txt",
                &protected_dir,
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("protected subpath .agents")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_bare_protected_directory_token() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected_dir = temp.path().join(".git");
        tokio::fs::create_dir_all(&protected_dir).await.unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "rm -rf .git",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("protected subpath .git")
        );
    }

    #[tokio::test]
    async fn test_sandbox_blocks_symlink_alias_into_protected_subpath() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected_dir = temp.path().join(".git");
        tokio::fs::create_dir_all(&protected_dir).await.unwrap();
        let alias = temp.path().join("safe");
        std::os::unix::fs::symlink(&protected_dir, &alias).unwrap();

        let result = sandbox.write(&alias.join("config"), b"[core]\n").await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("protected subpath .git")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_mutating_variable_expansion() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected_dir = temp.path().join(".git");
        tokio::fs::create_dir_all(&protected_dir).await.unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "d=.git && rm -rf \"$d\"",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects shell variable, command, brace, or glob expansion")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_globbed_process_paths() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected_dir = temp.path().join(".git");
        tokio::fs::create_dir_all(&protected_dir).await.unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "rm -rf .g*",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Read),
            )
            .await;
        assert!(result.is_err());
        assert!(protected_dir.exists());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects shell variable, command, brace, or glob expansion")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_set_plus_f_glob_bypass_attempt() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected_dir = temp.path().join(".git");
        tokio::fs::create_dir_all(&protected_dir).await.unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "set +f; rm -rf .g*",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(protected_dir.exists());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects shell variable, command, brace, or glob expansion")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_read_only_variable_expansion() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "f=/etc/passwd && cat \"$f\"",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Read),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects shell variable, command, brace, or glob expansion")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_brace_expansion() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "rm -rf .{git,alan}",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects shell variable, command, brace, or glob expansion")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_nested_shell_eval_wrapper() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "sh -c 'rm -rf .git'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like sh -c")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_nested_python_eval_wrapper() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "python3 -c 'print(\"hi\")'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Read),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like python3 -c")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_shell_eval_wrapper_with_leading_option() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "bash --noprofile -c 'rm -rf .git'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like bash -c")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_python_eval_wrapper_with_leading_option() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "python3 -B -c 'open(\".git/config\", \"w\").write(\"x\")'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like python3 -c")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_node_print_eval_wrapper_with_leading_option() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "node --trace-warnings -p 'require(\"fs\").writeFileSync(\".git/config\", \"x\")'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like node -p")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_allows_literal_sh_dash_c_arguments() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "printf '%s %s' sh -c",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Read),
            )
            .await
            .unwrap();

        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "sh -c");
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_eval_builtin() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "eval 'rm -rf .git'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like eval")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_command_eval_builtin() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "command eval 'rm -rf .git'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like command eval")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_source_builtin() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                ". ./script.sh",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Read),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like .")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_env_shell_eval_wrapper() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "env FOO=bar sh -c 'rm -rf .git'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like env sh -c")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_env_split_string_wrapper() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "env -S 'sh -c rm -rf .git'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like env -S")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_clustered_env_split_string_wrapper() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "env -iS 'sh -c rm -rf .git'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like env -iS")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_command_wrapper_with_leading_option() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "command -p sh -c 'rm -rf .git'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like command sh -c")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_allows_command_query_mode_with_eval_like_argv() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "command -v sh -c",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Read),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_builtin_eval_after_end_of_options() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "builtin -- eval 'rm -rf .git'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like builtin eval")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_exec_shell_eval_wrapper() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "exec sh -c 'rm -rf .git'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like exec sh -c")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_exec_shell_eval_wrapper_with_argv0_option() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());

        let result = sandbox
            .exec_with_timeout_and_capability(
                "exec -a alan sh -c 'rm -rf .git'",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("rejects nested command evaluators like exec sh -c")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_allows_bracket_test_syntax() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        tokio::fs::write(temp.path().join("README.md"), "ok")
            .await
            .unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "[ -f README.md ]",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Read),
            )
            .await
            .unwrap();

        assert_eq!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_protected_redirection_without_whitespace() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected_dir = temp.path().join(".git");
        tokio::fs::create_dir_all(&protected_dir).await.unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "echo x>.git/config",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("protected subpath .git")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_protected_path_built_from_quoted_segments() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected_dir = temp.path().join(".git");
        tokio::fs::create_dir_all(&protected_dir).await.unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "rm -rf .g''it",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("protected subpath .git")
        );
    }

    #[tokio::test]
    async fn test_sandbox_exec_blocks_protected_path_in_option_assignment() {
        let temp = TempDir::new().unwrap();
        let sandbox = Sandbox::new(temp.path().to_path_buf());
        let protected_dir = temp.path().join(".git");
        tokio::fs::create_dir_all(&protected_dir).await.unwrap();

        let result = sandbox
            .exec_with_timeout_and_capability(
                "git --git-dir=.git config alan.test true",
                temp.path(),
                None,
                Some(alan_protocol::ToolCapability::Write),
            )
            .await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("protected subpath .git")
        );
    }
}
