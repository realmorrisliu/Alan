use crate::daemon::auth_control::AuthControlState;
use crate::daemon::connection_control::{
    ConnectionControlState, ConnectionCredentialStatus, ConnectionCurrentState, ConnectionPinScope,
    ConnectionPinState, ConnectionProfileSummary,
};
use alan_auth::ChatgptAuthConfig;
use alan_runtime::{AlanHomePaths, CredentialKind, LlmProvider, sanitize_identifier};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

fn connection_control() -> Result<Arc<ConnectionControlState>> {
    let home_paths = AlanHomePaths::detect().context("Cannot determine Alan home directory")?;
    let auth_manager = alan_auth::ChatgptAuthManager::new(ChatgptAuthConfig::with_storage_path(
        home_paths.alan_home_dir.join("auth.json"),
    ))?;
    let auth_control = Arc::new(AuthControlState::new(auth_manager, false));
    Ok(ConnectionControlState::new(home_paths, auth_control))
}

fn parse_provider_id(raw: &str) -> Result<LlmProvider> {
    match raw.trim() {
        "chatgpt" => Ok(LlmProvider::Chatgpt),
        "google_gemini_generate_content" => Ok(LlmProvider::GoogleGeminiGenerateContent),
        "openai_responses" => Ok(LlmProvider::OpenAiResponses),
        "openai_chat_completions" => Ok(LlmProvider::OpenAiChatCompletions),
        "openai_chat_completions_compatible" => Ok(LlmProvider::OpenAiChatCompletionsCompatible),
        "anthropic_messages" => Ok(LlmProvider::AnthropicMessages),
        other => anyhow::bail!("unknown provider `{other}`"),
    }
}

fn parse_setting_pairs(pairs: &[String]) -> Result<BTreeMap<String, String>> {
    let mut settings = BTreeMap::new();
    for pair in pairs {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("invalid setting `{pair}`; expected key=value"))?;
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() {
            anyhow::bail!("invalid setting `{pair}`; key cannot be empty");
        }
        settings.insert(key.to_string(), value.to_string());
    }
    Ok(settings)
}

fn print_profile(profile: &ConnectionProfileSummary) {
    println!("profile_id: {}", profile.profile_id);
    if let Some(label) = profile.label.as_deref() {
        println!("label: {label}");
    }
    println!("provider: {}", profile.provider.as_str());
    if let Some(credential_id) = profile.credential_id.as_deref() {
        println!("credential_id: {credential_id}");
    }
    println!("credential_status: {:?}", profile.credential_status);
    println!("default: {}", profile.is_default);
    println!("source: {}", profile.source);
    if profile.settings.is_empty() {
        println!("settings: <none>");
    } else {
        println!("settings:");
        for (key, value) in &profile.settings {
            println!("  {key}={value}");
        }
    }
}

fn print_credential_status(status: &ConnectionCredentialStatus) {
    println!("credential:");
    println!("  status: {:?}", status.status);
    println!("  kind: {}", status.credential_kind.as_str());
    if let Some(credential_id) = status.credential_id.as_deref() {
        println!("  credential_id: {credential_id}");
    }
    if let Some(detail) = status.detail.as_ref() {
        if let Some(email) = detail.account_email.as_deref() {
            println!("  email: {email}");
        }
        if let Some(plan) = detail.account_plan.as_deref() {
            println!("  plan: {plan}");
        }
        if let Some(message) = detail.message.as_deref() {
            println!("  detail: {message}");
        }
    }
}

fn print_pin_state(label: &str, pin: Option<&ConnectionPinState>) {
    match pin {
        Some(pin) => println!(
            "{label}: {} ({}) [{}]",
            pin.profile_id,
            pin.scope.as_str(),
            pin.config_path.display()
        ),
        None => println!("{label}: <unset>"),
    }
}

fn print_current_state(current: &ConnectionCurrentState) {
    if let Some(workspace_dir) = current.workspace_dir.as_deref() {
        println!("workspace_dir: {}", workspace_dir.display());
    }
    print_pin_state("global_pin", current.global_pin.as_ref());
    print_pin_state("workspace_pin", current.workspace_pin.as_ref());
    match current.default_profile.as_deref() {
        Some(profile_id) => println!("default_profile: {profile_id}"),
        None => println!("default_profile: <unset>"),
    }
    match current.effective_profile.as_deref() {
        Some(profile_id) => println!("effective_profile: {profile_id}"),
        None => println!("effective_profile: <unset>"),
    }
    println!("effective_source: {}", current.effective_source.as_str());
}

