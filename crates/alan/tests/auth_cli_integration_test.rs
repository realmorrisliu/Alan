use alan_auth::{AuthStorage, AuthStore, ChatgptIdTokenInfo, ChatgptTokenData, StoredChatgptAuth};
use base64::Engine;
use serde_json::json;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn build_jwt(payload: serde_json::Value) -> String {
    let header =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
    format!("{header}.{payload}.sig")
}

fn seed_chatgpt_auth(home: &Path) {
    let auth_dir = home.join(".alan");
    std::fs::create_dir_all(&auth_dir).unwrap();
    let storage = AuthStorage::new(auth_dir.join("auth.json")).unwrap();
    let id_token = build_jwt(json!({
        "email": "user@example.com",
        "https://api.openai.com/auth": {
            "chatgpt_plan_type": "pro",
            "chatgpt_user_id": "user_123",
            "chatgpt_account_id": "acct_123"
        }
    }));
    let access_token = build_jwt(json!({"exp": 4_102_444_800_i64}));

    storage
        .save(&AuthStore {
            version: 1,
            chatgpt: Some(
                StoredChatgptAuth::from_tokens(ChatgptTokenData {
                    id_token: ChatgptIdTokenInfo {
                        email: Some("user@example.com".to_string()),
                        plan_type: Some("pro".to_string()),
                        user_id: Some("user_123".to_string()),
                        account_id: Some("acct_123".to_string()),
                        raw_jwt: id_token,
                    },
                    access_token,
                    refresh_token: "refresh_token".to_string(),
                })
                .unwrap(),
            ),
        })
        .unwrap();
}

#[test]
fn auth_status_reports_managed_chatgpt_login() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    seed_chatgpt_auth(&home);

    let output = Command::new(env!("CARGO_BIN_EXE_alan"))
        .args(["auth", "status"])
        .env("HOME", &home)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("provider: chatgpt"));
    assert!(stdout.contains("email: user@example.com"));
    assert!(stdout.contains("plan: pro"));
    assert!(!stdout.contains("acct_123"));
    assert!(!stdout.contains("user_123"));
}

#[test]
fn auth_logout_removes_managed_chatgpt_login() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    seed_chatgpt_auth(&home);

    let logout = Command::new(env!("CARGO_BIN_EXE_alan"))
        .args(["auth", "logout"])
        .env("HOME", &home)
        .output()
        .unwrap();
    assert!(logout.status.success(), "{logout:?}");
    assert!(String::from_utf8_lossy(&logout.stdout).contains("Removed managed ChatGPT login."));

    let status = Command::new(env!("CARGO_BIN_EXE_alan"))
        .args(["auth", "status"])
        .env("HOME", &home)
        .output()
        .unwrap();
    assert!(!status.status.success(), "{status:?}");
    assert!(String::from_utf8_lossy(&status.stdout).contains("No managed ChatGPT login found."));
}
