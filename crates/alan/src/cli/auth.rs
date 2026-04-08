use alan_auth::{BrowserLoginOptions, ChatgptAuthManager, DeviceCodeLoginOptions};
use anyhow::Result;

pub async fn run_auth_login_chatgpt(
    use_device_code: bool,
    open_browser: bool,
    workspace_id: Option<&str>,
) -> Result<()> {
    let manager = ChatgptAuthManager::detect()?;
    if use_device_code {
        let prompt = manager.start_device_code().await?;
        println!(
            "Open this URL in your browser:\n{}",
            prompt.verification_url
        );
        println!();
        println!("Enter this one-time code:\n{}", prompt.user_code);
        println!();
        let login = manager
            .complete_device_code(
                &prompt,
                DeviceCodeLoginOptions {
                    forced_workspace_id: workspace_id.map(str::to_owned),
                },
            )
            .await?;
        println!(
            "{}",
            format_login_success_message(&login.email, &login.plan_type)
        );
        return Ok(());
    }

    let login = manager
        .login_with_browser(BrowserLoginOptions {
            open_browser,
            forced_workspace_id: workspace_id.map(str::to_owned),
            ..BrowserLoginOptions::default()
        })
        .await?;
    println!(
        "{}",
        format_login_success_message(&login.email, &login.plan_type)
    );
    Ok(())
}

pub async fn run_auth_status() -> Result<bool> {
    let manager = ChatgptAuthManager::detect()?;
    if let Some(status) = manager.status().await? {
        println!("provider: chatgpt");
        println!("storage: {}", status.storage_path.display());
        println!("account_id: {}", status.account_id);
        if let Some(email) = status.email {
            println!("email: {email}");
        }
        if let Some(plan_type) = status.plan_type {
            println!("plan: {plan_type}");
        }
        if let Some(expires_at) = status.access_token_expires_at {
            println!("access_token_expires_at: {expires_at}");
        }
        if let Some(last_refresh_at) = status.last_refresh_at {
            println!("last_refresh_at: {last_refresh_at}");
        }
        Ok(true)
    } else {
        println!("No managed ChatGPT login found.");
        Ok(false)
    }
}

pub async fn run_auth_logout() -> Result<bool> {
    let manager = ChatgptAuthManager::detect()?;
    let removed = manager.logout().await?;
    if removed {
        println!("Removed managed ChatGPT login.");
    } else {
        println!("No managed ChatGPT login was present.");
    }
    Ok(removed)
}

fn format_login_success_message(email: &Option<String>, plan_type: &Option<String>) -> String {
    let mut message = "Logged in to ChatGPT".to_string();
    if let Some(email) = email.as_deref() {
        message.push_str(&format!(" ({email})"));
    }
    if let Some(plan_type) = plan_type.as_deref() {
        message.push_str(&format!(", plan={plan_type}"));
    }
    message
}