fn detect_workspace_dir_from_cwd() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    detect_workspace_dir(&cwd)
}

fn detect_workspace_dir(path: &Path) -> Option<PathBuf> {
    let normalized = std::fs::canonicalize(path)
        .ok()
        .unwrap_or_else(|| path.to_path_buf());
    if normalized
        .file_name()
        .map(|name| name == std::ffi::OsStr::new(".alan"))
        .unwrap_or(false)
        && normalized.is_dir()
    {
        return normalized.parent().map(Path::to_path_buf);
    }

    let alan_dir = normalized.join(".alan");
    if alan_dir.is_dir() {
        return Some(normalized);
    }

    None
}

fn prompt_secret_line(profile_id: &str) -> Result<String> {
    print!("Secret for {profile_id}: ");
    io::stdout().flush()?;
    let mut secret = String::new();
    io::stdin().read_line(&mut secret)?;
    let trimmed = secret.trim().to_string();
    if trimmed.is_empty() {
        anyhow::bail!("secret cannot be empty");
    }
    Ok(trimmed)
}

fn suggested_profile_id(provider: LlmProvider, requested: Option<String>) -> Result<String> {
    if let Some(profile_id) = requested {
        return sanitize_identifier(&profile_id)
            .ok_or_else(|| anyhow::anyhow!("invalid profile id `{profile_id}`"));
    }

    let default = match provider {
        LlmProvider::Chatgpt => "chatgpt-main",
        LlmProvider::OpenAiResponses => "openai-main",
        LlmProvider::OpenAiChatCompletions => "openai-chat",
        LlmProvider::OpenAiChatCompletionsCompatible => "compatible-main",
        LlmProvider::GoogleGeminiGenerateContent => "gemini",
        LlmProvider::AnthropicMessages => "anthropic-main",
    };
    Ok(default.to_string())
}

async fn profile_or_current(explicit_profile_id: Option<String>) -> Result<String> {
    if let Some(profile_id) = explicit_profile_id {
        return Ok(profile_id);
    }
    let control = connection_control()?;
    let workspace_dir = detect_workspace_dir_from_cwd();
    let current = control.current_selection(workspace_dir.as_deref())?;
    if let Some(profile_id) = current.effective_profile {
        return Ok(profile_id);
    }
    anyhow::bail!(
        "no profile selected; pass <profile-id>, set a default with `alan connection default set <profile-id>`, or pin one with `alan connection pin <profile-id>`"
    )
}

fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let status = Command::new("open").arg(url).status();
    #[cfg(target_os = "linux")]
    let status = Command::new("xdg-open").arg(url).status();
    #[cfg(target_os = "windows")]
    let status = Command::new("cmd").args(["/C", "start", "", url]).status();

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let status: std::io::Result<std::process::ExitStatus> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "automatic browser open is not supported on this platform",
    ));

    let status = status.context("failed to start browser opener")?;
    if !status.success() {
        anyhow::bail!("browser opener exited with status {status}");
    }
    Ok(())
}

pub async fn run_connection_list() -> Result<()> {
    let control = connection_control()?;
    let (default_profile, profiles) = control.list_profiles().await?;
    if let Some(default_profile) = default_profile {
        println!("default_profile: {default_profile}");
    }
    if profiles.is_empty() {
        println!("No connection profiles configured.");
        return Ok(());
    }
    for profile in profiles {
        println!(
            "{} | provider={} | credential={:?}{}",
            profile.profile_id,
            profile.provider.as_str(),
            profile.credential_status,
            if profile.is_default { " | default" } else { "" }
        );
    }
    Ok(())
}

pub async fn run_connection_show(profile_id: &str) -> Result<()> {
    let control = connection_control()?;
    let profile = control.get_profile(profile_id).await?;
    let status = control.credential_status(profile_id).await?;
    print_profile(&profile);
    print_credential_status(&status);
    Ok(())
}

pub async fn run_connection_add(
    provider_raw: &str,
    profile_id: Option<String>,
    label: Option<String>,
    credential_id: Option<String>,
    setting_pairs: &[String],
    activate: bool,
) -> Result<()> {
    let control = connection_control()?;
    let provider = parse_provider_id(provider_raw)?;
    let profile_id = suggested_profile_id(provider, profile_id)?;
    let settings = parse_setting_pairs(setting_pairs)?;
    let profile = control
        .create_profile(
            &profile_id,
            label,
            provider,
            credential_id,
            settings,
            activate,
        )
        .await?;
    println!("Created connection profile {}", profile.profile_id);
    print_profile(&profile);
    Ok(())
}

pub async fn run_connection_edit(
    profile_id: &str,
    label: Option<String>,
    credential_id: Option<String>,
    setting_pairs: &[String],
) -> Result<()> {
    let control = connection_control()?;
    let settings = if setting_pairs.is_empty() {
        None
    } else {
        Some(parse_setting_pairs(setting_pairs)?)
    };
    let profile = control
        .update_profile(profile_id, label, credential_id, settings)
        .await?;
    println!("Updated connection profile {}", profile.profile_id);
    print_profile(&profile);
    Ok(())
}

pub async fn run_connection_set_secret(profile_id: &str, value: Option<String>) -> Result<()> {
    let control = connection_control()?;
    let secret = match value {
        Some(value) => value,
        None => prompt_secret_line(profile_id)?,
    };
    let status = control.set_secret(profile_id, &secret).await?;
    println!("Stored secret for {profile_id}");
    print_credential_status(&status);
    Ok(())
}

pub async fn run_connection_login(
    profile_id: &str,
    use_device_code: bool,
    open_browser: bool,
) -> Result<()> {
    let control = connection_control()?;
    let profile = control.get_profile(profile_id).await?;
    if profile.provider != LlmProvider::Chatgpt {
        anyhow::bail!(
            "profile `{profile_id}` uses `{}`; managed login is supported only for chatgpt",
            profile.provider.as_str()
        );
    }

    if use_device_code {
        let start = control.start_device_login(profile_id, None).await?;
        println!("Open this URL in your browser:\n{}", start.verification_url);
        println!();
        println!("Enter this one-time code:\n{}", start.user_code);
        println!();
        let login = control
            .complete_device_login(profile_id, &start.login_id)
            .await?;
        println!("Logged in to {}.", profile.profile_id);
        if let Some(email) = login.email.as_deref() {
            println!("email: {email}");
        }
        if let Some(plan_type) = login.plan_type.as_deref() {
            println!("plan: {plan_type}");
        }
        return Ok(());
    }

    let start = control
        .start_browser_login(profile_id, None, Duration::from_secs(300))
        .await?;
    println!("Browser login started for {}.", profile.profile_id);
    println!("auth_url: {}", start.auth_url);
    if open_browser {
        open_url(&start.auth_url)?;
    }
    println!(
        "Complete the browser flow, then rerun `alan connection show {profile_id}` if needed."
    );
    Ok(())
}

pub async fn run_connection_logout(profile_id: &str) -> Result<()> {
    let control = connection_control()?;
    let profile = control.get_profile(profile_id).await?;
    match profile.provider {
        LlmProvider::Chatgpt => {
            let result = control.logout(profile_id).await?;
            println!(
                "{}",
                if result.removed {
                    format!("Removed managed credentials for {profile_id}.")
                } else {
                    format!("No managed credentials were present for {profile_id}.")
                }
            );
            let status = control.credential_status(profile_id).await?;
            print_credential_status(&status);
        }
        _ => {
            let status = control.credential_status(profile_id).await?;
            if status.credential_kind != CredentialKind::SecretString {
                anyhow::bail!(
                    "provider `{}` does not support logout",
                    profile.provider.as_str()
                );
            }
            let home_paths =
                AlanHomePaths::detect().context("Cannot determine Alan home directory")?;
            let store = alan_runtime::SecretStore::from_home_paths(&home_paths);
            let credential_id = profile
                .credential_id
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("profile `{profile_id}` has no credential"))?;
            let removed = store.delete(credential_id)?;
            println!(
                "{}",
                if removed {
                    format!("Removed secret for {profile_id}.")
                } else {
                    format!("No secret was present for {profile_id}.")
                }
            );
            let refreshed = control.credential_status(profile_id).await?;
            print_credential_status(&refreshed);
        }
    }
    Ok(())
}

pub async fn run_connection_current(workspace: Option<PathBuf>) -> Result<()> {
    let control = connection_control()?;
    let workspace_dir = workspace.as_deref().and_then(detect_workspace_dir);
    let fallback_workspace = if workspace_dir.is_none() {
        detect_workspace_dir_from_cwd()
    } else {
        None
    };
    let current =
        control.current_selection(workspace_dir.as_deref().or(fallback_workspace.as_deref()))?;
    print_current_state(&current);
    Ok(())
}

pub async fn run_connection_default_set(
    profile_id: &str,
    workspace: Option<PathBuf>,
) -> Result<()> {
    let control = connection_control()?;
    let workspace_dir = workspace.as_deref().and_then(detect_workspace_dir);
    let fallback_workspace = if workspace_dir.is_none() {
        detect_workspace_dir_from_cwd()
    } else {
        None
    };
    let current = control
        .set_default_profile(
            profile_id,
            workspace_dir.as_deref().or(fallback_workspace.as_deref()),
        )
        .await?;
    println!("Default profile set to {profile_id}");
    print_current_state(&current);
    Ok(())
}

pub async fn run_connection_default_clear(workspace: Option<PathBuf>) -> Result<()> {
    let control = connection_control()?;
    let workspace_dir = workspace.as_deref().and_then(detect_workspace_dir);
    let fallback_workspace = if workspace_dir.is_none() {
        detect_workspace_dir_from_cwd()
    } else {
        None
    };
    let current = control
        .clear_default_profile(workspace_dir.as_deref().or(fallback_workspace.as_deref()))
        .await?;
    println!("Cleared default profile.");
    print_current_state(&current);
    Ok(())
}

pub async fn run_connection_pin(
    profile_id: &str,
    scope: ConnectionPinScope,
    workspace: Option<PathBuf>,
) -> Result<()> {
    let control = connection_control()?;
    let workspace_dir = workspace.as_deref().and_then(detect_workspace_dir);
    let fallback_workspace = if workspace_dir.is_none() && scope == ConnectionPinScope::Workspace {
        detect_workspace_dir_from_cwd()
    } else {
        None
    };
    let effective_workspace = workspace_dir.as_deref().or(fallback_workspace.as_deref());
    let current = control
        .pin_profile(profile_id, scope, effective_workspace)
        .await?;
    println!("Pinned profile {profile_id} at {} scope.", scope.as_str());
    print_current_state(&current);
    Ok(())
}

pub async fn run_connection_unpin(
    scope: ConnectionPinScope,
    workspace: Option<PathBuf>,
) -> Result<()> {
    let control = connection_control()?;
    let workspace_dir = workspace.as_deref().and_then(detect_workspace_dir);
    let fallback_workspace = if workspace_dir.is_none() && scope == ConnectionPinScope::Workspace {
        detect_workspace_dir_from_cwd()
    } else {
        None
    };
    let effective_workspace = workspace_dir.as_deref().or(fallback_workspace.as_deref());
    let current = control.unpin_profile(scope, effective_workspace).await?;
    println!("Cleared {} pin.", scope.as_str());
    print_current_state(&current);
    Ok(())
}

pub async fn run_connection_test(profile_id: Option<String>) -> Result<()> {
    let control = connection_control()?;
    let profile_id = profile_or_current(profile_id).await?;
    let profile = control.get_profile(&profile_id).await?;
    let (resolved_model, message) = control.test_connection(&profile_id).await?;
    println!("profile_id: {}", profile.profile_id);
    println!("provider: {}", profile.provider.as_str());
    println!("resolved_model: {resolved_model}");
    println!("message: {message}");
    Ok(())
}

pub async fn run_connection_remove(profile_id: &str) -> Result<()> {
    let control = connection_control()?;
    let removed = control.delete_profile(profile_id).await?;
    if removed {
        println!("Removed connection profile {profile_id}.");
    } else {
        println!("Connection profile {profile_id} was not present.");
    }
    Ok(())
}
